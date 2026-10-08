//! The lightwalletd gRPC channel over the host's [`NetDialer`](crate::ports::NetDialer) (spec §3.2a,
//! ADR-0526). We REUSE ECC's upstream-generated `CompactTxStreamerClient` and
//! `prost` message types whole — no `.proto`, no codegen, no gRPC framing of our
//! own. The only code here is the glue that makes tonic ride OUR dialed stream
//! instead of its own connector, and that does TLS ourselves.
//!
//! The connector is the seam chokepoint: every RPC dials through
//! [`crate::net::dialer::resolve_dialer`]'s plan (so `TorPolicy::Required`
//! fail-closes by construction), and `https` endpoints are TLS-wrapped with the
//! same `tokio-rustls` + `webpki-roots` stack the swap adapter uses (ALPN `h2`).
//! tonic's own connector and TLS are never invoked.
//
// Staged: consumed by the `LightdSyncEngine` (inc-2c-iv) + account import
// (inc-2c-iii); the §8 seam tests exercise it meanwhile.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use async_trait::async_trait;
use http::Uri;
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::{Channel, Endpoint};
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_backend::proto::service::compact_tx_streamer_client::CompactTxStreamerClient;
use zcash_client_backend::proto::service::{
    BlockId, BlockRange, ChainSpec, Empty, GetAddressUtxosArg, GetSubtreeRootsArg, LightdInfo,
    RawTransaction, SendResponse, ShieldedProtocol, SubtreeRoot, TreeState, TxFilter,
};
use zcash_protocol::TxId;

use crate::enhance::FetchedTransaction;

use crate::config::{EndpointAuth, LightServerEndpoint, TorPolicy};
use crate::constants::{
    DIAL_TIMEOUT_SECS, GRPC_MAX_MESSAGE_BYTES, GRPC_STREAMING_TIMEOUT_SECS, GRPC_UNARY_TIMEOUT_SECS,
};
use crate::error::WalletError;
use crate::net::NetError;
use crate::net::dial_counters::DialCounters;
use crate::net::dialer::{PlanDialer, resolve_dialer};
use crate::net::tor_posture::{DialArm, PathClass, TorPosture};
use crate::ports::AsyncByteStream;
use crate::state::StallReason;
use crate::transparent::TransparentUtxoRecord;

/// A failed gRPC call (§6.1). Carries the policy stall reason so the sync engine
/// (inc-2c-iv) renders the right `SyncStatus::Stalled` — `TorUnavailable` under
/// `Required` (the fail-closed case), `EndpointUnreachable` otherwise — on a
/// TRANSPORT failure whose DIAL failed; a transport failure over a connection
/// the private path ACCEPTED, and a TIMEOUT, carry `EndpointUnreachable`
/// whatever the plan ([`transport_stall`], [`TIMEOUT_STALL`]). Server
/// statuses carry the CODE only (§5.4 — never the message, which can echo data).
#[derive(Debug)]
pub(crate) enum GrpcError {
    /// Could not reach/contact the endpoint (dial, TLS, or transport failure).
    /// `stall` is the plan's fail-closed reason only when the dial itself
    /// failed — see [`transport_stall`].
    Transport { stall: StallReason },
    /// The call timed out (monotonic): unary calls are bounded by
    /// [`GRPC_UNARY_TIMEOUT_SECS`]; stream establishment and each inter-block wait
    /// by [`GRPC_STREAMING_TIMEOUT_SECS`] (the iv-a `BlockStream` arm). `stall`
    /// is [`TIMEOUT_STALL`] whatever the plan's reason — see there.
    Timeout { stall: StallReason },
    /// The server returned a non-OK gRPC status — code only. The code is read by
    /// the `Debug` rendering alone (diagnostics); nothing keys on it (P3-7).
    Status {
        #[allow(dead_code)]
        code: i32,
    },
    /// The endpoint REFUSED a `GetSubtreeRoots` call because it does not know the
    /// `ShieldedProtocol` we asked for (T0-1 A9). Distinct from [`Self::Status`] so
    /// the ingestion can tell "this server is too old for this pool" apart from
    /// "this server is broken", and distinct from [`Self::Transport`] because the
    /// server ANSWERED — the link is fine.
    ///
    /// **Why this variant exists at all.** Measured 2026-09-08
    /// (`docs/plan/probes/ironwood-subtree-roots-probe.output.txt`): an unrecognized
    /// protocol is `Unknown (2)` on lightwalletd v0.5.3 and `InvalidArgument (3)` on
    /// v0.5.4, and `classify_status` maps `Unknown` to [`Self::Transport`] — so
    /// before this variant a v0.5.3 refusal was INDISTINGUISHABLE from a flaky link
    /// and retried forever. The `code` is carried for diagnostics only; NOTHING
    /// keys on it, precisely because it differs by server version (§5.4: never the
    /// message text either).
    ShieldedProtocolUnknown {
        #[allow(dead_code)] // diagnostics only — the variant doc says why (P3-7)
        code: i32,
    },
}

/// The endpoint's verdict on a submitted transaction (the mempool-acceptance half of
/// §2.5 `TxSubmitResult`), distinct from a [`GrpcError`] (= the tx never REACHED the
/// endpoint). Carries the CODE only — §5.4 drops the `SendResponse::error_message`,
/// which can echo a txid/amount.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SubmitOutcome {
    /// `error_code == 0`: the endpoint accepted the tx into its mempool.
    Accepted,
    /// `error_code != 0`: the endpoint rejected it (e.g. already-spent input, a fee
    /// floor) — the numeric code only.
    Rejected { code: i32 },
}

impl SubmitOutcome {
    /// Map the upstream `SendResponse`. `error_code == 0` is the lightwalletd success
    /// sentinel; the §5.4-sensitive `error_message` is DROPPED here at the funnel, never
    /// logged or surfaced.
    fn from_send_response(resp: SendResponse) -> Self {
        if resp.error_code == 0 {
            Self::Accepted
        } else {
            Self::Rejected {
                code: resp.error_code,
            }
        }
    }
}

/// A connected (lazily) lightwalletd client. `connect` never dials — the channel
/// is lazy, so the first RPC carries the dial under its own timeout; building a
/// client is therefore infallible past config resolution and offline-safe.
/// Injects the endpoint's auth header, when the host configured one, onto every
/// outbound request.
///
/// **It is installed unconditionally, and that is deliberate.** `with_interceptor`
/// changes the client's TYPE, so making it conditional would mean two different
/// `LightwalletdClient` shapes (or a boxed service) threaded through every call
/// site. One type with a `None` that injects nothing is simpler and cannot drift.
///
/// §5.4: the value it inserts is built per call and marked sensitive by
/// [`EndpointAuth::with_value`], so it is never HPACK-indexed and no
/// un-zeroizable copy outlives the request.
#[derive(Clone)]
pub(crate) struct AuthInterceptor {
    auth: Option<EndpointAuth>,
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        let Some(auth) = &self.auth else {
            return Ok(request);
        };
        // Both halves were validated in `EndpointAuth::new` (a valid header name,
        // and a value with no CR/LF/NUL), so neither conversion can fail here —
        // but an `expect` on a credential path would be a panic reachable from
        // host config, so they are handled as a refusal instead. `Unauthenticated`
        // is the honest code: we could not present the credential.
        let key = tonic::metadata::MetadataKey::from_bytes(auth.header().as_str().as_bytes())
            .map_err(|_| {
                tonic::Status::unauthenticated("endpoint auth header name is not sendable")
            })?;
        let mut value = auth
            .with_value(|v| tonic::metadata::MetadataValue::try_from(v))
            .map_err(|_| {
                tonic::Status::unauthenticated("endpoint auth header value is not sendable")
            })?;
        // NOT cosmetic: this keeps the secret out of HPACK's dynamic index, so it
        // is not retained in the connection's compression table and replayed from
        // there. Built fresh per request, so the only long-lived copy stays
        // `Zeroizing` in `EndpointAuth`.
        value.set_sensitive(true);
        request.metadata_mut().insert(key, value);
        Ok(request)
    }
}

/// The stall a TIMEOUT carries, for every plan (stage S1 `truth`, the F-C
/// correction). A timeout is what an ACCEPTED connection that carries nothing
/// looks like — the dial's own bound is shorter than any RPC's
/// (`DIAL_TIMEOUT_SECS < GRPC_UNARY_TIMEOUT_SECS`, asserted at compile time),
/// so a dial that fails surfaces as a transport failure first and a timer that
/// fires has an accepted dial behind it — and from here a blackholed transport
/// and a wedged server look the same, so it claims nothing about the path.
/// Until this stage a `Required` plan's timeout carried `TorUnavailable`,
/// which rendered "the private path is unavailable — nothing was sent in the
/// clear" over a wedged SERVER on a healthy path: an over-claim in the other
/// direction (contract §3.0 F-C). Now a `Required` wallet reads `Active` +
/// `Stalled { EndpointUnreachable }` from the first observed failure, and
/// `TorState::Unanswered` once the silence has lasted the maintainer's minute.
///
/// A TRANSPORT failure asks the same question of the connection it failed
/// over, [`transport_stall`], and the two agree on the one rule: the plan's
/// `TorUnavailable` is stamped by a dial that FAILED — refused, unreachable,
/// timed out at the dial, a bootstrap not ready, a retired or FAILED
/// descriptor — promptly, as before, and by nothing else. Over `https` (the
/// only scheme the door admits under a host runtime) an accepted dial that
/// carries nothing never completes TLS and is cut by the connector's own
/// bound as a transport `Status`, not a timeout; an accept-then-close is a
/// transport `Status` too. Keying the reason on which TIMER fired left both
/// reading "the private path is unavailable" — the ruling on the join,
/// `docs/adjudication/s1-truth/ruling.md`.
const TIMEOUT_STALL: StallReason = StallReason::EndpointUnreachable;

/// The stall a TRANSPORT failure carries, resolved against the connection it
/// failed over (stage S1 `truth`, the ruling's repair). The plan's fail-closed
/// reason is a claim about the PATH — "the Tor path could not be established"
/// — and only a dial that failed can make it. Over a connection whose dial
/// was ACCEPTED (the `connected` line, the dial's `Ok`) the path did its part;
/// what failed after it — a TLS handshake that never completes, a peer that
/// hangs up, a link that dies mid-RPC — is the far end's, and from here a
/// wedged server and a path that carries nothing are the same sight, so it
/// carries the same word a timeout does: `EndpointUnreachable`. The verdict
/// is the connection's LAST dial: the connector clears it when a redial of a
/// dead connection fails ([`ConnectionState::note_dial_failed`]), so an
/// accept from a minute ago cannot make a refused redial read as the server's
/// fault.
///
/// `None` is an `Off` plan: no private path exists to blame, and
/// `resolve_dialer` gives that plan a reason that claims none. `Preferred`'s
/// plan already claims nothing on either branch.
///
/// WHAT THIS RESTS ON, and where it is owned: "the path did its part" is only
/// true if a dialer's `Ok` means the stream is established END TO END. That is
/// not an inference about the first host — it is a clause of
/// [`crate::ports::NetDialer::dial`]'s contract, stated there because the SDK
/// is universal and a host whose transport accepts optimistically (a SOCKS
/// front end answering before the proxy's CONNECT reply, a circuit built
/// lazily behind a duplex pipe) would make a `Required` wallet read `Active` +
/// "the server is not answering" over a path that is dead. This function
/// asserts the property; the port doc owns it.
fn transport_stall(plan: StallReason, witness: Option<&PrivateRpcWitness>) -> StallReason {
    match witness {
        Some(witness) if witness.connection.dial_accepted() => StallReason::EndpointUnreachable,
        _ => plan,
    }
}

/// The handle that lets a COMPLETED RPC restart the maintainer's patience window
/// (ADR-0552 stage 1b).
///
/// It exists because a dial is not evidence: a transport that accepts and then
/// blackholes — active censorship's cheapest move — completes connects forever
/// while carrying nothing. The dialer marks a class's connection live; this is
/// what confirms that the circuit actually carried gRPC. `None` on a client
/// whose policy has no private window to report into (`Off`; `Required` has
/// one since stage S1 `truth` Q2).
///
/// §5.4: it records a CLASS, an arm and a timestamp. No host, port, txid or
/// isolation key is stored or logged here.
#[derive(Clone)]
struct PrivateRpcWitness {
    posture: Arc<TorPosture>,
    class: PathClass,
    /// The connection THIS witness reports on — the one its client's channel
    /// rides, and a stream keeps the one it was opened over. Evidence is
    /// attributed through it, never through the class (phase 2 §2b).
    connection: Arc<ConnectionState>,
}

impl PrivateRpcWitness {
    /// An RPC completed on this witness's connection. The posture ignores it
    /// unless the PRIVATE arm served that connection — so traffic that rode the
    /// clearnet fallback can never be read back as the private path working,
    /// whatever a sibling connection of the same class is doing.
    fn note_carried(&self) {
        if let Some(served_by) = self.connection.served_by() {
            self.posture.note_private_success(self.class, served_by);
        }
    }

    /// An RPC FAILED over this witness's connection — the other half of the
    /// evidence, and it is not optional.
    ///
    /// The blackhole shape finding (i) exists to catch is a transport that
    /// ACCEPTS and carries nothing: its dials succeed, so a failure clock fed
    /// only by the dialer would never start on it and the wallet would insist
    /// for ever. That is the defect the whole window was built to close, so the
    /// RPC layer — the only place that learns an accepted circuit delivered
    /// nothing — reports it.
    ///
    /// Guarded by the connection's arm: an RPC failing over the CLEARNET
    /// fallback says nothing about the private path, and counting it would let
    /// a bad clearnet link spend a window that decides whether traffic leaves
    /// in the clear. `None` — no dial has completed, or the last one FAILED
    /// (`served_by` is cleared by a failed redial) — is nobody's evidence
    /// here: the dialer has already recorded whatever the dial itself did.
    ///
    /// A PRIVATE connection that failed an RPC is also RETIRED: see
    /// [`ConnectionState::retired`].
    fn note_failed(&self) {
        let Some(served_by) = self.connection.served_by() else {
            return;
        };
        self.posture.note_rpc_failure(self.class, served_by);
        if served_by == DialArm::Private {
            self.connection.retire();
        }
    }
}

/// What a client knows about the ONE connection its channel rides: which arm
/// served it, and whether it is still worth an RPC. Written by the connector
/// at dial time and by the witness; never logged.
#[derive(Default)]
struct ConnectionState {
    /// [`DialArm`] as `1` = private, `2` = clearnet; `0` until a dial is
    /// ACCEPTED, and `0` again after one fails — the verdict of the LAST dial,
    /// which is what [`transport_stall`] reads (a failed redial must not
    /// inherit the accept before it).
    served_by: AtomicU8,
    /// A private connection that has FAILED an RPC is not reused: the client
    /// opens a fresh channel for its next call ([`LightwalletdClient::witness`]).
    ///
    /// This is the half of the consumer gap a dialer cannot close. The sync
    /// client is built once and kept for the wallet's life, its channel is lazy
    /// and has no keepalive, so a peer that accepts and never answers produced
    /// exactly ONE dial — and `PolicyDialer`, the only reader of the patience
    /// window, was never asked again however long the minute had been spent.
    /// Retiring the connection is what puts the next RPC back in front of it.
    /// The window is NOT read here: this says "dial again", and the dialer
    /// says where.
    ///
    /// An EDGE, not a level: one failed RPC retires one connection, once. After
    /// a switch the connection is the clearnet one, which never retires, so a
    /// spent window cannot turn into a redial per pass.
    retired: AtomicBool,
}

impl ConnectionState {
    fn served_by(&self) -> Option<DialArm> {
        match self.served_by.load(Ordering::Relaxed) {
            1 => Some(DialArm::Private),
            2 => Some(DialArm::Clearnet),
            _ => None,
        }
    }

    fn set_served_by(&self, arm: DialArm) {
        let raw = match arm {
            DialArm::Private => 1,
            DialArm::Clearnet => 2,
        };
        self.served_by.store(raw, Ordering::Relaxed);
    }

    /// The last dial FAILED: no connection stands behind the channel, so no
    /// arm served it. Written by the connector on a dial's `Err`, which is
    /// how a redial of a dead connection sheds the accept before it.
    fn note_dial_failed(&self) {
        self.served_by.store(0, Ordering::Relaxed);
    }

    /// Whether the last dial was ACCEPTED, on either arm — the `connected`
    /// line. What [`transport_stall`] keys the plan's fail-closed reason on.
    fn dial_accepted(&self) -> bool {
        self.served_by().is_some()
    }

    fn retire(&self) {
        self.retired.store(true, Ordering::Relaxed);
    }

    fn is_retired(&self) -> bool {
        self.retired.load(Ordering::Relaxed)
    }
}

type GrpcClient = CompactTxStreamerClient<InterceptedService<Channel, AuthInterceptor>>;

pub(crate) struct LightwalletdClient {
    client: GrpcClient,
    stall_on_failure: StallReason,
    /// See [`PrivateRpcWitness`] — the ADR-0552 stage 1b confirmation handle.
    /// Read through [`Self::witness`], which is also where a retired
    /// connection is replaced.
    witness: Option<PrivateRpcWitness>,
    /// What `client` was built from, kept so a retired connection can be
    /// replaced by a fresh lazy channel over the SAME plan, target and key.
    recipe: ChannelRecipe,
}

/// Everything a lazy channel is made of. Holds the client's second copy of the
/// endpoint credential (`Zeroizing`, like the interceptor's own).
struct ChannelRecipe {
    uri: Uri,
    connector: DialerConnector,
    auth: AuthInterceptor,
}

impl ChannelRecipe {
    /// A fresh lazy channel and the state of the connection it will ride. No
    /// dial happens here; the first RPC carries it under its own timeout.
    fn open(&self) -> (GrpcClient, Arc<ConnectionState>) {
        let connection = Arc::new(ConnectionState::default());
        let connector = DialerConnector {
            connection: Arc::clone(&connection),
            ..self.connector.clone()
        };
        let channel = Endpoint::from(self.uri.clone()).connect_with_connector_lazy(connector);
        let client = CompactTxStreamerClient::with_interceptor(channel, self.auth.clone())
            // every gRPC byte is hostile input (§4.6) — cap decode/encode size
            .max_decoding_message_size(GRPC_MAX_MESSAGE_BYTES)
            .max_encoding_message_size(GRPC_MAX_MESSAGE_BYTES);
        (client, connection)
    }
}

impl LightwalletdClient {
    /// Build the channel over the host's network infrastructure (ADR-0526): the
    /// `TorPolicy` resolves to a dialer, and tonic rides it via a lazy
    /// `connect_with_connector` channel. `isolation_key` is the sync circuit key
    /// (§2.3) passed to every dial.
    ///
    /// ISOLATION (inc-2d note): one `Channel` multiplexes all its RPCs over ONE
    /// dialed connection = one circuit, so a single client carries ONE isolation
    /// key. The §2.3 per-txid broadcast unlinkability therefore requires a FRESH
    /// `LightwalletdClient` per `send_transaction` (its own key → its own circuit);
    /// reusing the long-lived sync client for broadcast would link the tx to the
    /// sync circuit.
    ///
    /// `posture` is the wallet-level Tor posture (`Inner.tor_posture`) shared into the
    /// dialer (the INC-2D GATE): EVERY client the wallet builds — the one sync client
    /// AND each fresh broadcast client — passes the SAME flag, so a `Preferred`
    /// degradation on ANY of them surfaces to `tor_state()`. Only `Preferred` ever
    /// writes it (`Off`/`Required` leave it untouched, §3.2a). `counts` is the
    /// wallet's dial tally (`Inner.dial_counters`, FR-37), shared the same way so
    /// every dial any client makes lands in the one snapshot a host reads.
    /// `auth`, when the host configured it, is sent as a header on EVERY request
    /// through this client — including each fresh per-broadcast client, so a
    /// key-gated endpoint does not silently break `send_transaction` while sync
    /// keeps working.
    pub(crate) fn connect(
        endpoint: &LightServerEndpoint,
        policy: &TorPolicy,
        isolation_key: Option<String>,
        posture: &Arc<TorPosture>,
        counts: &Arc<DialCounters>,
        auth: Option<&EndpointAuth>,
    ) -> Result<Self, NetError> {
        // The door's ONE predicate (`config::validate_transport`), asked again
        // where a plaintext endpoint can arrive at RUNTIME — the sync-server
        // picker probes and switches onto a custom choice after open. TLS is
        // chosen by scheme alone, and only the SDK's own direct dialer (`Off`)
        // may carry a plaintext loopback dial; any runtime would hand the
        // hostname to the host's transport and ship raw gRPC through it (FR-29
        // stage 0 (ii); ADR-0544 decision 3). Refused before a channel exists,
        // with the door's reason (the probe keeps it typed); the closure's `_` arm is DEAD by contract (only `InvalidEndpoint` is returned) and keeps the match total — a future door error gets its own arm, never the generic reason.
        crate::config::validate_transport(endpoint, &[], policy).map_err(|e| match e {
            WalletError::InvalidEndpoint { reason } => NetError::InvalidEndpoint { reason },
            _ => NetError::InvalidEndpoint {
                reason: "transport validation refused the endpoint",
            },
        })?;
        Self::connect_unchecked(endpoint, policy, isolation_key, posture, counts, auth)
    }

    /// TEST ONLY — the dialer-routing proofs in this module (`…rides_the_host_
    /// dialer_not_a_direct_socket`, the silent-server timeouts) run a PLAINTEXT
    /// loopback gRPC server behind a test dialer, which is exactly the shape the
    /// production [`Self::connect`] refuses (FR-29 stage 0 (ii)). This seam skips
    /// ONLY that refusal so those proofs keep proving what they name (bytes go
    /// through the dialer, never a direct socket); the plaintext rule's own tests
    /// use `connect`. Does not exist outside `cfg(test)`, and
    /// `http_endpoint_over_a_host_dialer_is_refused_at_connect` pins that
    /// `connect_unchecked` has exactly these two callers.
    ///
    /// No dial tally is taken: there is no wallet behind this seam, and the
    /// counters are a wallet's (`Wallet::dial_counts`). The dials it makes are
    /// tallied into a throwaway so the attribution line's one writer keeps
    /// its shape; a test of the COUNTS drives a real wallet.
    #[cfg(test)]
    pub(crate) fn connect_plaintext_over_test_dialer(
        endpoint: &LightServerEndpoint,
        policy: &TorPolicy,
        isolation_key: Option<String>,
        posture: &Arc<TorPosture>,
        auth: Option<&EndpointAuth>,
    ) -> Result<Self, NetError> {
        Self::connect_unchecked(
            endpoint,
            policy,
            isolation_key,
            posture,
            &Arc::new(DialCounters::default()),
            auth,
        )
    }

    /// The channel build proper — reached ONLY through [`Self::connect`] (the
    /// transport rule applied) and the `cfg(test)` seam above.
    fn connect_unchecked(
        endpoint: &LightServerEndpoint,
        policy: &TorPolicy,
        isolation_key: Option<String>,
        posture: &Arc<TorPosture>,
        counts: &Arc<DialCounters>,
        auth: Option<&EndpointAuth>,
    ) -> Result<Self, NetError> {
        let plan = resolve_dialer(policy, posture, counts)?;
        // Read BEFORE `isolation_key` moves into the connector. The class must
        // be the SAME one the dialer derives from the same key — `PathClass::of`
        // is the one classifier, so a dial fails the window an RPC confirms.
        let class = PathClass::of(isolation_key.as_deref());
        let (uri, host, port, is_https) = endpoint_parts(endpoint)?;
        let recipe = ChannelRecipe {
            uri,
            connector: DialerConnector {
                dialer: plan.dialer,
                // OUR TLS for https; loopback http (validator-guaranteed, Off-only) is plaintext
                tls: is_https.then(|| Arc::new(GrpcTls::new())),
                host,
                port,
                isolation_key,
                // A template: every channel opened from it gets its own.
                connection: Arc::default(),
            },
            auth: AuthInterceptor {
                auth: auth.cloned(),
            },
        };
        // Lazy: no dial until the first RPC, so this never blocks/fails on the
        // network and the dial rides the first call's unary timeout.
        let (client, connection) = recipe.open();
        let witness = plan.private_window.map(|posture| PrivateRpcWitness {
            posture,
            class,
            connection,
        });
        Ok(Self {
            client,
            stall_on_failure: plan.stall_on_failure,
            witness,
            recipe,
        })
    }

