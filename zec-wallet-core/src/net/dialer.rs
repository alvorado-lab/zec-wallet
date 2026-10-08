//! `TorPolicy` → dialer resolution + the SDK's one built-in dialer (spec §3.2a,
//! ADR-0526). This is the SINGLE place a policy becomes a concrete network
//! path: the wallet rides the HOST app's network infrastructure (Tor
//! today, xray/VLESS tomorrow) through the protocol-agnostic [`NetDialer`] —
//! the SDK never knows or names the host's transport.
//!
//! The connector (`net/grpc.rs`) consumes a [`DialPlan`]: one ready
//! `Arc<dyn NetDialer>` that already encapsulates the §3.2a failure policy
//! (`Preferred` falls back to clearnet VISIBLY; `Required` fail-closes — zero
//! clearnet packets), plus the [`StallReason`] that dialer's ultimate failure
//! surfaces. The connector never re-implements policy; it dials and maps.
//
// Staged: `resolve_dialer`/`DialPlan` are consumed by the gRPC connector
// (`net/grpc.rs`, the next inc-2c-ii unit); exercised by the §8 tests meanwhile
// (same discipline as `derive_default_address` was before the wallet handle).

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use async_trait::async_trait;
use tokio::net::TcpStream;

use crate::config::{TorPolicy, TorRuntime};
use crate::constants::{
    DIAL_ATTEMPT_STAGGER_MS, DIAL_MAX_ADDRS_PER_FAMILY, DIAL_TIMEOUT_SECS,
    FALLBACK_ESTABLISH_BUDGET_SECS,
};
use crate::error::DialError;
use crate::net::NetError;
use crate::net::dial_counters::DialCounters;
use crate::net::tor_posture::{DialArm, PathClass, TorPosture};
use crate::ports::{AsyncByteStream, NetDialer};
use crate::state::{StallReason, TorRuntimeKind};

/// A dialed stream, with the two facts about it that must travel WITH it and
/// that the public port cannot carry (`NetDialer` and `AsyncByteStream` are
/// host-facing: no wallet-state concern rides on either — the DESIGN NOTE
/// below).
pub(crate) struct Dialed {
    pub(crate) stream: Box<dyn AsyncByteStream>,
    /// Which arm served THIS connection. The gRPC client reports every RPC's
    /// evidence against it, so a clearnet connection can never confirm the
    /// private path and a sibling's fall-back can never relabel a private one
    /// (`net/tor_posture.rs`, "evidence is attributed to the connection").
    pub(crate) served_by: DialArm,
    /// `Some` on a `Preferred` FALLBACK leg only: the instant by which the
    /// whole clearnet establishment must be done. The TCP connect has already
    /// been held to it; whoever layers TLS on this stream spends what is LEFT
    /// of it, never a fresh bound (`FALLBACK_ESTABLISH_BUDGET_SECS`).
    pub(crate) establish_by: Option<tokio::time::Instant>,
}

