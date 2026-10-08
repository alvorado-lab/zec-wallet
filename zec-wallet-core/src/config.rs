//! Wallet configuration (spec §2.3). POLICY (what must happen) × RUNTIME
//! (whose Tor does it) are orthogonal — the SDK can reuse the host app's
//! network infrastructure instead of always spawning its own (maintainer
//! 2026-06-10).

use std::path::PathBuf;
use std::sync::Arc;

use crate::constants::{
    BROADCAST_JITTER_MAX_MS_CEILING, BROADCAST_JITTER_MAX_MS_DEFAULT,
    ENDPOINT_AUTH_VALUE_MAX_BYTES, SYNC_SERVER_AUTH_HEADER_MAX_BYTES, SYNC_SERVER_URL_MAX_BYTES,
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::error::WalletError;
use crate::memo::MachineMemoPrefix;
use crate::money::{BlockHeight, Network};
use crate::net::host_dialer::{HostDialer, HostTransportDescriptor};
use crate::ports::NetDialer;
use crate::seed::SeedPersistence;
use crate::sync_server::SyncServer;

/// Wallet configuration. Every field is REQUIRED — no `Default`: the
/// endpoint (maintainer 2026-06-10: the SDK never picks one silently) and the
/// Tor policy (a conscious privacy decision, §5.2) must be stated by the
/// host.
pub struct WalletConfig {
    /// Host-owned data location (platform app dir). Config, not convention —
    /// this is what lets a host scope wallets per identity slot (§4.5).
    pub db_dir: PathBuf,
    pub network: Network,
    /// lightwalletd-protocol (Zaino-compatible) gRPC endpoint. REQUIRED, no
    /// default; the host feeds endpoints at W5.
    pub endpoint: LightServerEndpoint,
    /// OPTIONAL auth header sent on EVERY light-server request, for an endpoint
    /// behind a proxy that gates access (maintainer decision header key plus
    /// proxy rate-limits and per-IP quotas). `None` = send no auth header, which
    /// is every public endpoint. §5.4 NEVER-log — see [`EndpointAuth`].
    pub endpoint_auth: Option<EndpointAuth>,
    /// REQUIRED — forces a conscious privacy decision (§5.2).
    pub tor: TorPolicy,
    /// §4.2: `SealedKeychain` for Dart-path consumers; `None` for the host-supplied-seed path.
    pub seed_persistence: SeedPersistence,
    /// Restore-from height; `None` on create (= current tip). Validated at
    /// first sync: above-tip ⇒ typed `BirthdayInFuture` (a too-high birthday
    /// silently never finds the user's notes); below activation ⇒ clamped.
    pub birthday: Option<BlockHeight>,
    /// §5.3 broadcast-timing decorrelation. `JitterPolicy::default()` is the
    /// documented 0..=10 s uniform window.
    pub broadcast_jitter: JitterPolicy,
    /// FR-27 — the host's OPT-IN machine-memo read scope. EMPTY by default,
    /// and empty means the read verb is closed: `Wallet::machine_memos` on an
    /// unregistered scope is a typed refusal, never an empty answer that reads
    /// as "this transaction carries nothing".
    ///
    /// A host that writes its own envelope onto the chain registers the leading
    /// bytes it wants back. See [`MachineMemoPrefix`]
    /// for what a prefix does and does NOT mean — it bounds volume, not intent.
    pub machine_memo_prefixes: Vec<MachineMemoPrefix>,
    /// The sync servers this host OFFERS the user (`sync-server-picker.md` §2;
    /// P3-13). May be EMPTY — then the picker offers `endpoint` and a custom
    /// entry only. `endpoint` above is the DEFAULT: a wallet with no persisted
    /// choice dials it, and a choice that cannot be honoured falls back to it
    /// VISIBLY (`Wallet::sync_server_status`). A persisted choice WINS at open.
    /// Validated at the door by [`crate::sync_server::validate_sync_servers`]
    /// (bounded, ids unique); the reference two are
    /// `SyncServerCatalog::reference`.
    pub sync_servers: Vec<SyncServer>,
}

impl WalletConfig {
    /// Every cross-field rule create, watch-only create and open apply before
    /// touching anything: the machine-memo scope, the offered sync servers, the
    /// plaintext-transport rule and the jitter ceiling. The ONE list: each
    /// constructor calls this, and so does the bridge's `validateWalletConfig`
    /// pre-flight, so a pre-flight can never accept what a constructor refuses
    /// (FR-27, then N02 and its hardening in the 2026-10-06 follow-up review —
    /// three times a rule reached the constructors and not the pre-flight).
    pub fn validate(&self) -> Result<(), WalletError> {
        crate::memo::validate_machine_memo_scope(&self.machine_memo_prefixes)?;
        crate::sync_server::validate_sync_servers(&self.sync_servers)?;
        validate_transport(&self.endpoint, &self.sync_servers, &self.tor)?;
        self.broadcast_jitter.validate()
    }
}

/// Validated lightwalletd-protocol endpoint URL.
///
/// Accepted shapes: `https://host[:port]` anywhere, `http://` only for
/// loopback (local development against a localhost lightwalletd/Zaino —
/// plaintext to a remote host is never a configuration, it's a leak).
/// URLs carrying userinfo (`user:pass@`) are REJECTED outright: Basic-auth
/// userinfo is on the §5.4 never-log list, and refusing it at the door is
/// cheaper than policing every log line it could reach.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LightServerEndpoint(String);

impl LightServerEndpoint {
    pub fn new(url: impl Into<String>) -> Result<Self, WalletError> {
        let url = url.into();
        // §4.6 size-cap BEFORE the parser sees the bytes — HERE, in the ONE
        // validator, not only at the bridge (the P3-13 security review's
        // MEDIUM: a core caller could construct an endpoint the aux row's
        // bounded read-back would then refuse, and a switch onto it would
        // report "switched" while dialing the default). The bound is the
        // picker's URL bound; a hostname is at most 253 bytes, so nothing a
        // host or a user means as a server is near it.
        if url.len() > SYNC_SERVER_URL_MAX_BYTES {
            return Err(WalletError::InvalidEndpoint {
                reason: "url is longer than SYNC_SERVER_URL_MAX_BYTES",
            });
        }
        let uri: http::Uri = url.parse().map_err(|_| WalletError::InvalidEndpoint {
            reason: "unparseable url",
        })?;
        let authority = uri.authority().ok_or(WalletError::InvalidEndpoint {
            reason: "missing host",
        })?;
        if authority.as_str().contains('@') {
            return Err(WalletError::InvalidEndpoint {
                reason: "userinfo not allowed",
            });
        }
        let host = authority.host();
        if host.is_empty() {
            return Err(WalletError::InvalidEndpoint {
                reason: "missing host",
            });
        }
        // SEMANTIC loopback check (security fold, W2 review): parse the host
        // as an IP literal and use IpAddr::is_loopback — a string prefix test
        // would bless "127.evil.com" as loopback and ship plaintext gRPC to a
        // remote host. Only the literal "localhost" name qualifies by name.
        let bare = host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(host);
        let loopback = host == "localhost"
            || bare
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback());
        match uri.scheme_str() {
            Some("https") => {}
            Some("http") if loopback => {}
            Some("http") => {
                return Err(WalletError::InvalidEndpoint {
                    reason: "http only allowed for loopback",
                });
            }
            _ => {
                return Err(WalletError::InvalidEndpoint {
                    reason: "scheme must be https",
                });
            }
        }
        if uri.path() != "/" && !uri.path().is_empty() {
            return Err(WalletError::InvalidEndpoint {
                reason: "path not allowed",
            });
        }
        Ok(Self(url))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Do two endpoints name the SAME server — scheme, host (case-insensitive)
    /// and port with the scheme's default filled in? `https://zec.rocks` and
    /// `https://zec.rocks:443` are one server; a string comparison says two.
    /// The picker's same-server rule (P3-13, the security review's MEDIUM: a
    /// switch onto the server already in use must not rebuild the session,
    /// because a rebuild forgives the rewinding streak) compares with this,
    /// never with `as_str`.
    pub fn same_server(&self, other: &LightServerEndpoint) -> bool {
        fn parts(url: &str) -> Option<(String, String, u16)> {
            let uri: http::Uri = url.parse().ok()?;
            let scheme = uri.scheme_str()?.to_ascii_lowercase();
            let host = uri.host()?.to_ascii_lowercase();
            let port = uri
                .port_u16()
                .unwrap_or(if scheme == "https" { 443 } else { 80 });
            Some((scheme, host, port))
        }
        matches!((parts(&self.0), parts(&other.0)), (Some(a), Some(b)) if a == b)
    }

    /// Is this a plaintext (`http://`) endpoint? [`Self::new`] admits `http`
    /// for loopback only, so `true` means "a loopback development endpoint".
    /// Re-parsed rather than prefix-matched (the scheme is case-insensitive
    /// and `new` judged it through this same parser); an unparseable value —
    /// impossible after `new` — counts as plaintext, fail-closed.
    pub fn is_plaintext(&self) -> bool {
        match self.0.parse::<http::Uri>() {
            Ok(uri) => uri.scheme_str() != Some("https"),
            Err(_) => true,
        }
    }
}

/// The transport rule for a plaintext endpoint (FR-29 stage 0 (ii); ADR-0544
/// correction 3). `http://` — admitted by [`LightServerEndpoint::new`] for
/// loopback only — is a DEVELOPMENT convenience for the SDK's own direct
/// dialer, which resolves `localhost` on this machine. Under any other policy
/// the hostname is handed to the runtime's dialer (`Preferred` and `Required`
/// both dial the host's transport first), which resolves it wherever ITS
/// network says — and TLS is chosen by scheme alone, so the plaintext gRPC
/// (`SendTransaction`, `GetAddressUtxos`, compact blocks) would ride the host's
/// transport unencrypted. Refused at the door, for the default endpoint AND
/// every offered sync server; the runtime switch path has the same rule at
/// `LightwalletdClient::connect`.
pub(crate) fn validate_transport(
    endpoint: &LightServerEndpoint,
    sync_servers: &[SyncServer],
    tor: &TorPolicy,
) -> Result<(), WalletError> {
    if plaintext_admitted(tor) {
        return Ok(());
    }
    // FR-29 spec §2.1 / §6.1 E1 (RW-CFG-001, the T0-6 shape): a `HostDialer`
    // runtime with NOTHING registered is refused HERE, at the door every
    // constructor funnels through — never a first-dial failure, never a
    // clearnet fallback. The host registers its trampoline at trusted init,
    // BEFORE it validates the wallet's config (the header's lifecycle rule);
    // a `None` descriptor at validation is a host that registered late.
    if tor.host_dialer_unregistered() {
        return Err(WalletError::InvalidEndpoint {
            reason: "no host dialer registered — register before validating the config",
        });
    }
    let plaintext = endpoint.is_plaintext()
        || sync_servers
            .iter()
            .any(|server| server.endpoint().is_plaintext());
    if plaintext {
        return Err(WalletError::InvalidEndpoint {
            reason: "http only allowed under TorPolicy::Off (a host transport would carry plaintext)",
        });
    }
    Ok(())
}

/// An auth header sent on every request to the light server.
///
/// **Why this is its own type and NOT a field on [`LightServerEndpoint`].** That
/// type derives `Debug`, `PartialEq` and `Eq`, and it is carried through config
/// structs that get logged and compared. A secret inside it would reach a log the
/// first time anybody printed an endpoint. This type derives none of those: its
/// `Debug` names the header and **redacts the value**, and there is no `Display`
/// and no `as_str` for the value at all — the only way out is
/// [`Self::with_value`], a scoped read the transport adapter uses.
///
/// **What it is for, stated so it is not over-trusted.** A key shipped inside a
/// distributed app is extractable from the binary, so this is **abuse control**
/// — it makes an endpoint unusable by casual scrapers and gives the proxy a
/// handle for per-key rate-limits — and it is **not** access control against a
/// determined party. Maintainer decision, alongside proxy rate-limits and
/// per-IP quotas.
///
/// **A HOST'S VALUE MUST BE THE SAME FOR EVERY INSTALL.** A per-user or
/// per-install key defeats §2.3 broadcast unlinkability: [`crate::wallet`] mints
/// a fresh Tor circuit per `send_transaction` precisely so an endpoint cannot tie
/// a broadcast to the syncing wallet, and a distinct constant header re-joins
/// them by header equality on the very first request. One shared key is a
/// rate-limit handle; a per-user key is an identifier. **The one disclosed
/// exception is a USER's key for their own custom server (ADR-0568):** the user
/// brings it, the picker's trust notice tells them it identifies them to that
/// server, and the linkage is theirs to accept.
///
/// §5.4 NEVER-log. Every resident copy is `Zeroizing`: the config's
/// (`endpoint_auth`, and since P3-13 the offered list's gated entry and the
/// wallet's effective/default pair, which may be one, two or three copies of
/// the same value); since ADR-0568 a user's custom key — the bridge's
/// conversion (wrapped before its first check, [`Self::from_zeroizing`]), the
/// aux row's read (`sync_server::read`, copied only after its bound check),
/// `Resolved.auth`, the choice the status keeps (`Inner.sync_server_choice`)
/// and the session's `endpoint_auth` (what is NOT zeroized for a user's key —
/// SQLite's bound parameter, decrypted page cache and column text, the
/// `HeaderValue` check copy below — is listed in `sync-server-picker.md` §4);
/// and two clones per gRPC client, the probe's included: its interceptor's, and
/// the one its channel recipe keeps so a retired connection can be replaced
/// (`net/grpc.rs`). **Beyond that the transport
/// owns the bytes and does not zeroize
/// them:** the per-request header value is an un-zeroized `Bytes`, and h2
/// huffman-encodes it into a connection-lifetime write buffer. Same honest
/// framing as the swap client's (`zec-wallet-swap-near/src/http_client.rs`) —
/// what we control is `Zeroizing`, what hyper allocates is not.
#[derive(Clone)]
pub struct EndpointAuth {
    header: http::HeaderName,
    value: Zeroizing<String>,
}

impl EndpointAuth {
    /// Validate a header name/value pair for use as endpoint auth.
    ///
    /// Both halves are validated HERE rather than at first dial, for the same
    /// reason [`LightServerEndpoint`] validates its URL at construction: a
    /// credential that only fails on the network fails in front of a user.
    ///
    /// **The value check is the security-relevant one.** The charset check
    /// rejects CR, LF and NUL, which is what stops a crafted key from injecting
    /// a second header (or a whole request) into the stream.
    ///
    /// Note the endpoint's scheme is NOT checked here: `LightServerEndpoint`
    /// permits `http://` for loopback, so a developer mirroring a production
    /// config against `http://127.0.0.1:9067` sends this value in cleartext. That
    /// traffic never leaves the machine, so it is documented rather than refused. A host that reads its key
    /// from a config file or a `--dart-define` is one stray newline away from
    /// that, and the newline is invisible in most editors.
    ///
    /// Every refusal is [`WalletError::InvalidEndpointAuth`] (ADR-0568), so a
    /// caller tells a bad key from a bad URL by kind.
    pub fn new(header: impl AsRef<str>, value: impl Into<String>) -> Result<Self, WalletError> {
        Self::from_zeroizing(header, Zeroizing::new(value.into()))
    }

    /// [`Self::new`] for a caller that already holds the value `Zeroizing` — the
    /// bridge's conversion and the aux row's read (ADR-0568) — so the secret is
    /// moved in, never copied. Same checks, same refusals.
    pub fn from_zeroizing(
        header: impl AsRef<str>,
        value: Zeroizing<String>,
    ) -> Result<Self, WalletError> {
        // Bound the NAME before parsing it: since ADR-0568 a user types it.
        if header.as_ref().len() > SYNC_SERVER_AUTH_HEADER_MAX_BYTES {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header name is longer than SYNC_SERVER_AUTH_HEADER_MAX_BYTES",
            });
        }
        let header = http::HeaderName::try_from(header.as_ref()).map_err(|_| {
            WalletError::InvalidEndpointAuth {
                reason: "auth header name is not a valid HTTP header name",
            }
        })?;
        // A VALID HTTP HEADER NAME IS NOT ENOUGH — the transport needs a valid
        // gRPC *ASCII metadata key*, and the two differ in one place. tonic keys
        // metadata on the suffix: `Ascii::is_valid_key(k) == !k.ends_with("-bin")`
        // (`tonic/src/metadata/encoding.rs:115,184`), because `-bin` names carry
        // base64 binary values and a different value type.
        //
        // Caught by its own red-first: `x-key-bin` is a perfectly legal header
        // name, so it passed this door, and then `MetadataKey::from_bytes` failed
        // in the interceptor and refused **every request** — the wallet could not
        // talk to the endpoint at all, and the reason surfaced on the network as
        // an opaque `Unauthenticated`. That is exactly the failure this
        // constructor's own doc promises to prevent, so the check belongs here.
        if header.as_str().ends_with("-bin") {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header name must not end in -bin (that is a gRPC binary metadata key, which carries base64 and cannot hold this value)",
            });
        }
        // gRPC reserves the WHOLE `grpc-` prefix for the protocol, not only the
        // names listed below (`grpc-previous-rpc-attempts`, …); a credential
        // named into it collides with framing a future transport may set
        // (ADR-0568 design review — a user now types this name).
        if header.as_str().starts_with("grpc-") {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header name must not start with grpc- (gRPC reserves that prefix)",
            });
        }
        // Headers the transport OWNS. `insert` replaces, so naming one of these
        // as the auth header does not add a credential — it overwrites part of
        // the gRPC framing and breaks every call in a way that looks like a
        // server fault. A host that picks one has made a mistake; say so here.
        const TRANSPORT_OWNED: &[&str] = &[
            // gRPC's own reserved set.
            "content-type",
            "te",
            "user-agent",
            "grpc-timeout",
            "grpc-encoding",
            "grpc-accept-encoding",
            "grpc-status",
            "grpc-message",
            "grpc-message-type",
            // WIDENED by the security review, and these are worse than the
            // ones above because they fail SILENTLY. hyper strips all four from
            // every outbound h2 request — `CONNECTION_HEADERS` at
            // `hyper-1.10.1/src/proto/h2/mod.rs:36-41`, removed unconditionally
            // in `strip_connection_headers`. So naming one of them means the
            // credential is deleted before it reaches the wire and the wallet
            // talks to a gated endpoint with NO header, failing every call with
            // an opaque `Unauthenticated` and nothing local to say why.
            "keep-alive",
            "proxy-connection",
            "transfer-encoding",
            "upgrade",
            // Illegal or framing-owned in h2: `connection` is forbidden outright,
            // and `content-length`/`host` are set by the transport (hyper's
            // insert is `or_insert_with`, so a host-supplied value survives and
            // makes the request malformed).
            "connection",
            "content-length",
            "host",
        ];
        if TRANSPORT_OWNED.contains(&header.as_str()) {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header name is one the gRPC transport owns; setting it would overwrite the framing rather than add a credential",
            });
        }
        if value.is_empty() {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header value is empty",
            });
        }
        // Surrounding whitespace is a fat-finger, not a credential. `HeaderValue`
        // accepts it, the header goes out with it, and the endpoint answers 403 —
        // a failure whose cause is invisible in every log because the value is
        // (correctly) redacted everywhere. Refuse it rather than silently trim:
        // trimming a secret the host meant to send is its own surprise.
        if value.trim() != value.as_str() {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header value has leading or trailing whitespace",
            });
        }
        // Bound the length. Unbounded, a mis-pasted multi-megabyte "key" is
        // accepted here, blows past the server's SETTINGS_MAX_HEADER_LIST_SIZE on
        // every request, and huffman-encodes megabytes of the secret into h2
        // buffers nothing zeroizes. The sibling host-supplied string in this file
        // is capped the same way (`SOCKS_ADDR_MAX_BYTES`).
        if value.len() > ENDPOINT_AUTH_VALUE_MAX_BYTES {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header value is longer than ENDPOINT_AUTH_VALUE_MAX_BYTES",
            });
        }
        // THE VALUE'S CHARSET, AND `HeaderValue` IS NOT THE CHECK FOR IT.
        // WIDENED by the security review, which found this to be the exact
        // mirror of the `-bin` gap on the name side — the same defect class, in
        // the same constructor, in the commit written to close it.
        //
        // `http-1.4.2/src/header/value.rs:557-559` is
        // `is_valid(b) = b >= 32 && b != 127 || b == b'\t'`, so it accepts HTAB
        // and every byte in 0x80..=0xFF. gRPC's ASCII-Value is `1*(%x20-%x7E)`,
        // and tonic's `MetadataValue<Ascii>` delegates straight to `from_str`, so
        // nothing downstream catches it either. A key holding a typographic quote
        // pasted from a config file, or any UTF-8, is blessed here and then goes
        // out as raw multi-byte UTF-8: a conformant gateway rejects it, and a
        // latin-1-decoding proxy compares a DIFFERENT string. Either way every
        // call fails opaquely, which is what this constructor exists to prevent.
        if !value.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
            return Err(WalletError::InvalidEndpointAuth {
                reason: "auth header value must be printable ASCII only (0x20-0x7E) — gRPC ASCII metadata cannot carry HTAB or bytes above 0x7E",
            });
        }
        // Validate by construction, then DROP the probe: this is the only place
        // a non-`Zeroizing` copy of the secret exists, and it lives for one
        // statement. `from_str` is what rejects CR/LF/NUL. Kept even though the
        // charset check above subsumes it — it is the canonical refusal, and the
        // two disagreeing would be a bug worth failing on.
        http::HeaderValue::from_str(&value).map_err(|_| WalletError::InvalidEndpointAuth {
            reason: "auth header value has characters a header cannot carry (CR, LF or NUL)",
        })?;
        Ok(Self { header, value })
    }

    /// The header NAME — public config (`Debug` prints it too), never the
    /// value. The bridge returns it on a keyed custom choice so a picker can
    /// say a key is saved (ADR-0568).
    pub fn header_name(&self) -> &str {
        self.header.as_str()
    }

    pub(crate) fn header(&self) -> &http::HeaderName {
        &self.header
    }

    /// Read the secret inside a closure.
    ///
    /// **Scoped on purpose.** There is deliberately no `value(&self) -> &str`:
    /// a borrow that escapes is a borrow that can be formatted into a log or
    /// stored in an un-zeroizable buffer. The transport adapter uses this to
    /// build its per-request header and marks it sensitive there — the
    /// conversion lives in the adapter because `config` does not depend on the
    /// transport (hexagonal: policy here, protocol types there).
    pub(crate) fn with_value<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        f(&self.value)
    }
}