    /// The witness for the RPC about to be made — and, first, the ONE place a
    /// retired connection is replaced ([`ConnectionState::retired`]). Every
    /// RPC takes its witness here, so none can ride a connection that has
    /// already failed the private path.
    ///
    /// The old channel is dropped, not torn down: a stream still open over it
    /// keeps its own handle and its own witness, and goes on reporting against
    /// the connection it actually rode. `None` (`Off`) has no window and never
    /// rebuilds. `Required` has one since stage S1 `truth` (Q2): it reports
    /// the same evidence and retires a failed private connection the same way
    /// — a fresh channel after a failed RPC, never a fallback.
    fn witness(&mut self) -> Option<PrivateRpcWitness> {
        let witness = self.witness.as_mut()?;
        if witness.connection.is_retired() {
            let (client, connection) = self.recipe.open();
            self.client = client;
            witness.connection = connection;
        }
        Some(witness.clone())
    }

    /// Broadcast a fully-signed, consensus-serialized transaction to the endpoint's
    /// mempool (the inc-2d-3-a broadcast arm). `raw_tx` is the tx bytes read back from
    /// the wallet DB AFTER create+sign persisted it (§6.3 persist-before-submit) — this
    /// seam owns NO money/crypto logic, only the wire. `height: 0` is the upstream
    /// `RawTransaction` "submit to mempool" sentinel. Maps `SendResponse` to a
    /// [`SubmitOutcome`] (accepted vs the rejection CODE — §5.4 drops the message); a
    /// transport/timeout failure is the policy-stall [`GrpcError`] (the tx never reached
    /// the endpoint). ONE attempt — the §1.7 resubmission machinery (inc-2d-3-b) owns
    /// retry, never this call. The bytes are NEVER logged (§5.4: txids/amounts/tx bytes).
    ///
    /// SAFE-DIRECTION ASYMMETRY (money): if the endpoint ACCEPTS the tx into its mempool but
    /// the `SendResponse` is lost on the return path (a TCP reset after the server-side accept
    /// — common on flaky mobile), this returns `Err(GrpcError::Transport)` → the caller reports
    /// `GrpcFailure` for a tx that actually LANDED (a false NEGATIVE). The inverse — a false
    /// `Accepted` for an un-landed tx — is structurally impossible: `Accepted` requires a
    /// received `error_code == 0`. A false negative is the safe direction: the chain is the
    /// source of truth (§6.3), so the landed tx is rediscovered by scan and the §1.7
    /// resubmission is idempotent at the mempool level (re-submitting an accepted/mined tx is a
    /// no-op rejection, never a double-spend) — the mis-report self-heals on the next sync.
    pub(crate) async fn send_transaction(
        &mut self,
        raw_tx: Vec<u8>,
    ) -> Result<SubmitOutcome, GrpcError> {
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let call = self.client.send_transaction(RawTransaction {
            data: raw_tx,
            height: 0,
        });
        map_unary(stall, witness.as_ref(), timeout(call).await)
            .map(SubmitOutcome::from_send_response)
    }

    /// Server + network handshake DTO. The engine reads chain name + Sapling
    /// activation for the wrong-CHAIN guard (§6.1 `network_mismatch` endpoint
    /// arm) and `consensus_branch_id` + `block_height` for the consensus
    /// staleness verdict (`ironwood-nu63-support.md` §3.1).
    ///
    /// This comment used to say the engine read the branch. It did not — the
    /// two live fields were fetched here and discarded in `ServerIdentity`,
    /// which is how the Ironwood/NU6.3 send outage ran silently from
    /// 2026-07-28. Both are read now; a doc comment is not a check.
    pub(crate) async fn get_lightd_info(&mut self) -> Result<LightdInfo, GrpcError> {
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let call = self.client.get_lightd_info(Empty {});
        map_unary(stall, witness.as_ref(), timeout(call).await)
    }

    /// Best-chain tip (drives tip-following + the §2.3 birthday range check).
    /// Returns the RAW upstream DTO from an UNTRUSTED endpoint — the consumer
    /// validates (a lying endpoint is the §4.6 / M2 threat; this seam only
    /// authenticates the transport, never the endpoint's honesty).
    pub(crate) async fn get_latest_block(&mut self) -> Result<BlockId, GrpcError> {
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let call = self.client.get_latest_block(ChainSpec {});
        map_unary(stall, witness.as_ref(), timeout(call).await)
    }

    /// Commitment-tree state at `height` (the account-import birthday treestate,
    /// inc-2c-iii). RAW upstream DTO from an UNTRUSTED endpoint — the consumer
    /// validates: a poisoned tree-state is the M2 silent-fund-hiding threat
    /// (`poisoned_checkpoint_or_tree_state_does_not_silently_hide_funds`), and
    /// its continuity check rides the sync-engine crypto-change review, not this seam.
    pub(crate) async fn get_tree_state(&mut self, height: u64) -> Result<TreeState, GrpcError> {
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let call = self.client.get_tree_state(BlockId {
            height,
            hash: Vec::new(),
        });
        map_unary(stall, witness.as_ref(), timeout(call).await)
    }

    /// Open a server-stream of [`CompactBlock`]s for the inclusive height range
    /// `[start_height, end_height]` (the inc-2c-iv-d scan loop's download arm).
    /// Returns a [`BlockStream`] the caller pulls block-by-block; iv-b wraps it to
    /// spool blocks into the disposable FS cache. Establishing the stream is
    /// bounded by [`GRPC_STREAMING_TIMEOUT_SECS`] (the lazy dial rides here, itself
    /// already bounded by `DIAL_TIMEOUT_SECS`), so a hung connect can't park here.
    ///
    /// CONTRACT: `start_height <= end_height`, ascending — the caller (iv-d)
    /// derives ranges from `suggest_scan_ranges`, which are always valid; an
    /// inverted/garbage range is a caller bug, and the per-message idle timeout on
    /// [`BlockStream`] is the backstop against any resulting server-side hang. No
    /// money/crypto logic here: each block is hostile input (the
    /// `max_decoding_message_size` cap from `connect` applies per streamed
    /// message), VALIDATED later by `scan_cached_blocks` (consumed whole, §3.2) —
    /// this seam only authenticates the transport, never the blocks' honesty.
    pub(crate) async fn get_block_range(
        &mut self,
        start_height: u64,
        end_height: u64,
    ) -> Result<BlockStream, GrpcError> {
        // An inverted range is a caller (iv-d) bug, not hostile input — fire at the
        // call site in tests/debug rather than only as a server-side hang the idle
        // timeout eventually catches. Zero release cost.
        debug_assert!(
            start_height <= end_height,
            "get_block_range: inverted range is a caller bug (start > end)"
        );
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let range = BlockRange {
            start: Some(BlockId {
                height: start_height,
                hash: Vec::new(),
            }),
            end: Some(BlockId {
                height: end_height,
                hash: Vec::new(),
            }),
            // Empty ⇒ the legacy shielded default (Sapling + Orchard) — exactly the
            // v1 account-0 scope (§1.6, no transparent). Setting `pool_types`
            // non-empty requires first verifying server capability (upstream proto
            // contract), which we have not, so it stays empty.
            pool_types: Vec::new(),
        };
        let call = self.client.get_block_range(range);
        match stream_timeout(call).await {
            Err(_elapsed) => {
                // A stream that never sent headers is the blackhole shape at
                // establishment — the same evidence as a timed-out unary.
                if let Some(witness) = &witness {
                    witness.note_failed();
                }
                Err(GrpcError::Timeout {
                    stall: TIMEOUT_STALL,
                })
            }
            // Response HEADERS arrived: a round trip the private path carried,
            // which a blackhole never produces (ADR-0552 stage 1b). The witness
            // travels INTO the stream so every message pull keeps confirming —
            // without that, a slow-but-working link whose batch outlasts the
            // minute would bank silence and switch, making the repair leak more
            // readily than the defect.
            Ok(Ok(resp)) => {
                if let Some(witness) = &witness {
                    witness.note_carried();
                }
                Ok(MessageStream::live(resp.into_inner(), stall, witness))
            }
            Ok(Err(status)) => {
                let err = classify_status(transport_stall(stall, witness.as_ref()), &status);
                note_if_transport(&err, witness.as_ref());
                Err(err)
            }
        }
    }

    /// Open a server-stream of [`SubtreeRoot`]s for `protocol` (the inc-2c-iv-d-1
    /// commitment-tree ingestion — the streaming sibling of `get_block_range`).
    /// `start_index` is the first subtree index to return; `max_entries == 0` means
    /// "all remaining" (the upstream `GetSubtreeRootsArg` contract). Returns a
    /// [`SubtreeRootStream`] the caller drains root-by-root; establishment is bounded
    /// by [`GRPC_STREAMING_TIMEOUT_SECS`] (the lazy dial rides here), each
    /// inter-root wait by the same per-message idle timeout as [`BlockStream`].
    ///
    /// Pure transport, NO money/crypto logic: each [`SubtreeRoot`] is hostile input
    /// (the `max_decoding_message_size` cap from `connect` applies per message),
    /// VALIDATED later — the leaf-hash decode + the shardtree merge (the spend-witness
    /// trust anchor) live in the sync engine (`crate::sync`), not this seam. This
    /// seam only authenticates the transport, never the roots' honesty.
    pub(crate) async fn get_subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
        max_entries: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let arg = GetSubtreeRootsArg {
            start_index,
            shielded_protocol: protocol as i32,
            max_entries,
        };
        let call = self.client.get_subtree_roots(arg);
        match stream_timeout(call).await {
            Err(_elapsed) => {
                // A stream that never sent headers is the blackhole shape at
                // establishment — the same evidence as a timed-out unary.
                if let Some(witness) = &witness {
                    witness.note_failed();
                }
                Err(GrpcError::Timeout {
                    stall: TIMEOUT_STALL,
                })
            }
            // Response HEADERS arrived: a round trip the private path carried,
            // which a blackhole never produces (ADR-0552 stage 1b). The witness
            // travels INTO the stream so every message pull keeps confirming —
            // without that, a slow-but-working link whose batch outlasts the
            // minute would bank silence and switch, making the repair leak more
            // readily than the defect.
            Ok(Ok(resp)) => {
                if let Some(witness) = &witness {
                    witness.note_carried();
                }
                Ok(MessageStream::live(resp.into_inner(), stall, witness))
            }
            // PROTOCOL-AWARE (A9): a server that does not know `protocol` refuses
            // HERE if its handler errors before headers, and on the first pull if it
            // does not — `SubtreeRoot::classify_stream_status` covers that door with
            // the same classifier.
            Ok(Err(status)) => {
                let err = classify_subtree_roots_status(
                    transport_stall(stall, witness.as_ref()),
                    &status,
                );
                note_if_transport(&err, witness.as_ref());
                Err(err)
            }
        }
    }

    /// Poll the endpoint for the CURRENT unspent transparent outputs paying any of `addresses`,
    /// mined at or above `start_height` (the §3.3a Recv-2b detection RPC). Compact-block scan is
    /// shielded-only, so this UNARY poll is the ONLY path that surfaces a transparent receive.
    ///
    /// `max_entries` is left UNLIMITED (`0`) here: `GRPC_MAX_MESSAGE_BYTES` (from
    /// [`connect`](Self::connect)) is the wire DoS bound — a reply over it fails decode LOUDLY
    /// (retry next pass) — and the per-pass PUT cap lives one layer up, after validation
    /// (`SCOPED_UTXO_POLL_MAX_PUTS`, reported `truncated`, never a silent under-count). The reply is
    /// flattened to owned [`TransparentUtxoRecord`]s (the proto type never leaves this module).
    ///
    /// RAW DTOs from an UNTRUSTED endpoint: this seam only authenticates the transport. The
    /// consumer ([`crate::transparent::validate`]) re-derives each recipient from the script and
    /// matches it to OUR address — a lying endpoint that returns a UTXO for an address we do not
    /// own (the M2 threat) is rejected at validation, never put.
    pub(crate) async fn get_address_utxos(
        &mut self,
        addresses: Vec<String>,
        start_height: u64,
    ) -> Result<Vec<TransparentUtxoRecord>, GrpcError> {
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let arg = GetAddressUtxosArg {
            addresses,
            start_height,
            max_entries: 0,
        };
        let call = self.client.get_address_utxos(arg);
        map_unary(stall, witness.as_ref(), timeout(call).await).map(|list| {
            list.address_utxos
                .into_iter()
                .map(|u| TransparentUtxoRecord {
                    txid: u.txid,
                    index: u.index,
                    script: u.script,
                    value_zat: u.value_zat,
                    height: u.height,
                })
                .collect()
        })
    }

    /// Fetch the FULL consensus-encoded transaction for `txid` (the §3.3 tx-enhancement /
    /// memo-recovery path — compact blocks omit memo bytes, so the scanner flags these txids
    /// for download). `Ok(None)` ⇒ the endpoint returned an EMPTY body: it has no such tx, and
    /// the caller records `TxidNotRecognized` to clear the request. `TxFilter.hash` is INTERNAL
    /// byte order (`TxId::as_ref()`), the order the wire stores; `block`/`index` are unused for
    /// a hash lookup. The reply size is already bounded by the channel's
    /// `max_decoding_message_size` (`GRPC_MAX_MESSAGE_BYTES`) — every gRPC byte is hostile (§4.6).
    ///
    /// RAW bytes from an UNTRUSTED endpoint — this seam only authenticates the transport. The
    /// consumer ([`crate::enhance::parse_and_validate`]) RE-PARSES the bytes and verifies the
    /// parsed `Transaction::txid()` matches `txid` before anything is decrypted/stored: a lying
    /// endpoint that returns the wrong or garbled tx (the M2 threat) is rejected, never trusted.
    pub(crate) async fn get_transaction(
        &mut self,
        txid: TxId,
    ) -> Result<Option<FetchedTransaction>, GrpcError> {
        let stall = self.stall_on_failure;
        // Captured alongside `stall`, BEFORE the call borrows `self.client`
        // mutably — the same reason `stall` is copied out here.
        let witness = self.witness();
        let call = self.client.get_transaction(TxFilter {
            block: None,
            index: 0,
            hash: txid.as_ref().to_vec(),
        });
        map_unary(stall, witness.as_ref(), timeout(call).await).map(|raw| {
            // An empty body is lightwalletd's "no such tx" answer (not an error). Flatten the
            // proto into the domain DTO so the wire type never leaves this module.
            if raw.data.is_empty() {
                None
            } else {
                Some(FetchedTransaction {
                    data: raw.data,
                    height: raw.height,
                })
            }
        })
    }
}

/// The minimal "pull the next streamed message" capability behind
/// [`MessageStream`]. It exists so the reliability-critical pump (the per-message
/// idle timeout + error classification + the SPENT-ON-ERROR latch) is
/// unit-testable with NO live gRPC server: this crate ships only the lightwalletd
/// CLIENT codegen (no server), and `tonic::Streaming` has no public test
/// constructor, so tests inject a scripted source (`testing::ScriptedSource`)
/// while production wraps the real `tonic::Streaming<T>` (the blanket impl below).
#[async_trait]
trait MessageSource<T>: Send {
    /// `Ok(Some(msg))` = next message · `Ok(None)` = clean end of stream ·
    /// `Err(status)` = a stream fault (transport death or server status).
    async fn message(&mut self) -> Result<Option<T>, tonic::Status>;
}

#[async_trait]
impl<T: Send + 'static> MessageSource<T> for tonic::Streaming<T> {
    async fn message(&mut self) -> Result<Option<T>, tonic::Status> {
        tonic::Streaming::message(self).await
    }
}

/// How a non-OK [`tonic::Status`] on THIS stream's RPC is classified. Keyed on the
/// streamed message type rather than carried as a field, so a `MessageStream` can
/// never be built with the wrong classifier and every construction site (including
/// the `testing` seams) gets the right one for free.
///
/// Two RPCs, two answers, and the difference is load-bearing: on `GetBlockRange`
/// every non-OK status is either transport death or a broken server, but on
/// `GetSubtreeRoots` "I do not know that shielded protocol" is a documented,
/// MEASURED server behaviour — and it arrives with a code that
/// [`classify_status`] would otherwise read as a flaky link (see
/// [`GrpcError::ShieldedProtocolUnknown`]). The refusal can land at stream OPEN or
/// on the first pull depending on whether the server sends headers before its
/// handler errors, so BOTH doors go through the protocol-aware classifier; the
/// probe recorded the code, not the door.
///
/// `pub(crate)`, not private: it sits in the `where`-bound of `pub(crate)`
/// [`MessageStream`]'s inherent impl, so `private_bounds` requires at-least-equal
/// visibility. An internal pump detail — never part of the public API.
pub(crate) trait StreamStatus {
    fn classify_stream_status(stall: StallReason, status: &tonic::Status) -> GrpcError;
}

impl StreamStatus for CompactBlock {
    fn classify_stream_status(stall: StallReason, status: &tonic::Status) -> GrpcError {
        classify_status(stall, status)
    }
}

impl StreamStatus for SubtreeRoot {
    fn classify_stream_status(stall: StallReason, status: &tonic::Status) -> GrpcError {
        classify_subtree_roots_status(stall, status)
    }
}

/// A live server-stream — [`BlockStream`] (inc-2c-iv-a block download) or
/// [`SubtreeRootStream`] (inc-2c-iv-d-1 commitment-tree ingestion). Generic over
/// the streamed message `T` so BOTH share ONE reliability pump (the DRY
/// generalization mandated by §3.2g — the 12 iv-a `BlockStream` tests lock its
/// behavior across this refactor). Contains the upstream `tonic::Streaming`
/// (never leaked past this `pub(crate)` seam — ADR-0005 r5) and bounds EACH
/// inter-message wait by [`GRPC_STREAMING_TIMEOUT_SECS`], so a connection that
/// wedges mid-stream — the most common flaky-mobile failure — surfaces a typed
/// `Timeout` instead of hanging forever (§3.3 "a dead stream is a silent hang").
///
/// LOAD-BEARING DEPENDENCY (money): the "a truncated stream is `Err`, never a
/// clean `Ok(None)`" property below is enforced by tonic, which maps a stream
/// closed WITHOUT a final `grpc-status` trailer to `Err(Unknown)`. tonic INVERTED
/// this between 0.14.5 (→ silent clean end) and 0.14.6 (→ `Err`), so the dep is
/// floored at `>=0.14.6` in `sdk/Cargo.toml`. A real-`Streaming` truncation guard
/// belongs to the integration tier (§8 E2E — `tonic::Streaming` has no unit-test
/// constructor); the unit tests here cover the wrapper's own arms.
///
/// SPENT-ON-ERROR (money defense-in-depth, principle 10): the `source` is an
/// `Option` so any `Err` from `next_message` drops the underlying stream and
/// latches the handle terminal — a re-poll then fails LOUD + retryable instead of
/// returning the spent `tonic::Streaming`'s silent `Ok(None)`. A clean `Ok(None)`
/// end does NOT latch (it stays idempotent). This makes consumer-contract #2 a
/// checked invariant, not just advice.
pub(crate) struct MessageStream<T> {
    /// `None` once the stream has faulted (terminal) — see SPENT-ON-ERROR above.
    source: Option<Box<dyn MessageSource<T>>>,
    stall: StallReason,
    /// The ADR-0552 stage 1b confirmation handle, carried from the client that
    /// opened this stream: every message that arrives is fresh evidence that
    /// the private circuit is carrying gRPC, so a long download keeps the
    /// patience window alive while bytes actually flow.
    witness: Option<PrivateRpcWitness>,
}

/// A live [`CompactBlock`] download (inc-2c-iv-a) — pull with `next_block`.
pub(crate) type BlockStream = MessageStream<CompactBlock>;
/// A live [`SubtreeRoot`] stream (inc-2c-iv-d-1) — pull with `next_root`.
pub(crate) type SubtreeRootStream = MessageStream<SubtreeRoot>;

impl<T: Send + 'static + StreamStatus> MessageStream<T> {
    /// Wrap a freshly-opened upstream stream (the only production construction
    /// site — `get_block_range` / `get_subtree_roots`).
    fn live(
        stream: tonic::Streaming<T>,
        stall: StallReason,
        witness: Option<PrivateRpcWitness>,
    ) -> Self {
        Self {
            source: Some(Box::new(stream)),
            stall,
            witness,
        }
    }

    /// Pull the next message, bounding the inter-message wait by
    /// [`GRPC_STREAMING_TIMEOUT_SECS`] (monotonic). `Ok(None)` = the stream ended
    /// cleanly. The domain methods `next_block` / `next_root` are thin aliases.
    ///
    /// CRITICAL (money-adjacent though no money logic): a mid-stream fault is a
    /// typed `Err`, NEVER `Ok(None)`. A dropped connection misreported as a clean
    /// end would make the consumer believe it saw every message and silently skip
    /// the rest — for [`BlockStream`] `scan_cached_blocks` would skip the range
    /// tail (a fund-detection gap); for [`SubtreeRootStream`] the shardtree would
    /// be missing completed roots (a truncated spend-witness anchor). The idle
    /// timeout and the status arm both return `Err`; only the genuine
    /// end-of-stream is `Ok(None)`.
    ///
    /// THREE CONSUMER CONTRACTS the sync engine MUST honor:
    /// 1. `Ok(None)` means the server closed the stream cleanly — it does NOT
    ///    assert the whole requested span (block range / subtree index range) was
    ///    delivered. A clean-but-SHORT end is a valid outcome (an honest tip move,
    ///    or a lying endpoint); the consumer must independently verify completeness
    ///    (this seam can't — it doesn't track the expected count). A short end
    ///    trusted as "complete" is the same silent-skip gap by another door.
    /// 2. Any `Err` is TERMINAL for this handle: drop it and re-establish via the
    ///    originating RPC (which also re-keys the circuit). Re-polling is now
    ///    fail-LOUD (a typed `Transport` error, never a silent `Ok(None)` — the
    ///    SPENT-ON-ERROR latch), but it still won't deliver the skipped messages, so
    ///    the consumer must re-establish, not retry on the dead handle.
    /// 3. CANCEL-SAFE: dropping a pending pull (the app backgrounded mid-await)
    ///    consumes no message and leaks no timer — `tonic` buffers received frames
    ///    in the `Streaming`, so resuming is just another pull on the same handle
    ///    (nothing lost, nothing duplicated).
    async fn next_message(&mut self) -> Result<Option<T>, GrpcError> {
        // The stream was OPENED over this connection (headers came back), so
        // its dial was accepted: a transport fault here is the far end's.
        let stall = transport_stall(self.stall, self.witness.as_ref());
        // A prior fault latched the handle terminal (source taken) — fail loud +
        // retryable, never the spent stream's silent clean end (contract #2).
        let source = self.source.as_mut().ok_or(GrpcError::Transport { stall })?;
        match stream_timeout(source.message()).await {
            Err(_elapsed) => {
                self.source = None; // spent: a stalled stream can't be resumed
                // A stream that stopped delivering mid-flight is the private
                // path failing to carry, and the pump is where that is seen.
                if let Some(witness) = &self.witness {
                    witness.note_failed();
                }
                Err(GrpcError::Timeout {
                    stall: TIMEOUT_STALL,
                })
            }
            // A clean end does NOT latch — re-polls stay idempotently `Ok(None)`.
            // Either way the link just delivered something the server sent, so
            // the private path is demonstrably carrying (ADR-0552 stage 1b).
            Ok(Ok(msg)) => {
                if let Some(witness) = &self.witness {
                    witness.note_carried();
                }
                Ok(msg)
            }
            Ok(Err(status)) => {
                self.source = None; // spent: a faulted stream must be re-established
                let err = T::classify_stream_status(stall, &status);
                // A stream that FAULTS mid-flight is the same evidence as one
                // that stops delivering: the arm above is not the only way a
                // live stream dies. A server that RESETs every stream produces
                // only this one.
                note_if_transport(&err, self.witness.as_ref());
                Err(err)
            }
        }
    }
}

