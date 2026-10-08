//! Typed error surface (spec §6.1): stable, subsystem-prefixed codes — hosts
//! catch by TYPE / code, never by message text (FFI rule). Every
//! variant is logged by the layer that produces it; none are silent.
//!
//! Logging discipline: `Display` messages are STATIC and payload-free for any
//! field on the §5.4 never-log list (amounts, addresses, txids) — log the
//! `code()`, render the typed payload host-side. `Debug` carries payloads for
//! tests and must never reach a log line (enforced later by the §5.4 tracing
//! capture-layer test).

use thiserror::Error;

use crate::lifecycle::LifecyclePhase;
use crate::state::StallReason;

/// Wallet error taxonomy (spec §6.1). `#[non_exhaustive]` per the G2
/// extensibility policy: hosts keep a default arm; an SDK upgrade may add
/// variants without breaking compiled hosts.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WalletError {
    // ── Seed / mnemonic (RW-SEED-*) ─────────────────────────────────────────
    /// Seed bytes outside 32..=252 (§2.2) — rejected typed BEFORE the
    /// verified upstream panic can be reached.
    #[error("seed length out of bounds")]
    InvalidSeedLength { len: usize },
    /// BIP39 checksum/word failure. Index only — NEVER the word value
    /// (identity-core precedent).
    #[error("invalid mnemonic")]
    InvalidMnemonic { word_index: Option<u32> },
    /// `reveal_mnemonic` on a `RawBytes`-created wallet (the host-supplied-seed path) —
    /// structurally unsupported; recovery is the host's own story (§3.1).
    #[error("wallet has no mnemonic")]
    NoMnemonic,
    /// A Rust-side operation needs the seed re-supplied (§4.2 — the
    /// upstream-proven try-without-seed contract under `SeedPersistence::None`);
    /// also the typed reject for a `SeedSource::Generate` +
    /// `SeedPersistence::None` create (FR-24 RH2 — a seed that would exist
    /// nowhere), refused before any store write.
    #[error("seed required")]
    SeedRequired,
    /// Supplied seed does not match the wallet's existing account. Two paths:
    /// (1) a supplied `SeedSource` mismatches the provisioning remnant's stored
    /// fingerprint (§6.3 — the mistyped-mnemonic repair re-run); (2) FR-12 B1 — a
    /// host-custodied seed (`SeedPersistence::None`, via [`WalletSeedPort`]) whose
    /// derived UFVK does not control the stored account, rejected BEFORE signing so a
    /// wrong seed can never produce a transaction.
    ///
    /// [`WalletSeedPort`]: crate::seed::WalletSeedPort
    #[error("seed does not match existing wallet")]
    SeedMismatch,
    /// Upstream key derivation failed — unreachable for a bounds-valid seed,
    /// but a panic must never be the error path (§4.2).
    #[error("key derivation failed")]
    KeyDerivation,

    // ── Watch-only (RW-VIEW-*, #397 spec §3.7) ──────────────────────────────
    /// A spend-class operation (send/shield/swap/reveal/backup — the §3.7 D3
    /// matrix) on a WATCH-ONLY wallet. STRUCTURAL, not retryable — no seed
    /// will ever arrive, so this is deliberately NOT `SeedRequired` (which
    /// promises a retry-with-seed). Hosts render an explanation surface
    /// ("this wallet holds no spending keys"), never a retry affordance.
    #[error("watch-only wallet cannot perform this operation")]
    WatchOnly,
    /// A supplied UFVK string failed to decode for the wallet's network
    /// (garbage, tampered/truncated, or a wrong-network `uview…` — the
    /// ZIP-316 container is network-bound, so cross-net import fails HERE,
    /// §3.7 D2). The value is NEVER echoed (§5.4).
    #[error("viewing key is not valid for this network")]
    InvalidViewingKey,

    // ── Config (RW-CFG-*) ───────────────────────────────────────────────────
    /// Endpoint URL rejected (scheme/host/port shape — §2.3: https, or
    /// loopback for development).
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: &'static str },
    /// An endpoint auth header or value rejected at the door
    /// (`EndpointAuth::new`) — a host's key or a user's custom-server key
    /// (ADR-0568), or a key offered for a plaintext custom URL. Its own variant
    /// so a picker can tell a bad key from a bad URL by KIND, never by text.
    /// `reason` is a static string: the value is NEVER echoed (§5.4).
    #[error("invalid endpoint auth: {reason}")]
    InvalidEndpointAuth { reason: &'static str },
    /// Restore birthday above the endpoint's tip (§2.3): a too-HIGH birthday
    /// silently never finds the user's notes — surfaced, never scanned past.
    #[error("birthday is in the future")]
    BirthdayInFuture,
    /// Wallet data directory rejected at the config door (empty or relative).
    /// An unvalidated relative dir would `create_dir_all` against the process
    /// CWD (lifecycle/store): mobile fails closed on an unwritable cwd, but
    /// DESKTOP silently plants the wallet under the launch directory — the
    /// next launch from another cwd reads "no wallet here" and onboarding
    /// offers a fresh CREATE over a funded wallet sitting elsewhere.
    #[error("invalid db_dir: {reason}")]
    InvalidDbDir { reason: &'static str },
    /// The broadcast jitter window is above
    /// [`BROADCAST_JITTER_MAX_MS_CEILING`](crate::constants::BROADCAST_JITTER_MAX_MS_CEILING)
    /// (the 2026-10-05 review, F01): a longer wait before a broadcast can outlast
    /// a transaction's validity. A host configuration bug, refused at create/open
    /// rather than clamped, so the host learns of it.
    #[error("broadcast jitter {max_ms} ms is above the {ceiling_ms} ms ceiling")]
    BroadcastJitterTooLong { max_ms: u64, ceiling_ms: u64 },

    // ── Money / memo / payment construction (RW-PAY-*) ─────────────────────
    /// Amount negative or beyond max supply (§2.1 constructor bound).
    /// Payload-free on purpose: amounts are on the §5.4 never-log list.
    #[error("amount out of range")]
    AmountOutOfRange,
    /// Memo exceeds its ZIP-302 bound (§2.4).
    #[error("memo too long")]
    MemoTooLong { len: usize, max: usize },
    /// ZIP-302 reserved-range memos are receivable but never constructible
    /// for send (§2.4 — we never emit reserved framing).
    #[error("reserved memo range is not sendable")]
    ReservedMemoNotSendable,
    /// Memo attached to a transparent recipient (§6.1 — protocol can't carry
    /// it; honest construction-time rejection per ADR-0005 r3).
    #[error("memo requires a shielded recipient")]
    MemoRequiresShieldedRecipient,
    /// Address failed to parse for the configured network (oversized,
    /// malformed, unknown HRP — §4.6; the value is NEVER echoed back).
    #[error("address invalid")]
    AddressInvalid,
    /// Memo bytes failed ZIP-302 classification (a text-lead memo whose
    /// payload is not valid UTF-8 — the one invalid shape upstream defines).
    /// Payload-free: memo bytes are CONTENT (§5.4 never-log).
    #[error("memo invalid")]
    MemoInvalid,
    /// A payment leg was given BOTH a text memo and machine bytes (FR-28).
    /// Distinct from [`Self::MemoInvalid`] on purpose: neither memo is
    /// individually invalid, and this is a CALLER bug, not corrupt data. It
    /// rode `MemoInvalid` until an earlier revision, which reported a host programming error
    /// to the END USER as their memo being unsendable — wrong for both
    /// audiences, and it made RW-PAY-006 mean two unrelated things.
    #[error("one memo per payment: text and machine bytes were both set")]
    MemoConflict,
    /// Zero-valued output to a transparent-only recipient — disallowed by
    /// consensus (mirrors the audited zip321 construction rule; §2.4).
    #[error("zero-valued transparent output")]
    ZeroValuedTransparentOutput,
    /// ZIP-321 payment URI failed validation: oversized (size-capped BEFORE
    /// parse), structurally malformed, no payments, or rejected by the
    /// audited zip321 parser (§4.6 hostile input). DELIBERATELY payload-free
    /// and detail-free: URIs carry addresses + amounts (§5.4 never-log), and
    /// upstream error values echo URI content — they are dropped here.
    #[error("payment URI invalid")]
    PaymentUriInvalid,
    /// A host-supplied txid string was not 64 hex characters (§4.6 hostile
    /// input). Payload-free: txids are on the §5.4 never-log list, so the
    /// offending string is never echoed back — the host passes back the
    /// `txid_hex` a history row handed it, verbatim.
    #[error("txid invalid")]
    TxidInvalid,
    /// FR-27 — the machine-memo READ scope cannot be used. One class, two
    /// moments: the config door refuses a scope that could never work (an
    /// EMPTY prefix matches every memo ever written; an over-long one can
    /// never match; too many of them make a hostile transaction expensive),
    /// and the read verb refuses to serve a scope that was never registered
    /// rather than answering with a silent empty list — which a host would
    /// read as "this transaction carries nothing" over a memo that is there.
    /// `reason` is a static string; no host input is ever echoed.
    #[error("machine-memo scope invalid: {reason}")]
    MachineMemoScopeInvalid { reason: &'static str },

    // ── Keystore (RW-KEY-*) ─────────────────────────────────────────────────
    /// Keychain locked / transiently unreachable — "unlock your device",
    /// retryable (§4.2a disambiguation: locked ≠ missing).
    #[error("keystore unavailable")]
    KeystoreUnavailable,
    /// Keychain item genuinely absent (or permanently invalidated by a
    /// biometric/PIN reset) while the wallet DB exists — surfaced, NEVER
    /// silently treated as a fresh wallet (§4.2a, the keysMissing lesson).
    #[error("keystore inconsistent with wallet store")]
    KeystoreInconsistent { permanently_invalidated: bool },
    /// Sealed-seed blob carries an unknown version byte (§4.2a
    /// validate-before-use) — likely an app downgrade reading a newer seal;
    /// surfaced honestly, never guessed at.
    #[error("sealed seed version unsupported")]
    SealVersionUnsupported { found: u8 },
    /// Sealed-seed blob failed validation or authenticated decryption.
    /// Truncation, tamper, and wrong-key COLLAPSE to this one error on
    /// purpose — no oracle on why an unseal failed (§4.2a; the identity-core
    /// `Unlock` posture).
    #[error("sealed seed invalid")]
    SealInvalid,
    /// No platform key vault exists at all (no Keystore tier on Android, no
    /// keyring on headless Linux). FAIL-CLOSED by design (§4.3a): the wallet
    /// will not custody a seed without a vault — holding the wrap key in
    /// process memory "for this session" is the forbidden silent degrade.
    /// PERMANENT for this device/session — distinct from the retryable
    /// `KeystoreUnavailable` (locked / transient daemon flake).
    /// ALSO raised by `db::ensure_sqlcipher` (FR-5 D-24): the linked SQLite is
    /// not SQLCipher, or the database key did not attach — the database would be
    /// stored unencrypted. A BUILD/linkage defect, permanent for the build.
    #[error("no platform key vault, or no SQLCipher, available")]
    VaultAbsent,
    /// Wrap artifact failed validation or authenticated unwrap against THIS
    /// wallet's sealed blob (§4.3a AAD binding). Substitution and corruption
    /// collapse here — cryptographically indistinguishable — but the LAYER
    /// is attributed: this is the wrap artifact failing, never reported as
    /// the seed seal's `SealInvalid` (`swapped_wrap_artifact_fails_loudly`).
    #[error("wrap artifact invalid for this sealed seed")]
    WrapArtifactInvalid,
    /// Wrap artifact carries an unknown version byte (§4.3a
    /// validate-before-use) — likely an app downgrade reading a newer
    /// artifact; surfaced honestly, never guessed at.
    #[error("wrap artifact version unsupported")]
    WrapVersionUnsupported { found: u8 },
    /// FR-47 — the platform key store did not answer inside the SDK's bound
    /// (a JNI / Security.framework call cannot be cancelled, so the SDK stops
    /// WAITING for it). Retryable; a wedged key store keeps answering this at
    /// once until its stuck call returns. Distinct from `KeystoreUnavailable`
    /// (locked) so the fail-open index reads can propagate EXACTLY this class
    /// and leave every other keychain error as it was; the bridge carries it
    /// under the `keystoreUnavailable` kind with its own code.
    #[error("keystore did not answer in time")]
    KeychainTimeout { cause: KeychainTimeoutCause },

    // ── Store / provisioning (RW-STORE-*) ───────────────────────────────────
    /// `open()` with no wallet at `db_dir`.
    #[error("wallet not found")]
    NotFound,
    /// DB belongs to a different network than the config (§8
    /// network-mismatch row).
    #[error("network mismatch")]
    NetworkMismatch,
    /// Provisioning remnant without a completion marker (§6.3 two-phase
    /// contract); repair resumes FROM the remnant.
    #[error("provisioning incomplete")]
    ProvisioningIncomplete,
    /// Storage corruption — fail-closed, surfaced, never auto-wipe (§6.1).
    #[error("wallet store corrupt")]
    StoreCorrupt,
    /// Disk full — DISTINCT from `StoreCorrupt` ("free space and retry"). The
    /// interactive seams surface it and STOP (no auto-retry) so the user frees space
    /// first; the background sync loop keeps retrying but on the CAPPED backoff ladder
    /// (`SYNC_BACKOFF_MAX_SECS`, never a hot spin), so a persistent full disk settles to
    /// one doomed pass per cap-interval — surfaced honestly as `Stalled { StorageFull }`
    /// meanwhile — until space frees or the user acts. (A dedicated park-until-freed for
    /// the sync loop is a possible enhancement; today the cap bounds the cost — §6.1/§6.2.)
    #[error("disk full")]
    DiskFull,
    /// `create`/`restore` against a db_dir that ALREADY holds a completed
    /// wallet (§6.3 "never clobbers an existing wallet"). The non-clobber
    /// outcome is TYPED, never a silent open or a destructive re-provision —
    /// silently opening would hide a confused host create/open path, and
    /// clobbering would lose funds. The host opens an existing wallet with
    /// `open()`. (Variant order follows the append-only code sequence:
    /// RW-STORE-006, after DiskFull's 005.)
    #[error("wallet already exists at this location")]
    WalletAlreadyExists,
    /// A store write lost a WAL writer-vs-writer race past `busy_timeout`
    /// (`SQLITE_BUSY`, W-swap-4-a-4) — TRANSIENT and RETRYABLE, never
    /// corruption: nothing was written (a failed `COMMIT` rolls the
    /// `IMMEDIATE` txn back clean), the store is intact, and the same call
    /// succeeds once the concurrent writer (typically an engine scan-batch
    /// commit) finishes. User-facing aux seams already retry the whole txn
    /// in-SDK (`db::with_aux_busy_retry`), so a surfaced `StoreBusy` means
    /// the store stayed contended through every attempt — the host renders
    /// "busy, try again in a moment" (the `WalletBusy` copy family), NEVER
    /// the corruption remedy. (Pre-4-a-4 this case was mislabeled
    /// `StoreCorrupt` — the device blocker. Variant order follows the
    /// append-only code sequence: RW-STORE-007.)
    #[error("wallet store busy")]
    StoreBusy,

    // ── Lifecycle / locking (RW-LIFE-*) ─────────────────────────────────────
    /// A second opener (this process or another) holds the exclusive
    /// single-writer lock — rejected IS the contract (§3.3).
    #[error("wallet already open")]
    WalletAlreadyOpen,
    /// Transient lifecycle phase (provisioning/repair/closing) — retry when
    /// the phase completes; renderable, not a race (§3.3).
    #[error("wallet busy")]
    WalletBusy { phase: LifecyclePhase },
    /// Call on a handle in a terminal/wrong state (§3.3 state machine).
    #[error("invalid lifecycle state")]
    InvalidState { phase: LifecyclePhase },
    /// `wipe()` refused while an instance holds the DB open (§3.1).
    #[error("wallet is open")]
    WalletOpen,
    /// `wipe()` would destroy non-terminal swap records (§6.3 M3): surfaced
    /// BEFORE proceeding; third-party path is HD-recoverable from the phrase,
    /// the host's panic-wipe path overrides this by design. NOT produced by the
    /// FR-14 `Wallet::wipe(cfg)` path: that verb NEVER opens the DB (crypto-correct
    /// — it must not unseal the very key it is about to destroy), so it cannot read
    /// swap state. This guard is reserved for a future handle-based wipe that holds
    /// an open DB; do NOT add a DB-open to `wipe(cfg)` to satisfy it.
    #[cfg(feature = "swap")]
    #[error("wipe with pending swaps")]
    WipeWithPendingSwap { count: u32 },
    /// `rescan_from` refused while a send is IN FLIGHT (`Submitting`/`Sent` —
    /// §4.4 W-swap-4-a-3, the witness-inversion fence). A rescan rebuilds the
    /// data DB from chain while the durable outbox SURVIVES, so a
    /// broadcast-but-unmined send's spent notes would read UNSPENT in the
    /// rebuilt view; the next drain would `ReProposeFresh` over that inverted
    /// witness and re-sign — potentially disjoint notes, and with the original
    /// tx still valid BOTH can mine: 2× pay. Retryable, hours-bounded WHILE
    /// resubmission passes run (the same drain-liveness qualifier as the
    /// purge): an in-flight row resolves by mine + burial-delete or by tx
    /// expiry → requeue → deadline purge — all of which ride completed sync
    /// passes over an advancing tip. `Queued` rows never trigger this (nothing
    /// signed — they re-propose fresh against the rebuilt view, the correct
    /// recovery). Like every rescan fault the refused call consumed the
    /// handle (seals + DBs intact); the host re-opens. Payload-free.
    #[error("rescan refused while a send is in flight")]
    RescanWithInFlightSend,
    /// #390 — the user-triggered "Check older swap addresses" deep scan was REFUSED,
    /// never run as a success-shaped no-op (spec §3.2h item 5). `reason` distinguishes
    /// swap turned OFF from a prior check still outstanding (see
    /// [`SwapAddressCheckRefusal`]). The op moves NO money and derives no new label —
    /// it widens the never-recycle counter + backfill ceiling so the standard
    /// transparent poll surfaces pre-restore swap addresses; a REFUSAL means nothing
    /// was widened. Exhaustion of the 2^31 index space fails closed as
    /// [`StoreCorrupt`](Self::StoreCorrupt) (mirroring `reserve_next_index`), not here —
    /// astronomically unreachable in a wallet's life. Payload-free on purpose (§5.4):
    /// the typed `reason` carries the branch, no index/count leaks.
    #[cfg(feature = "swap")]
    #[error("swap address check refused")]
    SwapAddressCheckRefused { reason: SwapAddressCheckRefusal },

    // ── Sync / provisioning (RW-SYNC-*) ─────────────────────────────────────
    /// A sync/provisioning step could not reach the endpoint (§3.2f). Carries the
    /// policy [`StallReason`] so the host renders the right next step:
    /// `TorUnavailable` under `TorPolicy::Required` (fail-closed — zero clearnet)
    /// vs `EndpointUnreachable` otherwise. The FIRST public transport-error door
    /// (inc-2c-ii deferred it); iv-d's sync surface reuses it. The `Display` is
    /// static (§5.4) — the host reads `code()` + the typed `stall`, never a
    /// message that could echo an endpoint.
    #[error("sync transport unavailable")]
    Sync { stall: StallReason },

    // ── Sync server (RW-SRV-*) — the picker (`sync-server-picker.md` §2) ───
    /// A `SyncServerChoice::Predefined` id the current config does not OFFER
    /// (`WalletConfig.sync_servers`). A host bug, not a user state: the picker
    /// renders only offered entries, so the reference UI cannot produce it.
    #[error("sync server not offered")]
    SyncServerNotOffered,
    /// The reachability probe could not dial the server, or the server did
    /// not answer within `SYNC_SERVER_PROBE_TIMEOUT_SECS` — the TRANSPORT
    /// class, distinct from the sync loop's [`StallReason`] (a status, not an
    /// error). A gated server refusing the key answers with a gRPC status and
    /// lands here too; payload-free (§5.4 — the host may be one the user
    /// typed). The switch that carried the probe changed NOTHING.
    #[error("sync server unreachable")]
    SyncServerUnreachable,

    /// The network requires consensus rules this build does not implement — or
    /// the endpoint is not on the chain we think it is. The SDK cannot tell
    /// those apart from inside, so there is ONE code rather than a distinction
    /// we could not justify (`ironwood-nu63-support.md` §2).
    ///
    /// Signing is refused; reading is not. Balance, history, receive, seed
    /// backup and viewing-key export all stay live behind this (§1.5) — a
    /// staleness fault must never lock a user out of their own money or their
    /// own keys.
    ///
    /// `Display` is static (§5.4); the host reads `code()` plus the typed
    /// branch ids, which are public protocol constants carrying no user data.
    #[error("network upgrade unsupported by this build")]
    NetworkUpgradeUnsupported {
        /// What our COMPILED params compute for `judged_at_height`. Never the
        /// endpoint's claim (§1.3).
        expected_branch_id: u32,
        /// The endpoint's self-reported branch. UNTRUSTED, display/diagnostics
        /// only, `None` when it withheld the field.
        endpoint_branch_id: Option<u32>,
        /// The height the verdict was judged at — the highest either side
        /// attested.
        judged_at_height: u32,
    },

    /// The endpoint will not say which network it is on (it omits
    /// `consensus_branch_id`) and the §6.3 grace this wallet extends such a
    /// server has run out — on the chain (a day of blocks past the last verdict
    /// that confirmed this build can transact), on the DEVICE CLOCK (a day of
    /// wall time past it, the axis a server that freezes its tip cannot hold
    /// still — GRACE-1, §4p), or it never began (no capable verdict ever). NOT
    /// a network upgrade: nothing is known to have changed, an app update
    /// fixes nothing, and the honest next step is SWITCH SERVERS — plus "check
    /// the device's date and time" for the clock, the one benign cause. Its
    /// own variant precisely so the host cannot render the
    /// [`Self::NetworkUpgradeUnsupported`] copy for it (§4p G-1, G-6).
    ///
    /// Signing is refused; reading is not (§1.5). Composing and queueing stay
    /// live; the drain applies the verdict, and a later pass against a server
    /// that reports its branch restores signing — the queued rows drain then.
    ///
    /// `Display` is static (§5.4); the typed payload is a reason and a block
    /// COUNT (a difference of two heights the sync surface already carries),
    /// never the absolute capable timestamp (§4p G-12).
    #[error("the endpoint has not reported its consensus branch within the grace")]
    ConsensusGraceExpired {
        /// Which rule ended the grace, or that it never began.
        by: crate::state::GraceExpiry,
        /// Blocks the judged height advanced since the last capable verdict —
        /// what the `Blocks` copy names; `None` for `NeverConfirmed`.
        blocks_since_last_current: Option<u32>,
    },

    /// This wallet has never evaluated a consensus verdict — it has not
    /// completed a sync pass yet (a fresh install that has not reached a
    /// server; INC-004: a pass evaluates before it scans). Signing is refused
    /// because there is no honest reading of "we have never checked" that
    /// permits it, and this is NOT "the network was upgraded" (nothing is known
    /// either way) — it is "not synced yet", which the first completed pass
    /// resolves. Composing and queueing stay live; the host renders the
    /// wait-for-sync story, never an update prompt (§4p item 2). Payload-free.
    #[error("no consensus verdict has been evaluated yet")]
    ConsensusNotEvaluated,

    /// `sync_for` (FR-40, the bounded sync) refused because a sync is already
    /// running on this handle: the background loop `start_sync` spawned, or
    /// another bounded pass. The single-writer contract allows one pass at a
    /// time, and a bounded pass that waited for the loop would spend its budget
    /// waiting. Retryable: `stop_sync` first, or wait for the other bounded
    /// pass to return. Nothing ran. Payload-free.
    #[error("a sync is already running")]
    SyncRunning,

    // ── Send pipeline (RW-SEND-*) — inc-2d-1 propose ────────────────────────
    /// Not enough spendable value to fund the requested send (the audited
    /// note-selector's verdict, §3.2h). Carries the amounts the host renders the
    /// honest "you have X spendable, Y is still confirming, you need Z" next step
    /// (§6.1) — `available` + `required` come straight from the selector;
    /// `pending_incoming` is read from the SAME spendable-policy summary as the
    /// balance (the SSOT) so "wait for confirmations" is never a guess. Debug
    /// carries the values for the host; `Display` is static — amounts are §5.4
    /// never-log, so the log line shows only the `code()`.
    #[error("insufficient funds")]
    InsufficientFunds {
        available: crate::money::Zatoshis,
        required: crate::money::Zatoshis,
        pending_incoming: crate::money::Zatoshis,
    },
    /// A send leg carried no amount (the ZIP-321 donation form `zcash:ADDR`,
    /// valid to display but not to SEND — §2.4). The propose funnel is the one
    /// place that knows "this is a send", so it owns this check (the construction
    /// `Payment::new` deliberately permits `None`); the host fills the amount in
    /// between parse and confirm. Payload-free.
    #[error("send amount required")]
    SendAmountRequired,
    /// The one-shot proposal token was already consumed (a double-tap on
    /// "send", §6.3 double-spend guard). The retained `Proposal` is taken on the
    /// first `send`; a second `send` of the same id lands here — never a second
    /// broadcast of the same notes. Payload-free.
    #[error("proposal already used")]
    ProposalAlreadyUsed,
    /// The retained proposal's chain anchor is stale — the wall-clock
    /// [`PROPOSAL_TTL_SECS`](crate::constants::PROPOSAL_TTL_SECS) elapsed, or the
    /// wallet had no usable anchor yet (not synced far enough). Re-`propose` to
    /// show a fresh fee against the current chain state (§3.2h). Payload-free —
    /// no anchor heights leak.
    #[error("proposal stale")]
    ProposalStale,
    /// Propose could not produce a transaction for a reason with no finer typed
    /// mapping AND one that is deterministic on this input — a change/fee-
    /// computation fault, an internal note-selection error, or a structurally-
    /// unsupported request (e.g. a multi-step pool-crossing send, deferred past
    /// this build): retrying the same payment unchanged re-fails, so the host's
    /// copy asks the user to check the details ("Couldn't prepare this payment.
    /// Check the details and try again."). The refusal the wallet's own state
    /// will clear on its own is NOT this — it is
    /// [`ProposeTransient`](Self::ProposeTransient) (INC-018 (b), phase-2 P2-2;
    /// `send::propose_refusal_is_transient` is the one place the two classes are
    /// told apart). Fail-closed and payload-free (§4.6 / §5.4): the selector's
    /// message can echo amounts, so it is DROPPED here; the host renders
    /// "couldn't prepare this payment", never a code or an echoed value. A
    /// DB-access fault inside propose stays the typed
    /// [`StoreCorrupt`](Self::StoreCorrupt) (one corruption door), never this.
    #[error("could not prepare payment")]
    ProposeFailed,
    /// Propose could not produce a transaction because of a condition the
    /// wallet's OWN state will clear without the user changing anything — the
    /// retryable class INC-018 (b) separates from [`ProposeFailed`](Self::ProposeFailed)
    /// (phase-2 P2-2, maintainer decision 4). Named from upstream's enum, not
    /// from field logs (`send::propose_refusal_is_transient`): a note whose
    /// witness the scan has not completed (`CommitmentTree`), an anchor the
    /// wallet has not recorded yet (`Proposal(AnchorNotFound)`), or an input a
    /// concurrent proposal holds locked (`Proposal(InputsLocked)`, which upstream
    /// itself documents as "transient … retry"). The host renders "Couldn't
    /// prepare this payment just now. Try again in a moment." — never "check the
    /// details", which sent the user to correct input that was correct.
    /// Payload-free like its sibling. **Reach:** EMPTY on the propose path at
    /// this pin — the predicate's doc is the one place that says why, with the
    /// upstream sites, and INC-018's registry row is OWED on (b) for it. The
    /// same two upstream shapes on the create+sign path fold to
    /// [`ProposalStale`](Self::ProposalStale) instead (`send::map_create_err`:
    /// there a fresh `propose` is the remedy; here there is no proposal yet to
    /// refresh, and a moment's wait is the honest one).
    #[error("could not prepare payment right now")]
    ProposeTransient,
    /// Create+sign (`send`, inc-2d-2) could not build/prove/sign the proposed
    /// transaction for a reason with no finer typed mapping — a proving or
    /// transaction-builder fault, or the derived spending key's UFVK not matching
    /// the account (impossible on the production seed-derived path, but a
    /// defense-in-depth LOUD catch otherwise — never a silent wrong-key sign).
    /// Fail-closed and payload-free (§4.6 / §5.4): the builder's error value can echo
    /// amounts, so it is DROPPED here; the host renders "couldn't complete this
    /// payment". A stale or
    /// reorg-pruned anchor/witness folds to [`ProposalStale`](Self::ProposalStale)
    /// (re-`propose`); a DB-access fault stays the typed
    /// [`StoreCorrupt`](Self::StoreCorrupt) (one corruption door), never this.
    #[error("could not complete payment")]
    SignFailed,
    /// The ZIP-320 / TEX two-step send path reserves ONE engine-owned ephemeral
    /// transparent address per outstanding tx0, bounded by the engine's ephemeral gap
    /// limit (default 10). Outstanding reservations fill that window, so the next TEX
    /// send hits the ceiling at CREATE time (`create_proposed_transactions` →
    /// `reserve_next_n_ephemeral_addresses` → `SqliteClientError::ReachedGapLimit`).
    /// NOT a fault of this send and NOT corruption — but the ceiling is DUAL-NATURED
    /// (#315 review) and the host copy must promise neither "just wait" nor doom: a
    /// slot held by a LIVE in-flight tx0 frees when it MINES (a mined first-use
    /// advances the engine's ephemeral gap — that half self-heals), while a slot held
    /// by a LEAKED reservation (its tx0 expired un-mined) NEVER frees on its own — the
    /// engine has no un-reserve, and tx0 expiry does NOT release it (`find_gap_start`
    /// advances ONLY on a MINED first-use; refuted at the pinned engine source). A
    /// blind retry only churns the window (and each churned TEX attempt reserves a
    /// FRESH index — the #315 slice-1 attempt cap bounds that; the leaked half's
    /// remedy is the slice-2 self-mint reclaim, §3.2i-2 2e-2b-vi), a SEPARATE concern
    /// from the tx0-mined stranded-money surface (pt 6), never an endless wait.
    /// Distinct from
    /// [`StoreCorrupt`](Self::StoreCorrupt) (a real DB fault — mislabeling the ceiling as
    /// corruption would scare a user off a recoverable wallet) and from
    /// [`QueuedSendsFull`](Self::QueuedSendsFull) (the SDK's OWN intent-table cap, an
    /// unrelated resource). Payload-free (§5.4): the engine's scope/index are diagnostic
    /// ints, dropped here. Surfaced by `create_signed_core` via `map_create_err` (in
    /// [`send`](crate::send), the SSOT), so the interactive send returns it to the user and
    /// the background queued drain parks it distinctly (`Prepared::Ceiling`), never swallowed
    /// into a generic retry. Reachable in production now that the slice-A `steps()>1` gates are
    /// gone (gate-removal, 2e-2b-v-5) — a TEX two-step that exceeds the ephemeral gap-limit
    /// window surfaces this typed ceiling rather than stranding on a silent failure.
    #[error("too many transparent transfers still confirming")]
    TexSendLimitReached,

    // ── BROADCAST / OUTBOX (RW-BCAST-*) ──────────────────────────────────────
    /// A durably-queued send (§6.2 / inc-2d-3-b) could no longer be covered when the
    /// §1.7 resubmission machinery tried to re-propose it — the funds it would have spent
    /// have moved (e.g. another send, or a swept-away note) so a fresh proposal can't be
    /// built. Distinct from [`InsufficientFunds`](Self::InsufficientFunds) (a *fresh*
    /// propose verdict the host can act on with a different amount): this is a *queued*
    /// intent that has gone un-fundable since it was enqueued, so the honest host action is
    /// to drop/redo it, not retry. Payload-free (§5.4 — no amounts leak). Raised on the
    /// resubmission/reconcile path (inc-2d-3-b-ii-B); the variant is defined now so the
    /// `RW-BCAST` register lands in one coherent addition.
    #[error("queued send can no longer be funded")]
    QueuedSendStale,
    /// The durable queued-send store is at its [`QUEUED_SEND_INTENTS_MAX`](crate::constants::QUEUED_SEND_INTENTS_MAX)
    /// cap — a defensive bound enforced at the `queue_send` enqueue door so a buggy or
    /// hostile host can never grow the intent table without limit (each queued intent costs
    /// a propose+create+broadcast on every resubmission pass — inc-2d-3-b-ii). The honest
    /// host next step is to let the pending queue drain (sync/resubmit) before queueing more.
    /// Payload-free.
    #[error("too many queued sends")]
    QueuedSendsFull,
    /// A SECOND swap-deposit intent was refused at the enqueue door while one is
    /// already in flight (§4.4 — the ONE-deposit-in-flight guard, W-swap-4-a-2):
    /// any `deposit_deadline`-tagged row (Queued/Submitting/Sent) blocks a new
    /// deposit enqueue, checked ATOMICALLY inside the same `IMMEDIATE` insert
    /// transaction. This is the cross-QUOTE double-deposit guard: a `swapExecute`
    /// whose unbounded SIGN overran the host's Dart timeout looks failed to the
    /// host, invites a re-quote (a NEW quote id ⇒ a fresh single-flight), and
    /// without this refusal a second deposit would sign while the first drains
    /// (2× ZEC). Deadline-BOUNDED, never a wedge: a signed deposit dies at
    /// mined-buried delete or expiry→requeue, and a `Queued` one at the drain's
    /// SEEDLESS lapsed-deposit purge
    /// ([`send::purge_lapsed_queued_deposits`](crate::send) — run unconditionally
    /// every pass, because the in-gate `prepare_queued` delete is unreachable at
    /// a background drain on the host-custody tier and on a portless wallet), so
    /// the guard self-clears with the quote lifetime at EVERY custody tier —
    /// while resubmission passes run (W-swap-4-a-3: the purge rides the drain;
    /// a wallet that never completes a pass clears at its next one). Against an
    /// honest provider that lifetime is ~15 min: the adapter requests
    /// `SWAP_DEPOSIT_DEADLINE_OUT_OF_ZEC_SECS` for the wallet-sent OutOfZec
    /// deposit; the 24 h clamp is only the hostile-provider fence.
    /// Raised ONLY on the deposit enqueue (an ordinary
    /// `queue_send` never carries a deadline tag), so it never crosses the wallet
    /// FFI — the [`DepositSender`](crate::swap::DepositSender) impl maps it to the
    /// swap-namespaced `SwapAlreadyInFlight`, whose remedy (track, do NOT
    /// re-quote) differs from every `DepositSendFailed` cause. Distinct from
    /// [`QueuedSendsFull`](Self::QueuedSendsFull) (the generic table cap — a
    /// resource bound, not a money-safety single-flight). Payload-free.
    #[error("a swap deposit is already in flight")]
    SwapDepositInFlight,

    // ── IO (RW-IO-*) ────────────────────────────────────────────────────────
    /// Underlying IO failure not better classified above.
    #[error("io failure")]
    Io(#[from] std::io::Error),
}

impl WalletError {
    /// The ONE door for classifying raw IO errors (single source of truth):
    /// ENOSPC becomes the typed `DiskFull` ("free space and retry" — §6.1,
    /// distinct from corruption and from generic IO) so hosts keying on
    /// codes render the honest next step. Every filesystem call site maps
    /// through here.
    pub(crate) fn from_io(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::StorageFull {
            Self::DiskFull
        } else {
            Self::Io(e)
        }
    }
}

impl WalletError {
    /// Stable, greppable, subsystem-prefixed code (§6.1; the Zodl
    /// error-code lesson §1.7). Codes are append-only: a released code is
    /// never renumbered or reused.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidSeedLength { .. } => "RW-SEED-001",
            Self::InvalidMnemonic { .. } => "RW-SEED-002",
            Self::NoMnemonic => "RW-SEED-003",
            Self::SeedRequired => "RW-SEED-004",
            Self::SeedMismatch => "RW-SEED-005",
            Self::KeyDerivation => "RW-SEED-006",
            Self::WatchOnly => "RW-VIEW-001",
            Self::InvalidViewingKey => "RW-VIEW-002",
            Self::InvalidEndpoint { .. } => "RW-CFG-001",
            Self::BirthdayInFuture => "RW-CFG-002",
            Self::InvalidDbDir { .. } => "RW-CFG-003",
            Self::InvalidEndpointAuth { .. } => "RW-CFG-004",
            Self::BroadcastJitterTooLong { .. } => "RW-CFG-005",
            Self::AmountOutOfRange => "RW-PAY-001",
            Self::MemoTooLong { .. } => "RW-PAY-002",
            Self::ReservedMemoNotSendable => "RW-PAY-003",
            Self::MemoRequiresShieldedRecipient => "RW-PAY-004",
            Self::AddressInvalid => "RW-PAY-005",
            Self::MemoInvalid => "RW-PAY-006",
            Self::MemoConflict => "RW-PAY-011",
            Self::ZeroValuedTransparentOutput => "RW-PAY-007",
            Self::PaymentUriInvalid => "RW-PAY-008",
            Self::TxidInvalid => "RW-PAY-009",
            Self::MachineMemoScopeInvalid { .. } => "RW-PAY-010",
            Self::KeystoreUnavailable => "RW-KEY-001",
            Self::KeystoreInconsistent { .. } => "RW-KEY-002",
            Self::SealVersionUnsupported { .. } => "RW-KEY-003",
            Self::SealInvalid => "RW-KEY-004",
            Self::VaultAbsent => "RW-KEY-005",
            Self::WrapArtifactInvalid => "RW-KEY-006",
            Self::WrapVersionUnsupported { .. } => "RW-KEY-007",
            Self::KeychainTimeout { .. } => "RW-KEY-008",
            Self::NotFound => "RW-STORE-001",
            Self::NetworkMismatch => "RW-STORE-002",
            Self::ProvisioningIncomplete => "RW-STORE-003",
            Self::StoreCorrupt => "RW-STORE-004",
            Self::DiskFull => "RW-STORE-005",
            Self::WalletAlreadyExists => "RW-STORE-006",
            Self::StoreBusy => "RW-STORE-007",
            Self::WalletAlreadyOpen => "RW-LIFE-001",
            Self::WalletBusy { .. } => "RW-LIFE-002",
            Self::InvalidState { .. } => "RW-LIFE-003",
            Self::WalletOpen => "RW-LIFE-004",
            #[cfg(feature = "swap")]
            Self::WipeWithPendingSwap { .. } => "RW-LIFE-005",
            Self::RescanWithInFlightSend => "RW-LIFE-006",
            #[cfg(feature = "swap")]
            Self::SwapAddressCheckRefused { .. } => "RW-LIFE-007",
            Self::Sync { .. } => "RW-SYNC-001",
            Self::NetworkUpgradeUnsupported { .. } => "RW-SYNC-002",
            Self::ConsensusGraceExpired { .. } => "RW-SYNC-003",
            Self::ConsensusNotEvaluated => "RW-SYNC-004",
            Self::SyncRunning => "RW-SYNC-005",
            Self::SyncServerNotOffered => "RW-SRV-001",
            Self::SyncServerUnreachable => "RW-SRV-002",
            Self::InsufficientFunds { .. } => "RW-SEND-001",
            Self::SendAmountRequired => "RW-SEND-002",
            Self::ProposalAlreadyUsed => "RW-SEND-003",
            Self::ProposalStale => "RW-SEND-004",
            Self::ProposeFailed => "RW-SEND-005",
            Self::SignFailed => "RW-SEND-006",
            Self::TexSendLimitReached => "RW-SEND-007",
            Self::ProposeTransient => "RW-SEND-008",
            Self::QueuedSendStale => "RW-BCAST-001",
            Self::QueuedSendsFull => "RW-BCAST-002",
            Self::SwapDepositInFlight => "RW-BCAST-003",
            Self::Io(_) => "RW-IO-001",
        }
    }
}

/// FR-47 — why a key-store call answered [`WalletError::KeychainTimeout`]
/// without its result. `#[non_exhaustive]` per G2 (a host keeps a default arm).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum KeychainTimeoutCause {
    /// The call was submitted and did not return inside its bound.
    Timeout,
    /// An earlier call the SDK stopped waiting for is still inside the key
    /// store; this one was refused at once and never reached it.
    Busy,
    /// The wipe's key-store budget was already spent when this call was
    /// submitted; it never reached the key store.
    PastDeadline,
}