/// Equal when the header AND the value are — what decides whether a switch onto
/// the server in use keeps the session or rebuilds it with a new key
/// (ADR-0568). Hand-written so the value is compared in constant time
/// (`subtle`): the crypto audit ruled timing moot here (the caller is
/// in-process and the outcome is observable), and an early-exit `==` on a
/// secret type stays unwritten regardless.
impl PartialEq for EndpointAuth {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
            && bool::from(self.value.as_bytes().ct_eq(other.value.as_bytes()))
    }
}

impl Eq for EndpointAuth {}

impl core::fmt::Debug for EndpointAuth {
    /// Names the header, never the secret. A `Debug` that printed the value
    /// would put it in every log line that formats a config.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EndpointAuth")
            .field("header", &self.header.as_str())
            .field("value", &"<redacted>")
            .finish()
    }
}

/// The ONE home of the predicate "plaintext (`http://`) is admitted only
/// under `TorPolicy::Off`" (FR-29 stage 0 (ii)): [`validate_transport`] asks
/// it at the three doors and at `LightwalletdClient::connect`, and the
/// sync-server resolver asks it for a PERSISTED custom choice (spec §6.1 E12)
/// — so the door and the aux row can never disagree about the same URL.
pub(crate) fn plaintext_admitted(tor: &TorPolicy) -> bool {
    matches!(tor, TorPolicy::Off)
}

