//! §6.3 — the crash-atomic wallet store: two-phase provisioning + the
//! completion-marker create-vs-open guard. The synchronous orchestration the
//! async `Wallet` handle (inc-2b-ii) drives through `spawn_blocking` while
//! holding the single-writer lock (lifecycle.rs).
//!
//! **Ground rule (§6.3): the chain is the source of truth; the wallet DB is a
//! cache.** No kill window can lose FUNDS — only local bookkeeping. This module
//! owns the `create / restore` window: seal the seed + DB key + init the DB,
//! then write a completion marker LAST (CAS — never clobbers an existing
//! wallet). A marker-less remnant is `ProvisioningIncomplete`; repair resumes
//! FROM the remnant, NEVER re-derives.
//!
//! **One SealKey, two blobs, no split-brain (W3-inc-2b money-lens pin):** the
//! seed seal and the DB-key seal are produced TOGETHER in `store_wallet` under a
//! single custodied wrap key, then persisted together. There is no durable state
//! in which the seed is sealed but the DB key is not — so the split-brain the
//! pin guards against is eliminated by construction. The remnant checkpoint is
//! the MANIFEST: it is written durably AFTER the seal artifacts, so a remnant
//! always implies the seals are on disk and recoverable.
//!
//! **Repair / re-derive boundary (§6.3):** a `create`/`restore` re-run that finds
//! a remnant REPAIRS it. `Generate`-mode repair resumes from the already-sealed
//! seed (never regenerates — a fresh seed against a sealed remnant would
//! split-brain seed-vs-DB); `Mnemonic`/`RawBytes` repair verifies the supplied
//! seed against the sealed one (constant-time) — a mismatch is the mistyped-
//! mnemonic re-run, surfaced typed `SeedMismatch`.
//!
//! **Durability:** every artifact is written tmp → fsync(file) → atomic rename →
//! fsync(dir) (POSIX atomic-replace; iOS/Android/desktop-Unix). ENOSPC maps to
//! the typed `DiskFull` via the single `WalletError::from_io` door. The caller
//! holds the exclusive single-writer lock, so these writes never race.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use subtle::ConstantTimeEq;

use crate::constants::{
    COMPLETE_MARKER_FILE_NAME, DBKEY_SEAL_FILE_NAME, MANIFEST_FILE_NAME, MANIFEST_LEN_V1,
    MANIFEST_LEN_V2, MANIFEST_VERSION_V1, MANIFEST_VERSION_V2, PROVISION_MANIFEST_VERSION,
    RESCAN_TMP_DB_FILE_NAME, SEAL_FILE_MAX_BYTES, SEED_SEAL_FILE_NAME, WALLET_DB_FILE_NAME,
    WIPE_COMMITTED_FILE_NAME, WRAP_ARTIFACT_FILE_NAME,
};
use crate::custody::{CustodyId, CustodyIndexEntry};
use crate::db;
use crate::error::WalletError;
use crate::keychain::{
    BorrowedVault, KeychainPort, SealedSeedVault, VaultResolver, VaultStatus, WrapArtifact, bounded,
};
use crate::lifecycle::WalletLock;
use crate::money::Network;
use crate::seal::{SeedPayload, WalletDbKey};

/// Seed-at-rest mode, as recorded in the manifest. A `Copy` mirror of
/// `SeedPersistence` (which is intentionally non-Clone, §2.2) — the handle maps
/// one to the other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PersistenceKind {
    /// Seed sealed at rest (Dart-path default): a seed blob is persisted.
    SealedKeychain,
    /// Nothing persisted but the DB key (the host-supplied-seed path): no seed blob.
    None,
    /// #397 (spec §3.7 D4): a WATCH-ONLY wallet — NO seed exists anywhere
    /// (the account was imported from a UFVK), only the DB key is custodied
    /// (the UFVK at rest is privacy-sensitive, so the SQLCipher, sealed-DB-key
    /// and crypto-shred discipline is UNCHANGED). Written only inside a v3
    /// manifest; a v1/v2 manifest carrying this byte is corruption.
    WatchOnly,
}

impl PersistenceKind {
    fn to_byte(self) -> u8 {
        match self {
            Self::SealedKeychain => 0,
            Self::None => 1,
            Self::WatchOnly => 2,
        }
    }
    /// Decode a v1/v2 (SPENDING-manifest) persistence byte — `WatchOnly (2)`
    /// is deliberately NOT accepted here: it exists only inside a v3 manifest
    /// (`read_manifest`'s v3 arm matches it explicitly), so a v1/v2 manifest
    /// carrying it is corruption, never a silently-degraded custody mode.
    fn from_byte(b: u8) -> Result<Self, WalletError> {
        match b {
            0 => Ok(Self::SealedKeychain),
            1 => Ok(Self::None),
            _ => Err(WalletError::StoreCorrupt),
        }
    }
}

/// Seed provenance, recorded in the v2 manifest (#357, §3.2f). It gates whether
/// `store::repair` may write the creation stamp: ONLY a `Generated` remnant may
/// be stamped (provably fresh on this device / host-attested), so no
/// aborted-restore seed can inherit a creation time. A `Copy` fact derived at
/// provision from `ProvisionIntent::freshly_generated_at`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ManifestSeedSource {
    /// Freshly generated on THIS device / host-attested fresh (`SeedSource::
    /// Generate` / `FreshRawBytes` ⇒ `freshly_generated_at.is_some()`) — the
    /// only value `repair` may stamp over.
    Generated,
    /// Restored from prior history (`Mnemonic` / plain `RawBytes` ⇒
    /// `freshly_generated_at.is_none()`) — a stamp must NEVER exist for a seed
    /// whose history may predate it.
    Restored,
    /// A pre-#357 v1 manifest carried NO seed-source byte. Never stamped — the
    /// conservative default (byte-identical to pre-#357 "repair never stamps").
    /// Never ENCODED; produced only by the v1 read-back path.
    Unknown,
    /// #397 (spec §3.7 D4): a WATCH-ONLY store — there IS no seed, so seed
    /// provenance is structurally moot (`repair` never stamps it; the §3.2f
    /// stamp arms are seed-provenance logic a v3 manifest never enters).
    /// Written/read ONLY inside a v3 manifest, always paired with
    /// `PersistenceKind::WatchOnly`.
    WatchOnly,
}

impl ManifestSeedSource {
    /// Derive the provenance from the provisioning intent: the SAME fresh-vs-
    /// restore signal that gates the creation stamp (`freshly_generated_at`), now
    /// recorded durably so `repair` — which builds a fresh intent without knowing
    /// the original's freshness — can honour it.
    fn from_intent(intent: &ProvisionIntent<'_>) -> Self {
        if intent.persistence == PersistenceKind::WatchOnly {
            // #397: no seed exists — provenance is the wallet KIND itself.
            Self::WatchOnly
        } else if intent.freshly_generated_at.is_some() {
            Self::Generated
        } else {
            Self::Restored
        }
    }

    fn to_byte(self) -> u8 {
        match self {
            // 0 is reserved for "absent" (v1 read-back ⇒ Unknown); v2 always
            // writes 1/2, never 0; 3 is v3-only (watch-only).
            Self::Unknown => 0,
            Self::Generated => 1,
            Self::Restored => 2,
            Self::WatchOnly => 3,
        }
    }

    /// Decode a v2 (SPENDING-manifest) seed-source byte. `0`/`3 (WatchOnly)`/
    /// out-of-range in a v2 manifest is corruption (v2 always writes 1/2; the
    /// watch-only byte exists only inside a v3 manifest, matched explicitly in
    /// `read_manifest`'s v3 arm) — rejected, never silently degraded.
    fn from_byte(b: u8) -> Result<Self, WalletError> {
        match b {
            1 => Ok(Self::Generated),
            2 => Ok(Self::Restored),
            _ => Err(WalletError::StoreCorrupt),
        }
    }
}

/// How a repair verifies the supplied seed against the sealed remnant (§6.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RepairMode {
    /// `Generate`: resume from the sealed seed, NEVER regenerate.
    ResumeSealed,
    /// `Mnemonic`/`RawBytes`: verify the supplied seed matches the sealed one.
    VerifySupplied,
    /// `None`-persistence: no sealed seed to verify.
    NoSeed,
}

/// Everything provisioning needs from the handle. The DB key is generated
/// INSIDE provisioning (never passed in) and recovered from the remnant on
/// repair/open — so there is one and only one DB key per wallet, ever.
pub(crate) struct ProvisionIntent<'a> {
    pub(crate) network: Network,
    pub(crate) persistence: PersistenceKind,
    /// The resolved seed (SealedKeychain mode); `None` for None-persistence.
    pub(crate) seed: Option<&'a SeedPayload>,
    pub(crate) repair_mode: RepairMode,
    /// `Some(unix_secs)` ⇔ this create is FRESHLY GENERATED (`SeedSource::
    /// Generate`, or host-attested `FreshRawBytes` — FR-24; §3.2f
    /// #356-F4): provisioning writes the creation stamp so the lazy first-sync
    /// birthday can floor at ~creation time. Explicit (not derived from
    /// `repair_mode`) because `NoSeed` covers BOTH sources. `None` = restore
    /// (`Mnemonic`/plain `RawBytes`): a stamp must never exist for a seed whose
    /// history may predate it.
    pub(crate) freshly_generated_at: Option<u64>,
}

/// What `open` hands the wallet handle: the migrated `WalletDb` data handle plus
/// the recovered custody. The handle keeps the seed/db_key in Rust (never across
/// FFI).
///
/// CARRIER (inc-2b-ii-B): the async `Wallet` handle HOLDS this for the wallet's
/// lifetime (behind its lock + lifecycle); its fields are consumed field-by-field
/// by the §3.1 `&self` API methods that arrive with the sync/send engines (inc-2c/
/// 2d — `db` drives the data API, `seed`/`db_key` feed re-derivation and rekey,
/// `status` surfaces custody tier). Held-not-yet-read here, hence the allow.
pub(crate) struct OpenWallet {
    pub(crate) db: db::WalletConn,
    /// A SECOND SQLCipher-keyed connection to the SAME `wallet.db` file, dedicated
    /// to OUR aux tables (the queued-send intent store, §3.2h inc-2d-3-b-i). The
    /// `zcash_client_sqlite` `WalletDb` exposes no connection accessor, so aux SQL
    /// cannot ride the engine's handle; both connections share the one file under
    /// the one `WalletLock`, in VERIFIED WAL mode (W-swap-4-a-4 — readers never
    /// block the writer; residual writer-vs-writer contention is `busy_timeout` +
    /// the typed retryable `StoreBusy`). REKEY SEAM (W-swap-4-a-5): a future
    /// `PRAGMA rekey` over this carrier's `db_key` MUST fold the WAL first — see
    /// `db::ensure_wal`'s rekey-seam note.
    pub(crate) aux_db: rusqlite::Connection,
    pub(crate) db_key: WalletDbKey,
    /// SealedKeychain mode: `Some`; None-persistence: `None`.
    pub(crate) seed: Option<SeedPayload>,
    pub(crate) network: Network,
    pub(crate) persistence: PersistenceKind,
    pub(crate) status: VaultStatus,
    /// The creation stamp (§3.2f #356-F4), read back at open: `Some` ⇔ this
    /// wallet's seed was freshly generated at that wall-clock (the row is
    /// written only on a `Generate` / host-attested `FreshRawBytes` create).
    /// Bounds the lazy first-sync birthday; `None` (restore / pre-F4 wallet)
    /// keeps the conservative pre-F4 arms.
    pub(crate) created_at: Option<u64>,
    /// The sync-server choice row (P3-13), read back RAW at open on the
    /// blocking pool; `Wallet::from_open` resolves it against the offered
    /// list (`sync_server::resolve`). Infallible: a malformed row reads as
    /// `Unreadable` and resolves to the default with a visible fallback.
    pub(crate) sync_server_row: crate::sync_server::StoredChoice,
}

/// On-disk store state — derived from the MARKER + MANIFEST, never from the DB
/// file's existence (§6.3 create-vs-open guard).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StoreState {
    /// No manifest: nothing has been committed here.
    Empty,
    /// Manifest present, marker absent: a recoverable provisioning remnant.
    Remnant,
    /// Manifest + marker present: an openable wallet.
    Complete,
}

/// The provisioning steps, in order — the crash points the §8 fault-injection
/// test interrupts at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ProvisionStep {
    /// The path-keyed index item names the fresh custody id; the wrap key is
    /// NOT yet written (stage S16: index-first, so a key never exists without
    /// an index that names it). The window a duress sever can race.
    IndexWritten,
    /// Wrap key custodied (vault write) + both blobs sealed in memory.
    Custody,
    /// Prior-life hygiene done (destructive sweep; still nothing created on
    /// disk). Its own step (W-swap-4-a-6, →#374 crypto fold) so the
    /// fault-injection suite can kill BETWEEN the sweep and step 2 — the one
    /// window the custody-first reorder made otherwise test-unreachable.
    PriorLifeSwept,
    /// Seal artifacts + manifest durable (the remnant checkpoint).
    RemnantPersisted,
    /// Encrypted DB created + sentinel written.
    DbProvisioned,
    /// Completion marker written (LAST).
    Completed,
}

/// Inspect the store WITHOUT touching the DB or the vault (§6.3 source of truth).
pub(crate) fn inspect(db_dir: &Path) -> Result<StoreState, WalletError> {
    if file_exists(&db_dir.join(COMPLETE_MARKER_FILE_NAME))? {
        // A marker without a manifest is corruption, not a usable wallet.
        return if file_exists(&db_dir.join(MANIFEST_FILE_NAME))? {
            Ok(StoreState::Complete)
        } else {
            Err(WalletError::StoreCorrupt)
        };
    }
    if file_exists(&db_dir.join(MANIFEST_FILE_NAME))? {
        Ok(StoreState::Remnant)
    } else {
        Ok(StoreState::Empty)
    }
}

/// `create`/`restore` entry (§6.3): provision a fresh wallet, repair a remnant,
/// or refuse to clobber a completed one. The `lock` witness IS the single-writer
/// guarantee — `db_dir` is the lock's own directory, so reaching this path
/// without holding the exclusive lock is not expressible (lifecycle.rs). After
/// this returns Ok, [`open`] yields the handle.
///
/// Kept vault-shaped signature (the pre-S2 callers); production resolves
/// namespaces through [`create_or_repair_resolving`]. TEST-SEAM (dead in a
/// non-test build).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn create_or_repair(
    lock: &WalletLock,
    vault: &dyn KeychainPort,
    intent: &ProvisionIntent,
) -> Result<(), WalletError> {
    create_or_repair_resolving(lock, &BorrowedVault(vault), intent)
}

/// [`create_or_repair`] over the S2 resolver: a fresh provision mints the
/// custody identifier and custodies under ITS namespace (the path-keyed index
/// item is written with it, before any disk write); a remnant repair recovers
/// custody from whatever namespace the artifact's header names.
pub(crate) fn create_or_repair_resolving(
    lock: &WalletLock,
    resolver: &dyn VaultResolver,
    intent: &ProvisionIntent,
) -> Result<(), WalletError> {
    let db_dir = lock.dir();
    match inspect(db_dir)? {
        StoreState::Complete => Err(WalletError::WalletAlreadyExists),
        StoreState::Remnant => repair_resolving(db_dir, resolver, intent),
        StoreState::Empty => provision_resolving_with(db_dir, resolver, intent, &mut |_| Ok(())),
    }
}

/// Open an existing, completed wallet (§6.3). `NotFound` if nothing is here;
/// `ProvisioningIncomplete` for a marker-less remnant; `NetworkMismatch` if the
/// config network disagrees with the manifest. The `lock` witness binds the
/// target `db_dir` and proves exclusive ownership (see [`create_or_repair`]).
/// Migrates the DB up at open (idempotent — `db::open_db_migrated`).
///
/// Kept vault-shaped signature (the pre-S2 callers); production resolves
/// namespaces through [`open_resolving`]. TEST-SEAM (dead in a non-test build).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn open(
    lock: &WalletLock,
    vault: &dyn KeychainPort,
    cfg_network: Network,
) -> Result<OpenWallet, WalletError> {
    open_resolving(lock, &BorrowedVault(vault), cfg_network)
}

/// [`open`] over the S2 resolver: the wrap artifact's header names the custody
/// namespace (no id ⇒ the legacy path namespace, and the pre-stage migration
/// runs — see [`load_custody_resolving`]); the index item under the path
/// namespace is kept current at every successful open (written only when
/// missing or wrong — [`settle_index_after_open`]).
pub(crate) fn open_resolving(
    lock: &WalletLock,
    resolver: &dyn VaultResolver,
    cfg_network: Network,
) -> Result<OpenWallet, WalletError> {
    let db_dir = lock.dir();
    match inspect(db_dir)? {
        StoreState::Empty => return Err(WalletError::NotFound),
        StoreState::Remnant => return Err(WalletError::ProvisioningIncomplete),
        StoreState::Complete => {}
    }
    let manifest = read_manifest(db_dir)?;
    if manifest.network != cfg_network {
        return Err(WalletError::NetworkMismatch);
    }
    let loaded_custody = load_custody_resolving(db_dir, resolver, manifest.persistence)?;
    let (seed, db_key, status) = (
        loaded_custody.seed,
        loaded_custody.db_key,
        loaded_custody.status,
    );
    // A `.wipe-committed` breadcrumb in a wallet that just UNSEALED is provably
    // stale (W-swap-4-a-6 (b) upgrade arm, →#374 arch review): the
    // breadcrumb claims this namespace's custody is severed, but `load_custody`
    // succeeding IS the proof the wrap key is alive — so it belongs to a dead
    // prior life (a successor provisioned into a dirty dir BEFORE the 4-a-6
    // provision hygiene existed) and would disarm THIS life's destroy-time
    // verify-real-sever guard. Sweeping at open cures already-provisioned
    // wallets the provision-time sweep can no longer reach — REACH IS
    // OPEN-GATED (final review, honest residual): a pre-fix wallet WIPED
    // without ever opening post-upgrade keeps the disarm, because at destroy
    // an inherited breadcrumb is indistinguishable from this life's own
    // interrupted wipe (the legitimate converge exit). Pre-existing, needs a
    // triple coincidence (pre-fix dirty-dir provision + zero post-upgrade
    // opens + a B1 mis-derived namespace at wipe). Honest IO faults
    // propagate (same posture as the provision hygiene).
    db::remove_path_if_present(&db_dir.join(WIPE_COMMITTED_FILE_NAME))?;
    // …unless a duress sever of THIS path proved its sever between our unseal
    // and the sweep above: its breadcrumb is this life's, not a dead one's.
    restore_sever_breadcrumb(db_dir)?;
    // Thread the loaded custody seed into the migrate-up retry (§3.2b): a
    // post-account seed-validating migration can then run; `None`-mode passes
    // `None` and surfaces SeedRequired if such a migration is ever pending. The
    // borrow ends before `seed` moves into `OpenWallet` below.
    let db_path = db_dir.join(WALLET_DB_FILE_NAME);
    let db = db::open_db_migrated(
        &db_path,
        &db_key,
        manifest.network,
        seed.as_ref().map(|p| p.seed()),
    )?;
    // Open the aux connection AFTER the migrate-up so the `queued_send_intent` table
    // (created in `db::migrate`) is already on disk. Same file, same key, its own
    // handle — keyed-open applies `PRAGMA key` FIRST and the shared `busy_timeout`.
    let mut aux_db = db::open_existing_keyed_connection(&db_path, &db_key)?;
    // ADR-0568: the aux connection holds a user's custom-server key; an erased
    // one is zeroed, not freed. SQLCipher's keyed default already says so;
    // this makes it explicit and verified on the handle that holds the key.
    db::ensure_secure_delete(&aux_db)?;
    // The creation stamp (§3.2f #356-F4) — read EARLY (#387): besides bounding the lazy
    // first-sync birthday below, it is the PROVENANCE signal the restore-pessimistic
    // seed keys on (`Some` ⇔ Generate-created on THIS device). A money input, but only
    // ever DOWNWARD (a widened scan window / a bounded restore sweep) — so the read is
    // INFALLIBLE: any fault degrades to `None` (the pre-F4 arms + the CONSERVATIVE
    // restore-seed arm — a sweep, never a missed refund; review + #387).
    let created_at = crate::creation_stamp::read(&aux_db);
    // #382 — seed the backfill bounds at the QUIESCENT point: post-migrate, pre-service,
    // under the exclusive open lock, so no quote's reserve→persist gap can straddle the
    // ceiling snapshot (the seed-race MED; `refund_index::seed_backfill_bounds`
    // doc). First-wins — a no-op on every open after the first. PROPAGATES (not
    // best-effort like the sweeps below): the seed is a correctness bound for the §4.4
    // item-7 privacy envelope, it is one singleton `INSERT OR IGNORE` on a connection
    // migrate just wrote through, and a store that cannot take it is genuinely sick.
    // Documented availability trade: a bit-corrupt/tampered refund
    // COUNTER row (out-of-range `next_index`) now fails `open` typed `StoreCorrupt`
    // where pre-#382 it opened and only the swap reserve failed — deliberate
    // (fail-closed on sealed-store corruption is the crate doctrine; no legitimate
    // state is rejected: absent row ⇒ FLOOR, exhausted ⇒ the 2^31 allowance) —
    // recovery is restore-from-phrase, the same as every other StoreCorrupt at open.
    // #387 — the FIRST-EVER seed is RESTORE-PESSIMISTIC (spec §3.2h item 5): a wallet
    // whose counter row is absent and which was NOT Generate-created here seeds
    // counter = ceiling = RESTORE_BACKFILL_BREADTH, so the standard backfill restores
    // in-app visibility of pre-restore refunds within the breadth.
    crate::refund_index::seed_from_counter_or_restore_floor(&aux_db, created_at.is_some())?;
    // FR-8 (ADR-0537) — the diversified-receive counter's own #387-pattern
    // restore-pessimistic first seed: same provenance signal, no bounds/backfill
    // sibling (shielded trial decryption needs none).
    crate::diversified_index::seed_restore_pessimistic(&aux_db, created_at.is_some())?;
    // Open-time issued-quote sweep (W-swap-4-a-4): free `MAX_ISSUED_QUOTES` capacity
    // held by lapsed AND pre-clamp far-future rows (the far-future arm self-disables
    // below the clock-plausibility floor inside the store) — a capacity-bricked
    // pre-upgrade wallet un-bricks on its next launch, before any quote is attempted.
    // BEST-EFFORT like the creation-stamp read below: hygiene only (a dead row can
    // never execute — `take` re-checks both bounds), so a sweep fault must not brick
    // `open`; any real store fault surfaces typed on the next aux WRITE that matters.
    let sweep_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    let _ = crate::issued_quote_store::prune_expired(&mut aux_db, sweep_now);
    // P3-13: the user's sync-server choice, raw. Read HERE (the blocking pool)
    // so `from_open`'s resolution is pure; infallible like `created_at`.
    let sync_server_row = crate::sync_server::read(&aux_db);
    // (`created_at` was read above, pre-seed — the #387 provenance reorder.)
    Ok(OpenWallet {
        db,
        aux_db,
        db_key,
        seed,
        network: manifest.network,
        persistence: manifest.persistence,
        status,
        created_at,
        sync_server_row,
    })
}