impl KeychainTimeoutCause {
    /// The payload-free log vocabulary (`wallet.vault_call`'s `outcome`).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Busy => "busy",
            Self::PastDeadline => "past_deadline",
        }
    }
}

/// #390 — why a user-triggered "Check older swap addresses" deep scan was REFUSED
/// (carried by [`WalletError::SwapAddressCheckRefused`]). The op never silently no-ops:
/// each reason maps to distinct honest host copy. `#[non_exhaustive]` per G2 (a host
/// keeps a default arm).
#[cfg(feature = "swap")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SwapAddressCheckRefusal {
    /// Swap is turned OFF at this instance (a §3.5 WindDown/Hard kill stops the
    /// transparent-refresh polling the check depends on) — widening would be a
    /// success-shaped wedge (the band would register nothing while honest-off). Render
    /// "swap is unavailable right now".
    SwapDisabled,
    /// A prior check's band is still registering, OR its one-window `backfill:` watch
    /// rows have not yet expired — at most ONE outstanding band per settlement window.
    /// This makes an ambiguous-outcome retry safe (the raise is deliberately NOT
    /// idempotent), paces reruns, and CAPS the elective poll-set growth (the
    /// hostile-host griefing fence). Render "still checking the last range — try again
    /// later".
    CheckOutstanding,
}