/// The crate-internal face of a plan's dialer: the public port, plus the dial
/// that keeps its verdict. One object serves both — the gRPC connector takes
/// the verdict, the swap on-ramp (a peer crate, HTTP, its own TLS) takes the
/// port and drops it.
#[async_trait]
pub(crate) trait PlanDialer: NetDialer {
    async fn dial_attributed(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Dialed, DialError>;
}

/// The resolved dialing plan for a policy (§3.2a): the dialer to use + the
/// stall reason a FAILED DIAL surfaces. `Required` → `TorUnavailable`
/// (fail-closed); everything else → `EndpointUnreachable` (ordinary offline, incl.
/// a `Preferred` run whose clearnet fallback ALSO failed). A transport failure
/// over a connection whose dial was ACCEPTED does not take this reason — it is
/// the far end's, `EndpointUnreachable` under every plan (`net/grpc.rs`,
/// `transport_stall`; stage S1 `truth`).
pub(crate) struct DialPlan {
    pub(crate) dialer: Arc<dyn PlanDialer>,
    pub(crate) stall_on_failure: StallReason,
    /// The posture whose private patience window an RPC over this plan may
    /// restart or spend — `Some` under `Preferred` AND `Required`, `None` under
    /// `Off` (ADR-0552 stage 1b; stage S1 `truth` Q2 for `Required`).
    ///
    /// `Off` has no private path to have an opinion about, so nothing an RPC
    /// over it does is evidence. `Required` can never switch, but its RPCs are
    /// the ONLY evidence a host gets that an accepting path carries nothing
    /// (`TorState::Unanswered`), so its plan shares the wallet posture and
    /// reports into it like `Preferred`'s does. Deciding it HERE, where the
    /// policy is matched, is what keeps the gRPC layer from having to
    /// re-derive a policy question from a dialer it cannot see.
    pub(crate) private_window: Option<Arc<TorPosture>>,
}
// DESIGN NOTE (the INC-2D GATE, RESOLVED inc-2d-3-a): `TorState::FellBack` (§2.5) is
// driven by ONE wallet-level `Arc<TorPosture>` owned by `Inner` (`Inner.tor_posture`,
// which carries the sticky latch AND ADR-0552's patience clocks — a policy epoch must
// reset BOTH, see `net/tor_posture.rs`) that `resolve_dialer` shares into EVERY
// `Preferred` `PolicyDialer` the wallet builds — the sync client AND each fresh per-txid
// BROADCAST client (§2.3), so a window is shared per CIRCUIT FAMILY rather than minted
// fresh per dial (ADR-0552 stage 1b) and no degradation hides behind a stale `Active`
// (the §3.2g/S45 deferral). The earlier per-circuit `DialPlan.fell_back` handle
// + the engine-capture chain are GONE: one predicate, one source. Option (b) (a
// `fell_back` method on `NetDialer`) stays rejected — it would leak a wallet-state
// concern into the host-facing transport port; the signal is produced in the
// `PolicyDialer` and written THROUGH the shared flag, never exposed on the port.

/// Resolve a [`TorPolicy`] into a ready dialer (ADR-0526), sharing the wallet-level
/// `fell_back` latch into the one policy that can degrade (`Preferred`). `Dialer` (the
/// host-infrastructure path) is always available; `ExternalSocks5` and built-in arti are
/// named follow-ups → typed [`NetError::UnsupportedRuntime`], NEVER a silent clearnet
/// fall-through. `posture` is the caller's (`Inner`-owned) Tor posture: only the
/// `Preferred` dialer ever writes its latch; `Off` never sees it and `Required`
/// feeds only its windows (they never fall back). `counts` is the wallet's dial
/// tally (FR-37), handed to every dialer built here so `log_dial` counts what it
/// prints — `Off`'s direct dialer included, whose every dial is a clearnet one.
pub(crate) fn resolve_dialer(
    policy: &TorPolicy,
    posture: &Arc<TorPosture>,
    counts: &Arc<DialCounters>,
) -> Result<DialPlan, NetError> {
    match policy {
        // Explicit clearnet: the SDK's one built-in dialer. Not a hidden
        // fallback — the user's stated choice (§3.2a). Nothing to fall back FROM,
        // so the wallet flag is never written.
        TorPolicy::Off => Ok(DialPlan {
            dialer: direct(counts),
            stall_on_failure: StallReason::EndpointUnreachable,
            // No private path exists, so no RPC over this plan says anything
            // about one.
            private_window: None,
        }),
        // Tor-when-reachable: host runtime first, VISIBLE clearnet fallback that writes
        // the wallet-level latch so `tor_state()` surfaces the degradation (the INC-2D
        // GATE — the same flag every other circuit the wallet dials shares).
        TorPolicy::Preferred { runtime } => Ok(DialPlan {
            dialer: Arc::new(PolicyDialer::with_fallback(
                runtime_dialer(runtime)?,
                direct(counts),
                Arc::clone(posture),
                Arc::clone(counts),
            )),
            // ADR-0552's honesty half was BUILT HERE AND WITHDRAWN THE SAME DAY
            // (the security angle's HIGH). It read the latch to report
            // `TorUnavailable` while the wallet was still insisting. Two facts
            // refute that shape, and both are seams a unit test cannot see:
            // (1) THE PLAN IS BUILT ONCE FOR THE LONG-LIVED SYNC CLIENT
            // (`wallet.rs`: "obtained on the first pass + REUSED"), so the
            // reason freezes at "not yet switched" for the whole session — and
            // keeps claiming it after the wallet HAS gone to clearnet; and
            // (2) `walletTorUnavailable`'s copy says "nothing was sent in the
            // clear", so that frozen reason would tell a censored user no
            // clearnet traffic occurred at the moment their address reached the
            // lightwalletd operator. (3) It was also stamped on EVERY transport
            // failure `map_unary` classified, not just dial-origin ones, so a
            // wedged server over a healthy circuit would read as a dead private
            // path — since stage S1 `truth` the plan's reason reaches only a
            // FAILED dial (`net/grpc.rs`, `transport_stall`), but (1) and (2)
            // stand on their own. The honest state needs the reason resolved
            // AT FAILURE TIME and its OWN copy — stage 2, with the notification
            // edge. Until then this is the pre-ADR-0552 reason, which claims
            // nothing.
            stall_on_failure: StallReason::EndpointUnreachable,
            // THE one policy with a private window to keep: an RPC completing
            // over this plan is the evidence that restarts the maintainer's minute
            // for its circuit family (ADR-0552 stage 1b).
            private_window: Some(Arc::clone(posture)),
        }),
        // Fail-closed: host runtime ONLY, never clearnet. A DIAL failure is
        // `TorUnavailable` + zero packets past the failed dial — it never falls back,
        // so the wallet latch stays untouched (Required can never be `FellBack`).
        // A dial that FAILED is the only thing that stamps this reason. Over a
        // connection the path ACCEPTED, neither a timeout nor a transport
        // failure does — a TLS handshake that never completes, a peer that
        // hangs up, a link that dies mid-RPC are the far end's,
        // `EndpointUnreachable` — because from there a wedged server and a
        // path that carries nothing cannot be told apart (`net/grpc.rs`,
        // `TIMEOUT_STALL` and `transport_stall`: the F-C correction and its
        // repair on the join).
        TorPolicy::Required { runtime } => Ok(DialPlan {
            dialer: Arc::new(PolicyDialer::fail_closed(
                runtime_dialer(runtime)?,
                Arc::clone(posture),
                Arc::clone(counts),
            )),
            stall_on_failure: StallReason::TorUnavailable,
            // The WALLET posture, shared like `Preferred`'s (stage S1 `truth`
            // Q2): `Required` never switches, but its dials and RPCs are the
            // evidence behind `TorState::Unanswered`, which a fail-closed
            // wallet needs MORE than a `Preferred` one — nothing else will
            // ever tell it that an accepting path carries nothing. The
            // fallback arm is structurally unreachable with no fallback, so
            // sharing the posture cannot latch — pinned past the minute by
            // `required_reports_not_carrying_at_the_minute_and_never_the_switch`
            // (`wallet/tests/private_path_truth.rs`) and
            // `impl_probe_required_never_dials_clearnet_over_the_same_blackhole`
            // (`net/grpc_truth_tests.rs`), both asserting `!fell_back()`; it
            // does make a future `set_tor_policy` epoch reset load-bearing for
            // the windows too (`net/tor_posture.rs`, the field doc on
            // `fell_back`).
            private_window: Some(Arc::clone(posture)),
        }),
    }
}

/// Map a `TorRuntime` to its host dialer. Matched exhaustively WITHIN the crate
/// (the `non_exhaustive` lint only binds downstream), so a future runtime
/// variant is a compile error here — it can never silently degrade to clearnet.
fn runtime_dialer(runtime: &TorRuntime) -> Result<Arc<dyn NetDialer>, NetError> {
    match runtime {
        // THE host-network path (ADR-0526): the host hands us its transport.
        TorRuntime::Dialer(d) => Ok(Arc::clone(d)),
        // The REGISTERED cross-library dialer (FR-29; ADR-0543): a `HostDialer`
        // IS a `NetDialer`, so `PolicyDialer` wraps it like any other. Of the
        // host's five dial codes only `Unreachable` and `Timeout` are
        // switch-eligible (ADR-0546): `NotReady`, `Refused` (→ `Unsupported`)
        // and `Retired` land in the surface-as-is arm by construction and
        // never reach clearnet. The gate in front adds ONE refusal of its own,
        // `TransportFailed`, read from the descriptor's health and never from a
        // code. Nothing-registered never arrives here: the door refused it
        // (`config::validate_transport`, spec §6.1 E1); a registration CLEARED
        // mid-session fails each dial `Retired`, which is the same arm.
        TorRuntime::HostDialer(host) => Ok(super::readiness_gate::gated(host)),
        TorRuntime::ExternalSocks5 { .. } => Err(NetError::UnsupportedRuntime {
            runtime: TorRuntimeKind::ExternalSocks5,
        }),
        // NO fourth arm: the SDK-owned runtime (`BuiltIn`, feature `tor-builtin`)
        // was REMOVED at FR-5 C1 (ADR-0548 D3) — a host without a transport adds
        // the `zec_wallet_tor` plugin, which registers through `HostDialer` above.
        // The match stays exhaustive over the three variants (no wildcard arm).
    }
}

/// Concrete, so it coerces to either face: `Off`'s plan dialer, or the port
/// `Preferred` keeps as its fallback. Counts into the wallet's tally like every
/// dialer the plan builds (FR-37).
fn direct(counts: &Arc<DialCounters>) -> Arc<DirectTcpDialer> {
    Arc::new(DirectTcpDialer::new(Arc::clone(counts)))
}

/// The SDK's ONE clearnet dialer: a happy-eyeballs-style connect (FR-21,
/// RFC 8305-lite) bounded by [`DIAL_TIMEOUT_SECS`]. Used SOLELY for
/// `TorPolicy::Off` and as the VISIBLE `Preferred` fallback — never reachable
/// under `Required` (§3.2a / ADR-0526).
///
/// Why not a bare `TcpStream::connect((host, port))`: that tries resolved
/// addresses SEQUENTIALLY in resolver order, so an unroutable/blackholed IPv6
/// leg starves the IPv4 fallback — device-reproduced (the FR-21 P1): on
/// IPv4-only-global Wi-Fi against the dual-stack endpoint EVERY sync dial
/// failed while transports with their own dual-stack fallback connected fine.
/// Instead: resolve via `lookup_host`, interleave families with an IPv4
/// HEAD-START, and race attempts [`DIAL_ATTEMPT_STAGGER_MS`] apart — first
/// established stream wins, laggards abort. IPv4-first deliberately inverts
/// RFC 8305's IPv6 preference: this dialer exists for reachability, not
/// address-family advocacy, and the observed field failures are broken-v6
/// networks; a v6-only network still connects — its attempts simply race one
/// stagger later (or immediately, on the head-start's definitive failure).
///
/// Accepted residual (reliability review, speculative): the race judges TCP
/// ESTABLISHMENT only, so a middlebox that ACCEPTs on a broken family wins the
/// race and fails later at the connector's TLS-handshake bound (`net/grpc.rs`),
/// and a redial repeats the choice. If ever field-hit, the escalation is
/// remembering a last-good family per endpoint — tracked on the #377 backlog.
pub(crate) struct DirectTcpDialer {
    /// The wallet's dial tally (FR-37): every dial this dialer performs is a
    /// clearnet one and is counted as such through `log_dial`.
    counts: Arc<DialCounters>,
}

impl DirectTcpDialer {
    pub(crate) fn new(counts: Arc<DialCounters>) -> Self {
        Self { counts }
    }
}

/// Order resolved addresses for the race: interleave families starting with
/// IPv4 (`v4[0], v6[0], v4[1], v6[1], …`), preserving resolver order WITHIN
/// each family (the resolver's intra-family ranking is still respected) and
/// capping each family at [`DIAL_MAX_ADDRS_PER_FAMILY`]. A v4-mapped
/// `::ffff:a.b.c.d` (AI_V4MAPPED resolvers) is REBUILT as a genuine V4 socket
/// addr — it takes the head-start slot AND dials `AF_INET` on the wire (a
/// mapped dial through an `AF_INET6` socket fails `EAFNOSUPPORT` on a
/// v6-stack-disabled host, the exact broken-v6 family this dialer exists
/// for). Genuine v6 addrs pass through UNTOUCHED — rebuilding one would drop
/// a link-local scope id.
fn interleave_v4_first(addrs: impl IntoIterator<Item = SocketAddr>) -> Vec<SocketAddr> {
    let mut v4: Vec<SocketAddr> = Vec::new();
    let mut v6: Vec<SocketAddr> = Vec::new();
    let mut dropped = 0usize;
    for addr in addrs {
        let (addr, is_v4) = match addr.ip().to_canonical() {
            std::net::IpAddr::V4(ip) if !addr.is_ipv4() => {
                (SocketAddr::from((ip, addr.port())), true)
            }
            canonical => (addr, canonical.is_ipv4()),
        };
        let bucket = if is_v4 { &mut v4 } else { &mut v6 };
        if bucket.len() < DIAL_MAX_ADDRS_PER_FAMILY {
            bucket.push(addr);
        } else {
            dropped += 1;
        }
    }
    if dropped > 0 {
        // Count-only (§5.4 — never an address): the cap is visible, not silent.
        tracing::debug!(target: "zec_wallet_core", dropped, "wallet.dial.addrs_truncated");
    }
    let mut ordered = Vec::with_capacity(v4.len() + v6.len());
    let (mut i4, mut i6) = (v4.into_iter(), v6.into_iter());
    loop {
        match (i4.next(), i6.next()) {
            (None, None) => break,
            (a, b) => {
                ordered.extend(a);
                ordered.extend(b);
            }
        }
    }
    ordered
}

/// Race `connect` over `addrs` with staggered starts: attempt `i` begins at
/// `i × DIAL_ATTEMPT_STAGGER_MS` — or EARLIER, the moment `i` prior attempts
/// have already failed (RFC 8305 §5: a definitive failure starts the next
/// attempt immediately, so a v6-only NAT64 network whose v4 head-start dies
/// `ENETUNREACH` in ~1 ms pays ~nothing, not a full stagger) — while every
/// earlier attempt keeps running; the first success wins, and dropping the
/// `JoinSet` aborts the laggards (no leaked half-open connects; a laggard
/// stream that established just before the drop is closed with it).
///
/// When ALL attempts fail, the surfaced error is chosen DETERMINISTICALLY by
/// diagnostic value — NOT completion order (which varies with RTT and would
/// systematically favor the last, least-informative leg): a refused/reset
/// connect proves the network path WORKS and the endpoint answered (a
/// server-side problem) and must never be buried under this device's
/// unreachable-family noise; ties break to address order (the preferred
/// family). Generic over the connect fn so the race is deterministically
/// testable on the paused clock with scripted outcomes — no real sockets.
async fn connect_staggered<T, F, Fut>(
    mut addrs: Vec<SocketAddr>,
    connect: F,
) -> Result<T, DialError>
where
    T: Send + 'static,
    F: Fn(SocketAddr) -> Fut,
    Fut: Future<Output = Result<T, DialError>> + Send + 'static,
{
    // Belt: the race itself enforces the fan-out cap even if a
    // caller bypasses the interleave (whose per-family caps normally enforce
    // it) — the tail-headroom const assert reasons about ≤ 2·cap attempts,
    // so the race must never run more. Count-only log (§5.4), never silent.
    if addrs.len() > 2 * DIAL_MAX_ADDRS_PER_FAMILY {
        let dropped = addrs.len() - 2 * DIAL_MAX_ADDRS_PER_FAMILY;
        tracing::debug!(target: "zec_wallet_core", dropped, "wallet.dial.addrs_truncated");
        addrs.truncate(2 * DIAL_MAX_ADDRS_PER_FAMILY);
    }
    let attempt_count = addrs.len();
    let failures = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let one_failed = Arc::new(tokio::sync::Notify::new());
    // ONE schedule baseline, captured before any spawn: a per-task
    // `Instant::now()` would let a loaded executor's first-poll skew reorder
    // the head start.
    let base = tokio::time::Instant::now();
    let mut attempts = tokio::task::JoinSet::new();
    for (i, addr) in addrs.into_iter().enumerate() {
        let attempt = connect(addr);
        let failures = Arc::clone(&failures);
        let one_failed = Arc::clone(&one_failed);
        attempts.spawn(async move {
            // Start gate: the stagger timer OR `i` accumulated failures,
            // whichever first. The counter can also carry a LATER-indexed
            // attempt's failure, which is safe by monotonicity: for attempt
            // j > i to have started at all, either the count already reached
            // j (> i, so this gate was open) or j's timer fired (later than
            // i's, so i's had fired too) — no interleaving opens a gate
            // early.
            let start_at = base + Duration::from_millis(i as u64 * DIAL_ATTEMPT_STAGGER_MS);
            let mut timer = std::pin::pin!(tokio::time::sleep_until(start_at));
            loop {
                // ORDERING IS LOAD-BEARING: `notified()` snapshots the
                // notify_waiters EPOCH at CREATION (tokio's lost-wakeup
                // guarantee — a future created before a `notify_waiters`
                // completes on its first poll, notify.rs epoch check), so
                // creating it BEFORE the counter load closes the load→await
                // window. Three independent reviews stumbled on this exact
                // line — do not move `notified()` below the check, and do
                // not re-explain it as waiter "registration" (registration
                // happens at first poll; the CREATION epoch is the guard).
                let woken = one_failed.notified();
                if failures.load(Ordering::SeqCst) >= i {
                    break;
                }
                tokio::select! {
                    _ = &mut timer => break,
                    _ = woken => {}
                }
            }
            let outcome = attempt.await;
            if outcome.is_err() {
                failures.fetch_add(1, Ordering::SeqCst);
                one_failed.notify_waiters();
            }
            (i, outcome)
        });
    }
    let drain = async move {
        let mut errors: Vec<(usize, DialError)> = Vec::new();
        while let Some(joined) = attempts.join_next().await {
            match joined {
                // First established stream wins; the `attempts` drop aborts the rest.
                Ok((_, Ok(stream))) => return Ok(stream),
                Ok((i, Err(e))) => errors.push((i, e)),
                // A raced attempt panicked (or was cancelled — defensive: the
                // JoinSet is only ever cancelled wholesale by drop, after
                // which this drain stops being polled). Treat it EXACTLY like
                // a failed attempt: the unwind skipped the task's own
                // accounting, so count it into the gate HERE — later legs
                // must still wake early (a panicked head start
                // previously left every later leg sleeping out its full
                // stagger). Keep draining; never poison the dial for one
                // leg's crash. STATIC reason only (§5.4 structural):
                // `JoinError`'s Display forwards the panic payload verbatim.
                Err(join_err) => {
                    failures.fetch_add(1, Ordering::SeqCst);
                    one_failed.notify_waiters();
                    let reason = if join_err.is_panic() {
                        "panic"
                    } else {
                        "cancelled"
                    };
                    tracing::debug!(target: "zec_wallet_core", reason, "wallet.dial.attempt_join_failed");
                }
            }
        }
        Err(
            match errors
                .into_iter()
                .min_by_key(|(i, e)| (error_diagnostic_rank(e), *i))
            {
                Some((_, e)) => e,
                // Distinguish "nothing to dial" from the near-unreachable
                // "every leg died without a verdict" — the old shared message
                // blamed DNS for a resolution that succeeded.
                None if attempt_count == 0 => {
                    DialError::Io(std::io::Error::other("dns resolved no addresses"))
                }
                None => DialError::Io(std::io::Error::other("all dial attempts aborted")),
            },
        )
    };
    // The race carries its OWN [`DIAL_TIMEOUT_SECS`] bound (the security
    // review's surviving mutant: the caller-side belt in `dial` also covers
    // resolution, but the race must never RELY on a caller remembering it —
    // this is the bound the paused-clock test pins). All legs blackholed ⇒
    // typed `Timeout` here; the timeout drop aborts every attempt.
    match tokio::time::timeout(Duration::from_secs(DIAL_TIMEOUT_SECS), drain).await {
        Ok(r) => r,
        Err(_elapsed) => Err(DialError::Timeout),
    }
}

/// Diagnostic rank for the all-fail verdict (lower = more informative to a
/// field report): a refused/reset connect is the one signal that proves the
/// path end-to-end (the endpoint's stack answered), so it outranks every
/// other concrete IO error, which in turn outranks the shapeless kinds.
fn error_diagnostic_rank(e: &DialError) -> u8 {
    match e {
        DialError::Io(io)
            if matches!(
                io.kind(),
                std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionReset
            ) =>
        {
            0
        }
        DialError::Io(_) => 1,
        _ => 2,
    }
}

/// The `DirectTcpDialer` post-resolution path: interleave (IPv4 head-start,
/// per-family cap) + staggered race. Split from `dial` so the ordering WIRING
/// is unit-pinned — the interleave must be applied on the production path,
/// not merely exist as a function (a fresh-eyes review gap: deleting it from
/// the path left every test green).
async fn direct_dial_ordered<T, F, Fut>(
    resolved: impl IntoIterator<Item = SocketAddr>,
    connect: F,
) -> Result<T, DialError>
where
    T: Send + 'static,
    F: Fn(SocketAddr) -> Fut,
    Fut: Future<Output = Result<T, DialError>> + Send + 'static,
{
    connect_staggered(interleave_v4_first(resolved), connect).await
}

/// THE PER-DIAL ATTRIBUTION LINE (`on-device-log-layer-phase-1.md` §3a).
///
/// Neither side could show, from a device, that a wallet dial rode the host's
/// transport: under `Required` the argument is a proof about the PROGRAM
/// (`fail_closed` has no fallback), and under `Preferred` there is not even
/// that, because both arms exist. This line is the observation. It is emitted
/// by the dialer that PERFORMED the attempt — [`PolicyDialer`] for its primary,
/// [`DirectTcpDialer`] for itself — so there is one line per physical dial and
/// no second site that could disagree with the first.
///
/// §5.4: three closed vocabularies and nothing else. The class is DERIVED from
/// the isolation key and is all of it that may appear; never a host, a port, the
/// key itself, the host's transport name, or an `Io` payload (the variant's NAME
/// only — `io::Error`'s `Display` can carry an address). The field names and
/// this reasoning are in `tracing_guard::ALLOWLIST`, which prices each
/// `dial_class` value it admits; the allowlist gate matches on the NAME, so the
/// VALUE SET has its own row
/// (`the_dial_class_vocabulary_is_exactly_the_priced_four`).
///
/// It ALSO counts (FR-37): `counts` is the wallet's tally by arm × outcome,
/// incremented here and nowhere else, so the number `Wallet::dial_counts`
/// reports and the line a device log shows are one observation. An increment
/// beside a call would be a second site that could disagree with this one.
fn log_dial<T>(
    counts: &DialCounters,
    arm: DialArm,
    class: PathClass,
    outcome: &Result<T, DialError>,
) {
    counts.record(arm, outcome);
    let dial_arm = match arm {
        DialArm::Private => "host",
        DialArm::Clearnet => "sdk_direct",
    };
    // Exhaustive on purpose, like `PathClass::slot`: a new class is a compile
    // error here rather than a line that silently files it under `other`.
    let dial_class = match class {
        PathClass::Sync => "sync",
        PathClass::Broadcast => "broadcast",
        PathClass::Swap => "swap",
        PathClass::Other => "other",
    };
    // THE LEVEL IS PART OF THE CONTRACT (FR-35): a host's "errors only" setting
    // carries WARN and above, so what is a WARN here decides what a production
    // user's log can say about a connection that did not happen. `wrong` = the
    // dial FAILED for a reason that is not a normal state of the transport.
    // `not_ready` (a bootstrap in progress) and `retired` (a carrier switch) are
    // normal, expected many times a day, and stay INFO with `connected`; a
    // private path that is unreachable, timed out, refused the request shape,
    // faulted, or that its own host has declared failed is what "errors only"
    // exists to show.
    let (outcome, wrong) = match outcome {
        Ok(_) => ("connected", false),
        Err(DialError::NotReady) => ("not_ready", false),
        Err(DialError::Retired) => ("retired", false),
        Err(DialError::Unreachable) => ("unreachable", true),
        Err(DialError::Timeout) => ("timeout", true),
        Err(DialError::Unsupported) => ("unsupported", true),
        Err(DialError::Io(_)) => ("io", true),
        Err(DialError::TransportFailed) => ("transport_failed", true),
    };
    // Two macro calls because a `tracing` callsite's level is static.
    if wrong {
        tracing::warn!(target: "zec_wallet_core", dial_arm, dial_class, outcome, "wallet.dial");
    } else {
        tracing::info!(target: "zec_wallet_core", dial_arm, dial_class, outcome, "wallet.dial");
    }
}

#[async_trait]
impl NetDialer for DirectTcpDialer {
    async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        // Clearnet has no per-circuit unlinkability to offer, so the
        // `isolation_key` is accepted and IGNORED FOR ROUTING: `Off` is the user's
        // explicit opt-out of privacy (returning `Unsupported` would break it).
        // Distinct keys share the one clearnet path because there is no circuit to
        // split. It is read for ONE thing — the class on the attribution line.
        let outcome = self.connect(host, port).await;
        log_dial(
            &self.counts,
            DialArm::Clearnet,
            PathClass::of(isolation_key),
            &outcome,
        );
        outcome
    }
}