// ── Rescan: rebuild the data DB at an earlier birthday (ADR-0534) ─────────────

/// The rescan rebuild steps, in order — the crash points the §8 fault-injection
/// test (`rescan_is_crash_safe`) interrupts at. The SEAL artifacts + vault are
/// touched at NONE of them, so the seed cannot be lost at any interruption.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RescanStep {
    /// A fresh, ACCOUNT-LESS data DB is provisioned at the temp path under the
    /// SAME db_key (the old `wallet.db` is still the live, intact wallet).
    TempProvisioned,
    /// The durable aux tables are copied INTO the temp DB (the old DB only read).
    AuxCopied,
    /// The temp DB is fsync'd + atomic-renamed OVER `wallet.db` (the one atomic
    /// replace — after this the lower-birthday DB is the live wallet).
    Renamed,
}

/// Rebuild ONLY the data DB at a fresh, ACCOUNT-LESS state under the SAME `db_key`,
/// PRESERVING the durable aux tables, then atomically replace `wallet.db` (ADR-0534
/// post-restore recovery). The seal artifacts + keychain vault are NEVER touched, so
/// the seed cannot be lost at ANY interruption point; the zcash account/notes/
/// witnesses are re-derivable from the seed at the lower birthday and so are dropped.
///
/// Crash-atomic, mirroring the proven provisioning discipline: the new DB is fully
/// built (schema + aux copy) at a TEMP path, fsync'd, then atomic-renamed over
/// `wallet.db` with a dir fsync — so `wallet.db` is always intact-old or complete-new,
/// never torn. A kill before the rename leaves the old wallet (old birthday) live; a
/// kill after leaves the new wallet (lower birthday) live; either re-`open`s.
///
/// The caller holds the exclusive [`WalletLock`] (the witness — `db_dir` is the
/// lock's own dir) for the WHOLE operation and has CLOSED the prior `wallet.db`
/// connections first, so the rename replaces the file cleanly. **Synchronous +
/// blocking** — the handle calls this inside `spawn_blocking`.
pub(crate) fn reset_data_db_keep_seed(
    lock: &WalletLock,
    db_key: &WalletDbKey,
    network: Network,
) -> Result<(), WalletError> {
    reset_data_db_keep_seed_with(lock, db_key, network, &mut |_| Ok(()))
}

/// The rescan rebuild with a per-step hook. Production passes a no-op; the §8
/// fault-injection test passes a hook that returns `Err` after a chosen
/// [`RescanStep`] to simulate a process kill there.
fn reset_data_db_keep_seed_with(
    lock: &WalletLock,
    db_key: &WalletDbKey,
    network: Network,
    after_step: &mut dyn FnMut(RescanStep) -> Result<(), WalletError>,
) -> Result<(), WalletError> {
    let db_dir = lock.dir();
    let final_path = db_dir.join(WALLET_DB_FILE_NAME);
    let tmp_path = db_dir.join(RESCAN_TMP_DB_FILE_NAME);

    // A stale temp DB (+ SQLite sidecars) from an interrupted prior rescan is inert
    // (no marker, never opened) but must not pollute the rebuild — clear it first
    // (the shared db-file sweep, also used by the block-cache reset). The handle's
    // rescan path already ran this BEFORE closing its connections
    // ([`sweep_stale_rescan_tmp`], W-swap-4-a-5 — on a tight disk the close's WAL
    // fold is the first write that needs the freed headroom); re-run here so the
    // rebuild stays self-contained for every other caller.
    sweep_stale_rescan_tmp(lock)?;

    // Step 1 — a fresh, ACCOUNT-LESS data DB at the temp path under the SAME key
    // (idempotent zcash schema + our aux schema, via db::migrate). The old wallet.db
    // is untouched and still the live wallet. `created_at = None`: the REAL creation
    // stamp (if any) rides the step-2 aux copy — stamping "now" here would forge a
    // later creation time onto a rescanning wallet.
    db::provision_db(&tmp_path, db_key, network, None)?;
    after_step(RescanStep::TempProvisioned)?;

    // Step 2 — copy the durable aux tables (money intents / swap detection) from the
    // OLD wallet.db INTO the temp DB. Reads the source only — never writes it.
    db::copy_aux_tables(&tmp_path, &final_path, db_key)?;
    // The last-synced stamp rode the copy (its TABLE is ADR-0534-preserved) but its
    // ROW must NOT survive a rescan: balances rebuild from the birthday, and a stale
    // stamp would pair the old "as of block H, time" with a rebuilding balance.
    // Cleared HERE — on the temp DB, before the rename — so the reset is atomic with
    // the rebuild (a kill leaves intact-old-with-stamp or complete-new-without, never
    // "new DB, old stamp").
    {
        let tmp_conn = db::open_keyed_connection(&tmp_path, db_key)?;
        crate::sync_stamp::clear(&tmp_conn)?;
        // #357: the durable everSynced flag's ROW must not survive either — a rescan
        // resets the wallet to catching-up (balances rebuild from the birthday), so
        // the catch-up cue must re-show until the first post-rescan reached-tip. Its
        // TABLE rode the copy (ADR-0534 list == ensure set); clear the ROW here, atomic
        // with the rebuild, the same discipline as `sync_stamp` (see `ever_synced` doc).
        crate::ever_synced::clear(&tmp_conn)?;
        // #377 s357b-2: and SET the rescan-rebuilding breadcrumb in the same
        // atomic window — the rename that makes this DB live is exactly the
        // moment "this wallet's emptiness is a rescan rebuild" becomes true.
        // Cleared at the first post-rescan reached-tip (`rescan_rebuilding`
        // module doc); a kill before the rename leaves the intact old DB with
        // no breadcrumb.
        crate::rescan_rebuilding::set(&tmp_conn)?;
        // The engine-registration marker's row must not survive either (#368): step 1
        // provisioned a FRESH engine DB with none of the old `get_address_for_index`
        // registrations, so a copied marker would falsely assert the allocated indices
        // registered — the next transparent refresh must re-register the full range
        // (refund AND destination addresses; `refund_index::REGISTRATION_TABLE` doc).
        crate::refund_index::clear_registration(&tmp_conn)?;
        // ... and RE-SEED it right here (#382), from the counter step 2 just copied,
        // atomically with the rebuild (both land or neither — the rename is the commit
        // point): a post-rescan DB therefore NEVER carries an unseeded bounds row that
        // the first post-rescan quote's reserve→persist gap could race (the
        // seed-race MED — quote issuance is fenced across the rescan, so THIS is the
        // quiescent point; marker 0 + ceiling = the copied counter ⇒ the next refresh
        // re-registers the full allocated range).
        crate::refund_index::seed_from_counter(&tmp_conn)?;
        // T0-1a-R: and the A11 rewind ledger's rows go too, in the same atomic
        // window. The rebuilt DB has no shard rows and no scanned blocks, so the
        // height bind has nothing stale to relax — a surviving "a rewind happened"
        // would spend itself on the first post-rescan pass and buy the endpoint an
        // unbound one for free (`root_bind`'s rewind-gate section; the `sync_stamp`
        // clear-on-rescan posture, not `creation_stamp`'s preserve).
        crate::root_bind::clear_rewind_watch(&tmp_conn)?;
        // BIND-1-R: the boundary-bound ledger's rows go with it, in the same window
        // and for the same reason twice over. A bracket refutes a RECORDED completion
        // height, and the rebuilt DB has none; it was counted from `blocks` rows the
        // rebuild also drops, so a surviving bracket would refuse an honest server a
        // height this wallet could no longer prove wrong
        // (`root_bind::clear_boundary_bounds`).
        crate::root_bind::clear_boundary_bounds(&tmp_conn)?;
        // Fold the temp WAL with HONEST faults before this last close, whose own
        // checkpoint is silent (W-swap-4-a-5): a near-full disk surfaces `DiskFull`
        // here, at the point of failure, instead of the backstop's `StoreCorrupt`
        // below. This is the temp DB's LAST-standing connection (step 1's and
        // step 2's own handles closed already), so TRUNCATE meets no reader and
        // folds every prior connection's commits.
        db::checkpoint_wal_truncate(&tmp_conn)?;
    }
    after_step(RescanStep::AuxCopied)?;

    // Step 3 — fsync(temp) → atomic rename over wallet.db (the write_atomic discipline).
    //
    // WAL-FOLD GUARDS (W-swap-4-a-4 review fold — the 3-review converged HIGH):
    // the DBs are WAL now, so a COMMIT lands in `<db>-wal` and reaches the main file
    // only via a checkpoint whose LAST-CLOSE form fails SILENTLY (SQLite leaves the
    // WAL behind and close still succeeds; rusqlite's Drop discards the close result).
    // The rename below moves ONLY the main file — an unfolded temp WAL would install a
    // wallet.db MISSING the just-copied aux money tables, and an unfolded OLD WAL
    // means the old wallet's most recent commits live beside (not inside) the file we
    // are about to replace. Both sides fail CLOSED here, with the old wallet.db (and
    // its recoverable WAL) untouched: retrying the rescan re-runs the whole idempotent
    // rebuild.
    db::verify_wal_folded(&tmp_path)?;
    db::verify_wal_folded(&final_path)?;
    fsync_file(&tmp_path)?;
    std::fs::rename(&tmp_path, &final_path).map_err(WalletError::from_io)?;
    after_step(RescanStep::Renamed)?;

    // Make the rename durable across a power cut (the dir fsync), like write_atomic.
    sync_dir(db_dir)
}

/// Sweep a stale rescan temp DB (+ its SQLite sidecars) left by an interrupted
/// prior rescan (W-swap-4-a-5). Called by the handle's rescan path BEFORE it
/// closes the prior connections — the close's WAL fold
/// ([`db::checkpoint_wal_truncate`]) is the first write that needs disk
/// headroom, and a stale temp from a rescan that died mid-copy can be exactly
/// the space hog; sweeping first makes a tight-disk rescan converge in the
/// SAME cycle instead of surfacing an honest-but-stuck `DiskFull`. Idempotent
/// and inert to sweep (no marker, never opened by anyone else);
/// [`reset_data_db_keep_seed`] re-runs it so the rebuild stays self-contained,
/// and it centralizes the tmp-path assembly. Takes the [`WalletLock`] WITNESS,
/// not a bare path: this is the one crate-visible
/// destructive store helper, and the file's witness invariant must hold by
/// type, not by caller discipline.
pub(crate) fn sweep_stale_rescan_tmp(lock: &WalletLock) -> Result<(), WalletError> {
    db::remove_db_and_sidecars(&lock.dir().join(RESCAN_TMP_DB_FILE_NAME))
}

/// fsync a single file by path (open read-only, `sync_all`). Used to make the rescan
/// temp DB durable BEFORE the atomic rename (the write_atomic file-fsync, for a DB
/// file SQLite already committed — belt-and-suspenders + ENOSPC → `DiskFull`).
fn fsync_file(path: &Path) -> Result<(), WalletError> {
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(WalletError::from_io)
}

// ── Provisioning (fresh) ─────────────────────────────────────────────────────

// WITNESS INVARIANT: the `provision`/`provision_with`/`repair` helpers take a
// bare `db_dir: &Path`, but are reachable ONLY from `create_or_repair`, which
// derived `db_dir` from `lock.dir()` (the held single-writer witness). The
// witness guarantee therefore extends to these helpers by call-chain, not by
// type — keep them module-private so no caller can bypass the witnessed entry.
// (`provision_with` alone is `pub(crate)`, SOLELY so the wallet-level §8
// fault-injection tests — the FR-24 RH1 remnant-retry pin — can build a
// mid-provision remnant; no production caller exists outside this module.)
/// The provisioning sequence with a per-step hook. Production passes a no-op;
/// the §8 fault-injection test passes a hook that returns `Err` after a chosen
/// step to simulate a process kill there. Kept vault-shaped signature; the
/// resolver core is [`provision_resolving_with`]. TEST-SEAM (dead in a
/// non-test build).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn provision_with(
    db_dir: &Path,
    vault: &dyn KeychainPort,
    intent: &ProvisionIntent,
    after_step: &mut dyn FnMut(ProvisionStep) -> Result<(), WalletError>,
) -> Result<(), WalletError> {
    provision_resolving_with(db_dir, &BorrowedVault(vault), intent, after_step)
}