impl MessageStream<CompactBlock> {
    /// Pull the next [`CompactBlock`] (the iv-a download arm). See the
    /// `next_message` pump for the three consumer contracts + the money property.
    pub(crate) async fn next_block(&mut self) -> Result<Option<CompactBlock>, GrpcError> {
        self.next_message().await
    }
}

impl MessageStream<SubtreeRoot> {
    /// Pull the next [`SubtreeRoot`] (the iv-d-1 commitment-tree ingestion). See
    /// the `next_message` pump for the three consumer contracts + the money property.
    pub(crate) async fn next_root(&mut self) -> Result<Option<SubtreeRoot>, GrpcError> {
        self.next_message().await
    }
}

fn timeout<F>(call: F) -> tokio::time::Timeout<F>
where
    F: Future,
{
    tokio::time::timeout(Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS), call)
}

/// Streaming-bound sibling of [`timeout`]: bounds stream ESTABLISHMENT and EACH
/// inter-block wait (NOT the whole download, which legitimately runs long — the
/// `GRPC_STREAMING_TIMEOUT_SECS` doc). The total-progress bound is the engine's
/// stuck-sync watchdog.
///
/// MONOTONIC by construction (`tokio::time` is `Instant`-based), so an NTP /
/// timezone jump on a phone that just regained signal never fires it (§7).
/// TRIPWIRE: keep this on `tokio::time` — do NOT derive the deadline from
/// `SystemTime`/`chrono` (a wall-clock deadline reintroduces clock-jump
/// sensitivity that no unit test would catch).
///
/// SUSPEND (desktop-lens S30): `std::time::Instant` does NOT advance during system
/// suspend on macOS (`mach_absolute_time`) and typical Linux (`CLOCK_MONOTONIC`), so
/// a laptop sleep mid-stream does not burn the idle budget — on resume the dead TCP
/// surfaces a fast typed `Transport` error. On unusual kernels where
/// `CLOCK_MONOTONIC` counts suspend, a long sleep could fire this spuriously on
/// resume: still benign (a typed `Timeout` ⇒ a re-sync retry, never fund loss),
/// the behavioral guarantee just holds on the supported platforms, not at the
/// language level.
fn stream_timeout<F>(call: F) -> tokio::time::Timeout<F>
where
    F: Future,
{
    tokio::time::timeout(Duration::from_secs(GRPC_STREAMING_TIMEOUT_SECS), call)
}

/// `witness` is the ADR-0552 stage 1b confirmation handle, and it is a
/// REQUIRED parameter rather than something a call site may opt into: a full
/// response is the evidence that the circuit carried gRPC, so every unary RPC
/// must report one. Taking it here means a future RPC that forgets does not
/// compile, instead of silently leaving the patience window resting on bare
/// connects — which is the defect stage 1b exists to repair.
fn map_unary<T>(
    stall: StallReason,
    witness: Option<&PrivateRpcWitness>,
    res: Result<Result<tonic::Response<T>, tonic::Status>, tokio::time::error::Elapsed>,
) -> Result<T, GrpcError> {
    match res {
        // The blackhole: the circuit accepted and delivered nothing. This is the
        // ONLY place that observes it, so it is where the failure clock starts.
        // The plan's `stall` is NOT what it carries: a timeout says nothing
        // about the path (`TIMEOUT_STALL`); the transport arm below does.
        Err(_elapsed) => {
            if let Some(witness) = witness {
                witness.note_failed();
            }
            Err(GrpcError::Timeout {
                stall: TIMEOUT_STALL,
            })
        }
        Ok(Ok(resp)) => {
            if let Some(witness) = witness {
                witness.note_carried();
            }
            Ok(resp.into_inner())
        }
        Ok(Err(status)) => {
            // The reason is the CONNECTION's to give, not the plan's: the
            // plan's fail-closed word belongs to a dial that failed, and an
            // accepted dial that then died is the far end (`transport_stall`).
            let err = classify_status(transport_stall(stall, witness), &status);
            note_if_transport(&err, witness);
            Err(err)
        }
    }
}

/// A TRANSPORT failure is the private path not carrying; a SERVER status is the
/// endpoint answering, which means the circuit worked and must not count
/// against it. [`classify_status`] is the discriminator; this is what the
/// witness is told about its verdict.
///
/// FOUR CALL SITES, and until the S1 `truth` fold it was open-coded at one.
/// The other three — both stream OPENS and the stream PUMP — classified the
/// same statuses and told the witness nothing, so the posture was blind on
/// three of its four surfaces. What that left invisible is not exotic: a
/// server that answers the pass-head unary and RESETS every stream. The unary
/// CONFIRMS and clears the failing run, each reset records nothing, the run
/// never reaches the minute, a `Preferred` wallet never switches and a
/// `Required` one never reads `Unanswered` — the state holds `Active` for ever
/// while the wallet never syncs. That is the silent stall under a UI reading
/// fine that the whole item exists to close, and the pump's own comment
/// ("the pump is where that is seen") was true only of its timeout arm.
fn note_if_transport(err: &GrpcError, witness: Option<&PrivateRpcWitness>) {
    if let (GrpcError::Transport { .. }, Some(witness)) = (err, witness) {
        witness.note_failed();
    }
}

/// Classify a non-OK [`tonic::Status`] as a transport failure (→ the policy stall)
/// vs a genuine server status (→ code only, §5.4 never the text).
///
/// TRANSPORT-ORIGIN DETECTION (the iv-a `status.source()` refinement): tonic builds
/// a server status from response trailers (`Status::from_header_map`, `source: None`),
/// whereas ANY transport-origin failure — a connection that dies MID-RPC, an h2
/// RST_STREAM/GOAWAY, a keepalive PING timeout — is constructed from a hyper/h2 error
/// and ALWAYS carries a `source`. So `status.source().is_some()` is the robust
/// discriminator the old code-enumeration missed: a dropped connection can surface as
/// `Cancelled`/`Internal`, not just `Unavailable`, and code-matching alone would
/// misfile it as a server status — the far end ANSWERING — so the witness would
/// read a dropped private connection as the path carrying, and the retry path a
/// dead link as a server's verdict. (`stall` arrives already resolved by
/// [`transport_stall`]: a Tor link that drops mid-RPC is over an accepted dial and
/// carries `EndpointUnreachable`; only a dial that failed carries the plan's
/// fail-closed reason.) A malicious endpoint cannot forge this: it controls only
/// the trailers (→ `source: None`), never our client's error chain.
/// The `Unavailable | Unknown` code arm stays as defense-in-depth for any transport
/// status that arrives WITHOUT a source (tonic usually re-attaches the originating
/// error as the source, but a status constructed directly as `Status::unavailable`
/// would not carry one) — it never regresses the cases code-matching already caught.
/// This refines ALL three call sites (unary + both server streams).
fn classify_status(stall: StallReason, status: &tonic::Status) -> GrpcError {
    use std::error::Error;
    use tonic::Code;
    // DEBUG-ONLY diagnostic (D-2b-2 on-device): the transport detail is dropped at
    // the §5.4 reduction below, which makes an on-device sync failure invisible.
    // In debug builds emit the gRPC code + the full transport source chain
    // (hyper/h2/io) so the debug logcat subscriber surfaces the real cause. §5.4-
    // safe: a transport-origin error carries no wallet data, and we deliberately
    // log the numeric code + source chain only — NEVER `status.message()`, which a
    // server can populate with data. Compiled out of release entirely.
    #[cfg(debug_assertions)]
    {
        let mut chain = String::new();
        let mut src = status.source();
        while let Some(e) = src {
            chain.push_str(" <- ");
            chain.push_str(&e.to_string());
            src = e.source();
        }
        tracing::warn!(
            target: "zec_wallet_core",
            code = ?status.code(),
            transport_chain = %chain,
            "grpc call failed (debug diagnostic)"
        );
    }
    // A transport-origin failure carries a source chain (hyper/h2/io); a server-sent
    // status does not. This catches the mid-RPC connection death the code arm cannot.
    if status.source().is_some() {
        return GrpcError::Transport { stall };
    }
    match status.code() {
        Code::Unavailable | Code::Unknown => GrpcError::Transport { stall },
        other => GrpcError::Status {
            code: i32::from(other),
        },
    }
}

/// [`classify_status`], made PROTOCOL-AWARE for the one RPC where an argument the
/// server does not recognize is a real, measured answer rather than a broken link:
/// `GetSubtreeRoots` (T0-1 A9).
///
/// **The carve-out, and why it is exactly this narrow — NARROWED by the
/// post-build crypto audit, which refuted the premise the first version rested
/// on.** That version read `source().is_none() && (Unknown | InvalidArgument)`,
/// justified by *"any transport-origin failure always carries a source"*. **That
/// is false for the tonic we pin**, and the counter-example is a transport fault
/// this wallet will actually meet. `tonic-0.14.6/src/status.rs:827-830` builds a
/// SOURCE-LESS `Code::Unknown` when a stream returns HTTP 200 with no
/// `grpc-status` trailer — its own message names the cause: *"stream was
/// terminated without a final status (possible truncation by a proxy or load
/// balancer)"* — and `:832` maps every HTTP status tonic does not recognise
/// (500, 502, a CDN's 520/522/526) to `Unknown` the same way. `Status::new` sets
/// `source: None` (`:172-181`), so `source()` cannot separate those from a real
/// server answer.
///
/// What that would have cost, stated because it is the exact failure T0-1
/// exists to end: a proxy truncating `GetSubtreeRoots` would have been read as
/// *"this server does not know Ironwood"*, the pool would have been marked
/// `Unsupported`, `fetch_subtree_roots` would have returned `Ok`, and the wallet
/// would have reported a healthy sync with `ironwood_tree_shards.subtree_end_height`
/// still NULL — INC-020's user-visible failure, money arriving and never
/// clearing, recreated through a door this repair opened.
///
/// So the carve-out is now `InvalidArgument` **with trailers present**. Trailers
/// are what a server puts on a status it composed; the two tonic-synthesised
/// cases above carry an empty `MetadataMap`. `Unknown` goes back to
/// `classify_status`'s transport arm, where it was before.
///
/// **The cost of that narrowing, and it is a real loss.** Measured 2026-09-08:
/// lightwalletd v0.5.3 answers `Unknown (2)` and v0.5.4 answers
/// `InvalidArgument (3)`. So **A9 detection is now scoped to v0.5.4 and later.**
/// Against a v0.5.3 server that does not know Ironwood, the refusal reads as a
/// transport fault and is retried — which is the honest reading, because on the
/// wire that answer is genuinely indistinguishable from a truncated stream, and
/// retrying a link fault is recoverable while silently declaring a money pool
/// unsupported is not. Closing that properly needs a POSITIVE capability signal
/// (`GetLightdInfo`), not a finer reading of an error code; owed, not faked here.
///
/// Three things this deliberately does NOT do. It does not touch
/// [`classify_status`], so `GetBlockRange`, the unary calls and every existing
/// classification test are unchanged. It does not include `Unavailable`, which
/// stays a transport code everywhere. And it does not claim the pool is EMPTY: a
/// refusal and an empty stream are different answers, and only the caller
/// ([`crate::sync::fetch_subtree_roots`]) decides whether a refusal is survivable
/// for the pool that got it.
///
/// The cost, stated: a source-less `Unknown` from `GetSubtreeRoots` that really
/// was a link fault now reads `EndpointUnreachable` instead of the policy stall,
/// and is no longer retried as transport. That is the trade the A9 ruling bought,
/// and it is the truthful reading — a status built from trailers means a server
/// answered us.
fn classify_subtree_roots_status(stall: StallReason, status: &tonic::Status) -> GrpcError {
    use std::error::Error;
    use tonic::Code;
    // `!metadata().is_empty()` is the discriminator that `source()` could not be:
    // a status a SERVER composed arrives with its response trailers, while the two
    // statuses tonic synthesises for a truncated or unmapped-HTTP stream are built
    // by `Status::new`/`Status::unknown` with an empty `MetadataMap`. `Unknown` is
    // deliberately absent — see the doc above for the transport cases that produce
    // it source-less, and for the v0.5.3 detection this gives up to stay honest.
    if status.source().is_none()
        && status.code() == Code::InvalidArgument
        && !status.metadata().is_empty()
    {
        return GrpcError::ShieldedProtocolUnknown {
            code: i32::from(status.code()),
        };
    }
    classify_status(stall, status)
}

/// Map a transport failure during a sync/provisioning step to the public
/// [`WalletError::Sync`] (§3.2f) — the host renders `TorUnavailable` (fail-closed
/// under `Required`) vs `EndpointUnreachable` from the carried [`StallReason`]. A
/// non-OK server status is treated as endpoint-unreachable (it responded but
/// cannot serve us). Lives here (where [`GrpcError`] is defined) so both
/// `provision` (the birthday step) and `sync` (root ingestion) map through ONE
/// door without a peer-module dependency between them.
///
/// **What this mapping says today, stated so the contradiction is on the
/// record rather than in a comment that pretends otherwise (T0-1c-R2 G7; the
/// wrap's security #7).** A server STATUS — `PermissionDenied` for a wrong or
/// absent endpoint key (measured live, §4k-run owed 8 / §4k-R-run owed 4), a
/// v0.5.3-shaped protocol refusal that arrives source-less — is the endpoint
/// ANSWERING: the link worked, and by the class sentence T0-1c wrote at
/// `sync_controller::stall_for` ("usable transport, unusable data" is the
/// content tier, `EndpointMisbehaving`, never "check your connection") it does
/// not belong under `EndpointUnreachable`. It is mapped there anyway, below,
/// which renders a refused credential as a dead link forever. The mapping is
/// NOT changed here: the honest reading needs a distinct arm (a credential the
/// server refused is neither a dead link nor a lying server, and "switch
/// servers" is the wrong remedy for a mis-set key) and a named row driving the
/// shipped client against a scripted `PermissionDenied` — owed to its own row
/// (§4k-R-run owed 4), where the reading is priced. Until then the comment on
/// the arm says what the code does, not what the class sentence wants.
pub(crate) fn transport_err(e: GrpcError) -> WalletError {
    let stall = match e {
        GrpcError::Transport { stall } | GrpcError::Timeout { stall } => stall,
        // Both are the endpoint ANSWERING with something we cannot use, so neither
        // may claim `TorUnavailable` — the link demonstrably worked. And for the
        // same reason neither should read `EndpointUnreachable` either; they do,
        // today, and the doc above says why that stands and where it is owed.
        GrpcError::Status { .. } | GrpcError::ShieldedProtocolUnknown { .. } => {
            StallReason::EndpointUnreachable
        }
    };
    WalletError::Sync { stall }
}

/// Split a validated endpoint into `(uri, host, port, is_https)` — parsed ONCE
/// (the `Uri` is reused to build the `Endpoint`). The endpoint passed the §2.3
/// validator (https anywhere, http only for loopback), so the scheme drives the
/// TLS decision and the default port.
fn endpoint_parts(endpoint: &LightServerEndpoint) -> Result<(Uri, String, u16, bool), NetError> {
    let uri: Uri = endpoint
        .as_str()
        .parse()
        .map_err(|_| NetError::InvalidEndpoint {
            reason: "unparseable url",
        })?;
    let is_https = uri.scheme_str() == Some("https");
    let raw_host = uri.host().ok_or(NetError::InvalidEndpoint {
        reason: "missing host",
    })?;
    // `http::Uri::host()` returns IPv6 literals bracketed (`[::1]`); strip them
    // for the dialer + SNI, mirroring the §2.3 validator's loopback handling
    // (config.rs) — an unstripped `[::1]` would fail name resolution / SNI.
    let host = raw_host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(raw_host)
        .to_owned();
    let port = uri.port_u16().unwrap_or(if is_https { 443 } else { 80 });
    Ok((uri, host, port, is_https))
}

/// rustls over the dialed stream, ALPN locked to `h2` (gRPC is HTTP/2). WebPKI
/// roots (Mozilla bundle — no platform cert-store assumption on mobile), `ring`
/// provider (the workspace's pinned TLS stack) — the SAME stack the swap adapter
/// uses, so there is ONE TLS path across sync + swap (§3.2a).
struct GrpcTls {
    connector: TlsConnector,
}

impl GrpcTls {
    fn new() -> Self {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            // unreachable (the rust-patterns `expect` exception): errs only for a
            // provider with NO default TLS version; ring ships 1.2 + 1.3, pinned on
            .expect("ring provider supports the default TLS versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = vec![b"h2".to_vec()];
        Self {
            connector: TlsConnector::from(Arc::new(config)),
        }
    }

    async fn wrap(
        &self,
        host: &str,
        stream: Box<dyn AsyncByteStream>,
    ) -> Result<Box<dyn AsyncByteStream>, std::io::Error> {
        let name = ServerName::try_from(host.to_owned())
            .map_err(|_| std::io::Error::other("endpoint host is not a valid TLS server name"))?;
        let tls = self.connector.connect(name, stream).await?;
        Ok(Box::new(tls))
    }
}

/// tonic connector that dials the HOST infrastructure (never tonic's own), then
/// optionally TLS-wraps, and hands tonic a hyper-ready duplex. tonic calls this
/// per connection with the endpoint `Uri`, which we ignore — host/port/isolation
/// are fixed at construction so a mismatched `Uri` can never redirect the dial.
#[derive(Clone)]
struct DialerConnector {
    dialer: Arc<dyn PlanDialer>,
    tls: Option<Arc<GrpcTls>>,
    host: String,
    port: u16,
    isolation_key: Option<String>,
    /// Where each dial's verdict is recorded for this channel's witness. tonic
    /// redials a DEAD connection through this same connector, so the arm is
    /// rewritten per dial and always describes the connection now in use.
    connection: Arc<ConnectionState>,
}

impl tower::Service<Uri> for DialerConnector {
    type Response = TokioIo<Box<dyn AsyncByteStream>>;
    type Error = std::io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _uri: Uri) -> Self::Future {
        let dialer = Arc::clone(&self.dialer);
        let tls = self.tls.clone();
        let host = self.host.clone();
        let port = self.port;
        let isolation_key = self.isolation_key.clone();
        let connection = Arc::clone(&self.connection);
        Box::pin(async move {
            // DialError is ERASED here (io::Error::other → ErrorKind::Other) and
            // tonic's transport error is opaque downstream — so the policy stall
            // reason carried on the client, not this error, is the only
            // failure-category info preserved across the tonic boundary. What
            // IS preserved is that the dial failed: the connection's verdict is
            // cleared, so the RPC that surfaces this reads the plan's
            // fail-closed reason and not the accept of a dial before it (tonic
            // redials a dead connection through this same connector).
            let dialed = match dialer
                .dial_attributed(&host, port, isolation_key.as_deref())
                .await
            {
                Ok(dialed) => dialed,
                Err(e) => {
                    connection.note_dial_failed();
                    return Err(std::io::Error::other(e));
                }
            };
            // Recorded BEFORE the handshake below: a peer that accepts and then
            // stalls TLS is the blackhole over `https`, and the RPC that fails
            // on it must find which arm it failed over — and that the dial was
            // ACCEPTED, which is what keeps that failure the far end's
            // (`transport_stall`).
            connection.set_served_by(dialed.served_by);
            let stream = dialed.stream;
            let stream: Box<dyn AsyncByteStream> = match &tls {
                // The dial above is bounded by DIAL_TIMEOUT_SECS inside the
                // dialer, but the TLS handshake it hands us was UNBOUNDED — on a
                // lossy link (observed live on-device) a wedged handshake
                // then rides the FIRST RPC's timeout instead: 100 s when that RPC
                // is streaming (every sync pass opens with the subtree-roots
                // stream), vs the ~30 s connect posture. Give the handshake phase
                // its own DIAL_TIMEOUT_SECS budget so a link that dies after the
                // SYN fails in connect time, not stream time. Worst-case connect
                // is now dial + handshake, one bound each, typical unchanged.
                //
                // EXCEPT on a `Preferred` fallback leg, where TLS is not a leg
                // of its own: the dialer hands over the deadline its clearnet
                // connect was already held to, and the handshake gets what is
                // LEFT of it. A fresh bound here would put primary + connect +
                // handshake past the caller's unary budget, and the switch
                // would be reachable and useless (`FALLBACK_ESTABLISH_BUDGET_SECS`).
                Some(t) => {
                    let handshake_by = dialed.establish_by.unwrap_or_else(|| {
                        tokio::time::Instant::now() + Duration::from_secs(DIAL_TIMEOUT_SECS)
                    });
                    tokio::time::timeout_at(handshake_by, t.wrap(&host, stream))
                        .await
                        .map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "TLS handshake timed out",
                            )
                        })??
                }
                None => stream,
            };
            Ok(TokioIo::new(stream))
        })
    }
}

/// Test-only scripted message sources — no live gRPC server (the crate ships only
/// the lightwalletd client codegen). Lives at module scope (not inside `mod
/// tests`) and is `pub(crate)` so the sync-engine tests can build a scripted
/// [`SubtreeRootStream`]: the `MessageStream` fields are private to this module,
/// so another module cannot construct one without this seam.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::collections::VecDeque;

    /// A scripted source: yields its programmed results in order, then a clean end
    /// (`Ok(None)`) once exhausted. Generic over `T` so it drives both
    /// [`BlockStream`] and [`SubtreeRootStream`] through the one pump.
    pub(crate) struct ScriptedSource<T> {
        items: VecDeque<Result<Option<T>, tonic::Status>>,
    }

    impl<T> ScriptedSource<T> {
        pub(crate) fn new<I>(items: I) -> Self
        where
            I: IntoIterator<Item = Result<Option<T>, tonic::Status>>,
        {
            Self {
                items: items.into_iter().collect(),
            }
        }
    }

    #[async_trait]
    impl<T: Send + 'static> MessageSource<T> for ScriptedSource<T> {
        async fn message(&mut self) -> Result<Option<T>, tonic::Status> {
            self.items.pop_front().unwrap_or(Ok(None))
        }
    }

    /// Build a [`MessageStream`] over a scripted source (the cross-module test
    /// seam). Stall defaults to `EndpointUnreachable`; a faulted item still
    /// carries the per-message latch/timeout semantics of the production pump.
    pub(crate) fn scripted_stream<T: Send + 'static + StreamStatus>(
        items: impl IntoIterator<Item = Result<Option<T>, tonic::Status>>,
    ) -> MessageStream<T> {
        MessageStream {
            source: Some(Box::new(ScriptedSource::new(items))),
            stall: StallReason::EndpointUnreachable,
            // No posture: these seams test the pump's own semantics, not the
            // patience window.
            witness: None,
        }
    }

    /// A source that yields a clone of `item` FOREVER (never a clean end, never an
    /// error) — models a hostile endpoint flooding the stream, for testing a
    /// consumer's count cap without materializing a giant `Vec`.
    struct EndlessSource<T: Clone + Send> {
        item: T,
    }

    #[async_trait]
    impl<T: Clone + Send + 'static> MessageSource<T> for EndlessSource<T> {
        async fn message(&mut self) -> Result<Option<T>, tonic::Status> {
            Ok(Some(self.item.clone()))
        }
    }

    /// A [`MessageStream`] that never ends (a flooding endpoint) — the consumer's
    /// own ceiling must stop it.
    pub(crate) fn endless_stream<T: Clone + Send + 'static + StreamStatus>(
        item: T,
    ) -> MessageStream<T> {
        MessageStream {
            source: Some(Box::new(EndlessSource { item })),
            stall: StallReason::EndpointUnreachable,
            witness: None,
        }
    }
}

