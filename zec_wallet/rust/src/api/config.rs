//! Wallet configuration as Dart supplies it (spec §2.3). Every field is
//! REQUIRED — the endpoint (the SDK never picks a server silently) and the
//! Tor policy (a conscious privacy decision) must be stated by the host.
//! Validation happens Rust-side when the config crosses (typed
//! [crate::api::error::WalletApiError] on rejection), not in Dart.

/// Consensus network. A runtime parameter — one binary serves both networks.
#[derive(Clone)]
pub enum Network {
    Main,
    Test,
}

/// Wallet configuration.
///
/// `Clone` is derived so a host-seed provisioning flow (FR-15) can build the
/// validated core config TWICE from one DTO (create then reopen-with-seed-port —
/// the core `WalletConfig` is not `Clone` because of the optional injected dialer).
/// It is a plain value DTO; the derive is additive and FRB-irrelevant.
#[derive(Clone)]
pub struct WalletConfig {
    /// Host-owned data directory (platform app dir). MUST be an ABSOLUTE
    /// path — empty/relative is rejected typed (`InvalidDbDir`, RW-CFG-003)
    /// at create/open/validate, because a relative dir would resolve against
    /// the process working directory and strand the wallet (worst on
    /// desktop, where the cwd changes between launches). The host owns
    /// backup-exclusion for this dir (iOS `isExcludedFromBackup`, Android
    /// `no_backup/`).
    pub db_dir: String,
    pub network: Network,
    /// lightwalletd-protocol (Zaino-compatible) gRPC endpoint URL.
    /// `https://host[:port]` anywhere; `http://` only for loopback (local
    /// development) — plaintext to a remote host is a leak, not a
    /// configuration. URLs with userinfo (`user:pass@`) are rejected.
    pub endpoint_url: String,
    /// OPTIONAL name of an auth header sent on EVERY light-server request, for
    /// an endpoint behind a proxy that gates access. `null` (the common case)
    /// sends no auth header. Must be paired with [`Self::endpoint_auth_value`];
    /// supplying one without the other is a typed refusal at construction, not
    /// a silently unauthenticated wallet.
    pub endpoint_auth_header: Option<String>,
    /// OPTIONAL value for [`Self::endpoint_auth_header`]. A §5.4 NEVER-log
    /// value: wrapped `Zeroizing` in Rust at the boundary, marked sensitive on
    /// the wire so it is never HPACK-indexed, and never logged. Dart memory
    /// cannot be zeroized — the documented §10 exposure, the same framing as the
    /// swap JWT; keep it minimal-residency host-side and prefer
    /// `--dart-define` over anything that lands in a file.
    ///
    /// **What this buys, so it is not over-trusted:** a key shipped in a
    /// distributed app is extractable from the binary. This is abuse control —
    /// it gives the proxy a handle for per-key rate-limits — not access control
    /// against a determined party.
    ///
    /// **USE ONE KEY FOR EVERY INSTALL.** A per-user or per-install key defeats
    /// the SDK's broadcast unlinkability: it mints a fresh Tor circuit per send
    /// so the endpoint cannot tie a broadcast to the syncing wallet, and a
    /// distinct constant header re-joins them on the first request. One shared
    /// key is a rate-limit handle; a per-user key is a tracking identifier.
    pub endpoint_auth_value: Option<String>,
    /// REQUIRED: what must happen to wallet traffic.
    pub tor: TorPolicy,
    /// Where the seed lives between launches.
    pub seed_persistence: SeedPersistence,
    /// Restore-from height; `null` on create (= current tip). Above-tip is
    /// rejected typed at first sync; below-activation is clamped.
    pub birthday_height: Option<u32>,
    /// Broadcast-timing decorrelation (delays tx broadcast by a random
    /// in-window amount so sends don't timestamp-correlate with other
    /// host-app traffic).
    pub broadcast_jitter: JitterPolicy,
    /// FR-27 — the OPT-IN machine-memo read scope: the leading bytes an
    /// on-chain `0xFF` memo must carry for [WalletHandle.machineMemos] to
    /// return it. EMPTY by default, and empty means that verb stays CLOSED
    /// (it refuses typed rather than answering with an empty list).
    ///
    /// **A prefix bounds VOLUME, not intent.** Forging one costs an attacker
    /// nothing, so this keeps unrelated envelopes out of your parser — it is
    /// not evidence that what arrives is yours. And it does NOT mean "written
    /// by this device": the memos worth reading were written by whoever paid
    /// you.
    ///
    /// Each prefix must be non-empty and at most 32 bytes; at most 8 may be
    /// registered. A scope that could never work is refused at create/open
    /// ([WalletErrorKind.machineMemoScopeInvalid]) rather than silently
    /// reading nothing forever.
    pub machine_memo_prefixes: Vec<Vec<u8>>,
    /// The sync servers this host OFFERS the user (the picker, P3-13). `null`
    /// or empty = none offered: the picker shows `endpointUrl` and a custom
    /// entry only. `endpointUrl` is the DEFAULT — a wallet with no persisted
    /// choice dials it, and a choice that can no longer be honoured falls back
    /// to it with the reason on [WalletHandle.syncServerStatus]. A persisted
    /// choice WINS at open. Validated like the endpoint (bounded list, unique
    /// ids, every URL through the same door). `referenceSyncServers` gives the
    /// public reference servers; a host appends its OWN servers — gated ones
    /// with `authHeader`/`authValue` — to that list (ADR-0568).
    pub sync_servers: Option<Vec<SyncServer>>,
}

