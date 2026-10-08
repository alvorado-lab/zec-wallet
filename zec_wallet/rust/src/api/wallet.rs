//! The live wallet handle Dart holds (spec §3.1/§3.3) — the FIRST STATEFUL
//! FFI surface. Every other `api/` file is a stateless string/DTO codec; this
//! one owns the opaque handle and its async lifecycle + cold-read methods.
//!
//! WHY this file may name the core crate (the one carve-out from the `api/`
//! "no core type" rule in `api/mod.rs`): an OPAQUE handle by definition holds
//! core state, so the field type `zec_wallet_core::Wallet` is unavoidable — but
//! it is the ONLY core reference here, and FRB never mirrors an opaque's
//! fields, so no core type ever reaches a generated Dart signature. All the
//! core LOGIC (config validation, seed construction, DTO/error conversion)
//! stays in `crate::convert`; this module is just the handle + the locking.
//!
//! Almost NO key material crosses here (spec §3.3, enforced by the
//! `ffi_surface_exposes_no_key_types` gate): `createGenerated` makes the seed
//! in Rust and `open` needs none. The TWO sanctioned crossings are the
//! recovery-phrase pair — outbound `reveal_mnemonic` (backup) and inbound
//! `restore` (recovery) — both carry the words as a plain `Vec<String>` (never
//! a key TYPE), construct/destructure the `SeedSource` in `crate::convert` (off
//! this scanned surface), and are method/param-name allowlisted in the gate.
//!
//! CONCURRENCY: FRB wraps this opaque in a `RwLock` (it cannot mirror the
//! private `Option<Wallet>` field), so `&self` reads run CONCURRENTLY (shared
//! read lock) and `&mut self` `close` is EXCLUSIVE (write lock). `close`
//! consumes the inner `Wallet` so the core's `close()` runs its proper
//! teardown — stop+join the sync loop, then close the SQLCipher connection on
//! the blocking pool and release the single-writer lock BEFORE returning, so
//! a subsequent `open` at the same `dbDir` never races a half-closed DB. A
//! call after `close` (the inner is `None`) is a typed `invalidState`, never a
//! panic; a second `close` is idempotent.
//!
//! LIVE STREAM: `watch_sync_status` exposes the §3.3 honest sync indicator as a
//! Dart `Stream`. It does NOT pump inline — that would pin the `&self` read lock
//! for the stream's whole life and block `close`'s write lock (a close-hang). The
//! CORE owns the detached pump (`rw::Wallet::watch_sync_status` over the
//! `SyncStatusSink` seam, the runtime living where tokio belongs); the bridge only
//! adapts the FRB `StreamSink` + the DTO conversion (`convert::stream_sync_status`)
//! and returns at once, so the lock is released and a live stream holds none. The
//! bridge keeps no async runtime of its own. `start_sync`/`stop_sync` are the
//! control surface that makes the stream advance (the loop is off until started).
//!
//! SWAP (inc-D-1): four flat methods mirror the §3.3 `SwapService` facade onto this
//! handle (FRB cannot mirror a nullable sub-object) — `enable_near_swap` (the §3.5
//! kill-door composition, feature `swap-near`), `swap_quote`, `swap_execute`, and the
//! live `watch_swap_status` (the `watch_sync_status` StreamSink twin — core owns the §7
//! poll pump, the bridge holds no lock for the stream's life). `swap_execute` takes the
//! whole `SwapQuote` back yet is least-authority-equivalent to `send`'s token: the core
//! claims the durable single-flight by `quote.id` first and the deposit rides the durable
//! re-blessed row, never the DTO. All swap logic + DTO/secret handling stays in
//! `crate::convert` / `crate::swap_provider`; this module is the handle + locking only.

#![deny(unsafe_code)]

use flutter_rust_bridge::frb;
use zec_wallet_core::Wallet;

use crate::api::config::{
    SyncServer, SyncServerChoice, SyncServerProbe, SyncServerStatus, WalletConfig,
};
use crate::api::error::{SwapApiError, WalletApiError};
use crate::api::payments::SendProposal;
use crate::api::state::{
    BoundedSync, DeliveryState, DialCounts, EphemeralSweepSummary, HistoryPage, InFlightSend,
    IncomingFundsEvent, MintedDiversifiedAddress, ParkedAuthorization, ParkedSend, ReclaimOutcome,
    RecoverableEphemeralFunds, ReservationPressure, SeverReport, SwapAddressCheckReport,
    SwapAddressCoverage, SyncStatus, TorState, TxSubmitResult, WalletState,
};
use crate::api::swap::{
    QuoteRequest, SwapKill, SwapOutcome, SwapProviderConfig, SwapQuote, SwapRecord, SwapStatus,
    SwapTokenList,
};
use crate::frb_generated::StreamSink;

/// FR-14 H1 — the PRODUCTION per-tier custody honesty a host renders BEFORE a
/// [WalletHandle.wipe] (and as an ambient "your keys are protected by …" badge).
/// Non-secret by construction (a tier label + two booleans). Distinct from the
/// diagnostic `selftestSeedCustody` / `CustodyReport`, which provisions
/// selftest identities and must NOT drive shipped UX — this query touches no
/// seed and unseals nothing; it reads only the measured vault tier.
///
/// **HOST UX CONTRACT (branch on the right field).** `eraseAssurance` and
/// `degraded` are SEPARATE axes — do not collapse them:
/// - Gate the WIPE confirmation on `eraseAssurance` (a hardware-held key
///   deleted vs best-effort). NOT on `degraded` — `"apple_keychain"` (raw
///   fallback) is best-effort yet reports `degraded == false`.
/// - For a "your keys are protected by hardware" BADGE, require
///   `eraseAssurance == EraseAssurance.hardwareKeyDeleted` — do NOT gate it on
///   `!degraded`, because `tier == "none"` (no keystore at all — strictly worse
///   than a software keystore) ALSO reports `degraded == false`. Treat
///   `"none"`/`"unknown"` as NOT protected.
/// - Never tell the user a deleted key is "permanently unrecoverable": no value
///   of `eraseAssurance` establishes that (ADR-0571).
pub struct CustodyDisclosure {
    /// Measured vault tier: `"strongbox"` | `"tee"` | `"software_keystore"` |
    /// `"apple_keychain"` | `"apple_secure_enclave"` | `"none"` | `"unknown"`.
    /// `"none"` = no platform key vault (a headless desktop); `"unknown"` is the
    /// forward arm for a newer core. The label is diagnostic — `eraseAssurance`
    /// and `degraded` are the load-bearing predicates a host branches on.
    pub tier: String,
    /// What [WalletHandle.wipe] does to this wallet's custody key on this
    /// device. `hardwareKeyDeleted` only for hardware-held custody (StrongBox /
    /// TEE / Apple Secure Enclave). `bestEffort` otherwise (incl.
    /// `"apple_keychain"` raw fallback, `"software_keystore"`, and `"none"`):
    /// render the honest "a forensic recovery window remains until the OS
    /// reclaims storage" disclosure (and, for full assurance, the device's own
    /// Erase-All-Content). THIS is the field the wipe confirmation branches on,
    /// not `degraded`.
    pub erase_assurance: EraseAssurance,
    /// The §4.3a degraded-custody predicate — `true` ONLY when custody is rooted
    /// in a software keystore (Android `software_keystore`); the host renders a
    /// degraded-custody warning. NOTE it is `false` for BOTH the hardware tiers
    /// AND for `"none"` (no keystore) AND `"apple_keychain"` (raw best-effort) —
    /// so `!degraded` does NOT mean "protected". A "hardware-protected" badge must
    /// require `eraseAssurance == hardwareKeyDeleted`; `degraded` only
    /// distinguishes the software-keystore tier for an explicit "weaker custody"
    /// warning.
    pub degraded: bool,
}

/// What deleting a wallet's custody key does (ADR-0571). Neither value claims
/// the key can never come back: a platform establishes that only for a key it
/// keeps rollback-resistant, which this release does not request or check.
pub enum EraseAssurance {
    /// The key was held by secure hardware (StrongBox, TEE, Secure Enclave),
    /// never usable outside it, and the wipe deletes it.
    HardwareKeyDeleted,
    /// The key was stored as data, or there is no keystore; the wipe deletes it,
    /// and a copy may remain in storage until the device reclaims it.
    BestEffort,
    /// A value from a newer core this bridge does not know. Treat it as
    /// `bestEffort`: never render more custody than you can name. Always last.
    Unknown,
}

/// An open wallet. Hold it for the session; call [close] to release the
/// single-writer lock + storage deterministically (or just drop it — the sync
/// loop is torn down on drop and the lock frees when the last reference goes,
/// but `close` is the awaitable, deterministic path). Pass it around freely;
/// the SDK runs reads ([snapshot], [currentAddress]) CONCURRENTLY (a shared
/// read lock) and serialises [close] as an EXCLUSIVE write lock — so a `close`
/// waits for in-flight reads to finish (no torn read), and reads issued after
/// `close` resolve with a typed `invalidState`, never a torn/half-closed view.
#[frb(opaque)]
pub struct WalletHandle {
    /// `None` once [close] has moved the wallet out. Private + a non-mirrorable
    /// type ⇒ FRB treats the whole handle as opaque (see the module note).
    inner: Option<Wallet>,
}

impl WalletHandle {
    /// Create a BRAND-NEW wallet — a fresh 24-word seed is generated in Rust
    /// (`OsRng`), sealed per `config.seedPersistence`, and used entirely
    /// Rust-side; the seed NEVER crosses this bridge. The wallet is then
    /// opened and returned ready to sync.
    ///
    /// Throws [WalletErrorKind.walletAlreadyExists] if a completed wallet is
    /// already provisioned at `config.dbDir` (never a silent clobber — open it
    /// instead), and the usual config-validation errors (endpoint rules, …).
    /// To RESTORE an existing wallet from its recovery phrase, use the restore
    /// entry point (a separate, key-material-bearing call).
    pub async fn create_generated(config: WalletConfig) -> Result<WalletHandle, WalletApiError> {
        let wallet = crate::convert::create_generated_wallet(config).await?;
        Ok(WalletHandle {
            inner: Some(wallet),
        })
    }

    /// Restore an existing wallet from its BIP39 recovery phrase — the ONE
    /// sanctioned INBOUND key-material crossing (spec §3.3; the outbound
    /// counterpart is `revealMnemonic`). `mnemonicWords` are
    /// the recovery words in order (a `List<String>`, the symmetric counterpart to
    /// `revealMnemonic` — never a single joined string in Dart). Pass each word
    /// LOWERCASED (and trimmed): the audited BIP39 validator does NOT case-fold, so
    /// a soft keyboard's autocapitalized first word would otherwise reject a
    /// CORRECT backup. A mis-cased/unknown word is a typed [WalletErrorKind.invalidMnemonic]
    /// carrying its INDEX (never silently a different wallet); inter-word and
    /// surrounding whitespace IS tolerated. They are
    /// BIP39-validated (checksum + word list) and turned into a sealed seed
    /// ENTIRELY Rust-side, then the wallet is provisioned at `config.dbDir` and
    /// opened. The words live in Dart memory only as long as the restore screen
    /// holds them — Dart memory cannot be zeroized (the documented §10 inbound
    /// exposure, identical framing to the outbound `revealMnemonic`). Once Rust
    /// owns the words (as a `SeedSource`), the seed + phrase ride zeroizing
    /// buffers and wipe on drop; the brief inbound word-list copy itself is
    /// un-wiped (FRB cannot marshal a zeroizing type) — the same minimal,
    /// documented residue as the outbound reveal, minimised not eliminated.
    ///
    /// `passphrase` is the OPTIONAL BIP39 passphrase (the "25th word") — pass
    /// `null` for the common case (wallets created by this SDK / Zashi / Zodl use
    /// none). It is itself part of the secret: a user who set one MUST supply it
    /// or their funds are unreachable, and a wrong/omitted passphrase silently
    /// derives a DIFFERENT, empty wallet (this is BIP39, not an SDK choice).
    ///
    /// Set `config.birthdayHeight` to the wallet's creation height (or via
    /// `estimateBirthday`) so the scan starts there; `null` floors the scan to
    /// Sapling activation — a full, slower but money-SAFE scan that finds the
    /// whole history, NEVER ~tip (which would silently skip older funds).
    ///
    /// Throws [WalletErrorKind.invalidMnemonic] (carrying the offending word
    /// INDEX, never the word) for a phrase that fails BIP39 validation;
    /// [WalletErrorKind.walletAlreadyExists] if a completed wallet is already
    /// provisioned at `config.dbDir`. That rejection is STRUCTURAL — the existing
    /// sealed seed is never overwritten regardless of whether the host probed
    /// `walletExists` first (the boot fork normally does, so a host rarely sees
    /// it). Throws [WalletErrorKind.seedMismatch] if a DIFFERENT phrase is
    /// restored over an interrupted-create remnant of another wallet; plus the
    /// usual config-validation errors (endpoint rules, …).
    pub async fn restore(
        config: WalletConfig,
        mnemonic_words: Vec<String>,
        passphrase: Option<String>,
    ) -> Result<WalletHandle, WalletApiError> {
        // SeedSource construction lives in convert.rs (off the gate-scanned api
        // surface — the words/passphrase never name a key TYPE here, only the
        // sanctioned param names, allowlisted in `ffi_surface_exposes_no_key_types`).
        let wallet = crate::convert::restore_wallet(config, mnemonic_words, passphrase).await?;
        Ok(WalletHandle {
            inner: Some(wallet),
        })
    }

    /// Open an EXISTING wallet at `config.dbDir`.
    ///
    /// Throws [WalletErrorKind.notFound] if nothing is provisioned there,
    /// [WalletErrorKind.networkMismatch] if `config.network` disagrees with
    /// the stored wallet, [WalletErrorKind.walletAlreadyOpen] if another
    /// instance already holds it, and [WalletErrorKind.provisioningIncomplete]
    /// for an interrupted create — open does NOT repair a remnant; `create`
    /// resumes the repair from the sealed seed, so the host's boot fork routes a
    /// remnant (`walletExists` == false) to create, never here.
    pub async fn open(config: WalletConfig) -> Result<WalletHandle, WalletApiError> {
        let wallet = crate::convert::open_wallet(config).await?;
        Ok(WalletHandle {
            inner: Some(wallet),
        })
    }

    /// Create a WATCH-ONLY wallet from an exported viewing key (#397, spec
    /// §3.7 D2 / ADR-0538 — the consumer half of `exportUfvk`). Full read
    /// path (sync, balance, history, receive addresses); every spend-class
    /// surface — send, shield, swap, `revealMnemonic`, rescan — throws the
    /// STRUCTURAL [WalletErrorKind.watchOnly] (nothing to retry: this wallet
    /// holds no spending keys, by design). Reopen later with the plain
    /// `open`; a pre-#397 binary refuses the store typed instead of
    /// half-opening it.
    ///
    /// `ufvk` must be the standard unified encoding for `config.network`
    /// (`uview1…` / `uviewtest1…`): garbage, truncation and a wrong-network
    /// key all throw [WalletErrorKind.invalidViewingKey] BEFORE anything is
    /// written. `birthdayHeight` is REQUIRED — use the exporting wallet's
    /// birthday (or `estimateBirthday` from a date); too high silently hides
    /// older history (the same contract as a seed restore), too low only
    /// scans longer. `config.seedPersistence` is ignored (there is no seed).
    pub async fn create_watch_only(
        config: WalletConfig,
        ufvk: String,
        birthday_height: u32,
    ) -> Result<WalletHandle, WalletApiError> {
        let wallet =
            crate::convert::create_watch_only_wallet(config, ufvk, birthday_height).await?;
        Ok(WalletHandle {
            inner: Some(wallet),
        })
    }