/// [`provision_with`] over the S2 resolver: a fresh wallet is born with a minted
/// [`CustodyId`] — custody under ITS namespace, the artifact framed with the
/// locator at the step-2 write, and the path-keyed index item recording the id
/// with the custody itself.
pub(crate) fn provision_resolving_with(
    db_dir: &Path,
    resolver: &dyn VaultResolver,
    intent: &ProvisionIntent,
    after_step: &mut dyn FnMut(ProvisionStep) -> Result<(), WalletError>,
) -> Result<(), WalletError> {
    std::fs::create_dir_all(db_dir).map_err(WalletError::from_io)?;

    // Step 1 — custody FIRST: ONE SealKey seals both blobs; the vault takes the
    // key. Custody writes only to the VAULT, and it runs BEFORE the destructive
    // prior-life sweep below (W-swap-4-a-6 (a), security MED-1): a
    // locked/unavailable keystore at create — realistic on mobile — must return
    // HERE with a prior life's stray files untouched. (The pre-4-a-6 order swept
    // first, so sweep-ok → custody-FAIL destroyed a prior `wallet.db` with
    // neither custody taken nor step 2 reached — the hole in the 4-a-5
    // zero-marginal-loss proof. The ordering closes that branch; a kill/fault
    // INSIDE the sweep window below still deletes prior DB files without
    // reaching step 2, but there the prior file is dead ciphertext (an SDK
    // life's SealKey was severed first) or the documented ACCEPTED BLAST
    // RADIUS — never a live, custody-recoverable wallet.)
    // A kill AFTER custody (custodied, but nothing on disk yet) leaves the store
    // `Empty` (no manifest) — a retry re-provisions fresh, leaving the gen-N
    // vault alias ORPHANED (no on-disk artifact references it). Bounded: at most
    // one orphan per failed attempt, including a sweep-fault attempt below
    // (subsequent kills past step 2 land in `Remnant` and repair from the
    // existing artifact, never minting a new alias). The orphan holds no
    // plaintext and is unreachable. SEAM NOTE for the wipe chunk: sweep vault
    // aliases that have no corresponding `wrap.artifact` on disk (arch HARDENING).
    //
    // S2: the wallet is born under its OWN identity-derived namespace, not the
    // path's — a relocation later opens the SAME custody. The path-keyed index
    // item is written with the custody (still before any disk write), in the
    // `done` state: a fresh wallet has no legacy namespace and no migration,
    // and a bare-config wipe can already resolve this wallet's namespace even
    // if the process dies before the first open.
    //
    // INDEX FIRST (stage S16 §3.1 item 4): the index names the id BEFORE the
    // key exists under it, so a key that lands before a duress sever's
    // tombstone is always resolvable by that sever, and one that would land
    // after it is refused inside its key-store job. The invariant this
    // restates: the index may name an id whose key is not yet written (a kill
    // between the two writes). That resolves as an orphan — the purge severs
    // 0, the store is `Empty` so the wipe's guard does not trip, `open`
    // answers `NotFound`, and a re-create mints a new id and overwrites the
    // index.
    let custody_id = CustodyId::generate();
    let id_ns = crate::custody::namespace_for(&custody_id);
    let path_ns = crate::wallet::keychain_namespace_for(db_dir);
    let path_index_vault = resolver.vault_for(&path_ns)?;
    path_index_vault
        .as_port()
        .store_index(&CustodyIndexEntry::done(&custody_id))?;
    after_step(ProvisionStep::IndexWritten)?;
    let id_vault = resolver.vault_for(&id_ns)?;
    let vault = id_vault.as_port();
    let db_key = WalletDbKey::generate();
    let sealed = SealedSeedVault::new(vault).store_wallet(intent.seed, &db_key)?;
    after_step(ProvisionStep::Custody)?;

    // Prior-life hygiene AFTER custody, BEFORE anything lands on disk
    // (W-swap-4-a-5; ordered W-swap-4-a-6): an `Empty` dir can still hold DB
    // files from a dead wallet — the partial-wipe warn-and-Ok arm, or a
    // hand-tampered dir. The PROVEN permanent brick (bite-validated in the
    // named test) is a stray keyed `wallet.db` MAIN file: provisioning's forced
    // probe HMAC-fails it under the new key, `StoreCorrupt`, identically on
    // every retry. Orphan sidecars (`-wal`/`-shm`/`-journal`) sweep with it as
    // defense-in-depth: upstream SQLite DELETES a `-wal` beside a ZERO-page
    // main file at first access rather than recovering it
    // (`pagerOpenWalIfPresent` — library-internal, not contract), but beside a
    // NON-empty main the same `-wal` fails the open typed — which is exactly
    // why main + sidecars sweep TOGETHER (the rescan temp and the block cache
    // already did; provisioning was the gap).
    // For anything THIS SDK custodied the sweep is safe by construction: the
    // arm is reachable ONLY when `inspect` read `Empty` (no manifest), custody
    // for THIS life already succeeded above (a failure never reaches here), and
    // a post-wipe survivor is dead ciphertext (its SealKey was severed first).
    // ACCEPTED BLAST RADIUS (security MED, documented not fixed): a
    // `wallet.db` this SDK never wrote — a foreign producer's file in a
    // misconfigured dir (the name is the ecosystem-generic "wallet.db") — is
    // swept too, where the pre-4-a-5 code failed closed around it. `db_dir`
    // is SDK-EXCLUSIVE by host contract (§4.3 / RW-CFG): a create on an
    // `Empty` dir is DESTRUCTIVE to prior DB files in it. Honest IO faults
    // (`DiskFull`/`Io`) propagate.
    db::remove_db_and_sidecars(&db_dir.join(WALLET_DB_FILE_NAME))?;
    // The dead life's stale rescan temp sweeps here too (→#374 review
    // fold, crypto + arch converged): an interrupted rescan + partial wipe can
    // leave a full-size `wallet.db.rescan-tmp` that nothing else touches until
    // the successor's FIRST rescan — inert dead ciphertext, but it pins mobile
    // disk for a wallet that may never rescan. (A dead life's `block-cache.db`
    // is deliberately DEFERRED to its own opener — validated-or-deleted under
    // the new key at first open — so it needs no arm here.)
    db::remove_db_and_sidecars(&db_dir.join(RESCAN_TMP_DB_FILE_NAME))?;
    // A dead life's WIPE STATE must not be inherited (W-swap-4-a-6 (b)
    // money MED): an interrupted wipe's `.wipe-committed` breadcrumb records
    // "THIS namespace's custody is already severed" — true for the DEAD life,
    // false for the successor provisioned here. Inherited, it silently disarms
    // the successor's destroy-time verify-real-sever guard for its whole life
    // (`severed == 0` would converge as "expected" instead of failing closed —
    // the B1 hole reopened). Stale `*.tmp` remnants (a kill inside
    // `write_atomic`'s create→rename window) sweep too: same-ciphertext blobs
    // of a dead life with no live reader — the §3.3 wipe already sweeps them
    // via `read_dir`; a fresh provision inherits the same hygiene. (`.tmp` is
    // matched on raw bytes: the SDK's own tmp names are always UTF-8, but the
    // destroy sweep is name-agnostic and this one shouldn't be weaker.)
    db::remove_path_if_present(&db_dir.join(WIPE_COMMITTED_FILE_NAME))?;
    restore_sever_breadcrumb(db_dir)?;
    for entry in std::fs::read_dir(db_dir).map_err(WalletError::from_io)? {
        let entry = entry.map_err(WalletError::from_io)?;
        if entry.file_name().as_encoded_bytes().ends_with(b".tmp") {
            db::remove_path_if_present(&entry.path())?;
        }
    }
    // A dead life's FINAL-NAME seal artifacts sweep too (W-swap-4-a-6 (b),
    // fold — #378 security MED): a SealedKeychain provision killed inside
    // step 2 (`seed.seal` durable, manifest not) reads back `Empty`. A
    // SealedKeychain successor overwrites all three below, but a
    // None-persistence successor never writes `seed.seal` — unswept, the dead
    // life's SEALED SEED would sit at rest for the successor's whole life
    // under a manifest that promises "no seed at rest". (On the
    // alias-generation keystore shape the dead wrap alias ALSO survives as
    // the bounded step-1 orphan; the successor's unconditional
    // `wrap.artifact` overwrite below breaks the READY-MADE unwrap chain, so
    // the at-rest residual is sealed ciphertext plus whatever was captured
    // pre-successor — review precision. On the fixed-item shape the
    // successor's custody overwrites the dead item outright. The
    // manifest-promise violation is identical on both, which is what this
    // sweep closes. The orphan wrap key itself is reached by NO wipe under
    // S2: each attempt custodies under its own freshly minted id namespace,
    // and once that attempt's index write is replaced (or never happened —
    // a custody call that timed out, FR-47) no header and no index names it,
    // so destroy's purge never resolves it. ADR-0563 states the residual; the
    // step-1 SEAM NOTE's alias sweep is its tracked closure.)
    // Safe by construction: this arm is `Empty`-gated, and
    // seals-without-manifest means the seals were never SDK-loadable —
    // a mid-step-2 kill (step 2 writes seals BEFORE the manifest) or a
    // mid-destroy kill (custody severed FIRST — dead ciphertext) —
    // `read_manifest` fails before any seal reader runs.
    // `dbkey.seal`/`wrap.artifact` sweep harmlessly (step 2 rewrites both
    // unconditionally).
    for name in [
        SEED_SEAL_FILE_NAME,
        DBKEY_SEAL_FILE_NAME,
        WRAP_ARTIFACT_FILE_NAME,
    ] {
        db::remove_path_if_present(&db_dir.join(name))?;
    }
    after_step(ProvisionStep::PriorLifeSwept)?;

    // Step 2 — remnant: seal artifacts FIRST, manifest LAST (so a manifest on
    // disk always implies the seals are durable → recoverable).
    // "A None manifest never sits beside a live `seed.seal`" (the #378 sweep's
    // NO-UPGRADE-ARM rationale) is structural only if the seed follows the
    // persistence — enforced at the production intent-construction site
    // (wallet.rs) and pinned here (review NIT).
    debug_assert!(
        intent.persistence == PersistenceKind::SealedKeychain || intent.seed.is_none(),
        "a None-persistence intent must not carry a seed to seal"
    );
    if let Some(seed_blob) = &sealed.seed_blob {
        write_atomic(db_dir, SEED_SEAL_FILE_NAME, seed_blob)?;
    }
    write_atomic(db_dir, DBKEY_SEAL_FILE_NAME, &sealed.dbkey_blob)?;
    // The artifact is written FRAMED with the custody locator — the one file a
    // later open reads before any key (the backend bytes inside are untouched).
    write_atomic(
        db_dir,
        WRAP_ARTIFACT_FILE_NAME,
        &sealed
            .wrap_artifact
            .with_custody_id(&custody_id)
            .file_bytes(),
    )?;
    write_atomic(
        db_dir,
        MANIFEST_FILE_NAME,
        &encode_manifest(
            intent.network,
            intent.persistence,
            // #357: record the seed provenance so a later `repair` can honour it.
            ManifestSeedSource::from_intent(intent),
        ),
    )?;
    after_step(ProvisionStep::RemnantPersisted)?;

    // Step 3 — DB: create the encrypted DB + sentinel + zcash schema (idempotent),
    // stamping the creation time for a freshly-generated create (Generate /
    // host-attested FreshRawBytes — §3.2f #356-F4).
    db::provision_db(
        &db_dir.join(WALLET_DB_FILE_NAME),
        &db_key,
        intent.network,
        intent.freshly_generated_at,
    )?;
    after_step(ProvisionStep::DbProvisioned)?;

    // Step 4 — marker LAST (the atomic Complete transition).
    write_marker(db_dir)?;
    after_step(ProvisionStep::Completed)?;
    Ok(())
}

// ── Repair (resume from a remnant) ───────────────────────────────────────────

/// Remnant repair over the S2 resolver: custody is recovered from whatever
/// namespace the remnant's artifact header names (a pre-S2 remnant carries no
/// locator and still opens under the legacy path namespace — and its migration
/// runs here with it, before the remnant completes; every step is idempotent
/// and resumable, exactly as at open).
fn repair_resolving(
    db_dir: &Path,
    resolver: &dyn VaultResolver,
    intent: &ProvisionIntent,
) -> Result<(), WalletError> {
    let manifest = read_manifest(db_dir)?;
    // A create/restore against a remnant with a different network or persistence
    // is a confused host — refuse rather than half-rewrite the remnant. The
    // persistence-mode mismatch reuses `ProvisioningIncomplete` deliberately: the
    // host can never legitimately change SealedKeychain↔None on an existing
    // wallet (it is fixed at create time, not user-selectable), so this is an
    // unreachable-in-practice arm; surfacing "this remnant isn't yours to finish
    // under these params" as ProvisioningIncomplete keeps the host on the
    // resume-or-open path rather than inventing a variant for a can't-happen case
    // (review NIT — documented choice; revisit in a future taxonomy pass).
    if manifest.network != intent.network {
        return Err(WalletError::NetworkMismatch);
    }
    if manifest.persistence != intent.persistence {
        return Err(WalletError::ProvisioningIncomplete);
    }

    // Recover custody FROM the remnant (never a new key/seed).
    let loaded = load_custody_resolving(db_dir, resolver, manifest.persistence)?;
    let (sealed_seed, db_key, _status) = (loaded.seed, loaded.db_key, loaded.status);

    // Verify the supplied seed where the mode demands it (§6.3).
    match intent.repair_mode {
        RepairMode::VerifySupplied => {
            let supplied = intent.seed.ok_or(WalletError::SeedMismatch)?;
            let sealed = sealed_seed.as_ref().ok_or(WalletError::StoreCorrupt)?;
            // Constant-time: a mistyped-mnemonic re-run must not be a timing oracle.
            if !bool::from(supplied.seed().ct_eq(sealed.seed())) {
                return Err(WalletError::SeedMismatch);
            }
        }
        // Generate resumes from the sealed seed (never regenerates); None has none.
        RepairMode::ResumeSealed | RepairMode::NoSeed => {}
    }

    // Resume the remaining steps idempotently with the RECOVERED db_key.
    // SEAM NOTE (security HARDENING, inc-2b-ii-B): the create-vs-open sentinel guards
    // `open`, not `repair` — if an attacker truncated a remnant's `wallet.db` to 0
    // bytes between provisioning attempts, `provision_db` re-creates the sentinel +
    // re-runs `init_wallet_db` into the blanked DB and repair completes to `Complete`.
    // This is NOT fund loss (the chain is the source of truth; the seed was
    // constant-time-verified against the sealed remnant above; the migrator rebuilds
    // the schema idempotently). If the chain ever stops being authoritative, repair
    // must distinguish "DB present-but-blank" from "DB absent" before re-provisioning.
    // STAMP ON REPAIR ONLY FOR A `Generated` REMNANT (#357; the review HIGH
    // reconciled). The historical posture was NEVER-stamp, because a v1 manifest
    // recorded no seed-source kind and `ResumeSealed` verifies nothing — so the
    // remnant's sealed seed might originate from an aborted RESTORE, and a stamp
    // could durably attach a creation time to a seed with prior history, survive
    // reopen AND ride the rescan's preserved aux tables, permanently defeating the
    // `rescan_from(None)` full-history recovery hatch. #357's v2 manifest records
    // the provenance (`ManifestSeedSource`), so the stamp is now EVIDENCE-based:
    // a `Generated` remnant is provably fresh on this device ⇒ a fresh-shaped
    // repair MAY stamp it (the match below); a `Restored`/`Unknown` remnant is
    // still NEVER stamped. Pre-#357 a fresh create
    // killed AFTER DbProvisioned already had its stamp durable while one killed
    // EARLIER degraded to the stamp-less belt arms (tip−lag in-session via
    // `freshly_generated`; activation on a not-yet-synced reopen — a session that
    // already synced imported at tip−lag and only `rescan_from(None)` lowers it,
    // the pre-F4 class in every arm) — safe, and #357 now RECOVERS that killed-
    // early `Generated` case to a real stamp instead of the belt arms. #387's
    // stamp-less consequence (that wallet's first open also took the
    // restore-pessimistic seed — one bounded breadth sweep + a widened-poll
    // settlement window, spec §3.2h item 5 residual (c)) likewise only applies to
    // the remaining stamp-less arms (`Restored`/`Unknown`).
    db::provision_db(
        &db_dir.join(WALLET_DB_FILE_NAME),
        &db_key,
        manifest.network,
        None,
    )?;
    // #357: with the v2 manifest recording seed PROVENANCE, `repair` honours the
    // remnant's origin instead of blanket-refusing to stamp.
    match (intent.freshly_generated_at, manifest.seed_source) {
        // RESTORE-shaped retry — the DOWNGRADE UN-STAMP (crypto audit MED-1):
        // completing a remnant under a restore intent clears any stamp an aborted
        // FRESH attempt left durable. Without it, the documented recovery for a
        // doubted attestation — retry the create with the plain restore shape —
        // silently inherited the aborted attempt's ~creation scan floor and
        // skipped the #387 restore-pessimistic refund sweep (probe-proven:
        // first-sync birthday at ~stamp instead of the activation floor the
        // restore shape promises). Conservative wider-scan direction, never a
        // higher birthday. See `creation_stamp::clear` for the crash-atomicity
        // argument.
        (None, _) => {
            let conn = db::open_keyed_connection(&db_dir.join(WALLET_DB_FILE_NAME), &db_key)?;
            crate::creation_stamp::ensure_table(&conn)?;
            crate::creation_stamp::clear(&conn)?;
        }
        // GENERATE-over-GENERATE (#357): a `Generated` manifest byte from a
        // fresh-shaped retry lets `repair` stamp — even a remnant killed BEFORE its
        // original stamp landed (which pre-#357 degraded to the stamp-less belt
        // arms). MONEY-SAFETY — the REAL basis (post-ship security MED corrects an
        // earlier crypto audit misstatement). A creation stamp is NOT "only ever
        // downward-widening": `resolve_birthday` keys its arm on `created_at.is_some()`
        // (NOT on `freshly_generated`), and the SAME `created_at.is_some()` gates both
        // the #387 refund sweep and the ADR-0537 diversified sweep — so a stamp on a
        // seed with PRIOR HISTORY floors the birthday at ~creation AND skips both
        // sweeps, HIDING pre-stamp funds (and it re-floors `rescan_from(None)`; only a
        // #390 deep scan recovers). The safety therefore rests on WHO can land a stamp
        // here: this arm fires ONLY on the USER's own FRESH attestation
        // (`freshly_generated_at.is_some()` on the retry). A fresh-attested RESTORE is
        // exactly the §3.2f FreshRawBytes mis-attestation whose fund-hiding is already
        // the documented lying-host consequence on a NORMAL (non-crash) fresh create —
        // #357 does not create a new class of exposure, it only trades the BELT
        // (which uniquely caught the crash-then-fresh-retry sub-case by never stamping
        // a repair) for the killed-early-GENUINE-fresh birthday recovery. The manifest
        // is PLAINTEXT/unauthenticated (a no-DB-key attacker can flip `Restored`→
        // `Generated`), but that grants NO new capability: it only bites if the user
        // ALSO mis-attests fresh on the retry — retrying as a plain RESTORE hits the
        // `(None, _)` downgrade-unstamp below and stays safe. `record_once`'s ON
        // CONFLICT keeps the EARLIEST stamp; the clock-ahead cap is applied inside it.
        (Some(at), ManifestSeedSource::Generated) => {
            let conn = db::open_keyed_connection(&db_dir.join(WALLET_DB_FILE_NAME), &db_key)?;
            crate::creation_stamp::ensure_table(&conn)?;
            crate::creation_stamp::record_once(&conn, manifest.network, at)?;
        }
        // Fresh-shaped retry over a RESTORED or UNKNOWN (pre-#357 v1) remnant:
        // still NEVER stamp — the sealed seed may originate
        // from an aborted restore, or an origin a v1 manifest cannot vouch for, so
        // a creation time here could durably attach to a seed with prior history.
        (Some(_), ManifestSeedSource::Restored | ManifestSeedSource::Unknown) => {}
        // #397: UNREACHABLE in practice — a fresh-shaped (seeded) intent over a
        // watch-only remnant already rejected at the persistence-mismatch guard
        // above (seeded intents are SealedKeychain/None). Exhaustive + no-stamp
        // (a watch-only store has no seed for a stamp to describe).
        (Some(_), ManifestSeedSource::WatchOnly) => {}
    }
    write_marker(db_dir)?;
    Ok(())
}

// ── Destroy (FR-14 crypto-shred — the inverse of provision) ──────────────────

/// FR-14 crypto-shred: sever the keychain wrap key FIRST (the authoritative
/// shred — the §4.2a SealKey is deleted, so this key store can no longer open any
/// on-disk seal or the `wallet.db` ciphertext; ADR-0571 for how strongly), THEN
/// remove the on-disk artifacts best-effort.
/// The `lock` witness binds `db_dir` + proves single-writer exclusivity (the same
/// WITNESS INVARIANT discipline as [`create_or_repair`]/[`open`]).
///
/// `vault = None` ⇒ this platform has NO keystore (headless desktop): permitted
/// ONLY when the store is `Empty` (nothing was ever custodied — a file-only
/// no-op); a provisioned store with no reachable vault fails LOUD (`VaultAbsent`),
/// never a silent file-only "shred" that leaves the real wrap key live.
///
/// `force = true` is the host's escape hatch (see `Wallet::wipe_force`): it skips
/// the verify-real-sever guard so a wallet whose custody item is ALREADY GONE (a
/// manual keychain deletion, a pre-FR-13 bare-name item, an iOS `ThisDeviceOnly`
/// item that didn't migrate) can still be deleted. Use only with a cfg the host is
/// certain matches the wallet's (a wrong cfg under `force` would delete the wrong
/// files while leaving the real item — the very B1 hole the guard plugs).
///
/// Idempotent: a fully-wiped `db_dir` reads `Empty`, severs 0, removes nothing.
/// An INTERRUPTED wipe auto-converges via the `.wipe-committed` breadcrumb (below).
///
/// Kept vault-shaped signature; production (and the bare-config convergence,
/// which must resolve the custody namespace through the header or the index)
/// goes through [`destroy_resolving`]. TEST-SEAM (dead in a non-test build).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn destroy(
    lock: &WalletLock,
    vault: Option<&dyn KeychainPort>,
    force: bool,
) -> Result<(), WalletError> {
    destroy_with(lock, vault, force, &mut |_| {})
}

/// `destroy` with a per-unlink observation hook (the `reset_data_db_keep_seed_with`
/// pattern): called with each entry name BEFORE its removal, and with the lock file
/// name LAST — so a test can pin the lock-last sweep order (#317 review F2: the
/// ORDER is the fix; without this seam a revert to the racy whole-dir sweep passed
/// every test). Kept vault-shaped signature. TEST-SEAM (dead in a non-test build).
#[cfg_attr(not(test), allow(dead_code))]
fn destroy_with(
    lock: &WalletLock,
    vault: Option<&dyn KeychainPort>,
    force: bool,
    on_unlink: &mut dyn FnMut(&std::ffi::OsStr),
) -> Result<(), WalletError> {
    match vault {
        Some(v) => destroy_with_resolving(lock, Some(&BorrowedVault(v)), force, on_unlink),
        None => destroy_with_resolving(lock, None, force, on_unlink),
    }
    .map(|_| ())
}

/// What a completed destroy severed. `severed` is the count the purges
/// returned — the only PROOF a sever happened in this call; `converged` says
/// why a zero count still ended in `Ok` (stage S16 §3.1 item 2: an `Ok(())`
/// alone cannot tell a genuine sever from the idempotent no-op).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct DestroyOutcome {
    pub(crate) severed: usize,
    pub(crate) converged: Converged,
}

/// Why a destroy converged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Converged {
    /// This call severed a non-zero count.
    Severed,
    /// Nothing on disk named a key to sever: the store was `Empty`, or no wrap
    /// artifact was present (or the host forced a zero count past the guard).
    Empty,
    /// An earlier, interrupted wipe or a proven duress sever left the
    /// breadcrumb: the custody was already gone.
    Breadcrumb,
    /// An earlier wipe's purge of one of these namespaces landed late in this
    /// process with a non-zero count (FR-47).
    SeveredLate,
}

/// S2 `destroy` (both the Dart static and any future caller): resolve the
/// custody identifier from the path-keyed INDEX item (a keychain item) and the
/// wrap artifact's HEADER (a file) — the header counts only when the index
/// agrees with it or it unseals this directory's seals, else the index decides
/// — so the wipe converges from a bare config with the host having already
/// deleted the files, and never severs a namespace a tampered header names.
/// Purge set: the id
/// namespace always; the legacy namespace too while the index says `pending`
/// (a half-finished migration left a live wrap key there); the PATH namespace
/// when no id is resolvable at all (the pre-stage shape — everything lives
/// there, exactly the pre-S2 wipe). The index item is deleted LAST: it is the
/// breadcrumb when the directory is gone. A second call is the no-op success.
///
/// Stage S16 split this at its seams without changing its behaviour: (a)
/// [`resolve_custody`], (b) [`purge_resolved`], (c) [`guard_real_sever`], (d)
/// the breadcrumb, (e) [`sweep_files`] and (f) [`forget_index`]. `destroy`
/// runs (a)→(f) under the lock; the duress sever's keychain-only path runs
/// every piece but (e), under a [`CustodyOnlyDir`] witness. A completed
/// destroy clears any duress tombstone on this path.
pub(crate) fn destroy_resolving(
    lock: &WalletLock,
    resolver: Option<&dyn VaultResolver>,
    force: bool,
) -> Result<DestroyOutcome, WalletError> {
    destroy_with_resolving(lock, resolver, force, &mut |_| {})
}