/// `TorPolicy::Off`'s plan dialer: every connection is a clearnet one and none
/// is a fallback leg, so there is no budget to carry.
#[async_trait]
impl PlanDialer for DirectTcpDialer {
    async fn dial_attributed(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Dialed, DialError> {
        let stream = self.dial(host, port, isolation_key).await?;
        Ok(Dialed {
            stream,
            served_by: DialArm::Clearnet,
            establish_by: None,
        })
    }
}

impl DirectTcpDialer {
    /// The clearnet connect itself: resolve, race, bound, tune. Split from
    /// `dial` only so the attribution line sees EVERY exit — the early
    /// `Timeout` / resolver returns included — through one `Result`.
    async fn connect(&self, host: &str, port: u16) -> Result<Box<dyn AsyncByteStream>, DialError> {
        let race = async {
            // OS resolution (IP literals parse without a DNS query). Resolver
            // ORDER no longer decides reachability — the staggered race does.
            let resolved = tokio::net::lookup_host((host, port))
                .await
                .map_err(DialError::Io)?;
            // THE one dial route: resolution feeds the interleave+race ONLY
            // through this seam (its body is test-pinned; this call edge is
            // convention — reviews proved a `connect_staggered`-direct
            // bypass here keeps every test green while silently dropping the
            // IPv4 head-start; the race's own fan-out belt would still cap
            // it). Route any refactor through `direct_dial_ordered`.
            direct_dial_ordered(resolved, |addr| async move {
                TcpStream::connect(addr).await.map_err(DialError::Io)
            })
            .await
        };
        // Belt over resolve + race: the race carries its own internal
        // [`DIAL_TIMEOUT_SECS`] bound (pinned on the paused clock); this outer
        // bound additionally covers `lookup_host` (a hung resolver), so the
        // caller gets a typed `Timeout` either way, never an unbounded park.
        let stream = match tokio::time::timeout(Duration::from_secs(DIAL_TIMEOUT_SECS), race).await
        {
            Err(_elapsed) => return Err(DialError::Timeout),
            Ok(Err(e)) => return Err(e),
            Ok(Ok(s)) => s,
        };
        // Family-only observability (§5.4 — never host/port/address): which leg
        // won distinguishes an IPv4-only-network report from dual-stack at debug.
        if let Ok(peer) = stream.peer_addr() {
            tracing::debug!(
                target: "zec_wallet_core",
                family = if peer.ip().to_canonical().is_ipv4() { "v4" } else { "v6" },
                "wallet.dial.connected"
            );
        }
        // gRPC unary RPCs are small request/response pairs — Nagle's batching
        // only adds latency (the upstream lightwalletd clients disable it).
        // Best-effort: a failure here only forgoes the optimization, but log it
        // rather than swallow it (no silent failures, principle 10).
        if let Err(e) = stream.set_nodelay(true) {
            tracing::debug!(target: "zec_wallet_core", error = %e, "set_nodelay failed (best-effort)");
        }
        Ok(Box::new(stream))
    }
}

/// Wraps a host runtime dialer with the §3.2a failure policy. `Preferred`
/// (a `fallback` is present) degrades to clearnet once the private path has
/// carried nothing AND kept failing for the maintainer's minute — however it
/// fails: unreachable, timed out, declared failed by its host, or ACCEPTING and
/// silent — and surfaces it (`fell_back` + the `wallet.private_path_fell_back`
/// event); `Required` (no `fallback`) fail-closes — the fallback dial is NEVER
/// made, so a failed `Required` run emits zero clearnet packets.
///
/// WHAT THE ACCEPT ARM COSTS, stated rather than buried: once a class has been
/// SEEN failing past its window nothing but a confirmed private RPC ends that
/// run, and a class whose every dial now switches can never produce one. So a
/// class that has left does not return to the private path within the session,
/// even if that path recovers — where before it would have, on any redial the
/// primary accepted. That is the price of not trusting an accept, and an accept
/// is exactly what the blackhole offers. The latch is sticky and the state
/// reads `FellBack`, so it is visible; a new session insists again from zero.
///
/// And what it does NOT do: leave on the clock alone. An accept is left only
/// when a failure was observed after the minute had already run out; where the
/// minute ran out in silence (one failure, then an idle gap — a backgrounded
/// app) the accepted connection is used, and only its own failure sends the
/// next dial to clearnet. That is one RPC's delay on a path that really is
/// dead, paid so that a path that is fine is never abandoned unseen.
pub(crate) struct PolicyDialer {
    primary: Arc<dyn NetDialer>,
    fallback: Option<Arc<dyn NetDialer>>,
    /// The WALLET-LEVEL Tor posture (`Inner.tor_posture`), shared in by
    /// `resolve_dialer`: its latch makes a fall-back INSIDE this erased `Arc<dyn
    /// NetDialer>` visible to `tor_state()` no matter WHICH circuit (sync or any
    /// broadcast) degraded — the INC-2D GATE resolution — and its patience
    /// clocks (ADR-0552) are shared for the same reason, so the maintainer's minute
    /// is a window per CIRCUIT FAMILY rather than a fresh minute per dial.
    /// (Stage 1b split what was one wallet-wide window into one per
    /// [`PathClass`]: a wallet-wide window let a healthy sync circuit reset the
    /// clock a starved broadcast was measuring.) `fail_closed` shares the SAME
    /// wallet posture since stage S1 `truth` (Q2): it feeds the windows —
    /// they are what `TorState::Unanswered` reads — and never the latch, whose
    /// one writer is the fallback arm it does not have, so the latch still only
    /// ever reflects a genuine `Preferred` leak.
    posture: Arc<TorPosture>,
    /// The wallet's dial tally (FR-37), handed to `log_dial` with every
    /// primary line this dialer writes; the fallback counts its own.
    counts: Arc<DialCounters>,
}

impl PolicyDialer {
    pub(crate) fn with_fallback(
        primary: Arc<dyn NetDialer>,
        fallback: Arc<dyn NetDialer>,
        posture: Arc<TorPosture>,
        counts: Arc<DialCounters>,
    ) -> Self {
        Self {
            primary,
            fallback: Some(fallback),
            posture,
            counts,
        }
    }

    /// `Required`: no fallback, ever. `posture` is the WALLET's (stage S1
    /// `truth` Q2) — its windows take this dialer's evidence, its latch cannot
    /// be written from here (no fallback ⇒ `dial_attributed` never reaches
    /// `switch_to_direct`, the latch's one writer), which is the honest
    /// "Required cannot fall back" signal.
    pub(crate) fn fail_closed(
        primary: Arc<dyn NetDialer>,
        posture: Arc<TorPosture>,
        counts: Arc<DialCounters>,
    ) -> Self {
        Self {
            primary,
            fallback: None,
            posture,
            counts,
        }
    }

    /// Whether THIS dialer has degraded to clearnet — reads the shared latch the `dial`
    /// path writes. TEST-ONLY: production never reads fell-back here; it reads the
    /// wallet-level `Inner.tor_posture` (the SSOT every dialer shares) via
    /// `Wallet::tor_state()`. This accessor only lets the dialer unit tests observe a real
    /// fall-back on a `PolicyDialer` directly.
    #[cfg(test)]
    pub(crate) fn fell_back(&self) -> bool {
        self.posture.fell_back()
    }
}

/// The public port: the same dial, its verdict dropped. The swap on-ramp rides
/// this face (`Wallet::swap_dialer`); nothing of the wallet's state crosses it.
#[async_trait]
impl NetDialer for PolicyDialer {
    async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        self.dial_attributed(host, port, isolation_key)
            .await
            .map(|dialed| dialed.stream)
    }
}