    /// Provision a NEW host-custodied-seed wallet at `config.dbDir` (FR-15), pulling
    /// the seed from the C-ABI seed port the HOST registered (in its own native
    /// library) via `zec_wallet_register_seed_port`. The seed crosses
    /// native-to-native from the host lib into this one and NEVER through Dart — use
    /// this INSTEAD of `createGenerated`/`restore` for the keys-in-host integration
    /// (Relim ADR-0031): there is no mnemonic and no Dart-side seed. The SDK PINS
    /// `config.seedPersistence` to `none` (a host-custodied seed is never sealed at
    /// rest) — any other value is enforced to `none`, not an error.
    ///
    /// The returned handle ALREADY has the port attached (account import + signing
    /// pull it on demand), so no host-side close/reopen dance is needed.
    ///
    /// `freshlyGenerated` (FR-24, default `false`/omitted): pass `true` ONLY when
    /// the supplied seed was minted BRAND-NEW in this same onboarding flow — no
    /// address of it ever derived or shared, so it can have NO chain history — and
    /// has NEVER existed anywhere else before. (The load-bearing property is
    /// "no prior chain history", NOT elapsed seconds — a fresh seed created a few
    /// minutes before this call, after a backup/consent step, still qualifies.
    /// A restore, migration, reinstall, or any re-derivation of an identity that
    /// existed before ⇒ pass `false`; it is a restore.)
    ///
    /// With `true` the account imports AT CREATE, fully offline: the receive address
    /// works from second zero with no network, and the host may CLEAR its staged seed
    /// the moment this returns (no create→first-sync staging window). TWO narrow
    /// exceptions keep the import lazy despite the flag — keep the staging contract
    /// there: a `config.birthday` ABOVE the bundled checkpoint tail (the config-typo
    /// guard needs a live tip; don't combine the flag with an above-tail birthday if
    /// you clear at return), and a retry over a create that previously crashed
    /// BEFORE its stamp landed (first sync then imports via the port — the
    /// re-stage contract; typed [WalletErrorKind.seedRequired] if nothing is
    /// staged, never fund loss).
    ///
    /// ⚠️ THE FLAG IS A MONEY-VISIBILITY ATTESTATION: marking a seed with prior chain
    /// history fresh pins the wallet's scan floor at ~now — pre-existing funds and
    /// swap refunds become INVISIBLE (recoverable by `rescanFrom` with an EXPLICIT
    /// lower height through the registered seed port, or by wiping and re-creating
    /// with `false`; a `rescanFrom(null)` re-floors at the creation stamp and does
    /// NOT go deeper).
    ///
    /// When in doubt pass `false` — always money-SAFE. Its costs are a full history
    /// scan at first sync, no offline receive address until that sync completes (the
    /// FR-24 gap itself: `currentAddress` is typed [WalletErrorKind.seedRequired]
    /// until import), and the FR-12 staging window staying open (the host stages the
    /// seed for this provisioning scope and clears it once first sync completes). A
    /// configured ≤-bundle-tail `config.birthday` also imports at create and allows
    /// clearing at return, for either flag value.
    ///
    /// Throws [WalletErrorKind.seedRequired] if no seed port is registered or the host
    /// supplies none; [WalletErrorKind.walletAlreadyExists] on a completed store — if
    /// a previous freshly-generated create FAILED after provisioning, recover via
    /// `openWithHostSeed`, never re-create (note: the surviving creation stamp keeps
    /// the ~creation scan floor on that reopen — for a wallet MIS-attested fresh,
    /// deeper history needs `rescanFrom` with an explicit lower height or the wipe +
    /// re-create-with-`false` path above); the usual config-validation errors
    /// (endpoint rules, …).
    pub async fn create_with_host_seed(
        config: WalletConfig,
        freshly_generated: Option<bool>,
    ) -> Result<WalletHandle, WalletApiError> {
        let wallet = crate::convert::create_wallet_with_host_seed(
            config,
            freshly_generated.unwrap_or(false),
        )
        .await?;
        Ok(WalletHandle {
            inner: Some(wallet),
        })
    }

    /// Open an EXISTING host-custodied-seed wallet at `config.dbDir` (FR-15),
    /// attaching the registered C-ABI seed port so signing + fresh-refund-address
    /// derivation can pull the seed on demand. For a wallet whose account is ALREADY
    /// imported (the steady state), balance, history, and the receive address need
    /// NO seed and never call the port; the host stages narrowly and clears per the
    /// FR-12 contract (take-once for a send). This only transports the pull
    /// native-to-native, never through Dart.
    ///
    /// EXCEPTION — a wallet reopened BEFORE its account import: a restore-shaped
    /// `createWithHostSeed` create whose first sync hasn't completed, OR the
    /// failed-freshly-generated-create recovery `createWithHostSeed` routes here.
    /// There, first sync imports the account THROUGH the port — keep the FR-12
    /// staging contract until it completes, and until then `currentAddress` is a
    /// typed [WalletErrorKind.seedRequired] (not a hang, and never fund loss).
    ///
    /// Throws [WalletErrorKind.seedRequired] if no port is registered;
    /// [WalletErrorKind.notFound]/`networkMismatch`/`walletAlreadyOpen` as `open`.
    pub async fn open_with_host_seed(config: WalletConfig) -> Result<WalletHandle, WalletApiError> {
        let wallet = crate::convert::open_wallet_with_host_seed(config).await?;
        Ok(WalletHandle {
            inner: Some(wallet),
        })
    }

    /// Cheap probe: is a COMPLETED, openable wallet provisioned at
    /// `config.dbDir`? Drives the host's boot fork — `false` → offer create,
    /// `true` → `open` it (then gate on backup confirmation). It is LOCK-FREE
    /// and does NOT open the wallet: it reads only the on-disk provisioning
    /// marker, so it never contends the single-writer lock the subsequent `open`
    /// needs (an `open`-and-catch-`notFound` would, and then break that `open`).
    ///
    /// `true` ONLY for a COMPLETED wallet. An interrupted-create remnant reads
    /// `false` and routes to `createGenerated`, which RESUMES its repair from
    /// the sealed seed (never clobbers) — the only path that finishes it, since
    /// `open` cannot repair a remnant (it throws
    /// [WalletErrorKind.provisioningIncomplete], having no seed source to
    /// resume). A corrupt store (a completion marker with no manifest) throws
    /// [WalletErrorKind.storeCorrupt] — the honest recover-from-phrase path,
    /// NEVER silently reported as "no wallet". Unlike `createGenerated`/`open`
    /// this validates ONLY `dbDir` (existence is a path question), not the
    /// endpoint/Tor policy.
    ///
    /// PLACEMENT (a deliberate, reversible choice): this is a `Self`-less static
    /// even though it returns `bool`, not a handle. It lives on `WalletHandle`
    /// because it is the THIRD boot-fork entry — the host drives
    /// `walletExists → open`/`createGenerated` as one sequence — so grouping the
    /// trio on the handle the provisioner owns keeps that coupling visible in
    /// Dart (`WalletHandle.walletExists` → `WalletHandle.open`). It is NOT the
    /// `validate_wallet_config` case (a settings-form pre-check with no boot/open
    /// intent — correctly a free function). If a host prefers a free function
    /// here, the move is mechanical (convert/core unchanged). Manager-flagged.
    pub async fn wallet_exists(config: WalletConfig) -> Result<bool, WalletApiError> {
        crate::convert::wallet_exists(config).await
    }

    /// Delete the wallet at `config.dbDir` (FR-14) — the host's "delete wallet" /
    /// panic-wipe primitive. Deletes the keychain wrap key FIRST (so on any crash
    /// point this key store can no longer open the on-disk seals or the DB
    /// ciphertext; how strong that is is `custodyDisclosure`'s `eraseAssurance`,
    /// ADR-0571), then removes the data directory. The seed NEVER crosses this bridge — only the
    /// config does.
    ///
    /// A `Self`-less static (like `walletExists`) because a wipe runs with no live
    /// handle — typically AFTER `close` (panic-wipe runs post-teardown). It
    /// **refuses while an instance is open** ([WalletErrorKind.walletOpen]): close
    /// the handle first, then wipe. Idempotent — a no-wallet `dbDir` is a no-op,
    /// and an interrupted wipe auto-converges on a plain re-run.
    ///
    /// **Fail-closed money safety:** if `config.dbDir` does not byte-match the
    /// create/open path, the per-wallet keychain namespace is mis-derived; rather
    /// than delete the files while the real wrap key survives, a non-empty store
    /// whose custody sever removed nothing fails CLOSED
    /// ([WalletErrorKind.keystoreInconsistent], files untouched). Pass the SAME
    /// `config` you opened with. The rare already-gone-item case is the
    /// `wipeForce` escape hatch.
    ///
    /// **Do NOT auto-escalate a `keystoreInconsistent` into `wipeForce`.** That
    /// would defeat the guard — `keystoreInconsistent` is the same signal a
    /// mis-derived `dbDir` raises, and forcing past it can downgrade a true
    /// crypto-erase to best-effort (deleting files while a real wrap key survives).
    /// Reach for `wipeForce` only deliberately, for a wallet you are CERTAIN is
    /// yours whose custody item is genuinely gone.
    ///
    /// **Erase strength is per-device** — render `custodyDisclosure` BEFORE this
    /// to tell the user honestly whether deletion removes a key held by secure
    /// hardware or is best-effort removal. Neither is a proven permanent erase
    /// (ADR-0571).
    ///
    /// **The device log goes Off.** Any call, a failed one included, first
    /// closes the `watchDeviceLog` stream and sets the device log `Off`. It
    /// stays Off until the host re-arms it with `setDeviceLog` and
    /// `watchDeviceLog`. A host must not re-arm on a duress path.
    pub async fn wipe(config: WalletConfig) -> Result<(), WalletApiError> {
        crate::convert::wipe_wallet(config, false).await
    }

    /// Force-complete a `wipe` whose keychain custody item is ALREADY GONE — the
    /// escape hatch for the rare states where a plain `wipe` returns
    /// [WalletErrorKind.keystoreInconsistent] (a manually-deleted item, a
    /// pre-namespacing bare item, an iOS `ThisDeviceOnly` item that didn't migrate
    /// while `dbDir` did). It SKIPS the verify-real-sever guard, so the caller MUST
    /// be certain `config` matches the wallet's create/open config — a wrong config
    /// under force would delete the wrong files. Use ONLY after a plain `wipe`
    /// returned `keystoreInconsistent` on a wallet you are sure is yours.
    ///
    /// **HOST CONTRACT — run with the device UNLOCKED.** On Apple Secure-Enclave
    /// custody a locked-device key can be invisible to the purge; forcing past the
    /// guard while locked could delete the seals while the wrap key survives,
    /// silently downgrading the crypto-erase. Ensure protected data is available
    /// (device unlocked) before calling.
    ///
    /// **The device log goes Off**, as for `wipe`: any call, a failed one
    /// included, first closes the `watchDeviceLog` stream and sets the device
    /// log `Off` until the host re-arms it.
    pub async fn wipe_force(config: WalletConfig) -> Result<(), WalletApiError> {
        crate::convert::wipe_wallet(config, true).await
    }

    /// **The duress force-sever** (FR-53): make the wallet at `config.dbDir`
    /// unrecoverable within `deadlineMs`, EVEN WHILE something still holds it
    /// open (a handle never closed, a straggler past `close`, a rescan or
    /// server switch mid-rebuild, an open in flight, another process) — the
    /// case where `wipe` refuses with [WalletErrorKind.walletOpen].
    ///
    /// Nothing holds the wallet → it is the ordinary `wipe` (key store first,
    /// then the files): `files: removed`, `holder: none`. Something holds it →
    /// the key-store custody is severed anyway, NO file is deleted
    /// (`files: leftForHost`), and every instance of the wallet in this
    /// process is poisoned: each later call answers `invalidState` (`wiped`),
    /// and a broadcast in flight does not go out. On a locked key store the
    /// sever deletes blind and answers `severedUnproven` with `leftForHost`,
    /// whether or not anything held the wallet.
    ///
    /// `deadlineMs` is whole milliseconds, clamped to `[0, 4000]` (the SDK's
    /// key-store budget for a wipe; a longer deadline gains nothing, and a
    /// small one is kept as given). It is a HARD bound on the WHOLE call: the
    /// answer comes by `deadlineMs + severAnswerGraceMs()` (250 ms) whatever
    /// stalls. An overrun answers the DISTINCT `notSevered(stillRunning)`,
    /// `files: stillInUse`, `holder: unknown`. The SDK's own sever cannot be
    /// cancelled and may still be deleting in `dbDir`, so on `stillRunning`
    /// do NOT purge or re-create at that `dbDir`. Call again with a backoff
    /// until the cause is not `stillRunning`, or exit the process. A call made
    /// while that sever runs answers `stillRunning` at once without reaching
    /// the store, so it never double-purges, and each overrun parks one
    /// uncancellable blocking task, so never call it in a tight loop. While it
    /// runs, every other door at that `dbDir` refuses before any work: `open`,
    /// the creates and `restore` answer `walletAlreadyOpen`, `wipe` and
    /// `wipeForce` answer `walletOpen`. "That `dbDir`" is the directory's real
    /// path, so a symlinked or `..` spelling of it counts (ADR-0566). The guard
    /// fails CLOSED: during a filesystem stall while a sever is in flight, those
    /// doors may refuse at any directory, and never hang. It is a check at the
    /// door, not a hold: a sever that STARTS after a door has passed it runs
    /// alongside that door's work, under the SDK's store lock and tombstone. `0`
    /// makes no key-store call and answers `notSevered(pastDeadline)`. A
    /// `dbDir` that never held a wallet, or a call after the sever ended,
    /// answers `alreadyGone`. The call never throws for a custody outcome. It
    /// throws [WalletApiError] only for a bad `dbDir` (the same door as `wipe`:
    /// pass the SAME absolute `dbDir` you opened with) or an I/O fault. It
    /// consumes ONLY `config.dbDir`. Like `wipe`, it sets the device log `Off`
    /// and closes the `watchDeviceLog` stream before anything else.
    ///
    /// **The host's four obligations:**
    ///
    /// 1. **Read `severed`, not the absence of an exception.** `severed` is
    ///    PROVEN. `severedUnproven(countUnreadable)`: the phone was locked; the
    ///    delete was issued but cannot be confirmed — a `wipe` after the next
    ///    unlock confirms it. `notSevered(timeout | busy)`: the key store was
    ///    occupied, by a custody write already in flight OR by ANY other
    ///    key-store call in this process (another open wallet's, or
    ///    `selftestSeedCustody`'s) — all of them share one worker. Call
    ///    `severCustody` again: no new custody write can land meanwhile, and
    ///    the retry succeeds once that other call returns, so a host keeping
    ///    several wallets open, or running the selftest, should not expect
    ///    `severed` within the first deadline. `notSevered(stillRunning)`: the
    ///    SDK stopped waiting, but its own sever still runs; call again (with a
    ///    backoff) until it is not. Any other `notSevered` (and `unknown`): the
    ///    custody may be live.
    /// 2. **Purge ONLY on `files: leftForHost`: you delete the directory
    ///    yourself**, after this call has answered. On `stillInUse`, NEVER:
    ///    the SDK is still working in it. On `unknown`, call again.
    /// 3. **Severed means the key that unlocks the files on disk is deleted from
    ///    the key store** (how strongly is `custodyDisclosure`'s
    ///    `eraseAssurance`), not that the live instance's memory is clean: its
    ///    seed and database key stay in RAM until the holder drops. A host that
    ///    must end that exits the process.
    /// 4. **Before creating a wallet at the same `dbDir` again in this
    ///    process**, run a plain `wipe` of it. Until then `create`/`open` there
    ///    answer `invalidState` (`wiped`). A process restart also clears that;
    ///    after a restart, an `open` at a severed directory you did NOT delete
    ///    answers the store's custody error, not `wiped` — the plain `wipe` is
    ///    still the remedy.
    pub async fn sever_custody(
        config: WalletConfig,
        deadline_ms: u32,
    ) -> Result<SeverReport, WalletApiError> {
        crate::convert::sever_custody(config, deadline_ms).await
    }

    /// The PRODUCTION custody disclosure (FR-14 H1) for the wallet at
    /// `config.dbDir` — render an HONEST pre-`wipe` confirmation (a hardware-held
    /// key deleted vs best-effort removal) and an ambient custody badge from it.
    /// An ACTIVE probe of the measured vault tier; reads NO seed and unseals
    /// nothing. A `Self`-less static so it works before/independent of an open
    /// handle. `VaultAbsent` (headless desktop) ⇒ `tier: "none"`,
    /// `eraseAssurance: bestEffort`.
    ///
    /// Distinct from the diagnostic `selftestSeedCustody`: that one provisions
    /// selftest identities and must not drive shipped UX; THIS is the shippable
    /// query.
    pub async fn custody_disclosure(
        config: WalletConfig,
    ) -> Result<CustodyDisclosure, WalletApiError> {
        crate::convert::custody_disclosure(config).await
    }