// The implementer's drives of the patience window's consumer (stage S1): a
// child module, so they reach this file's private types; their own file, so
// this one's cited test lines stay where their watches printed them.
#[cfg(test)]
#[path = "grpc_window_tests.rs"]
mod window_tests;

// The same, for stage S1 `truth`: the not-carrying state's predicate and
// wakes, the F-C reading under `Required`, the counters' one writer.
#[cfg(test)]
#[path = "grpc_truth_tests.rs"]
mod truth_tests;

#[cfg(test)]
mod tests {
    use super::testing::ScriptedSource;
    use super::*;
    use crate::config::TorRuntime;
    use crate::ports::NetDialer;
    use crate::sync::SubtreeRootSource;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// LIVE regression for the on-device sync stall (D-2b-2): the Tor-off sync RPCs
    /// against a real lightwalletd MUST succeed, including draining `get_subtree_roots`
    /// — the call that previously failed with `z_getsubtreesbyindex … Invalid params`
    /// because the SDK passed `max_entries = 1<<16` (one past zcashd's accepted
    /// `limit`). The `SubtreeRootSource` impl now passes the proto's `0` ("all
    /// entries") sentinel; this pins that the stream actually drains roots. Ignored
    /// by default (live network); override the endpoint with `ZEC_WALLET_LWD`. A gated
    /// endpoint takes its credential from `ZEC_WALLET_LWD_KEY_HEADER` +
    /// `ZEC_WALLET_LWD_KEY` (both or neither — the config door's rule), sent through
    /// the shipped [`EndpointAuth`] interceptor, so this is the SDK-level check that
    /// the credential path works against a real proxy (DevOps's lightwalletd).
    /// Run:
    ///   cargo test -p zec-wallet-core --features swap live_sync_rpcs_succeed -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "live network: hits a real lightwalletd endpoint"]
    async fn live_sync_rpcs_succeed() {
        let url =
            std::env::var("ZEC_WALLET_LWD").unwrap_or_else(|_| "https://zec.rocks:443".into());
        let endpoint = LightServerEndpoint::new(url.clone()).expect("valid endpoint");
        let auth = match (
            std::env::var("ZEC_WALLET_LWD_KEY_HEADER"),
            std::env::var("ZEC_WALLET_LWD_KEY"),
        ) {
            (Ok(h), Ok(v)) => Some(EndpointAuth::new(h, v).expect("valid auth pair")),
            (Err(_), Err(_)) => None,
            _ => panic!("ZEC_WALLET_LWD_KEY_HEADER and ZEC_WALLET_LWD_KEY: both or neither"),
        };
        println!(
            "live: {url} auth_header={:?}",
            auth.as_ref().map(|a| a.header().as_str())
        );
        let fell_back = Arc::new(TorPosture::new());
        let mut client = LightwalletdClient::connect(
            &endpoint,
            &TorPolicy::Off,
            None,
            &fell_back,
            &Default::default(),
            auth.as_ref(),
        )
        .expect("connect builds lazily");

        let started = std::time::Instant::now();
        let info = client.get_lightd_info().await.expect("get_lightd_info");
        println!(
            "get_lightd_info: {:?} {:?} chain={:?} tip={} in {:.0} ms",
            info.vendor,
            info.version,
            info.chain_name,
            info.block_height,
            started.elapsed().as_secs_f64() * 1000.0
        );
        let started = std::time::Instant::now();
        let latest = client.get_latest_block().await.expect("get_latest_block");
        println!(
            "get_latest_block: height={} in {:.0} ms",
            latest.height,
            started.elapsed().as_secs_f64() * 1000.0
        );

        // The fix: drain `subtree_roots` via the real `SubtreeRootSource` path (which
        // now sends `max_entries = 0`). Before the fix this errored with a server
        // `InvalidArgument` on the first message pull. Ironwood joined with T0-1
        // (wire value 2); a v0.5.4 endpoint serves it.
        for proto in [
            ShieldedProtocol::Sapling,
            ShieldedProtocol::Orchard,
            ShieldedProtocol::Ironwood,
        ] {
            let mut stream = client
                .subtree_roots(proto, 0)
                .await
                .expect("subtree_roots opens");
            let mut roots = 0u32;
            while let Some(_root) = stream
                .next_root()
                .await
                .expect("subtree stream drains cleanly")
            {
                roots += 1;
                if roots >= 2 {
                    break; // a couple proves it streams; full drain isn't the point
                }
            }
            assert!(roots > 0, "{proto:?} returned at least one subtree root");
        }
    }

    /// FR-29 spec §4 / §8 T13: a REGISTERED dialer cannot influence TLS. The
    /// connector's TLS is built by `GrpcTls::new()` — NULLARY, pinned at the
    /// type level below — from WebPKI roots, the `ring` provider, ALPN `h2`
    /// and no client auth, chosen by the endpoint's SCHEME alone
    /// (`is_https.then(...)`): no plan, no policy, no descriptor and no
    /// registry reaches the builder. Pinned at the source, on this file, so a
    /// future `GrpcTls::new_for(&plan)` or a plan-gated `tls:` line fails
    /// here; and behaviourally: a `Required { HostDialer }` client and an
    /// `Off` client for the same https endpoint both build (lazily, no dial)
    /// through the one `connect_unchecked`, differing ONLY in their policy
    /// stall — the TLS half of the connector is the same call in both (a lazy
    /// channel is built inside a runtime — hence `tokio::test`).
    #[tokio::test]
    async fn a_registered_dialer_cannot_influence_tls_config() {
        use crate::net::host_dialer::HostDialer;
        use crate::net::host_dialer::testing::ScriptedHostDialer;
        // Nullary by TYPE: the builder takes nothing a registrant could supply.
        let build: fn() -> GrpcTls = GrpcTls::new;
        let _ = build();

        // The source pin: the ONE tls line in the channel build, verbatim, and
        // a builder body that names none of the policy-side inputs.
        let src = include_str!("grpc.rs");
        let connect = src
            // Split like the caller-count pin below: this literal must not
            // itself count as a caller of the unchecked build.
            .find(concat!("fn connect_", "unchecked("))
            .expect("connect_unchecked is the channel build");
        let body = &src[connect..];
        let end = body.find("\n    }\n").expect("the fn body closes");
        let body = &body[..end];
        let tls_lines: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("tls:"))
            .collect();
        assert_eq!(
            tls_lines,
            vec!["tls: is_https.then(|| Arc::new(GrpcTls::new())),"],
            "the connector's TLS is decided by the scheme alone and built nullary"
        );
        let builder = src.find("impl GrpcTls {").expect("the TLS builder impl");
        let builder_end = src[builder..].find("\n}\n").expect("the impl closes");
        let builder = &src[builder..builder + builder_end];
        for forbidden in [
            "plan",
            "policy",
            "descriptor",
            "HostDialer",
            "registry",
            "TorPolicy",
            "TorRuntime",
        ] {
            assert!(
                !builder.contains(forbidden),
                "the TLS builder must not read `{forbidden}` — TLS is SDK-owned, never a registrant's"
            );
        }
        assert!(
            builder.contains("webpki_roots::TLS_SERVER_ROOTS")
                && builder.contains("with_no_client_auth()")
                && builder.contains("alpn_protocols = vec![b\"h2\".to_vec()]"),
            "WebPKI roots, no client auth, ALPN h2 — the SDK's TLS, whole"
        );