/// [`destroy_resolving`] with the per-unlink observation hook.
fn destroy_with_resolving(
    lock: &WalletLock,
    resolver: Option<&dyn VaultResolver>,
    force: bool,
    on_unlink: &mut dyn FnMut(&std::ffi::OsStr),
) -> Result<DestroyOutcome, WalletError> {
    let resolved = resolve_custody(lock.dir(), resolver, force)?;
    destroy_resolved(lock, resolver, &resolved, force, on_unlink)
}

/// What (a) resolved: the store's state, the proof anchors, and the custody
/// namespaces to sever. Carries no key; the identifier inside never reaches a
/// log (§5.4).
struct Resolved<'a> {
    state: StoreState,
    artifact_present: bool,
    wipe_already_committed: bool,
    /// The wrap artifact header's locator, proven or not (the blind path's
    /// namespace when the keychain is locked, §3.1 item 7).
    header_id: Option<CustodyId>,
    path_ns: crate::keychain::KeychainNamespace,
    path_vault: Option<crate::keychain::ResolvedVault<'a>>,
    custody_id: Option<CustodyId>,
    pending_legacy_ns: Option<crate::keychain::KeychainNamespace>,
    /// The index read answered the locked class (`KeystoreUnavailable`) and
    /// was read as absent. `destroy` ignores it (its posture is unchanged);
    /// the duress sever takes the blind path on it.
    index_locked: bool,
}

/// (a) Resolve the custody namespaces BEFORE anything is severed — ONE
/// function, because the header/index/proof decision is one rule.
fn resolve_custody<'a>(
    db_dir: &Path,
    resolver: Option<&'a dyn VaultResolver>,
    force: bool,
) -> Result<Resolved<'a>, WalletError> {
    // A corrupt store (completion marker present, manifest absent — manual
    // corruption / fs fault) STILL has custody to shred + files to remove, so treat
    // it as non-Empty rather than bail: `open`/`create` already refuse it as
    // `StoreCorrupt`, so a `?` here would leave a corrupt-but-real wallet the ONE
    // thing it can never be — wiped. Any OTHER inspect error (an io fault stat-ing
    // the dir) genuinely propagates.
    let state = match inspect(db_dir) {
        Ok(s) => s,
        Err(WalletError::StoreCorrupt) => StoreState::Complete,
        Err(e) => return Err(e),
    };
    // The on-disk artifact is the proof a real keychain item SHOULD exist — the
    // verify-real-sever anchor below. Read BEFORE any deletion.
    let artifact_present = file_exists(&db_dir.join(WRAP_ARTIFACT_FILE_NAME))?;
    // A breadcrumb from a PRIOR interrupted wipe: a genuine sever (severed > 0)
    // wrote it before the file deletes, so THIS namespace's custody is ALREADY
    // gone ⇒ a now-empty purge is EXPECTED, not a mismatch (so the guard below is
    // skipped and the interrupted wipe converges on this re-run).
    let wipe_already_committed = file_exists(&db_dir.join(WIPE_COMMITTED_FILE_NAME))?;

    // ── S2: resolve the custody namespaces BEFORE anything is severed. ──
    // The header first (the directory exists); the path-keyed index item second
    // — the one thing left when the host already deleted the files. Both decodes
    // are FAIL-OPEN to "absent" (a corrupt artifact/index must not make a wallet
    // unwipeable; the verify-real-sever guard below still fails closed when
    // nothing severs). Payload-free outcome codes only — the identifier NEVER
    // reaches a log (§5.4).
    let header_id = if artifact_present {
        match read_seal_file(db_dir, WRAP_ARTIFACT_FILE_NAME).and_then(WrapArtifact::from_bytes) {
            Ok(artifact) => artifact.custody_id(),
            Err(e) => {
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = e.code(),
                    "wallet.wipe_custody_header_unreadable",
                );
                None
            }
        }
    } else {
        None
    };
    let path_ns = crate::wallet::keychain_namespace_for(db_dir);
    let path_vault = match resolver.map(|r| r.vault_for(&path_ns)) {
        Some(res) => Some(res?),
        None => None,
    };
    // An index that cannot be READ is not an index that is absent. With the
    // directory present the header can still name the namespace, so the read
    // is fail-open there; with the directory gone (the host deleted the files
    // first — the bare-config wipe) the index is the ONLY thing that can, and a
    // fail-open read would end in `Ok` with the wallet's wrap key still live
    // (the verify-real-sever guard keys on files that are, by construction,
    // absent here). So a bare-config wipe that cannot read the index fails
    // typed — nothing has been severed, a retry is safe — unless the host
    // forces it (a corrupt, present index would otherwise make the wallet
    // unwipeable from a bare config). An index that reads as ABSENT stays a
    // no-op success: the second call of a converged wipe, a wallet that never
    // existed at this path.
    //
    // FR-47: a read that TIMED OUT is neither — the key store never answered —
    // so it fails the wipe typed whether or not the directory is there and
    // whether or not `force` is set. Read as absent, it would purge the PATH
    // namespace (not this wallet's) and, under `force` with the directory
    // gone, delete the index: the only record of the wallet's namespace.
    let mut index_locked = false;
    let index = match &path_vault {
        Some(v) => match v.as_port().load_index() {
            Ok(entry) => entry,
            Err(e @ WalletError::KeychainTimeout { .. }) => {
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = e.code(),
                    "wallet.wipe_custody_index_unreadable",
                );
                return Err(e);
            }
            Err(e) if !artifact_present && !force => {
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = e.code(),
                    "wallet.wipe_custody_index_unreadable",
                );
                return Err(e);
            }
            Err(e) => {
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = e.code(),
                    "wallet.wipe_custody_index_unreadable",
                );
                index_locked = matches!(e, WalletError::KeystoreUnavailable);
                None
            }
        },
        None => None,
    };
    // Which identifier the wipe severs. The index is a KEYCHAIN item; the
    // header is a file in the sandbox that anything with write access there
    // can edit. A header the index corroborates is taken as it is; one it does
    // not (no index — a wallet never opened at this path since it moved — or a
    // different identifier) must PROVE itself the way an open does, by
    // unsealing THIS directory's seals under its namespace. An unproven header
    // never names what is severed — §3.1 row 7, a wipe reaches nothing outside
    // its wallet — and the index decides.
    let index_id = index.as_ref().map(|ix| ix.id);
    let custody_id = match (header_id, resolver) {
        (Some(h), _) if index_id == Some(h) => Some(h),
        (Some(h), r) => {
            // A proof that TIMED OUT is no verdict (FR-47): it fails the wipe
            // typed rather than letting the index decide.
            let proven = match r {
                Some(r) => header_unseals(db_dir, r, &h)?,
                None => false,
            };
            if proven {
                Some(h)
            } else {
                tracing::warn!(
                    target: "zec_wallet_core",
                    "wallet.wipe_custody_header_unproven",
                );
                index_id
            }
        }
        (None, _) => index_id,
    };
    // A pending record's legacy namespace is purged only when it is THIS
    // path's own — the only value a migration ever records (the same equality
    // the migration's resume check demands). Anything else is not this
    // wallet's to sever.
    let pending_legacy_ns = match &index {
        Some(ix) if ix.is_pending() && ix.legacy_ns.as_ref() == Some(&path_ns) => {
            ix.legacy_ns.clone()
        }
        _ => None,
    };
    Ok(Resolved {
        state,
        artifact_present,
        wipe_already_committed,
        header_id,
        path_ns,
        path_vault,
        custody_id,
        pending_legacy_ns,
        index_locked,
    })
}

/// The id and legacy namespaces (a) resolved — what a duress sever tombstones
/// before it submits their purge.
fn resolved_namespaces(r: &Resolved<'_>) -> Vec<crate::keychain::KeychainNamespace> {
    r.custody_id
        .as_ref()
        .map(crate::custody::namespace_for)
        .into_iter()
        .chain(r.pending_legacy_ns.clone())
        .collect()
}

/// (b) Keychain FIRST: the authoritative crypto-shred. Returns the count
/// severed and every namespace a purge was asked of.
fn purge_resolved(
    resolver: Option<&dyn VaultResolver>,
    r: &Resolved<'_>,
) -> Result<(usize, Vec<crate::keychain::KeychainNamespace>), WalletError> {
    let (state, custody_id, pending_legacy_ns, path_ns, path_vault) = (
        r.state,
        &r.custody_id,
        &r.pending_legacy_ns,
        &r.path_ns,
        &r.path_vault,
    );
    // Each purge runs NAMED (`bounded::purging`): a bounded purge that times
    // out here and lands later records the namespace it was asked to sever —
    // the evidence the guard below accepts on the re-run (FR-47).
    let mut severed = 0usize;
    let mut purged: Vec<crate::keychain::KeychainNamespace> = Vec::new();
    let mut purge = |vault: &dyn KeychainPort,
                     ns: &crate::keychain::KeychainNamespace|
     -> Result<usize, WalletError> {
        purged.push(ns.clone());
        bounded::purging(ns, || SealedSeedVault::new(vault).purge())
    };
    match resolver {
        None => {
            // No platform keystore. Custody requires the vault (probe-first), so an
            // Empty store has nothing to shred ⇒ file-only no-op. A non-Empty store
            // here is a provisioned wallet we CANNOT crypto-shred (the vault that
            // sealed it is unreachable) ⇒ fail loud, preserve files (the host retries
            // with the vault reachable — no silent partial shred, invariant 10).
            if state != StoreState::Empty {
                return Err(WalletError::VaultAbsent);
            }
        }
        Some(r) => {
            if let Some(id) = &custody_id {
                let id_ns = crate::custody::namespace_for(id);
                severed += purge(r.vault_for(&id_ns)?.as_port(), &id_ns)?;
            }
            if let Some(legacy_ns) = &pending_legacy_ns {
                // A half-finished migration left a LIVE wrap key under the legacy
                // namespace — purge it WITH the id namespace (the purge is wrap
                // material only: the index survives it and is deleted LAST,
                // below, so an interrupted re-run still finds both).
                severed += purge(r.vault_for(legacy_ns)?.as_port(), legacy_ns)?;
            }
            if custody_id.is_none() {
                // No id resolvable anywhere: the pre-stage shape, where ALL of
                // the wallet's custody lives under the path namespace — the
                // pre-S2 purge, verbatim (orphan-alias sweep included; any
                // stray index item under it is deleted LAST, below).
                severed += purge(
                    path_vault
                        .as_ref()
                        .expect("resolver is Some ⇒ the path vault resolved")
                        .as_port(),
                    path_ns,
                )?;
            }
        }
    }
    Ok((severed, purged))
}

/// (c) The verify-real-sever guard. Files are untouched when it trips.
fn guard_real_sever(
    r: &Resolved<'_>,
    severed: usize,
    severed_late: bool,
    force: bool,
) -> Result<(), WalletError> {
    // ── Verify-real-sever (the silent-fail-OPEN guard, crypto review B1). A
    // non-Empty store whose on-disk artifact names a real item but whose purges
    // severed ZERO (in total, across the id + legacy + path namespaces) means
    // the namespaces we derived do not match the custody on disk (a `db_dir`
    // path variant — trailing slash / case-fold — mis-derives the index
    // namespace, or the header's locator names an absent namespace).
    // `remove_dir_all` would still delete the REAL files (the OS normalizes
    // the path) while the REAL wrap key survives + the seals stay
    // flash-recoverable. Fail CLOSED instead — like `open` does on a namespace
    // miss — files UNTOUCHED. TWO legitimate exits do NOT trap:
    //   • `wipe_already_committed` — an interrupted prior wipe already severed this
    //     wallet's custody (the breadcrumb), so `severed == 0` is expected; converge.
    //   • `force` — the host's explicit "this IS my wallet, its item is already
    //     gone" escape hatch ([`Wallet::wipe_force`]).
    //   • `severed_late` (FR-47) — an earlier wipe's purge of one of THESE
    //     namespaces timed out and then landed in this process with a non-zero
    //     count: recorded evidence, exactly like the breadcrumb. Never inferred
    //     from a zero count (a locked iOS keychain severs zero with the key
    //     live); after a restart the evidence is gone and `wipe_force` is the
    //     path, as before.
    if r.state != StoreState::Empty
        && r.artifact_present
        && severed == 0
        && !r.wipe_already_committed
        && !severed_late
        && !force
    {
        // `permanently_invalidated: false` — UNLIKE a real key-invalidation, the data
        // is still ALIVE here (the vault is untouched, the files are preserved); the
        // host retries with the correct (byte-identical) cfg, OR — if it is CERTAIN
        // this is its wallet whose custody item is already gone — via `wipe_force`.
        return Err(WalletError::KeystoreInconsistent {
            permanently_invalidated: false,
        });
    }
    Ok(())
}

/// (d) The breadcrumb: a GENUINE sever (severed > 0) records that this
/// namespace's custody is gone. Durable (`write_atomic` fsyncs) so a power cut
/// keeps it; best-effort — a write fault just means a later re-run needs
/// `wipe_force`. A file, not a DB page, so it is safe beside a live
/// connection (the duress sever writes it there). (S2: it names THIS wallet's
/// namespaces — severed across the id/legacy/path set, all of them.)
fn write_breadcrumb(db_dir: &Path) {
    let _ = write_atomic(db_dir, WIPE_COMMITTED_FILE_NAME, b"1");
}

/// A proven duress sever of this path (stage S16 §3.1 item 6) may have
/// written its breadcrumb between an open's unseal and the open's sweep of
/// stale breadcrumbs; put it back. The sever marks `proven` BEFORE it writes,
/// so in every interleaving either its write lands after the sweep or this
/// read sees the mark. No lock is held across the write. A blind sever never
/// marks, so an unproven key is never covered by a breadcrumb.
fn restore_sever_breadcrumb(db_dir: &Path) -> Result<(), WalletError> {
    if bounded::is_proven(&crate::wallet::keychain_namespace_for(db_dir)) {
        write_atomic(db_dir, WIPE_COMMITTED_FILE_NAME, b"1")?;
    }
    Ok(())
}

/// (b)→(f) over what (a) resolved, under the lock.
fn destroy_resolved(
    lock: &WalletLock,
    resolver: Option<&dyn VaultResolver>,
    r: &Resolved<'_>,
    force: bool,
    on_unlink: &mut dyn FnMut(&std::ffi::OsStr),
) -> Result<DestroyOutcome, WalletError> {
    let db_dir = lock.dir();
    let (severed, purged) = purge_resolved(resolver, r)?;
    let severed_late = purged.iter().any(bounded::severed_late);
    guard_real_sever(r, severed, severed_late, force)?;

    // ── Files SECOND: best-effort cleanup of (now-dead) ciphertext. ──
    // Breadcrumb BEFORE the deletes, so a crash anywhere between here and the
    // end lets the re-run skip the guard + converge (closing the `purge →
    // remove_file` window the earlier "artifact removed first" reasoning did
    // NOT cover). Swept by the sweep it precedes, so a SUCCESSFUL wipe leaves
    // no breadcrumb.
    if severed > 0 {
        write_breadcrumb(db_dir);
    }
    let cleanup = sweep_files(lock, on_unlink);
    forget_index(r, &purged);
    // A wipe of this path completed: a duress sever's tombstones on it (and on
    // the namespaces that sever linked) are spent (§3.1 item 6).
    bounded::clear_severed_path(&r.path_ns);
    let converged = if severed > 0 {
        Converged::Severed
    } else if r.wipe_already_committed {
        Converged::Breadcrumb
    } else if severed_late {
        Converged::SeveredLate
    } else {
        Converged::Empty
    };
    let outcome = DestroyOutcome { severed, converged };
    match cleanup {
        Ok(()) => Ok(outcome),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(outcome),
        // The crypto-shred is COMPLETE the instant `purge` returned (the keychain
        // wrap key is gone; what remains on disk is dead ciphertext). A cleanup
        // failure is surfaced via tracing (only the io kind — NO path/namespace,
        // §5.4 + the wipe log allowlist) and returns Ok: a hard Err would tell the
        // host the shred failed when it did not, and a retry re-trips nothing.
        Err(e) => {
            tracing::warn!(io_kind = ?e.kind(), "wallet wipe: file cleanup incomplete after crypto-shred");
            Ok(outcome)
        }
    }
}

/// (e) The file sweep — the ONLY piece that deletes a file, and it takes the
/// held [`WalletLock`] as its witness: the duress sever's keychain-only path
/// ([`CustodyOnlyDir`]) cannot reach it.
fn sweep_files(
    lock: &WalletLock,
    on_unlink: &mut dyn FnMut(&std::ffi::OsStr),
) -> std::io::Result<()> {
    let db_dir = lock.dir();
    let _ = std::fs::remove_file(db_dir.join(WRAP_ARTIFACT_FILE_NAME));
    // Sweep the CONTENTS first — wallet.db + WAL/SHM sidecars + block-cache +
    // seals + manifest + marker — while the caller STILL HOLDS the advisory
    // lock, and unlink the lock file LAST (#317, the unlink-race fold).
    // The old `remove_dir_all` deleted the lock mid-hold, so a concurrent
    // `open` could create+flock a FRESH lock inode and start provisioning INTO
    // the directory this sweep was still deleting (fail-safe for funds — the
    // keychain was severed first — but it could clobber the newcomer's fresh
    // wallet). With the lock swept last, the destructive pass can never
    // overlap a newcomer: they either wait on OUR inode (and find the dir
    // empty/gone afterward) or lock a fresh inode only after the sweep is
    // done — at which point the residual `remove_dir` below simply fails
    // ENOTEMPTY and leaves their directory alone. (Windows residual: unlinking
    // a LockFileEx'd file fails — that arm lands in the warn-and-Ok cleanup
    // policy in `destroy_resolved`, same as today.)
    (|| {
        for entry in std::fs::read_dir(db_dir)? {
            let entry = entry?;
            if entry.file_name().to_str() == Some(crate::constants::WALLET_LOCK_FILE_NAME) {
                continue; // the advisory lock goes LAST, below
            }
            on_unlink(&entry.file_name());
            // A vanished entry (external cleanup / AV racing the wipe) must not
            // abort the sweep — and must NOT reach the closure-level NotFound
            // arm below, which means "db_dir already gone" and would mask a
            // PARTIAL sweep as complete, unlogged (review F1, probed).
            let removed = (|| {
                let p = entry.path();
                if entry.file_type()?.is_dir() {
                    std::fs::remove_dir_all(&p)
                } else {
                    std::fs::remove_file(&p)
                }
            })();
            match removed {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        on_unlink(std::ffi::OsStr::new(
            crate::constants::WALLET_LOCK_FILE_NAME,
        ));
        match std::fs::remove_file(db_dir.join(crate::constants::WALLET_LOCK_FILE_NAME)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        std::fs::remove_dir(db_dir)
    })()
}

/// (f) The index item LAST (S2): it is the breadcrumb that finds the custody
/// namespace when the directory is already gone, so it must outlive the file
/// sweep. Best-effort: the crypto-shred completed before it, and a delete
/// fault only leaves a stale entry whose namespace is ALREADY severed — the
/// next wipe re-resolves it, severs nothing, and deletes it (converges). Then
/// any late-sever evidence for the purged namespaces is spent.
fn forget_index(r: &Resolved<'_>, purged: &[crate::keychain::KeychainNamespace]) {
    if let Some(v) = r.path_vault.as_ref()
        && let Err(e) = v.as_port().delete_index()
    {
        tracing::warn!(
            target: "zec_wallet_core",
            outcome = e.code(),
            "wallet.wipe_index_cleanup",
        );
    }
    for ns in purged {
        bounded::forget_severed_late(ns);
    }
}

// ── The duress sever (FR-53, stage S16 §3.1) ─────────────────────────────────

/// The witness for the duress sever's keychain-only path: it names the
/// directory — so the path namespace, the wrap artifact's header and the
/// breadcrumb can be reached — and nothing that deletes a file will take it
/// (the §6.3 witness discipline: the type decides what the path may be used
/// for). [`sweep_files`] takes the held [`WalletLock`] instead.
pub(crate) struct CustodyOnlyDir<'a>(pub(crate) &'a Path);

/// What the sever did to the custody, as the store saw it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CustodySevered {
    /// A non-zero count was severed — the proof.
    Proven(usize),
    /// A zero count on a store whose custody was already gone (empty, no
    /// artifact, a breadcrumb, a late sever).
    AlreadyGone,
    /// The locked keychain's blind scoped delete (§3.1 item 7): every delete
    /// returned success or not-found, and no count could be read.
    Unproven,
}

/// A sever's store-level answer: the custody outcome, and whether the files
/// were removed (the free-lock branch only).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct StoreSever {
    pub(crate) custody: CustodySevered,
    pub(crate) files_removed: bool,
}