    /// The on-resume cold snapshot (spec §3.3): everything a UI needs to
    /// re-render after a background gap — balance, sync status, Tor state,
    /// chain tip, and a monotonic `seq` that also stamps every live stream
    /// item (so a stale stream event can never beat a fresher snapshot in a UI
    /// race: keep the larger `seq`). Cheap; safe to call on resume before
    /// re-subscribing to the live streams.
    ///
    /// `lastSynced` is the PERSISTED stamp of the last completed sync pass
    /// (`{height, at}` — written before each `UpToDate`, surviving relaunches),
    /// so a cold resume renders "as of block H, time" immediately. `null`
    /// before the first completed pass and after a rescan (the rebuild resets
    /// it — a stale stamp would pair the old height/time with a rebuilding
    /// balance); always the recorded truth, never a fabricated age.
    pub async fn snapshot(&self) -> Result<WalletState, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let state = wallet.snapshot().await.map_err(WalletApiError::from)?;
        Ok(state.into())
    }

    /// THE ONE SANCTIONED UFVK EGRESS (#397, spec §3.7 D1 / ADR-0538; §5.4
    /// HARD-D re-cast: "never leaves the device SILENTLY"). Returns the
    /// wallet's full viewing key in the standard unified encoding.
    ///
    /// **HOST CONTRACT (LOUD):** the returned string grants TOTAL HISTORY
    /// VISIBILITY — every incoming AND outgoing payment, past and future —
    /// to anyone who ever holds it. It cannot spend and cannot reach the
    /// seed. Gate it behind a deliberate user action at the SAME bar as the
    /// backup-phrase reveal (re-auth + screenshot protection + the §3.7 D9
    /// warning copy — the reference UI export screen is the normative
    /// example); never call it implicitly; never log or persist the value
    /// beyond the user's explicit share. The only un-share is moving funds
    /// to a new wallet. Works at every custody tier including watch-only
    /// (identity operation — the host supplied the key).
    pub async fn export_ufvk(&self) -> Result<String, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        wallet.export_ufvk().await.map_err(WalletApiError::from)
    }

    /// Is this a WATCH-ONLY wallet (#397 §3.7 D4/D5)? SYNC + lock-free (the
    /// kind is immutable after open). The reference UI keys its chrome on
    /// this — the watch-only badge, hidden Send/Shield/Swap affordances, the
    /// no-backup security screen; the typed [WalletErrorKind.watchOnly]
    /// refusals stand regardless, so a host ignoring it only ever sees
    /// honest errors.
    pub fn is_watch_only(&self) -> Result<bool, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        Ok(wallet.is_watch_only())
    }

    /// The wallet's transaction history (FR-1 — the activity-history list), newest
    /// first and paginated by an OPAQUE keyset cursor. Pending (unmined) rows sort
    /// ABOVE confirmed ones; `after = None` is the first page, `after = Some(token)`
    /// the page after the cursor (the [`HistoryPage::next_cursor`] from the previous
    /// call — passed back VERBATIM; never parse it); `limit` caps the page. The keyset
    /// cursor (full `(height, tx_index, txid)` sort key) means a page boundary never
    /// drops a same-height row nor strands the confirmed history behind a full page of
    /// pending txs. Each row is a typed [`TxSummary`](crate::api::state::TxSummary) (txid hex in block-explorer
    /// order, SIGNED net amount, status, fee, memo flag) — no key material, amounts
    /// stay typed. Reads librustzcash's audited `v_transactions` view off the engine
    /// lock, so it never blocks sync.
    pub async fn transactions(
        &self,
        limit: u32,
        after: Option<String>,
    ) -> Result<HistoryPage, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let page = wallet
            .transactions(limit, after)
            .await
            .map_err(WalletApiError::from)?;
        Ok(page.into())
    }

    /// FR-27 — the MACHINE-MEMO bytes of one transaction, scoped to the
    /// prefixes registered on [WalletConfig.machineMemoPrefixes]. Returns them
    /// in output order; `txidHex` is the `txidHex` a history row gave you,
    /// passed back verbatim.
    ///
    /// **THESE BYTES ARE ATTACKER-CONTROLLABLE.** They came off a public chain,
    /// and anyone can put anything in a memo attached to a payment they send
    /// you. Parse them with something that fails closed — validate your own
    /// framing before you use any of it, reject trailing bytes and a length
    /// that lies, and treat an unknown value as hostile rather than as a
    /// default. This SDK does none of that for you, by design: it never parses
    /// the envelope, so its layout stays yours rather than becoming a format
    /// this SDK would have to version.
    ///
    /// **PREFIX MATCHING IS NOT A SECURITY BOUNDARY.** Forging a prefix costs
    /// an attacker nothing — your magic bytes are visible on-chain to anyone
    /// who looks — so the scope cuts the VOLUME reaching your parser, never the
    /// INTENT behind what arrives. Your parser is on the hostile path by
    /// default, not as defence in depth. A prefix also does not mean "written
    /// by this device": the memos worth reading were written by whoever paid
    /// you.
    ///
    /// Scope, exactly: `Arbitrary` (`0xFF`) memos only — a ZIP-302 `Reserved`
    /// memo NEVER returns bytes here and a text memo has its own lane — each
    /// bounded at 511 bytes, and only for a transaction this wallet can
    /// decrypt (the same entitlement the history read already has: outputs
    /// this wallet sent or received). Memos arrive with transaction
    /// ENHANCEMENT, not compact scan, so a freshly received transaction
    /// returns nothing until its enhancement pass has run.
    ///
    /// WIRE REALITY, and a host that ignores it will read padding as data: the
    /// `0xFF` field carries no length framing, so a memo shorter than 511 bytes
    /// was zero-padded on the wire and comes back as the full 511. Frame your
    /// own length INSIDE the bytes.
    ///
    /// WHAT THE LIST POSITION IS NOT: it is not an output index (empty and
    /// out-of-scope memos are filtered out before you see it), and this verb
    /// alone cannot tell you whether a memo was one this wallet WROTE or one a
    /// payer wrote to it — both are readable and both are returned. Join
    /// against the history row if you need direction.
    ///
    /// One output whose memo the audited ZIP-302 classifier rejects does NOT
    /// deny the rest: that row is skipped and the readable memos still come
    /// back. The sender of a transaction picks every byte of every memo in it,
    /// so a strict read would let a hostile counterparty suppress the envelope
    /// they were paying you with.
    ///
    /// Throws [WalletErrorKind.machineMemoScopeInvalid] when no prefix is
    /// registered — the verb is opt-in and CLOSED by default, and an empty list
    /// would read as "this transaction carries nothing" over a memo that is
    /// there. Throws [WalletErrorKind.txidInvalid] for a `txidHex` that is not
    /// 64 hex characters.
    pub async fn machine_memos(&self, txid_hex: String) -> Result<Vec<Vec<u8>>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        crate::convert::machine_memos(wallet, &txid_hex).await
    }

    /// The wallet's DELIVERY OBLIGATION for one transaction it created — the
    /// same value a history row carries on [TxSummary.delivery], for a caller
    /// that holds a `txidHex` (the one `send` returned in its
    /// [TxSubmitResult]s, passed back verbatim). Read it right after a `send`
    /// whose result was not all-success: `null` or anything but
    /// [DeliveryState.retryPending] means the wallet is NOT promising to send
    /// it on a later sync, so do not say so. `null` for a transaction this
    /// wallet did not create, one that has expired (its `TxStatus` says so) or
    /// one it holds no row for. Reads the aux connection off the engine lock,
    /// so it never blocks sync. Throws [WalletErrorKind.txidInvalid] for a
    /// `txidHex` that is not 64 hex characters; [WalletErrorKind.invalidState]
    /// on a closed handle.
    pub async fn delivery_state(
        &self,
        txid_hex: String,
    ) -> Result<Option<DeliveryState>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        crate::convert::delivery_state(wallet, &txid_hex).await
    }

    /// Funds recoverable on wallet-controlled *one-time* (ephemeral) transparent addresses — a TEX
    /// transfer whose forwarding step expired, or an exchange that returned a deposit to that
    /// single-use address. A read-only PULL the host calls on demand (e.g. to render a "recover" row
    /// on the balance). Each entry is amount + welded reorg-finality
    /// ([`RecoverableEphemeralFunds`]); the one-time
    /// address never crosses the bridge. **PRESENTATION: the amount is a SUBSET of the displayed
    /// balance — render it as part of the balance, NEVER as additional funds.** Live now that TEX
    /// two-step sends run in production (gate-removal, 2e-2b-v-5); empty on a wallet that has made no
    /// TEX send (no one-time address has ever held a return).
    pub async fn recoverable_ephemeral_funds(
        &self,
    ) -> Result<Vec<RecoverableEphemeralFunds>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let funds = wallet.list_stranded().await.map_err(WalletApiError::from)?;
        Ok(funds
            .into_iter()
            .map(RecoverableEphemeralFunds::from)
            .collect())
    }

    /// MANUALLY recover funds stranded on wallet-controlled *one-time* (ephemeral) transparent
    /// addresses into the wallet's own SHIELDED balance — the user-discoverable recovery for the funds
    /// `recoverableEphemeralFunds` surfaces, PLUS late exchange returns / a 2nd deposit / aggregate dust
    /// the automatic surface cannot see (it raw-enumerates each one-time address's on-chain UTXOs).
    /// MONEY-MOVING: it signs + broadcasts one consolidating transaction per funded address (each over
    /// its own circuit, never co-spent — so a recovery never links the one-time-address set on-chain).
    ///
    /// Returns an [EphemeralSweepSummary] (COUNTS-only; the one-time addresses NEVER cross the bridge).
    /// `recoveredZat` is the AGGREGATE that landed in the shielded balance, counted ONLY on endpoint
    /// acceptance (idempotent on re-run — the engine excludes already-spent UTXOs, so this can never
    /// double-spend). `failed > 0` ⇒ some funds stay on-chain + re-runnable (a flaky link, NOT a loss);
    /// `truncated > 0` ⇒ a heavy wallet hit the per-invocation cap — run it again. Throws
    /// [WalletErrorKind.seedRequired] when the wallet cannot sign (no seed/credential available),
    /// [WalletErrorKind.invalidState] on a closed handle. Live now that TEX two-step sends run in
    /// production (gate-removal, 2e-2b-v-5); on a wallet that has made no TEX send no one-time address is
    /// reserved ⇒ an empty no-op.
    pub async fn sweep_ephemeral_funds(&self) -> Result<EphemeralSweepSummary, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let summary = wallet
            .sweep_ephemeral_funds()
            .await
            .map_err(WalletApiError::from)?;
        Ok(summary.into())
    }

    /// RECLAIM a one-time-address (TEX) send window bricked by LEAKED reservations — sends that
    /// reserved a one-time address but never confirmed, which the wallet cannot free on its own
    /// (#315). Self-mints a small amount from your SHIELDED balance to the highest provably-abandoned
    /// one-time address; mining it reopens the WHOLE window at once. MONEY-MOVING + EXPLICIT: gate it
    /// behind the #327 authorizer with an honest-cost disclosure (the mint + a later recovery are TWO
    /// transactions, ~4 network fees; the minted principal returns to your wallet).
    ///
    /// Returns a [ReclaimOutcome]: `NothingToReclaim` when no reservation is provably abandoned (the
    /// window is transient — never mint on a maybe-live one, which could cancel a real payment);
    /// `Minted` on a signed+accepted mint (INITIATED — the window reopens once it confirms, and the
    /// moved amount returns via `sweepEphemeralFunds`); `NotBroadcast` on a transport miss (money-safe
    /// — it expires and frees the funds; try again). It unblocks the WINDOW only — it NEVER re-sends a
    /// paused/queued payment (no double-pay). Throws [WalletErrorKind.seedRequired] with no signing
    /// key, [WalletErrorKind.insufficientFunds] when the shielded balance can't fund the mint,
    /// [WalletErrorKind.invalidState] on a closed handle. Live now that TEX two-step sends run in
    /// production (gate-removal, 2e-2b-v-5); a wallet that has made no TEX send has nothing to reclaim.
    ///
    /// ⚠ HOST GATING (money-relevant): surface this ONLY behind the #327 authorizer + the honest-cost
    /// disclosure, and ONLY when a send is actually parked/paused. The SDK cannot tell a bricked
    /// window from a healthy one (the engine exposes no occupancy read — the owed upstream FR), so a
    /// call on a wallet with only OLD completed one-time-address sends still mints uselessly (a few
    /// network fees; the principal returns via `sweepEphemeralFunds`; no benefit). Fee-waste only,
    /// never fund-loss — but do NOT auto-invoke, loop, or offer it without the parked/paused trigger.
    pub async fn reclaim_ephemeral_slots(&self) -> Result<ReclaimOutcome, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let outcome = wallet
            .reclaim_ephemeral_slots()
            .await
            .map_err(WalletApiError::from)?;
        Ok(outcome.into())
    }

    /// #390 — "Check older swap addresses": widen the range the wallet watches so the
    /// normal sync surfaces older swap deposits/refunds a SEED-ONLY RESTORE left unchecked
    /// (a wallet with a long swap history restored from its phrase). It does NOT itself
    /// find funds — it WIDENS, then any older swap money appears in the balance over the
    /// next minutes of sync. Returns a counts-only [SwapAddressCheckReport] — render
    /// "checking older swap addresses as your wallet syncs; anything found appears in your
    /// balance", NEVER "found X". Rerunnable (each accepted run goes deeper).
    ///
    /// SURFACE IT in an overflow entry ADJACENT to Rescan — but NOT behind the rescan
    /// package-custody gate: this runs entirely over the live session (the one-time-address
    /// check class). Throws [WalletErrorKind.swapAddressCheckRefused] (branch on
    /// `reason`): `swapDisabled` (swap is off — retry when back), `checkOutstanding` (a
    /// prior check is still running — one per settlement window, try later);
    /// [WalletErrorKind.invalidState] on a closed handle. Being elective, prefer to run it
    /// when the transport is healthy (not degraded-Tor). It moves NO money and needs no
    /// authorizer.
    pub async fn check_older_swap_addresses(
        &self,
    ) -> Result<SwapAddressCheckReport, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let report = wallet
            .check_older_swap_addresses()
            .await
            .map_err(WalletApiError::from)?;
        Ok(report.into())
    }

    /// #390 — the render-only coverage read for the "Check older swap addresses" sheet:
    /// how far the wallet has already checked, for the coverage line and the
    /// Check/Check-deeper affordance. COUNTS ONLY — no address or index crosses. Read-only,
    /// side-effect-free; safe to poll while the sheet is open. Throws
    /// [WalletErrorKind.invalidState] on a closed handle.
    pub async fn swap_address_check_coverage(&self) -> Result<SwapAddressCoverage, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let coverage = wallet
            .swap_address_check_coverage()
            .await
            .map_err(WalletApiError::from)?;
        Ok(coverage.into())
    }

    /// EVERY queued send sitting in `Queued` that has NOT completed yet — a one-time-address (TEX)
    /// two-step awaiting its drain pass / ceiling-parked, or (since #331) a plain single-step send
    /// waiting in the offline queue; the DTO `kind` says which (presentation-only). They have NO
    /// on-chain transaction yet (INVISIBLE in the activity / transaction list), so this surfaces
    /// them "saved & pending" instead of silence — across an app relaunch it is the ONLY place the
    /// committed spend exists (hidden, a queued send is a DOUBLE-PAY window). Each is amount + id +
    /// kind + `paused` ([`ParkedSend`]); the recipient address NEVER
    /// crosses the bridge. A read-only PULL the host calls on demand; cause-agnostic copy (never
    /// "blocked"/"stuck"). A HEALTHY row auto-broadcasts once it can drain — present it per the
    /// [`ParkedSend`] DOUBLE-PAY CAUTION (never invite a re-send) and EARMARK its amount over the
    /// balance (it stays in spendable). A `paused` row does NOT auto-broadcast (#315 — the wallet
    /// gave up auto-retrying it; it waits for `retryParkedSend` or `cancelParkedSend` and MUST be
    /// phrased as paused, never "will send when ready"). To DISCARD either, pass its
    /// `(id, createdAt)` to `cancelParkedSend` (the SDK cancel — the SAFE counter-affordance, landed v-4a,
    /// kind-agnostic; the host UI button/confirm is v-4b). Throws [WalletErrorKind.invalidState] on a
    /// closed handle.
    pub async fn list_parked_sends(&self) -> Result<Vec<ParkedSend>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let parked = wallet
            .list_parked_sends()
            .await
            .map_err(WalletApiError::from)?;
        Ok(parked.into_iter().map(ParkedSend::from).collect())
    }

    /// The IN-FLIGHT two-step (TEX/ZIP-320) sends: first leg signed + broadcast, send not yet
    /// complete — the user's money is IN MOTION through a wallet-controlled one-time address. The
    /// host renders a DURABLE "on its way — don't send it again" cue from this list (see
    /// [`InFlightSend`] for the lifecycle + the DOUBLE-PAY
    /// CAUTION): the post-send result screen's caution is dismissible, and without this surface an
    /// in-flight two-step reads as an ordinary pending send exactly when a worried user is most
    /// tempted to re-pay. DISJOINT from `listParkedSends` (parked = queued, nothing signed yet;
    /// in-flight = signed + broadcast) — a send is in at most one. Read-only PULL, cheap + LOCAL
    /// (a SQLite read; no network, no money movement); call it on the same edges as
    /// `listParkedSends` (sync ticks / resume / after a send completes). Empty when nothing is
    /// mid-flight — the common case. Amount-only (§5.4; no recipient/txid/address). Throws
    /// [WalletErrorKind.invalidState] on a closed handle. HOST CONTRACT on a THROW: the call
    /// fails LOUD on a corrupt store row (fail-closed — a broken row must never silently hide a
    /// possibly-in-motion send); prefer SURFACING that failure over hiding the cue, because a
    /// swallowed error removes the double-pay caution for EVERY in-flight send at once (a host
    /// may fall back to hiding only because the pending tx stays visible in `transactions`).
    pub async fn list_in_flight_sends(&self) -> Result<Vec<InFlightSend>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let in_flight = wallet
            .list_in_flight_sends()
            .await
            .map_err(WalletApiError::from)?;
        Ok(in_flight.into_iter().map(InFlightSend::from).collect())
    }

    /// CANCEL a parked (queued) send the host listed via `listParkedSends` — the user-facing escape
    /// hatch to discard a TEX send waiting for the multi-step path (which would otherwise AUTO-BROADCAST
    /// once that path is enabled — see the [`ParkedSend`] DOUBLE-PAY
    /// CAUTION; cancel is the SAFE counter-affordance, never a re-send). Pass BOTH the `id` AND the
    /// `createdAt` from the [`ParkedSend`] the user is cancelling. Returns `true` if it was removed,
    /// `false` if it was already gone / began sending (post-multi-step-enable) / its rowid was reused
    /// (idempotent). On `false`, re-read `listParkedSends` — but do NOT present it as definitively
    /// "cancelled" nor invite a re-send: a send that began sending has LEFT the parked surface, so the
    /// payment may be IN-FLIGHT (direct the user to the activity list; a re-send risks a DOUBLE-PAY).
    /// IRREVERSIBLE + UNCONFIRMED — it discards the committed intent immediately; the host MUST confirm
    /// before calling (the confirm dialog is v-4b). MONEY-SAFE: only a still-`Queued` intent is deleted
    /// (no note reserved, nothing signed, no tx on-chain — cancel cannot strand funds), and the `createdAt`
    /// identity pin makes a reused id cancel NOTHING. §5.4: ids/timestamps only — no amount or address
    /// crosses. Throws [WalletErrorKind.invalidState] on a closed handle.
    pub async fn cancel_parked_send(
        &self,
        id: i64,
        created_at: i64,
    ) -> Result<bool, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        wallet
            .cancel_parked_send(id, created_at)
            .await
            .map_err(WalletApiError::from)
    }

    /// RETRY a `paused` parked send the host listed via `listParkedSends` — resume the SAME queued
    /// intent after the wallet gave up auto-retrying it (see [`ParkedSend::paused`]
    /// (crate::api::state::ParkedSend)). The wallet caps auto-retries of a one-time-address send
    /// because every retry permanently uses up one of a small number of address slots; when the
    /// user believes conditions changed (back online, the recipient service reachable again),
    /// THIS is the resume path — NOT cancel + re-enter, which re-pays the retry budget on a fresh
    /// send and MUST NOT be offered while the paused row exists (double-pay + cap evasion). Pass
    /// BOTH the `id` AND the `createdAt` from the [`ParkedSend`]. Returns `true` if the send was
    /// re-armed (it attempts again on the next background pass — not instantly); `false` if it
    /// was already gone / began sending / its rowid was reused / it had nothing to reset — an
    /// untouched (never-retried) row is refused, never silently refilled (idempotent — re-read
    /// `listParkedSends`). MONEY-SAFE: this only re-arms the existing queued intent's retry
    /// budget — nothing is signed or sent by the call itself. §5.4: ids/timestamps only. Throws
    /// [WalletErrorKind.invalidState] on a closed handle.
    pub async fn retry_parked_send(
        &self,
        id: i64,
        created_at: i64,
    ) -> Result<bool, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        wallet
            .retry_parked_send(id, created_at)
            .await
            .map_err(WalletApiError::from)
    }

    /// **AUTHORIZE a parked send NOW** (FR-23-b) — sign a send the user already committed, at this
    /// user-present moment, instead of waiting for a background pass to sign it. Pass BOTH the `id`
    /// AND the `createdAt` from the [`ParkedSend`] the user acted on.
    ///
    /// **Call this INSIDE your authorize-spend bracket** (`WalletSendAuthorizer.authorizeSpend`),
    /// with your per-spend credential staged — that is the whole point: a custody model with no
    /// standing signing capability (per-spend passphrase, biometric-per-send, external signer)
    /// cannot sign at an unattended background drain, so its offline queue could never drain. This
    /// is the SDK's answer, and it is what makes `walletOfflineQueueSupportedProvider = true`
    /// honest at host custody. Hosts with a BOUND native seed port must stage the seed for THIS
    /// row's `ParkedSend.binding` (FR-17) — a stage recorded for any other row is refused and the
    /// send stays parked (funds safe), never wrongly signed.
    ///
    /// ONE ROW PER CALL, ONE CALL PER BRACKET: the row's own binding rides the seed pull, so one
    /// staged credential serves exactly one row. An "authorize all" affordance is a LOOP over
    /// brackets host-side — never one prompt covering N rows.
    ///
    /// Contrast `retryParkedSend`, which only re-arms the retry budget of a `paused` one-time-address
    /// row and signs NOTHING. They compose: re-arm a paused row first, then authorize it.
    ///
    /// MONEY-SAFE: it moves only the TIMING of the signature — the payment itself was committed when
    /// the user queued it. Nothing new is decided here, the wallet's double-send guard is unchanged
    /// (the wallet's own background pass and this call can never both sign one send), and a wrong
    /// staged seed is refused before any transaction exists. Returns a
    /// [`ParkedAuthorization`] — read its HOST CONTRACT:
    /// only `signed` may be phrased as "sending now", and `stillQueued` is NOT a failure.
    /// §5.4: ids/timestamps only — no amount or address crosses. Throws
    /// [WalletErrorKind.invalidState] on a closed handle; a watch-only wallet throws
    /// [WalletErrorKind.watchOnly] (structurally seedless — it can never sign).
    pub async fn authorize_parked_send(
        &self,
        id: i64,
        created_at: i64,
    ) -> Result<ParkedAuthorization, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let outcome = wallet
            .authorize_parked_send(id, created_at)
            .await
            .map_err(WalletApiError::from)?;
        Ok(outcome.into())
    }

    /// How close the wallet is to the one-time-address (ZIP-320 ephemeral) gap-limit ceiling — a
    /// [`ReservationPressure`] (COUNTS-only). ⚠ `outstanding`
    /// is a LOWER BOUND (#315 slice-2 proof discovery): the underlying engine read is BLIND to slots
    /// held by sends CREATED then never confirmed (their output row exists from create-persist), so
    /// it can read `0` while the window is actually full — treat it as "at least N in use", never as
    /// proof the window is clear. Surface it as a proactive hint; the real gate is the engine's
    /// `TexSendLimitReached` at send time, and the #315 reclaim (`reclaimEphemeralSlots`) gates on
    /// its own wide reservation read, never on this gauge. An INDICATOR for display, NEVER a money
    /// gate. `0` outstanding in the common case (no TEX send has reserved an address). Throws
    /// [WalletErrorKind.invalidState] on a closed handle.
    pub async fn ephemeral_reservation_pressure(
        &self,
    ) -> Result<ReservationPressure, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let pressure = wallet
            .ephemeral_reservation_pressure()
            .await
            .map_err(WalletApiError::from)?;
        Ok(pressure.into())
    }

    /// The wallet's default receive address — a Unified Address with Orchard +
    /// Sapling receivers (no transparent receiver), encoded for display/QR.
    /// Deterministic per seed.
    pub async fn current_address(&self) -> Result<String, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let address = wallet
            .current_address()
            .await
            .map_err(WalletApiError::from)?;
        // `encoded()` is `&str`; `to_owned()` matches the crate's `&str`→String
        // convention (e.g. convert.rs) and avoids a `Display` round-trip.
        Ok(address.encoded().to_owned())
    }

    /// The wallet's transparent RECEIVE address (Recv-2 / ADR-0528): the account's
    /// canonical external-scope (`m/44'/coin'/0'/0/0`) P2PKH t-address, surfaced ALONGSIDE
    /// the shielded UA behind the receive screen's address-type toggle (default shielded).
    /// PUBLIC + reused-address-linkable (visible on-chain) — the host labels it as such and
    /// keeps the shielded UA the recommended default. Deterministic per seed; the spending
    /// key never leaves Rust (§4.1). Returns the bare t-address string (no librustzcash type
    /// crosses the bridge). A wallet whose stored UFVK predates `transparent-inputs` (pre-GA)
    /// surfaces the typed key-derivation error, never a panic.
    pub async fn current_transparent_address(&self) -> Result<String, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        wallet
            .current_transparent_address()
            .await
            .map_err(WalletApiError::from)
    }

    /// Mint the NEXT public diversified receive address (FR-8 / Recv-4, ADR-0537): a fresh
    /// unlinkable Unified Address for a contact/invoice that still credits the one wallet
    /// account. Payments to it are detected by the normal shielded scan — including after a
    /// seed-only restore — with no extra host work. Same receiver set as `currentAddress`
    /// (Orchard + Sapling, no transparent). Every call returns a NEW address from the
    /// never-recycle counter, which survives restore: each seed-only restore starts
    /// minting in a fresh RANDOM band (~2048 mints wide), so no prior life's addresses
    /// are re-issued — deterministic for lives up to ~2048 mints, ≤ ~2^-18 per-restore
    /// band-overlap chance beyond (attribution/linkage caveat, never a fund
    /// risk). Local + offline + cheap (no network, no sync
    /// requirement). Deterministic per index — keep [MintedDiversifiedAddress.diversifierIndex]
    /// as the durable attribution key, and treat it as WALLET-INTERNAL (never share it with
    /// a counterparty — it is a sequential mint ordinal). Throws
    /// [WalletErrorKind.keyDerivation] before any account is provisioned (exotic —
    /// accounts import eagerly at create), [WalletErrorKind.invalidState] on a closed
    /// handle. §5.4: render/store the fields, never log them.
    pub async fn mint_diversified_address(
        &self,
    ) -> Result<MintedDiversifiedAddress, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let minted = wallet
            .mint_diversified_address()
            .await
            .map_err(WalletApiError::from)?;
        Ok(minted.into())
    }

    /// Prepare a send from a ZIP-321 payment URI — propose (the FIRST half of the
    /// propose→confirm→send flow, inc-2d-ffi). Runs the audited note-selection + fee +
    /// change over the wallet DB and returns a [SendProposal]: the EXACT numbers
    /// (total/fee/change, per-recipient pool + amount, the §5.1 de-shield disclosure) the
    /// user confirms before signing. DETERMINISTIC — no spending keys, no proofs, no
    /// network, no DB writes; NOTHING is sent. The opaque proposal token is retained
    /// Rust-side; `send` consumes it by `proposalId`.
    ///
    /// `requestUri` is the ZIP-321 URI the host composed from its send form (via
    /// `encodePaymentUri`) or scanned from a QR — the §2.4 lossless request token. It is
    /// lowered with THIS wallet's network (a cross-network URI is
    /// [WalletErrorKind.networkMismatch]); a malformed/oversized URI is
    /// [WalletErrorKind.paymentUriInvalid]; a leg with no amount (the donation form) is
    /// [WalletErrorKind.sendAmountRequired]; not enough confirmed funds is
    /// [WalletErrorKind.insufficientFunds] (carrying available / required / pending-incoming,
    /// so the host can say "wait for confirmations" honestly); a not-yet-anchorable wallet is
    /// [WalletErrorKind.proposalStale] (re-propose after sync). No amount or address is ever
    /// logged (§5.4); [WalletErrorKind.invalidState] on a closed handle.
    pub async fn propose(&self, request_uri: String) -> Result<SendProposal, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let proposal = wallet
            .propose_uri(&request_uri)
            .await
            .map_err(WalletApiError::from)?;
        Ok(proposal.into())
    }

    /// Propose shielding the wallet's detected transparent funds into its shielded pool —
    /// the Recv-3 companion to the transparent receive address (exchange withdrawals and
    /// swap-in deliveries land TRANSPARENT, §1.7; this is the privacy-positive "move them
    /// shielded" action). Wraps the audited `propose_shielding` (Rule Zero), gated at the
    /// 0.001-ZEC shielding threshold:
    /// - returns a [SendProposal] (`isShield == true`) when ≥ the threshold is shieldable —
    ///   confirm it, then pass its `proposalId` to `send` (the SAME one-shot token the send
    ///   path consumes; the shield reuses the send pipeline WHOLE), and
    /// - returns `null` when there is nothing worth shielding yet (below the threshold or no
    ///   transparent funds) — the host hides the "shield now" action.
    ///
    /// DETERMINISTIC, like `propose`: no spending keys, no proofs, no network, no DB writes;
    /// NOTHING is sent. No amount is ever logged (§5.4); [WalletErrorKind.invalidState] on a
    /// closed handle, [WalletErrorKind.proposalStale] on a not-yet-anchorable wallet.
    pub async fn propose_shield(&self) -> Result<Option<SendProposal>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let proposal = wallet
            .propose_shield()
            .await
            .map_err(WalletApiError::from)?;
        Ok(proposal.map(Into::into))
    }

    /// Sign + broadcast a prepared proposal — the SECOND half of the send flow (inc-2d-ffi).
    /// `proposalId` is the opaque one-shot token from the [SendProposal] `propose` returned;
    /// pass back ONLY that id — the host cannot mutate what gets signed (the retained proposal
    /// is the sole source of truth, so a tampered id is just a registry miss, never a
    /// wrong-amount sign). The transient spending key is derived, used to sign inside a
    /// blocking proving task, and zeroized — it NEVER crosses this bridge. Each resulting tx
    /// is PERSISTED before broadcast (§6.3), then broadcast over a FRESH per-tx Tor circuit
    /// (§2.3) after a §5.3 jitter.
    ///
    /// Returns a per-tx [TxSubmitResult] list (a proposal can mint several txs when crossing
    /// pools — §1.7): a broadcast failure is first-class DATA here, NEVER a thrown error, and
    /// every persisted tx is a DELIVERY OBLIGATION (ADR-0556, stage S8): the same bytes go out
    /// again on every later sync pass and after a reopen until they confirm or expire — so
    /// even a TOTAL broadcast failure loses no funds, only immediacy. (Before S8 this sentence
    /// was true of outbox rows only; a single-step send kept no row and was never retried —
    /// the 2026-09-20 review's R02.) Read the live state per tx from [TxSummary.delivery] or
    /// [WalletHandle.deliveryState] — persisted / retryPending / accepted / confirmed. Throws
    /// [WalletErrorKind.proposalAlreadyUsed] on a re-send/double-tap (the token was consumed —
    /// the notes are never broadcast twice, §6.3) or [WalletErrorKind.signFailed]. No txid/amount
    /// is ever logged (§5.4); [WalletErrorKind.invalidState] on a closed handle.
    ///
    /// ORDERING (the per-tx → leg contract): the list is in BROADCAST order, which is also mine
    /// order. For a ZIP-320 TEX two-step ([SendProposal.isTwoStepTex]) it is EXACTLY two entries:
    /// `[0]` is tx0 — the unshield that funds a wallet-controlled one-time (ephemeral) address —
    /// and `[1]` is tx1 — the forward from that address to the TEX recipient. tx1 SPENDS tx0's
    /// output, so it can only mine after tx0. Only a TRANSPORT failure latches the tail as
    /// [TxSubmitResult.notAttempted] (the endpoint never saw the predecessor, so the tail cannot
    /// propagate this call); a mempool REJECT of tx0 does NOT stop tx1 — tx0 may simply be
    /// already known (the wallet's own background re-broadcast can race this call and land it
    /// first), and a genuinely-invalid tx0 just makes tx1 a harmless reject. So a two-step list
    /// holding ANY success but not ALL — `[Success, non-Success]` or `[non-Success, Success]`
    /// (the latter implies an HONEST endpoint already knew tx0) — is the "funds in motion on the
    /// ephemeral" state: the host renders the honest in-motion partial (do-not-resend;
    /// recoverable), NOT the ordinary "each tx saved-for-retry" copy (see
    /// [SendProposal.isTwoStepTex]). RESIDUAL: a ZERO-success two-step can ALSO be that race
    /// (both legs already landed → both read rejected), but reject codes are backend-specific
    /// and deliberately uninterpreted, so it reads as the ordinary saved-for-retry — the durable
    /// [WalletHandle.listInFlightSends] cue owns the honest "don't re-send" through that window.
    /// Every non-accepted TWO-STEP tx stays persisted and re-broadcasts on the next sync (the
    /// send is enrolled durably before broadcast); a failed SINGLE-step send instead self-heals
    /// by expiry (~40 blocks) freeing its notes for a fresh re-send. An ordinary
    /// pool-crossing send's txs are INDEPENDENT — position is broadcast order only, no leg meaning.
    ///
    /// PROPOSAL FRESHNESS (the confirm-screen host contract): a [SendProposal] is a one-shot
    /// token with a short, minutes-scale TTL (MONOTONIC — device suspend counts). If the user
    /// opens the confirm screen and resumes after a long background gap, `send` returns
    /// [WalletErrorKind.proposalStale]; the host MUST call `propose` AGAIN for a fresh
    /// [SendProposal] (the fee/anchor may have moved) and re-present the numbers — NEVER retry the
    /// same `proposalId`. `proposalStale` is also raised if the wallet is not yet anchorable.
    pub async fn send(&self, proposal_id: i64) -> Result<Vec<TxSubmitResult>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        // i64→u64: the host hands back the id `propose` minted (the DTO carries it as a Dart
        // `int`). An out-of-range/negative id from a buggy host wraps to a u64 the registry
        // never issued ⇒ a typed `ProposalAlreadyUsed` miss, never a wrong-token sign.
        let results = wallet
            .send_by_id(proposal_id as u64)
            .await
            .map_err(WalletApiError::from)?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Queue a send for OFFLINE-first delivery (inc-2d-ffi). Durably persists the send INTENT
    /// (the ZIP-321 URI) and returns its opaque id IMMEDIATELY — NO network, NO signing, NO
    /// money movement. The §6.2 "queued is a normal state" path: tap send with zero
    /// connectivity and it survives a process kill, then proposes→signs→broadcasts on the next
    /// online sync pass (the resubmission machinery) — proposing at SEND time, not queue time,
    /// is exactly what stops a long-queued send from ever carrying a stale anchor.
    ///
    /// `requestUri` is the same lossless ZIP-321 token `propose` takes (same network /
    /// donation-form / oversize validation, same typed errors). The returned String is the
    /// OPAQUE queued-send id — do not parse it. Throws [WalletErrorKind.queuedSendsFull] when
    /// the durable queue is at capacity (a real send is never silently dropped). No amount or
    /// address is ever logged (§5.4); [WalletErrorKind.invalidState] on a closed handle.
    pub async fn queue_send(&self, request_uri: String) -> Result<String, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let id = wallet
            .queue_send_uri(&request_uri)
            .await
            .map_err(WalletApiError::from)?;
        // QueuedSendId crosses as an OPAQUE string (the host must not do arithmetic on it).
        Ok(id.value().to_string())
    }

    /// Reveal this wallet's BIP39 recovery phrase — word by word, in index order —
    /// so the user can back it up. The ONE sanctioned outbound key-material crossing
    /// (spec §3.3). The words ARE the wallet's whole secret: show them ONCE on a
    /// secure screen the host marks FLAG_SECURE, never log / persist / screenshot /
    /// transmit them, and prompt the user to write them down offline. Returns the
    /// words as a Dart `List<String>` (24 for a generated wallet; a restored phrase
    /// reveals its own length) — Dart memory cannot be zeroized (the documented §10
    /// exposure); the Rust side holds the words in zeroizing buffers and wipes its
    /// own copies when this call returns.
    ///
    /// Throws [WalletErrorKind.noMnemonic] for a wallet with no stored phrase — one
    /// opened from raw seed bytes (a host-custodied-seed integration recovers from
    /// its OWN master secret, never a wallet-local phrase) — and
    /// [WalletErrorKind.invalidState] on a closed handle.
    pub async fn reveal_mnemonic(&self) -> Result<Vec<String>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        // the core call is SYNC (a pure in-memory read); the `async` here is the
        // FRB bridge-method convention — there is no await. The zeroizing→String
        // conversion lives in convert.rs (off the gate-scanned api surface).
        crate::convert::reveal_mnemonic(wallet)
    }

    /// Start the background sync loop — required for the live `watchSyncStatus`
    /// stream (and balance progress) to advance; a freshly opened wallet does
    /// NOT auto-sync. The engine runs over the configured endpoint/Tor with
    /// retry/backoff + a stuck-sync watchdog, so a fault surfaces as a
    /// `SyncStatus.stalled` EVENT on the stream, never a thrown error or a dead
    /// stream. Idempotent (a second call while running is a no-op).
    ///
    /// Throws [WalletErrorKind.invalidState] if the handle is closed or the
    /// wallet is mid-teardown.
    pub async fn start_sync(&self) -> Result<(), WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        // the core `start_sync` is SYNC (it just spawns the loop + returns); the
        // `async` here is the FRB bridge-method convention — there is no await.
        wallet.start_sync().map_err(WalletApiError::from)
    }

    /// Stop the background sync loop and await clean teardown. Idempotent;
    /// durable progress is preserved (the chain is the source of truth), so
    /// stopping loses nothing. Prompt even mid-sync (cooperative cancel ≤ one
    /// batch scan). Use it to pause sync WITHOUT closing the wallet; `close`
    /// stops sync as part of its own teardown. A no-op on an already-closed
    /// handle (nothing to stop).
    pub async fn stop_sync(&self) -> Result<(), WalletApiError> {
        if let Some(wallet) = self.inner.as_ref() {
            wallet.stop_sync().await;
        }
        Ok(())
    }

    /// Sync for at most `budgetMs` milliseconds, then report how far the
    /// wallet got — for a host that holds the process only briefly (a
    /// background wake, a pull-to-refresh). ONE pass runs; when the budget
    /// runs out it stops the way `stopSync` stops the loop (progress kept,
    /// nothing half-written) and the call returns a report with
    /// `finished: false`, never an error. The budget is always yours: the SDK
    /// has no default and schedules nothing.
    ///
    /// **The bound:** the call returns within `budgetMs` plus one network
    /// request's timeout (30 s) — a request already in flight is not cut.
    /// Queued and in-flight sends are resubmitted after the pass only when the
    /// budget left covers that step's worst case (two minutes); otherwise
    /// `resubmitted` is `false` and the next `startSync` pass carries them.
    ///
    /// Throws [WalletErrorKind.syncRunning] while the `startSync` loop runs or
    /// another `syncFor` is in progress (stop the loop first; nothing ran);
    /// [WalletErrorKind.invalidState] on a closed handle; a pass that faults
    /// throws its error ([WalletErrorKind.sync] and the rest) with the same
    /// reason the `watchSyncStatus` stream shows.
    pub async fn sync_for(&self, budget_ms: u32) -> Result<BoundedSync, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        wallet
            .sync_for(std::time::Duration::from_millis(u64::from(budget_ms)))
            .await
            .map(BoundedSync::from)
            .map_err(WalletApiError::from)
    }

    /// The sync servers this host OFFERS (`WalletConfig.syncServers`, as
    /// validated) — WITHOUT their keys: `authValue` is always `null` on this
    /// list (the host supplied it and holds it); `authHeader` says whether an
    /// entry is gated. The picker, P3-13.
    /// [WalletErrorKind.invalidState] if the handle is closed.
    pub fn sync_servers(&self) -> Result<Vec<SyncServer>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        Ok(crate::convert::sync_servers_of(wallet))
    }

    /// Which server the wallet dials, which remembered choice produced it,
    /// and whether a fallback is in force. Render the HOST of `effectiveUrl`
    /// on the sync sheet's Server row — the connection's own truth, so the
    /// display and the dial can never drift. Readable in every lifecycle
    /// phase. [WalletErrorKind.invalidState] if the handle is closed.
    pub fn sync_server_status(&self) -> Result<SyncServerStatus, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        Ok(crate::convert::sync_server_status_of(wallet))
    }

    /// Dial `choice` under THIS wallet's Tor policy and ask the server who it
    /// is (one `GetLightdInfo`, 15 s budget). Refuses typed —
    /// [WalletErrorKind.syncServerUnreachable] (could not dial, timed out, or
    /// the server refused), [WalletErrorKind.networkMismatch] (the server is
    /// on another Zcash network), [WalletErrorKind.syncServerNotOffered] —
    /// and NEVER switches. Use it for the picker's "Check server" step; a
    /// switch probes again on its own. `default` probes `endpointUrl`.
    pub async fn probe_sync_server(
        &self,
        choice: SyncServerChoice,
    ) -> Result<SyncServerProbe, WalletApiError> {
        let Some(wallet) = self.inner.as_ref() else {
            // A closed handle still received the choice: its key (ADR-0568)
            // is dropped zeroized, never as a plain String.
            crate::convert::discard_choice(choice);
            return Err(crate::convert::handle_closed());
        };
        // conversion + the call live in crate::convert (the api/ rule).
        crate::convert::probe_sync_server(wallet, choice).await
    }

    /// Probe, then switch the wallet onto `choice` and REMEMBER it (the row
    /// survives restarts and rescans; `default` forgets it). The session is
    /// swapped in place: the sync loop is stopped and joined, the choice
    /// written, the wallet rebuilt over the SAME database — no rescan, no
    /// re-download; funds, history, queued sends and the sync verdict are
    /// untouched. Afterwards call `startSync`, and re-subscribe the live
    /// streams the swap ends — the old session's `watchSyncStatus` stream
    /// ENDS (the reference UI swaps its whole session, as after a rescan).
    /// `watchTorState` and `watchIncomingFunds` are the exceptions: their
    /// channels are CARRIED into the rebuilt session, so those subscriptions
    /// keep delivering and start reporting the NEW session at once. A
    /// transport chip rendered from `watchTorState` must not go silent here,
    /// because the rebuilt session is exactly where the private path can
    /// change. The swap service is re-initialised: call `enableNearSwap`
    /// again if you use it.
    ///
    /// Refusals BEFORE the swap leave everything exactly as it was — the
    /// handle stays open and usable: [WalletErrorKind.syncServerUnreachable],
    /// [WalletErrorKind.networkMismatch], [WalletErrorKind.syncServerNotOffered],
    /// [WalletErrorKind.walletBusy]. A fault PAST the stop-join (rare — a
    /// storage fault writing the choice, a wedged pass) leaves the handle
    /// CLOSED, exactly like a failed `rescanFrom`: re-`open` (the choice is
    /// honoured if its write landed). [WalletErrorKind.invalidState] if the
    /// handle is already closed. Through THIS bridge a concurrent call QUEUES
    /// behind the switch (the handle's write lock is held for the whole swap,
    /// probe and quiesce included) and resolves after it; only a direct Rust
    /// host sees [WalletErrorKind.walletBusy] carrying
    /// [LifecyclePhase.switchingServer] mid-swap. A switch that resolves to
    /// the server already in use — `https://zec.rocks` for `…:443` counts as
    /// the same — remembers the choice WITHOUT swapping the session (no loop
    /// restart, the sync judgement kept).
    pub async fn switch_sync_server(
        &mut self,
        choice: SyncServerChoice,
    ) -> Result<(), WalletApiError> {
        match self.inner.take() {
            Some(wallet) => match crate::convert::switch_sync_server(wallet, choice).await {
                Ok(switched) => {
                    self.inner = Some(switched);
                    Ok(())
                }
                Err((kept, error)) => {
                    // A pre-stop refusal hands the handle back: nothing
                    // changed, the host keeps its wallet. Past the stop-join
                    // `kept` is `None` and the handle stays closed.
                    self.inner = kept;
                    Err(error)
                }
            },
            None => {
                // The key a closed handle received is dropped zeroized.
                crate::convert::discard_choice(choice);
                Err(crate::convert::handle_closed())
            }
        }
    }

    /// The live `SyncStatus` stream (spec §3.3) — the host's honest sync
    /// indicator. The stream emits the CURRENT status immediately on subscribe
    /// (so re-subscribing after an `AppLifecycleState.paused` gap re-renders at
    /// once), then every change coalesced latest-wins (a burst never floods the
    /// UI). It NEVER completes on a transient fault — a stall is a
    /// `SyncStatus.stalled` EVENT, not an error or EOF; it completes only when
    /// the host cancels the subscription or the wallet is closed (a final status
    /// is drained first). Call `startSync` for it to advance past `idle`.
    ///
    /// LIFECYCLE (flutter-patterns § Stream Lifecycle): cancel this on `paused`
    /// and re-subscribe on a REAL `resumed` (re-fetch the cold balance via
    /// `snapshot`, then re-subscribe for live changes); a stream error or
    /// completion is itself a reconnect trigger, independent of lifecycle.
    /// Desktop never fires `paused`, so the stream stays live while hidden (the
    /// wanted behavior). The pump holds no lock for its lifetime, so a live
    /// subscription never blocks `close`.
    ///
    /// Errors (delivered as a stream error): [WalletErrorKind.invalidState] if
    /// the handle is already closed.
    pub async fn watch_sync_status(
        &self,
        sink: StreamSink<SyncStatus>,
    ) -> Result<(), WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        // brief: the CORE spawns the detached pump (owning an independent watch
        // receiver), so this returns at once and the read lock releases — a live
        // stream never blocks `close`. The bridge holds no async runtime of its own.
        crate::convert::stream_sync_status(wallet, sink);
        Ok(())
    }

    /// The live [TorState] stream — the transport state as it changes, so a
    /// host renders the private path's truth without polling `snapshot`. The
    /// stream emits the CURRENT state immediately on subscribe, then every
    /// change coalesced latest-wins: the moment a `preferred` wallet leaves the
    /// private path ([TorState.fellBack] — the switch a user should be told
    /// about), and the moment a ready path has carried nothing for a minute
    /// ([TorState.unanswered]). It NEVER completes on a transient fault; it
    /// completes only when the host cancels the subscription, the wallet is
    /// closed, or `rescanFrom` rebuilds the session (which ends every live
    /// stream — re-subscribe after a rescan). A `switchSyncServer` does NOT
    /// end it: the stream is carried across the swap and reports the new
    /// session. Readable in every lifecycle phase, like `snapshot`'s `tor`.
    ///
    /// LIFECYCLE (flutter-patterns § Stream Lifecycle): cancel this on `paused`
    /// and re-subscribe on a REAL `resumed` — snapshot FIRST, then
    /// re-subscribe: a transition that happened while the sink was paused is
    /// not replayed (the clock keeps running while the app is backgrounded),
    /// and the snapshot is where it is read. The first event after
    /// re-subscribing is the state as of that call. The pump holds no lock for
    /// its lifetime, so a live subscription never blocks `close`.
    ///
    /// Errors (delivered as a stream error): [WalletErrorKind.invalidState] if
    /// the handle is already closed.
    pub async fn watch_tor_state(&self, sink: StreamSink<TorState>) -> Result<(), WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        // Brief, like `watch_sync_status`: the core owns the detached pump.
        crate::convert::stream_tor_state(wallet, sink);
        Ok(())
    }

    /// How many connections each arm served, by outcome, since this wallet
    /// opened — the numbers to show beside the transport state (the network
    /// panel's "N private, M direct"). One connection is one dial; the count
    /// is exactly what the SDK's own `wallet.dial` log line records, so the
    /// two never disagree. In memory only: a reopen starts at zero. Readable
    /// in every lifecycle phase, cheap, no I/O. Never logged by the SDK — it
    /// crosses only on this call.
    /// [WalletErrorKind.invalidState] if the handle is closed.
    pub fn dial_counts(&self) -> Result<DialCounts, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        Ok(wallet.dial_counts().into())
    }

    /// Stream incoming-funds events (ADR-0536 — the FR-1 "funds arrived" hook):
    /// ONE `replay` event first (arrivals already in history strictly above
    /// `sinceCursor`; a null cursor gets a count-0 baseline that hands you a
    /// cursor to persist), then a `live` event per scan batch that detected
    /// arrivals, and a `memoRefresh` nudge when a later enhancement pass changes how
    /// existing rows read — either decrypted tx data landed, or a transaction's chain
    /// status did. Treat `memoRefresh` as "re-pull", never as "new memo data exists".
    ///
    /// The payload is DELIBERATELY minimal — counts, heights, and an opaque
    /// cursor; no txid, amount, memo, or address — so it is safe to log or to
    /// forward toward a notification path without a sanitizing layer. Pull the
    /// details via `transactions` in the main app (never in a notification
    /// extension). Delivery is AT-LEAST-ONCE with latest-wins coalescing:
    /// `totalTxDetected` is the reliable monotonic; counts/spans are advisory.
    /// A rescan or seed-restore REPLAYS history with old spans — gate any
    /// notification on `spanToHeight > your persisted watermark` (and your own
    /// consumed-txid ledger for exactly-once). `live` spans cover SHIELDED
    /// arrivals; a transparent receive surfaces via `memoRefresh` + the
    /// ordinary balance/history pulls (see [IncomingFundsEvent]).
    ///
    /// LIFECYCLE (flutter-patterns § Stream Lifecycle): cancel on `paused`,
    /// re-subscribe on a REAL `resumed` passing your persisted cursor — the
    /// `replay` event then covers anything missed while backgrounded. Desktop
    /// never fires `paused`, so the stream stays live while hidden. The pump
    /// holds no lock for its lifetime, so a live subscription never blocks
    /// `close`.
    ///
    /// Errors: [WalletErrorKind.invalidState] if the handle is closed;
    /// [WalletErrorKind.storeCorrupt] on a malformed `sinceCursor` (including a
    /// [HistoryPage.nextCursor] keyset token passed here by mistake — the two
    /// cursor families are deliberately incompatible, so the mix-up rejects
    /// typed instead of silently suppressing a catch-up).
    pub async fn watch_incoming_funds(
        &self,
        since_cursor: Option<String>,
        sink: StreamSink<IncomingFundsEvent>,
    ) -> Result<(), WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        // Brief hold: the cursor validation + the per-subscriber replay read run
        // here, then the CORE spawns the detached pump and this returns — the
        // read lock releases; a live stream never blocks `close`.
        crate::convert::stream_incoming_funds(wallet, since_cursor, sink).await
    }

    /// Construct the NEAR Intents swap provider over THIS wallet's own transport
    /// and turn the swap on-ramp ON (spec §3.5; the `swap-near` feature). The
    /// composition entry behind the `swapQuote`/`swapExecute`/`watchSwapStatus`
    /// surface — until it succeeds, [WalletHandle.swap] is conceptually `null`
    /// (kill layer 2) and those three throw [SwapErrorKind.swapDisabled].
    ///
    /// Swap rides the wallet's CONFIGURED transport (the same host-provided dialer
    /// and Tor policy sync uses, ADR-0526 — no independent egress; internal Tor is
    /// optional, the host may supply its own). [config.jwt] is an OPTIONAL provider
    /// auth token (a §5.4 NEVER-log value): it is wrapped in a zeroizing buffer in
    /// Rust at this boundary and never logged; Dart memory cannot be zeroized (the
    /// documented §10 exposure, same as the mnemonic crossing) — keep it minimal.
    /// `swapEnabled` + `declaredKill` are the host's §3.5 layer-3 kill state (e.g.
    /// lowered from a signed manifest); they are applied MONOTONIC-TOWARD-OFF, so a
    /// forged/replayed "on" can only ever make swap MORE off, never resurrect it.
    ///
    /// Set-once, first-wins: a second call is a no-op (re-enabling swap is host
    /// RECONSTRUCTION of the wallet, never an in-place provider swap; §3.5). On any
    /// failure NO provider is attached — the wallet stays at kill layer 2, zero swap
    /// surface. Throws a [SwapApiError]: [SwapErrorKind.watchOnly] on a WATCH-ONLY
    /// wallet (#397 §3.7 D3 — STRUCTURAL and NEVER retryable, checked FIRST so it
    /// outranks any config-shaped failure; render the view-only framing, not a
    /// retry), [SwapErrorKind.providerUnavailable] when the wallet's transport
    /// cannot be resolved (the on-ramp has no network to ride — retryable),
    /// [SwapErrorKind.swapStateUnavailable] on a closed handle, and
    /// [SwapErrorKind.swapDisabled] in a build compiled WITHOUT the `swap-near`
    /// adapter (§3.5 kill layer 1 — the method stays present so the Dart surface is
    /// STABLE across feature flavors, but honestly reports swap is off).
    pub async fn enable_near_swap(
        &self,
        config: SwapProviderConfig,
        swap_enabled: bool,
        declared_kill: Option<SwapKill>,
    ) -> Result<(), SwapApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::swap_handle_closed)?;
        // The method is NOT `#[cfg]`-gated (FRB's generated dispatcher can't honor a
        // method-level cfg ⇒ a `--no-default-features` build would call a missing fn);
        // instead the BODY branches, so the Dart surface stays stable and a no-adapter
        // build degrades honestly (principle 6) to kill-layer-1 `SwapDisabled`.
        #[cfg(feature = "swap-near")]
        {
            // the core call is SYNC (construct + attach + set_kill, no await); the `async`
            // here is the FRB bridge-method convention. All composition lives in
            // `convert::enable_near_swap` → `swap_provider::enable_near_swap` (the kill door).
            crate::convert::enable_near_swap(wallet, config, swap_enabled, declared_kill)
        }
        #[cfg(not(feature = "swap-near"))]
        {
            // The NEAR adapter is not linked in this build (the host built
            // `--no-default-features`): not one byte of it ships (§3.5 layer 1), so the
            // on-ramp cannot be constructed and swap stays off. Bind the unused params
            // and return the honest kill-layer-1 outcome.
            let _ = (wallet, config, swap_enabled, declared_kill);
            Err(crate::convert::swap_disabled())
        }
    }

    /// Request a bounds-checked swap quote (spec §2.6/§3.3) — the FIRST step of the
    /// on-ramp. `request` names the direction, the user's EXACT side (the anchor every
    /// returned number is bounds-checked against before anything is signed), the
    /// slippage tolerance, and the direction-specific destination/refund fields. The
    /// returned [SwapQuote] carries the provider deposit address, the min-out floor, the
    /// ZEC side in zatoshis, and the §2.6 privacy `disclosure` the host MUST render —
    /// funds NEVER move here. No amount/address is logged (§5.4).
    ///
    /// Throws a [SwapApiError]: [SwapErrorKind.swapDisabled] when no provider is attached
    /// (Dart `swap == null`; kill layer 2), [SwapErrorKind.swapStateUnavailable] on a
    /// closed handle, and the §2.6 quote errors ([SwapErrorKind.slippageToleranceTooHigh],
    /// [SwapErrorKind.quoteOutOfBounds], [SwapErrorKind.destinationInvalid],
    /// [SwapErrorKind.providerUnavailable], [SwapErrorKind.refundAddressUnavailable]).
    pub async fn swap_quote(&self, request: QuoteRequest) -> Result<SwapQuote, SwapApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::swap_handle_closed)?;
        let service = wallet.swap().ok_or_else(crate::convert::swap_disabled)?;
        // the inbound `TryFrom` rejects a host-bug `Unknown` direction typed (in
        // convert.rs, off the gate-scanned api surface). The request's host-supplied
        // strings (destination / refund / foreign amount) are NOT re-capped here: the
        // core's `validate_request` is the single cap point (DRY) — it bounds every
        // provider string at `PROVIDER_STR_MAX_BYTES` before any allocation, so a bridge
        // cap would only duplicate it (unlike `swap_id`, which has no such core door and
        // would otherwise spin the poll loop).
        let core_request =
            zec_wallet_core::QuoteRequest::try_from(request).map_err(SwapApiError::from)?;
        let quote = service
            .quote(core_request)
            .await
            .map_err(SwapApiError::from)?;
        Ok(quote.into())
    }

    /// The dynamic source-asset list for the IntoZec picker (spec §3.3b D5/L6) — LAZY: the host
    /// calls this when the user opens the IntoZec form (NOT on launch, so an idle user emits no
    /// token traffic). The SDK fetches the provider's `/v0/tokens` on its OWN circuit (never the
    /// sync circuit), filters to real quotable foreign assets (the ZEC asset itself + any
    /// `$0`/null-price entry dropped), and returns the picker rows + a freshness flag. On a fetch
    /// fault it serves the LAST good list with [SwapTokenList.fresh] = false (the host shows a
    /// "couldn't refresh, showing cached" banner — honest degradation), never a blank picker; an
    /// empty fresh list is the honest "no assets available right now." The host maps each
    /// `(chain, symbol)` to a BUNDLED icon (never CDN-fetched). No token field is logged (§5.4).
    ///
    /// Throws a [SwapApiError]: [SwapErrorKind.swapDisabled] when no provider is attached (kill
    /// layer 2), [SwapErrorKind.swapStateUnavailable] on a closed handle, and
    /// [SwapErrorKind.providerUnavailable] on a first-ever fetch fault with nothing cached to serve.
    pub async fn swap_list_tokens(&self) -> Result<SwapTokenList, SwapApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::swap_handle_closed)?;
        let service = wallet.swap().ok_or_else(crate::convert::swap_disabled)?;
        let list = service.list_tokens().await.map_err(SwapApiError::from)?;
        Ok(list.into())
    }

    /// Execute a quote — register the swap intent with the provider and (for OutOfZec)
    /// queue the §4.4 ZEC deposit send (spec §2.6/§3.3). Pass back the [SwapQuote]
    /// `swapQuote` returned; this is LEAST-AUTHORITY-equivalent to the `send` token even
    /// though it carries the whole DTO: the SDK PEEKS its own durable record by `quote.id`,
    /// compares the DTO's terms and binding to it field-by-field — a difference is
    /// [SwapErrorKind.quoteTermsDiffer] with nothing consumed — then claims the record
    /// single-flight; EVERY execution term comes from that record, never this DTO
    /// (ADR-0555), so a tampered `depositAddress`/amount cannot redirect funds.
    /// Returns the SDK-minted `SwapId` as an opaque String (`quote.id` itself, never the
    /// provider's deposit address) — do NOT parse it; pass it to `watchSwapStatus` as-is.
    /// Idempotent against a double-tap / crash-retry: ONE deposit per quote, ever (§5.4: nothing logged).
    ///
    /// Throws a [SwapApiError]: [SwapErrorKind.swapDisabled] / [SwapErrorKind.swapStateUnavailable]
    /// as for `swapQuote`; [SwapErrorKind.quoteExpired] past the deposit deadline AND for a quote
    /// this wallet did not issue or already executed (re-quote, never retry the same DTO);
    /// [SwapErrorKind.quoteTermsDiffer] when the DTO differs from the issued record (get a fresh
    /// quote); [SwapErrorKind.depositSendFailed] if the deposit leg could not be queued (re-quote).
    pub async fn swap_execute(&self, quote: SwapQuote) -> Result<String, SwapApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::swap_handle_closed)?;
        let service = wallet.swap().ok_or_else(crate::convert::swap_disabled)?;
        let core_quote = zec_wallet_core::SwapQuote::try_from(quote).map_err(SwapApiError::from)?;
        let id = service
            .execute(&core_quote)
            .await
            .map_err(SwapApiError::from)?;
        // SwapId → opaque String (the SDK-minted handle, never the provider's deposit
        // address; the host must not parse it). NEVER logged here (§5.4).
        Ok(id.as_str().to_string())
    }

    /// The live [SwapStatus] stream for one in-flight swap (spec §3.3/§7) — the swap
    /// parallel of `watchSyncStatus`. `swapId` is the opaque id `swapExecute` returned.
    /// The stream emits the CURRENT status immediately, then polls at a bounded
    /// exponential cadence (5→60s; §7), coalescing onto each change. It NEVER completes
    /// on a transient provider fault (a stall is retried, never a dead stream) — it
    /// completes only on a TERMINAL status (success/refunded/failed), when the host
    /// cancels the subscription, or when the wallet is closed/swap is hard-killed (§3.5).
    ///
    /// LIFECYCLE (flutter-patterns § Stream Lifecycle): foreground-only — cancel on
    /// `AppLifecycleState.paused`, re-subscribe on a REAL `resumed` (it re-emits the
    /// current status at once). Desktop never fires `paused`, so it stays live while
    /// hidden. The core owns the detached poll pump and holds no lock for the stream's
    /// life, so a live subscription never blocks `close`. The `swapId` is NEVER logged
    /// (§5.4); an empty/over-bound id is rejected typed (a host bug), never a spinning
    /// stream.
    ///
    /// Errors (delivered as a stream error): [SwapErrorKind.swapDisabled] when no
    /// provider is attached, [SwapErrorKind.swapStateUnavailable] on a closed handle,
    /// [SwapErrorKind.requestInvalid] for an empty/over-bound id.
    pub async fn watch_swap_status(
        &self,
        swap_id: String,
        sink: StreamSink<SwapStatus>,
    ) -> Result<(), SwapApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::swap_handle_closed)?;
        let service = wallet.swap().ok_or_else(crate::convert::swap_disabled)?;
        // CORE resolves the provider handle from the home row, spawns the detached §7
        // poll pump and returns, so the read lock releases — a live swap stream never
        // blocks `close`. The boundary id cap + sink adaptation live in
        // `convert::stream_swap_status`.
        crate::convert::stream_swap_status(service, swap_id, sink).await
    }

    /// The durable IN-FLIGHT SWAP surface (W-swap-5, #366): every swap whose order was
    /// registered at execute and has neither been dismissed nor self-lapsed. THE
    /// kill→relaunch re-attach: after a process restart the home screen lists these and
    /// re-opens live tracking by feeding [SwapRecord.id] to `watchSwapStatus` — without
    /// this list an armed OutOfZec deposit was invisible everywhere while the
    /// one-swap-in-flight guard refused new swaps. NOT gated on swap being enabled or
    /// the §3.5 kill state (a local read is not swap traffic — records stay readable so
    /// the user never loses sight of money in motion). Read-only, cheap + LOCAL (a
    /// SQLite read); call it on the same edges as `listParkedSends` (sync ticks /
    /// resume / after an execute). Empty when nothing is in flight — the common case.
    /// §5.4: [SwapRecord.id] is render-never-log. Throws [WalletApiError] only for a
    /// closed handle / store fault.
    pub async fn list_in_flight_swaps(&self) -> Result<Vec<SwapRecord>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let swaps = wallet
            .list_in_flight_swaps()
            .await
            .map_err(WalletApiError::from)?;
        Ok(swaps.into_iter().map(SwapRecord::from).collect())
    }

    /// Pin an OBSERVED terminal outcome into an in-flight swap record (#367): call
    /// when the tracking stream reports a TERMINAL status (success / refunded /
    /// failed) so the home row renders the truth even if the user was away at
    /// observation time — deleting on observation (the pre-#367 contract) erased
    /// outcomes the user never saw, durably if the process died in the gap.
    /// FIRST-WINS (a terminal is forever; a later conflicting pin no-ops) and
    /// idempotent — returns `false` for an absent / lapsed / already-pinned id.
    /// Display-only state; never touches the deposit outbox, the in-flight guard,
    /// or detection. §5.4: `swapId` is render-never-log.
    pub async fn record_swap_outcome(
        &self,
        swap_id: String,
        outcome: SwapOutcome,
    ) -> Result<bool, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        wallet
            .record_swap_outcome(&swap_id, outcome.into())
            .await
            .map_err(WalletApiError::from)
    }

    /// Remove one in-flight swap record. USER-INTENT only since #367: call from the
    /// terminal card's Done (the outcome rendered full-screen = seen) or the home
    /// row's explicit Remove — a terminal OBSERVATION pins via `recordSwapOutcome`
    /// instead, so an away-at-terminal outcome is never erased unseen.
    /// Display-only state: this never touches the deposit outbox, the in-flight
    /// guard, or detection (money in motion keeps moving; only the home row goes).
    /// Idempotent — returns `false` for an already-absent id, so a raced
    /// double-dismiss is harmless. A never-dismissed record self-lapses out of
    /// `listInFlightSwaps` at [SwapRecord.expiresAt].
    pub async fn dismiss_swap_record(&self, swap_id: String) -> Result<bool, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        wallet
            .dismiss_swap_record(&swap_id)
            .await
            .map_err(WalletApiError::from)
    }

    /// Close the wallet: stop and join the sync loop, QUIESCE any straggling
    /// scan-batch section (a bounded wait — v-5c finding #2), then release the
    /// single-writer lock and close storage on a background thread — all
    /// before this future resolves, so a subsequent `open` at the same
    /// `dbDir` never observes a half-closed DB. BOUNDED residual: a wedged
    /// straggler past the quiesce budget can keep the lock alive briefly
    /// after this resolves (integrity still holds; the lock frees the moment
    /// it ends — an immediate re-`open` should tolerate a transient
    /// `WalletAlreadyOpen` with a short retry). The chain is the source of
    /// truth, so closing loses nothing. Idempotent: a second call is a no-op.
    ///
    /// A registered host dialer's outstanding operations are closed with the
    /// wallet (FR-39): every one still waiting on the host is marked closed,
    /// and a connection the host delivers to one afterwards is handed back to
    /// the host's `close`. Nothing the host still holds is freed early. The
    /// operation table is process-wide — with two wallets open at once, this
    /// also fails the other wallet's in-flight dials (typed, retryable).
    pub async fn close(&mut self) -> Result<(), WalletApiError> {
        match self.inner.take() {
            Some(wallet) => {
                let closed = wallet.close().await.map_err(WalletApiError::from);
                // After the core's close, whatever it returned: the sync loop
                // is joined, so what is still outstanding is a straggler's.
                crate::net_dialer_cabi::close_outstanding_ops();
                closed
            }
            None => Ok(()),
        }
    }

    /// Rescan from an EARLIER birthday to recover funds an over-high restore
    /// birthday skipped (ADR-0534 — the in-app fix for the ADR-0533 "balance reads
    /// zero, sync says done" gap). Rebuilds ONLY the data DB at `fromHeight` — NO
    /// recovery-phrase re-entry, and NO key material crosses this bridge (the only
    /// argument is a bare block height). A SEALED-KEYCHAIN wallet re-imports from
    /// the seed that stays sealed in the keychain. A HOST-CUSTODIED
    /// (`SeedPersistence.none`) wallet opened through `openWithHostSeed` re-imports
    /// through its registered seed port at the FIRST sync after the rescan — the
    /// same pull its first-ever sync made, native-to-native, so the host must have
    /// the seed STAGED for that sync (until it runs, the rebuilt wallet honestly
    /// reads "account not yet provisioned", balance zero, history empty); a port
    /// answering the WRONG seed is refused [WalletErrorKind.seedMismatch] before a
    /// note is derived, and the next sync asks again. A host-custodied wallet with
    /// NO seed port registered is a typed up-front [WalletErrorKind.seedRequired]
    /// (nothing can serve the re-import). Pass `null` to scan the FULL history
    /// (floors to Sapling activation, a slower but money-SAFE scan, NEVER ~tip —
    /// except a freshly-generated wallet, whose durable creation stamp re-floors
    /// `null` at ~creation: money-equivalent and hours faster; pass an explicit
    /// height to go deeper). Pair with `estimateBirthday` for a date-picker UX.
    ///
    /// The durable money state (pending queued sends, the refund-address index, and
    /// in-flight swap detection) is PRESERVED across the rebuild; the seed seal +
    /// keychain vault are never touched, so a crash mid-rescan can never lose the
    /// seed (the wallet re-opens at the old OR the new birthday, never bricked).
    ///
    /// Stops + joins sync first, then rebuilds under the single-writer lock — a
    /// concurrent call observes a retryable [WalletErrorKind.walletBusy] carrying the
    /// `rescanning` phase. CONSUMES the handle's wallet: on success the rebuilt wallet
    /// REPLACES it (call `startSync` to scan from the lower birthday, and re-subscribe
    /// EVERY live stream — a rescan is a whole new session, so `watchSyncStatus`,
    /// `watchTorState` and `watchIncomingFunds` all complete, unlike a
    /// `switchSyncServer`); on ANY error
    /// (e.g. [WalletErrorKind.seedRequired] for a portless host-custodied wallet) the
    /// handle is left closed but the seals + data DB are intact, so the host
    /// re-`open`s. [WalletErrorKind.invalidState] if the handle is already closed.
    pub async fn rescan_from(&mut self, from_height: Option<u32>) -> Result<(), WalletApiError> {
        match self.inner.take() {
            Some(wallet) => {
                // On error the wallet is consumed (the core `rescan_from` takes `self`
                // by value); `inner` stays `None`, so the handle is closed and the host
                // re-opens — the seals + data DB are untouched on every error path.
                let rebuilt = crate::convert::rescan_wallet(wallet, from_height).await?;
                self.inner = Some(rebuilt);
                Ok(())
            }
            None => Err(crate::convert::handle_closed()),
        }
    }

    /// The account birthday height — the scan floor a rescan rebuilds from and
    /// the lowest useful `rescanFrom` target. Clamp a rescan date-picker's
    /// DEFAULT with this instead of a blind "one year ago": map the default
    /// date to a height with `estimateBirthday` and take the max of the two —
    /// on any wallet younger than a year the blind default silently costs
    /// hours of scanning. Do NOT max-clamp a date the USER picked: an
    /// explicitly earlier pick is the post-restore recovery path (this height
    /// may itself be a too-recent restore estimate, and rescan exists
    /// precisely to go below it), so clamping it up to this floor silently
    /// no-ops that recovery. `null` before the account is provisioned (first
    /// sync hasn't run). A height, never money.
    /// [WalletErrorKind.invalidState] if the handle is closed.
    pub async fn birthday_height(&self) -> Result<Option<u32>, WalletApiError> {
        let wallet = self
            .inner
            .as_ref()
            .ok_or_else(crate::convert::handle_closed)?;
        let h = wallet
            .birthday_height()
            .await
            .map_err(WalletApiError::from)?;
        Ok(h.map(|h| h.value()))
    }
}