        // Behaviour: both policies build a client through the same path for the
        // same https endpoint; only the stall reason differs.
        let endpoint = LightServerEndpoint::new("https://zec.rocks:443").expect("tls endpoint");
        let host = Arc::new(ScriptedHostDialer::ready());
        let required = TorPolicy::Required {
            runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
        };
        let flag = Arc::new(TorPosture::new());
        let counts = Arc::new(DialCounters::default());
        let over_host =
            LightwalletdClient::connect(&endpoint, &required, None, &flag, &counts, None)
                .expect("Required + a registered host dialer builds lazily");
        assert_eq!(over_host.stall_on_failure, StallReason::TorUnavailable);
        let off =
            LightwalletdClient::connect(&endpoint, &TorPolicy::Off, None, &flag, &counts, None)
                .expect("Off builds lazily");
        assert_eq!(off.stall_on_failure, StallReason::EndpointUnreachable);
        assert!(
            host.keys_seen().is_empty(),
            "building the channel dials nothing — TLS setup reached no registrant"
        );
    }

    /// FR-5 spec §8 **P28, the behavioural half** (the source half is
    /// `extraction_policy.rs::the_sdk_tls_stack_has_exactly_two_explicit_builder_sites`).
    /// The SDK's TLS never reads the PROCESS-DEFAULT `CryptoProvider`. That
    /// default is a process-global that whichever crate in the host binary gets
    /// there first installs — arti reads it for its TLS to guards (the
    /// `dialer-tor` manifest says so), so once the plugin lands, a host that
    /// installed a provider owns arti's TLS. The wallet's own TLS must not be
    /// in that set: `GrpcTls::new` passes `ring` explicitly.
    ///
    /// A FOREIGN provider is installed as the process default first — `ring`'s
    /// with every cipher suite removed, so it is both distinguishable (no
    /// suites) and unusable (a builder that read it would fail to build). The
    /// builder must still stand up, and the config it built must carry `ring`'s
    /// full suite list. The test asserts its own premise too: if something
    /// installed a default before it, the foreign one is not in effect and the
    /// check would prove nothing, so that is a failure, not a skip.
    ///
    /// Watched red: the explicit `builder_with_provider(provider)` chain in
    /// `GrpcTls::new` replaced by `rustls::ClientConfig::builder()` (the
    /// process-default form).
    #[test]
    fn the_sdk_tls_never_reads_the_process_default_provider() {
        let mut foreign = rustls::crypto::ring::default_provider();
        foreign.cipher_suites.clear();
        let _ = foreign.install_default(); // PERMANENT for this test binary (no uninstall): safe only while nothing in src/** builds TLS from the default — P28's source scan in extraction_policy.rs refuses that
        let installed = rustls::crypto::CryptoProvider::get_default()
            .expect("a process-default provider is installed");
        assert!(
            installed.cipher_suites.is_empty(),
            "P28 premise: the process default is not this test's FOREIGN provider (it has \
             {} suites) — something installed a default first, so this binary cannot tell \
             an explicit provider from the default. Find the installer.",
            installed.cipher_suites.len()
        );

        let built = std::panic::catch_unwind(GrpcTls::new);
        let tls = built.unwrap_or_else(|_| {
            panic!(
                "P28: GrpcTls::new PANICKED with a foreign process-default provider installed \
                 — the builder read the process default instead of passing `ring` explicitly"
            )
        });
        let suites: Vec<_> = tls
            .connector
            .config()
            .crypto_provider()
            .cipher_suites
            .iter()
            .map(|s| s.suite())
            .collect();
        let ring: Vec<_> = rustls::crypto::ring::default_provider()
            .cipher_suites
            .iter()
            .map(|s| s.suite())
            .collect();
        assert!(!ring.is_empty(), "ring ships cipher suites");
        assert_eq!(
            suites, ring,
            "P28: the sync TLS config was not built from the explicit `ring` provider — its \
             suites are the process default's"
        );
    }

    /// Records every dial; returns an in-memory duplex whose peer is dropped, so
    /// the gRPC handshake fails FAST (no real server, no hang) while the routing
    /// is proven by the recorded dial.
    struct RecordingDialer {
        calls: Mutex<Vec<(String, u16, Option<String>)>>,
    }

    impl RecordingDialer {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
            })
        }
        fn calls(&self) -> Vec<(String, u16, Option<String>)> {
            self.calls.lock().expect("poisoned").clone()
        }
    }

    /// A peer that CONNECTS instantly then never speaks — the on-device
    /// wedge shape (SYN/accept fine on a lossy link; the TLS exchange stalls).
    /// The peer half is retained so the stream never EOFs.
    struct SilentDialer {
        held_peers: Mutex<Vec<tokio::io::DuplexStream>>,
    }

    impl SilentDialer {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                held_peers: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl NetDialer for SilentDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            let (a, b) = tokio::io::duplex(64);
            self.held_peers.lock().expect("poisoned").push(b);
            Ok(Box::new(a))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_wedged_tls_handshake_fails_in_connect_time_not_stream_time() {
        // observed live on-device: TCP connects instantly on a lossy link,
        // then the TLS handshake stalls forever. Pre-fix, that wedge rode the
        // FIRST RPC's 100 s streaming timeout on every sync attempt (the lazy
        // channel makes the first RPC carry dial + TLS); the connector now gives
        // the handshake phase its own DIAL_TIMEOUT_SECS budget.
        let dialer = SilentDialer::new();
        let mut connector = DialerConnector {
            // The connector takes a PLAN's dialer; `fail_closed` is the plain
            // wrapper (the private arm, no fallback, so no fallback budget).
            dialer: Arc::new(crate::net::dialer::PolicyDialer::fail_closed(
                dialer as Arc<dyn NetDialer>,
                Arc::new(TorPosture::new()),
                Default::default(),
            )),
            tls: Some(Arc::new(GrpcTls::new())),
            host: "example.com".into(),
            port: 443,
            isolation_key: None,
            connection: Arc::default(),
        };
        let started = tokio::time::Instant::now();
        let res =
            tower::Service::call(&mut connector, Uri::from_static("https://example.com")).await;
        let waited = started.elapsed();
        let err = match res {
            Ok(_) => panic!("a silent peer must be cut off, never hang"),
            Err(e) => e,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
        // Paused clock: the elapsed time IS the budget that fired. The posture
        // this pins: connect-phase failures cost connect time (30 s), not the
        // 100 s streaming budget.
        assert_eq!(waited, Duration::from_secs(DIAL_TIMEOUT_SECS));
        const { assert!(DIAL_TIMEOUT_SECS < GRPC_STREAMING_TIMEOUT_SECS) };
    }

    #[async_trait]
    impl NetDialer for RecordingDialer {
        async fn dial(
            &self,
            host: &str,
            port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            self.calls.lock().expect("poisoned").push((
                host.to_owned(),
                port,
                isolation_key.map(str::to_owned),
            ));
            let (a, b) = tokio::io::duplex(64);
            drop(b); // peer gone → the stream EOFs immediately
            Ok(Box::new(a))
        }
    }

    /// A transport-origin error whose `source()` is an inner `Status`. Fed through
    /// `tonic::Status::from_error` it yields a Status that KEEPS the inner gRPC code
    /// AND attaches a source chain — exactly how a mid-RPC connection death surfaces
    /// (the h2/hyper error becomes the Status `source`, see tonic's
    /// `find_status_in_source_chain`). Lets us build a Status with a NON-transport code
    /// (e.g. Cancelled) that `classify_status` must still read as Transport.
    #[derive(Debug)]
    struct TransportOrigin(tonic::Status);

    impl std::fmt::Display for TransportOrigin {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "transport error")
        }
    }

    impl std::error::Error for TransportOrigin {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn classify_status_reads_a_mid_rpc_drop_as_transport_not_a_server_status() {
        use std::error::Error;
        use tonic::Code;
        // The unstable-network case the old code-enumeration missed: a connection that
        // dies MID-RPC surfaces with a NON-transport gRPC code (Cancelled from an h2
        // RST_STREAM, Internal from a GOAWAY) yet ALWAYS carries a source. The source
        // check must read it as Transport carrying the stall it was GIVEN, verbatim —
        // never a server status. Which stall that is, is resolved before this function
        // by `transport_stall` (stage S1 `truth`: over an accepted dial a mid-RPC drop
        // carries `EndpointUnreachable`; the plan's `TorUnavailable` here stands for
        // "whatever the caller resolved"). Same code, different source ⇒ different class.
        for code in [Code::Cancelled, Code::Internal, Code::NotFound] {
            let sourced = tonic::Status::from_error(Box::new(TransportOrigin(tonic::Status::new(
                code, "rst",
            ))));
            assert!(
                sourced.source().is_some(),
                "fixture carries a transport source"
            );
            assert_eq!(sourced.code(), code, "fixture keeps the inner gRPC code");
            assert!(
                matches!(
                    classify_status(StallReason::TorUnavailable, &sourced),
                    GrpcError::Transport {
                        stall: StallReason::TorUnavailable
                    }
                ),
                "a sourced {code:?} is a transport failure carrying the policy stall",
            );
        }
    }

    #[test]
    fn classify_status_keeps_a_genuine_server_status_as_status() {
        use std::error::Error;
        use tonic::Code;
        // A server-sent status (from response trailers) has NO source — the endpoint
        // RESPONDED with an error, not a transport failure. It stays a Status (code
        // only, §5.4) → the retry path treats it as endpoint-unreachable, never as the
        // fail-closed Tor stall. NotFound appears here AND above: identical code, but a
        // source-less one is a server status while a sourced one is transport.
        for code in [
            Code::NotFound,
            Code::InvalidArgument,
            Code::PermissionDenied,
        ] {
            let server = tonic::Status::new(code, "server says no");
            assert!(server.source().is_none(), "a plain status has no source");
            assert!(
                matches!(
                    classify_status(StallReason::TorUnavailable, &server),
                    GrpcError::Status { code: c } if c == i32::from(code)
                ),
                "a source-less {code:?} is a server status (code only)",
            );
        }
    }

    #[test]
    fn classify_status_code_arm_still_catches_source_less_transport() {
        use std::error::Error;
        use tonic::Code;
        // Defense-in-depth: a hyper keepalive PING timeout becomes `Status::unavailable`
        // with NO source. The Unavailable|Unknown code arm must still catch it as
        // Transport, so the source refinement never REGRESSES the cases it already handled.
        for code in [Code::Unavailable, Code::Unknown] {
            let s = tonic::Status::new(code, "x");
            assert!(s.source().is_none());
            assert!(matches!(
                classify_status(StallReason::EndpointUnreachable, &s),
                GrpcError::Transport { .. }
            ));
        }
    }

    fn status_has_source(s: &tonic::Status) -> bool {
        use std::error::Error;
        s.source().is_some()
    }

    /// The realistic non-OK gRPC codes (classify_status's precondition is a non-OK status).
    fn arb_code() -> impl proptest::strategy::Strategy<Value = tonic::Code> {
        use proptest::prelude::Just;
        use tonic::Code;
        proptest::prop_oneof![
            Just(Code::Cancelled),
            Just(Code::Unknown),
            Just(Code::InvalidArgument),
            Just(Code::DeadlineExceeded),
            Just(Code::NotFound),
            Just(Code::AlreadyExists),
            Just(Code::PermissionDenied),
            Just(Code::ResourceExhausted),
            Just(Code::FailedPrecondition),
            Just(Code::Aborted),
            Just(Code::OutOfRange),
            Just(Code::Unimplemented),
            Just(Code::Internal),
            Just(Code::Unavailable),
            Just(Code::DataLoss),
            Just(Code::Unauthenticated),
        ]
    }

    fn arb_stall() -> impl proptest::strategy::Strategy<Value = StallReason> {
        use proptest::prelude::Just;
        proptest::prop_oneof![
            Just(StallReason::EndpointUnreachable),
            Just(StallReason::TorUnavailable),
            Just(StallReason::StorageFull),
            Just(StallReason::ChainReorg),
            Just(StallReason::Internal),
        ]
    }

    #[test]
    fn auth_interceptor_sends_the_header_and_marks_it_sensitive() {
        use tonic::service::Interceptor;

        // this is the assertion that the FEATURE works. Everything else
        // about endpoint auth is validation — if the interceptor does not put the
        // header on the request, a key-gated endpoint refuses every call and the
        // config that looked right was never sent.
        let auth = EndpointAuth::new("x-zcash-rpc-key", "abc123").expect("valid pair");
        let mut interceptor = AuthInterceptor { auth: Some(auth) };
        let req = interceptor
            .call(tonic::Request::new(()))
            .expect("the interceptor must not refuse a validated pair");

        let sent = req
            .metadata()
            .get("x-zcash-rpc-key")
            .expect("the auth header reaches the request");
        assert_eq!(sent.to_str().expect("ascii"), "abc123");
        assert!(
            sent.is_sensitive(),
            "the value must be marked sensitive, or HPACK indexes the secret into \
             the connection's compression table and replays it from there"
        );
    }

    #[tokio::test]
    async fn connect_accepts_an_auth_config_and_builds() {
        // code review: every other `connect` call in this module passes
        // `None`, and every `Some` test builds `AuthInterceptor` by hand — so the
        // one production line that threads auth INTO the client (`auth.cloned()`
        // in `connect`) was reached by nothing. This covers that line.
        //
        // What it does NOT do, stated rather than implied: the built client is
        // opaque, so this cannot observe the header on the wire. The injection
        // behaviour is covered by `auth_interceptor_sends_the_header_and_marks_it_sensitive`;
        // this asserts only that the wiring compiles and builds a client.
        let endpoint = LightServerEndpoint::new("https://zec.rocks:443").expect("endpoint");
        let auth = EndpointAuth::new("x-zcash-rpc-key", "abc123").expect("valid pair");
        let fell_back = Arc::new(TorPosture::new());
        assert!(
            LightwalletdClient::connect(
                &endpoint,
                &TorPolicy::Off,
                None,
                &fell_back,
                &Default::default(),
                Some(&auth),
            )
            .is_ok(),
            "an authenticated client must build lazily, same as an unauthenticated one"
        );
    }

    #[test]
    fn no_auth_configured_sends_no_header_at_all() {
        use tonic::service::Interceptor;

        // The other half, and it is not decoration: the interceptor is installed
        // UNCONDITIONALLY (one client type), so the `None` path is what every
        // public endpoint takes. An interceptor that injected an empty or
        // placeholder header would send a credential-shaped value to every
        // public server on the network.
        let mut interceptor = AuthInterceptor { auth: None };
        let req = interceptor
            .call(tonic::Request::new(()))
            .expect("no auth is not an error");
        assert!(
            req.metadata().is_empty(),
            "an unauthenticated client must add nothing: {:?}",
            req.metadata()
        );
    }

    proptest::proptest! {
        /// Over the WHOLE (code × sourced? × stall) space, classify_status keys ONLY on
        /// source-presence then the code, and carries the policy `stall` VERBATIM:
        /// - sourced ⇒ Transport{stall} (the EXACT input stall — a hostile endpoint
        ///   controls only the code/trailers, so it can never forge nor alter the stall);
        /// - else Unavailable|Unknown ⇒ Transport{stall};
        /// - else ⇒ Status{code} (a server status, NEVER the fail-closed Tor stall).
        /// MONEY-safety: the refinement can only ESCALATE to Transport, never downgrade a
        /// real transport failure to a server Status (which would mask a Tor outage).
        #[test]
        fn classify_status_keys_on_source_then_code_and_carries_stall_verbatim(
            code in arb_code(),
            has_source in proptest::prelude::any::<bool>(),
            stall in arb_stall(),
        ) {
            let status = if has_source {
                // A transport-origin status keeps its inner code AND carries a source.
                tonic::Status::from_error(Box::new(TransportOrigin(tonic::Status::new(code, "x"))))
            } else {
                tonic::Status::new(code, "x")
            };
            let sourced = status_has_source(&status);
            let is_transport_code = matches!(code, tonic::Code::Unavailable | tonic::Code::Unknown);

            match classify_status(stall, &status) {
                GrpcError::Transport { stall: carried } => {
                    proptest::prop_assert_eq!(carried, stall); // verbatim, un-forgeable
                    proptest::prop_assert!(sourced || is_transport_code); // justified
                }
                GrpcError::Status { code: c } => {
                    proptest::prop_assert!(!sourced);          // a server status has no source
                    proptest::prop_assert!(!is_transport_code); // ...and not a transport code
                    proptest::prop_assert_eq!(c, i32::from(code));
                }
                GrpcError::Timeout { .. } => {
                    proptest::prop_assert!(false, "classify_status never yields Timeout")
                }
                GrpcError::ShieldedProtocolUnknown { .. } => {
                    // The protocol-aware carve-out lives in
                    // `classify_subtree_roots_status`, NOT here: `classify_status`
                    // itself is unchanged by T0-1, and every OTHER call site (the
                    // unary calls, `get_block_range`, the block stream) must keep
                    // reading a source-less `Unknown` as transport.
                    proptest::prop_assert!(
                        false,
                        "classify_status never yields ShieldedProtocolUnknown"
                    )
                }
            }
        }
    }

    #[test]
    fn loopback_http_skips_tls_remote_https_wraps_tls() {
        // §8 `loopback_http_skips_tls_remote_https_wraps_tls` (decision half): the
        // scheme drives the TLS choice + default port. https ⇒ TLS, 443 default;
        // loopback http ⇒ plaintext, explicit port.
        let https =
            LightServerEndpoint::new("https://zec.rocks:443").expect("https endpoint valid");
        let (_, host, port, is_https) = endpoint_parts(&https).expect("parts");
        assert_eq!((host.as_str(), port, is_https), ("zec.rocks", 443, true));

        let https_default =
            LightServerEndpoint::new("https://mainnet.example").expect("https no port");
        let (_, _, port, is_https) = endpoint_parts(&https_default).expect("parts");
        assert!(is_https, "https scheme ⇒ TLS");
        assert_eq!(port, 443, "https default port");

        let loopback = LightServerEndpoint::new("http://127.0.0.1:9067").expect("loopback http");
        let (_, host, port, is_https) = endpoint_parts(&loopback).expect("parts");
        assert_eq!((host.as_str(), port, is_https), ("127.0.0.1", 9067, false));

        // IPv6 loopback: the §2.3 validator blesses `http://[::1]`, so the host
        // must be UN-bracketed for the dialer/SNI (not "[::1]").
        let v6 = LightServerEndpoint::new("http://[::1]:9067").expect("ipv6 loopback http");
        let (_, host, port, is_https) = endpoint_parts(&v6).expect("parts");
        assert_eq!((host.as_str(), port, is_https), ("::1", 9067, false));
    }

    #[tokio::test]
    async fn grpc_channel_rides_the_host_dialer_not_a_direct_socket() {
        // §8 `grpc_channel_rides_the_host_dialer_not_a_direct_socket`: the RPC
        // reaches the network ONLY through the host's NetDialer (the ADR-0526
        // contract). A loopback http endpoint isolates the dial+h2 path (no TLS);
        // the dead duplex makes the handshake fail typed, never hang/panic.
        let dialer = RecordingDialer::new();
        let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::clone(&dialer) as Arc<dyn NetDialer>),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some("sync".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");

        let r = client.get_lightd_info().await;
        assert!(
            matches!(
                r,
                Err(GrpcError::Transport { .. }) | Err(GrpcError::Timeout { .. })
            ),
            "no server ⇒ a typed transport/timeout error, never a hang or panic"
        );
        let calls = dialer.calls();
        assert_eq!(calls.len(), 1, "exactly one dial, through the host dialer");
        assert_eq!(
            calls[0],
            ("localhost".to_owned(), 9067, Some("sync".to_owned())),
            "dialed the endpoint host:port with the sync isolation key — no direct socket"
        );
    }

    #[test]
    fn unsupported_runtime_refuses_before_any_channel() {
        // ExternalSocks5 has no dialer yet (§3.2a) — `connect` refuses typed at
        // resolution, never builds a channel that could leak to clearnet.
        let endpoint = LightServerEndpoint::new("https://zec.rocks:443").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::ExternalSocks5 {
                addr: "127.0.0.1:9050".into(),
            },
        };
        assert!(matches!(
            LightwalletdClient::connect(
                &endpoint,
                &policy,
                None,
                &Arc::new(TorPosture::new()),
                &Default::default(),
                None
            ),
            Err(NetError::UnsupportedRuntime { .. })
        ));
    }

    /// FR-29 stage 0 (ii) / ADR-0544 decision 3, the RUNTIME half: a plaintext
    /// loopback endpoint that reaches `connect` under a host dialer (the picker's
    /// switch path — the door never saw it) is refused before any channel exists;
    /// the same endpoint under `Off` builds its (lazy) channel as before (a
    /// lazy channel still needs a runtime to be built in — hence `tokio::test`).
    /// Also pins that the unchecked channel build has exactly its two callers —
    /// the production `connect` and the `cfg(test)` plaintext seam — so no
    /// production path can grow around the rule.
    #[tokio::test]
    async fn http_endpoint_over_a_host_dialer_is_refused_at_connect() {
        use crate::ports::testing::StubDialer;
        let loopback = LightServerEndpoint::new("http://127.0.0.1:9067").expect("loopback");
        let host_dialer = || Arc::new(StubDialer) as Arc<dyn NetDialer>;
        let required = TorPolicy::Required {
            runtime: TorRuntime::Dialer(host_dialer()),
        };
        let preferred = TorPolicy::Preferred {
            runtime: TorRuntime::Dialer(host_dialer()),
        };
        let fell_back = Arc::new(TorPosture::new());
        let counts = Arc::new(DialCounters::default());
        for policy in [&required, &preferred] {
            assert!(
                matches!(
                    LightwalletdClient::connect(&loopback, policy, None, &fell_back, &counts, None),
                    Err(NetError::InvalidEndpoint { reason }) if reason.contains("TorPolicy::Off")
                ),
                "plaintext through a host dialer is refused at connect (Preferred dials the host first)"
            );
        }
        assert!(
            !fell_back.fell_back(),
            "a refusal before any dial never writes the fell-back latch"
        );
        assert!(
            LightwalletdClient::connect(
                &loopback,
                &TorPolicy::Off,
                None,
                &fell_back,
                &counts,
                None
            )
            .is_ok(),
            "the SDK's own direct dialer still carries the loopback development endpoint"
        );
        // The rule cannot be grown around: the unchecked build is called from
        // exactly two places in this file — `connect` and the test seam. A
        // TEXTUAL invariant (the code reviewer's note): an ordinary new caller
        // bumps the count and fails loudly; a function-pointer alias would not —
        // `connect_unchecked` is module-private, so that alias would have to be
        // written in this file, under this test.
        let source = include_str!("grpc.rs");
        let needle = concat!("connect_unchecked", "(");
        assert_eq!(
            source.matches(needle).count(),
            3,
            "one definition + two callers of the unchecked channel build"
        );
    }

    // ── streaming block-download seam (inc-2c-iv-a) ──────────────────────────
    //
    // The generated lightwalletd SERVER is not in this crate (only the client),
    // and `tonic::Streaming` has no public test constructor, so the stream pump
    // is driven through a scripted `CompactBlockSource` (the seam built for
    // exactly this) — deterministic, no live gRPC server, no real network.

    // `ScriptedSource<CompactBlock>` (the in-order-then-clean-end source) is shared
    // from `super::testing` so the sync-engine tests reuse it for `SubtreeRoot`.

    /// A source whose next message NEVER resolves — models a connection wedged
    /// mid-stream (blackholed, dead circuit). The per-message idle timeout must
    /// turn this into a typed `Timeout`, never an indefinite park.
    struct PendingSource;

    #[async_trait]
    impl MessageSource<CompactBlock> for PendingSource {
        async fn message(&mut self) -> Result<Option<CompactBlock>, tonic::Status> {
            std::future::pending().await
        }
    }

    fn stream_from(src: impl MessageSource<CompactBlock> + 'static) -> BlockStream {
        BlockStream {
            source: Some(Box::new(src)),
            stall: StallReason::EndpointUnreachable,
            witness: None,
        }
    }

    fn block_at(height: u64) -> CompactBlock {
        CompactBlock {
            height,
            ..Default::default()
        }
    }

    /// A source that sleeps `delay` BEFORE delivering each scripted item (so it is
    /// cancel-safe: a future dropped mid-sleep has consumed nothing). Exhausted ⇒
    /// clean end. Drives the slow-but-alive-link and drop-resume tests under a
    /// virtual clock.
    struct DelayedSource {
        delay: Duration,
        items: VecDeque<Result<Option<CompactBlock>, tonic::Status>>,
    }

    impl DelayedSource {
        fn new<I>(delay: Duration, items: I) -> Self
        where
            I: IntoIterator<Item = Result<Option<CompactBlock>, tonic::Status>>,
        {
            Self {
                delay,
                items: items.into_iter().collect(),
            }
        }
    }

    #[async_trait]
    impl MessageSource<CompactBlock> for DelayedSource {
        async fn message(&mut self) -> Result<Option<CompactBlock>, tonic::Status> {
            if self.items.is_empty() {
                return Ok(None);
            }
            // Sleep BEFORE popping: if the awaiting future is cancelled mid-sleep,
            // the item stays queued — nothing is consumed (cancel-safety).
            tokio::time::sleep(self.delay).await;
            self.items.pop_front().unwrap_or(Ok(None))
        }
    }

    /// Yields `good` blocks, then NEVER resolves — models a radio drop AFTER
    /// progress (the commonest flaky-mobile failure: the stream goes silent
    /// part-way, not at open).
    struct StallAfterSource {
        good: u64,
        next_height: u64,
    }

    #[async_trait]
    impl MessageSource<CompactBlock> for StallAfterSource {
        async fn message(&mut self) -> Result<Option<CompactBlock>, tonic::Status> {
            if self.good == 0 {
                return std::future::pending().await;
            }
            self.good -= 1;
            let h = self.next_height;
            self.next_height += 1;
            Ok(Some(block_at(h)))
        }
    }

    #[tokio::test]
    async fn get_block_range_streams_compact_blocks_in_order() {
        // §8 `get_block_range_streams_compact_blocks_in_order`: blocks arrive in
        // order, then a single clean end, and the end is idempotent (a re-poll of
        // an exhausted stream stays `Ok(None)`, never a spurious block or error).
        let mut s = stream_from(ScriptedSource::new([
            Ok(Some(block_at(200))),
            Ok(Some(block_at(201))),
            Ok(Some(block_at(202))),
        ]));
        let mut heights = Vec::new();
        while let Some(b) = s.next_block().await.expect("no stream error") {
            heights.push(b.height);
        }
        assert_eq!(heights, vec![200, 201, 202], "in order, then clean end");
        assert!(
            s.next_block().await.expect("still ok").is_none(),
            "end of stream is idempotent"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn block_stream_idle_timeout_fires_at_the_streaming_bound() {
        // §8 `block_stream_idle_timeout_fires_at_the_streaming_bound` (gate 7 — the
        // constant pinned at its boundary): a stream that goes silent mid-download
        // must surface a typed `Timeout` no earlier than GRPC_STREAMING_TIMEOUT_SECS,
        // never hang. (start_paused: tokio auto-advances the virtual clock to the
        // deadline — no real 100 s wait.)
        let mut s = stream_from(PendingSource);
        let start = tokio::time::Instant::now();
        let r = s.next_block().await;
        let elapsed = start.elapsed();
        assert!(
            matches!(r, Err(GrpcError::Timeout { .. })),
            "a wedged stream times out typed, never hangs"
        );
        assert!(
            elapsed >= Duration::from_secs(GRPC_STREAMING_TIMEOUT_SECS),
            "idle timeout fires no earlier than the streaming bound"
        );
    }

    #[tokio::test]
    async fn block_stream_mid_stream_failure_is_typed_never_a_silent_end() {
        // §8 `block_stream_mid_stream_failure_is_typed_never_a_silent_end` (the
        // load-bearing reliability property): a connection that dies MID-stream
        // surfaces a typed `Err`, NEVER `Ok(None)`. A clean-end misreport would let
        // `scan_cached_blocks` believe it saw the whole range and skip the rest —
        // a silent fund-detection gap.
        // `Internal` is a server-reported status (not a connection-layer death) —
        // it exercises the `GrpcError::Status` arm; the `Unavailable` case below
        // exercises the distinct `GrpcError::Transport` arm. Both must be `Err`.
        let mut s = stream_from(ScriptedSource::new([
            Ok(Some(block_at(100))),
            Err(tonic::Status::internal("server internal error")),
        ]));
        assert_eq!(
            s.next_block()
                .await
                .expect("first ok")
                .expect("a block")
                .height,
            100
        );
        assert!(
            matches!(s.next_block().await, Err(GrpcError::Status { .. })),
            "mid-stream server status is a typed Err, never a silent Ok(None)"
        );

        // A transport-origin death (`Unavailable`) classifies to `Transport` — the
        // stall the engine renders — and is still an `Err`, never a clean end.
        let mut s2 = stream_from(ScriptedSource::new([Err(tonic::Status::unavailable(
            "transport gone",
        ))]));
        assert!(matches!(
            s2.next_block().await,
            Err(GrpcError::Transport { .. })
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn slow_but_alive_link_resets_idle_timeout_per_message() {
        // §8 `slow_but_alive_link_resets_idle_timeout_per_message`: a Tor link
        // delivering one block every (bound − 1)s for FAR longer than the bound must
        // NOT be killed. The idle timeout is PER-MESSAGE and re-arms each call, so a
        // slow-but-progressing stream survives — there is no cumulative/total cap
        // (that bound is the engine's stuck-watchdog, iv-d). A regression that
        // hoisted one deadline over the whole download would fail here.
        let count = 5u64;
        let per_block = Duration::from_secs(GRPC_STREAMING_TIMEOUT_SECS - 1);
        let mut s = stream_from(DelayedSource::new(
            per_block,
            (0..count).map(|i| Ok(Some(block_at(500 + i)))),
        ));
        let start = tokio::time::Instant::now();
        let mut got = 0u64;
        while let Some(_b) = s
            .next_block()
            .await
            .expect("a slow-but-alive link must not be killed mid-download")
        {
            got += 1;
        }
        assert_eq!(got, count, "every block of a slow link arrives");
        assert!(
            start.elapsed() >= per_block * count as u32,
            "ran past the per-message bound many times over — no total cap"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn idle_timeout_bites_after_progress_not_only_at_stream_open() {
        // §8 `idle_timeout_bites_after_progress_not_only_at_stream_open`: a radio
        // drop AFTER 5 good blocks (the commonest flaky-mobile failure — the stream
        // goes silent part-way). The per-message timeout re-arms on EVERY call, so a
        // mid-stream stall still surfaces a typed `Timeout`; a bug arming the timer
        // only at stream-open would hang here forever.
        let mut s = stream_from(StallAfterSource {
            good: 5,
            next_height: 700,
        });
        for expected in 700..705 {
            assert_eq!(
                s.next_block()
                    .await
                    .expect("good block")
                    .expect("some")
                    .height,
                expected
            );
        }
        let start = tokio::time::Instant::now();
        assert!(
            matches!(s.next_block().await, Err(GrpcError::Timeout { .. })),
            "a stall after progress times out typed, never hangs"
        );
        assert!(
            start.elapsed() >= Duration::from_secs(GRPC_STREAMING_TIMEOUT_SECS),
            "the mid-stream idle timeout fires fresh at the bound"
        );
    }

    #[tokio::test]
    async fn empty_range_is_a_clean_end_not_an_error() {
        // §8 `empty_range_is_a_clean_end_not_an_error`: a quiet tip-following poll
        // (zero new blocks) is an immediate, idempotent clean end — never an error
        // that would freeze balance updates on every quiet poll.
        let mut s = stream_from(ScriptedSource::new([]));
        assert!(
            s.next_block()
                .await
                .expect("empty range is clean")
                .is_none()
        );
        assert!(
            s.next_block().await.expect("still clean").is_none(),
            "empty end is idempotent"
        );
    }

    #[tokio::test]
    async fn single_block_range_yields_exactly_one_then_clean_end() {
        // §8 `single_block_range_yields_exactly_one_then_clean_end`: the tip-follow
        // hot path — exactly one block then a clean end, no off-by-one. (The
        // inclusive-vs-half-open range conversion is the consumer's job, iv-d.)
        let mut s = stream_from(ScriptedSource::new([Ok(Some(block_at(900)))]));
        assert_eq!(
            s.next_block().await.expect("ok").expect("one block").height,
            900
        );
        assert!(
            s.next_block().await.expect("ok").is_none(),
            "exactly one block, then clean end"
        );
    }

    #[tokio::test]
    async fn re_poll_after_error_is_loud_not_a_silent_clean_end() {
        // §8 `re_poll_after_error_is_loud_not_a_silent_clean_end` (the SPENT-ON-ERROR
        // money latch, principle 10): after any `Err` the handle is terminal — a
        // re-poll is a typed `Transport` error (loud, retryable), NEVER the spent
        // stream's silent `Ok(None)` that the scanner would read as "range complete".
        let mut s = stream_from(ScriptedSource::new([
            Ok(Some(block_at(100))),
            Err(tonic::Status::unavailable("transport gone")),
        ]));
        assert_eq!(
            s.next_block().await.expect("ok").expect("block").height,
            100
        );
        assert!(
            matches!(s.next_block().await, Err(GrpcError::Transport { .. })),
            "the fault itself is typed"
        );
        assert!(
            matches!(s.next_block().await, Err(GrpcError::Transport { .. })),
            "the re-poll stays loud, never a phantom Ok(None)"
        );
    }

    #[tokio::test]
    async fn clean_end_re_poll_stays_idempotent_ok_none() {
        // §8 `clean_end_re_poll_stays_idempotent_ok_none`: the latch must NOT fire on
        // a clean end — only an `Err` latches terminal — so a clean `Ok(None)` stays
        // idempotent (tip-following re-polls keep returning a clean end, not an Err).
        let mut s = stream_from(ScriptedSource::new([Ok(Some(block_at(1)))]));
        assert!(s.next_block().await.expect("ok").is_some());
        assert!(s.next_block().await.expect("ok").is_none());
        assert!(
            s.next_block().await.expect("ok").is_none(),
            "clean-end re-poll stays Ok(None), not latched to Err"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn dropped_next_block_consumes_no_block_then_resumes() {
        // §8 `dropped_next_block_consumes_no_block_then_resumes` (cancel-safety,
        // contract #3): the app backgrounds mid-`next_block`, dropping the future.
        // The wrapper must consume NO block — the resuming call still delivers the
        // first block, in order, nothing eaten or duplicated.
        let per_block = Duration::from_secs(GRPC_STREAMING_TIMEOUT_SECS - 1);
        let mut s = stream_from(DelayedSource::new(
            per_block,
            [Ok(Some(block_at(200))), Ok(Some(block_at(201)))],
        ));
        // Cancel a pending next_block before its delay elapses (backgrounded).
        let cancelled = tokio::time::timeout(Duration::from_secs(1), s.next_block()).await;
        assert!(
            cancelled.is_err(),
            "the next_block was dropped mid-await (app backgrounded)"
        );
        // Resume: the FIRST block is still delivered — the dropped poll ate nothing.
        assert_eq!(
            s.next_block().await.expect("ok").expect("block").height,
            200
        );
        assert_eq!(
            s.next_block().await.expect("ok").expect("block").height,
            201
        );
    }

    #[tokio::test]
    async fn get_block_range_streams_through_the_host_dialer_not_a_direct_socket() {
        // §8 `get_block_range_streams_through_the_host_dialer_not_a_direct_socket`:
        // the STREAMING RPC reaches the network ONLY through the host's NetDialer
        // (the ADR-0526 contract, same as the unary seam). The dead duplex makes
        // stream establishment fail TYPED, never hang/panic.
        let dialer = RecordingDialer::new();
        let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::clone(&dialer) as Arc<dyn NetDialer>),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some("sync".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");

        let r = client.get_block_range(280_000, 280_010).await;
        assert!(
            matches!(
                r,
                Err(GrpcError::Transport { .. }) | Err(GrpcError::Timeout { .. })
            ),
            "no server ⇒ a typed transport/timeout error establishing the stream, never a hang"
        );
        let calls = dialer.calls();
        assert_eq!(calls.len(), 1, "exactly one dial, through the host dialer");
        assert_eq!(
            calls[0],
            ("localhost".to_owned(), 9067, Some("sync".to_owned())),
            "dialed the endpoint host:port with the sync isolation key — no direct socket"
        );
    }

    fn root_at(height: u64, hash: Vec<u8>) -> SubtreeRoot {
        SubtreeRoot {
            root_hash: hash,
            completing_block_hash: Vec::new(),
            completing_block_height: height,
        }
    }

    #[tokio::test]
    async fn subtree_root_stream_yields_roots_in_order_then_clean_end() {
        // §8 `subtree_root_stream_yields_roots_in_order_then_clean_end`: the generic
        // pump drives a `SubtreeRootStream` exactly like `BlockStream` — the iv-a
        // generalization is behaviour-preserving for the new T (roots arrive in
        // order, then ONE idempotent clean end). Built through the cross-module
        // `scripted_stream` seam (the same one the sync-engine tests use).
        let mut s = testing::scripted_stream([
            Ok(Some(root_at(1_900_000, vec![0xab; 32]))),
            Ok(Some(root_at(1_950_000, vec![0xcd; 32]))),
        ]);
        let mut heights = Vec::new();
        while let Some(r) = s.next_root().await.expect("no stream error") {
            heights.push(r.completing_block_height);
        }
        assert_eq!(
            heights,
            vec![1_900_000, 1_950_000],
            "in order, then clean end"
        );
        assert!(
            s.next_root().await.expect("still ok").is_none(),
            "subtree-root clean end is idempotent"
        );
    }

    #[tokio::test]
    async fn subtree_root_stream_mid_stream_failure_is_typed_never_a_silent_end() {
        // §8 `subtree_root_stream_mid_stream_failure_is_typed_never_a_silent_end`:
        // the money property holds for the SUBTREE-ROOT stream too — a mid-stream
        // fault is a typed `Err` (and latches the handle terminal, contract #2),
        // NEVER `Ok(None)`. A clean-end misreport would truncate the spend-witness
        // tree (a silent missing-funds-at-spend gap, the iv-d-1 analogue of the
        // block-stream fund-detection gap).
        let mut s = testing::scripted_stream([
            Ok(Some(root_at(1_900_000, vec![0x11; 32]))),
            Err(tonic::Status::unavailable("transport gone")),
        ]);
        assert_eq!(
            s.next_root()
                .await
                .expect("first ok")
                .expect("a root")
                .completing_block_height,
            1_900_000
        );
        assert!(
            matches!(s.next_root().await, Err(GrpcError::Transport { .. })),
            "mid-stream fault is a typed Err, never a silent Ok(None)"
        );
        assert!(
            matches!(s.next_root().await, Err(GrpcError::Transport { .. })),
            "the re-poll stays loud (SPENT-ON-ERROR latch), never a phantom clean end"
        );
    }

    #[tokio::test]
    async fn get_subtree_roots_streams_through_the_host_dialer_not_a_direct_socket() {
        // §8 `get_subtree_roots_streams_through_the_host_dialer_not_a_direct_socket`:
        // the new STREAMING RPC reaches the network ONLY through the host's NetDialer
        // (the ADR-0526 contract, same as the block-range + unary seams). The dead
        // duplex makes stream establishment fail TYPED, never hang/panic.
        let dialer = RecordingDialer::new();
        let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::clone(&dialer) as Arc<dyn NetDialer>),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some("sync".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");

        let r = client
            .get_subtree_roots(ShieldedProtocol::Sapling, 0, 0)
            .await;
        assert!(
            matches!(
                r,
                Err(GrpcError::Transport { .. }) | Err(GrpcError::Timeout { .. })
            ),
            "no server ⇒ a typed transport/timeout error establishing the stream, never a hang"
        );
        let calls = dialer.calls();
        assert_eq!(calls.len(), 1, "exactly one dial, through the host dialer");
        assert_eq!(
            calls[0],
            ("localhost".to_owned(), 9067, Some("sync".to_owned())),
            "dialed the endpoint host:port with the sync isolation key — no direct socket"
        );
    }

    /// A peer that completes the dial (returns a live stream) but never speaks
    /// gRPC: it DRAINS the client's bytes into the void and sends nothing back — a
    /// DPI/censor middlebox that accepts the connection then stalls. Because the
    /// dial SUCCEEDED, `DIAL_TIMEOUT_SECS` does NOT fire; only the establishment
    /// streaming timeout can catch it.
    struct SilentPeerDialer;

    #[async_trait]
    impl NetDialer for SilentPeerDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            use tokio::io::AsyncReadExt;
            // 8 KiB so the client's h2 preface is accepted without backpressure.
            let (client_end, mut server_end) = tokio::io::duplex(8192);
            tokio::spawn(async move {
                // Drain forever; never write — the client parks awaiting SETTINGS
                // /HEADERS that never come.
                let mut buf = [0u8; 1024];
                while let Ok(n) = server_end.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                }
            });
            Ok(Box::new(client_end))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn silent_accepting_server_establishment_times_out() {
        // §8 `silent_accepting_server_establishment_times_out`: a censor/DPI
        // middlebox completes the dial+handshake but never sends the gRPC response
        // headers. The 30 s DIAL_TIMEOUT_SECS does NOT fire (the dial succeeded);
        // ONLY the establishment streaming bound catches it. Without that bound the
        // sync worker would hang forever on a silent-accepting endpoint. (Virtual
        // clock: tokio auto-advances to the 100 s deadline — no real wait.)
        let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some("sync".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");

        let start = tokio::time::Instant::now();
        let r = client.get_block_range(280_000, 280_010).await;
        let elapsed = start.elapsed();
        assert!(
            matches!(r, Err(GrpcError::Timeout { .. })),
            "a silent-accepting server is caught by the establishment timeout, never a hang"
        );
        assert!(
            elapsed >= Duration::from_secs(GRPC_STREAMING_TIMEOUT_SECS),
            "the establishment bound fired — distinct from the faster dial-failure path"
        );
    }

    // ── the patience window's CONSUMER (stage S1 `window`) ───────────────────
    //
    // `docs/plan/stage-1-private-path-truth.md` §3.1. The posture tests prove
    // the PREDICATE turns true on an accepted-and-silent path; these prove that
    // something ACTS on it. Which arm dialled is read off the `wallet.dial`
    // capture and the wallet latch — never a mock's call count (§3.1 assertion
    // 8) — and none of them touches `spend_the_minute_for_test`, which would
    // hide any new consumer behind the one predicate it short-circuits (§3.1
    // THE FIXTURE TRAP).

    /// One `wallet.dial` line, with the instant it was written (virtual, under a
    /// paused clock) and the wallet latch AS IT READ at that instant.
    #[derive(Clone, Debug)]
    struct DialLine {
        at: Duration,
        arm: String,
        class: String,
        outcome: String,
        latched: bool,
    }

    /// The `wallet.dial` capture. Its own layer rather than `CaptureLayer`
    /// because two of §3.1's assertions are about WHEN: no clearnet line before
    /// the minute, and the latch set AT the clearnet dial rather than at the
    /// deadline. Those need the clock and the latch read where the line is
    /// written, which a field sink cannot give.
    #[derive(Clone)]
    struct DialLines {
        started: tokio::time::Instant,
        posture: Arc<TorPosture>,
        lines: Arc<Mutex<Vec<DialLine>>>,
    }

    impl DialLines {
        fn new(posture: &Arc<TorPosture>) -> Self {
            Self {
                started: tokio::time::Instant::now(),
                posture: Arc::clone(posture),
                lines: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn all(&self) -> Vec<DialLine> {
            self.lines.lock().expect("dial lines poisoned").clone()
        }

        fn of_arm(&self, arm: &str) -> Vec<DialLine> {
            self.all().into_iter().filter(|l| l.arm == arm).collect()
        }
    }

    #[derive(Default)]
    struct DialFields {
        message: String,
        arm: String,
        class: String,
        outcome: String,
    }

    impl tracing::field::Visit for DialFields {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = format!("{value:?}");
            }
        }
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            match field.name() {
                "dial_arm" => self.arm = value.to_owned(),
                "dial_class" => self.class = value.to_owned(),
                "outcome" => self.outcome = value.to_owned(),
                _ => {}
            }
        }
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for DialLines {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut fields = DialFields::default();
            event.record(&mut fields);
            // exactly the attribution line — not the debug `wallet.dial.connected`
            if fields.message != "wallet.dial" {
                return;
            }
            self.lines
                .lock()
                .expect("dial lines poisoned")
                .push(DialLine {
                    at: self.started.elapsed(),
                    arm: fields.arm,
                    class: fields.class,
                    outcome: fields.outcome,
                    latched: self.posture.fell_back(),
                });
        }
    }

    /// Install the capture for this test's thread. The guard must outlive every
    /// dial the test drives.
    fn capture_dial_lines(
        posture: &Arc<TorPosture>,
    ) -> (DialLines, tracing::subscriber::DefaultGuard) {
        use tracing_subscriber::layer::SubscriberExt;
        crate::tracing_guard::force_wallet_callsites_enabled();
        let lines = DialLines::new(posture);
        let guard =
            tracing::subscriber::set_default(tracing_subscriber::registry().with(lines.clone()));
        (lines, guard)
    }

    /// A loopback port with NO listener — where §3.1 aims the REAL clearnet
    /// dialer, because `resolve_dialer` hard-wires `direct()` as the `Preferred`
    /// fallback and no seam hands a client a prebuilt plan. Bound, read, dropped;
    /// the reuse caveat is `direct_dialer_maps_refused_connect_to_io`'s.
    async fn dead_loopback_endpoint() -> LightServerEndpoint {
        let port = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind loopback");
            listener.local_addr().expect("addr").port()
        };
        LightServerEndpoint::new(format!("http://127.0.0.1:{port}")).expect("loopback endpoint")
    }

    /// Hold a PAUSED clock to small steps while a REAL socket is in flight.
    ///
    /// A paused runtime that goes idle leaps to its next timer. With a refused
    /// loopback connect pending, that leap can land on a dial bound and turn
    /// `io` into `timeout` — or cancel the clearnet dial before it writes its
    /// line at all. A timer that is always 10 ms away keeps every idle step
    /// short, and the I/O driver is polled between steps, so the refusal is seen
    /// within a few virtual milliseconds. Nothing in-memory is perturbed: every
    /// other timer still fires at its own instant.
    fn hold_the_paused_clock_to_small_steps() {
        tokio::spawn(async {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }

    fn preferred_over(primary: Arc<dyn NetDialer>) -> TorPolicy {
        TorPolicy::Preferred {
            runtime: TorRuntime::Dialer(primary),
        }
    }

    /// The watch a driver keeps while NO clearnet line is on record. Two things
    /// can be wrong at that instant: the latch is already set (a switch announced
    /// at a deadline rather than at the dial that made it), or the window read
    /// spent a while ago and the wallet is still where it was — the red this
    /// item exists to turn green.
    struct Insisting {
        class: PathClass,
        spent_seen_at: Option<Duration>,
        attempts: usize,
    }

    impl Insisting {
        /// Once the window reads spent, the wallet has this long to be seen on
        /// clearnet. Two unary budgets: room for a mechanism that ends the silent
        /// connection during one attempt and redials on the next, and nothing
        /// like what a wallet waits today (for ever).
        const GRACE: Duration = Duration::from_secs(2 * GRPC_UNARY_TIMEOUT_SECS);

        fn on(class: PathClass) -> Self {
            Self {
                class,
                spent_seen_at: None,
                attempts: 0,
            }
        }

        fn before_the_next_attempt(
            &mut self,
            lines: &DialLines,
            posture: &TorPosture,
            still: &str,
        ) {
            let now = lines.started.elapsed();
            assert!(
                !posture.fell_back(),
                "latched at {now:?} with NO clearnet dial on record — the latch is set where \
                 the clearnet dial is made, never at a deadline"
            );
            if self.spent_seen_at.is_none() && posture.may_switch_to_direct(self.class) {
                self.spent_seen_at = Some(now);
            }
            if let Some(spent_at) = self.spent_seen_at {
                assert!(
                    now < spent_at + Self::GRACE,
                    "the {:?} window read SPENT at {spent_at:?}; it is now {now:?}, {} attempts \
                     in, and {still}: {} private dial(s), 0 clearnet dials. Nothing consumes \
                     `may_switch_to_direct` unless a dial FAILS, and an accepted connection never \
                     fails to dial (F-A)",
                    self.class,
                    self.attempts,
                    lines.of_arm("host").len()
                );
            }
            self.attempts += 1;
            assert!(
                self.attempts <= 64,
                "64 attempts and the {:?} window never read spent — the failure clock never \
                 started on an accepted-and-silent path, which is a different defect from F-A",
                self.class
            );
        }
    }

    /// Sync passes, as far as the network is concerned: the next RPC on the SAME
    /// client, again and again, until a clearnet dial is on record.
    async fn pass_until_the_wallet_leaves(
        client: &mut LightwalletdClient,
        lines: &DialLines,
        posture: &TorPosture,
    ) -> DialLine {
        let mut insisting = Insisting::on(PathClass::Sync);
        loop {
            if let Some(first) = lines.of_arm("sdk_direct").first() {
                return first.clone();
            }
            insisting.before_the_next_attempt(
                lines,
                posture,
                "the ONE reused client is still riding the accepted-and-silent private connection",
            );
            let _ = client.get_lightd_info().await;
        }
    }

    /// Sends, as `Wallet::broadcast_one` makes them: a FRESH client on a fresh
    /// per-send key every time, all sharing the wallet's one posture.
    async fn send_until_the_wallet_leaves(
        endpoint: &LightServerEndpoint,
        policy: &TorPolicy,
        lines: &DialLines,
        posture: &Arc<TorPosture>,
    ) -> DialLine {
        use crate::constants::BROADCAST_ISOLATION_KEY_PREFIX;
        let mut insisting = Insisting::on(PathClass::Broadcast);
        loop {
            if let Some(first) = lines.of_arm("sdk_direct").first() {
                return first.clone();
            }
            insisting.before_the_next_attempt(
                lines,
                posture,
                "every fresh per-send client still dials the accepted-and-silent private path",
            );
            let key = format!(
                "{BROADCAST_ISOLATION_KEY_PREFIX}-{:016x}",
                insisting.attempts
            );
            let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
                endpoint,
                policy,
                Some(key),
                posture,
                None,
            )
            .expect("lazy connect never blocks");
            let _ = client.send_transaction(vec![0x05, 0x00, 0x00, 0x80]).await;
        }
    }

    /// What the capture must say at the end of the attempt that switched.
    fn assert_one_visible_switch(
        lines: &DialLines,
        posture: &TorPosture,
        first_clearnet: &DialLine,
        class: PathClass,
        class_name: &str,
    ) {
        assert_eq!(
            first_clearnet.class, class_name,
            "the class that was silent is the class that left"
        );
        assert!(
            posture.may_switch_to_direct(class),
            "a clearnet dial while the {class:?} window reads UNSPENT is a leak, whatever made it"
        );
        let all = lines.all();
        assert_eq!(
            all.iter().filter(|l| l.arm == "sdk_direct").count(),
            1,
            "exactly one clearnet dial by the end of the attempt that switched: {all:?}"
        );
        assert!(
            first_clearnet.latched && posture.fell_back(),
            "the switch is VISIBLE: latched by the time the clearnet dial's line is written"
        );
        assert!(
            all.iter()
                .take_while(|l| l.arm != "sdk_direct")
                .all(|l| !l.latched),
            "every dial BEFORE the clearnet one ran unlatched — the latch belongs to the \
             clearnet dial, not to the deadline: {all:?}"
        );
    }

    /// §3.1 ASSERTION 1 — TIMING. F-A, both halves, through the shape the wallet
    /// actually has: ONE `LightwalletdClient`, built once and reused for every
    /// pass (the engine's cached slot, `wallet.rs` `LightdSyncEngine::client`),
    /// over a private path that ACCEPTS the connection and never answers.
    ///
    /// No fresh client is minted here and `dial()` is never called by hand: the
    /// only thing this test does after building the client is what a sync pass
    /// does — issue the next RPC on it. If the wallet is to leave the private
    /// path, the client and the dialer under it have to do that by themselves.
    ///
    /// The clearnet line's OUTCOME is deliberately unasserted: the fallback is
    /// the real `DirectTcpDialer` aimed at a port with no listener, and
    /// `a_spent_window_over_an_accepting_silent_path_connects_over_clearnet`
    /// reads `connected` on a real clock.
    #[tokio::test(start_paused = true)]
    async fn an_accepted_and_silent_private_path_is_left_at_the_minute_by_the_one_reused_client() {
        use crate::constants::{TOR_PATIENCE_SECS, WALLET_SYNC_ISOLATION_KEY};

        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>);
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
            &posture,
            None,
        )
        .expect("lazy connect never blocks");

        let first_clearnet = pass_until_the_wallet_leaves(&mut client, &lines, &posture).await;
        assert!(
            first_clearnet.at >= Duration::from_secs(TOR_PATIENCE_SECS),
            "a clearnet dial at {:?} — INSIDE the founder's minute",
            first_clearnet.at
        );
        assert_one_visible_switch(&lines, &posture, &first_clearnet, PathClass::Sync, "sync");
    }

    // An ANSWERING peer. `SilentPeerDialer` is the path that carries nothing; the
    // rows below also need one that carries — a private path that recovers, a
    // slow stream, a sibling connection, a clearnet server that completes the RPC
    // the user was waiting on. This crate ships the lightwalletd CLIENT codegen
    // only, so the peer speaks exactly as much HTTP/2 as a tonic client needs:
    // the server preface, SETTINGS and PING acknowledgements, and per request a
    // response whose every message is EMPTY (five zero bytes — a valid encoding
    // of any protobuf message's defaults, so one peer serves every RPC). Request
    // headers are never decoded, and responses use only static-table and
    // never-indexed HPACK forms, so neither side's dynamic table is involved.

    /// `:status: 200` (static index 8) and `content-type: application/grpc`
    /// (literal, never indexed, name = static index 31).
    const H2_RESPONSE_HEADERS: &[u8] = b"\x88\x0f\x10\x10application/grpc";
    /// `grpc-status: 0` (literal, never indexed, new name).
    const H2_OK_TRAILERS: &[u8] = b"\x00\x0bgrpc-status\x010";
    /// One gRPC message: uncompressed, zero bytes long.
    const EMPTY_GRPC_MESSAGE: [u8; 5] = [0; 5];

    fn h2_frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
        let len = payload.len() as u32;
        let mut frame = vec![(len >> 16) as u8, (len >> 8) as u8, len as u8, kind, flags];
        frame.extend_from_slice(&stream.to_be_bytes());
        frame.extend_from_slice(payload);
        frame
    }

    async fn h2_write<W>(to: &tokio::sync::Mutex<W>, frame: Vec<u8>)
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        use tokio::io::AsyncWriteExt;
        let mut to = to.lock().await;
        // A peer that has gone away is the test's business, not this writer's.
        let _ = to.write_all(&frame).await;
        let _ = to.flush().await;
    }

    /// How an [`AnsweringPeer`] replies to each complete request.
    #[derive(Clone, Copy)]
    enum Reply {
        /// One message and `grpc-status: 0`.
        AtOnce,
        /// A server stream: headers at once, then `messages` messages `gap`
        /// apart, then a clean end.
        Stream { messages: usize, gap: Duration },
    }

    struct AnsweringPeer {
        reply: Reply,
        /// Replies wait until this reads `true`: a path that ACCEPTED and has
        /// not answered YET. Connections stay up and PINGs are acknowledged
        /// throughout — only the answers are held.
        gate: tokio::sync::watch::Sender<bool>,
        /// Ends every connection this peer is serving (a transport torn down
        /// under a backgrounded app).
        cut: Arc<tokio::sync::Notify>,
    }

    impl AnsweringPeer {
        fn answering(reply: Reply) -> Arc<Self> {
            Self::new(reply, true)
        }

        fn holding_its_answers(reply: Reply) -> Arc<Self> {
            Self::new(reply, false)
        }

        fn new(reply: Reply, open: bool) -> Arc<Self> {
            Arc::new(Self {
                reply,
                gate: tokio::sync::watch::channel(open).0,
                cut: Arc::new(tokio::sync::Notify::new()),
            })
        }

        fn release(&self) {
            self.gate.send_replace(true);
        }

        fn cut_every_connection(&self) {
            self.cut.notify_waiters();
        }

        fn serve<S>(&self, io: S)
        where
            S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
        {
            tokio::spawn(serve_h2(
                io,
                self.gate.subscribe(),
                self.reply,
                Arc::clone(&self.cut),
            ));
        }
    }

    #[async_trait]
    impl NetDialer for AnsweringPeer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            let (client_end, server_end) = tokio::io::duplex(8192);
            self.serve(server_end);
            Ok(Box::new(client_end))
        }
    }

    async fn serve_h2<S>(
        io: S,
        gate: tokio::sync::watch::Receiver<bool>,
        reply: Reply,
        cut: Arc<tokio::sync::Notify>,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
    {
        use tokio::io::AsyncReadExt;
        let (mut from, to) = tokio::io::split(io);
        let to = Arc::new(tokio::sync::Mutex::new(to));
        // Streams the client gave up on (its own timeout): never answered late.
        let reset = Arc::new(Mutex::new(std::collections::HashSet::<u32>::new()));
        let mut preface = [0_u8; 24];
        if from.read_exact(&mut preface).await.is_err() {
            return;
        }
        h2_write(&to, h2_frame(0x4, 0, 0, &[])).await;
        loop {
            let mut head = [0_u8; 9];
            tokio::select! {
                _ = cut.notified() => return,
                read = from.read_exact(&mut head) => {
                    if read.is_err() {
                        return;
                    }
                }
            }
            let len = u32::from_be_bytes([0, head[0], head[1], head[2]]) as usize;
            let (kind, flags) = (head[3], head[4]);
            let stream = u32::from_be_bytes([head[5], head[6], head[7], head[8]]) & 0x7fff_ffff;
            let mut payload = vec![0_u8; len];
            if from.read_exact(&mut payload).await.is_err() {
                return;
            }
            match kind {
                // SETTINGS and PING, unless they are themselves acknowledgements.
                0x4 if flags & 0x1 == 0 => h2_write(&to, h2_frame(0x4, 0x1, 0, &[])).await,
                0x6 if flags & 0x1 == 0 => h2_write(&to, h2_frame(0x6, 0x1, 0, &payload)).await,
                // RST_STREAM.
                0x3 => {
                    reset.lock().expect("reset set poisoned").insert(stream);
                }
                // DATA or HEADERS carrying END_STREAM: the request is complete.
                0x0 | 0x1 if flags & 0x1 != 0 => {
                    let (to, reset, mut gate) = (Arc::clone(&to), Arc::clone(&reset), gate.clone());
                    tokio::spawn(async move {
                        if gate.wait_for(|open| *open).await.is_err() {
                            return;
                        }
                        if reset.lock().expect("reset set poisoned").contains(&stream) {
                            return;
                        }
                        h2_write(&to, h2_frame(0x1, 0x4, stream, H2_RESPONSE_HEADERS)).await;
                        match reply {
                            Reply::AtOnce => {
                                h2_write(&to, h2_frame(0x0, 0, stream, &EMPTY_GRPC_MESSAGE)).await;
                            }
                            Reply::Stream { messages, gap } => {
                                for _ in 0..messages {
                                    tokio::time::sleep(gap).await;
                                    h2_write(&to, h2_frame(0x0, 0, stream, &EMPTY_GRPC_MESSAGE))
                                        .await;
                                }
                            }
                        }
                        h2_write(&to, h2_frame(0x1, 0x5, stream, H2_OK_TRAILERS)).await;
                    });
                }
                _ => {}
            }
        }
    }

    /// A LIVE loopback listener for the clearnet arm. With a peer every accepted
    /// socket is answered; without one it is accepted and held — a clearnet
    /// connection that establishes and then carries nothing.
    ///
    /// An answered socket is `TCP_NODELAY`, as a real lightwalletd's is (Go
    /// sets it on every TCP connection) and as our own dialer sets it on the
    /// client end. The peer answers in three small writes; with Nagle on, the
    /// second and third wait for the client's ACK of the first, and Linux
    /// DELAYS that ACK on a warm connection (~40 ms of REAL time). A paused
    /// clock does not wait for it: the runtime is idle, so it auto-advances in
    /// 10 ms steps and spends a 30 s RPC deadline in a few real milliseconds —
    /// the answer is in the server's send queue and the RPC reads `Timeout`.
    async fn live_loopback_endpoint(peer: Option<Arc<AnsweringPeer>>) -> LightServerEndpoint {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((socket, _)) = listener.accept().await {
                match &peer {
                    Some(peer) => {
                        socket
                            .set_nodelay(true)
                            .expect("TCP_NODELAY on the answering socket");
                        peer.serve(socket);
                    }
                    None => held.push(socket),
                }
            }
        });
        LightServerEndpoint::new(format!("http://127.0.0.1:{port}")).expect("loopback endpoint")
    }

    /// A private path scripted PER CIRCUIT, by isolation key — what a censor that
    /// treats circuits differently looks like from the dialer. A key containing
    /// `refused` cannot be reached at all; any other key is served by `accepted`.
    struct CircuitsByKey {
        refused: &'static str,
        accepted: Arc<dyn NetDialer>,
    }

    #[async_trait]
    impl NetDialer for CircuitsByKey {
        async fn dial(
            &self,
            host: &str,
            port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            if isolation_key.is_some_and(|key| key.contains(self.refused)) {
                return Err(crate::error::DialError::Unreachable);
            }
            self.accepted.dial(host, port, isolation_key).await
        }
    }

    fn sync_client(
        endpoint: &LightServerEndpoint,
        policy: &TorPolicy,
        posture: &Arc<TorPosture>,
    ) -> LightwalletdClient {
        LightwalletdClient::connect_plaintext_over_test_dialer(
            endpoint,
            policy,
            Some(crate::constants::WALLET_SYNC_ISOLATION_KEY.to_owned()),
            posture,
            None,
        )
        .expect("lazy connect never blocks")
    }

    /// A fresh per-send client, as `Wallet::broadcast_one` builds one.
    fn send_client(
        endpoint: &LightServerEndpoint,
        policy: &TorPolicy,
        posture: &Arc<TorPosture>,
        tag: &str,
    ) -> LightwalletdClient {
        let key = format!("{}-{tag}", crate::constants::BROADCAST_ISOLATION_KEY_PREFIX);
        LightwalletdClient::connect_plaintext_over_test_dialer(
            endpoint,
            policy,
            Some(key),
            posture,
            None,
        )
        .expect("lazy connect never blocks")
    }

    fn arm_class_outcome(line: &DialLine) -> (&str, &str, &str) {
        (&line.arm, &line.class, &line.outcome)
    }

    /// THE INSTRUMENT, checked before it is believed: when a switch DOES happen —
    /// through the arm that exists at the base commit, a private path that
    /// REFUSES for a genuine minute — this capture sees it, on a paused clock,
    /// over a real loopback connection, with the RPC completing. So a row below
    /// that reads "0 clearnet dials" is reading the wallet, not a blind capture.
    #[tokio::test(start_paused = true)]
    async fn the_capture_sees_a_switch_when_a_refused_private_path_makes_one() {
        use crate::constants::TOR_PATIENCE_SECS;
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = live_loopback_endpoint(Some(AnsweringPeer::answering(Reply::AtOnce))).await;
        let policy = preferred_over(Arc::new(crate::ports::testing::FailingDialer));
        let mut client = sync_client(&endpoint, &policy, &posture);

        assert!(
            client.get_lightd_info().await.is_err(),
            "inside the minute a refused private path fails the pass"
        );
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS + 1)).await;
        client
            .get_lightd_info()
            .await
            .expect("the minute is spent: the pass completes over clearnet");

        let all = lines.all();
        assert_eq!(
            all.iter().map(arm_class_outcome).collect::<Vec<_>>(),
            [
                ("host", "sync", "unreachable"),
                ("host", "sync", "unreachable"),
                ("sdk_direct", "sync", "connected"),
            ],
        );
        assert_eq!(
            all.iter().map(|l| l.latched).collect::<Vec<_>>(),
            [false, false, true],
            "the latch is read where each line is written"
        );
    }

    /// §3.1 ASSERTION 2 — THE ARM, on a real loopback and a real clock, and the
    /// answered half of ASSERTION 3.
    ///
    /// The posture is one whose sync window is ALREADY spent (back-dated, so the
    /// real predicate computes it — not `spend_the_minute_for_test`'s
    /// short-circuit). The private path ACCEPTS. A wallet in that state must dial
    /// clearnet, the line must read `connected`, and the RPC the user was waiting
    /// on must COMPLETE over it — which is the whole point of leaving.
    ///
    /// Then five further passes, all answered: the window stays spent for the
    /// session (nothing clears a failing run on clearnet), so a consumer that
    /// acts on the predicate's LEVEL would dial again on every one of them.
    #[tokio::test]
    async fn a_spent_window_over_an_accepting_silent_path_connects_over_clearnet() {
        let posture = Arc::new(TorPosture::with_the_minute_already_spent());
        let (lines, _capture) = capture_dial_lines(&posture);

        let endpoint = live_loopback_endpoint(Some(AnsweringPeer::answering(Reply::AtOnce))).await;
        let policy = preferred_over(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>);
        let mut client = sync_client(&endpoint, &policy, &posture);

        // Bounded by the TEST: at the base commit this RPC rides the silent
        // private connection and would otherwise sit out the real unary budget.
        let first = tokio::time::timeout(Duration::from_secs(3), client.get_lightd_info()).await;

        let all = lines.all();
        let clearnet = lines.of_arm("sdk_direct");
        assert_eq!(
            clearnet.iter().map(arm_class_outcome).collect::<Vec<_>>(),
            [("sdk_direct", "sync", "connected")],
            "a spent window over a path that ACCEPTS must still leave it — every dial: {all:?}"
        );
        assert!(
            matches!(first, Ok(Ok(_))),
            "and the RPC completes over the clearnet connection: {first:?}"
        );
        assert!(
            clearnet[0].latched && posture.fell_back(),
            "the switch is VISIBLE"
        );

        for _ in 0..5 {
            client
                .get_lightd_info()
                .await
                .expect("answered over the clearnet connection");
        }
        assert_eq!(
            lines.all().len(),
            all.len(),
            "five further passes over a working clearnet connection dial NOTHING: {:?}",
            lines.all()
        );
    }

    /// §3.1 ASSERTION 3 — NO REDIAL STORM, the half where passes keep FAILING.
    ///
    /// A genuine minute on a paused clock, then a clearnet leg that establishes
    /// and carries nothing either. Every later pass times out over it; the
    /// window still reads spent; and none of those passes may dial again. A
    /// consumer keyed to the predicate's level — "rebuild whenever
    /// `may_switch_to_direct`" — redials on every pass for ever, because nothing
    /// on clearnet ever makes the predicate false again.
    #[tokio::test(start_paused = true)]
    async fn after_the_switch_further_failing_passes_dial_nothing() {
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = live_loopback_endpoint(None).await;
        let policy = preferred_over(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>);
        let mut client = sync_client(&endpoint, &policy, &posture);

        let first_clearnet = pass_until_the_wallet_leaves(&mut client, &lines, &posture).await;
        assert_eq!(
            first_clearnet.outcome, "connected",
            "fixture: further passes only mean something over a LIVE clearnet connection"
        );
        assert_one_visible_switch(&lines, &posture, &first_clearnet, PathClass::Sync, "sync");

        let at_the_switch = lines.all();
        for _ in 0..4 {
            assert!(
                client.get_lightd_info().await.is_err(),
                "fixture: the clearnet server is silent too"
            );
        }
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "the window STAYS spent on clearnet — which is what makes a level-keyed consumer storm"
        );
        assert_eq!(
            lines.all().len(),
            at_the_switch.len(),
            "four further passes, zero further dials of either arm: {:?}",
            lines.all()
        );
    }

    /// §3.1 ASSERTION 4 — BROADCAST, the money shape. The one place a second dial
    /// DOES occur is a fresh client per send, so here F-A's first half stands
    /// alone: the redial reaches the same accepting private path and the same
    /// `Ok` arm, which never reads the window.
    #[tokio::test(start_paused = true)]
    async fn a_blackholed_broadcast_leaves_at_the_minute_on_a_fresh_client_per_send() {
        use crate::constants::TOR_PATIENCE_SECS;
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>);

        let first_clearnet =
            send_until_the_wallet_leaves(&endpoint, &policy, &lines, &posture).await;
        assert!(
            first_clearnet.at >= Duration::from_secs(TOR_PATIENCE_SECS),
            "a payment dialled in the clear at {:?} — INSIDE the founder's minute",
            first_clearnet.at
        );
        assert_one_visible_switch(
            &lines,
            &posture,
            &first_clearnet,
            PathClass::Broadcast,
            "broadcast",
        );
    }

    /// §3.1 ASSERTION 5 — `Required`, on the SAME fixtures: the reused sync
    /// client and the fresh client per send, over the same accepting, silent
    /// private path, for far longer than the minute. Zero clearnet dials, ever,
    /// and nothing latched.
    ///
    /// TWO things refuse here, and this row cannot tell them apart: `Required`
    /// has no clearnet arm to dial, AND its client carries no witness, so on this
    /// fixture its window never spends in the first place. It is a guard for a
    /// consumer that lives in the client or the engine — one that must not find
    /// an arm. The row that sees the ARM itself break is the next one, where the
    /// window does spend.
    #[tokio::test(start_paused = true)]
    async fn required_never_dials_clearnet_over_an_accepted_and_silent_path() {
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = dead_loopback_endpoint().await;
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>),
        };
        let fail_closed = |when: &str| {
            assert!(
                lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
                "Required sent a clearnet dial {when}: {:?}",
                lines.all()
            );
        };

        let mut client = sync_client(&endpoint, &policy, &posture);
        for pass in 1..=10 {
            assert!(client.get_lightd_info().await.is_err());
            fail_closed(&format!("on sync pass {pass}"));
        }
        for send in 1..=6 {
            let mut client = send_client(&endpoint, &policy, &posture, &format!("{send:016x}"));
            assert!(
                client
                    .send_transaction(vec![0x05, 0x00, 0x00, 0x80])
                    .await
                    .is_err()
            );
            fail_closed(&format!("on send {send}"));
        }
        assert!(
            lines.started.elapsed() >= Duration::from_secs(8 * crate::constants::TOR_PATIENCE_SECS),
            "fixture: eight minutes of a silent private path"
        );
        assert!(
            !lines.of_arm("host").is_empty(),
            "fixture: the private path WAS dialled, so the capture was looking"
        );
    }

    /// §3.1 ASSERTION 5, on the fixture where `Required`'s window DOES spend: a
    /// private path that REFUSES, through the plan `resolve_dialer` builds, for
    /// three minutes. Dial failures are recorded whatever the policy, so after a
    /// minute the only thing between this wallet and clearnet is that its plan
    /// has no clearnet arm. `tor_required_dialer_failure_is_fail_closed_zero_
    /// bytes` makes ONE dial, inside the minute, and would not notice an arm
    /// that only opens after it.
    #[tokio::test(start_paused = true)]
    async fn required_never_dials_clearnet_however_long_the_private_path_refuses() {
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = dead_loopback_endpoint().await;
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::new(crate::ports::testing::FailingDialer)),
        };
        let mut client = sync_client(&endpoint, &policy, &posture);
        for pass in 1..=18 {
            assert!(client.get_lightd_info().await.is_err());
            assert!(
                lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
                "Required sent a clearnet dial on pass {pass}, {:?} in: {:?}",
                lines.started.elapsed(),
                lines.all()
            );
            tokio::time::advance(Duration::from_secs(10)).await;
        }
        assert_eq!(
            lines.of_arm("host").len(),
            18,
            "fixture: every pass dialled the private path and was refused"
        );
    }

    /// NOT IN THE CONTRACT'S LIST (IT-1 +A). A private path that goes silent and
    /// then ANSWERS inside the minute is never left.
    ///
    /// §3.1 asks for a consumer that acts "whether or not a dial ever fails
    /// again", and the cheapest way to build one is a deadline armed when the
    /// trouble starts. Such a consumer must read the window AGAIN when it acts:
    /// here the first pass times out (the failing run starts at 30 s), the
    /// second is answered at 59 s, and the path then carries for three more
    /// minutes — across the instant (90 s) at which a deadline armed by the
    /// first failure would fire.
    #[tokio::test(start_paused = true)]
    async fn a_private_path_that_answers_inside_the_minute_is_never_left() {
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let private = AnsweringPeer::holding_its_answers(Reply::AtOnce);
        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over(Arc::clone(&private) as Arc<dyn NetDialer>);
        let mut client = sync_client(&endpoint, &policy, &posture);

        assert!(
            matches!(
                client.get_lightd_info().await,
                Err(GrpcError::Timeout { .. })
            ),
            "fixture: the first pass sits out its budget on the silent path"
        );
        let recover = async {
            tokio::time::sleep(Duration::from_secs(29)).await;
            private.release();
        };
        let (answered, ()) = tokio::join!(client.get_lightd_info(), recover);
        answered.expect("fixture: the path starts carrying at 59 s and the pending RPC completes");
        assert!(lines.started.elapsed() < Duration::from_secs(60));

        for _ in 0..6 {
            tokio::time::advance(Duration::from_secs(30)).await;
            client
                .get_lightd_info()
                .await
                .expect("the private path carries");
            assert!(
                lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
                "left a private path that was CARRYING, at {:?}: {:?}",
                lines.started.elapsed(),
                lines.all()
            );
        }
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "a confirmed RPC ended the failing run; nothing has failed since"
        );
    }

    /// NOT IN THE CONTRACT'S LIST (IT-1 +A), the second one. The window has a
    /// SUBJECT, so the consumer must have one too: a spent SYNC window — latched,
    /// the wallet visibly on clearnet for sync — must not send a BROADCAST in the
    /// clear while fresh per-send circuits are being carried. "Once latched,
    /// dial direct" and "any class spent" are both one-line ways to close F-A
    /// (i), and both undo the per-class window (ADR-0552 stage 1b, finding ii).
    #[tokio::test(start_paused = true)]
    async fn a_spent_sync_window_does_not_send_a_broadcast_in_the_clear() {
        use crate::constants::{TOR_PATIENCE_SECS, WALLET_SYNC_ISOLATION_KEY};
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over(Arc::new(CircuitsByKey {
            refused: WALLET_SYNC_ISOLATION_KEY,
            accepted: AnsweringPeer::answering(Reply::AtOnce),
        }));

        let mut sync = sync_client(&endpoint, &policy, &posture);
        assert!(sync.get_lightd_info().await.is_err());
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS + 1)).await;
        assert!(sync.get_lightd_info().await.is_err());
        assert_eq!(
            lines.of_arm("sdk_direct").len(),
            1,
            "fixture: the refused sync circuit left at the minute: {:?}",
            lines.all()
        );
        assert!(posture.fell_back(), "fixture: and the wallet is latched");

        let mut send = send_client(&endpoint, &policy, &posture, "0123456789abcdef");
        let outcome = send.send_transaction(vec![0x05, 0x00, 0x00, 0x80]).await;
        let all = lines.all();
        assert_eq!(
            all.last().map(arm_class_outcome),
            Some(("host", "broadcast", "connected")),
            "the broadcast class has not failed once — its payment rides the private path: {all:?}"
        );
        assert!(
            matches!(outcome, Ok(SubmitOutcome::Accepted)),
            "and is carried by it: {outcome:?}"
        );
        assert_eq!(
            lines.of_arm("sdk_direct").len(),
            1,
            "the only clearnet dial on record is the sync one: {all:?}"
        );
    }

    /// `tor-patience-phase-2.md` §5: a slow stream that keeps delivering never
    /// switches. 70 s between messages spends the SILENCE conjunct every time and
    /// is healthy by every other bound in the tree; nothing is failing, so
    /// nothing may leave — and nothing may end the connection for being quiet.
    #[tokio::test(start_paused = true)]
    async fn a_slow_stream_that_keeps_delivering_never_switches() {
        use crate::constants::TOR_PATIENCE_SECS;
        const GAP_SECS: u64 = 70;
        const { assert!(GAP_SECS > TOR_PATIENCE_SECS && GAP_SECS < GRPC_STREAMING_TIMEOUT_SECS) };

        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over(AnsweringPeer::answering(Reply::Stream {
            messages: 3,
            gap: Duration::from_secs(GAP_SECS),
        }));
        let mut client = sync_client(&endpoint, &policy, &posture);

        // The instant that matters is the one just BEFORE each message lands,
        // with 69 s of silence on the clock — not the one after it, where the
        // stamp has just moved. So the window is read every second, throughout.
        let read_spent = Arc::new(AtomicBool::new(false));
        tokio::spawn({
            let (posture, read_spent) = (Arc::clone(&posture), Arc::clone(&read_spent));
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if posture.may_switch_to_direct(PathClass::Sync) {
                        read_spent.store(true, Ordering::Relaxed);
                    }
                }
            }
        });

        let mut blocks = client
            .get_block_range(280_000, 280_002)
            .await
            .expect("response headers arrive at once");
        for delivered in 1..=3_u64 {
            blocks
                .next_block()
                .await
                .expect("a slow link is not a fault")
                .expect("a message");
            assert!(
                lines.started.elapsed() >= Duration::from_secs(delivered * GAP_SECS),
                "fixture: each message is {GAP_SECS} s after the last"
            );
            assert!(
                !posture.may_switch_to_direct(PathClass::Sync)
                    && lines.of_arm("sdk_direct").is_empty()
                    && !posture.fell_back(),
                "message {delivered}: a stream that keeps delivering is not a path that is \
                 failing: {:?}",
                lines.all()
            );
        }
        assert!(
            blocks.next_block().await.expect("clean end").is_none(),
            "fixture: the stream ends cleanly"
        );
        assert!(
            !read_spent.load(Ordering::Relaxed),
            "the window read SPENT at some second of a download that never failed — silence \
             alone is what an unhurried link accumulates"
        );
        assert_eq!(
            lines
                .all()
                .iter()
                .map(arm_class_outcome)
                .collect::<Vec<_>>(),
            [("host", "sync", "connected")],
            "one private connection carried the whole download"
        );
    }

    /// `tor-patience-phase-2.md` §5 / §3.1 ASSERTION 6: a private RPC on a SIBLING
    /// connection is not dropped.
    ///
    /// Two broadcasts race by design (the kick task and the resubmission pass).
    /// One is accepted at 45 s and waits for its answer; the censor refuses the
    /// other's circuit, the broadcast window runs out at 61 s and that send goes
    /// to clearnet. Then the first one is ANSWERED — over the private connection
    /// it has held all along. That is a confirmed private RPC and it restarts
    /// the minute (ADR-0552 decision 4); one verdict per CLASS forgets that this
    /// connection was private the moment its sibling fell back.
    #[tokio::test(start_paused = true)]
    async fn a_private_rpc_on_a_sibling_connection_is_not_dropped() {
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let private = AnsweringPeer::holding_its_answers(Reply::AtOnce);
        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over(Arc::new(CircuitsByKey {
            refused: "refused",
            accepted: Arc::clone(&private) as Arc<dyn NetDialer>,
        }));
        let raw_tx = || vec![0x05, 0x00, 0x00, 0x80];

        let mut refused_1 = send_client(&endpoint, &policy, &posture, "refused-1");
        assert!(refused_1.send_transaction(raw_tx()).await.is_err());

        tokio::time::advance(Duration::from_secs(45)).await;
        let mut accepted = send_client(&endpoint, &policy, &posture, "accepted");
        let waiting = tokio::spawn(async move { accepted.send_transaction(raw_tx()).await });
        while !lines.all().iter().any(|l| l.outcome == "connected") {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        tokio::time::advance(Duration::from_secs(16)).await;
        let mut refused_2 = send_client(&endpoint, &policy, &posture, "refused-2");
        assert!(refused_2.send_transaction(raw_tx()).await.is_err());
        assert_eq!(
            lines.of_arm("sdk_direct").len(),
            1,
            "fixture: the refused sibling left at the minute: {:?}",
            lines.all()
        );
        assert!(
            posture.may_switch_to_direct(PathClass::Broadcast),
            "fixture: the broadcast window is spent"
        );

        private.release();
        let outcome = waiting.await.expect("the waiting send did not panic");
        assert!(
            matches!(outcome, Ok(SubmitOutcome::Accepted)),
            "fixture: the waiting send is answered over its private connection: {outcome:?}"
        );
        assert!(
            !posture.may_switch_to_direct(PathClass::Broadcast),
            "the private path just CARRIED a broadcast, and that restarts the minute — the \
             evidence was dropped because a SIBLING connection of the same class had fallen \
             back: {:?}",
            lines.all()
        );
    }

    /// `tor-patience-phase-2.md` §5 / §3.1 ASSERTION 6: a clearnet RPC on a sibling
    /// connection never restarts the private window.
    ///
    /// The other interleaving, and it needs a clearnet connection that stays UP:
    /// one send falls back at 61 s and keeps its (answering) clearnet
    /// connection. The private path then recovers — the waiting sibling is
    /// answered, a further send is carried privately and its connection is
    /// alive — and a new circuit is refused, which starts a fresh failing run.
    /// Now the OLD clearnet connection completes another RPC. It is evidence
    /// about clearnet. With one verdict per class it reads as the private path
    /// carrying, because the class's most recent dial was private: the failing
    /// run is cleared, and a minute later the wallet is still insisting on a
    /// path that has carried nothing since.
    #[tokio::test(start_paused = true)]
    async fn a_clearnet_rpc_on_a_sibling_connection_never_restarts_the_private_window() {
        use crate::constants::TOR_PATIENCE_SECS;
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let private = AnsweringPeer::holding_its_answers(Reply::AtOnce);
        let endpoint = live_loopback_endpoint(Some(AnsweringPeer::answering(Reply::AtOnce))).await;
        let policy = preferred_over(Arc::new(CircuitsByKey {
            refused: "refused",
            accepted: Arc::clone(&private) as Arc<dyn NetDialer>,
        }));
        let raw_tx = || vec![0x05, 0x00, 0x00, 0x80];
        let accepted = |outcome: &Result<SubmitOutcome, GrpcError>| {
            matches!(outcome, Ok(SubmitOutcome::Accepted))
        };

        let mut refused_1 = send_client(&endpoint, &policy, &posture, "refused-1");
        assert!(refused_1.send_transaction(raw_tx()).await.is_err());
        tokio::time::advance(Duration::from_secs(45)).await;
        let mut waiting_client = send_client(&endpoint, &policy, &posture, "accepted-1");
        let waiting = tokio::spawn(async move { waiting_client.send_transaction(raw_tx()).await });
        while !lines.all().iter().any(|l| l.outcome == "connected") {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // 61 s: the refused sibling falls back, and its clearnet connection LIVES.
        tokio::time::advance(Duration::from_secs(16)).await;
        let mut on_clearnet = send_client(&endpoint, &policy, &posture, "refused-2");
        let fell_back = on_clearnet.send_transaction(raw_tx()).await;
        assert!(
            accepted(&fell_back),
            "fixture: the fallen-back send is carried by clearnet: {fell_back:?} {:?}",
            lines.all()
        );
        assert_eq!(
            lines
                .of_arm("sdk_direct")
                .iter()
                .map(arm_class_outcome)
                .collect::<Vec<_>>(),
            [("sdk_direct", "broadcast", "connected")],
        );

        // The private path recovers, and a private connection is alive again.
        private.release();
        let answered = waiting.await.expect("the waiting send did not panic");
        assert!(accepted(&answered), "fixture: {answered:?}");
        let mut private_again = send_client(&endpoint, &policy, &posture, "accepted-2");
        let carried = private_again.send_transaction(raw_tx()).await;
        assert!(accepted(&carried), "fixture: {carried:?}");
        assert_eq!(
            lines.all().last().map(arm_class_outcome),
            Some(("host", "broadcast", "connected")),
            "the private path recovered (a confirmed RPC restarted the minute), so the next \
             send rides it: {:?}",
            lines.all()
        );

        // A new circuit is refused: a fresh failing run, starting now.
        let mut refused_3 = send_client(&endpoint, &policy, &posture, "refused-3");
        assert!(refused_3.send_transaction(raw_tx()).await.is_err());
        let failing_since = lines.started.elapsed();
        assert_eq!(lines.of_arm("sdk_direct").len(), 1, "inside the new minute");

        // The OLD clearnet connection carries another RPC.
        tokio::time::advance(Duration::from_secs(10)).await;
        let dials = lines.all().len();
        let over_clearnet = on_clearnet.send_transaction(raw_tx()).await;
        assert!(accepted(&over_clearnet), "fixture: {over_clearnet:?}");
        assert_eq!(
            lines.all().len(),
            dials,
            "fixture: it rode the clearnet connection it already had"
        );

        let minute_of_failing = failing_since + Duration::from_secs(TOR_PATIENCE_SECS + 1);
        tokio::time::advance(minute_of_failing - lines.started.elapsed()).await;
        assert!(
            posture.may_switch_to_direct(PathClass::Broadcast),
            "the private path has carried nothing and has been failing for a minute; the only \
             RPC since was over CLEARNET, and it restarted the private window"
        );
    }

    /// A registered host transport whose descriptor the test moves, over a peer
    /// that answers whenever the host is allowed to dial.
    struct HostTransport {
        descriptor: Mutex<Option<crate::net::host_dialer::HostTransportDescriptor>>,
        peer: Arc<AnsweringPeer>,
    }

    impl HostTransport {
        fn describe(readiness: u8, health: crate::net::host_dialer::TransportHealth) -> Self {
            let host = Self {
                descriptor: Mutex::new(None),
                peer: AnsweringPeer::answering(Reply::AtOnce),
            };
            host.set(readiness, health);
            host
        }

        fn set(&self, readiness: u8, health: crate::net::host_dialer::TransportHealth) {
            use crate::net::host_dialer::{
                HostTransportDescriptor, IsolationSupport, TransportExposure, testing::name,
            };
            *self.descriptor.lock().expect("descriptor poisoned") = Some(HostTransportDescriptor {
                name: name("Tor"),
                readiness,
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
                health,
            });
        }
    }

    #[async_trait]
    impl NetDialer for HostTransport {
        async fn dial(
            &self,
            host: &str,
            port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            self.peer.dial(host, port, isolation_key).await
        }
    }

    impl crate::net::host_dialer::HostDialer for HostTransport {
        fn descriptor(&self) -> Option<crate::net::host_dialer::HostTransportDescriptor> {
            *self.descriptor.lock().expect("descriptor poisoned")
        }
    }

    fn preferred_over_host(host: &Arc<HostTransport>) -> TorPolicy {
        TorPolicy::Preferred {
            runtime: TorRuntime::HostDialer(
                Arc::clone(host) as Arc<dyn crate::net::host_dialer::HostDialer>
            ),
        }
    }

    /// `tor-patience-phase-2.md` §5 / §2a: a bootstrapping transport never
    /// switches, however long it takes — and one the host declares FAILED does.
    ///
    /// Five whole windows of `Starting`, a pass every ten seconds: a bootstrap
    /// fails continuously and is not the private path failing. Then the host
    /// says FAILED — it is not going to start. §3.1: "a bootstrapping transport
    /// never counts as failing; one the host declared FAILED does", so the
    /// minute starts THERE: the first pass after the declaration must not leave
    /// (the bootstrap banked nothing), and a minute of FAILED later the wallet
    /// does.
    #[tokio::test(start_paused = true)]
    async fn a_bootstrapping_transport_never_switches_however_long_it_takes() {
        use crate::constants::TOR_PATIENCE_SECS;
        use crate::net::host_dialer::TransportHealth;
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let host = Arc::new(HostTransport::describe(40, TransportHealth::Starting));
        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over_host(&host);
        let mut client = sync_client(&endpoint, &policy, &posture);

        for _ in 0..(5 * TOR_PATIENCE_SECS / 10) {
            assert!(client.get_lightd_info().await.is_err());
            assert!(
                lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
                "left a BOOTSTRAPPING transport at {:?}: {:?}",
                lines.started.elapsed(),
                lines.all()
            );
            tokio::time::advance(Duration::from_secs(10)).await;
        }

        host.set(40, TransportHealth::Failed);
        let declared_at = lines.started.elapsed();
        let minute = Duration::from_secs(TOR_PATIENCE_SECS);
        assert!(client.get_lightd_info().await.is_err());
        assert!(
            lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
            "the FIRST refusal of a failed transport starts its minute — five windows of \
             bootstrapping are not five windows of failing: {:?}",
            lines.all()
        );

        let first_clearnet = loop {
            if let Some(first) = lines.of_arm("sdk_direct").first() {
                break first.clone();
            }
            let now = lines.started.elapsed();
            assert!(
                now < declared_at + minute + Insisting::GRACE,
                "the host declared its transport FAILED at {declared_at:?}; it is now {now:?} \
                 and the wallet has not left. A transport its host has given up on is a \
                 private path that is not working, and the refusal has to say so to the one \
                 place that decides: {:?}",
                lines.of_arm("host").last()
            );
            tokio::time::advance(Duration::from_secs(10)).await;
            let _ = client.get_lightd_info().await;
        };
        assert!(
            first_clearnet.at >= declared_at + minute,
            "left {:?} after the declaration — inside the minute",
            first_clearnet.at - declared_at
        );
        assert_one_visible_switch(&lines, &posture, &first_clearnet, PathClass::Sync, "sync");
    }

    /// `tor-patience-phase-2.md` §5: a resumed app gets a fresh minute before it
    /// may switch.
    ///
    /// Five minutes in the background spend the SILENCE conjunct (`Instant`
    /// runs while an app is merely backgrounded), the host tears its circuits
    /// down, and on resume the transport re-bootstraps for three minutes. Every
    /// pass in that time fails — through the one reused client, whose last
    /// private connect is still on record — and none of it is the private path
    /// failing. A consumer that acts on the window "whether or not a dial ever
    /// fails again" must not act on this.
    #[tokio::test(start_paused = true)]
    async fn a_resumed_app_gets_a_fresh_minute_before_it_may_switch() {
        use crate::net::host_dialer::{HOST_TRANSPORT_READY, TransportHealth};
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let host = Arc::new(HostTransport::describe(
            HOST_TRANSPORT_READY,
            TransportHealth::Ready,
        ));
        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over_host(&host);
        let mut client = sync_client(&endpoint, &policy, &posture);

        for _ in 0..3 {
            client
                .get_lightd_info()
                .await
                .expect("fixture: a healthy private path");
            tokio::time::advance(Duration::from_secs(10)).await;
        }

        // Backgrounded: nothing runs, the clock does.
        tokio::time::advance(Duration::from_secs(300)).await;

        host.set(10, TransportHealth::Starting);
        host.peer.cut_every_connection();
        for _ in 0..18 {
            assert!(
                client.get_lightd_info().await.is_err(),
                "fixture: the transport is re-bootstrapping"
            );
            assert!(
                lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
                "a RESUMED app left its private path while the transport was re-bootstrapping, \
                 at {:?}: {:?}",
                lines.started.elapsed(),
                lines.all()
            );
            tokio::time::advance(Duration::from_secs(10)).await;
        }

        host.set(HOST_TRANSPORT_READY, TransportHealth::Ready);
        client
            .get_lightd_info()
            .await
            .expect("the transport is back and the private path carries");
        let all = lines.all();
        assert_eq!(
            all.last().map(arm_class_outcome),
            Some(("host", "sync", "connected")),
            "{all:?}"
        );
        assert!(lines.of_arm("sdk_direct").is_empty() && !posture.fell_back());
    }

    /// NOT IN THE CONTRACT'S LIST (IT-1 +A), the third one: the THRESHOLD of
    /// `kept_failing_past_the_window`. An accept carries no evidence of its
    /// own, so it may be left only on a run that was SEEN to outlast the
    /// minute — a failure observed `TOR_PATIENCE_SECS` or more after the run
    /// began. Two refusals a second apart are a run one second long; an idle
    /// gap after them spends the CLOCK (both of `may_switch_to_direct`'s
    /// conjuncts read true) but adds no failure, and the next dial is
    /// ACCEPTED. A threshold of anything under the minute would let that
    /// accept be left for clearnet on one second of evidence — the resumed-app
    /// leak by another door. The sibling above and the implementer's idle-gap
    /// probe both see a run of ONE failure, which any threshold refuses; this
    /// is the row that pins the constant.
    #[tokio::test(start_paused = true)]
    async fn a_failing_run_shorter_than_the_minute_does_not_leave_an_accept_after_an_idle_gap() {
        use crate::constants::TOR_PATIENCE_SECS;
        use crate::net::host_dialer::{HOST_TRANSPORT_READY, TransportHealth};
        let posture = Arc::new(TorPosture::new());
        let (lines, _capture) = capture_dial_lines(&posture);
        hold_the_paused_clock_to_small_steps();

        let host = Arc::new(HostTransport::describe(
            HOST_TRANSPORT_READY,
            TransportHealth::Failed,
        ));
        let endpoint = dead_loopback_endpoint().await;
        let policy = preferred_over_host(&host);
        let mut client = sync_client(&endpoint, &policy, &posture);

        // Two refusals of a transport its host declared FAILED, a second apart:
        // a failing run that began, and was last seen, inside one second.
        for _ in 0..2 {
            assert!(
                client.get_lightd_info().await.is_err(),
                "fixture: a FAILED transport refuses the dial"
            );
            tokio::time::advance(Duration::from_secs(1)).await;
        }
        assert!(
            lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
            "fixture: two refusals inside the minute leave nothing: {:?}",
            lines.all()
        );

        // Idle past the minute; the transport comes back, and nothing failed
        // in between.
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS + 1)).await;
        host.set(HOST_TRANSPORT_READY, TransportHealth::Ready);
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "fixture: the clock alone reads spent — silent and failing for over a minute"
        );

        let outcome = client.get_lightd_info().await;
        let all = lines.all();
        assert!(
            lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
            "left an ACCEPTING private path on a failing run ONE SECOND long: the run was never \
             seen to outlast the minute, and an idle gap is not a failure: {all:?}"
        );
        assert_eq!(
            all.last().map(arm_class_outcome),
            Some(("host", "sync", "connected")),
            "the accept is USED: {all:?}"
        );
        outcome.expect("and the accepted private path carries the RPC");
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "a confirmed RPC ended the failing run"
        );
    }

    /// `tor-patience-phase-2.md` §5 / §2c: the ephemeral-detect poll keeps the
    /// strict rule, and so does the sync-server probe.
    ///
    /// "A connect is the only evidence" was justified by the swap on-ramp alone
    /// — HTTP from a peer crate, no RPC that can report back — and applied to a
    /// whole bucket that also holds these two, which ride `LightwalletdClient`
    /// and DO confirm. For them it reopens the hole the window exists to close:
    /// every poll's connect is accepted, restarts the window, and the poll that
    /// finds stranded funds insists on a blackhole for ever.
    ///
    /// A FRESH client per poll, as the wallet mints them, so every poll dials —
    /// the one after the window first reads spent is the one that leaves, and
    /// each poll after THAT dials once more and is refused on clearnet again
    /// (no listener). The count is therefore taken at the end of the poll that
    /// switched, never after a fixed number of polls: the first RPC timeout is
    /// where the failing run is first OBSERVED, so the window reads spent one
    /// budget later than the clock alone would say.
    #[tokio::test(start_paused = true)]
    async fn the_ephemeral_detect_poll_keeps_the_strict_rule() {
        use crate::constants::{
            EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX, SYNC_SERVER_PROBE_ISOLATION_KEY_PREFIX,
            TOR_PATIENCE_SECS,
        };
        for prefix in [
            EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX,
            SYNC_SERVER_PROBE_ISOLATION_KEY_PREFIX,
        ] {
            let posture = Arc::new(TorPosture::new());
            let (lines, _capture) = capture_dial_lines(&posture);
            hold_the_paused_clock_to_small_steps();

            let endpoint = dead_loopback_endpoint().await;
            let policy = preferred_over(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>);
            let key = format!("{prefix}-0123456789abcdef");
            let class = PathClass::of(Some(&key));
            let poll = || {
                LightwalletdClient::connect_plaintext_over_test_dialer(
                    &endpoint,
                    &policy,
                    Some(key.clone()),
                    &posture,
                    None,
                )
                .expect("lazy connect never blocks")
            };

            // Polls, each accepted and never answered, until the window reads
            // spent: three budgets, 90 s. Under the rule this row keeps out
            // every accept restarts it and it never does — five polls, 150 s,
            // is more than enough to tell.
            let mut polls = 0;
            while !posture.may_switch_to_direct(class) {
                polls += 1;
                assert!(
                    polls <= 5,
                    "`{prefix}`: 150 s of accepted-and-silent polls and the window is NOT spent \
                     — a bare connect restarted it, on a path whose RPCs can confirm: {:?}",
                    lines.all()
                );
                assert!(poll().get_lightd_info().await.is_err());
                assert!(
                    lines.of_arm("sdk_direct").is_empty() && !posture.fell_back(),
                    "`{prefix}`: left BEFORE the window read spent, at {:?}: {:?}",
                    lines.started.elapsed(),
                    lines.all()
                );
            }
            assert!(
                lines.started.elapsed() >= Duration::from_secs(TOR_PATIENCE_SECS),
                "`{prefix}`: the window read spent at {:?} — INSIDE the founder's minute",
                lines.started.elapsed()
            );

            // The next poll leaves — and is the ONLY one measured after it.
            assert!(poll().get_lightd_info().await.is_err());
            let first_clearnet = lines.of_arm("sdk_direct").first().cloned().unwrap_or_else(|| {
                panic!(
                    "`{prefix}`: the window read spent {polls} polls in, and the next poll still \
                     rode the accepted-and-silent private path: {:?}",
                    lines.all()
                )
            });
            assert_one_visible_switch(&lines, &posture, &first_clearnet, class, "other");
        }
    }

    /// A private path that never answers the DIAL (bounded by the SDK to
    /// `DIAL_TIMEOUT_SECS`).
    struct HangingPrivatePath;

    #[async_trait]
    impl NetDialer for HangingPrivatePath {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            std::future::pending().await
        }
    }

    /// A clearnet leg whose TCP takes `connect` and whose peer then never speaks.
    struct SlowSilentClearnet {
        connect: Duration,
        silent: Arc<SilentDialer>,
    }

    #[async_trait]
    impl NetDialer for SlowSilentClearnet {
        async fn dial(
            &self,
            host: &str,
            port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, crate::error::DialError> {
            tokio::time::sleep(self.connect).await;
            self.silent.dial(host, port, isolation_key).await
        }
    }

    /// §3.1 ASSERTION 7 — TLS IS A LEG.
    ///
    /// The switch happens INSIDE one RPC: the private dial hangs for its whole
    /// 25 s bound, and what is left of the caller's 30 s has to cover the
    /// clearnet TCP connect, the TLS handshake and the RPC. Here TCP takes 4 s
    /// and the handshake wedges. One bound over the whole post-switch
    /// establishment gives up inside the caller's wait; a fresh bound for the
    /// handshake does not (25 + 4 + 5 is already past 30, and today's
    /// `DIAL_TIMEOUT_SECS` for it is 25 + 4 + 25).
    ///
    /// Built on the connector, like `a_wedged_tls_handshake_fails_in_connect_
    /// time_not_stream_time`, because a test needs a clearnet arm it can slow
    /// down and `resolve_dialer` offers only the real one.
    #[tokio::test(start_paused = true)]
    async fn the_post_switch_handshake_shares_one_budget_with_the_clearnet_connect() {
        use crate::constants::TOR_PATIENCE_SECS;
        use crate::net::dialer::PolicyDialer;
        let posture = Arc::new(TorPosture::new());
        let dialer = PolicyDialer::with_fallback(
            Arc::new(HangingPrivatePath) as Arc<dyn NetDialer>,
            Arc::new(SlowSilentClearnet {
                connect: Duration::from_secs(4),
                silent: SilentDialer::new(),
            }) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );
        let mut connector = DialerConnector {
            dialer: Arc::new(dialer),
            tls: Some(Arc::new(GrpcTls::new())),
            host: "example.com".into(),
            port: 443,
            isolation_key: Some(crate::constants::WALLET_SYNC_ISOLATION_KEY.to_owned()),
            connection: Arc::default(),
        };
        let uri = || Uri::from_static("https://example.com");

        // A genuine minute: the first hang starts the failing run.
        assert!(tower::Service::call(&mut connector, uri()).await.is_err());
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS + 1)).await;
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "fixture: the window is spent"
        );

        let started = tokio::time::Instant::now();
        let res = tower::Service::call(&mut connector, uri()).await;
        let waited = started.elapsed();
        assert!(posture.fell_back(), "fixture: this establishment switched");
        assert!(res.is_err(), "fixture: the clearnet handshake is wedged");
        assert!(
            waited <= Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
            "the establishment that switched took {waited:?}: {DIAL_TIMEOUT_SECS} s of private \
             dial, 4 s of clearnet TCP, and then a handshake with a bound of its own. The \
             caller's whole wait is {GRPC_UNARY_TIMEOUT_SECS} s, so the clearnet packet left, \
             the wallet latched, and the RPC it was for had already been abandoned"
        );
    }

    // ── broadcast seam (inc-2d-3-a) ──────────────────────────────────────────

    #[test]
    fn submit_outcome_maps_send_response_code_and_drops_the_message() {
        // §8 `submit_outcome_maps_send_response_code_and_drops_the_message`: error_code
        // 0 ⇒ Accepted; any non-zero ⇒ Rejected carrying the CODE only. The §5.4-sensitive
        // error_message (it can echo a txid/amount) is structurally absent from
        // SubmitOutcome — there is nowhere for it to leak. The message is set on BOTH
        // fixtures to prove it is dropped regardless.
        let accepted = SubmitOutcome::from_send_response(SendResponse {
            error_code: 0,
            error_message: "txid abcdef… would leak here".into(),
        });
        assert_eq!(accepted, SubmitOutcome::Accepted);

        for code in [-25, 1, 26, i32::MAX, i32::MIN] {
            let rejected = SubmitOutcome::from_send_response(SendResponse {
                error_code: code,
                error_message: "16: bad-txns-inputs-missingorspent".into(),
            });
            assert_eq!(
                rejected,
                SubmitOutcome::Rejected { code },
                "non-zero error_code carries the code, never the message"
            );
        }
    }

    #[tokio::test]
    async fn send_transaction_rides_the_host_dialer_on_a_fresh_circuit() {
        // §8 `send_transaction_rides_the_host_dialer_on_a_fresh_circuit`: the broadcast
        // RPC reaches the network ONLY through the host's NetDialer (ADR-0526), with the
        // CALLER's per-broadcast isolation key (its own circuit, §2.3 — NOT the sync key).
        // The dead duplex makes the submit fail TYPED (the tx never reached the endpoint),
        // never a hang/panic. The raw bytes are arbitrary (this seam owns no tx semantics).
        let dialer = RecordingDialer::new();
        let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::clone(&dialer) as Arc<dyn NetDialer>),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some("wallet-send-deadbeef".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");

        let r = client.send_transaction(vec![0x05, 0x00, 0x00, 0x80]).await;
        assert!(
            matches!(
                r,
                Err(GrpcError::Transport { .. }) | Err(GrpcError::Timeout { .. })
            ),
            "no server ⇒ a typed transport/timeout error, never a hang or a false Accepted"
        );
        let calls = dialer.calls();
        assert_eq!(calls.len(), 1, "exactly one dial, through the host dialer");
        assert_eq!(
            calls[0],
            (
                "localhost".to_owned(),
                9067,
                Some("wallet-send-deadbeef".to_owned())
            ),
            "broadcast rode the per-tx isolation key — its own circuit, not the sync one"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn send_transaction_silent_accepting_server_times_out_with_tor_stall() {
        // §8 OPERATIONAL: the MONEY-critical broadcast RPC on a DPI/censor middlebox that
        // completes the TCP+TLS handshake but never sends gRPC response headers must surface a
        // typed `Timeout` (→ GrpcFailure → 3-b resubmission), NEVER a hang and NEVER a false
        // `Accepted`. The committed silent-server test covers `get_lightd_info`; this pins the
        // SAME bound on `send_transaction` (it shares `map_unary`/`timeout`, but the broadcast
        // RPC is the one moving money — a regression wrapping it un-timed would hang on a censor
        // and is caught here). The stall a TIMEOUT carries is `TIMEOUT_STALL`, under Required
        // too (stage S1 `truth`, the F-C correction: an accepted-and-silent connection says
        // nothing about the path — this row used to pin `TorUnavailable` here, the over-claim
        // the correction removed; its unary sibling is superseded the same way). (Virtual
        // clock: tokio auto-advances to the unary deadline — no real wait.)
        let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some("wallet-send-cafe".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");

        let start = tokio::time::Instant::now();
        let r = client.send_transaction(vec![0x05, 0x00, 0x00, 0x80]).await;
        let elapsed = start.elapsed();
        assert!(
            matches!(
                r,
                Err(GrpcError::Timeout {
                    stall: StallReason::EndpointUnreachable
                })
            ),
            "a silent-accepting censor on the broadcast RPC times out typed, claiming nothing \
             about the path: {r:?}"
        );
        assert!(
            elapsed >= Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
            "the unary bound fired on the broadcast RPC (gate-7), never a hang"
        );
    }

    // ── the private path tells the truth (stage S1 `truth`, §3.0 F-C) ────────

    /// REPLACES `unary_silent_accepting_server_times_out_with_tor_stall` for
    /// the accepted-and-silent case (`stage-1-private-path-truth.md` §3.0 F-C,
    /// §3.2 "`Required`, two cases"; the superseded row was deleted at the
    /// S1 `truth` test-side repair — `docs/adjudication/s1-truth/ruling.md`
    /// VERDICT row 3). That row pinned the stall a `Required`
    /// timeout carries as `TorUnavailable`, and that is the over-claim: the
    /// dial was ACCEPTED, so the private path is up as far as anything here can
    /// see, and what did not answer is the far end — the path OR the server.
    /// The reason such a failure carries is the endpoint's,
    /// `EndpointUnreachable`, which `live_tor_state` renders as `Active` with
    /// a stalled sync (`required_with_a_wedged_endpoint_stays_active_tor_is_up`)
    /// and never as "the private path is unavailable". The unary bound still
    /// fires, exactly as before: only the WORD for it changes.
    ///
    /// The other `Required` case is the control: a dial the private path
    /// REFUSES made no connection at all, and `TorUnavailable` is the honest
    /// reason for that. The wallet-level row
    /// `required_over_a_refused_dial_stays_unavailable_however_long_it_refuses`
    /// holds it past the minute.
    #[tokio::test(start_paused = true)]
    async fn unary_silent_accepting_server_under_required_no_longer_blames_the_private_path() {
        let endpoint = LightServerEndpoint::new("http://localhost:9067").expect("endpoint");
        let policy = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::new(SilentPeerDialer) as Arc<dyn NetDialer>),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some("sync".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");

        let start = tokio::time::Instant::now();
        let r = client.get_lightd_info().await;
        let elapsed = start.elapsed();
        assert!(
            matches!(
                r,
                Err(GrpcError::Timeout {
                    stall: StallReason::EndpointUnreachable
                })
            ),
            "an ACCEPTED connection that carried nothing is the far end not answering — the \
             path or the server — never \"the private path is unavailable\": {r:?}"
        );
        assert!(
            elapsed >= Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
            "the unary bound still fires; only its reason changed"
        );

        // The control: a REFUSED dial made no connection, and keeps its reason.
        let refused = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::new(crate::ports::testing::FailingDialer)),
        };
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &refused,
            Some("sync".into()),
            &Arc::new(TorPosture::new()),
            None,
        )
        .expect("lazy connect never blocks");
        let r = client.get_lightd_info().await;
        assert!(
            matches!(
                r,
                Err(GrpcError::Transport {
                    stall: StallReason::TorUnavailable
                }) | Err(GrpcError::Timeout {
                    stall: StallReason::TorUnavailable
                })
            ),
            "a dial the private path refused is the private path unavailable — that word stays: {r:?}"
        );
    }
}