/// What MUST happen to wallet traffic (§2.3).
#[non_exhaustive]
pub enum TorPolicy {
    /// Direct gRPC.
    Off,
    /// Tor when reachable; falls back to clearnet WITH a surfaced
    /// `TorFellBack` event — degradation is visible, never silent (§6.1).
    Preferred { runtime: TorRuntime },
    /// Fail-closed: runtime unreachable ⇒ zero packets, sync `Stalled`.
    /// Covers swap API calls too — no independent clearnet path (§3.2 m5).
    ///
    /// READ THIS AS "only through the registered runtime", NOT "always
    /// private" (from the host's own use): a `HostDialer` host may
    /// legitimately register a DIRECT backing — Relim's `Required` means every
    /// dial goes through ITS dialer, whose backing is the direct one whenever
    /// the user turns the private path off, and de-registering it would leave
    /// the wallet with no dialer at all (a dead balance and no send) rather
    /// than with privacy. So `Required` guarantees the SDK emits nothing
    /// outside the host's transport; whether that transport is private is the
    /// descriptor's `exposure` axis, which is what keeps the "not private"
    /// rendering honest (ADR-0545/0547).
    Required { runtime: TorRuntime },
}

impl TorPolicy {
    /// The configured runtime, if the policy carries one (`Off` carries none).
    fn runtime(&self) -> Option<&TorRuntime> {
        match self {
            Self::Off => None,
            Self::Preferred { runtime } | Self::Required { runtime } => Some(runtime),
        }
    }