#[cfg(test)]
mod tests {
    //! Bridge-OWNED behaviour only: the closed-handle (`inner == None`)
    //! contract, which the core has no equivalent for (it models "closed" by
    //! move semantics, the bridge by an `Option`). The Some-path — a real
    //! create/open → snapshot → close round-trip — needs a platform vault, so
    //! it rides the core's lifecycle tests (which DO inject a test vault) and
    //! the on-device GA e2e, not a bridge unit test.
    //!
    //! OWED to the on-device e2e (recorded in the spec §8 register so it can't
    //! fall through the cracks): (1) the live Some→None `close` transition
    //! (idempotency through a REAL open wallet — here only the fast `None`
    //! branch is reachable by construction); (2) that two concurrent
    //! `snapshot()` calls on one handle actually OVERLAP (proves the read lock
    //! the generated wiring takes is shared, not accidentally exclusive after a
    //! future FRB bump); (3) that the bridged `current_address` returns a
    //! G7-valid (Orchard+Sapling, no-transparent) UA and `snapshot` carries no
    //! key bytes — the boundary, asserted on the artifact, not just statically;
    //! (4) the live `watch_sync_status` pump end-to-end (a real `StreamSink`
    //! needs a Dart port — not constructible in a Rust unit test): that a
    //! subscription emits the current status first, advances on `start_sync`,
    //! coalesces latest-wins, survives a `Stalled` fault as DATA, and ENDS on
    //! `close` after draining the final status WITHOUT blocking `close`'s write
    //! lock. The closed-handle path is the IDENTICAL `inner.as_ref().ok_or` shape
    //! proven by `start_sync` below, and the pump's drain-then-EOF teardown +
    //! host-cancel exit are pinned at the source in `zec-wallet-core`'s
    //! `watch_sync_status_streams_current_then_drains_final_and_ends_on_close`
    //! and `status_pump_delivers_changes_and_stops_when_the_sink_reports_cancelled`.
    //!
    //! SWAP closed-handle contract: `enable_near_swap`/`swap_quote`/`swap_execute` take
    //! no `StreamSink`, so their `inner.as_ref().ok_or` shape IS unit-testable here (the
    //! `swap_*_on_a_closed_handle_*` tests below — a closed handle is `SwapStateUnavailable`,
    //! the swap-vocabulary "closed", not a panic-across-FFI). `watch_swap_status` is the
    //! ONE swap method that needs a real `StreamSink` (a Dart port — not constructible in a
    //! Rust unit), so its closed-handle path is OWED to the on-device e2e exactly like
    //! `watch_sync_status` above (same `inner.as_ref().ok_or` guard, fired BEFORE the sink
    //! is touched — proven by the three sibling swap tests). ALSO OWED to that e2e (a real
    //! OPEN wallet, no bridge unit reaches it): the kill-layer-2 path — `swap()==None` ⇒
    //! `SwapDisabled` on each of `swap_quote`/`swap_execute`/`watch_swap_status` (distinct
    //! from the closed-handle `SwapStateUnavailable`) — and `enable_near_swap`'s no-adapter
    //! `SwapDisabled` (a `--no-default-features` build, covered by the feature-policy tier).
    //! The inbound-conversion edges those methods drive (id cap, out-of-range/clamp/drop,
    //! `swap_kill_in` fail-safe, Unknown rejections) ARE pinned, in `convert.rs`'s tests.
    use super::*;
    use crate::api::config::{JitterPolicy, Network, SeedPersistence, TorPolicy};
    use crate::api::error::{LifecyclePhase, SwapErrorKind, WalletErrorKind};
    use crate::convert::FFI_CLOSED_CODE;