/// Where (a)'s resolution sends a sever: on to the purge, or — the keychain
/// answering locked (the index unreadable, or `KeystoreUnavailable`) — to the
/// blind scoped delete, with the header's id if one can be read.
enum Routed<'a> {
    Resolved(Resolved<'a>),
    Blind(Option<CustodyId>),
}

/// The ONE routing of both sever branches (held and free lock), so neither can
/// reach the blind path by a different rule than the other.
fn resolve_or_blind<'a>(
    db_dir: &Path,
    resolver: Option<&'a dyn VaultResolver>,
) -> Result<Routed<'a>, WalletError> {
    match resolve_custody(db_dir, resolver, false) {
        Ok(r) if !r.index_locked => Ok(Routed::Resolved(r)),
        Ok(r) => Ok(Routed::Blind(r.header_id)),
        Err(WalletError::KeystoreUnavailable) => Ok(Routed::Blind(custody_header_id(db_dir))),
        Err(e) => Err(e),
    }
}

/// The sever with the lock HELD by someone else (§3.1 item 3): (a), the
/// tombstone on every resolved namespace, (b), (c), (d), (f) — never (e). The
/// caller has already tombstoned the path namespace (step (i)). The guard's
/// trip is `KeystoreInconsistent`, as in `destroy`; a locked keychain takes
/// the blind path.
pub(crate) fn sever_custody_only(
    dir: CustodyOnlyDir<'_>,
    resolver: Option<&dyn VaultResolver>,
) -> Result<CustodySevered, WalletError> {
    let db_dir = dir.0;
    let resolved = match resolve_or_blind(db_dir, resolver)? {
        Routed::Resolved(r) => r,
        Routed::Blind(header_id) => return blind_sever(db_dir, resolver, header_id),
    };
    // (iv) Tombstone every namespace (a) resolved BEFORE their purge is
    // submitted: a key written there before the purge job runs is purged by
    // it, one written after is refused inside its own job.
    bounded::tombstone_linked(&resolved.path_ns, &resolved_namespaces(&resolved));
    let (severed, purged) = match purge_resolved(resolver, &resolved) {
        Ok(v) => v,
        Err(WalletError::KeystoreUnavailable) => {
            return blind_sever(db_dir, resolver, resolved.header_id);
        }
        Err(e) => return Err(e),
    };
    let severed_late = purged.iter().any(bounded::severed_late);
    guard_real_sever(&resolved, severed, severed_late, false)?;
    if severed > 0 {
        // `proven` BEFORE the write, so a racing open's sweep either runs
        // before this write or sees the mark and puts the breadcrumb back.
        bounded::mark_proven(&resolved.path_ns);
        write_breadcrumb(db_dir);
    }
    forget_index(&resolved, &purged);
    Ok(if severed > 0 {
        CustodySevered::Proven(severed)
    } else {
        CustodySevered::AlreadyGone
    })
}

/// The sever with the lock FREE and held by the sever itself (§3.1 item 2):
/// the plain destroy, whose count is the proof — unless the keychain is
/// locked, where it takes the blind path and removes NO file (deleting the
/// seals while a filtered key may survive is the `wipe_force` hazard).
pub(crate) fn sever_under_lock(
    lock: &WalletLock,
    resolver: Option<&dyn VaultResolver>,
) -> Result<StoreSever, WalletError> {
    let db_dir = lock.dir();
    let blind = |header_id| {
        blind_sever(db_dir, resolver, header_id).map(|custody| StoreSever {
            custody,
            files_removed: false,
        })
    };
    let resolved = match resolve_or_blind(db_dir, resolver)? {
        Routed::Resolved(r) => r,
        Routed::Blind(header_id) => return blind(header_id),
    };
    match destroy_resolved(lock, resolver, &resolved, false, &mut |_| {}) {
        Ok(outcome) => Ok(StoreSever {
            custody: if outcome.severed > 0 {
                CustodySevered::Proven(outcome.severed)
            } else {
                CustodySevered::AlreadyGone
            },
            files_removed: true,
        }),
        Err(WalletError::KeystoreUnavailable) => blind(resolved.header_id),
        Err(e) => Err(e),
    }
}

/// The locked keychain's BLIND SCOPED delete (§3.1 item 7, Relim's choice at
/// sync point 1): the count cannot be read, so the deletes are issued anyway,
/// but only under OUR namespaces — the path namespace, and the id namespace
/// the wrap artifact's header names (unproven: the proof needs the keychain).
/// Every delete is attempted; any failure is the answer. No breadcrumb and no
/// `proven` mark: a not-found while locked may be a key filtered from view,
/// and a breadcrumb would disarm a later wipe's guard over a key that
/// survived. RESIDUAL (ADR-0565): the header is trusted unproven.
fn blind_sever(
    db_dir: &Path,
    resolver: Option<&dyn VaultResolver>,
    header_id: Option<CustodyId>,
) -> Result<CustodySevered, WalletError> {
    let Some(resolver) = resolver else {
        return Err(WalletError::VaultAbsent);
    };
    let path_ns = crate::wallet::keychain_namespace_for(db_dir);
    let header_ns = header_id
        .as_ref()
        .map(crate::custody::namespace_for)
        .filter(|ns| *ns != path_ns);
    // Tombstone the path and the header's namespace BEFORE the deletes, on
    // BOTH branches (the S16 diff review's CRITICAL): the held branch already
    // tombstoned the path in step (i), but a free-lock sever reaches here with
    // none. An unproven delete may have left a key the lock hid, so an open
    // in this process must refuse `Wiped` until a plain wipe of the path
    // converges and clears it — the same standing tombstone as the held case.
    bounded::tombstone_path(&path_ns);
    bounded::tombstone_linked(&path_ns, header_ns.as_slice());
    // No log line: the sever has begun (the report's `SeveredUnproven` says it).
    let mut first_err: Option<WalletError> = None;
    let mut note = |r: Result<(), WalletError>| {
        if let Err(e) = r
            && first_err.is_none()
        {
            first_err = Some(e);
        }
    };
    for ns in std::iter::once(&path_ns).chain(header_ns.as_ref()) {
        note(
            resolver
                .vault_for(ns)
                .and_then(|v| bounded::purging(ns, || v.as_port().purge_namespace()).map(|_| ())),
        );
    }
    note(
        resolver
            .vault_for(&path_ns)
            .and_then(|v| v.as_port().delete_index()),
    );
    match first_err {
        None => Ok(CustodySevered::Unproven),
        Some(e) => Err(e),
    }
}

// ── Custody load (shared by open + repair) ───────────────────────────────────

/// What a custody load recovered. (The custody IDENTITY is observable through
/// the wrap artifact file — `custody_header_id` — and the keychain slots; it
/// deliberately rides no wider surface.)
pub(crate) struct LoadedCustody {
    pub(crate) seed: Option<SeedPayload>,
    pub(crate) db_key: WalletDbKey,
    pub(crate) status: VaultStatus,
}

/// The pre-S2 vault-shaped loader (kept for the store's own test seams);
/// production loads through [`load_custody_resolving`].
#[cfg_attr(not(test), allow(dead_code))]
fn load_custody(
    db_dir: &Path,
    vault: &dyn KeychainPort,
    persistence: PersistenceKind,
) -> Result<(Option<SeedPayload>, WalletDbKey, VaultStatus), WalletError> {
    let loaded = load_custody_resolving(db_dir, &BorrowedVault(vault), persistence)?;
    Ok((loaded.seed, loaded.db_key, loaded.status))
}

/// S2 custody load: read the wrap artifact's header FIRST, then load through
/// the vault for the namespace the header names — the id namespace for a
/// post-S2 wallet, the legacy PATH namespace for a pre-stage one (no locator
/// in the file). Integrity is the AEAD's: a locator that names a namespace
/// whose wrap key does not authenticate this artifact is a typed refusal
/// (`WrapArtifactInvalid` / the keysMissing class) — never a wrong wallet.
///
/// A pre-stage wallet MIGRATES here (the contract's one-commit-point sequence
/// lives in [`migrate_pre_stage_wallet`]); the index item under the path
/// namespace is kept current at every successful load of a committed wallet
/// — written when missing or wrong, which is the rewrite-after-relocation the
/// identity's portability depends on ([`settle_index_after_open`]).
fn load_custody_resolving(
    db_dir: &Path,
    resolver: &dyn VaultResolver,
    persistence: PersistenceKind,
) -> Result<LoadedCustody, WalletError> {
    let dbkey_blob = read_seal_file(db_dir, DBKEY_SEAL_FILE_NAME)?;
    let seed_blob = match persistence {
        PersistenceKind::SealedKeychain => Some(read_seal_file(db_dir, SEED_SEAL_FILE_NAME)?),
        // Watch-only (#397): structurally seed-less — custody is the DB key
        // only, exactly the None shape.
        PersistenceKind::None | PersistenceKind::WatchOnly => None,
    };
    let artifact = WrapArtifact::from_bytes(read_seal_file(db_dir, WRAP_ARTIFACT_FILE_NAME)?)?;
    let path_ns = crate::wallet::keychain_namespace_for(db_dir);
    match artifact.custody_id() {
        Some(id) => {
            // Post-S2: load under the identity's namespace. NO legacy fallback
            // on failure — a migrated wallet that consults the path derivation
            // is exactly the mutant the contract's row 4 reddens.
            let id_ns = crate::custody::namespace_for(&id);
            let id_vault = resolver.vault_for(&id_ns)?;
            let vault = id_vault.as_port();
            let loaded = SealedSeedVault::new(vault).load_wallet(
                seed_blob.as_deref(),
                &dbkey_blob,
                &artifact,
            )?;
            // Step (5) and the index, AFTER the load proved the wallet opens:
            // best-effort, never a refused open (see `settle_index_after_open`).
            settle_index_after_open(resolver, &path_ns, &id, vault);
            Ok(LoadedCustody {
                seed: loaded.seed,
                db_key: loaded.db_key,
                status: loaded.status,
            })
        }
        None => {
            // Pre-stage: the wallet still custodies under the path namespace.
            // Loading here PROVES that custody is alive before anything is
            // minted — a MIGRATED wallet whose locator was tampered to none
            // dies right here (its legacy namespace was purged at step (5)),
            // the typed refusal the contract's tamper row demands, BEFORE any
            // key is re-custodied or any id minted.
            let legacy_vault = resolver.vault_for(&path_ns)?;
            let legacy_vault = legacy_vault.as_port();
            let _ = SealedSeedVault::new(legacy_vault).load_wallet(
                seed_blob.as_deref(),
                &dbkey_blob,
                &artifact,
            )?;
            migrate_pre_stage_wallet(
                db_dir,
                resolver,
                &path_ns,
                legacy_vault,
                &artifact,
                seed_blob.as_deref(),
                &dbkey_blob,
            )
        }
    }
}

/// The migration's one-commit-point sequence (contract §3.1 mechanism):
/// (1)/(2) mint the id — or REUSE the pending index's id, so a re-run after a
/// crash in any window never orphans a namespace with a fresh mint; the index
/// `{id, legacy_ns, pending}` is written FIRST. (3) `store_wrap_key` under the
/// id namespace (the legacy item stays live — the crash window after (3)
/// re-opens under it and re-runs). (4) **the commit:** the single
/// `write_atomic` of the wrap artifact carrying the locator. (5) is DEFERRED:
/// on the NEXT successful open under the id namespace
/// ([`load_custody_resolving`]'s committed-wallet branch) the legacy namespace
/// is purged and the index set `done` — this call proves the new custody opens
/// and returns it, nothing past the commit.
#[allow(clippy::too_many_arguments)]
fn migrate_pre_stage_wallet(
    db_dir: &Path,
    resolver: &dyn VaultResolver,
    path_ns: &crate::keychain::KeychainNamespace,
    legacy_vault: &dyn KeychainPort,
    artifact: &WrapArtifact,
    seed_blob: Option<&[u8]>,
    dbkey_blob: &[u8],
) -> Result<LoadedCustody, WalletError> {
    let path_vault_handle = resolver.vault_for(path_ns)?;
    let path_vault = path_vault_handle.as_port();
    // (1)/(2): the id, reused ONLY from this path's own unfinished migration —
    // a `pending` record naming this path as its legacy namespace. The pending
    // index item IS the record: it carries the legacy namespace the DEFERRED
    // step (5) reads back on the next open. Any other entry (a `done` one left
    // by a wallet that moved away or was deleted without a wipe) is NOT this
    // migration's: borrowing its identifier would custody this wallet under
    // another wallet's namespace (Apple's Secure Enclave `store_wrap_key`
    // begins by deleting everything there) and would leave no `pending`
    // record, so the legacy key would never be purged. Mint, and overwrite it.
    let id = match read_index(path_vault)? {
        Some(entry)
            if entry.is_pending()
                && entry.legacy_ns.as_ref().map(|ns| ns.as_str()) == Some(path_ns.as_str()) =>
        {
            entry.id
        }
        _ => {
            let id = CustodyId::generate();
            path_vault.store_index(&CustodyIndexEntry::pending(&id, path_ns))?;
            id
        }
    };
    let id_ns = crate::custody::namespace_for(&id);
    let id_vault_handle = resolver.vault_for(&id_ns)?;
    let id_vault = id_vault_handle.as_port();
    // (3): re-custody the SAME SealKey under the id namespace. The AAD binds
    // to the same primary blob — the seal files never move.
    let primary: &[u8] = seed_blob.unwrap_or(dbkey_blob);
    // The key moves into custody (FR-47: by value); nothing below uses it —
    // the proof-of-open reloads it through `load_wallet`.
    let seal_key = legacy_vault.load_wrap_key(artifact, primary)?;
    #[cfg(test)]
    crate::wallet::sever_seams::fire(
        crate::wallet::sever_seams::Seam::MigrationBeforeStoreWrap,
        db_dir,
    );
    let new_artifact = id_vault
        .store_wrap_key(seal_key, primary)?
        .with_custody_id(&id);
    // (4): THE COMMIT — the single `write_atomic` that carries the identifier.
    // Before it, every step is idempotent and resumable; after it, deferred
    // and recorded (the pending index names the legacy namespace to purge).
    write_atomic(db_dir, WRAP_ARTIFACT_FILE_NAME, &new_artifact.file_bytes())?;
    // The committing call STOPS at the commit: step (5) — the legacy purge +
    // the index `done` — runs on the NEXT successful open under the id
    // namespace, so a purge that cannot complete (a wedged keystore) never
    // fails an open that has already committed. This call proves the new
    // custody opens — the same load that next open performs — and returns it.
    let loaded =
        SealedSeedVault::new(id_vault).load_wallet(seed_blob, dbkey_blob, &new_artifact)?;
    Ok(LoadedCustody {
        seed: loaded.seed,
        db_key: loaded.db_key,
        status: loaded.status,
    })
}

/// The post-load bookkeeping of a committed wallet's open — contract §3.1
/// step (5) and "the index … written at every successful open", after the
/// load above it has already proven the wallet opens:
/// - this path's own `pending` record → the deferred legacy purge, then
///   `done`. A purge that fails leaves the record `pending` (so a wipe still
///   severs both namespaces) and the next open retries it — the deferral's
///   point: a post-commit keystore fault never refuses a committed wallet's
///   open (the s2-custody ruling §1.2).
/// - the index already `done` for this identifier → NOTHING is written. On
///   Android an index write deletes the prior alias and generates a new
///   Keystore key (StrongBox first), so an unconditional write is a key
///   generation per open and a window with no index at all.
/// - missing (the first open after a relocation) or naming another
///   identifier → written `done`. A write that fails is retried at the next
///   open (the entry is still missing or wrong); until then a wipe from a
///   bare config at THIS path cannot find the namespace — surfaced, like
///   every fault here, as a payload-free outcome code (§5.4).
fn settle_index_after_open(
    resolver: &dyn VaultResolver,
    path_ns: &crate::keychain::KeychainNamespace,
    id: &CustodyId,
    id_vault: &dyn KeychainPort,
) {
    let path_handle = match resolver.vault_for(path_ns) {
        Ok(handle) => handle,
        Err(e) => {
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = e.code(),
                "wallet.custody_index_deferred",
            );
            return;
        }
    };
    let path_vault = path_handle.as_port();
    // A read that TIMED OUT (FR-47; the only error `read_index` returns) is not
    // "no index": writing `done` over it could replace a `pending` record and
    // strand the legacy key. Nothing is written; the next open settles.
    let current = match read_index(path_vault) {
        Ok(current) => current,
        Err(e) => {
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = e.code(),
                "wallet.custody_index_deferred",
            );
            return;
        }
    };
    let done = CustodyIndexEntry::done(id);
    match current {
        // Only this path's own pending record is acted on — its legacy
        // namespace IS this path's (the one value a migration records); a
        // record naming any other namespace falls through and is overwritten.
        Some(entry)
            if entry.id == *id
                && entry.is_pending()
                && entry.legacy_ns.as_ref() == Some(path_ns) =>
        {
            if let Some(legacy_ns) = &entry.legacy_ns
                && let Err(e) = purge_legacy_unless_conflated(resolver, legacy_ns, id_vault)
            {
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = e.code(),
                    "wallet.custody_legacy_purge_deferred",
                );
                return;
            }
        }
        Some(entry) if entry == done => return,
        _ => {}
    }
    if let Err(e) = path_vault.store_index(&done) {
        tracing::warn!(
            target: "zec_wallet_core",
            outcome = e.code(),
            "wallet.custody_index_deferred",
        );
    }
}

/// Step (5)'s purge, guarded against the fixed-vault seams: when the resolver
/// hands out the SAME vault instance for both namespaces (a pre-built
/// `*_with_vault` vault — the base tree's shape), purging "the legacy
/// namespace" would sever the JUST-written id-namespace custody. A real
/// resolver constructs a distinct vault per namespace, where the purge is the
/// contract's deferred cleanup; the address comparison suppresses exactly the
/// conflated case and nothing else (`addr_eq`, not `eq`: two `dyn` pointers
/// to one object may carry different vtable pointers).
fn purge_legacy_unless_conflated(
    resolver: &dyn VaultResolver,
    legacy_ns: &crate::keychain::KeychainNamespace,
    id_vault: &dyn KeychainPort,
) -> Result<(), WalletError> {
    let legacy_handle = resolver.vault_for(legacy_ns)?;
    let legacy_vault = legacy_handle.as_port();
    let same = std::ptr::addr_eq(
        legacy_vault as *const dyn KeychainPort,
        id_vault as *const dyn KeychainPort,
    );
    if same {
        return Ok(());
    }
    legacy_vault.purge_namespace()?;
    Ok(())
}