    /// The host transport's descriptor SNAPSHOT (FR-29 spec §3.4 / §0 A11):
    /// `Some` only for a `HostDialer` runtime with a registration — `None` for
    /// every other runtime AND for a `HostDialer` with nothing registered (or
    /// cleared). Read once per state derivation, never cached.
    pub(crate) fn host_descriptor(&self) -> Option<HostTransportDescriptor> {
        match self.runtime()? {
            TorRuntime::HostDialer(host) => host.descriptor(),
            _ => None,
        }
    }

    /// Is this a `HostDialer` policy with NOTHING registered — the door's E1
    /// refusal shape? `false` for every other runtime (they have no registry).
    fn host_dialer_unregistered(&self) -> bool {
        matches!(self.runtime(), Some(TorRuntime::HostDialer(_)))
            && self.host_descriptor().is_none()
    }
}

/// Whose transport carries it (§2.3; FR-29 spec §0 A1). Circuit-isolation
/// keys flow through EVERY runtime (ExternalSocks5 → per-key SOCKS auth,
/// Tor's IsolateSOCKSAuth; Dialer → the host's mapping; HostDialer → passed
/// on every dial, and a non-isolating host transport IGNORES them and never
/// refuses for them — ADR-0545 D2; the descriptor says whether they were
/// honoured — the `zec_wallet_tor` plugin maps them to arti circuit groups).
///
/// HONEST TRUST BOUNDARY: for external runtimes the SDK can verify it handed
/// bytes to the proxy/dialer — it cannot attest the host's infrastructure
/// actually routes through Tor. `Required` + external runtime = fail-closed
/// on unreachability; the torness CLAIM belongs to the host (§2.5
/// `TorState` reports which runtime is speaking so a UI can attribute it).
#[non_exhaustive]
pub enum TorRuntime {
    // THREE runtimes, none SDK-owned: the June `BuiltIn` variant (feature
    // `tor-builtin`) was REMOVED at FR-5 C1 (ADR-0548 D3). A host without a
    // transport adds the `zec_wallet_tor` plugin, which registers Tor through
    // `HostDialer` below; a Rust host wraps `dialer-tor` in a `Dialer`.
    /// Host-side Tor speaking SOCKS5 (Orbot, system Tor, a host sidecar).
    /// REFUSED at the config door (T0-6; ADR-0543: the loopback-SOCKS5 shape
    /// is out) — a Dart host uses [`Self::HostDialer`].
    ExternalSocks5 { addr: String },
    /// Rust-level injection: any byte-stream dialer — the host-network path
    /// for an IN-WORKSPACE host (wallet traffic rides the host's ONE arti
    /// instance via its transport). Stays for Rust hosts; a trait object
    /// cannot cross a dylib seam, which is what `HostDialer` is for.
    Dialer(Arc<dyn NetDialer>),
    /// The REGISTERED, cross-library dialer (FR-29; ADR-0543): the host's
    /// native library registered a trampoline through the C contract
    /// (`include/zec_wallet_net_dialer.h`) at trusted init, and swaps its
    /// backing transport (Tor, Shadowsocks, VLESS, direct) behind it. The
    /// bridge's cabi module implements [`HostDialer`] over that registry; the
    /// core consumes it like any other [`NetDialer`]. A `HostDialer` with
    /// nothing registered is refused at the config door (spec §6.1 E1);
    /// `Preferred` falls back on unreachable/timeout ONLY (ADR-0546).
    HostDialer(Arc<dyn HostDialer>),
}