    fn closed_handle() -> WalletHandle {
        WalletHandle { inner: None }
    }

    /// A bridge config pointing at `db_dir` whose `endpoint_url` is one that
    /// `WalletConfig::try_from` REJECTS — remote `http://` (plaintext to a
    /// non-loopback host) is a typed `InvalidEndpoint` for create/open. Using a
    /// rejected endpoint here ACTIVELY proves the validation-bypass contract:
    /// `wallet_exists` looks at `dbDir` only, so it must still answer
    /// presence/absence here, NEVER surface `InvalidEndpoint`. A future
    /// regression that routed the probe through `try_from` would flip the
    /// empty-dir test to an error and the corrupt test's kind off `StoreCorrupt`
    /// — both fail loudly.
    fn cfg_at(db_dir: &std::path::Path) -> WalletConfig {
        WalletConfig {
            db_dir: db_dir.to_string_lossy().into_owned(),
            network: Network::Main,
            endpoint_url: "http://remote.example.com:9067".to_string(),
            endpoint_auth_header: None,
            endpoint_auth_value: None,
            tor: TorPolicy::Off,
            seed_persistence: SeedPersistence::SealedKeychain,
            birthday_height: None,
            broadcast_jitter: JitterPolicy::None,
            // FR-27 is opt-in: no read scope on the default test config.
            machine_memo_prefixes: Vec::new(),
            // P3-13: nothing offered — the default plus a custom entry.
            sync_servers: None,
        }
    }