/// A scripted source that yields its items and then STALLS — never another
/// message, never an end — so the pump's per-message
/// [`GRPC_STREAMING_TIMEOUT_SECS`] bound turns it into a MID-DRAIN
/// `GrpcError::Timeout`, the one fault [`testing::scripted_stream`] cannot
/// raise. Drive it under a paused tokio clock (`#[tokio::test(start_paused =
/// true)]`), which auto-advances to the deadline. S15-F1's final fold (the
/// mid-drain timeout at a non-zero start); appended at the end of this file so
/// no cited line above it moves.
#[cfg(test)]
pub(crate) mod stall_testing {
    use super::*;
    use std::collections::VecDeque;

    struct StallingSource<T> {
        items: VecDeque<Result<Option<T>, tonic::Status>>,
    }

    #[async_trait]
    impl<T: Send + 'static> MessageSource<T> for StallingSource<T> {
        async fn message(&mut self) -> Result<Option<T>, tonic::Status> {
            match self.items.pop_front() {
                Some(item) => item,
                None => std::future::pending().await,
            }
        }
    }

    /// A [`MessageStream`] over `items`, then a stall.
    pub(crate) fn stalling_stream<T: Send + 'static + StreamStatus>(
        items: impl IntoIterator<Item = Result<Option<T>, tonic::Status>>,
    ) -> MessageStream<T> {
        MessageStream {
            source: Some(Box::new(StallingSource {
                items: items.into_iter().collect(),
            })),
            stall: StallReason::EndpointUnreachable,
            witness: None,
        }
    }
}