/// Whether the wrap key under `id`'s namespace unseals THIS directory's seals —
/// the open's own authentication (the §4.3a AAD on the wrap layer, the §4.2a
/// envelope on the seal layer; on Apple, where the wrap layer has no AAD, the
/// seal layer is what refuses a foreign key). A wipe asks it before letting a
/// header the keychain index does not corroborate name what is severed. Any
/// fault reads as "does not": the caller then goes by the index — except a
/// key-store call that TIMED OUT (FR-47), which is no answer at all and is
/// returned.
fn header_unseals(
    db_dir: &Path,
    resolver: &dyn VaultResolver,
    id: &CustodyId,
) -> Result<bool, WalletError> {
    let attempt = || -> Result<(), WalletError> {
        let artifact = WrapArtifact::from_bytes(read_seal_file(db_dir, WRAP_ARTIFACT_FILE_NAME)?)?;
        let dbkey_blob = read_seal_file(db_dir, DBKEY_SEAL_FILE_NAME)?;
        let seed_blob = if file_exists(&db_dir.join(SEED_SEAL_FILE_NAME))? {
            Some(read_seal_file(db_dir, SEED_SEAL_FILE_NAME)?)
        } else {
            None
        };
        let vault = resolver.vault_for(&crate::custody::namespace_for(id))?;
        SealedSeedVault::new(vault.as_port()).load_wallet(
            seed_blob.as_deref(),
            &dbkey_blob,
            &artifact,
        )?;
        Ok(())
    };
    match attempt() {
        Ok(()) => Ok(true),
        Err(e @ WalletError::KeychainTimeout { .. }) => Err(e),
        Err(_) => Ok(false),
    }
}

/// Read the path namespace's index item, FAIL-OPEN to `None`: the header is
/// the authority and the index a breadcrumb — a corrupt item must not brick
/// an open (the caller rewrites it) or a wipe (the header still resolves).
/// Surfaced, not silent: a payload-free outcome code (§5.4 — the identifier
/// itself never reaches a log). The ONE error it returns is a read that
/// TIMED OUT (FR-47): the key store never answered, which is not "absent" —
/// every other error (a locked keychain included) stays fail-open.
fn read_index(vault: &dyn KeychainPort) -> Result<Option<CustodyIndexEntry>, WalletError> {
    match vault.load_index() {
        Ok(entry) => Ok(entry),
        Err(e) => {
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = e.code(),
                "wallet.custody_index_unreadable",
            );
            match e {
                WalletError::KeychainTimeout { .. } => Err(e),
                _ => Ok(None),
            }
        }
    }
}

/// Lock-free, best-effort read of the wrap artifact's custody locator (the
/// disclosure probe's namespace resolver — a wallet's tier lives under the
/// namespace its header names). ANY fault reads as `None` (fall back to the
/// path namespace): this is a read-only honesty probe, never a gate.
pub(crate) fn custody_header_id(db_dir: &Path) -> Option<CustodyId> {
    let path = db_dir.join(WRAP_ARTIFACT_FILE_NAME);
    let meta = std::fs::metadata(&path).ok()?;
    if meta.len() > crate::keychain::WRAP_ARTIFACT_MAX_BYTES as u64 {
        return None;
    }
    let bytes = std::fs::read(&path).ok()?;
    WrapArtifact::from_bytes(bytes).ok()?.custody_id()
}

// ── Manifest ─────────────────────────────────────────────────────────────────

struct Manifest {
    network: Network,
    persistence: PersistenceKind,
    /// #357: the seed provenance. `Unknown` for a pre-#357 v1 manifest.
    seed_source: ManifestSeedSource,
}

fn encode_manifest(
    network: Network,
    persistence: PersistenceKind,
    seed_source: ManifestSeedSource,
) -> [u8; MANIFEST_LEN_V2] {
    let net = match network {
        Network::Main => 0u8,
        Network::Test => 1u8,
    };
    // v2: [ver][network][persistence][seed_source] — spending wallets.
    // v3 (#397): SAME layout, watch-only ONLY — the version byte is the
    // capability gate (a pre-#397 binary rejects it typed at the version
    // dispatch instead of half-opening a store that has no seed custody).
    let version = if persistence == PersistenceKind::WatchOnly {
        crate::constants::MANIFEST_VERSION_V3
    } else {
        PROVISION_MANIFEST_VERSION
    };
    [version, net, persistence.to_byte(), seed_source.to_byte()]
}

fn read_manifest(db_dir: &Path) -> Result<Manifest, WalletError> {
    let bytes = read_seal_file(db_dir, MANIFEST_FILE_NAME)?;
    // Validate-before-use: dispatch on the version byte, then demand the EXACT
    // length for THAT version + in-range enums. An unknown version or a length
    // matching neither is corruption — never guessed at (§4.6).
    let version = *bytes.first().ok_or(WalletError::StoreCorrupt)?;
    let (seed_source, persistence) = match version {
        MANIFEST_VERSION_V1 => {
            if bytes.len() != MANIFEST_LEN_V1 {
                return Err(WalletError::StoreCorrupt);
            }
            // A pre-#357 wallet carried no seed-source byte — the conservative
            // no-stamp arm, unchanged.
            (
                ManifestSeedSource::Unknown,
                PersistenceKind::from_byte(bytes[2])?,
            )
        }
        MANIFEST_VERSION_V2 => {
            if bytes.len() != MANIFEST_LEN_V2 {
                return Err(WalletError::StoreCorrupt);
            }
            // v2 = a SPENDING wallet: both decoders reject the watch-only
            // bytes (a v2 manifest claiming watch-only is corruption).
            (
                ManifestSeedSource::from_byte(bytes[3])?,
                PersistenceKind::from_byte(bytes[2])?,
            )
        }
        // #397 (spec §3.7 D4): v3 = WATCH-ONLY, exactly — the byte pair is
        // matched EXPLICITLY (never through the v2 decoders) and anything
        // else under this version is corruption: no mixed states, so a v3
        // manifest can never half-claim seed custody.
        crate::constants::MANIFEST_VERSION_V3 => {
            if bytes.len() != crate::constants::MANIFEST_LEN_V3 {
                return Err(WalletError::StoreCorrupt);
            }
            if bytes[2] != PersistenceKind::WatchOnly.to_byte()
                || bytes[3] != ManifestSeedSource::WatchOnly.to_byte()
            {
                return Err(WalletError::StoreCorrupt);
            }
            (ManifestSeedSource::WatchOnly, PersistenceKind::WatchOnly)
        }
        _ => return Err(WalletError::StoreCorrupt),
    };
    let network = match bytes[1] {
        0 => Network::Main,
        1 => Network::Test,
        _ => return Err(WalletError::StoreCorrupt),
    };
    Ok(Manifest {
        network,
        persistence,
        seed_source,
    })
}

// ── Durable IO primitives ────────────────────────────────────────────────────

/// Write `bytes` to `db_dir/name` atomically: tmp → fsync(file) → rename →
/// fsync(dir). The rename is the atomic-replace; the dir fsync makes it durable
/// across a power cut (POSIX; the platforms we ship — §9). ENOSPC ⇒ `DiskFull`.
fn write_atomic(db_dir: &Path, name: &str, bytes: &[u8]) -> Result<(), WalletError> {
    let final_path = db_dir.join(name);
    // A kill between create(tmp) and rename leaves a stale `<name>.tmp`. Harmless
    // (reads target the final names; a retry re-creates/truncates the tmp) — but it
    // is a same-ciphertext blob, so the §3.3 WIPE chunk must sweep `*.tmp` too so a
    // seized device can't yield a partially-written `seed.seal.tmp` (crypto HARDENING).
    let tmp_path = db_dir.join(format!("{name}.tmp"));
    {
        let mut f = File::create(&tmp_path).map_err(WalletError::from_io)?;
        f.write_all(bytes).map_err(WalletError::from_io)?;
        f.sync_all().map_err(WalletError::from_io)?;
    }
    std::fs::rename(&tmp_path, &final_path).map_err(WalletError::from_io)?;
    sync_dir(db_dir)
}

/// Write the completion marker LAST, CAS-style (`create_new`). An existing
/// marker is tolerated (idempotent re-run); the marker's mere PRESENCE is the
/// signal — its content is irrelevant. fsync'd + dir-fsync'd for durability.
fn write_marker(db_dir: &Path) -> Result<(), WalletError> {
    let path = db_dir.join(COMPLETE_MARKER_FILE_NAME);
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(f) => {
            f.sync_all().map_err(WalletError::from_io)?;
        }
        // Already present: the wallet is already marked complete — idempotent.
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(WalletError::from_io(e)),
    }
    sync_dir(db_dir)
}

fn sync_dir(dir: &Path) -> Result<(), WalletError> {
    // SEAM NOTE — DESKTOP PORTABILITY (arch HARDENING; maintainer's keeps-honest rule):
    // `File::open(dir).sync_all()` is the correct POSIX directory-fsync on Linux /
    // macOS / iOS / Android. On WINDOWS (a §9 desktop GA target, v2) opening a
    // directory handle for FlushFileBuffers needs a non-default share/access mode,
    // so this will need a `cfg(windows)` variant (either a documented no-op — NTFS
    // journalling + the file fsync above already bound durability — or
    // `NtFlushBuffersFileEx`). Named here so the desktop adapter author is not
    // surprised; no functional change is owed before Windows desktop ships.
    File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(WalletError::from_io)
}

fn file_exists(path: &Path) -> Result<bool, WalletError> {
    path.try_exists().map_err(WalletError::from_io)
}