    #[tokio::test]
    async fn wallet_exists_is_false_for_an_empty_dir() {
        // The boot-fork probe across the bridge: an empty data dir reads as
        // absent (→ the host offers create), and never errors on a bare stat.
        let dir = tempfile::tempdir().expect("tempdir");
        match WalletHandle::wallet_exists(cfg_at(dir.path())).await {
            Ok(present) => assert!(!present, "an empty dir holds no wallet"),
            Err(_) => panic!("probing an empty dir must not error"),
        }
    }

    #[tokio::test]
    async fn wallet_exists_surfaces_store_corrupt_as_a_typed_kind() {
        // The error path the host's onboarding classifier routes to the
        // recover-from-phrase step: a completion marker with no manifest crosses
        // the bridge as a TYPED `StoreCorrupt` kind (not "absent", not a panic,
        // not a string to match). Pins the `WalletError → WalletApiError` mapping
        // for the probe path end-to-end.
        let dir = tempfile::tempdir().expect("tempdir");
        // the core's marker name (a public const) — no magic string, no drift
        std::fs::write(
            dir.path()
                .join(zec_wallet_core::constants::COMPLETE_MARKER_FILE_NAME),
            b"",
        )
        .expect("write a completion marker with no manifest");

        match WalletHandle::wallet_exists(cfg_at(dir.path())).await {
            Ok(_) => panic!("a corrupt store must not read as a clean result"),
            Err(err) => assert!(
                matches!(err.kind, WalletErrorKind::StoreCorrupt),
                "corruption surfaces as the typed StoreCorrupt kind, got {:?}",
                err.code
            ),
        }
    }