#[async_trait]
impl PlanDialer for PolicyDialer {
    async fn dial_attributed(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Dialed, DialError> {
        // Bound the host dial at the SDK level (DIAL_TIMEOUT_SECS): a HUNG host
        // dialer — a blackholed connect or a dead Tor circuit that never RSTs, the
        // most common flaky-mobile failure — must become a typed Timeout, NEVER an
        // indefinite park that wedges the tonic channel worker (which even a
        // drop-rebuild can't clear, since it re-wraps the same dialer). The
        // guarantee never rests on the host dialer's own (possibly absent) timeout
        // (the constants.rs DIAL_TIMEOUT_SECS contract). NOTE (inc-2c-iv): tuning so
        // the `Preferred` fallback completes inside ONE RPC's unary budget (it needs
        // DIAL_TIMEOUT_SECS < GRPC_UNARY_TIMEOUT_SECS for the fast-fallback case) is
        // a recovery-loop concern; here the bound's job is "no dial hangs forever".
        // ADR-0552 stage 1b: the window has a SUBJECT. Sync rides one long-lived
        // circuit and every broadcast mints a fresh one, so a censor that
        // carries the established stream while refusing new circuits would, under
        // one wallet-wide window, let each sync pass reset the window a starved
        // send is measuring. Derived from the key the dial already carries.
        let class = PathClass::of(isolation_key);
        let primary = self.primary.dial(host, port, isolation_key);
        let outcome =
            match tokio::time::timeout(Duration::from_secs(DIAL_TIMEOUT_SECS), primary).await {
                Ok(r) => r,
                Err(_elapsed) => Err(DialError::Timeout),
            };
        // The PRIMARY's attribution, whatever it returned — the SDK-bounded
        // `Timeout` included. A fallback below is the clearnet dialer's own dial
        // and logs itself, so a switch reads as two lines: `private … timeout`,
        // then `clearnet … connected`.
        log_dial(&self.counts, DialArm::Private, class, &outcome);
        // WHAT THE PRIMARY'S ANSWER IS WORTH — recorded BEFORE the decision so a
        // run that began a minute ago is visible to it, and recorded for
        // `Required` too: it never reads the clock, but a posture that silently
        // under-counts would be a trap for whoever wires `set_tor_policy` (G3)
        // and lets a policy epoch change. `may_leave` is whether this answer is
        // one a `Preferred` wallet may EVER follow to clearnet (the error set,
        // ADR-0546 as amended); WHEN is the window's, below.
        let may_leave = match &outcome {
            // The private path ACCEPTED. That is not evidence it CARRIES: a
            // transport that accepts and then blackholes — active censorship's
            // cheapest move — accepts for ever, so an accept neither restarts
            // the maintainer's minute (stage 1b; the reset happens at `net/grpc.rs`,
            // where an RPC proves the circuit carried something) nor, once that
            // minute is spent AND the path has been seen to go on failing past
            // it, earns the connection another try. This is the consumer the
            // window lacked: the predicate was read on the FAILED dial only,
            // and the blackhole shape never fails a dial.
            //
            // Two things an accept is NOT. On the class whose accept IS its
            // evidence, the connect has just ended any failing run and there is
            // nothing to leave. And it is never fresh evidence of FAILURE, so it
            // may be left only on failures already observed — see
            // `kept_failing_past_the_window` for the resumed-app leak a bare
            // clock read would open here.
            Ok(_) => {
                self.posture.note_private_connect(class);
                !class.connect_is_the_only_evidence()
                    && self.posture.kept_failing_past_the_window(class)
            }
            // A genuine REACHABILITY failure (incl. a now-bounded hang), or a
            // transport its own host has declared failed (ADR-0553: the refusal
            // that means "not going to start", told apart from a bootstrap by
            // the readiness gate and decided HERE). Each starts, or continues,
            // this class's failure run — the second conjunct of the window.
            Err(DialError::Unreachable | DialError::Timeout | DialError::TransportFailed) => {
                // The DIAL entry, not the plain one: the path would not take
                // the connection at all, which is the one fact that separates
                // "the path is down" from "the path or the server" — see
                // `TorPosture::path_refused`. This is its only production
                // caller.
                self.posture.note_private_dial_failure(class);
                true
            }
            // `NotReady` (a bootstrap, however long it takes), `Retired`,
            // `Unsupported` (the dialer can't honor isolation) and `Io` are
            // surfaced as-is — never silently routed around (that would leak at
            // every slow start, drop isolation or mask a hard fault), and under
            // `Required` they too reach no clearnet.
            //
            // Deliberately NOT counted as a failure run: a bootstrap is not the
            // path failing, `Unsupported` is the host declining on policy and
            // `Io` is a local fault. Counting them would let a host that refuses
            // one isolation key spend the window that decides whether the
            // user's traffic leaves in the clear.
            Err(_) => false,
        };
        match &self.fallback {
            // THE ONE READER of `may_switch_to_direct` (ADR-0552): the private
            // path gets `TOR_PATIENCE_SECS` of carrying nothing AND failing
            // before any clearnet packet, however it fails. `Required` has no
            // fallback and never reaches the read.
            Some(direct) if may_leave && self.posture.may_switch_to_direct(class) => {
                // An accepted private stream is CLOSED, not kept open beside the
                // clearnet one.
                drop(outcome);
                self.switch_to_direct(direct.as_ref(), host, port, isolation_key)
                    .await
            }
            // Still INSIDE the minute (or not an answer that may ever leave):
            // the primary's own outcome, and nothing sent in the clear. The
            // plan's reason says nothing about the private path — the honest
            // "still trying" reading was withdrawn the day it was built; see
            // `resolve_dialer` above for why.
            _ => outcome.map(|stream| Dialed {
                stream,
                served_by: DialArm::Private,
                establish_by: None,
            }),
        }
    }
}

impl PolicyDialer {
    /// Leave the private path for THIS dial: latch, announce, then dial the
    /// clearnet fallback inside [`FALLBACK_ESTABLISH_BUDGET_SECS`]. The only
    /// site that does any of the three.
    async fn switch_to_direct(
        &self,
        direct: &dyn NetDialer,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Dialed, DialError> {
        // The latch is set AT the clearnet dial — never earlier, at a deadline
        // nobody dialled on: `FellBack` says traffic left, not that it may. It
        // is sticky (false → true, NEVER reset within a posture's life) and
        // publishes no companion data — the engine's reader only returns the
        // payload-free `TorState::FellBack`; `Relaxed` is sufficient because
        // there is no happens-before to establish, and a stale `false` observed
        // on one poll is a correct "not yet degraded" answer that converges on
        // the next read (the honest, conservative direction).
        self.posture.latch_fell_back();
        // §5.4-allowlisted instability event — fields-free (no host, port, or
        // isolation key ever logged). The TorState stream + the capture-layer
        // guard wire in with the engine (inc-2c-v).
        //
        // A WARN, so it survives a host's "errors only" setting: it is the ONE
        // line that says traffic left the private path. Named for the PATH, not
        // for Tor (it was `wallet.tor_fell_back` until an earlier revision): the host names its
        // transport (ADR-0547) and it may not be Tor, and since the device log
        // ships, a message that names a circumvention product is the
        // disclosure by another door.
        tracing::warn!(target: "zec_wallet_core", "wallet.private_path_fell_back");
        // ONE deadline for the whole clearnet establishment: the primary may
        // have used its entire bound, and the caller's unary budget is what is
        // left. It travels on with the stream so a TLS handshake spends the
        // remainder of it rather than a fresh bound of its own.
        let establish_by =
            tokio::time::Instant::now() + Duration::from_secs(FALLBACK_ESTABLISH_BUDGET_SECS);
        let dial = direct.dial(host, port, isolation_key);
        match tokio::time::timeout_at(establish_by, dial).await {
            Ok(outcome) => outcome.map(|stream| Dialed {
                stream,
                served_by: DialArm::Clearnet,
                establish_by: Some(establish_by),
            }),
            Err(_elapsed) => {
                // The fallback logs its own line when it returns; cut short it
                // never did, and a clearnet dial that left no line is the one
                // thing the attribution log exists to prevent.
                let cut: Result<(), DialError> = Err(DialError::Timeout);
                log_dial(
                    &self.counts,
                    DialArm::Clearnet,
                    PathClass::of(isolation_key),
                    &cut,
                );
                Err(DialError::Timeout)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ADR-0552's minute: spent by the tests below, never read in production here.
    // `GRPC_UNARY_TIMEOUT_SECS` is the CALLER's budget the stage 1b reachability
    // tests model; the broadcast prefix is the money path's isolation class.
    use crate::constants::{
        BROADCAST_ISOLATION_KEY_PREFIX, GRPC_UNARY_TIMEOUT_SECS, TOR_PATIENCE_SECS,
    };
    use std::sync::Mutex;

    /// Spend BOTH conjuncts of the patience window (ADR-0552 phase 2) by driving
    /// a RUN of reachability failures that lasts the maintainer's minute.
    ///
    /// Every test that wants "the window is spent" must go through here, because
    /// advancing the clock alone no longer spends it: silence is what an IDLE
    /// class accumulates, and switching on silence alone was the HIGH this phase
    /// repairs. A censored wallet fails continuously; that is what this models.
    ///
    /// It asserts the run's own first dial does NOT switch — the property every
    /// caller depends on, asserted once here rather than restated in each.
    async fn spend_the_window(pd: &dyn NetDialer, iso: Option<&str>) {
        let first = pd.dial("zec.example", 443, iso).await;
        assert!(
            first.is_err(),
            "the first failure of a run starts the clock; it must never switch"
        );
        // A full window PLUS a second. The failure stamp is clamped to 1 ms (its
        // `0` means "not failing"), so a run that begins at posture creation is
        // recorded one millisecond late and advancing exactly the window leaves
        // it one millisecond short. The tests that pin the BOUNDARY drive the
        // posture directly, on exact milliseconds; this helper only needs to be
        // unambiguously past it.
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS + 1)).await;
    }

    /// A scripted host dialer: records every dial (host/port/iso) and returns a
    /// programmed result. Stands in for a host's Tor/xray/VLESS transport — the
    /// wallet only ever sees a `NetDialer`.
    struct MockDialer {
        result: Box<dyn Fn() -> Result<Box<dyn AsyncByteStream>, DialError> + Send + Sync>,
        calls: Mutex<Vec<(String, u16, Option<String>)>>,
    }

    impl MockDialer {
        fn failing(err: fn() -> DialError) -> Arc<Self> {
            Arc::new(Self {
                result: Box::new(move || Err(err())),
                calls: Mutex::new(Vec::new()),
            })
        }

        /// Succeeds by returning an in-memory duplex end (a real
        /// `AsyncByteStream`, never a socket).
        fn ok() -> Arc<Self> {
            Arc::new(Self {
                result: Box::new(|| {
                    let (a, _b) = tokio::io::duplex(64);
                    Ok(Box::new(a) as Box<dyn AsyncByteStream>)
                }),
                calls: Mutex::new(Vec::new()),
            })
        }

        fn call_count(&self) -> usize {
            self.calls.lock().expect("mock calls poisoned").len()
        }

        fn last_iso(&self) -> Option<String> {
            self.calls
                .lock()
                .expect("mock calls poisoned")
                .last()
                .and_then(|(_, _, iso)| iso.clone())
        }
    }

    #[async_trait]
    impl NetDialer for MockDialer {
        async fn dial(
            &self,
            host: &str,
            port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            self.calls.lock().expect("mock calls poisoned").push((
                host.to_owned(),
                port,
                isolation_key.map(str::to_owned),
            ));
            (self.result)()
        }
    }

    /// A host dialer that HANGS forever (never returns) — models a blackholed
    /// connect / dead Tor circuit. The SDK's `DIAL_TIMEOUT_SECS` bound must turn
    /// this into a typed `Timeout`, never an indefinite park.
    struct HangingDialer;

    #[async_trait]
    impl NetDialer for HangingDialer {
        async fn dial(
            &self,
            _host: &str,
            _port: u16,
            _isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            std::future::pending().await
        }
    }

    #[tokio::test(start_paused = true)]
    async fn hung_host_dial_times_out_and_preferred_falls_back() {
        // The defect a raw-awaited host dial would cause: a HUNG host dialer never
        // returns Unreachable/Timeout, so the visible fallback could never fire.
        // With the SDK bound, the hang becomes a Timeout and Preferred falls back.
        // (start_paused: tokio auto-advances the virtual clock — no real 30s wait.)
        let direct = MockDialer::ok();
        let pd = PolicyDialer::with_fallback(
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::new(TorPosture::new()),
            Default::default(),
        );
        // ADR-0552: switching is only ALLOWED once the wallet has insisted on the
        // private path for the maintainer's minute — a minute of FAILING, not merely
        // of elapsed time (phase 2). The dial bound alone is a fraction of a
        // window; the other half is pinned next door by the test that must NOT
        // fall back.
        spend_the_window(&pd, Some("sync")).await;
        let r = pd.dial("zec.example", 443, Some("sync")).await;
        assert!(r.is_ok(), "fell back after the host dial timed out");
        assert!(pd.fell_back(), "the bounded hang is a VISIBLE fallback");
        assert_eq!(direct.call_count(), 1, "the fallback clearnet dial ran");
    }

    #[tokio::test(start_paused = true)]
    async fn hung_host_dial_under_required_is_fail_closed() {
        // Under Required a hung host dial is bounded to Timeout and fail-closes —
        // no fallback exists, so zero clearnet even though the dialer HUNG (rather
        // than erroring). The channel worker can never wedge on this dial.
        let pd = PolicyDialer::fail_closed(
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::new(TorPosture::new()),
            Default::default(),
        );
        assert!(matches!(
            pd.dial("zec.example", 443, None).await,
            Err(DialError::Timeout)
        ));
    }

    #[tokio::test]
    async fn tor_required_dialer_failure_is_fail_closed_zero_bytes() {
        // §8 `tor_required_dialer_failure_is_fail_closed_zero_bytes`: Required +
        // a failing host dialer ⇒ the error propagates, NO clearnet fallback dial
        // is ever made (the host dialer is the only one that saw a dial), and the
        // plan's stall reason is TorUnavailable.
        let host = MockDialer::failing(|| DialError::Unreachable);
        let plan = resolve_dialer(
            &TorPolicy::Required {
                runtime: TorRuntime::Dialer(Arc::clone(&host) as Arc<dyn NetDialer>),
            },
            &Arc::new(TorPosture::new()),
            &Default::default(),
        )
        .expect("required+dialer resolves");
        assert_eq!(plan.stall_on_failure, StallReason::TorUnavailable);

        let r = plan.dialer.dial("zec.example", 443, Some("sync")).await;
        assert!(matches!(r, Err(DialError::Unreachable)), "fail-closed");
        assert_eq!(
            host.call_count(),
            1,
            "the host dialer was tried exactly once"
        );
        // there is no second (clearnet) dialer to count — zero clearnet by
        // construction (PolicyDialer::fail_closed has no fallback).
    }

    #[tokio::test(start_paused = true)]
    async fn preferred_dialer_failure_falls_back_visibly() {
        // §8 `preferred_dialer_failure_falls_back_visibly`: Preferred + a failing
        // host dialer ⇒ fall back to the direct dialer, and `fell_back` flips
        // (the visible degradation TorState::FellBack reads). Since ADR-0552 the
        // fall-back is also TIMED: the minute is spent below, on a paused clock, so
        // spending it costs no wall time.
        let host = MockDialer::failing(|| DialError::Timeout);
        let direct = MockDialer::ok();
        let pd = PolicyDialer::with_fallback(
            Arc::clone(&host) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::new(TorPosture::new()),
            Default::default(),
        );
        spend_the_window(&pd, Some("sync")).await;
        let r = pd.dial("zec.example", 443, Some("sync")).await;
        assert!(r.is_ok(), "fell back to the working clearnet dialer");
        assert!(pd.fell_back(), "fallback is VISIBLE, never silent");
        assert_eq!(
            host.call_count(),
            2,
            "the run's dial, then the one that switched"
        );
        assert_eq!(direct.call_count(), 1, "the fallback was actually used");
    }

    #[tokio::test]
    async fn preferred_plan_writes_the_caller_owned_fellback_flag() {
        // The INC-2D GATE half: `resolve_dialer` writes the CALLER's (Inner-owned) flag,
        // not a private per-circuit one — so `tor_state()` reads ANY circuit's degradation
        // off the one wallet-level atomic. Pass a flag IN, drive a real Preferred fallback,
        // assert the SAME flag flipped (it is the atomic the `dial` path writes).
        // A posture whose ADR-0552 minute is already spent: this test runs on the
        // REAL clock (it needs a real loopback accept, which a paused clock breaks),
        // so the window is back-dated rather than waited out.
        let flag = Arc::new(TorPosture::with_the_minute_already_spent());
        let host = MockDialer::failing(|| DialError::Unreachable);
        let plan = resolve_dialer(
            &TorPolicy::Preferred {
                runtime: TorRuntime::Dialer(Arc::clone(&host) as Arc<dyn NetDialer>),
            },
            &flag,
            &Default::default(),
        )
        .expect("preferred+dialer resolves");
        assert!(!flag.fell_back(), "no fall-back before any dial");

        // The Preferred fallback is the real DirectTcpDialer — give it a live loopback
        // listener to connect to so the fallback SUCCEEDS (in-process, no real network).
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        let accept = tokio::spawn(async move { listener.accept().await.map(|_| ()) });
        let r = plan
            .dialer
            .dial(&addr.ip().to_string(), addr.port(), Some("send-abc"))
            .await;
        assert!(r.is_ok(), "primary failed ⇒ fell back to clearnet");
        assert!(
            flag.fell_back(),
            "the caller's wallet-level flag observed the fall-back the dialer wrote"
        );
        let _ = accept.await;
    }

    /// `on-device-log-layer-phase-1.md` §3a — the per-dial ATTRIBUTION line. Every
    /// physical dial says which ARM performed it, for which circuit family, and
    /// how it ended; a `Preferred` switch reads as two lines, the private failure
    /// and then the clearnet connect, in that order. The field set is EXACTLY
    /// those three: no host, no port, no isolation key.
    ///
    /// And its LEVEL, which is part of the host contract (FR-35): a host's "errors
    /// only" setting carries WARN and above, so a private path that is
    /// UNREACHABLE is a WARN, while `connected` and `not_ready` — a bootstrap in
    /// progress, a normal state — are INFO. The fall-back event between the two
    /// switch lines is a WARN too, and is named for the PATH, never for Tor.
    ///
    /// Watched against: the `dial_arm` field dropped from `log_dial` (the field
    /// set assertion reds); `DialArm::Clearnet` logged as `"host"` (the second
    /// line reds — the mutation that would turn a clearnet leak into a line
    /// claiming the host's transport carried it);the `log_dial` call removed from
    /// `PolicyDialer::dial` (the private lines vanish); `Unreachable` filed as
    /// not-`wrong` (the level assertion reds — "errors only" would go blind to a
    /// dead private path).
    #[tokio::test]
    async fn a_dial_names_its_arm_its_class_and_its_outcome() {
        use crate::constants::WALLET_SYNC_ISOLATION_KEY;
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing::Level;
        use tracing_subscriber::layer::SubscriberExt;

        /// The capture harness keeps fields, not levels; this keeps the level of
        /// every event the dial path emits, beside its message.
        #[derive(Clone, Default)]
        struct Levels(Arc<Mutex<Vec<(Level, String)>>>);
        struct Message(String);
        impl tracing::field::Visit for Message {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}");
                }
            }
        }
        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Levels {
            fn on_event(
                &self,
                event: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                let mut message = Message(String::new());
                event.record(&mut message);
                // exactly the attribution line (not the debug `wallet.dial.connected`)
                // and the fall-back event
                if message.0 == "wallet.dial" || message.0.contains("fell_back") {
                    self.0
                        .lock()
                        .expect("levels poisoned")
                        .push((*event.metadata().level(), message.0));
                }
            }
        }

        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let levels = Levels::default();
        let subscriber = tracing_subscriber::registry()
            .with(CaptureLayer::new(sink.clone()))
            .with(levels.clone());
        let _guard = tracing::subscriber::set_default(subscriber);

        // 1 + 2: a `Preferred` wallet whose minute is spent, an unreachable private
        // path, the REAL clearnet dialer as its fallback (a loopback listener — the
        // real clock, as the fall-back test above explains), on the MONEY class.
        let plan = resolve_dialer(
            &TorPolicy::Preferred {
                runtime: TorRuntime::Dialer(
                    MockDialer::failing(|| DialError::Unreachable) as Arc<dyn NetDialer>
                ),
            },
            &Arc::new(TorPosture::with_the_minute_already_spent()),
            &Default::default(),
        )
        .expect("preferred+dialer resolves");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        let accept = tokio::spawn(async move { listener.accept().await.map(|_| ()) });
        let send_key = format!("{BROADCAST_ISOLATION_KEY_PREFIX}-abc");
        plan.dialer
            .dial(&addr.ip().to_string(), addr.port(), Some(&send_key))
            .await
            .expect("the fallback connects");
        let _ = accept.await;

        // 3: a private success on the sync class. 4: `Required` refusing a dial
        // that carries no key at all — a non-reachability failure, named as such.
        PolicyDialer::fail_closed(
            MockDialer::ok(),
            Arc::new(TorPosture::new()),
            Default::default(),
        )
        .dial("zec.example", 443, Some(WALLET_SYNC_ISOLATION_KEY))
        .await
        .expect("the private path connects");
        let refused = PolicyDialer::fail_closed(
            MockDialer::failing(|| DialError::NotReady),
            Arc::new(TorPosture::new()),
            Default::default(),
        )
        .dial("zec.example", 443, None)
        .await;
        assert!(matches!(refused, Err(DialError::NotReady)));