/// Swap error taxonomy (spec §2.6/§6.1; `swap` feature). Same code
/// discipline as `WalletError`.
#[cfg(feature = "swap")]
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SwapError {
    /// Requested tolerance beyond the hard `SLIPPAGE_MAX_BPS` ceiling (M1):
    /// a wider window is a drain surface, not a preference.
    #[error("slippage tolerance above hard ceiling")]
    SlippageToleranceTooHigh { requested_bps: u16, max_bps: u16 },
    /// THE M1 user-anchored sanity bound: a quote side outside
    /// [user request ± tolerance] — rejected BEFORE SIGNING; a deposit is
    /// never signed anchored to provider numbers alone.
    #[error("quote outside user-anchored bounds")]
    QuoteOutOfBounds { side: QuoteBoundSide },
    /// Deposit deadline lapsed (monotonic-anchored, §2.6) — no signing past
    /// it, ever.
    #[error("quote expired")]
    QuoteExpired,
    /// OutOfZec destination rejected (§2.6).
    #[error("swap destination invalid")]
    DestinationInvalid { reason: DestinationInvalidReason },
    /// Request shape inconsistent with the direction (e.g. exact side names
    /// the wrong asset family).
    #[error("quote request invalid: {reason}")]
    RequestInvalid { reason: &'static str },
    /// Provider unreachable / 5xx — retryable; the wallet core is unaffected
    /// (port isolation).
    #[error("swap provider unavailable")]
    ProviderUnavailable,
    /// Provider response violated the protocol contract — closed-world
    /// handling (§4.6): typed, never a panic, never trusted further.
    #[error("swap provider protocol violation")]
    ProviderProtocol { reason: ProviderProtocolReason },
    /// Swap is turned OFF at this instance (§3.5 kill layers 3/4): a host
    /// `WindDown`/`Hard` kill reached `SwapService`, so new quotes/executes
    /// are refused. Honest-degradation — the UI renders "swap unavailable",
    /// never a code. Payload-free on purpose: the service state carries the
    /// severity; the door only says "off" (both severities refuse NEW work).
    #[error("swap is disabled")]
    SwapDisabled,
    /// The wallet-side §4.4 deposit leg could not be queued after the provider
    /// registered intent — the wallet was tearing down / wiped, or its store
    /// errored. Honest-degradation: the swap does NOT proceed (no ZEC left the
    /// pool), the user re-quotes; the provider refunds an un-deposited quote
    /// after its deadline. The [`DepositSender`](crate::swap::DepositSender) impl
    /// maps the underlying `WalletError` to this so the swap port never leaks a
    /// wallet-internal type (port isolation). Distinct from `ProviderUnavailable`
    /// (that is the PROVIDER; this is OUR side).
    #[error("swap deposit send failed")]
    DepositSendFailed,
    /// The wallet could not produce a FRESH transparent refund address for an
    /// `OutOfZec` quote (§2.6 HARD-H; the QUOTE-time counterpart of
    /// `DepositSendFailed`). Either the wallet is tearing down / wiped, its
    /// never-recycle index store errored, OR this wallet structurally cannot
    /// serve OutOfZec refunds (no seed at rest, or an exotic seed length BIP32
    /// transparent derivation cannot use — see
    /// `derivation::supports_transparent_refund`). Honest-
    /// degradation: NO quote is returned, so nothing leaves the pool — the host
    /// renders "swap unavailable / retry"; a structurally-incapable wallet stays
    /// incapable until reconstructed. Payload-free on purpose (a refund address
    /// is §5.4 never-log). The [`RefundAddressSource`](crate::swap::RefundAddressSource)
    /// impl maps the underlying `WalletError` to this so the swap port never
    /// leaks a wallet-internal type (port isolation).
    #[error("swap refund address unavailable")]
    RefundAddressUnavailable,
    /// The wallet could not mint a FRESH ZEC DESTINATION address for an `IntoZec`
    /// quote (§3.3b D1 / ADR-0530; the QUOTE-time MIRROR of
    /// `RefundAddressUnavailable` — refunds are the OutOfZec leg, destinations the
    /// IntoZec leg). Either the wallet is tearing down / wiped, its never-recycle
    /// index store errored, NO account is provisioned yet (a fresh wallet that has
    /// not synced — the engine UFVK the derivation reads is not there), OR the
    /// engine could not derive a conforming diversified address. Honest-degradation:
    /// NO quote is returned, so the provider never bound a deposit to a destination
    /// we can't track — nothing is at risk; the host renders "swap unavailable /
    /// retry". Payload-free on purpose (a destination address is §5.4 never-log).
    /// The [`DestinationAddressSource`](crate::swap::DestinationAddressSource) impl
    /// maps the underlying `WalletError` to this so the swap port never leaks a
    /// wallet-internal type (port isolation).
    #[error("swap destination address unavailable")]
    DestinationAddressUnavailable,
    /// The wallet could not maintain its DURABLE in-flight issued-quote state
    /// (§3.2 W2; the W-swap-3-c-1 `issued_swap_quote` store) — `persist` at quote
    /// time, or the atomic single-flight `take` at execute time, failed because the
    /// wallet was tearing down / wiped or its sealed aux store errored. Fail-CLOSED:
    /// a quote we cannot durably record is NOT returned, and an execute we cannot
    /// durably consume does NOT register provider intent — nothing leaves the pool
    /// either way, the user re-quotes. Distinct from `DepositSendFailed`/
    /// `RefundAddressUnavailable` (those are the deposit/refund LEGS); this is the
    /// durable-bookkeeping store that makes a crash-then-requote double-deposit
    /// observable to recovery (W-swap-3-c-3). Payload-free on purpose (the stored
    /// deposit address + amount are §5.4 never-log). The
    /// [`IssuedQuoteStore`](crate::swap::IssuedQuoteStore) impl maps the underlying
    /// `WalletError` to this so the swap port never leaks a wallet-internal type
    /// (port isolation). Since #367 a residual `StoreBusy` at the QUOTE persist
    /// or the EXECUTE take maps to the typed retryable [`Self::SwapStateBusy`]
    /// instead (nothing was consumed at either site — a plain retry genuinely
    /// succeeds); a busy `record_started` still collapses HERE (it sits
    /// post-claim, where the honest remedy is re-quote, not retry).
    #[error("swap durable state unavailable")]
    SwapStateUnavailable,
    /// The wallet's durable swap store was transiently BUSY (a residual
    /// `StoreBusy` that survived the in-SDK bounded `with_aux_busy_retry` —
    /// W-swap-4-a-4: WAL makes it rare by construction) at the quote-time
    /// `persist` or the execute-time single-flight `take` (#367). The HOST
    /// contract: RETRYABLE — a plain retry of the SAME action genuinely
    /// succeeds once the contending writer finishes; render "the wallet is
    /// busy for a moment, try again" (the send path's StoreBusy honesty),
    /// never the `SwapStateUnavailable` dead-end whose remedy is a re-quote.
    /// For an execute-take busy the quote is UNCONSUMED and still executable —
    /// the take's own txn never began, and the ONE post-commit sibling write
    /// on that path (the IntoZec `mark_executed` watch-window extension)
    /// tolerates a residual busy internally rather than surfacing here
    /// (money-review precision: a consumed quote must never be labeled
    /// retryable). For a quote-persist busy the retry mints a fresh quote,
    /// which works regardless of which of the two persist-path txns went busy
    /// (a first-wins row from the committed half squats at most one cap slot
    /// until its deadline prune). Payload-free on purpose (same §5.4 posture
    /// as its sibling).
    #[error("swap durable state busy — retry")]
    SwapStateBusy,
    /// A swap deposit is ALREADY in flight (§4.4 W-swap-4-a-2 — the ONE-deposit-
    /// in-flight guard refused a second enqueue while a deadline-tagged intent
    /// exists). The REMEDY is what makes this its own variant and not another
    /// `DepositSendFailed`: the host must NOT re-quote (a re-quote mints a new
    /// quote id, defeats the per-quote single-flight, and is exactly the
    /// double-deposit door this guard closes) — it renders "a swap is already in
    /// progress" and points the user at the tracked swap / activity surface. The
    /// refusing execute already consumed its quote and registered provider
    /// intent — fail-safe (a registered-but-undeposited quote refunds
    /// provider-side after its deadline; nothing left the pool). Self-clears
    /// with the in-flight deposit's lifetime (mined, or deadline-lapsed +
    /// deleted). Payload-free. The [`DepositSender`](crate::swap::DepositSender)
    /// impl maps the wallet-internal `SwapDepositInFlight` to this (port
    /// isolation).
    #[error("a swap is already in progress")]
    SwapAlreadyInFlight,
    /// The wallet is WATCH-ONLY (#397 §3.7 D3) — it holds no spending keys,
    /// so the WHOLE swap surface is structurally off (refused at the
    /// `enable_swap` constructor; no service is ever wired). STRUCTURAL, not
    /// retryable — unlike `ProviderUnavailable` (a transient transport /
    /// provider outage a retry can clear), no retry ever helps: only a
    /// spending wallet can swap. The swap-taxonomy mirror of
    /// `WalletError::WatchOnly` (RW-VIEW-001), so the api/swap layer passes
    /// the refusal through TYPED instead of masking it as a retryable
    /// provider outage (the #397 review M3/H5 bridge caveat, now closed).
    /// Payload-free by construction.
    #[error("watch-only wallet cannot swap")]
    WatchOnly,
    /// The caller's [`SwapQuote`](crate::swap::SwapQuote) names an issued record
    /// but carries DIFFERENT terms from it (stage S8, R01): a deposit address,
    /// amount, foreign amount, memo, refund target or spend binding that is not
    /// what this service issued under that id. Refused BEFORE the durable claim
    /// and before any provider or deposit call — nothing was consumed, nothing
    /// left the pool, and the record stays executable by the DTO the SDK
    /// actually returned. A host sees it when it passes back an edited or
    /// hand-built DTO, or one quote's numbers under another quote's id; the
    /// remedy is to pass the returned DTO unchanged, else re-quote. Payload-free
    /// (the differing values are §5.4 never-log).
    #[error("quote terms differ from the issued record")]
    QuoteTermsDiffer,
}

#[cfg(feature = "swap")]
impl SwapError {
    /// Stable, append-only codes (see `WalletError::code`).
    pub fn code(&self) -> &'static str {
        match self {
            Self::SlippageToleranceTooHigh { .. } => "RW-SWAP-001",
            Self::QuoteOutOfBounds { .. } => "RW-SWAP-002",
            Self::QuoteExpired => "RW-SWAP-003",
            Self::DestinationInvalid { .. } => "RW-SWAP-004",
            Self::RequestInvalid { .. } => "RW-SWAP-005",
            Self::ProviderUnavailable => "RW-SWAP-006",
            Self::ProviderProtocol { .. } => "RW-SWAP-007",
            Self::SwapDisabled => "RW-SWAP-008",
            Self::DepositSendFailed => "RW-SWAP-009",
            Self::RefundAddressUnavailable => "RW-SWAP-010",
            Self::SwapStateUnavailable => "RW-SWAP-011",
            Self::DestinationAddressUnavailable => "RW-SWAP-012",
            Self::SwapAlreadyInFlight => "RW-SWAP-013",
            Self::SwapStateBusy => "RW-SWAP-014",
            Self::WatchOnly => "RW-SWAP-015",
            Self::QuoteTermsDiffer => "RW-SWAP-016",
        }
    }
}