/// §5.3 broadcast-timing decorrelation policy.
#[non_exhaustive]
pub enum JitterPolicy {
    /// No delay — honest opt-out (third-party choice, documented).
    None,
    /// Uniform random delay in `0..=max_ms` before tx broadcast.
    Uniform { max_ms: u64 },
}

impl Default for JitterPolicy {
    /// The documented default: uniform 0..=10 s (§5.3).
    fn default() -> Self {
        Self::Uniform {
            max_ms: BROADCAST_JITTER_MAX_MS_DEFAULT,
        }
    }
}

impl JitterPolicy {
    /// The config door (F01): a `Uniform` window above
    /// [`BROADCAST_JITTER_MAX_MS_CEILING`] is refused, never clamped, so a host's
    /// mistake surfaces at create/open instead of as a late broadcast. One of
    /// [`WalletConfig::validate`]'s rules.
    pub(crate) fn validate(&self) -> Result<(), WalletError> {
        match self {
            Self::Uniform { max_ms } if *max_ms > BROADCAST_JITTER_MAX_MS_CEILING => {
                Err(WalletError::BroadcastJitterTooLong {
                    max_ms: *max_ms,
                    ceiling_ms: BROADCAST_JITTER_MAX_MS_CEILING,
                })
            }
            Self::None | Self::Uniform { .. } => Ok(()),
        }
    }

    /// This policy with its window cut to at most `cap_ms` — what a
    /// deadline-tagged deposit group draws from (F01: a deposit never waits longer
    /// than the default window, whatever the host configured).
    pub(crate) fn capped(&self, cap_ms: u64) -> Self {
        match self {
            Self::None => Self::None,
            Self::Uniform { max_ms } => Self::Uniform {
                max_ms: (*max_ms).min(cap_ms),
            },
        }
    }