    #[tokio::test]
    async fn wallet_exists_surfaces_a_degraded_filesystem_as_typed_io_not_absent() {
        // THE MONEY-ROUTING boundary on a degraded mobile filesystem: a probe
        // that hits a non-`NotFound` IO error (here a `dbDir` that is a FILE, not
        // a directory — modelling iOS data-protection-locked / Android pre-unlock
        // storage where the stat is denied) must cross the bridge as a TYPED `Io`
        // kind (RW-IO-001), NEVER as `Ok(false)`. The host classifier routes a
        // thrown error to a RETRYABLE failure, but `Ok(false)` to "offer create" —
        // so a false here would invite a SECOND wallet over a real-but-unreadable
        // one. Pins that the degraded-FS error reaches Dart typed, not as absence.
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("not-a-directory");
        std::fs::write(&file, b"x").expect("write a regular file");

        match WalletHandle::wallet_exists(cfg_at(&file)).await {
            Ok(_) => panic!("a degraded-FS probe must error, never read a clean bool"),
            Err(err) => {
                assert!(
                    matches!(err.kind, WalletErrorKind::Io),
                    "a non-NotFound IO error crosses as the typed Io kind, got {:?}",
                    err.code
                );
                assert_eq!(err.code, "RW-IO-001", "the stable Io code");
            }
        }
    }

    #[tokio::test]
    async fn wipe_and_custody_disclosure_never_validate_the_endpoint() {
        // FR-14 H1 (review HARDENING — duress safety): a crypto-shred / panic-wipe
        // and its pre-wipe honesty probe must NEVER be blockable by an unrelated
        // transport-config validation — a host kill-switch may assemble a minimal
        // config, or a stored endpoint may have drifted invalid. `cfg_at` carries a
        // `try_from`-REJECTED endpoint (remote `http://`); both verbs consume ONLY
        // `dbDir`, so on a random empty dir they return Ok (no-op) or a
        // keychain/store error — but NEVER `InvalidEndpoint`. A regression that
        // re-routed these through `WalletConfig::try_from` would flip this loudly.
        // (The db_dir is a fresh tempdir ⇒ a unique keychain namespace ⇒ the probe
        // touches no real keys.)
        // A wipe quiesces the process device log (S5): serialise with the tests
        // that assert on it.
        let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
        let dir = tempfile::tempdir().expect("tempdir");
        let not_endpoint = |err: WalletApiError, verb: &str| {
            assert!(
                !matches!(err.kind, WalletErrorKind::InvalidEndpoint { .. }),
                "{verb} must not validate the endpoint, got {:?}",
                err.code
            );
        };
        if let Err(e) = WalletHandle::wipe(cfg_at(dir.path())).await {
            not_endpoint(e, "wipe");
        }
        if let Err(e) = WalletHandle::wipe_force(cfg_at(dir.path())).await {
            not_endpoint(e, "wipe_force");
        }
        if let Err(e) = WalletHandle::custody_disclosure(cfg_at(dir.path())).await {
            not_endpoint(e, "custody_disclosure");
        }
    }