/// A sync server a host OFFERS (the picker, P3-13).
#[derive(Clone)]
pub struct SyncServer {
    /// Stable, host-chosen id — what the wallet remembers. Lowercase ascii
    /// letters, digits, `-`; at most 32 bytes. Never shown; the label is.
    pub id: String,
    /// Display name. Printable, at most 64 bytes.
    pub label: String,
    /// `https://host[:port]` (loopback `http://` for development), like
    /// `endpointUrl`.
    pub url: String,
    /// The auth header a GATED entry sends on every request, e.g.
    /// `x-api-key`. `null` for a public server. On the list a HANDLE
    /// returns, this says whether the entry is gated; the value is never
    /// returned.
    pub auth_header: Option<String>,
    /// The header's value — the same §5.4 framing as `endpointAuthValue`:
    /// never logged, wrapped `Zeroizing` in Rust, ONE key for every install
    /// (a rate-limit handle, never an authorization boundary), prefer a
    /// `--dart-define` over anything that lands in a file. Both-or-neither
    /// with `authHeader`: one without the other is refused at the door.
    /// NEVER persisted by the wallet: the choice row stores the id. On the
    /// list a HANDLE returns it is always `null`.
    pub auth_value: Option<String>,
}

/// A key the USER gives for their own custom server (ADR-0568): the header
/// name their server expects and its value. A plain struct ON PURPOSE — the
/// generated Dart class has no `toString` that renders fields (a freezed
/// enum variant's would print the value), so `'$choice'` never shows the key.
///
/// On input, the header is at most 64 bytes, must be a valid header name and
/// not one the transport owns or `grpc-…`; the value is at most 4096 bytes of
/// printable ASCII with no surrounding whitespace. A refusal is
/// `WalletErrorKind.invalidEndpointAuth`. A key is refused on an `http://`
/// server. The wallet STORES it (encrypted, with the choice) so the server
/// keeps working across launches, erases it when another server is chosen,
/// and never logs it. On a value the SDK RETURNS (a status), `value` is
/// ALWAYS the empty string — the header says a key is saved; the key itself
/// never leaves Rust — so a returned keyed choice cannot be passed back in.
/// The user's key identifies them to that server (the picker says so).
pub struct SyncServerKey {
    pub header: String,
    pub value: String,
}

/// What the user asked the picker for.
pub enum SyncServerChoice {
    /// One of the offered entries, by id.
    Predefined { id: String },
    /// A URL the user typed, and the key they gave for it (`null` = none).
    /// The URL is validated at the door like `endpointUrl`, and bounded
    /// (2048 bytes) before the validator sees it.
    Custom {
        url: String,
        key: Option<SyncServerKey>,
    },
    /// Clear the choice: back to `endpointUrl`.
    Default,
    /// Forward-compatibility arm (never accepted as input).
    Unknown,
}

/// Why a persisted choice is NOT the server in use. Never silent: the
/// picker renders it.
pub enum SyncServerFallback {
    /// The remembered id is one this app no longer offers; `id` names it.
    ChoiceNotOffered { id: String },
    /// The remembered choice could not be read (a malformed row). The wallet
    /// is usable on the default; pick again.
    ChoiceUnreadable,
    /// The remembered custom server uses an unencrypted (`http://`) address,
    /// which the wallet's private path (a host-registered dialer) cannot
    /// carry: the default is in use, the choice is kept (FR-29 E12). Pick an
    /// `https://` server, or run the wallet with `TorPolicy.off` to use it.
    ChoiceRefusedByTransport,
    /// Forward-compatibility arm.
    Unknown,
}

/// Which server the wallet dials, which choice produced it, and whether a
/// fallback is in force ([WalletHandle.syncServerStatus]). A URL that names
/// an offered server is spelled as that entry's `url`, so comparing strings
/// against `syncServers()` finds its row (`https://zec.rocks` and
/// `https://zec.rocks:443` are one server).
pub struct SyncServerStatus {
    /// The endpoint every request goes to right now. Render its HOST, never
    /// the whole URL.
    pub effective_url: String,
    /// `endpointUrl` — "App default" when it is not among the offered entries.
    pub default_url: String,
    /// The persisted choice; `null` = no choice ever made (the default by
    /// absence, not by fallback).
    pub choice: Option<SyncServerChoice>,
    /// Non-null = the choice could not be honoured; `effectiveUrl` is then
    /// `defaultUrl`.
    pub fallback: Option<SyncServerFallback>,
}

/// The reachability probe's answer ([WalletHandle.probeSyncServer]): the
/// server answered under the wallet's own Tor policy and claims this wallet's
/// network. A fact about reachability and identity, never about honesty —
/// the sync guards judge that pass by pass.
pub struct SyncServerProbe {
    /// The server's claimed tip. Untrusted; a bare height.
    pub tip: u32,
}