/// Which quote side broke the M1 bound.
#[cfg(feature = "swap")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum QuoteBoundSide {
    Zec,
    Foreign,
}

/// Why an OutOfZec destination was refused (§2.6).
#[cfg(feature = "swap")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum DestinationInvalidReason {
    /// OutOfZec requires a foreign destination.
    Missing,
    /// IntoZec must NOT carry one (we receive to our own fresh address).
    NotAllowedForDirection,
    /// A ZEC→ZEC "swap" is a fee-burning provider round-trip (§1.7).
    ZcashAddressNotAllowed,
    /// Exceeds `PROVIDER_STR_MAX_BYTES` (§4.6 m1).
    Oversized,
}

/// How a provider response broke the contract (§4.6) — typed so the host can
/// render an honest "provider misbehaved" without us leaking the payload.
#[cfg(feature = "swap")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum ProviderProtocolReason {
    /// Echoed refundTo ≠ the one WE sent — abort before any deposit (§2.6).
    RefundAddressMismatch,
    /// A provider string exceeded its named bound (m1).
    OversizedField,
    /// Amount field not a finite, in-range decimal (m1).
    MalformedAmount,
    /// Unknown status variant (closed-enum rule).
    UnknownStatus,
    /// OutOfZec deposit address failed validation (§4.4: parseable,
    /// TRANSPARENT, right network — checked before any signing path).
    DepositAddressInvalid,
    // ── Appended for the wire adapter (ADR-0525, W-swap-1) ──────────────────
    /// A field the flow REQUIRES was absent (e.g. `depositAddress` or
    /// `deadline` missing from a non-dry quote response).
    MissingField,
    /// HTTP-level contract break: non-success status, or a redirect (a funds
    /// path NEVER silently follows to another host).
    UnexpectedHttpStatus,
    /// Response body exceeded the endpoint's named byte cap (§4.6 —
    /// size-capped BEFORE buffering; distinct from a single oversized field).
    OversizedBody,
    /// Body was not valid JSON of the documented shape.
    UndecodableResponse,
    /// An OUTOFZEC quote carried a required `depositMemo` the wallet cannot
    /// honor: the wallet sends its OWN deposit (a shielded→transparent ZEC tx)
    /// and cannot attach a source-chain memo, so depositing would lose funds —
    /// refused typed. (IntoZec, where the USER sends the deposit, now CARRIES
    /// the memo and shows it on D7; this variant is the OutOfZec guard only.
    /// ZEC-chain deposits carry no memo, so it never fires in practice.)
    DepositMemoUnsupported,
    /// The provider no longer knows this swap id (HTTP 404 on status) —
    /// definitive, not transient; the record-quarantine policy is the
    /// service's (resume contract, §3.2).
    SwapNotFound,
    /// The provider's swap id collides with a wallet-RESERVED synthetic key
    /// namespace (#368: the `backfill:` prefix keying the ADR-0527 one-shot
    /// watch rows). Ids are §4.6 every-byte-hostile API data; an id inside the
    /// reserved namespace could pre-claim or extend a synthetic row and
    /// silently suppress refund detection, so it is refused at the quote door
    /// — no real provider id shape (base58/bech32 handles) contains `:`.
    /// (FFI: mirrored as the bridge's `ReservedId`.)
    ReservedId,
    /// The provider's echo of our quote request states different terms from
    /// the ones we sent — a different recipient, asset, amount, exact side or
    /// slippage — so the quote is refused before any review or deposit
    /// (2026-10-07 external review, finding 3). A different refund keeps
    /// `RefundAddressMismatch`. (FFI: mirrored as the bridge's own variant, as
    /// `bridge_enums_cover_core_variants` requires.)
    RequestEchoMismatch,
}