    /// Sample the §5.3 pre-broadcast delay (the send path sleeps it on the MONOTONIC
    /// `tokio::time` clock — never wall-clock, so a phone NTP jump can't fire it early).
    /// `None` ⇒ zero (the honest opt-out). `Uniform { max_ms }` ⇒ a uniform draw in the
    /// INCLUSIVE `0..=max_ms` from `OsRng`. This is timing DECORRELATION, not a crypto
    /// requirement: the modulo bias of one `u64` draw over a ~10 000-value range is far
    /// below timer granularity, so a rejection loop would be theater. `Uniform { max_ms:
    /// 0 }` collapses to zero (span 1), matching `None`.
    pub(crate) fn sample_delay(&self) -> std::time::Duration {
        match self {
            Self::None => std::time::Duration::ZERO,
            Self::Uniform { max_ms } => {
                // INCLUSIVE upper bound ⇒ span = max_ms + 1; saturating so max_ms == u64::MAX
                // never overflows (it clamps to a still-uniform draw across the full range).
                let span = max_ms.saturating_add(1);
                let ms = rand_core::RngCore::next_u64(&mut rand_core::OsRng) % span;
                std::time::Duration::from_millis(ms)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-29 stage 0 (ii) / ADR-0544 decision 3: a plaintext loopback endpoint
    /// is admitted ONLY under `TorPolicy::Off` (the SDK's own direct dialer). Under
    /// a host dialer — `Required` or `Preferred` — the name would be resolved and
    /// dialed by the host's transport and the gRPC would ride it unencrypted; the
    /// validator refuses, for the default endpoint and for an offered sync server
    /// alike. `https` is unaffected by the policy.
    #[test]
    fn http_endpoint_over_a_host_dialer_is_refused() {
        use crate::ports::testing::StubDialer;
        use crate::sync_server::SyncServerId;
        let host_dialer = || Arc::new(StubDialer) as Arc<dyn NetDialer>;
        let loopback = LightServerEndpoint::new("http://127.0.0.1:9067").expect("loopback");
        let tls = LightServerEndpoint::new("https://zec.rocks:443").expect("tls");
        let required = TorPolicy::Required {
            runtime: TorRuntime::Dialer(host_dialer()),
        };
        let preferred = TorPolicy::Preferred {
            runtime: TorRuntime::Dialer(host_dialer()),
        };
        for policy in [&required, &preferred] {
            assert!(
                matches!(
                    validate_transport(&loopback, &[], policy),
                    Err(WalletError::InvalidEndpoint { reason }) if reason.contains("TorPolicy::Off")
                ),
                "plaintext under a host dialer is refused at the door"
            );
            assert!(
                validate_transport(&tls, &[], policy).is_ok(),
                "https rides any policy"
            );
        }
        assert!(
            validate_transport(&loopback, &[], &TorPolicy::Off).is_ok(),
            "Off is the SDK's own direct dialer — loopback plaintext stays a development option"
        );
        // An OFFERED server is a dial target too: one plaintext entry in the list
        // is refused under a host dialer even when the default endpoint is https.
        let offered = SyncServer::new(
            SyncServerId::new("dev").expect("id"),
            "Dev",
            loopback.clone(),
            None,
        )
        .expect("offered server");
        assert!(
            matches!(
                validate_transport(&tls, std::slice::from_ref(&offered), &required),
                Err(WalletError::InvalidEndpoint { .. })
            ),
            "a plaintext offered server is refused under a host dialer"
        );
        assert!(
            validate_transport(&tls, std::slice::from_ref(&offered), &TorPolicy::Off).is_ok(),
            "and accepted under Off"
        );
        assert!(loopback.is_plaintext() && !tls.is_plaintext());
    }

    /// `WalletConfig::validate` is the one list every constructor and the
    /// bridge's pre-flight run (N02 and its security-review hardening,
    /// 2026-10-06): each of its four rules, broken alone, is refused by it, and
    /// the unbroken config passes — so a rule dropped from the list reds here.
    #[test]
    fn the_shared_config_validator_applies_every_rule() {
        use crate::ports::testing::StubDialer;
        use crate::sync_server::SyncServerId;
        let base = || WalletConfig {
            db_dir: PathBuf::from("/tmp/zw-validate"),
            network: Network::Test,
            endpoint: LightServerEndpoint::new("https://zec.rocks:443").expect("tls"),
            endpoint_auth: None,
            tor: TorPolicy::Off,
            seed_persistence: SeedPersistence::SealedKeychain,
            birthday: None,
            broadcast_jitter: JitterPolicy::default(),
            machine_memo_prefixes: Vec::new(),
            sync_servers: Vec::new(),
        };
        assert!(base().validate().is_ok(), "the unbroken fixture passes");

        let mut memo = base();
        memo.machine_memo_prefixes = (0..=crate::constants::MACHINE_MEMO_PREFIX_MAX_COUNT)
            .map(|i| MachineMemoPrefix::new(vec![0xFF, i as u8]).expect("prefix"))
            .collect();
        assert!(
            matches!(
                memo.validate(),
                Err(WalletError::MachineMemoScopeInvalid { .. })
            ),
            "the machine-memo scope rule"
        );

        let server = || {
            SyncServer::new(
                SyncServerId::new("dup").expect("id"),
                "Dup",
                LightServerEndpoint::new("https://zec.rocks:443").expect("tls"),
                None,
            )
            .expect("server")
        };
        let mut servers = base();
        servers.sync_servers = vec![server(), server()];
        assert!(
            servers.validate().is_err(),
            "the offered-servers rule (ids unique)"
        );

        let mut transport = base();
        transport.endpoint = LightServerEndpoint::new("http://127.0.0.1:9067").expect("loopback");
        transport.tor = TorPolicy::Required {
            runtime: TorRuntime::Dialer(Arc::new(StubDialer) as Arc<dyn NetDialer>),
        };
        assert!(
            matches!(
                transport.validate(),
                Err(WalletError::InvalidEndpoint { reason }) if reason.contains("TorPolicy::Off")
            ),
            "the plaintext-transport rule"
        );

        let mut jitter = base();
        jitter.broadcast_jitter = JitterPolicy::Uniform {
            max_ms: BROADCAST_JITTER_MAX_MS_CEILING + 1,
        };
        assert!(
            matches!(
                jitter.validate(),
                Err(WalletError::BroadcastJitterTooLong { .. })
            ),
            "the jitter ceiling"
        );
    }

    #[test]
    fn an_over_long_url_is_refused_at_the_one_validator() {
        // P3-13 (the security review's MEDIUM): the bound lives HERE, so the
        // core, the bridge and the aux row's read-back all agree on what an
        // endpoint can be — a URL the row could not read back cannot exist.
        let host = "h".repeat(SYNC_SERVER_URL_MAX_BYTES);
        let over = format!("https://{host}.example:443");
        assert!(over.len() > SYNC_SERVER_URL_MAX_BYTES);
        assert!(matches!(
            LightServerEndpoint::new(over),
            Err(WalletError::InvalidEndpoint { reason }) if reason.contains("SYNC_SERVER_URL_MAX_BYTES")
        ));
        // At the bound exactly: the parser decides, not the cap.
        let at = format!(
            "https://{}.example",
            "h".repeat(SYNC_SERVER_URL_MAX_BYTES - 16)
        );
        assert!(at.len() <= SYNC_SERVER_URL_MAX_BYTES);
        assert!(LightServerEndpoint::new(at).is_ok());
    }

    #[test]
    fn same_server_ignores_the_default_port_and_host_case() {
        let a = LightServerEndpoint::new("https://zec.rocks").expect("a");
        let b = LightServerEndpoint::new("https://ZEC.rocks:443").expect("b");
        let c = LightServerEndpoint::new("https://zec.rocks:8443").expect("c");
        let d = LightServerEndpoint::new("https://lightwalletd.example.com:443").expect("d");
        assert!(
            a.same_server(&b),
            "the default port and the host's case are one server"
        );
        assert!(!a.same_server(&c), "a different port is a different server");
        assert!(!a.same_server(&d));
        assert!(a.same_server(&a));
        let lo = LightServerEndpoint::new("http://localhost").expect("lo");
        let lo80 = LightServerEndpoint::new("http://localhost:80").expect("lo80");
        assert!(lo.same_server(&lo80), "http's default port is 80");
        // Boundaries (the code review's MINOR) — the risky direction is
        // "same" for two servers that differ, which skips a needed rebuild:
        // a bracketed IPv6 literal keeps its brackets on both sides; a
        // trailing-dot host is NOT folded onto the bare one (DNS treats them
        // alike, but the endpoint the wallet dials is the string it holds).
        let v6 = LightServerEndpoint::new("https://[::1]").expect("v6");
        let v6_443 = LightServerEndpoint::new("https://[::1]:443").expect("v6_443");
        assert!(
            v6.same_server(&v6_443),
            "an IPv6 literal with the default port"
        );
        let dotted = LightServerEndpoint::new("https://zec.rocks.").expect("dotted");
        assert!(
            !a.same_server(&dotted),
            "a trailing-dot host is a different server (conservative)"
        );
    }

    #[test]
    fn endpoint_accepts_https_and_loopback_http_only() {
        assert!(LightServerEndpoint::new("https://zec.rocks:443").is_ok());
        assert!(LightServerEndpoint::new("https://mainnet.lightwalletd.com:9067").is_ok());
        assert!(LightServerEndpoint::new("http://localhost:9067").is_ok());
        assert!(LightServerEndpoint::new("http://127.0.0.1:9067").is_ok());
        // plaintext to a remote host is a leak, not a configuration
        assert!(matches!(
            LightServerEndpoint::new("http://example.com:9067"),
            Err(WalletError::InvalidEndpoint { .. })
        ));
        assert!(LightServerEndpoint::new("grpc://example.com").is_err());
        assert!(LightServerEndpoint::new("not a url").is_err());
    }

    #[test]
    fn endpoint_loopback_is_semantic_not_a_string_prefix() {
        // W2 security fold: "127.<domain>" shapes must NOT count as loopback
        // (they resolve wherever the attacker likes — plaintext to a remote).
        for host in [
            "http://127.evil.com:9067",
            "http://127.0.0.1.evil.com:9067",
            "http://localhost.evil.com:9067",
        ] {
            assert!(
                matches!(
                    LightServerEndpoint::new(host),
                    Err(WalletError::InvalidEndpoint {
                        reason: "http only allowed for loopback"
                    })
                ),
                "{host} must not pass as loopback"
            );
        }
        // real loopback literals still work, including IPv6 and short forms
        assert!(LightServerEndpoint::new("http://[::1]:9067").is_ok());
        assert!(LightServerEndpoint::new("http://127.0.0.1:9067").is_ok());
    }

    #[test]
    fn endpoint_rejects_userinfo_and_paths() {
        // Basic-auth userinfo is never-log material — refused at the door.
        assert!(matches!(
            LightServerEndpoint::new("https://user:pw@example.com:443"),
            Err(WalletError::InvalidEndpoint {
                reason: "userinfo not allowed"
            })
        ));
        assert!(matches!(
            LightServerEndpoint::new("https://example.com/api"),
            Err(WalletError::InvalidEndpoint {
                reason: "path not allowed"
            })
        ));
    }

    #[test]
    fn endpoint_auth_refuses_a_value_that_could_inject_a_header() {
        // §8 SECURITY (rust-patterns "injection in any emitted wire format"). A
        // key carrying CR/LF ends the header and starts another one — a host
        // reading its key from a file or a `--dart-define` is one invisible
        // trailing newline away from this, so it is refused at construction and
        // not at first dial. NUL is refused for the same reason.
        for bad in [
            "key\r\nx-injected: 1",
            "key\nx-injected: 1",
            "key\r",
            "key\0",
        ] {
            assert!(
                matches!(
                    EndpointAuth::new("x-zcash-rpc-key", bad),
                    Err(WalletError::InvalidEndpointAuth { .. })
                ),
                "a value carrying a control character must be refused: {bad:?}"
            );
        }
        // The ordinary case still works, or the check above proves nothing.
        assert!(EndpointAuth::new("x-zcash-rpc-key", "abc123").is_ok());
    }

    #[test]
    fn endpoint_auth_refuses_an_empty_value_and_a_bad_header_name() {
        assert!(matches!(
            EndpointAuth::new("x-zcash-rpc-key", ""),
            Err(WalletError::InvalidEndpointAuth {
                reason: "auth header value is empty"
            })
        ));
        // A space is not legal in a header name; so is the empty name.
        assert!(EndpointAuth::new("bad header", "v").is_err());
        assert!(EndpointAuth::new("", "v").is_err());
    }

    #[test]
    fn endpoint_auth_refuses_names_the_transport_cannot_actually_send() {
        // review fix, and it was caught by its own red-first: a VALID HTTP
        // header name is not automatically a valid gRPC ASCII metadata key.
        // tonic keys on the suffix — `Ascii::is_valid_key(k) == !k.ends_with("-bin")`
        // — so `x-key-bin` passed this door and then made the interceptor refuse
        // EVERY request with `Unauthenticated`. The wallet could not reach the
        // endpoint at all, and the cause showed up on the network rather than at
        // config time, which is the exact failure this constructor exists to
        // prevent.
        assert!(
            matches!(
                EndpointAuth::new("x-key-bin", "abc123"),
                Err(WalletError::InvalidEndpointAuth { .. })
            ),
            "a -bin metadata key cannot carry this value and must be refused at the door"
        );
        // Not a blanket ban on the substring: only the suffix is special.
        assert!(EndpointAuth::new("x-bin-key", "abc123").is_ok());

        // WIDENED by the security review: these four are STRIPPED by hyper
        // from every outbound h2 request (`CONNECTION_HEADERS`,
        // hyper-1.10.1/src/proto/h2/mod.rs:36-41), so naming one means the
        // credential silently never reaches the wire — worse than the reserved
        // names above, which fail loudly. `connection`/`content-length`/`host`
        // are h2-illegal or transport-set.
        for stripped in [
            "keep-alive",
            "proxy-connection",
            "transfer-encoding",
            "upgrade",
            "connection",
            "content-length",
            "host",
            "grpc-message-type",
        ] {
            assert!(
                matches!(
                    EndpointAuth::new(stripped, "abc123"),
                    Err(WalletError::InvalidEndpointAuth { .. })
                ),
                "{stripped} never reaches the wire and must be refused at the door"
            );
        }
        // Case does not bypass it — `HeaderName` lowercases on construction.
        assert!(EndpointAuth::new("Content-Type", "abc123").is_err());
        assert!(EndpointAuth::new("X-KEY-BIN", "abc123").is_err());

        // Headers the transport owns: `insert` REPLACES, so naming one of these
        // overwrites gRPC framing instead of adding a credential.
        for owned in ["content-type", "te", "user-agent", "grpc-timeout"] {
            assert!(
                matches!(
                    EndpointAuth::new(owned, "abc123"),
                    Err(WalletError::InvalidEndpointAuth { .. })
                ),
                "{owned} is owned by the transport and must be refused"
            );
        }
    }

    #[test]
    fn endpoint_auth_refuses_any_grpc_prefix_and_an_oversized_header() {
        // ADR-0568: a user now types the header name. gRPC reserves the WHOLE
        // `grpc-` prefix, not only the names TRANSPORT_OWNED lists.
        for reserved in ["grpc-previous-rpc-attempts", "grpc-anything", "GRPC-Key"] {
            assert!(
                matches!(
                    EndpointAuth::new(reserved, "abc123"),
                    Err(WalletError::InvalidEndpointAuth {
                        reason: "auth header name must not start with grpc- (gRPC reserves that prefix)"
                    })
                ),
                "{reserved} is in gRPC's reserved prefix and must be refused"
            );
        }
        // The prefix, not the substring.
        assert!(EndpointAuth::new("x-grpc-key", "abc123").is_ok());
        // The NAME bound, asserted at the constant (gate 7).
        let at_cap = "x".repeat(SYNC_SERVER_AUTH_HEADER_MAX_BYTES);
        assert!(EndpointAuth::new(&at_cap, "abc123").is_ok());
        let over = "x".repeat(SYNC_SERVER_AUTH_HEADER_MAX_BYTES + 1);
        assert!(matches!(
            EndpointAuth::new(&over, "abc123"),
            Err(WalletError::InvalidEndpointAuth {
                reason: "auth header name is longer than SYNC_SERVER_AUTH_HEADER_MAX_BYTES"
            })
        ));
    }

    #[test]
    fn endpoint_auth_equality_is_by_header_and_value() {
        // ADR-0568: equality decides whether a switch onto the server in use
        // keeps the session or rebuilds it with the new key.
        let a = EndpointAuth::new("x-api-key", "value-one").expect("auth");
        assert_eq!(
            a,
            EndpointAuth::new("X-Api-Key", "value-one").expect("auth")
        );
        assert_ne!(
            a,
            EndpointAuth::new("x-api-key", "value-two").expect("auth")
        );
        assert_ne!(a, EndpointAuth::new("x-other", "value-one").expect("auth"));
        // A prefix of the value is not the value.
        assert_ne!(a, EndpointAuth::new("x-api-key", "value-on").expect("auth"));
    }

    #[test]
    fn endpoint_auth_refuses_surrounding_whitespace_rather_than_trimming_it() {
        // A trailing newline or space survives `HeaderValue`, goes out on the
        // wire, and earns a 403 whose cause is invisible in every log because the
        // value is redacted everywhere. Refused, not silently trimmed — trimming
        // a secret the host meant to send is its own surprise.
        for bad in [" abc123", "abc123 ", "\tabc123", "abc123\t"] {
            assert!(
                matches!(
                    EndpointAuth::new("x-zcash-rpc-key", bad),
                    Err(WalletError::InvalidEndpointAuth {
                        reason: "auth header value has leading or trailing whitespace"
                    })
                ),
                "surrounding whitespace must be refused: {bad:?}"
            );
        }
        // An interior space is not our business to police — some schemes use it.
        assert!(EndpointAuth::new("authorization", "Bearer abc123").is_ok());
    }

    #[test]
    fn endpoint_auth_value_must_be_printable_ascii_and_bounded() {
        // `HeaderValue::from_str` is NOT a gRPC-ASCII
        // check. `http`'s predicate is `b >= 32 && b != 127 || b == b'\t'`, so
        // HTAB and every byte in 0x80..=0xFF pass it, while gRPC's ASCII-Value is
        // 1*(%x20-%x7E). This was the exact mirror of the `-bin` name-side gap,
        // in the same constructor — a key with a typographic quote or any UTF-8
        // was blessed here and then failed every request opaquely.
        for bad in ["a\u{a0}b", "a\u{80}b", "a\tb", "clé-secrète", "key\u{7f}"] {
            assert!(
                matches!(
                    EndpointAuth::new("x-zcash-rpc-key", bad),
                    Err(WalletError::InvalidEndpointAuth { .. })
                ),
                "a non-printable-ASCII value must be refused: {bad:?}"
            );
        }
        // Printable ASCII across the whole legal range still passes, so the
        // guard is not quietly narrower than the spec it cites.
        let legal: String = (0x20u8..=0x7E).map(|b| b as char).collect();
        assert!(
            EndpointAuth::new("x-zcash-rpc-key", legal.trim()).is_ok(),
            "every printable-ASCII byte is legal in a gRPC ASCII metadata value"
        );

        // The length bound, asserted AT the constant (gate 7).
        let at_cap = "k".repeat(ENDPOINT_AUTH_VALUE_MAX_BYTES);
        assert!(EndpointAuth::new("x-zcash-rpc-key", at_cap).is_ok());
        let over = "k".repeat(ENDPOINT_AUTH_VALUE_MAX_BYTES + 1);
        assert!(matches!(
            EndpointAuth::new("x-zcash-rpc-key", over),
            Err(WalletError::InvalidEndpointAuth {
                reason: "auth header value is longer than ENDPOINT_AUTH_VALUE_MAX_BYTES"
            })
        ));
    }

    #[test]
    fn endpoint_auth_debug_never_prints_the_secret() {
        // §5.4 NEVER-log, asserted rather than asserted-in-a-comment. `Debug` is
        // the leak that actually happens: something formats a config into a log
        // line and the credential rides along. The header NAME is not secret and
        // is kept, because a redacted name makes a misconfiguration unreadable.
        let auth = EndpointAuth::new("x-zcash-rpc-key", "s3cret-value-do-not-log")
            .expect("valid auth pair");
        let rendered = format!("{auth:?}");
        assert!(
            !rendered.contains("s3cret-value-do-not-log"),
            "EndpointAuth's Debug leaked the secret: {rendered}"
        );
        assert!(
            rendered.contains("x-zcash-rpc-key"),
            "the header name should survive redaction, for diagnosability: {rendered}"
        );
        assert!(
            rendered.contains("redacted"),
            "and it should say so: {rendered}"
        );
    }

    #[test]
    fn jitter_none_is_zero_and_uniform_respects_the_inclusive_bound() {
        // §8 (§5.3, gate 7 — the constant at its boundary): None ⇒ exactly zero (the honest
        // opt-out); Uniform { 0 } collapses to zero (span 1); Uniform { max_ms } ⇒ a delay in
        // the INCLUSIVE 0..=max_ms over MANY draws, never beyond. Probabilistic but bounded —
        // the assertion is on the RANGE, not any one draw.
        use std::time::Duration;
        assert_eq!(JitterPolicy::None.sample_delay(), Duration::ZERO);
        assert_eq!(
            JitterPolicy::Uniform { max_ms: 0 }.sample_delay(),
            Duration::ZERO,
            "Uniform {{ 0 }} is the no-delay edge, matching None",
        );
        let policy = JitterPolicy::Uniform { max_ms: 50 };
        for _ in 0..1000 {
            assert!(
                policy.sample_delay() <= Duration::from_millis(50),
                "never beyond the inclusive ceiling",
            );
        }
        // The default IS the §5.3 10 s window (the named constant, not a literal).
        assert!(matches!(
            JitterPolicy::default(),
            JitterPolicy::Uniform { max_ms } if max_ms == BROADCAST_JITTER_MAX_MS_DEFAULT
        ));
    }
}