        let lines: Vec<Vec<(String, String)>> = sink
            .records_of("wallet.dial")
            .into_iter()
            .map(|r| r.into_iter().filter(|(n, _)| n != "message").collect())
            .collect();
        let line = |arm: &str, class: &str, outcome: &str| {
            [
                ("dial_arm", arm),
                ("dial_class", class),
                ("outcome", outcome),
            ]
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .to_vec()
        };
        assert_eq!(
            lines,
            [
                line("host", "broadcast", "unreachable"),
                line("sdk_direct", "broadcast", "connected"),
                line("host", "sync", "connected"),
                line("host", "other", "not_ready"),
            ],
            "one line per physical dial, from the dialer that performed it, carrying \
             exactly the arm, the class and the outcome"
        );
        assert_5_4_clean(&sink.fields());

        let levels = levels.0.lock().expect("levels poisoned").clone();
        let levels: Vec<(Level, &str)> = levels.iter().map(|(l, m)| (*l, m.as_str())).collect();
        assert_eq!(
            levels,
            [
                (Level::WARN, "wallet.dial"), // private … unreachable
                (Level::WARN, "wallet.private_path_fell_back"),
                (Level::INFO, "wallet.dial"), // clearnet … connected
                (Level::INFO, "wallet.dial"), // private … connected
                (Level::INFO, "wallet.dial"), // private … not_ready: a normal state
            ],
            "what survives a host's \"errors only\" setting is the dead private path and the \
             switch off it — never the routine lines, and never a transport's name"
        );
    }