/// The reference sync servers for `network` — PUBLIC servers only, with no
/// key: on mainnet `zec.rocks`, on testnet the `zec.rocks` testnet rail.
/// To offer servers of your own (gated or not), append them and pass the list
/// as `WalletConfig.syncServers`:
/// `[...referenceSyncServers(network: n), SyncServer(id: 'mine', label: 'My
/// server', url: 'https://…', authHeader: 'x-api-key', authValue: key)]`,
/// with `key` from a `--dart-define` — never a committed file.
#[flutter_rust_bridge::frb(sync)]
pub fn reference_sync_servers(
    network: Network,
) -> Result<Vec<SyncServer>, crate::api::error::WalletApiError> {
    crate::convert::reference_sync_servers(network)
}

/// What MUST happen to wallet traffic.
///
/// HONEST TRUST BOUNDARY: with an external runtime the SDK can verify it
/// handed bytes to the proxy — it cannot attest the host's infrastructure
/// actually routes through Tor.
#[derive(Clone)]
pub enum TorPolicy {
    /// Direct connections.
    Off,
    /// Tor when reachable; falls back to clearnet WITH a visible
    /// `TorState.fellBack` — degradation is never silent.
    Preferred { runtime: TorRuntimeConfig },
    /// Fail-closed: runtime unreachable ⇒ ZERO packets, sync `Stalled`.
    /// Covers swap API calls too — there is no independent clearnet path.
    Required { runtime: TorRuntimeConfig },
}

/// Whose transport carries the traffic.
///
/// `externalSocks5` is REFUSED at the config door (T0-6; the loopback-SOCKS5
/// shape is out for good — ADR-0543): `validateWalletConfig` answers
/// `InvalidEndpoint` / `RW-CFG-001` before anything opens, with a reason
/// naming the alternative. `hostDialer` selects the dialer the host's NATIVE
/// library registered through the C contract
/// (`include/zec_wallet_net_dialer.h`, FR-29): the host registers ONE
/// trampoline at trusted init, BEFORE it validates this config, and swaps its
/// backing transport (Tor, Shadowsocks, VLESS, direct) behind it. A
/// `hostDialer` policy validated while nothing is registered is refused the
/// same way (`RW-CFG-001`, "no host dialer registered") — never a first-dial
/// failure, never a clearnet fallback. A Rust host can still inject a dialer
/// directly (`TorRuntime::Dialer`, ADR-0526); that one is not expressible
/// here.
#[derive(Clone)]
pub enum TorRuntimeConfig {
    /// Host-side Tor speaking SOCKS5 (Orbot, system Tor, a sidecar), e.g.
    /// `"127.0.0.1:9050"`. REFUSED at the config door (T0-6, ADR-0543) —
    /// use [`TorRuntimeConfig::HostDialer`].
    ExternalSocks5 { addr: String },
    /// The dialer the host's native library registered (FR-29, the C contract
    /// `include/zec_wallet_net_dialer.h`). No payload: the registration lives
    /// in the wallet library's process-global registry, not in this value.
    /// `TorState.active(runtime: hostDialer(name, isolation, exposure))` then
    /// reports what the host declared: its OWN name for the transport
    /// (ADR-0547 — the SDK has no list of kinds), whether the path keeps the
    /// wallet's connections apart, and whether it hides the device's address.
    HostDialer,
}

/// Where the seed lives between launches.
#[derive(Clone)]
pub enum SeedPersistence {
    /// The SDK seals seed + mnemonic under a keychain-held key (XChaCha20-
    /// Poly1305, versioned envelope). Default for Dart-path consumers:
    /// enables `revealMnemonic` after restart. At-rest honesty: protection =
    /// the OS keychain ACL + device lock — there is no passphrase KDF.
    SealedKeychain,
    /// Nothing persisted: every launch must re-supply the seed. The host
    /// owns the recovery story.
    None,
}

/// Broadcast-timing decorrelation policy.
#[derive(Clone)]
pub enum JitterPolicy {
    /// No delay — an honest opt-out.
    None,
    /// Uniform random delay in `0..=maxMs` before each tx broadcast. The
    /// documented default window is 0..=10 s. `maxMs` above 30 000 (30 s) is
    /// refused with `broadcastJitterTooLong` by create, open and
    /// `validateWalletConfig` alike, never clamped. (`u32`, so Dart gets a
    /// plain `int`.)
    Uniform { max_ms: u32 },
}

/// Validate a [WalletConfig] without opening anything — for settings-form
/// UX. Runs the SAME Rust-side validation that `create`/`open` will run
/// (endpoint rules, bounds); throws the same typed
/// [crate::api::error::WalletApiError].
#[flutter_rust_bridge::frb(sync)]
pub fn validate_wallet_config(
    config: WalletConfig,
) -> Result<(), crate::api::error::WalletApiError> {
    // conversion + validation live in crate::convert — api/ files never
    // reference the core crate (the rule in api/mod.rs)
    crate::convert::validate_wallet_config(config)
}