/// Read a small store artifact, size-capped BEFORE allocation (§4.6 hostile
/// input — these files are attacker-substitutable on a seized device). A
/// missing file when the manifest/marker promised it is `StoreCorrupt`; an
/// over-cap file is `StoreCorrupt`. The envelope/`WrapArtifact` constructors
/// re-validate the exact shape.
fn read_seal_file(db_dir: &Path, name: &str) -> Result<Vec<u8>, WalletError> {
    let path = db_dir.join(name);
    // metadata() then read() is a two-syscall window, but the caller holds the
    // exclusive single-writer lock (lifecycle.rs) so no concurrent writer exists;
    // a shrink only yields a smaller (still-capped) read, a grow yields bytes the
    // envelope's own size window rejects (security HARDENING — lock closes it).
    // INVARIANT BOUNDARY: every `read_seal_file` caller reaches here under that
    // lock (via `open`/`create_or_repair`). The lock-FREE `inspect` probe
    // (`Wallet::exists`) deliberately reads NO bodies — only `file_exists` stats
    // — so it never enters this window; a future edit that adds a body read to
    // the `inspect` path would break this guarantee and must take the lock first.
    let meta = std::fs::metadata(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            WalletError::StoreCorrupt
        } else {
            WalletError::from_io(e)
        }
    })?;
    if meta.len() > SEAL_FILE_MAX_BYTES as u64 {
        return Err(WalletError::StoreCorrupt);
    }
    std::fs::read(&path).map_err(WalletError::from_io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keychain::VaultTier;
    use crate::keychain::testvault::TestVault;
    use zeroize::Zeroizing;

    // Crypto-rules: the open result carries the seed + DB key — pinned
    // non-Clone/non-Debug at compile time (structurally enforced by the
    // secret members, asserted here so a future field can't loosen it).
    static_assertions::assert_not_impl_any!(OpenWallet: Clone, std::fmt::Debug);

    fn sealed_payload() -> SeedPayload {
        SeedPayload::new(
            Zeroizing::new(vec![0x42; 64]),
            Some(Zeroizing::new(
                "provision crash-safety test seed phrase".to_owned(),
            )),
        )
        .expect("valid payload")
    }

    fn sealed_intent(seed: &SeedPayload) -> ProvisionIntent<'_> {
        ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(seed),
            repair_mode: RepairMode::VerifySupplied,
            freshly_generated_at: None,
        }
    }

    // The witnessed entry points take `&WalletLock`; in production the async
    // handle acquires it once and holds it (wallet.rs). These test helpers
    // acquire-forward-release per call (the unit tests are sequential, with no
    // concurrent opener) so each case still drives the REAL `WalletLock::acquire`
    // path and the witness type.
    fn create_or_repair_at(
        dir: &Path,
        vault: &dyn KeychainPort,
        intent: &ProvisionIntent,
    ) -> Result<(), WalletError> {
        let lock = WalletLock::acquire(dir)?;
        create_or_repair(&lock, vault, intent)
    }

    fn open_at(
        dir: &Path,
        vault: &dyn KeychainPort,
        network: Network,
    ) -> Result<OpenWallet, WalletError> {
        let lock = WalletLock::acquire(dir)?;
        open(&lock, vault, network)
    }

    /// W-swap-4-a-5: an `Empty` dir holding leftovers of a prior DB life must
    /// not brick provisioning. The PROVEN permanent brick (bite-validated: with
    /// the sweep disabled this test fails `StoreCorrupt`, identically on every
    /// retry) is a stray keyed `wallet.db` MAIN file — the new key's forced
    /// probe HMAC-fails forever; the partial-wipe warn-and-Ok arm is exactly
    /// the path that can leave one. An orphan `-wal` beside a fresh CREATE is
    /// swept as defense-in-depth: upstream SQLite DELETES a `-wal` beside a
    /// zero-page main file at first access rather than recovering it
    /// (`pagerOpenWalIfPresent` — library-internal, not contract; beside a
    /// NON-empty main the same `-wal` fails typed, which is why main +
    /// sidecars sweep together) — and the sweep also clears `-shm`/`-journal`.
    #[test]
    fn provision_sweeps_an_orphan_wal_sidecar_in_an_empty_dir() {
        // Arm 1 (the proven brick): a stray MAIN wallet.db under an unknown key.
        let dir = tempfile::tempdir().expect("tempdir");
        let alien_key = WalletDbKey::generate();
        {
            let conn = db::open_keyed_connection(&dir.path().join(WALLET_DB_FILE_NAME), &alien_key)
                .expect("stray main db from a prior life");
            conn.execute_batch("CREATE TABLE t (x INTEGER);")
                .expect("alien schema");
        }
        // Arm 2 (hygiene): a genuine non-empty alien `-wal`, copied while live —
        // a clean close would have unlinked it; the partial-wipe arm skipped that.
        let scratch = tempfile::tempdir().expect("scratch");
        let alien_path = scratch.path().join("alien.db");
        {
            let conn = db::open_keyed_connection(&alien_path, &alien_key).expect("alien open");
            conn.execute_batch("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1);")
                .expect("alien frames");
            let alien_wal =
                std::fs::read(db::with_suffix(&alien_path, "-wal")).expect("live alien wal");
            assert!(!alien_wal.is_empty(), "the fabricated wal carries frames");
            std::fs::write(
                dir.path().join(format!("{WALLET_DB_FILE_NAME}-wal")),
                alien_wal,
            )
            .expect("orphan planted");
        }

        assert_eq!(
            inspect(dir.path()).unwrap(),
            StoreState::Empty,
            "stray DB files alone still read Empty (no manifest/marker — nothing custodied)"
        );
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed))
            .expect("provisioning sweeps the prior life instead of bricking");
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Complete);
        // The real property: the fresh wallet opens — nothing alien reached it.
        open_at(dir.path(), &vault, Network::Main).expect("the provisioned wallet opens clean");
    }

    /// W-swap-4-a-6 (a) — security MED-1, bite-valid (the pre-4-a-6 order
    /// fails this test): custody runs BEFORE the destructive prior-life sweep.
    /// A locked/unavailable keystore at create — realistic on mobile — must
    /// leave a prior life's stray `wallet.db` byte-for-byte where it was. With
    /// the old order (sweep first) this sequence destroyed it while NOTHING
    /// else doomed it: step 2's seal overwrite (the 4-a-5 zero-marginal-loss
    /// argument) is never reached when custody fails.
    #[test]
    fn provision_custody_failure_leaves_a_prior_life_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");
        let stray = dir.path().join(WALLET_DB_FILE_NAME);
        let alien_key = WalletDbKey::generate();
        {
            let conn = db::open_keyed_connection(&stray, &alien_key).expect("prior-life db");
            conn.execute_batch("CREATE TABLE t (x INTEGER);")
                .expect("prior-life schema");
        }
        let before = std::fs::read(&stray).expect("stray bytes");

        // Custody fails at its probe-first door (`store_wallet` probes before
        // any key exists) — the vault-side shape of a locked keystore.
        let vault = TestVault::absent();
        let seed = sealed_payload();
        assert!(
            create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).is_err(),
            "provisioning fails at custody"
        );
        assert_eq!(
            std::fs::read(&stray).expect("the prior life is still present"),
            before,
            "a custody failure leaves the prior wallet.db untouched"
        );
    }

    /// W-swap-4-a-6 (b) — money MED, bite-valid (with the breadcrumb sweep
    /// disabled, the destroy below CONVERGES and deletes the live files): a
    /// successor wallet provisioned into a dir carrying a dead life's
    /// `.wipe-committed` breadcrumb must NOT inherit it — inherited, it disarms
    /// the destroy-time verify-real-sever guard (`severed == 0` reads as an
    /// expected interrupted-wipe re-run instead of the B1 mis-derived-namespace
    /// trap). Uses `SharedKeychainVault` (namespace-keyed) because the guard's
    /// scenario IS a namespace miss: same OS keychain, wrong namespace ⇒ purge
    /// severs zero while the on-disk artifact names the live successor's item.
    #[test]
    fn successor_destroy_fails_closed_after_a_dirty_prior_wipe() {
        use crate::keychain::testvault::SharedKeychainVault;

        let dir = tempfile::tempdir().expect("tempdir");
        // The dead life's dirty wipe: breadcrumb written (a genuine sever
        // happened), file cleanup incomplete (the warn-and-Ok arm) — plus a
        // stale `write_atomic` tmp remnant. The dir still reads `Empty`.
        std::fs::write(dir.path().join(WIPE_COMMITTED_FILE_NAME), b"1").expect("dirty prior wipe");
        std::fs::write(
            dir.path().join(format!("{SEED_SEAL_FILE_NAME}.tmp")),
            b"junk",
        )
        .expect("stale tmp remnant");
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Empty);

        // The successor life, custodied under namespace "wallet-a".
        let keychain = SharedKeychainVault::shared();
        let vault_a = SharedKeychainVault::new(VaultTier::Tee, "wallet-a", keychain.clone());
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault_a, &sealed_intent(&seed))
            .expect("the successor provisions over the dead life");
        assert!(
            !file_exists(&dir.path().join(WIPE_COMMITTED_FILE_NAME)).unwrap(),
            "provision hygiene swept the dead life's wipe breadcrumb"
        );
        assert!(
            !file_exists(&dir.path().join(format!("{SEED_SEAL_FILE_NAME}.tmp"))).unwrap(),
            "provision hygiene swept the dead life's stale tmp remnant"
        );

        // A mis-derived-namespace destroy (the B1 shape): same keychain,
        // namespace "wallet-b" ⇒ purge severs ZERO while artifact + files are
        // the LIVE successor's. Must fail CLOSED, files untouched.
        let vault_b = SharedKeychainVault::new(VaultTier::Tee, "wallet-b", keychain);
        {
            let lock = WalletLock::acquire(dir.path()).expect("lock");
            assert!(
                matches!(
                    destroy(&lock, Some(&vault_b), false),
                    Err(WalletError::KeystoreInconsistent {
                        permanently_invalidated: false
                    })
                ),
                "severed == 0 on a live successor fails closed — an inherited \
                 breadcrumb would have converged here and deleted the files"
            );
        }
        // Fail-closed means UNTOUCHED: the successor still opens under its vault.
        open_at(dir.path(), &vault_a, Network::Main)
            .expect("the successor survived the wrong-namespace destroy");
    }

    /// W-swap-4-a-6 (b), fold (#378 security MED) — bite-valid (with the
    /// final-name seal sweep disabled, the dead life's `seed.seal` survives the
    /// successor's provision): a SealedKeychain provision killed INSIDE step 2
    /// (`seed.seal` durable, manifest not) reads back `Empty`. A SealedKeychain
    /// successor overwrites the seals at step 2 — but a None-persistence
    /// successor never writes `seed.seal`, so without the hygiene sweep the
    /// dead life's SEALED SEED sits at rest for the successor's whole life
    /// under a manifest that promises "no seed at rest". Uses `TestVault`
    /// (the alias-GENERATION keystore shape, review fold) — the one where
    /// the dead life's wrap key genuinely outlives the successor's custody as
    /// the bounded orphan; on the fixed-item shape the successor's custody
    /// overwrites the dead item and the residue is dead ciphertext — the
    /// manifest-promise violation is identical, and the sweep covers both.
    #[test]
    fn none_persistence_successor_inherits_no_prior_seed_seal() {
        // The dead life: a REAL SealedKeychain provision (real ciphertext,
        // real keychain alias)…
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("dead life");
        // …reduced to the exact killed-mid-step-2 residue: seals on disk,
        // manifest/marker/db never written (step 2 seals BEFORE the manifest;
        // step 3 never ran). On this alias-generation shape the dead life's
        // wrap key stays in the vault — provision never purges aliases, so it
        // survives the successor's custody below as the orphan.
        for name in [MANIFEST_FILE_NAME, COMPLETE_MARKER_FILE_NAME] {
            std::fs::remove_file(dir.path().join(name)).expect("mid-step-2 kill shape");
        }
        db::remove_db_and_sidecars(&dir.path().join(WALLET_DB_FILE_NAME)).expect("no step 3");
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Empty);
        assert!(file_exists(&dir.path().join(SEED_SEAL_FILE_NAME)).unwrap());

        // The None-persistence successor over the residue: step 2 writes only
        // dbkey.seal + wrap.artifact — NOTHING ever overwrites the dead
        // seed.seal; only the hygiene sweep removes it.
        let intent = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::None,
            seed: None,
            repair_mode: RepairMode::NoSeed,
            freshly_generated_at: None,
        };
        create_or_repair_at(dir.path(), &vault, &intent).expect("successor provisions");
        assert!(
            !file_exists(&dir.path().join(SEED_SEAL_FILE_NAME)).unwrap(),
            "provision hygiene swept the dead life's sealed seed"
        );
        let opened = open_at(dir.path(), &vault, Network::Main).expect("successor opens");
        assert!(opened.seed.is_none(), "the successor persists no seed");
        assert_eq!(opened.persistence, PersistenceKind::None);
    }

    /// →#374 review fold (all three reviewers): a DIRECTORY planted where the
    /// hygiene expects a file (`x.tmp/` here; `remove_path_if_present` owns the
    /// unit matrix) must not brick provisioning — destroy's sweep already had
    /// dir parity, and the file-only arm was a fresh brick shape in the very
    /// batch that un-bricks provisioning.
    #[test]
    fn provision_hygiene_removes_a_planted_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let planted = dir.path().join("x.tmp");
        std::fs::create_dir(&planted).expect("planted dir");
        std::fs::write(planted.join("junk"), b"x").expect("child");

        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed))
            .expect("provisioning sweeps the planted dir instead of bricking");
        assert!(!planted.exists(), "the planted dir was removed whole");
        open_at(dir.path(), &vault, Network::Main).expect("the wallet opens");
    }

    /// W-swap-4-a-6 (b) upgrade arm (→#374 arch review MED): the
    /// provision-time breadcrumb sweep cannot reach a wallet that was ALREADY
    /// provisioned into a dirty dir before the fix. `open` succeeding proves
    /// the wrap key is alive, so a breadcrumb present at open is provably a
    /// dead prior life's — open sweeps it, re-arming this life's destroy-time
    /// verify-real-sever guard.
    #[test]
    fn open_sweeps_a_provably_stale_wipe_breadcrumb() {
        use crate::keychain::testvault::SharedKeychainVault;

        let dir = tempfile::tempdir().expect("tempdir");
        let keychain = SharedKeychainVault::shared();
        let vault_a = SharedKeychainVault::new(VaultTier::Tee, "wallet-a", keychain.clone());
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault_a, &sealed_intent(&seed)).expect("provision");
        // The pre-4-a-6 inheritance: a dead life's breadcrumb already in the
        // dir when THIS wallet was provisioned (planted after the fact here —
        // same on-disk state).
        std::fs::write(dir.path().join(WIPE_COMMITTED_FILE_NAME), b"1")
            .expect("inherited breadcrumb");

        open_at(dir.path(), &vault_a, Network::Main).expect("opens");
        assert!(
            !file_exists(&dir.path().join(WIPE_COMMITTED_FILE_NAME)).unwrap(),
            "open swept the provably-stale breadcrumb"
        );

        // The guard is re-armed: a wrong-namespace destroy now fails closed.
        let vault_b = SharedKeychainVault::new(VaultTier::Tee, "wallet-b", keychain);
        let lock = WalletLock::acquire(dir.path()).expect("lock");
        assert!(matches!(
            destroy(&lock, Some(&vault_b), false),
            Err(WalletError::KeystoreInconsistent {
                permanently_invalidated: false
            })
        ));
    }

    /// `inspect` is the §6.3 source of truth, now reachable from an FFI boot
    /// probe (`Wallet::exists`) on every cold launch — so pin its FULL truth
    /// table exhaustively: over all four (marker, manifest) presence
    /// combinations it yields exactly one documented `StoreState` (or the typed
    /// `StoreCorrupt`) and never panics. The load-bearing money-safety row is
    /// (marker, no-manifest) → `StoreCorrupt`, NEVER `Empty` (which would route
    /// a corrupt store to CREATE). A future edit that added a third on-disk
    /// signal would have to extend this table.
    #[test]
    fn inspect_truth_table_over_marker_and_manifest_presence() {
        for &marker in &[false, true] {
            for &manifest in &[false, true] {
                let dir = tempfile::tempdir().expect("tempdir");
                if marker {
                    std::fs::write(dir.path().join(COMPLETE_MARKER_FILE_NAME), b"")
                        .expect("write marker");
                }
                if manifest {
                    std::fs::write(dir.path().join(MANIFEST_FILE_NAME), b"")
                        .expect("write manifest");
                }
                let got = inspect(dir.path());
                match (marker, manifest) {
                    // marker + manifest → an openable wallet
                    (true, true) => assert_eq!(got.unwrap(), StoreState::Complete),
                    // marker, NO manifest → corruption (fail-closed); never
                    // reported as empty — the money-safety routing
                    (true, false) => assert!(
                        matches!(got, Err(WalletError::StoreCorrupt)),
                        "marker-without-manifest is corrupt, got {got:?}"
                    ),
                    // manifest, no marker → a recoverable provisioning remnant
                    (false, true) => assert_eq!(got.unwrap(), StoreState::Remnant),
                    // neither → truly empty (the host offers create)
                    (false, false) => assert_eq!(got.unwrap(), StoreState::Empty),
                }
            }
        }
    }

    /// §8 named test — the headline crash-safety gate. For EVERY provisioning
    /// step, a kill there recovers idempotently: a pre-manifest kill re-
    /// provisions fresh; a post-manifest kill repairs from the remnant; in all
    /// cases `open` succeeds and the seed + DB key round-trip. Since inc-2b-ii-A,
    /// step 3 (`DbProvisioned`) runs the REAL `init_wallet_db` migrator, so this
    /// now also proves the DB-migration arm is crash-atomic + idempotent (a kill
    /// at `DbProvisioned` and the post-manifest repair both re-run the migrator
    /// cleanly) — not just the seal / custody / marker / create-vs-open layers.
    /// Every provisioning step in order, walked through an EXHAUSTIVE `match`:
    /// a new `ProvisionStep` variant does not compile until it names its
    /// successor here, so the kill loop below cannot silently skip it (the S16
    /// diff review: `IndexWritten` was missing from a hand-written array).
    fn every_provision_step() -> Vec<ProvisionStep> {
        fn next(step: ProvisionStep) -> Option<ProvisionStep> {
            match step {
                ProvisionStep::IndexWritten => Some(ProvisionStep::Custody),
                ProvisionStep::Custody => Some(ProvisionStep::PriorLifeSwept),
                ProvisionStep::PriorLifeSwept => Some(ProvisionStep::RemnantPersisted),
                ProvisionStep::RemnantPersisted => Some(ProvisionStep::DbProvisioned),
                ProvisionStep::DbProvisioned => Some(ProvisionStep::Completed),
                ProvisionStep::Completed => None,
            }
        }
        std::iter::successors(Some(ProvisionStep::IndexWritten), |s| next(*s)).collect()
    }

    #[test]
    fn provisioning_killed_at_any_step_recovers_idempotently() {
        let steps = every_provision_step();
        assert_eq!(steps.len(), 6, "every step is walked: {steps:?}");
        for kill in steps {
            let dir = tempfile::tempdir().expect("tempdir");
            // A prior life's stray DB under the sweep (W-swap-4-a-6): a kill at
            // `PriorLifeSwept` — post-sweep, pre-step-2 — must still recover
            // (state stays `Empty`; the retry idempotently re-sweeps nothing).
            std::fs::write(dir.path().join(WALLET_DB_FILE_NAME), b"dead prior life")
                .expect("stray prior db");
            // …and the dead life's FINAL-NAME seal residue under the seal
            // sweep (#378): every kill point must converge over it too, and no
            // dead seal byte may survive into a Complete store (step 2's
            // rewrite replaces dbkey/wrap; the sweep is what removes seed.seal
            // for a would-be None successor — here the retry re-seals over it).
            for name in [
                SEED_SEAL_FILE_NAME,
                DBKEY_SEAL_FILE_NAME,
                WRAP_ARTIFACT_FILE_NAME,
            ] {
                std::fs::write(dir.path().join(name), b"dead prior seal")
                    .expect("stray prior seal");
            }
            let vault = TestVault::new(VaultTier::Tee);
            let seed = sealed_payload();
            let intent = sealed_intent(&seed);

            // Simulate a kill right after `kill`.
            let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
                if step == kill {
                    Err(WalletError::Io(std::io::Error::other("simulated kill")))
                } else {
                    Ok(())
                }
            };
            let first = provision_with(dir.path(), &vault, &intent, &mut hook);
            assert!(first.is_err(), "kill at {kill:?} must abort the call");
            if kill == ProvisionStep::IndexWritten {
                // The index names an id with no key yet (index-first, S16). With
                // no retry, an open finds no wallet — never a half-custodied one.
                match open_at(dir.path(), &vault, Network::Main) {
                    Err(WalletError::NotFound) => {}
                    Err(e) => panic!("an orphan index must open as NotFound, got {e:?}"),
                    Ok(_) => panic!("an orphan index must not open"),
                }
            }

            // Recover: the user retries create() with the SAME seed.
            if kill == ProvisionStep::Completed {
                // The marker was already durable before the hook fired — the
                // wallet is Complete, so the retry is the typed non-clobber and
                // the host opens it instead.
                assert_eq!(inspect(dir.path()).unwrap(), StoreState::Complete);
                assert!(matches!(
                    create_or_repair_at(dir.path(), &vault, &intent),
                    Err(WalletError::WalletAlreadyExists)
                ));
            } else {
                create_or_repair_at(dir.path(), &vault, &intent).expect("recovery completes");
            }
            assert_eq!(inspect(dir.path()).unwrap(), StoreState::Complete);

            // The wallet opens and the seed + DB key round-trip.
            let opened = open_at(dir.path(), &vault, Network::Main).expect("opens after recovery");
            let recovered = opened.seed.as_ref().expect("sealed seed recovered");
            assert_eq!(
                recovered.seed(),
                seed.seed(),
                "seed survived the kill at {kill:?}"
            );
            // open() also opened the DB under the recovered key (the sentinel is
            // verified inside open_db) — opened drops at the end of the iteration.
        }
    }

    /// §8 named test (ADR-0534 crash-safety) — the rescan headline gate. For EVERY
    /// rescan rebuild step, a kill there: (a) NEVER touches the seal artifacts (so the
    /// seed is always recoverable), (b) leaves the wallet OPENABLE (the OLD birthday
    /// before the rename, the NEW after — never bricked), and (c) a retry completes and
    /// preserves the durable aux money state. Mirrors the proven
    /// `provisioning_killed_at_any_step_recovers_idempotently` guarantee.
    #[test]
    fn rescan_is_crash_safe() {
        // Walked through an exhaustive `match`, like `every_provision_step()`: a
        // new `RescanStep` does not compile until it names its successor here.
        fn next(step: RescanStep) -> Option<RescanStep> {
            match step {
                RescanStep::TempProvisioned => Some(RescanStep::AuxCopied),
                RescanStep::AuxCopied => Some(RescanStep::Renamed),
                RescanStep::Renamed => None,
            }
        }
        let steps: Vec<RescanStep> =
            std::iter::successors(Some(RescanStep::TempProvisioned), |s| next(*s)).collect();
        assert_eq!(steps.len(), 3, "every step is walked: {steps:?}");
        for kill in steps {
            let dir = tempfile::tempdir().expect("tempdir");
            let vault = TestVault::new(VaultTier::Tee);
            let seed = sealed_payload();

            // A completed wallet with a durable aux row (a queued-send intent) to prove
            // the §6.3 money state survives the rebuild.
            create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
            let (_s, db_key, _st) =
                load_custody(dir.path(), &vault, PersistenceKind::SealedKeychain).expect("custody");
            let db_path = dir.path().join(WALLET_DB_FILE_NAME);
            {
                let mut conn = db::open_existing_keyed_connection(&db_path, &db_key).expect("aux");
                crate::intent_store::enqueue(&mut conn, "zcash:survivor", 0, None, None)
                    .expect("enqueue");
                // Also reserve a refund index (counter → 2): a rescan that lost it would
                // recycle index 1 — a privacy break (NOT just a torn money row).
                assert_eq!(
                    crate::refund_index::reserve_next_index(&mut conn).expect("reserve"),
                    1,
                    "first refund index is the floor"
                );
                // And a diversified receive index (Recv-4, review fold): the
                // never-reuse authority for a PUBLISHED contact address must ride
                // the rescan copy too — losing it would re-issue the base index.
                assert_eq!(
                    crate::diversified_index::reserve_next_index(&mut conn)
                        .expect("diversified reserve"),
                    crate::diversified_index::DIVERSIFIED_INDEX_BASE,
                    "first diversified index is the region base"
                );
            }
            let seal_before = std::fs::read(dir.path().join(SEED_SEAL_FILE_NAME)).expect("seal");
            let dbkey_before =
                std::fs::read(dir.path().join(DBKEY_SEAL_FILE_NAME)).expect("dbkey seal");
            let wrap_before =
                std::fs::read(dir.path().join(WRAP_ARTIFACT_FILE_NAME)).expect("wrap artifact");

            let lock = WalletLock::acquire(dir.path()).expect("lock");
            let mut hook = |step: RescanStep| -> Result<(), WalletError> {
                if step == kill {
                    Err(WalletError::Io(std::io::Error::other("simulated kill")))
                } else {
                    Ok(())
                }
            };
            assert!(
                reset_data_db_keep_seed_with(&lock, &db_key, Network::Main, &mut hook).is_err(),
                "a kill at {kill:?} must abort the rescan"
            );

            // (a) the seal artifacts are byte-identical — the seed was never at risk
            assert_eq!(
                std::fs::read(dir.path().join(SEED_SEAL_FILE_NAME)).expect("seal"),
                seal_before,
                "the seed seal must survive a killed rescan at {kill:?}"
            );
            assert_eq!(
                std::fs::read(dir.path().join(DBKEY_SEAL_FILE_NAME)).expect("dbkey"),
                dbkey_before,
                "the DB-key seal must survive a killed rescan at {kill:?}"
            );
            assert_eq!(
                std::fs::read(dir.path().join(WRAP_ARTIFACT_FILE_NAME)).expect("wrap"),
                wrap_before,
                "the wrap artifact must survive a killed rescan at {kill:?}"
            );

            // (b) the store still reads Complete and opens (old DB pre-rename, new DB post)
            assert_eq!(inspect(dir.path()).unwrap(), StoreState::Complete);
            drop(open(&lock, &vault, Network::Main).expect("opens after a killed rescan"));

            // (c) a retry completes, opens, and the durable aux row survived
            reset_data_db_keep_seed(&lock, &db_key, Network::Main).expect("retry completes");
            drop(open(&lock, &vault, Network::Main).expect("opens after the retry"));
            let mut conn =
                db::open_existing_keyed_connection(&db_path, &db_key).expect("verify open");
            assert_eq!(
                crate::intent_store::list_queued(&conn).expect("list").len(),
                1,
                "the durable aux row must survive the rescan (kill at {kill:?})"
            );
            assert_eq!(
                crate::refund_index::reserve_next_index(&mut conn).expect("reserve after retry"),
                2,
                "the refund counter survived the rescan — the next reserve is 2, never a \
                 recycled 1 (kill at {kill:?})"
            );
            assert_eq!(
                crate::diversified_index::reserve_next_index(&mut conn)
                    .expect("diversified reserve after retry"),
                crate::diversified_index::DIVERSIFIED_INDEX_BASE + 1,
                "the diversified counter survived the rescan — never a re-issued \
                 published contact address (kill at {kill:?})"
            );
        }
    }

    #[test]
    fn rescan_clears_a_stale_temp_db_before_rebuilding() {
        // §3.3 hardening: an interrupted prior rescan can leave a `wallet.db.rescan-tmp`
        // (+ sidecars) behind. It is inert (no marker, never opened), but the next rescan
        // MUST clear it first so a half-written temp can't pollute the fresh rebuild. Drop
        // garbage at the temp path + sidecars, then rescan, and assert it completes and the
        // temp is gone (consumed by the atomic rename).
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
        let (_s, db_key, _st) =
            load_custody(dir.path(), &vault, PersistenceKind::SealedKeychain).expect("custody");

        // A stale temp DB + every SQLite sidecar from a "crashed" prior rescan (the ONE
        // canonical suffix set, so this can't drift from what the cleanup actually sweeps).
        let tmp = dir.path().join(RESCAN_TMP_DB_FILE_NAME);
        for suffix in db::DB_FILE_SUFFIXES {
            let p = db::with_suffix(&tmp, suffix);
            std::fs::write(&p, b"garbage from an interrupted rescan").expect("write stale temp");
        }

        let lock = WalletLock::acquire(dir.path()).expect("lock");
        reset_data_db_keep_seed(&lock, &db_key, Network::Main).expect("rescan over a stale temp");
        assert!(
            !tmp.exists(),
            "the rescan must consume/clear the temp DB (renamed over wallet.db)"
        );
        drop(open(&lock, &vault, Network::Main).expect("opens after rescan over a stale temp"));
    }

    #[test]
    fn destroy_sweeps_the_advisory_lock_last_and_tolerates_vanishing_entries() {
        // §8 (#317 review F2 + F1): the ORDER is the race fix — the advisory lock
        // must be the FINAL unlink (so a concurrent open can never flock a fresh
        // inode while the destructive sweep still runs), and a mid-sweep vanished
        // entry (external cleanup racing the wipe) must neither abort the sweep
        // nor mask a partial one as complete. A revert to the whole-dir
        // `remove_dir_all` fails this test (it never drives the per-entry hook).
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
        let lock = WalletLock::acquire(dir.path()).expect("lock");

        let mut seen: Vec<String> = Vec::new();
        let mut vanished_one = false;
        let base = dir.path().to_path_buf();
        destroy_with(&lock, Some(&vault), false, &mut |name| {
            // Deterministic vanishing-entry injection: delete the FIRST announced
            // content entry out from under the sweep, right before it removes it.
            if !vanished_one && name.to_str() != Some(crate::constants::WALLET_LOCK_FILE_NAME) {
                vanished_one = true;
                let p = base.join(name);
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(&p);
                } else {
                    let _ = std::fs::remove_file(&p);
                }
            }
            seen.push(name.to_string_lossy().into_owned());
        })
        .expect("destroy");

        assert!(
            vanished_one,
            "the injection ran (a real store has content entries)"
        );
        assert!(
            !dir.path().exists(),
            "the sweep completed past the vanished entry — dir fully removed",
        );
        assert_eq!(
            seen.iter()
                .filter(|n| n.as_str() == crate::constants::WALLET_LOCK_FILE_NAME)
                .count(),
            1,
            "the lock is unlinked exactly once",
        );
        assert_eq!(
            seen.last().map(String::as_str),
            Some(crate::constants::WALLET_LOCK_FILE_NAME),
            "the advisory lock is the LAST unlink — the order IS the fix",
        );
    }

    #[test]
    fn rescan_disk_full_propagates_verbatim_not_folded_to_corruption() {
        // §6.1 hardening: this proves the rescan ORCHESTRATION propagates a typed `DiskFull`
        // ("free space and retry") VERBATIM — never folding it into `StoreCorrupt` (which
        // reads as unrecoverable) — and that the seals + old DB stay untouched (the wallet
        // re-opens). Inject `DiskFull` at a rebuild step via the fault hook to exercise the
        // pass-through. SCOPE NOTE: the copy/fsync/rename steps DO surface a real ENOSPC as
        // `DiskFull` (via `map_aux_err`/`from_io`); the step-1 `provision_db` path, however,
        // routes a real ENOSPC through the schemerz migrator, which wraps the underlying
        // `io::Error` opaquely → it still folds to `StoreCorrupt` there (a PRE-EXISTING
        // honest-degradation gap, db.rs `migrate`, NOT introduced by this rescan work). Both
        // outcomes are money-SAFE (old DB + seals intact, wallet re-opens); only the user
        // message differs. Tracked for the migrator-error-classification follow-up.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
        let (_s, db_key, _st) =
            load_custody(dir.path(), &vault, PersistenceKind::SealedKeychain).expect("custody");

        let lock = WalletLock::acquire(dir.path()).expect("lock");
        let mut hook = |step: RescanStep| -> Result<(), WalletError> {
            if step == RescanStep::AuxCopied {
                Err(WalletError::DiskFull)
            } else {
                Ok(())
            }
        };
        assert!(
            matches!(
                reset_data_db_keep_seed_with(&lock, &db_key, Network::Main, &mut hook),
                Err(WalletError::DiskFull)
            ),
            "a full disk during the rebuild must surface DiskFull, not StoreCorrupt"
        );
        // The old wallet is untouched — it still opens (pre-rename DB intact).
        drop(open(&lock, &vault, Network::Main).expect("opens after a DiskFull-aborted rescan"));
    }

    #[test]
    fn generate_repair_resumes_from_sealed_seed_never_regenerates() {
        // §6.3: a Generate-mode repair must use the ALREADY-SEALED seed, never a
        // fresh one. Provision past the remnant checkpoint, then repair with a
        // DIFFERENT seed but RepairMode::ResumeSealed → the sealed seed wins.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let original = sealed_payload();

        // crash after the remnant is durable (before the marker)
        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::RemnantPersisted {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        let _ = provision_with(dir.path(), &vault, &sealed_intent(&original), &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);

        // repair in Generate mode with a DIFFERENT supplied seed → ignored
        let different = SeedPayload::new(Zeroizing::new(vec![0x99; 64]), None).expect("payload");
        let intent = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(&different),
            repair_mode: RepairMode::ResumeSealed,
            freshly_generated_at: None,
        };
        create_or_repair_at(dir.path(), &vault, &intent).expect("generate repair resumes");

        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(
            opened.seed.expect("seed").seed(),
            original.seed(),
            "the SEALED seed must win — never the regenerated one"
        );
    }

    #[test]
    fn generate_create_stamps_creation_time_and_open_reads_it_back() {
        // §3.2f #356-F4: a Generate-create persists its creation stamp and a
        // (simulated-restart) open reads it back; a restore-shaped create
        // (VerifySupplied, `freshly_generated_at: None`) never stamps.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        let intent = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(&seed),
            repair_mode: RepairMode::ResumeSealed,
            freshly_generated_at: Some(1_751_000_000),
        };
        create_or_repair_at(dir.path(), &vault, &intent).expect("create");
        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(
            opened.created_at,
            Some(1_751_000_000),
            "the stamp round-trips create → (restart) → open"
        );

        let dir2 = tempfile::tempdir().expect("tempdir");
        create_or_repair_at(dir2.path(), &vault, &sealed_intent(&seed)).expect("restore create");
        let restored = open_at(dir2.path(), &vault, Network::Main).expect("opens");
        assert_eq!(
            restored.created_at, None,
            "a restore never stamps — its history may predate any stamp"
        );
    }

    #[test]
    fn repair_rerun_keeps_the_earliest_creation_stamp() {
        // §3.2f #356-F4 crash ordering: a kill AFTER the DB was provisioned
        // (stamp durable, and — #357 — a `Generated` manifest) followed by a
        // fresh-shaped repair carrying a LATER wall-clock must keep the ORIGINAL
        // stamp. #357's repair DOES re-stamp a `Generated` remnant, but
        // `record_once`'s `ON CONFLICT DO NOTHING` keeps the EARLIEST value, so
        // the durable attempt-1 stamp survives the later retry untouched.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        let intent_t1 = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(&seed),
            repair_mode: RepairMode::ResumeSealed,
            freshly_generated_at: Some(1_751_000_000),
        };
        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::DbProvisioned {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        let _ = provision_with(dir.path(), &vault, &intent_t1, &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);

        // The user's retry carries "now" (later); repair re-stamps the Generated
        // remnant but ON CONFLICT keeps the durable earlier stamp.
        let intent_t2 = ProvisionIntent {
            freshly_generated_at: Some(1_751_999_999),
            ..intent_t1
        };
        create_or_repair_at(dir.path(), &vault, &intent_t2).expect("repair resumes");
        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(
            opened.created_at,
            Some(1_751_000_000),
            "repair keeps the earliest stamp, never creeps it forward"
        );
    }

    #[test]
    fn repair_stamps_a_generated_remnant_killed_early() {
        // #357: a FRESH create killed BEFORE the DB step (no stamp on disk yet)
        // leaves a `Generated` v2 manifest (the manifest is written at step 2,
        // before this kill). A fresh-shaped repair may now RECOVER the stamp from
        // that provably-generated remnant — pre-#357 this degraded to the
        // stamp-less belt arms. (Money-safe: the stamp is a strictly downward
        // birthday input; see the repair-site comment.)
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        let intent = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(&seed),
            repair_mode: RepairMode::ResumeSealed,
            freshly_generated_at: Some(1_751_000_000),
        };
        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::RemnantPersisted {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        let _ = provision_with(dir.path(), &vault, &intent, &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);
        assert_eq!(
            read_manifest(dir.path()).expect("manifest").seed_source,
            ManifestSeedSource::Generated,
            "the killed-early fresh remnant recorded Generated provenance",
        );

        create_or_repair_at(dir.path(), &vault, &intent).expect("repair resumes");
        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(
            opened.created_at,
            Some(1_751_000_000),
            "a Generated remnant IS stamped by a fresh-shaped repair (#357)",
        );
    }

    #[test]
    fn a_fresh_shaped_repair_never_stamps_a_restored_remnant() {
        // The protection #357 PRESERVES: a remnant whose v2 manifest records
        // `Restored` (a kill before the DB step of a RESTORE create) is NEVER
        // stamped — not even by a later FRESH-shaped retry — because its sealed
        // seed may carry pre-stamp history. Only `Generated` unlocks the stamp.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::RemnantPersisted {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        // A RESTORE create (freshly_generated_at: None ⇒ Restored manifest).
        let _ = provision_with(dir.path(), &vault, &sealed_intent(&seed), &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);
        assert_eq!(
            read_manifest(dir.path()).expect("manifest").seed_source,
            ManifestSeedSource::Restored,
        );

        // A FRESH-shaped retry must STILL NOT stamp the Restored remnant.
        let fresh_retry = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(&seed),
            repair_mode: RepairMode::VerifySupplied,
            freshly_generated_at: Some(1_751_000_000),
        };
        create_or_repair_at(dir.path(), &vault, &fresh_retry).expect("repair resumes");
        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(
            opened.created_at, None,
            "a Restored-origin remnant is never stamped, even by a fresh-shaped retry (S176)",
        );
    }

    #[test]
    fn a_restore_shaped_repair_over_a_generated_remnant_unstamps() {
        // The crypto audit MED-1 DOWNGRADE UN-STAMP: a FRESH create killed
        // AFTER the DB step has a durable stamp AND a `Generated` manifest; the
        // documented recovery for a doubted attestation — retry as a plain RESTORE
        // (freshly_generated_at: None) — must CLEAR that stamp, so the wallet takes
        // the activation floor + the #387 restore-pessimistic sweep, never the
        // aborted fresh attempt's ~creation scan floor.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        let fresh = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(&seed),
            repair_mode: RepairMode::ResumeSealed,
            freshly_generated_at: Some(1_751_000_000),
        };
        // Kill AFTER the DB step: the stamp is durable, the manifest is Generated.
        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::DbProvisioned {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        let _ = provision_with(dir.path(), &vault, &fresh, &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);

        // The restore-shaped retry (sealed_intent: freshly_generated_at None) fires
        // the (None, _) downgrade arm and CLEARS the durable stamp.
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("restore repair");
        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(
            opened.created_at, None,
            "a restore-shaped retry un-stamps the aborted fresh attempt (S213)",
        );
    }

    #[test]
    fn wrong_mnemonic_repair_is_seed_mismatch_not_clobber() {
        // §6.3: a mistyped-mnemonic re-run against a remnant is typed SeedMismatch
        // (constant-time compare) — never silently re-provisions over the remnant.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let original = sealed_payload();

        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::RemnantPersisted {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        let _ = provision_with(dir.path(), &vault, &sealed_intent(&original), &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);

        let wrong = SeedPayload::new(Zeroizing::new(vec![0xEE; 64]), None).expect("payload");
        let intent = sealed_intent(&wrong); // VerifySupplied + wrong seed
        assert!(matches!(
            create_or_repair_at(dir.path(), &vault, &intent),
            Err(WalletError::SeedMismatch)
        ));
        // the remnant is untouched: the correct seed still repairs it
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&original))
            .expect("correct repairs");
        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(opened.seed.expect("seed").seed(), original.seed());
    }

    #[test]
    fn create_on_existing_complete_is_typed_not_clobber() {
        // §6.3 "never clobbers": create against a completed wallet is typed
        // WalletAlreadyExists, and the existing wallet is untouched.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("first create");

        assert!(matches!(
            create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)),
            Err(WalletError::WalletAlreadyExists)
        ));
        // untouched: still opens with the original seed
        let opened = open_at(dir.path(), &vault, Network::Main).expect("opens");
        assert_eq!(opened.seed.expect("seed").seed(), seed.seed());
    }

    #[test]
    fn keystore_inconsistency_surfaced_not_fresh_wallet() {
        // §6.1/§8: the wallet DB is present but the keychain item is gone (a
        // different/empty vault) → typed KeystoreInconsistent, NEVER a silent
        // fresh wallet. (The vault returns KeystoreInconsistent for an absent
        // alias generation — keychain/mod.rs.)
        let dir = tempfile::tempdir().expect("tempdir");
        let vault_a = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault_a, &sealed_intent(&seed)).expect("provision");

        // open with a FRESH vault that never custodied this wrap key
        let vault_b = TestVault::new(VaultTier::Tee);
        assert!(matches!(
            open_at(dir.path(), &vault_b, Network::Main),
            Err(WalletError::KeystoreInconsistent { .. })
        ));
        // the on-disk store still reads as Complete (not silently downgraded)
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Complete);
    }

    #[test]
    fn open_on_empty_and_remnant_are_typed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        // empty
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::NotFound)
        ));
        // remnant (kill before the marker)
        let seed = sealed_payload();
        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::DbProvisioned {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        let _ = provision_with(dir.path(), &vault, &sealed_intent(&seed), &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::ProvisioningIncomplete)
        ));
    }

    #[test]
    fn network_mismatch_on_open_is_typed() {
        // §8 network_mismatch_rejected_everywhere (DB-open arm): a wallet
        // provisioned on mainnet, opened with a testnet config, is typed.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision main");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Test),
            Err(WalletError::NetworkMismatch)
        ));
        // the matching network still opens
        open_at(dir.path(), &vault, Network::Main).expect("main opens");
    }

    #[test]
    fn none_persistence_provisions_and_opens_without_seed() {
        // The host-supplied-seed path: nothing sealed but the DB key; open recovers no seed
        // and the DB opens under the recovered key.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let intent = ProvisionIntent {
            network: Network::Test,
            persistence: PersistenceKind::None,
            seed: None,
            repair_mode: RepairMode::NoSeed,
            freshly_generated_at: None,
        };
        create_or_repair_at(dir.path(), &vault, &intent).expect("provision none-mode");
        let opened = open_at(dir.path(), &vault, Network::Test).expect("opens");
        assert!(opened.seed.is_none(), "None mode never persists a seed");
        assert_eq!(opened.persistence, PersistenceKind::None);
    }

    #[test]
    fn corrupt_manifest_is_typed_reject() {
        // validate-before-use: a wrong-length / wrong-version / bad-enum manifest
        // is StoreCorrupt, never a guess.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");

        // overwrite the manifest with a bad version byte (unknown version — never guessed)
        write_atomic(dir.path(), MANIFEST_FILE_NAME, &[0xFF, 0, 0, 1]).expect("write");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::StoreCorrupt)
        ));
        // wrong length for the DECLARED version: a v2 manifest demands exactly
        // MANIFEST_LEN_V2 (a 3-byte body under the v2 version byte is corruption,
        // NOT silently read as v1).
        write_atomic(
            dir.path(),
            MANIFEST_FILE_NAME,
            &[PROVISION_MANIFEST_VERSION, 0, 0],
        )
        .expect("write");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::StoreCorrupt)
        ));
        // unknown network byte (the `_ => StoreCorrupt` arm in read_manifest)
        write_atomic(
            dir.path(),
            MANIFEST_FILE_NAME,
            &[PROVISION_MANIFEST_VERSION, 0xFF, 0, 1],
        )
        .expect("write");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::StoreCorrupt)
        ));
        // unknown persistence byte (PersistenceKind::from_byte reject)
        write_atomic(
            dir.path(),
            MANIFEST_FILE_NAME,
            &[PROVISION_MANIFEST_VERSION, 0, 0xFF, 1],
        )
        .expect("write");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::StoreCorrupt)
        ));
        // #357: unknown seed-source byte in a v2 manifest (ManifestSeedSource::from_byte reject)
        write_atomic(
            dir.path(),
            MANIFEST_FILE_NAME,
            &[PROVISION_MANIFEST_VERSION, 0, 0, 0xFF],
        )
        .expect("write");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::StoreCorrupt)
        ));
        // #397: the WATCH-ONLY bytes inside a v2 manifest are corruption — a
        // spending manifest can never half-claim the seed-less kind (persist
        // byte 2 / seed-source byte 3 exist only under version 0x03).
        write_atomic(
            dir.path(),
            MANIFEST_FILE_NAME,
            &[PROVISION_MANIFEST_VERSION, 0, 2, 3],
        )
        .expect("write");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::StoreCorrupt)
        ));
        // #397: a v3 manifest with anything but the EXACT watch-only byte pair
        // is corruption — no mixed states (a v3 can never claim seed custody).
        for bad in [[0x03u8, 0, 0, 3], [0x03, 0, 2, 1], [0x03, 0, 0, 1]] {
            write_atomic(dir.path(), MANIFEST_FILE_NAME, &bad).expect("write");
            assert!(matches!(
                open_at(dir.path(), &vault, Network::Main),
                Err(WalletError::StoreCorrupt)
            ));
        }
    }

    #[test]
    fn a_v3_watch_only_manifest_round_trips_and_opens_seedless() {
        // #397 (spec §3.7 D4): a watch-only provision writes the v3 manifest
        // ([0x03][net][2][3]); open reads it back as the WatchOnly kind with
        // NO seed blob expected — and the version byte is the capability gate
        // (`corrupt_manifest_is_typed_reject` pins the reject arms around it).
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let intent = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::WatchOnly,
            seed: None,
            repair_mode: RepairMode::NoSeed,
            freshly_generated_at: None,
        };
        create_or_repair_at(dir.path(), &vault, &intent).expect("provision watch-only");
        let raw = std::fs::read(dir.path().join(MANIFEST_FILE_NAME)).expect("manifest bytes");
        assert_eq!(
            raw,
            vec![crate::constants::MANIFEST_VERSION_V3, 0, 2, 3],
            "the exact v3 watch-only byte pair"
        );
        let opened = open_at(dir.path(), &vault, Network::Main).expect("open watch-only");
        assert!(opened.seed.is_none(), "watch-only never persists a seed");
        assert_eq!(opened.persistence, PersistenceKind::WatchOnly);
        // Network mismatch still enforced ahead of custody (same as spending).
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Test),
            Err(WalletError::NetworkMismatch)
        ));
    }

    #[test]
    fn a_v1_manifest_still_opens_with_unknown_seed_source() {
        // #357 back-compat: a pre-#357 v1 manifest (3 bytes, NO seed-source byte)
        // still opens; its provenance reads as `Unknown` — the conservative
        // no-stamp arm, byte-identical to pre-#357.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
        // A valid v1 manifest: [ver=1][network=Main(0)][persistence=SealedKeychain(0)].
        write_atomic(dir.path(), MANIFEST_FILE_NAME, &[MANIFEST_VERSION_V1, 0, 0]).expect("write");
        open_at(dir.path(), &vault, Network::Main).expect("a valid v1 manifest opens");
        assert_eq!(
            read_manifest(dir.path()).expect("read v1").seed_source,
            ManifestSeedSource::Unknown
        );
    }

    #[test]
    fn a_v2_manifest_round_trips_the_seed_source() {
        // #357: provisioning writes a v2 manifest; `sealed_intent` is a restore
        // (`freshly_generated_at: None`), so its provenance round-trips as
        // `Restored`.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
        let m = read_manifest(dir.path()).expect("read v2");
        assert_eq!(m.seed_source, ManifestSeedSource::Restored);
        assert_eq!(m.persistence, PersistenceKind::SealedKeychain);

        // The `Generated` byte round-trips too (encode `from_intent`→to_byte=1,
        // decode from_byte=1) — a fresh-shaped provision.
        let dir2 = tempfile::tempdir().expect("tempdir");
        let fresh = ProvisionIntent {
            network: Network::Main,
            persistence: PersistenceKind::SealedKeychain,
            seed: Some(&seed),
            repair_mode: RepairMode::ResumeSealed,
            freshly_generated_at: Some(1_751_000_000),
        };
        create_or_repair_at(dir2.path(), &vault, &fresh).expect("fresh provision");
        assert_eq!(
            read_manifest(dir2.path()).expect("read v2").seed_source,
            ManifestSeedSource::Generated,
        );
    }

    #[test]
    fn marker_without_manifest_is_corrupt_not_complete() {
        // inspect's fourth outcome: a completion marker with NO manifest is
        // corruption (fail-closed), never a usable Complete wallet.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
        std::fs::remove_file(dir.path().join(MANIFEST_FILE_NAME)).expect("rm manifest");
        assert!(matches!(
            inspect(dir.path()),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn oversized_seal_file_rejected_before_alloc() {
        // §4.6 cap-before-alloc, tested at the boundary: a seal artifact larger
        // than SEAL_FILE_MAX_BYTES is StoreCorrupt before any read allocates it.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let seed = sealed_payload();
        create_or_repair_at(dir.path(), &vault, &sealed_intent(&seed)).expect("provision");
        // overwrite a real seal artifact with one byte over the cap
        write_atomic(
            dir.path(),
            SEED_SEAL_FILE_NAME,
            &vec![0u8; SEAL_FILE_MAX_BYTES + 1],
        )
        .expect("write oversized");
        assert!(matches!(
            open_at(dir.path(), &vault, Network::Main),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn none_persistence_recovers_from_remnant_kill() {
        // None-mode crash recovery: a kill after the manifest is durable repairs
        // from the remnant (no seed to verify) and opens seedless.
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = TestVault::new(VaultTier::Tee);
        let intent = ProvisionIntent {
            network: Network::Test,
            persistence: PersistenceKind::None,
            seed: None,
            repair_mode: RepairMode::NoSeed,
            freshly_generated_at: None,
        };
        let mut hook = |step: ProvisionStep| -> Result<(), WalletError> {
            if step == ProvisionStep::RemnantPersisted {
                Err(WalletError::Io(std::io::Error::other("kill")))
            } else {
                Ok(())
            }
        };
        let _ = provision_with(dir.path(), &vault, &intent, &mut hook);
        assert_eq!(inspect(dir.path()).unwrap(), StoreState::Remnant);
        create_or_repair_at(dir.path(), &vault, &intent).expect("none-mode repair");
        let opened = open_at(dir.path(), &vault, Network::Test).expect("opens");
        assert!(opened.seed.is_none());
        assert_eq!(opened.persistence, PersistenceKind::None);
    }
}