    /// The `dial_class` VALUE SET, pinned — the complement of the field-NAME
    /// allowlist, and the guard that was missing when `swap` shipped.
    ///
    /// `tracing_guard::ALLOWLIST` matches `wallet.dial`'s fields by NAME, so a
    /// new `PathClass` reaches a host-readable device log as a new VALUE with
    /// nothing red and nothing priced — which is how `dial_class=swap` got
    /// there `window` while the allowlist's own record still said the
    /// field was "one of three". The value is not neutral: `swap` says this
    /// handset used the cross-chain on-ramp, when and how often, which joins
    /// to the provider's logs and to an on-chain deposit. The four values are
    /// priced in that record; this row makes the SET a gate.
    ///
    /// The match below is EXHAUSTIVE on purpose (like `log_dial`'s own and
    /// like `PathClass::slot`): a fifth class is a COMPILE error here, so
    /// whoever adds one has to write its log literal down in the same change
    /// — and the allowlist record is where they must then price it.
    ///
    /// Watched against: `PathClass::Swap => "swap"` folded to `=> "other"` in
    /// `log_dial` (the per-class assertion reds — and the fold is the
    /// alternative remedy this row exists to make visible, not invisible).
    #[tokio::test]
    async fn the_dial_class_vocabulary_is_exactly_the_priced_four() {
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::layer::SubscriberExt;

        /// The literal each class prints, as the priced record spells it.
        fn priced(class: PathClass) -> &'static str {
            match class {
                PathClass::Sync => "sync",
                PathClass::Broadcast => "broadcast",
                PathClass::Swap => "swap",
                PathClass::Other => "other",
            }
        }
        const EVERY_CLASS: [PathClass; 4] = [
            PathClass::Sync,
            PathClass::Broadcast,
            PathClass::Swap,
            PathClass::Other,
        ];

        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(CaptureLayer::new(sink.clone())),
        );

        let counts = DialCounters::default();
        for class in EVERY_CLASS {
            log_dial(&counts, DialArm::Private, class, &Ok::<(), DialError>(()));
        }
        let classes: Vec<String> = sink
            .records_of("wallet.dial")
            .into_iter()
            .map(|fields| {
                fields
                    .iter()
                    .find(|(n, _)| n == "dial_class")
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(
            classes,
            EVERY_CLASS.map(priced).to_vec(),
            "every class logs the literal the allowlist record prices for it"
        );
        let mut vocabulary = classes.clone();
        vocabulary.sort();
        vocabulary.dedup();
        assert_eq!(
            vocabulary,
            ["broadcast", "other", "swap", "sync"],
            "EXACTLY these four values may reach a device log under `dial_class`. \
             A fifth is a new disclosure — price it in `tracing_guard::ALLOWLIST`'s \
             `wallet.dial` record (what it tells a stranger holding the log) before \
             it ships, the way `swap` was not"
        );
        assert_5_4_clean(&sink.fields());
    }

    #[tokio::test]
    async fn off_and_required_never_write_the_fellback_flag() {
        // The complement: neither clearnet-by-choice (Off) nor fail-closed (Required)
        // may EVER write the wallet latch — only a genuine Preferred degradation can.
        // Off dials its primary (direct) and succeeds without "falling back"; Required
        // fail-closes on a failing host with no fallback. Both leave the flag false.
        let off_flag = Arc::new(TorPosture::new());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        let accept = tokio::spawn(async move { listener.accept().await.map(|_| ()) });
        let off =
            resolve_dialer(&TorPolicy::Off, &off_flag, &Default::default()).expect("off resolves");
        off.dialer
            .dial(&addr.ip().to_string(), addr.port(), None)
            .await
            .expect("off direct dial connects");
        assert!(
            !off_flag.fell_back(),
            "Off is the primary path — it never falls back"
        );
        let _ = accept.await;

        let req_flag = Arc::new(TorPosture::new());
        let host = MockDialer::failing(|| DialError::Unreachable);
        let req = resolve_dialer(
            &TorPolicy::Required {
                runtime: TorRuntime::Dialer(Arc::clone(&host) as Arc<dyn NetDialer>),
            },
            &req_flag,
            &Default::default(),
        )
        .expect("required resolves");
        assert!(matches!(
            req.dialer.dial("zec.example", 443, Some("sync")).await,
            Err(DialError::Unreachable)
        ));
        assert!(
            !req_flag.fell_back(),
            "Required is fail-closed — the latch can never flip for it"
        );
    }

    #[tokio::test]
    async fn preferred_propagates_config_and_io_errors_without_falling_back() {
        // `Unsupported` (isolation can't be honored) and `Io` are NOT reachability
        // failures — falling back would silently drop isolation / mask a hard
        // fault. Even under Preferred, both propagate and never trigger clearnet.
        let cases: [fn() -> DialError; 2] = [
            || DialError::Unsupported,
            || DialError::Io(std::io::Error::other("reset")),
        ];
        for make_err in cases {
            let host = MockDialer::failing(make_err);
            let direct = MockDialer::ok();
            let pd = PolicyDialer::with_fallback(
                Arc::clone(&host) as Arc<dyn NetDialer>,
                Arc::clone(&direct) as Arc<dyn NetDialer>,
                Arc::new(TorPosture::new()),
                Default::default(),
            );
            let r = pd.dial("h", 1, Some("k")).await;
            assert!(
                matches!(r, Err(DialError::Unsupported) | Err(DialError::Io(_))),
                "config/IO errors propagate unchanged"
            );
            assert!(!pd.fell_back(), "no fallback on a non-reachability error");
            assert_eq!(direct.call_count(), 0, "clearnet was never dialed");
        }
    }

    #[tokio::test]
    async fn sync_isolation_key_reaches_the_dialer() {
        // §8 `sync_isolation_key_reaches_the_dialer`: the connector's isolation
        // key flows through PolicyDialer to the host dialer unchanged (§2.3).
        let host = MockDialer::ok();
        let pd = PolicyDialer::fail_closed(
            Arc::clone(&host) as Arc<dyn NetDialer>,
            Arc::new(TorPosture::new()),
            Default::default(),
        );
        pd.dial("zec.example", 443, Some("per-sync-circuit"))
            .await
            .expect("ok");
        assert_eq!(host.last_iso().as_deref(), Some("per-sync-circuit"));
    }

    #[test]
    fn unsupported_runtime_is_typed_config_error_not_clearnet_leak() {
        // §8 `unsupported_runtime_is_typed_config_error_not_clearnet_leak`:
        // ExternalSocks5 (no dialer yet) ⇒ typed NetError::UnsupportedRuntime at
        // resolution, for BOTH Preferred and Required — never a direct-dialer
        // fall-through.
        for policy in [
            TorPolicy::Required {
                runtime: TorRuntime::ExternalSocks5 {
                    addr: "127.0.0.1:9050".into(),
                },
            },
            TorPolicy::Preferred {
                runtime: TorRuntime::ExternalSocks5 {
                    addr: "127.0.0.1:9050".into(),
                },
            },
        ] {
            assert!(matches!(
                resolve_dialer(&policy, &Arc::new(TorPosture::new()), &Default::default()),
                Err(NetError::UnsupportedRuntime {
                    runtime: TorRuntimeKind::ExternalSocks5
                })
            ));
        }
    }

    #[tokio::test]
    async fn off_resolves_to_direct_with_endpoint_unreachable_stall() {
        // Off ⇒ the built-in direct dialer, ordinary-offline stall reason. The
        // direct dialer actually connects to a loopback listener (in-process, no
        // external network) — proves it is a working TCP dialer, not a stub.
        let plan = resolve_dialer(
            &TorPolicy::Off,
            &Arc::new(TorPosture::new()),
            &Default::default(),
        )
        .expect("off resolves");
        assert_eq!(plan.stall_on_failure, StallReason::EndpointUnreachable);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        let accept = tokio::spawn(async move { listener.accept().await.map(|_| ()) });
        let stream = plan
            .dialer
            .dial(&addr.ip().to_string(), addr.port(), None)
            .await;
        assert!(stream.is_ok(), "direct dialer connects to a live listener");
        let _ = accept.await;
    }

    #[tokio::test]
    async fn direct_dialer_maps_refused_connect_to_io() {
        // A closed port ⇒ typed DialError (not a panic). Bind-then-drop yields a
        // port nothing listens on (best-effort; loopback connect-refused is
        // immediate, no real network). PORT-REUSE: another process re-binding the
        // freed port in the gap is vanishingly unlikely on loopback; the assert
        // accepts Io OR Timeout, and the property under test is "typed, never a
        // panic" — a stray success would surface as the explicit panic below.
        let port = {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            l.local_addr().expect("addr").port()
            // listener dropped here → port free, connect refused
        };
        let r = DirectTcpDialer::new(Default::default())
            .dial("127.0.0.1", port, None)
            .await;
        assert!(
            matches!(r, Err(DialError::Io(_)) | Err(DialError::Timeout)),
            "refused/again connect is a typed dial error, never a panic"
        );
    }

    // ── FR-21 happy-eyeballs (the staggered race + IPv4-first interleave) ────
    //
    // The race core is tested with SCRIPTED connect outcomes on the paused
    // clock (documentation-range addresses, never dialed) — deterministic, no
    // real network. The real-socket path stays covered by the loopback tests
    // above (they now flow through lookup_host → interleave → race).

    fn v4(octet: u8) -> SocketAddr {
        SocketAddr::from(([192, 0, 2, octet], 443))
    }

    fn v6(seg: u16) -> SocketAddr {
        SocketAddr::from(([0x2001, 0xdb8, 0, 0, 0, 0, 0, seg], 443))
    }

    #[test]
    fn interleave_puts_v4_first_and_alternates_families() {
        // Resolver handed v6-heavy, v6-first ordering;
        // the interleave gives IPv4 the head-start slot and alternates,
        // preserving resolver order WITHIN each family.
        let ordered = interleave_v4_first(vec![v6(1), v6(2), v4(1), v6(3), v4(2)]);
        assert_eq!(ordered, vec![v4(1), v6(1), v4(2), v6(2), v6(3)]);
    }

    #[test]
    fn interleave_caps_each_family_preserving_order() {
        let many_v4 = (1u8..=6).map(v4);
        let ordered = interleave_v4_first(many_v4.chain([v6(1)]));
        assert_eq!(
            ordered,
            vec![v4(1), v6(1), v4(2), v4(3), v4(4)],
            "the per-family cap keeps the FIRST {DIAL_MAX_ADDRS_PER_FAMILY} of a family"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_blackholed_v6_first_leg_loses_to_v4_within_one_stagger() {
        // THE FR-21 regression, at the race core: force IPv6-FIRST ordering (as
        // if the interleave didn't exist) against a blackholed v6 — the raced
        // v4 attempt must win ~one stagger in, never after the 30 s budget
        // (the sequential dial burned the whole budget here).
        let start = tokio::time::Instant::now();
        let r = connect_staggered(vec![v6(1), v4(1)], |addr| async move {
            if addr.is_ipv4() {
                Ok("v4")
            } else {
                std::future::pending().await // blackhole: no RST, no timeout
            }
        })
        .await;
        assert_eq!(r.expect("the v4 leg wins the race"), "v4");
        let waited = start.elapsed();
        assert!(
            waited >= Duration::from_millis(DIAL_ATTEMPT_STAGGER_MS)
                && waited < Duration::from_secs(1),
            "won one stagger in (virtual {waited:?}), not after the dial budget"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_first_leg_success_pays_no_stagger() {
        // The healthy-path cost of the race is ZERO: a working first attempt
        // wins immediately — no stagger, no extra sockets consulted.
        let start = tokio::time::Instant::now();
        let r = connect_staggered(vec![v4(1), v6(1)], |_| async move { Ok("first") }).await;
        assert_eq!(r.expect("first leg wins"), "first");
        assert!(
            start.elapsed() < Duration::from_millis(DIAL_ATTEMPT_STAGGER_MS),
            "no stagger paid on the happy path"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_v6_leg_wins_early_when_v4_fails_fast() {
        // The fallback works in BOTH directions (a v6-only NAT64 network whose
        // v4 head-start dies ENETUNREACH in ~0 ms), AND the definitive failure
        // wakes the next leg EARLY (RFC 8305 §5) — v6-only networks must not
        // pay a fixed stagger tax on every dial.
        let start = tokio::time::Instant::now();
        let r = connect_staggered(vec![v4(1), v6(1)], |addr| async move {
            if addr.is_ipv4() {
                Err(DialError::Io(std::io::Error::other("network unreachable")))
            } else {
                Ok("v6")
            }
        })
        .await;
        assert_eq!(r.expect("the v6 leg wins"), "v6");
        assert!(
            start.elapsed() < Duration::from_millis(DIAL_ATTEMPT_STAGGER_MS),
            "the v4 failure started the v6 leg immediately — no stagger tax"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_all_fail_surfaces_the_diagnostic_refused_error() {
        // The refused verdict (proof the network path works end-to-end) is on
        // the LATER leg here, and the earlier leg's unreachable-noise both
        // STARTS and COMPLETES first — so neither first-completed nor
        // last-completed selection can pass this; only the diagnostic rank
        // does (the original refused-on-first-leg script let a
        // first-completed mutant survive).
        let r: Result<&str, _> = connect_staggered(vec![v4(1), v6(1)], |addr| async move {
            if addr.ip().to_canonical().is_ipv4() {
                Err(DialError::Io(std::io::Error::other("network unreachable")))
            } else {
                Err(DialError::Io(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "refused",
                )))
            }
        })
        .await;
        match r {
            Err(DialError::Io(e)) => assert_eq!(
                e.kind(),
                std::io::ErrorKind::ConnectionRefused,
                "the diagnostic error wins regardless of completion order"
            ),
            other => panic!("expected the refused Io error, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn race_same_rank_tie_breaks_to_address_order_not_completion_order() {
        // leg0 (the preferred-family head start) fails SLOWLY; leg1 fails
        // instantly at its stagger — completion order is [leg1, leg0], both
        // rank-equal Io. The surfaced error must be leg0's (address order),
        // pinning the `*i` tie-break: dropping it (or taking the first
        // completion) surfaces leg1's error and fails here.
        let r: Result<&str, _> = connect_staggered(vec![v4(1), v6(1)], |addr| async move {
            if addr.ip().to_canonical().is_ipv4() {
                tokio::time::sleep(Duration::from_millis(400)).await;
                Err(DialError::Io(std::io::Error::other("leg0-unreachable")))
            } else {
                Err(DialError::Io(std::io::Error::other("leg1-unreachable")))
            }
        })
        .await;
        match r {
            Err(DialError::Io(e)) => {
                let inner = e.get_ref().map(ToString::to_string).unwrap_or_default();
                assert_eq!(
                    inner, "leg0-unreachable",
                    "address order breaks the rank tie, never completion order"
                );
            }
            other => panic!("expected an Io error, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn race_panicked_leg_counts_into_the_early_wake_gate() {
        // A panicking leg must act like a failed one for the gate (
        // the task-side accounting is skipped by the unwind, so the DRAIN
        // counts it): the next attempt wakes immediately instead of sleeping
        // out its stagger, and the healthy leg still wins.
        let start = tokio::time::Instant::now();
        let r = connect_staggered(vec![v4(1), v6(1)], |addr| async move {
            if addr.ip().to_canonical().is_ipv4() {
                panic!("scripted leg crash");
            }
            Ok("v6")
        })
        .await;
        assert_eq!(r.expect("the healthy leg still wins"), "v6");
        assert!(
            start.elapsed() < Duration::from_millis(DIAL_ATTEMPT_STAGGER_MS),
            "the panic counted as a failure and woke the next leg early"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_is_internally_bounded_when_every_leg_blackholes() {
        // The security review's surviving mutant: the race must carry its OWN
        // DIAL_TIMEOUT_SECS bound, never rely on the caller's belt. Every leg
        // blackholed ⇒ typed Timeout at the bound on the virtual clock (and
        // the timeout's JoinSet drop aborts the parked attempts). The HARNESS
        // timeout exists so the mutant dies with a named assert (
        // without it, deleting the internal bound leaves the paused clock
        // with no timers and the test wedges CI instead of failing).
        let start = tokio::time::Instant::now();
        let r = tokio::time::timeout(
            Duration::from_secs(DIAL_TIMEOUT_SECS * 2),
            connect_staggered(vec![v4(1), v6(1)], |_| {
                std::future::pending::<Result<&'static str, DialError>>()
            }),
        )
        .await
        .expect("the race's OWN bound must fire — reaching the harness timeout means it is gone");
        assert!(matches!(r, Err(DialError::Timeout)), "typed, never a park");
        let waited = start.elapsed();
        assert!(
            waited >= Duration::from_secs(DIAL_TIMEOUT_SECS)
                && waited < Duration::from_secs(DIAL_TIMEOUT_SECS + 1),
            "the race's own bound fired (virtual {waited:?})"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn dial_path_applies_the_interleave_before_the_race() {
        // The WIRING pin (reliability review): `direct_dial_ordered` — the
        // production dial path after resolution — must reorder a v6-first
        // resolver answer so the v4 leg dials at t=0. Deleting the interleave
        // from the path (racing raw resolver order) leaves v6 first: the
        // recorded first dial flips to v6 and this fails.
        let dialed = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&dialed);
        let r = direct_dial_ordered(vec![v6(1), v6(2), v4(1)], move |addr| {
            let seen = Arc::clone(&seen);
            async move {
                seen.lock().expect("order log").push(addr.is_ipv4());
                if addr.is_ipv4() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok("via-v4")
                } else {
                    std::future::pending().await
                }
            }
        })
        .await;
        assert_eq!(r.expect("connects"), "via-v4");
        assert_eq!(
            *dialed.lock().expect("order log"),
            vec![true],
            "the interleaved v4 head-start dialed FIRST and won before any v6 leg started"
        );
    }

    #[test]
    fn interleave_classifies_v4_mapped_addresses_as_v4() {
        // AI_V4MAPPED resolvers hand back `::ffff:a.b.c.d` — an IPv4 wire dial
        // that must take an IPv4 slot (head-start + v4 cap), not dilute the v6
        // lane (security review LOW).
        let mapped =
            SocketAddr::from((std::net::Ipv4Addr::new(192, 0, 2, 9).to_ipv6_mapped(), 443));
        let rebuilt = SocketAddr::from(([192, 0, 2, 9], 443));
        let ordered = interleave_v4_first(vec![v6(1), mapped]);
        assert_eq!(
            ordered,
            vec![rebuilt, v6(1)],
            "the v4-mapped address is REBUILT as genuine v4 (AF_INET wire dial) in the head-start slot"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_all_legs_failing_returns_typed_error_after_trying_every_addr() {
        let tried = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = Arc::clone(&tried);
        let r = connect_staggered(vec![v4(1), v6(1), v4(2)], move |_| {
            let seen = Arc::clone(&seen);
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                Result::<&str, _>::Err(DialError::Io(std::io::Error::other("refused")))
            }
        })
        .await;
        assert!(matches!(r, Err(DialError::Io(_))), "typed, never a panic");
        assert_eq!(
            tried.load(Ordering::SeqCst),
            3,
            "every resolved address was actually attempted"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_winner_aborts_the_laggard_attempts() {
        // No leaked half-open connects: once a leg wins, the still-pending
        // laggards are aborted with the JoinSet — a laggard that WOULD have
        // completed later never runs to completion.
        let leaked = Arc::new(TorPosture::new());
        let flag = Arc::clone(&leaked);
        let r = connect_staggered(vec![v4(1), v6(1)], move |addr| {
            let flag = Arc::clone(&flag);
            async move {
                if addr.is_ipv4() {
                    Ok("winner")
                } else {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    flag.latch_fell_back();
                    Ok("laggard")
                }
            }
        })
        .await;
        assert_eq!(r.expect("race succeeds"), "winner");
        // Give a leaked task every chance to run on the paused clock.
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(
            !leaked.fell_back(),
            "the laggard was aborted at the win, not left running"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn race_with_no_resolved_addresses_is_a_typed_io_error() {
        // An empty resolution (bogus record set) is a typed error — the race
        // must not hang awaiting attempts that were never spawned — and the
        // verdict must NAME the DNS emptiness (the no-addresses vs
        // all-aborted distinction was unpinned at message level).
        let r = connect_staggered(Vec::new(), |_| async move { Ok("unreachable") }).await;
        match r {
            Err(DialError::Io(e)) => assert!(
                e.get_ref()
                    .is_some_and(|inner| inner.to_string().contains("no addresses")),
                "the empty-resolution verdict names DNS, not an abort"
            ),
            other => panic!("expected the typed Io error, got {other:?}"),
        }
    }

    // ── FR-29: the registered host dialer through the policy (spec §8) ───────
    //
    // The RUST fake (`host_dialer::testing::ScriptedHostDialer`) stands in for
    // the bridge's `CAbiHostDialer`: the same trait, a scripted descriptor and
    // dial outcome. What these rows prove is the CORE's wiring — `runtime_dialer`
    // hands the host dialer to `PolicyDialer` unchanged, so the frozen code
    // table (§3.3) lands in the two arms `PolicyDialer` already has.

    use crate::net::host_dialer::testing::{ScriptedHostDialer, name};
    use crate::net::host_dialer::{
        HostDialCode, HostDialer, HostTransportDescriptor, IsolationSupport, TransportExposure,
        TransportHealth,
    };
    use crate::state::{SyncStatus, TorState};
    use crate::tor_status::live_tor_state;

    fn host_policy(required: bool, host: &Arc<ScriptedHostDialer>) -> TorPolicy {
        let runtime = TorRuntime::HostDialer(Arc::clone(host) as Arc<dyn HostDialer>);
        if required {
            TorPolicy::Required { runtime }
        } else {
            TorPolicy::Preferred { runtime }
        }
    }

    fn tor_unavailable() -> SyncStatus {
        SyncStatus::Stalled {
            reason: StallReason::TorUnavailable,
        }
    }

    /// Spec §8 T1 / ADR-0546 / the §6.2 matrix, `Required` column: `NOT_READY`
    /// then `RETIRED` from the host — the dial fails TYPED with the code's own
    /// variant, no clearnet is dialed (a `Required` plan has no fallback by
    /// construction, and the wallet latch never moves), the plan's stall is
    /// `TorUnavailable`, and the state renders `Bootstrapping { readiness }`
    /// while the host is not ready and `Unavailable` once the stall is up.
    #[tokio::test]
    async fn required_fails_closed_on_not_ready_and_retired() {
        let host = Arc::new(ScriptedHostDialer::with(
            Some(HostTransportDescriptor {
                name: name("Tor"),
                readiness: 42,
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
                health: TransportHealth::Starting,
            }),
            Err(HostDialCode::NotReady),
        ));
        let flag = Arc::new(TorPosture::new());
        let policy = host_policy(true, &host);
        let plan = resolve_dialer(&policy, &flag, &Default::default())
            .expect("Required + HostDialer resolves");
        assert_eq!(plan.stall_on_failure, StallReason::TorUnavailable);

        assert!(
            matches!(
                plan.dialer
                    .dial("zec.example", 443, Some("wallet-sync"))
                    .await,
                Err(DialError::NotReady)
            ),
            "not-ready is surfaced as itself — a bootstrap in progress, not unreachable"
        );
        assert_eq!(
            live_tor_state(
                &policy,
                flag.fell_back(),
                &tor_unavailable(),
                policy.host_descriptor(),
                flag.is_unanswered(),
                flag.path_refused(),
            ),
            TorState::Bootstrapping {
                percent: Some(0.42),
                transport: Some(name("Tor"))
            },
            "the state says the host is bootstrapping, at the host's readiness"
        );

        // The host tears its backing down (a carrier switch): RETIRED, and the
        // descriptor push says READY again — the dial still fails typed.
        host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
        host.set_outcome(Err(HostDialCode::Retired));
        assert!(matches!(
            plan.dialer
                .dial("zec.example", 443, Some("wallet-sync"))
                .await,
            Err(DialError::Retired)
        ));
        assert_eq!(
            live_tor_state(
                &policy,
                flag.fell_back(),
                &tor_unavailable(),
                policy.host_descriptor(),
                flag.is_unanswered(),
                flag.path_refused(),
            ),
            TorState::Unavailable {
                transport: Some(name("Tor"))
            },
            "fail-closed: the policy stall reads Unavailable"
        );
        assert!(
            !flag.fell_back(),
            "Required never writes the fell-back latch"
        );
        assert_eq!(
            host.keys_seen().len(),
            1,
            "only the READY dial reached the host — the not-ready one was refused by the gate before the host (T26); zero clearnet by construction"
        );
    }

    /// Spec §8 T2 / ADR-0546: under `Preferred`, `UNREACHABLE` and `TIMEOUT` flip
    /// the latch and reach the clearnet fallback; `NOT_READY`, `REFUSED` and
    /// `RETIRED` do NOT — the error propagates, the fallback is never dialed,
    /// the latch stays false (the audit's HIGH: a bootstrapping host must not
    /// leak clearnet at every app start).
    #[tokio::test(start_paused = true)]
    async fn preferred_fell_back_on_unreachable_or_timeout_only() {
        for code in [HostDialCode::Unreachable, HostDialCode::Timeout] {
            let host = Arc::new(ScriptedHostDialer::with(
                Some(ScriptedHostDialer::tor_ready()),
                Err(code),
            ));
            let direct = MockDialer::ok();
            let flag = Arc::new(TorPosture::new());
            let pd = PolicyDialer::with_fallback(
                Arc::clone(&host) as Arc<dyn NetDialer>,
                Arc::clone(&direct) as Arc<dyn NetDialer>,
                Arc::clone(&flag),
                Default::default(),
            );
            // Each iteration gets a FRESH posture, so each spends its own window
            // (ADR-0552) before the error set is exercised. This test pins WHICH
            // errors may switch; WHEN is pinned by the patience tests below.
            spend_the_window(&pd, Some("wallet-sync")).await;
            let r = pd.dial("zec.example", 443, Some("wallet-sync")).await;
            assert!(
                r.is_ok(),
                "{code:?}: fell back to the working clearnet dialer"
            );
            assert!(flag.fell_back(), "{code:?}: the fallback is VISIBLE");
            assert_eq!(direct.call_count(), 1, "{code:?}: clearnet dialed once");
        }
        for (code, expected) in [
            (HostDialCode::NotReady, "NotReady"),
            (HostDialCode::Refused, "Unsupported"),
            (HostDialCode::Retired, "Retired"),
        ] {
            let host = Arc::new(ScriptedHostDialer::with(
                Some(ScriptedHostDialer::tor_ready()),
                Err(code),
            ));
            let direct = MockDialer::ok();
            let flag = Arc::new(TorPosture::new());
            let pd = PolicyDialer::with_fallback(
                Arc::clone(&host) as Arc<dyn NetDialer>,
                Arc::clone(&direct) as Arc<dyn NetDialer>,
                Arc::clone(&flag),
                Default::default(),
            );
            // Spend the window HERE TOO, and spend it with a SIBLING dialer that
            // fails for a REACHABILITY reason. The point of this loop is that
            // these codes never reach clearnet even when a switch WOULD
            // otherwise be allowed, so the window has to be genuinely open when
            // they are exercised.
            //
            // Advancing the clock is not enough, and neither is dialling THIS
            // dialer: since phase 2 the window has a second conjunct, and these
            // three codes deliberately do NOT start a failure run (they are not
            // evidence the private path is failing). So a loop that spent only
            // silence would find `may_switch_to_direct` false for a reason that
            // has nothing to do with the error set — VACUOUS, and row 773's
            // mutant (widening the arm to a policy refusal) would stop reddening
            // it. The sibling shares the posture, so its failing run opens the
            // window that this dialer's excluded code must then decline.
            let failing = PolicyDialer::with_fallback(
                MockDialer::failing(|| DialError::Timeout) as Arc<dyn NetDialer>,
                Arc::clone(&direct) as Arc<dyn NetDialer>,
                Arc::clone(&flag),
                Default::default(),
            );
            spend_the_window(&failing, Some("wallet-sync")).await;
            assert!(
                flag.may_switch_to_direct(PathClass::Sync),
                "{code:?}: the window must be OPEN, or this loop proves nothing"
            );
            let r = pd.dial("zec.example", 443, Some("wallet-sync")).await;
            let surfaced = match r {
                Err(DialError::NotReady) => "NotReady",
                Err(DialError::Unsupported) => "Unsupported",
                Err(DialError::Retired) => "Retired",
                Err(other) => panic!("{code:?}: expected the code's own variant, got {other:?}"),
                Ok(_) => panic!("{code:?}: expected the code's own variant, got a stream"),
            };
            assert_eq!(surfaced, expected, "{code:?} maps to its own variant");
            assert!(
                !flag.fell_back(),
                "{code:?} is not a reachability failure — the latch never moves"
            );
            assert_eq!(
                direct.call_count(),
                0,
                "{code:?}: clearnet was never dialed"
            );
        }
    }

    /// Spec §8 T3 / §6.1 E1 / §6.2 row 1: a `HostDialer` runtime with NOTHING
    /// registered is refused at the door under `Required` AND `Preferred` — the
    /// door's `InvalidEndpoint` with the registration reason, never a first-dial
    /// failure (an https endpoint, nothing else wrong). And a registration
    /// CLEARED mid-session (the door already passed): every dial fails `Retired`
    /// through the policy and `Preferred` never reaches clearnet for it.
    #[tokio::test]
    async fn an_empty_registry_under_required_never_falls_back() {
        use crate::config::{LightServerEndpoint, validate_transport};
        use crate::error::WalletError;
        let tls = LightServerEndpoint::new("https://zec.rocks:443").expect("tls");
        let empty = Arc::new(ScriptedHostDialer::unregistered());
        for required in [true, false] {
            let policy = host_policy(required, &empty);
            assert!(
                matches!(
                    validate_transport(&tls, &[], &policy),
                    Err(WalletError::InvalidEndpoint { reason })
                        if reason.contains("no host dialer registered")
                ),
                "required={required}: nothing registered is refused at the door, with the reason"
            );
        }
        // A registered host passes the same door.
        let host = Arc::new(ScriptedHostDialer::ready());
        assert!(validate_transport(&tls, &[], &host_policy(true, &host)).is_ok());

        // Mid-session CLEAR: the descriptor goes, dials fail `Retired`, and the
        // Preferred plan (whose fallback is the REAL direct dialer here) never
        // reaches it — the error is the host's, the latch stays false.
        host.set_descriptor(None);
        host.set_outcome(Err(HostDialCode::Retired));
        for required in [true, false] {
            let flag = Arc::new(TorPosture::new());
            let policy = host_policy(required, &host);
            let plan = resolve_dialer(&policy, &flag, &Default::default()).expect("resolves");
            assert!(
                matches!(
                    plan.dialer
                        .dial("zec.example", 443, Some("wallet-sync"))
                        .await,
                    Err(DialError::Retired)
                ),
                "required={required}: a cleared registry fails typed, never direct"
            );
            assert!(!flag.fell_back(), "required={required}: no fallback");
            assert_eq!(
                live_tor_state(
                    &policy,
                    false,
                    &SyncStatus::Idle,
                    policy.host_descriptor(),
                    flag.is_unanswered(),
                    flag.path_refused(),
                ),
                TorState::Unavailable { transport: None },
                "required={required}: nothing registered ⇒ Unavailable, and there is \
                 no transport to name"
            );
        }
    }

    /// Spec §8 T7 / §3.3 row 4: the host's `REFUSED` ("the transport will not
    /// carry this request") crosses as `Unsupported` — a policy, not a fault —
    /// and `Preferred` does not fall back on it; `Required` surfaces it as-is.
    #[tokio::test]
    async fn cabi_unsupported_is_not_a_reachability_failure_and_never_falls_back() {
        let host = Arc::new(ScriptedHostDialer::with(
            Some(ScriptedHostDialer::tor_ready()),
            Err(HostDialCode::Refused),
        ));
        let direct = MockDialer::ok();
        let flag = Arc::new(TorPosture::new());
        let preferred = PolicyDialer::with_fallback(
            Arc::clone(&host) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&flag),
            Default::default(),
        );
        assert!(matches!(
            preferred
                .dial("zec.example", 443, Some("wallet-sync"))
                .await,
            Err(DialError::Unsupported)
        ));
        assert!(!flag.fell_back(), "Preferred did not fall back");
        assert_eq!(direct.call_count(), 0, "clearnet never dialed");

        let plan = resolve_dialer(&host_policy(true, &host), &flag, &Default::default())
            .expect("resolves");
        assert!(matches!(
            plan.dialer
                .dial("zec.example", 443, Some("wallet-sync"))
                .await,
            Err(DialError::Unsupported)
        ));
        assert!(!flag.fell_back());
    }

    /// Spec §8 T9 / ADR-0545 D2 / §6.2 last row: a host transport that declares
    /// isolation `Unsupported` still receives the isolation key on the dial (the
    /// SDK passes it regardless), the dial PROCEEDS (the retired ADR-0526 clause
    /// would have refused with `Unsupported`), and the state reports the
    /// linkability honestly: `Active { HostDialer { name, isolation: Unsupported, exposure } }`.
    #[tokio::test]
    async fn a_non_isolating_descriptor_dials_without_isolation_and_the_state_reports_it() {
        let host = Arc::new(ScriptedHostDialer::with(
            Some(HostTransportDescriptor {
                name: name("Shadowsocks"),
                readiness: 100,
                isolation: IsolationSupport::Unsupported,
                exposure: TransportExposure::Hidden,
                health: TransportHealth::Ready,
            }),
            Ok(()),
        ));
        let flag = Arc::new(TorPosture::new());
        let policy = host_policy(true, &host);
        let plan = resolve_dialer(&policy, &flag, &Default::default()).expect("resolves");
        assert!(
            plan.dialer
                .dial("zec.example", 443, Some("wallet-send-abc"))
                .await
                .is_ok(),
            "a non-isolating host transport carries the dial — it never refuses for the key"
        );
        assert_eq!(
            host.keys_seen(),
            vec![Some("wallet-send-abc".to_string())],
            "the isolation key is still passed, verbatim (the host ignores it)"
        );
        assert_eq!(
            live_tor_state(
                &policy,
                flag.fell_back(),
                &SyncStatus::Idle,
                policy.host_descriptor(),
                flag.is_unanswered(),
                flag.path_refused(),
            ),
            TorState::Active {
                runtime: TorRuntimeKind::HostDialer {
                    name: name("Shadowsocks"),
                    isolation: IsolationSupport::Unsupported,
                    exposure: TransportExposure::Hidden,
                }
            },
            "the state says which path and that connections can be linked on it"
        );
    }

    /// The wave review's MEDIUM (all three angles): a descriptor that says
    /// "not ready" fails the dial `NotReady` BEFORE the host is asked — so a host
    /// that parks the dial while its transport bootstraps (instead of refusing
    /// `NOT_READY` synchronously) can never run the SDK into `DIAL_TIMEOUT_SECS`,
    /// a reachability `Timeout` that `Preferred` would follow to clearnet at
    /// every app start. A cleared registry (`None`) fails `Retired` the same
    /// way; the host is called only once the descriptor reads ready. Under
    /// `Preferred` neither outcome writes the fell-back latch.
    #[tokio::test]
    async fn a_not_ready_descriptor_fails_the_dial_before_the_host_is_called() {
        use crate::net::host_dialer::testing::{ScriptedHostDialer, name};
        use crate::net::host_dialer::{
            HostDialer, HostTransportDescriptor, IsolationSupport, TransportExposure,
            TransportHealth,
        };
        let host = Arc::new(ScriptedHostDialer::with(
            Some(HostTransportDescriptor {
                name: name("Tor"),
                readiness: 42,
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
                health: TransportHealth::Starting,
            }),
            Ok(()),
        ));
        let policy = TorPolicy::Preferred {
            runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
        };
        let flag = Arc::new(TorPosture::new());
        let plan = resolve_dialer(&policy, &flag, &Default::default())
            .expect("a registered host dialer resolves");
        assert!(
            matches!(
                plan.dialer
                    .dial("zec.example", 443, Some("wallet-sync"))
                    .await,
                Err(DialError::NotReady)
            ),
            "not ready ⇒ NotReady from the gate, never a host dial, never a fallback"
        );
        assert!(
            host.keys_seen().is_empty(),
            "the host was never asked to dial"
        );
        assert!(!flag.fell_back(), "NotReady never writes the latch");
        host.set_descriptor(None);
        assert!(
            matches!(
                plan.dialer
                    .dial("zec.example", 443, Some("wallet-sync"))
                    .await,
                Err(DialError::Retired)
            ),
            "cleared ⇒ Retired from the gate"
        );
        assert!(host.keys_seen().is_empty());
        host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
        assert!(
            plan.dialer
                .dial("zec.example", 443, Some("wallet-sync"))
                .await
                .is_ok(),
            "ready ⇒ the host dials"
        );
        assert_eq!(
            host.keys_seen().len(),
            1,
            "exactly the ready dial reached the host"
        );
        assert!(!flag.fell_back());

        // ADR-0549 AT THE SEAM, not at the predicate. `health = FAILED` at
        // readiness 100 — the ONE case no registrant could express before v3,
        // and the one a "simplification" would reopen: this gate reads like a
        // readiness gate, so inlining `readiness < HOST_TRANSPORT_READY` here
        // would pass every OTHER test in this file and send a declared-failed
        // transport's traffic to clearnet under `Preferred`. Composing
        // `!failed_at_100.is_ready()` with the rows above does not cover it —
        // the two tests never meet (REVIEW.md §3, the pure-function-vs-seam
        // tell). So: the DIAL is refused, the host is never asked, and the
        // latch stays false, which together are "a declared failure cannot
        // become a leak" — on THIS dial. Since phase 2 the refusal carries the
        // health verdict (`TransportFailed`, not `NotReady`) and a full minute
        // of it is switch-eligible (ADR-0553); one refusal never is.
        host.set_descriptor(Some(HostTransportDescriptor {
            name: name("Shadowsocks"),
            readiness: 100,
            isolation: IsolationSupport::Supported,
            exposure: TransportExposure::Hidden,
            health: TransportHealth::Failed,
        }));
        assert!(
            matches!(
                plan.dialer
                    .dial("zec.example", 443, Some("wallet-sync"))
                    .await,
                Err(DialError::TransportFailed)
            ),
            "a FAILED transport at readiness 100 is refused at the gate — health \
             forbids a dial, it never permits one"
        );
        assert_eq!(
            host.keys_seen().len(),
            1,
            "the host was NOT asked to dial a transport it declared failed"
        );
        assert!(
            !flag.fell_back(),
            "a declared failure does not reach the clearnet fallback on its first refusal"
        );
    }

    // ----- ADR-0552: the maintainer's minute -------------------------------------
    // The maintainer's words: "keep waiting, but I think 1 minute is enough to
    // switch with notification". These pin WHEN a `Preferred` wallet may leave
    // the private path; `preferred_fell_back_on_unreachable_or_timeout_only`
    // above pins WHICH failures may take it there.

    /// A reachability failure INSIDE the window sends no clearnet packet: the
    /// error returns typed and the fallback dialer is never even called.
    #[tokio::test(start_paused = true)]
    async fn a_preferred_wallet_does_not_go_clearnet_inside_the_patience_window() {
        let host = MockDialer::failing(|| DialError::Unreachable);
        let direct = MockDialer::ok();
        let posture = Arc::new(TorPosture::new());
        let pd = PolicyDialer::with_fallback(
            Arc::clone(&host) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );

        let first = pd.dial("zec.example", 443, Some("wallet-sync")).await;
        assert!(
            matches!(first, Err(DialError::Unreachable)),
            "the first failure keeps insisting on the private path"
        );

        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        let last = pd.dial("zec.example", 443, Some("wallet-sync")).await;
        assert!(
            matches!(last, Err(DialError::Unreachable)),
            "one second short of the minute STILL insists"
        );

        assert_eq!(
            direct.call_count(),
            0,
            "not one clearnet dial inside the window — the leak this window exists to prevent"
        );
        assert!(
            !posture.fell_back(),
            "and nothing latched, so the wallet never reports a degradation it did not make"
        );
    }

    /// And the first qualifying failure AFTER the minute switches, in that same
    /// call — the user does not wait another dial bound for it.
    #[tokio::test(start_paused = true)]
    async fn the_first_qualifying_failure_after_the_minute_switches_and_latches() {
        let host = MockDialer::failing(|| DialError::Timeout);
        let direct = MockDialer::ok();
        let posture = Arc::new(TorPosture::new());
        let pd = PolicyDialer::with_fallback(
            Arc::clone(&host) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );

        spend_the_window(&pd, Some("wallet-sync")).await;
        let r = pd.dial("zec.example", 443, Some("wallet-sync")).await;
        assert!(
            r.is_ok(),
            "the minute is spent: this dial switches to direct"
        );
        assert_eq!(direct.call_count(), 1, "exactly one clearnet dial");
        assert!(
            posture.fell_back(),
            "and the switch is VISIBLE — tor_state() reads FellBack"
        );
    }

    /// A private dial that is CONFIRMED by an RPC restarts the minute (ADR-0552
    /// decision 4), so a wallet on a flaky-but-alive private path never
    /// accumulates its way to clearnet — and a dial ALONE does not, so a
    /// transport that accepts and blackholes cannot hold the wallet on a path
    /// that carries nothing (stage 1b).
    ///
    /// This test used to assert the first half with a bare dial, which is the
    /// defect the built-diff review found: the reset it pinned was reachable by
    /// a connect that had delivered no byte. It now pins BOTH halves, so the
    /// distinction itself is guarded rather than the old behaviour.
    #[tokio::test(start_paused = true)]
    async fn a_successful_private_dial_resets_the_patience_clock() {
        let direct = MockDialer::ok();
        let posture = Arc::new(TorPosture::new());
        let working = PolicyDialer::with_fallback(
            MockDialer::ok() as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );
        let failing = PolicyDialer::with_fallback(
            MockDialer::failing(|| DialError::Timeout) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );

        // HALF ONE — a bare CONNECT buys nothing. The private path is dialled
        // successfully in the MIDDLE of a failing run on the same class, and the
        // switch still arrives on time, because only a confirmed RPC moves the
        // silence stamp or ends the run. (What this half does NOT drive is the
        // accepted-and-silent shape itself: the run here is made of dials that
        // FAIL. The accept is placed inside the window because, since phase 2
        // closed the consumer gap, an accept on a SPENT window switches too.)
        assert!(
            failing
                .dial("zec.example", 443, Some("wallet-sync"))
                .await
                .is_err(),
            "the first failure of the run starts the clock"
        );
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS / 2)).await;
        assert!(
            working
                .dial("zec.example", 443, Some("wallet-sync"))
                .await
                .is_ok(),
            "the private path ACCEPTS a dial half-way through the run"
        );
        assert_eq!(
            direct.call_count(),
            0,
            "inside the window that accept is served by the private arm"
        );
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS / 2 + 1)).await;
        assert!(
            failing
                .dial("zec.example", 443, Some("wallet-sync"))
                .await
                .is_ok(),
            "a bare connect is not evidence: the window is spent and this switches"
        );
        assert_eq!(
            direct.call_count(),
            1,
            "a connect that carried nothing neither restarted the window nor held the wallet"
        );

        // HALF TWO — a CONFIRMED success (the dial plus the RPC that proves the
        // circuit carried gRPC) does buy another minute, on a fresh posture so
        // the sticky latch above cannot confuse the reading.
        let posture = Arc::new(TorPosture::new());
        let direct = MockDialer::ok();
        let working = PolicyDialer::with_fallback(
            MockDialer::ok() as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );
        let failing = PolicyDialer::with_fallback(
            MockDialer::failing(|| DialError::Timeout) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        assert!(
            working
                .dial("zec.example", 443, Some("wallet-sync"))
                .await
                .is_ok()
        );
        // What `net/grpc.rs` does when a response comes back over that circuit.
        posture.note_private_success(PathClass::Sync, DialArm::Private);

        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        let r = failing.dial("zec.example", 443, Some("wallet-sync")).await;
        assert!(
            matches!(r, Err(DialError::Timeout)),
            "the window restarted at that CONFIRMED success, so this failure still insists"
        );
        assert_eq!(
            direct.call_count(),
            0,
            "the confirmed reset bought another minute — no clearnet dial yet"
        );
    }

    /// STAGE 1b, finding (iii): the switch must be REACHABLE on the money path.
    ///
    /// `send_transaction` wraps the whole lazy connect in the unary budget. When
    /// that budget equalled the dial bound — both 30 s before this — a hanging
    /// private dialer raced: the caller could cancel the dial future at the very
    /// instant the dial bound fired, so `PolicyDialer` never observed an outcome
    /// and neither the success record nor the fallback arm ran. A `Preferred`
    /// wallet would then fail every send, inside the minute, forever, having
    /// never once evaluated the switch the maintainer ruled for.
    ///
    /// Driven on a BROADCAST isolation key on purpose: it is the class where a
    /// starved switch costs a payment, and it proves the class keying and the
    /// timing margin together.
    #[tokio::test(start_paused = true)]
    async fn a_hanging_primary_switches_on_the_broadcast_path() {
        let direct = MockDialer::ok();
        let posture = Arc::new(TorPosture::new());
        let pd = PolicyDialer::with_fallback(
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );

        let key = format!("{BROADCAST_ISOLATION_KEY_PREFIX}-0badc0de0badc0de");
        // The broadcast class has carried nothing AND has been failing for the
        // whole window — both conjuncts, on the class this test measures.
        spend_the_window(&pd, Some(&key)).await;

        let start = tokio::time::Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
            pd.dial("zec.example", 443, Some(&key)),
        )
        .await;

        let dialed = outcome.expect(
            "the CALLER's unary budget must not cancel the dial: the dial bound is strictly \
             below it, so the dialer observes the timeout and gets to decide",
        );
        // THE MARGIN IS THE PROPERTY, and asserting only the outcome above does
        // not reach it: on a paused clock two timers set to the SAME instant
        // both fire, and the inner one is polled first, so the switch happens
        // even under the equality defect. (The first version of this test did
        // exactly that and SURVIVED its own mutant.) What equality really costs
        // is the room to act, so measure it: the dial must resolve with seconds
        // to spare inside the caller's budget, not at the same tick as it.
        let waited = start.elapsed();
        assert!(
            waited >= Duration::from_secs(DIAL_TIMEOUT_SECS)
                && waited < Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
            "the dial bound must fire STRICTLY inside the caller's budget (virtual {waited:?}; \
             bound {DIAL_TIMEOUT_SECS}s, caller {GRPC_UNARY_TIMEOUT_SECS}s) — at equality the \
             real-clock race is what decides, and a cancelled dial means the switch is never \
             evaluated"
        );
        assert!(
            dialed.is_ok(),
            "and having decided, it switched — the send leaves over clearnet rather than \
             failing forever inside a minute that never ends"
        );
        assert_eq!(direct.call_count(), 1, "exactly one clearnet dial");
        assert!(
            posture.fell_back(),
            "and the switch is VISIBLE — tor_state() reads FellBack"
        );
    }

    /// `Required` has no fallback, so no amount of silence can move it. Pinned
    /// with a silence far past the window (ADR-0552 decision 5).
    #[tokio::test(start_paused = true)]
    async fn required_never_switches_however_long_the_private_path_is_silent() {
        let host = MockDialer::failing(|| DialError::Unreachable);
        let pd = PolicyDialer::fail_closed(
            Arc::clone(&host) as Arc<dyn NetDialer>,
            Arc::new(TorPosture::new()),
            Default::default(),
        );

        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS * 10)).await;
        let r = pd.dial("zec.example", 443, Some("wallet-sync")).await;
        assert!(
            matches!(r, Err(DialError::Unreachable)),
            "Required fail-closes for as long as the private path is down"
        );
        assert!(!pd.fell_back(), "and it can never latch a degradation");
    }

    /// The window changes WHEN a qualifying failure switches, never WHICH
    /// failures qualify: ADR-0546's set is untouched, so a non-reachability
    /// failure still propagates after the minute has run out.
    #[tokio::test(start_paused = true)]
    async fn a_non_reachability_failure_never_switches_even_after_the_minute() {
        let direct = MockDialer::ok();
        let posture = Arc::new(TorPosture::new());
        let pd = PolicyDialer::with_fallback(
            MockDialer::failing(|| DialError::Unsupported) as Arc<dyn NetDialer>,
            Arc::clone(&direct) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );

        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS * 2)).await;
        let r = pd.dial("zec.example", 443, Some("wallet-sync")).await;
        assert!(
            matches!(r, Err(DialError::Unsupported)),
            "an isolation-unsupported dialer is a configuration truth, not a reachability failure"
        );
        assert_eq!(direct.call_count(), 0, "so it reaches no clearnet");
        assert!(!posture.fell_back());
    }

    // ----- stage S1 `window`: the clearnet leg of a switch ---------------------
    // `docs/plan/stage-1-private-path-truth.md` §3.1 and `tor-patience-phase-2.md`
    // §2d/§3: `DIAL < UNARY` makes the switch REACHABLE inside one RPC; these pin
    // that it is USABLE — the clearnet leg fits what the private dial left of the
    // caller's wait, and a leg that does not fit fails typed instead of outliving
    // the caller that asked for it.

    /// A clearnet leg that takes `delay` to establish: a working network that is
    /// not instant. Every `MockDialer::ok()` fallback above answers in zero time,
    /// which is the one clearnet no censored user has.
    struct SlowDialer {
        delay: Duration,
        then: Arc<MockDialer>,
    }

    #[async_trait]
    impl NetDialer for SlowDialer {
        async fn dial(
            &self,
            host: &str,
            port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, DialError> {
            tokio::time::sleep(self.delay).await;
            self.then.dial(host, port, isolation_key).await
        }
    }

    /// `tor-patience-phase-2.md` §5: the fallback leg COMPLETES inside the
    /// caller's budget. The private dial hangs for its whole bound and the
    /// clearnet leg then takes four seconds: 29 of the caller's 30. Whatever
    /// bounds the clearnet leg must leave room for this one.
    #[tokio::test(start_paused = true)]
    async fn the_fallback_leg_completes_inside_the_callers_budget() {
        let direct = MockDialer::ok();
        let posture = Arc::new(TorPosture::new());
        let pd = PolicyDialer::with_fallback(
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::new(SlowDialer {
                delay: Duration::from_secs(4),
                then: Arc::clone(&direct),
            }) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );
        spend_the_window(&pd, Some("wallet-sync")).await;

        let start = tokio::time::Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
            pd.dial("zec.example", 443, Some("wallet-sync")),
        )
        .await;
        let dialed = outcome.expect("the switch and its clearnet leg fit the caller's budget");
        assert!(
            dialed.is_ok(),
            "a clearnet leg that needs four seconds is a working network — it must not be cut"
        );
        assert_eq!(
            start.elapsed(),
            Duration::from_secs(DIAL_TIMEOUT_SECS + 4),
            "the private bound, then the clearnet leg, and nothing else"
        );
        assert!(posture.fell_back(), "and the switch is VISIBLE");
        assert_eq!(direct.call_count(), 1, "exactly one clearnet dial");
    }

    /// The other side of the same budget, and the half that is missing today: a
    /// clearnet leg that does NOT establish is cut inside the caller's wait, with
    /// a typed error the dialer itself returns.
    ///
    /// The private dial is bounded (`DIAL_TIMEOUT_SECS`) and so is the real
    /// clearnet dialer's own connect, but the LEG — what `PolicyDialer` awaits
    /// after it has latched and sent the packet — is not: it is awaited for as
    /// long as it takes, the caller's budget abandons it, and the next attempt
    /// starts from the top. The honest residual the plan accepts is "fails that
    /// attempt"; an attempt that never returns has not failed, it has been lost.
    #[tokio::test(start_paused = true)]
    async fn a_clearnet_leg_that_never_establishes_is_cut_inside_the_callers_budget() {
        let posture = Arc::new(TorPosture::new());
        let pd = PolicyDialer::with_fallback(
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );
        spend_the_window(&pd, Some("wallet-sync")).await;

        let start = tokio::time::Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
            pd.dial("zec.example", 443, Some("wallet-sync")),
        )
        .await;
        let waited = start.elapsed();
        let dialed = outcome.unwrap_or_else(|_| {
            panic!(
                "the caller's {GRPC_UNARY_TIMEOUT_SECS} s ran out ({waited:?}) with the dialer \
                 still awaiting a clearnet leg that has no bound of its own — the switch was \
                 made, the wallet latched, and the dial never returned"
            )
        });
        assert!(
            dialed.is_err(),
            "a clearnet leg that never establishes fails the attempt, typed"
        );
        assert!(
            posture.fell_back(),
            "the clearnet dial WAS made, so the latch stands whatever became of it"
        );
    }

    /// The bound's NAME and VALUE, as `tor-patience-phase-2.md` §3 (c) decides
    /// them and §3.1 prices them: `FALLBACK_ESTABLISH_BUDGET_SECS` = 5, ONE bound
    /// for the whole post-switch establishment, and `DIAL + FALLBACK <= UNARY`
    /// as a compile-time relation like the one already at `constants.rs`. A
    /// clearnet leg that never establishes is cut at exactly the private bound
    /// plus the fallback budget — no earlier (a slow network is not a failure)
    /// and no later (the caller has nothing left).
    ///
    /// Written against the contract's name; it cannot compile at the base
    /// commit, so it is committed apart from the rows that ran red there.
    #[tokio::test(start_paused = true)]
    async fn the_clearnet_leg_is_cut_at_the_fallback_establish_budget() {
        use crate::constants::FALLBACK_ESTABLISH_BUDGET_SECS;
        const { assert!(DIAL_TIMEOUT_SECS + FALLBACK_ESTABLISH_BUDGET_SECS <= GRPC_UNARY_TIMEOUT_SECS) };
        assert_eq!(
            FALLBACK_ESTABLISH_BUDGET_SECS, 5,
            "the budget the contract priced; a different number is a decision (phase-2 §3)"
        );

        let posture = Arc::new(TorPosture::new());
        let pd = PolicyDialer::with_fallback(
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::new(HangingDialer) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        );
        spend_the_window(&pd, Some("wallet-sync")).await;

        let start = tokio::time::Instant::now();
        // Bounded by the TEST, a second past the caller's budget: an unbounded
        // leg would otherwise park a paused runtime for ever instead of failing.
        let r = tokio::time::timeout(
            Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS + 1),
            pd.dial("zec.example", 443, Some("wallet-sync")),
        )
        .await
        .expect("the clearnet leg is bounded at all");
        assert!(
            matches!(r, Err(DialError::Timeout)),
            "a clearnet leg that never establishes is a typed Timeout"
        );
        assert_eq!(
            start.elapsed(),
            Duration::from_secs(DIAL_TIMEOUT_SECS + FALLBACK_ESTABLISH_BUDGET_SECS),
            "the private bound, then exactly the fallback budget, and nothing else"
        );
    }

    /// A `Preferred` plan's stall reason NEVER depends on the fell-back latch
    /// (the security angle's HIGH on ADR-0552, — the honesty half was
    /// built here and withdrawn the same day).
    ///
    /// Why this is the property worth pinning rather than the appealing one:
    /// the plan is built ONCE for the long-lived sync client, so a
    /// latch-derived reason freezes at "not yet switched" for the session and
    /// keeps asserting it after the wallet has gone to clearnet — while
    /// `walletTorUnavailable`'s copy says "nothing was sent in the clear".
    /// That is a false privacy assurance at the exact moment the user's
    /// address reaches the server. `Required` keeps `TorUnavailable`, where it
    /// is unconditionally true.
    #[tokio::test]
    async fn a_preferred_plan_never_claims_the_private_path_from_a_latched_flag() {
        let host = MockDialer::ok();
        let policy = TorPolicy::Preferred {
            runtime: TorRuntime::Dialer(Arc::clone(&host) as Arc<dyn NetDialer>),
        };
        let posture = Arc::new(TorPosture::new());

        let before = resolve_dialer(&policy, &posture, &Default::default()).expect("resolves");
        assert_eq!(
            before.stall_on_failure,
            StallReason::EndpointUnreachable,
            "a Preferred plan claims nothing about the private path"
        );

        posture.latch_fell_back();
        let after = resolve_dialer(&policy, &posture, &Default::default()).expect("resolves");
        assert_eq!(
            after.stall_on_failure,
            StallReason::EndpointUnreachable,
            "and it STILL claims nothing once the wallet has switched — a plan-time \
             read of the latch would be frozen and would lie in one direction or the other"
        );
    }
}