    // the api error/state DTOs deliberately do NOT derive `Debug` (so an
    // amount-bearing state can't be Debug-logged — the §5.4 posture), hence
    // `match` rather than `expect`/`expect_err`.
    fn assert_closed_err(err: WalletApiError) {
        // the one bridge-originated error: a greppable code + the InvalidState
        // taxonomy so a host renders "wallet closed", never matches a string
        assert_eq!(err.code, FFI_CLOSED_CODE);
        assert!(matches!(
            err.kind,
            WalletErrorKind::InvalidState {
                phase: LifecyclePhase::Closing
            }
        ));
    }

    /// A bridge config at `db_dir` whose endpoint PASSES `WalletConfig::try_from`
    /// (a public HTTPS endpoint) — for the restore paths that must get PAST the
    /// config door to exercise the mnemonic crossing. (`cfg_at` deliberately uses a
    /// REJECTED endpoint; this is its valid sibling.)
    fn valid_cfg_at(db_dir: &std::path::Path) -> WalletConfig {
        WalletConfig {
            endpoint_url: "https://zec.rocks:443".to_string(),
            ..cfg_at(db_dir)
        }
    }

    fn words(phrase: &str) -> Vec<String> {
        phrase.split_whitespace().map(str::to_owned).collect()
    }

    #[tokio::test]
    async fn restore_validates_config_before_touching_the_seed() {
        // The inbound mnemonic crossing runs the SAME config door as create/open
        // FIRST: a rejected endpoint surfaces typed `InvalidEndpoint` before any
        // seed material is constructed (a bad endpoint can never slip a restore
        // through). Portable across platforms — `try_from` precedes the vault, so
        // no keychain is touched. The words here are valid + irrelevant (the door
        // fails first); they prove the ordering, not the BIP39 path.
        let dir = tempfile::tempdir().expect("tempdir");
        let recovery = words(
            "abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon about",
        );
        match WalletHandle::restore(cfg_at(dir.path()), recovery, None).await {
            Ok(_) => panic!("a rejected endpoint must not restore a wallet"),
            Err(err) => assert!(
                matches!(err.kind, WalletErrorKind::InvalidEndpoint { .. }),
                "a bad endpoint is a typed InvalidEndpoint before any seed work, got {:?}",
                err.code
            ),
        }
    }

    // The next two drive the mnemonic crossing through to BIP39 validation, which
    // sits behind `platform_vault()`. On Apple targets the vault constructs with no
    // I/O and validation runs before any keychain seal, so an invalid phrase is a
    // deterministic typed reject; on a vault-less target `Wallet::create` short-
    // circuits to `VaultAbsent` BEFORE validation (its own honest behaviour), so
    // these are Apple-gated. The valid-phrase round-trip needs a real seal and rides
    // the core's vault-injected tests + on-device e2e (spec §8), never a bridge unit.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[tokio::test]
    async fn restore_surfaces_an_invalid_phrase_as_a_typed_word_index() {
        // Serialised with the sever rows: a door refuses while any sever is
        // registering (ADR-0566), which would read here as a wrong error.
        let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
        // §5.4 money posture across the FFI boundary: a mistyped recovery word
        // crosses back as a typed `InvalidMnemonic` carrying the offending word
        // INDEX only — never the word value, never a panic, never a string to
        // match. `zzzz` at index 1 of an otherwise-valid 12-word phrase.
        let dir = tempfile::tempdir().expect("tempdir");
        let recovery = words(
            "abandon zzzz abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon about",
        );
        match WalletHandle::restore(valid_cfg_at(dir.path()), recovery, None).await {
            Ok(_) => panic!("an invalid recovery phrase must not restore a wallet"),
            Err(err) => assert!(
                matches!(
                    err.kind,
                    WalletErrorKind::InvalidMnemonic {
                        word_index: Some(1)
                    }
                ),
                "the bad word's INDEX (never the word) crosses the bridge, got {:?}",
                err.code
            ),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[tokio::test]
    async fn restore_with_an_empty_word_list_is_a_typed_invalid_mnemonic() {
        let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
        // Defensive edge: an empty `List<String>` from the host joins to "" and
        // must be a typed `InvalidMnemonic` (wrong word count), never a panic on
        // the empty-input join/parse.
        let dir = tempfile::tempdir().expect("tempdir");
        match WalletHandle::restore(valid_cfg_at(dir.path()), Vec::new(), None).await {
            Ok(_) => panic!("an empty word list is not a wallet"),
            Err(err) => assert!(
                matches!(err.kind, WalletErrorKind::InvalidMnemonic { .. }),
                "an empty phrase is a typed InvalidMnemonic, got {:?}",
                err.code
            ),
        }
    }

    #[tokio::test]
    async fn restore_rejects_an_oversized_word_list_before_allocating() {
        let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
        // MOBILE robustness (size-cap-before-alloc, §4.6): a pathological clipboard
        // paste — a huge word LIST, or a single enormous "word" — is a typed
        // `InvalidMnemonic` rejected by the byte-cap BEFORE the `.join` allocates,
        // never an OOM/jetsam on a low-memory device. PORTABLE (no platform gate):
        // the cap returns inside `restore_wallet` BEFORE `Wallet::create` touches the
        // vault, so it is deterministic on every target. Both shapes over the cap:
        let dir = tempfile::tempdir().expect("tempdir");
        let many = vec!["abandon".to_string(); 10_000];
        match WalletHandle::restore(valid_cfg_at(dir.path()), many, None).await {
            Ok(_) => panic!("a 10000-word list is not a wallet"),
            Err(err) => assert!(
                matches!(
                    err.kind,
                    WalletErrorKind::InvalidMnemonic { word_index: None }
                ),
                "an oversized list is a typed InvalidMnemonic (whole-phrase, no index), got {:?}",
                err.code
            ),
        }
        let one_giant = vec!["a".repeat(5_000)];
        match WalletHandle::restore(valid_cfg_at(dir.path()), one_giant, None).await {
            Ok(_) => panic!("a single 5000-byte token is not a wallet"),
            Err(err) => assert!(
                matches!(err.kind, WalletErrorKind::InvalidMnemonic { .. }),
                "a single oversized token is a typed InvalidMnemonic, got {:?}",
                err.code
            ),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[tokio::test]
    async fn restore_surfaces_an_uppercase_word_as_a_typed_word_index() {
        let _serial = crate::device_log::test_sink::PROCESS_LOG.lock().await;
        // REAL-WORLD MOBILE EDGE: a soft keyboard auto-capitalizes the FIRST word of
        // a pasted recovery phrase (`"Abandon …"`). BIP39's word list is lowercase and
        // the audited `parse_in_normalized` does NOT case-fold, so the capitalized
        // word crosses the bridge as a typed `InvalidMnemonic` carrying its INDEX only
        // (§5.4 / §6.1 — never the word value, never a panic, never a string to match).
        // This is the bridge-level proof of the core characterization
        // (`restore_rejects_uppercase_words_so_the_host_must_lowercase`): the HOST
        // restore UI MUST lowercase the words before this crossing — the SDK reports
        // the offending index, it does not silently re-case. A future bip39 that began
        // case-folding would FLIP this to a successful restore and fail the test, a
        // deliberate review trigger for the lowercase-before-restore host contract.
        let dir = tempfile::tempdir().expect("tempdir");
        let recovery = words(
            "Abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon about",
        );
        match WalletHandle::restore(valid_cfg_at(dir.path()), recovery, None).await {
            Ok(_) => panic!(
                "an uppercase recovery word restored — bip39 began case-folding; \
                 revisit the host lowercase-before-restore contract"
            ),
            Err(err) => assert!(
                matches!(
                    err.kind,
                    WalletErrorKind::InvalidMnemonic {
                        word_index: Some(0)
                    }
                ),
                "the capitalized word's INDEX (never the word) crosses the bridge, got {:?}",
                err.code
            ),
        }
    }

    #[tokio::test]
    async fn snapshot_on_a_closed_handle_is_typed_not_a_panic() {
        match closed_handle().snapshot().await {
            Ok(_) => panic!("a closed handle must not read a snapshot"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn current_address_on_a_closed_handle_is_typed_not_a_panic() {
        match closed_handle().current_address().await {
            Ok(_) => panic!("a closed handle has no address to read"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn rescan_from_on_a_closed_handle_is_typed_not_a_panic() {
        // ADR-0534 bridge: rescan on a closed handle (inner == None) is the typed
        // bridge `invalidState`, never a panic-across-FFI — and never touches the
        // moved-out wallet. The `&mut self` take-and-replace short-circuits on `None`
        // before any core call. (The Some-path rebuild rides the core `rescan_from_*`
        // tests + the on-device e2e; no platform vault exists in a bridge unit.)
        let mut handle = closed_handle();
        match handle.rescan_from(None).await {
            Ok(()) => panic!("a closed handle cannot rescan"),
            Err(err) => assert_closed_err(err),
        }
    }

    // The send surface (inc-2d-ffi) shares the closed-check shape: a propose/send/queue_send
    // fired after close (or racing a dispose / a double-tap) is a typed bridge error, never a
    // panic-across-FFI — and never touches the moved-out wallet. The Some-path (a real funded
    // propose→send) rides the core `Wallet` tests (`propose_uri_*`/`send_by_id_*`/`queue_send_*`)
    // + the on-device e2e; no platform vault exists in a bridge unit.

    #[tokio::test]
    async fn propose_on_a_closed_handle_is_typed_not_a_panic() {
        match closed_handle().propose("zcash:addr".to_string()).await {
            Ok(_) => panic!("a closed handle cannot propose a send"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn send_on_a_closed_handle_is_typed_not_a_panic() {
        match closed_handle().send(1).await {
            Ok(_) => panic!("a closed handle cannot send"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn queue_send_on_a_closed_handle_is_typed_not_a_panic() {
        match closed_handle().queue_send("zcash:addr".to_string()).await {
            Ok(_) => panic!("a closed handle cannot queue a send"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn reveal_mnemonic_on_a_closed_handle_is_typed_not_a_panic() {
        // the sanctioned outbound key crossing shares the closed-check shape: a
        // reveal after close (or racing a dispose) is a typed bridge error, never
        // a panic-across-FFI — and never an attempt to read a moved-out wallet's
        // (already-zeroized) seed. The Some-path reveal (real words out) rides the
        // core's vault-injected tests; no platform vault exists in a bridge unit.
        match closed_handle().reveal_mnemonic().await {
            Ok(_) => panic!("a closed handle has no mnemonic to reveal"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn machine_memos_on_a_closed_handle_is_typed_not_a_panic() {
        // FR-27 shares the closed-check shape with every other read: a memo read
        // after close (or racing a provider dispose) is a typed bridge error,
        // never a panic across FFI. The Some-path — the scope refusal, the
        // prefix filter and both polarities — rides the core's tests; no
        // platform vault exists in a bridge unit test.
        match closed_handle().machine_memos("00".repeat(32)).await {
            Ok(_) => panic!("a closed handle has no transaction to read"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn machine_memos_checks_the_handle_before_the_txid() {
        // ORDER matters on a closed handle: a garbage txid must NOT produce
        // `TxidInvalid` from a wallet that no longer exists — the honest answer
        // to "read this after close" is "the handle is closed", whatever else
        // the caller got wrong. (The reverse order would also make the hex door
        // a liveness oracle on a disposed handle.)
        match closed_handle()
            .machine_memos("not-a-txid".to_string())
            .await
        {
            Ok(_) => panic!("a closed handle reads nothing"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn close_is_idempotent_on_an_already_closed_handle() {
        // a close button racing a provider dispose (or a double-tap) must NOT
        // surface an error for "already closed" — it is a successful no-op
        let mut handle = closed_handle();
        assert!(handle.close().await.is_ok(), "idempotent close is Ok");
        // and still Ok a second time — the None branch never errors
        assert!(handle.close().await.is_ok(), "redundant close is Ok");
    }

    #[tokio::test]
    async fn start_sync_on_a_closed_handle_is_typed_not_a_panic() {
        // start_sync shares the closed-check shape with watch_sync_status (which
        // a unit test cannot drive — it needs a real StreamSink/Dart port), so
        // this pins that the closed branch is a typed bridge error, not a panic
        match closed_handle().start_sync().await {
            Ok(()) => panic!("a closed handle cannot start sync"),
            Err(err) => assert_closed_err(err),
        }
    }

    #[tokio::test]
    async fn stop_sync_on_a_closed_handle_is_a_noop() {
        // mirrors close(): stopping a wallet that is already gone is a successful
        // no-op (a lifecycle teardown racing a stop button must not error)
        assert!(
            closed_handle().stop_sync().await.is_ok(),
            "stop_sync on a closed handle is a no-op Ok"
        );
    }

    /// The swap-surface closed-handle assertion (the [`assert_closed_err`] parallel): a
    /// swap op on a torn-down handle is the typed swap-vocabulary `SwapStateUnavailable`
    /// ("swap unavailable, retry"), never a panic-across-FFI. The guard fires BEFORE the
    /// request/quote/config is inspected or `wallet.swap()` is reached.
    fn assert_swap_closed_err(err: SwapApiError) {
        // SwapErrorKind has no Debug (a §5.4 anti-leak discipline), so the message
        // carries the stable code instead of the kind.
        assert!(
            matches!(err.kind, SwapErrorKind::SwapStateUnavailable),
            "a swap op on a closed handle must be SwapStateUnavailable (got code {})",
            err.code
        );
    }

    /// A minimal IntoZec quote request — any well-formed value works, since the
    /// closed-handle guard returns before the request is read.
    fn a_quote_request() -> QuoteRequest {
        use crate::api::swap::{AssetId, ExactSide, SwapAmount, SwapDirection};
        QuoteRequest {
            direction: SwapDirection::IntoZec {
                from: AssetId {
                    chain: "eth".to_string(),
                    symbol: "usdc".to_string(),
                },
            },
            exact: ExactSide::In {
                amount: SwapAmount::Foreign {
                    amount: "100".to_string(),
                },
            },
            slippage_tolerance_bps: 200,
            destination: None,
            refund_address: None,
        }
    }

    #[tokio::test]
    async fn enable_near_swap_on_a_closed_handle_is_swap_state_unavailable() {
        // the §3.5 on-switch on a gone handle is typed, not a panic — and never reaches
        // the (feature-gated) provider construction.
        let cfg = SwapProviderConfig {
            endpoint: "https://1click.example".to_string(),
            jwt: None,
        };
        match closed_handle().enable_near_swap(cfg, true, None).await {
            Ok(()) => panic!("a closed handle cannot enable swap"),
            Err(err) => assert_swap_closed_err(err),
        }
    }

    #[tokio::test]
    async fn swap_quote_on_a_closed_handle_is_swap_state_unavailable() {
        match closed_handle().swap_quote(a_quote_request()).await {
            Ok(_) => panic!("a closed handle cannot quote a swap"),
            Err(err) => assert_swap_closed_err(err),
        }
    }

    #[tokio::test]
    async fn swap_execute_on_a_closed_handle_is_swap_state_unavailable() {
        // a fabricated quote never reaches the durable claim — the closed guard fires
        // first; any well-formed SwapQuote DTO suffices (it is not inspected).
        use crate::api::swap::{DisclosureItem, SwapPrivacyDisclosure};
        let quote = SwapQuote {
            id: "deposit-addr".to_string(),
            binding: None,
            deposit_address: "deposit-addr".to_string(),
            deposit_memo: None,
            expires_at: 2_000_000_000,
            amount_in: "100".to_string(),
            min_amount_out: "0.9".to_string(),
            zec_side_zat: 100_000_000,
            refund_to: None,
            disclosure: SwapPrivacyDisclosure {
                ends_shielded: true,
                deshields: false,
                provider_legs_transparent: true,
                provider_sees: vec![DisclosureItem::Amounts],
            },
        };
        match closed_handle().swap_execute(quote).await {
            Ok(_) => panic!("a closed handle cannot execute a swap"),
            Err(err) => assert_swap_closed_err(err),
        }
    }
}