/// `NetDialer` failure surface (spec §3.2). Host-constructible: dialer
/// implementations live OUTSIDE this crate (the whole point of the seam).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DialError {
    /// Proxy / runtime unreachable. Under `TorPolicy::Required` this
    /// fail-closes the caller: zero packets, `Stalled { TorUnavailable }`.
    #[error("dial target or proxy unreachable")]
    Unreachable,
    /// Dial timed out (monotonic).
    #[error("dial timed out")]
    Timeout,
    /// The dialer cannot serve this request shape (e.g. isolation demanded
    /// but unsupported) — a configuration truth, surfaced not papered over.
    #[error("dial request unsupported by this dialer")]
    Unsupported,
    /// Underlying IO failure.
    #[error("dial io failure")]
    Io(#[from] std::io::Error),
    /// The host's transport is not ready yet (a bootstrap in progress —
    /// FR-29, `ZW_DIAL_NOT_READY`). NOT a reachability failure: `Required`
    /// fails closed, `Preferred` WAITS and never falls back to clearnet on
    /// it (ADR-0546), however long the bootstrap takes — the state renders
    /// `Bootstrapping`. A transport its host has declared FAILED is not a
    /// bootstrap, and is [`Self::TransportFailed`] instead.
    #[error("host transport not ready")]
    NotReady,
    /// The host's transport backing was retired or replaced while this work
    /// was in flight (FR-29, `ZW_DIAL_RETIRED`; a carrier switch, honest-off):
    /// the op fails typed, the host re-arms behind the same registration.
    /// Never a clearnet fallback (ADR-0546).
    #[error("host transport retired")]
    Retired,
    /// The host DECLARED its transport failed (`ZW_HEALTH_FAILED`, ADR-0549):
    /// it is not going to start, as opposed to not having started yet. Raised
    /// by the SDK's own readiness gate from the host's descriptor — NO dial
    /// code maps to it (the `ZW_DIAL_*` table is unchanged) and the host is
    /// never asked to dial. `Required` fails closed. Under `Preferred` it is
    /// the private path FAILING, so a minute of it is switch-eligible like
    /// `Unreachable` (ADR-0553 as narrowed by ADR-0552 phase 2: "a path that
    /// never starts should switch" holds once the host itself has given up,
    /// and never for a bootstrap still in progress).
    #[error("host transport declared failed")]
    TransportFailed,
}
