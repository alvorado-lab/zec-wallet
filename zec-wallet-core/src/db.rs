//! §4.3 — open the wallet DB as a SQLCipher-encrypted rusqlite connection.
//!
//! We open the connection ourselves and apply `PRAGMA key` as the FIRST
//! statement (SQLCipher contract + rust-patterns), then force header decryption
//! so a wrong key / corruption fails HERE — typed — rather than lazily mid-
//! operation (SQLCipher validates the key on first access, not at PRAGMA time).
//! Then we register the rarray vtab module (`load_module`) that
//! `zcash_client_sqlite` requires, wrap the keyed connection in
//! `WalletDb::from_connection`, and run `init_wallet_db` to create / migrate the
//! schema (W3-inc-2b-ii-A — [`provision_db`]).
//!
//! Failure posture (§6.1): a failed open/decrypt is `StoreCorrupt` — fail-closed,
//! surfaced, NEVER auto-wiped. (Finer IO classification — `DiskFull` — for the DB
//! path is owed with the lifecycle chunk: rusqlite errors don't surface the
//! underlying `std::io::Error`, so the single `WalletError::from_io` door can't
//! reach this path yet.)
//!
//! **Key residue (security review W3-inc-2a):** the PRAGMA string lives in a
//! `Zeroizing<String>` for the one `execute_batch` call, but SQLCipher copies the
//! key into its own C page-context for the connection's lifetime — that copy is
//! OUTSIDE Rust zeroization (OS memory-protection territory). The zeroizing
//! guarantee holds up to the FFI boundary, no further.
//!
//! **Provision vs open (§6.3 create-vs-open guard — BUILT W3-inc-2b-i).** Three
//! cooperating layers stop a truncated / 0-byte / missing `wallet.db` from
//! masquerading as a fresh wallet (the upstream `keysMissing` hazard):
//!
//!  1. **The §6.3 completion marker** (store.rs, the source of truth): no marker
//!     ⇒ never open-as-fresh — the store refuses (`ProvisioningIncomplete` /
//!     `NotFound`). This gates the call BEFORE [`open_db`] runs.
//!  2. **No `SQLITE_OPEN_CREATE` on open** ([`open_existing_keyed_connection`]):
//!     a MISSING file yields `SQLITE_CANTOPEN` → typed `NotFound`, never a
//!     silently-created empty DB.
//!  3. **The provisioning sentinel** ([`provision_db`] writes a
//!     `wallet_provisioning` row; [`open_db`] verifies it): SQLite treats a
//!     0-byte file as a *fresh empty DB* (open + key + probe all SUCCEED), so
//!     no-CREATE alone cannot catch a marker-present-but-truncated DB — the
//!     MISSING sentinel does, failing typed `StoreCorrupt`.
//!
//! The provision path ([`provision_db`]) is idempotent (`init_wallet_db`'s
//! schemerz migrator + the `CREATE TABLE IF NOT EXISTS` sentinel both no-op on a
//! re-run — repair relies on this). The zcash schema and our sentinel are
//! ORTHOGONAL: `init_wallet_db` owns `schemerz_migrations` + its own
//! `user_version`; the sentinel is a separate single-row table that exists ONLY
//! to catch a marker-present-but-truncated DB (the create-vs-open guard's layer
//! 3), since a 0-byte file opens as a fresh empty DB.
//!
//! **`init_wallet_db` seed retry (`None` → `Some(seed)`) — AS BUILT W3-inc-2c-iii.**
//! [`provision_db`] passes `seed = None` (a fresh, account-less wallet has no
//! derived account for the migrator to validate a seed against, so every migration
//! applies cleanly). [`open_db_migrated`] threads the loaded custody seed into
//! [`migrate`], which tries `None` first and, on `SeedRequired` (a post-account
//! seed-validating migration), retries with `Some(seed)` when available — else
//! surfaces the typed `SeedRequired` for the host to re-supply (§4.2 / §3.2b).
//! Account creation itself (`import_account_hd` over an `AccountBirthday`) lives in
//! account.rs (§3.2b); the engine drives it (inc-2c-iv).

use std::path::{Path, PathBuf};
use std::time::Duration;

use rand_core::OsRng;
use rusqlite::{Connection, OpenFlags};
use secrecy::SecretVec;
use zcash_client_sqlite::WalletDb;
use zcash_client_sqlite::error::SqliteClientError;
use zcash_client_sqlite::util::SystemClock;
use zcash_client_sqlite::wallet::init::{WalletMigrationError, init_wallet_db};

use crate::constants::{SQLITE_BUSY_TIMEOUT_MS, WALLET_SCHEMA_VERSION};
use crate::error::WalletError;
use crate::money::Network;
use crate::seal::WalletDbKey;

/// The wallet's `zcash_client_sqlite` data handle: OUR SQLCipher-keyed
/// `rusqlite::Connection`, wrapped with the runtime-switchable consensus
/// network, the system clock, and `OsRng` (the upstream API's capabilities —
/// none mint OUR key material; the DB key is sealed in seal.rs). This is a
/// librustzcash type and is `pub(crate)` by contract — it NEVER appears in the
/// public Dart/Rust API (ADR-0005 r5: upstream nu-churn is absorbed inside the
/// core). The async `Wallet` handle holds one of these (behind its lock +
/// lifecycle) for the life of an open wallet; the sync/send engines (inc-2c/2d)
/// drive it inside `spawn_blocking`.
pub(crate) type WalletConn = WalletConnFor<zcash_protocol::consensus::Network>;

/// [`WalletConn`] with the consensus parameters left open — the ONE shape a
/// money-path write may be generic in. Connection, clock and RNG stay concrete:
/// `sync::put_subtree_roots` runs over the live `Network` wallet and over the
/// funded `LocalNetwork` harness (`ironwood_spendability`), and nothing else about
/// the handle differs between them. Named so the widening is spelled once here
/// rather than as an inline `WalletDb<Connection, P, SystemClock, OsRng>` at each
/// site (arch review, §4ac row 9).
pub(crate) type WalletConnFor<P> = WalletDb<Connection, P, SystemClock, OsRng>;

/// Is the SQLite this connection runs SQLCipher? `PRAGMA cipher_version` answers
/// its version on SQLCipher and NO row on plain SQLite, where `PRAGMA key` is an
/// unknown pragma that silently does nothing — the wallet would read and write
/// its database in PLAINTEXT. A host whose link binds `sqlite3_*` to a plain
/// SQLite (FR-5 D-24: an Apple `:linkage => :static` host that also links the
/// platform sqlite, which the Tor plugin does) must fail here, closed.
fn is_sqlcipher(cipher_version: Option<&str>) -> bool {
    cipher_version.is_some_and(|v| !v.trim().is_empty())
}

/// Refuse a connection that is not SQLCipher ([`is_sqlcipher`]), or whose key is
/// not attached, right after `PRAGMA key` and before anything reads or writes.
/// `cipher_provider` answers only on a keyed connection (measured the
/// provider name keyed, no row unkeyed), so it proves THIS connection's key took.
/// `VaultAbsent`: permanent for this build, fail-closed, and never `StoreCorrupt`
/// (whose next step is a restore the user must not be sent to — nothing is
/// damaged).
fn ensure_sqlcipher(conn: &Connection) -> Result<(), WalletError> {
    let answer = |pragma: &str| -> Result<Option<String>, WalletError> {
        match conn.query_row(pragma, [], |r| r.get(0)) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(classify_sqlite_error(&e)),
        }
    };
    let version = answer("PRAGMA cipher_version")?;
    let provider = answer("PRAGMA cipher_provider")?;
    if is_sqlcipher(version.as_deref()) && is_sqlcipher(provider.as_deref()) {
        Ok(())
    } else {
        tracing::error!(target: "zec_wallet_core", "wallet.db_not_sqlcipher");
        Err(WalletError::VaultAbsent)
    }
}

/// Open `path` as a SQLCipher connection keyed by `db_key`. On a fresh path this
/// CREATES an encrypted DB under the key; on an existing one a wrong key fails
/// (the header won't authenticate). The returned connection has the rarray vtab
/// registered, ready for `WalletDb::from_connection`.
///
/// **Synchronous + blocking** (security review + mobile W3-inc-2a): the file open,
/// SQLCipher key setup, and the probe query are blocking syscalls. The inc-2b async
/// wallet handle MUST call this inside `tokio::task::spawn_blocking` — off the Dart/
/// UI thread AND off the async runtime (rust-patterns). This fn owns no runtime by
/// design. See the module doc for the provision-vs-open (create-flags) contract.
pub(crate) fn open_keyed_connection(
    path: &Path,
    db_key: &WalletDbKey,
) -> Result<Connection, WalletError> {
    let conn = Connection::open(path).map_err(map_aux_err)?;
    // PRAGMA key FIRST — before ANY other statement touches the DB. The
    // statement (key included) lives in a zeroizing buffer for this call only.
    conn.execute_batch(db_key.pragma_key_statement().as_str())
        .map_err(map_aux_err)?;
    // The key only protects anything on SQLCipher (D-24): refuse plain SQLite.
    ensure_sqlcipher(&conn)?;
    // Wait out a busy lock BEFORE the probe (R12 §4.1): the wallet DB has TWO
    // connections (engine + aux intent store, inc-2d-3-b-i), and a checkpoint in
    // flight would otherwise fail the probe at once (§ SQLITE_BUSY_TIMEOUT_MS).
    conn.busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
        .map_err(map_aux_err)?;
    // Force header decryption now: SQLCipher validates the key on first access,
    // so a wrong key / corrupt header surfaces HERE as a typed error, never as a
    // silently half-open handle.
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(map_aux_err)?;
    // The rarray (carray) vtab module zcash_client_sqlite requires (documented).
    rusqlite::vtab::array::load_module(&conn).map_err(map_aux_err)?;
    // WAL, verified (W-swap-4-a-4) — see `ensure_wal` for why this is load-bearing.
    ensure_wal(&conn)?;
    // Durability pinned, not assumed (W-swap-4-a-5) — see `ensure_synchronous_full`.
    ensure_synchronous_full(&conn)?;
    // No plaintext temp spill OUTSIDE the encrypted DB (§4.3 at-rest + FR-14 wipe
    // completeness): force temp B-trees / index builds into RAM so a sync or
    // migration never lands a cleartext temp file in the system temp dir — which
    // `remove_dir_all(db_dir)` (the wipe's file sweep) could not reach, and which
    // SQLCipher does not guarantee to encrypt. RAM tradeoff: bounded by the wallet's
    // own data size (notes + tx rows, thousands not millions for any real ZEC
    // wallet — no large sorts/joins), so this is safe on a low-memory mobile device;
    // if a future heavy path changes that, scope this off the hot path or point
    // SQLite's temp dir INTO db_dir (so the spill is swept) instead of dropping it.
    conn.execute_batch("PRAGMA temp_store = MEMORY;")
        .map_err(map_aux_err)?;
    Ok(conn)
}

/// Open an EXISTING keyed connection WITHOUT creating it (§6.3 layer 2). The
/// `OpenFlags` deliberately OMIT `SQLITE_OPEN_CREATE`, so a missing path is
/// `SQLITE_CANTOPEN` → typed `NotFound` (never a silently-created empty DB);
/// any other open failure, or a wrong-key / corrupt header (caught by the
/// forced probe), is `StoreCorrupt` — fail-closed, never auto-wiped.
///
/// **Synchronous + blocking** (same contract as [`open_keyed_connection`]): the
/// inc-2b-ii async handle calls this inside `spawn_blocking`.
pub(crate) fn open_existing_keyed_connection(
    path: &Path,
    db_key: &WalletDbKey,
) -> Result<Connection, WalletError> {
    open_existing_keyed_with_flags(path, db_key, OpenFlags::empty())
}

/// [`open_existing_keyed_connection`] with EXTRA open flags OR'd onto the base
/// (no-CREATE, no-MUTEX) set. The rescan aux copy passes `SQLITE_OPEN_URI` so the
/// source can be ATTACHed read-only via a `file:…?mode=ro` URI; `path` (the main DB)
/// is a plain absolute path that never begins with `file:`, so URI processing leaves
/// it interpreted literally (SQLite only URI-parses names that start with `file:`).
fn open_existing_keyed_with_flags(
    path: &Path,
    db_key: &WalletDbKey,
    extra: OpenFlags,
) -> Result<Connection, WalletError> {
    // NO_MUTEX matches the default threading mode (the lifecycle.rs single-writer
    // lock is our concurrency contract); the load-bearing omission is CREATE.
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX | extra;
    let conn = Connection::open_with_flags(path, flags).map_err(map_open_err)?;
    // PRAGMA key FIRST, then force header decryption so a wrong key / corrupt
    // header fails HERE, typed (SQLCipher validates lazily on first access).
    conn.execute_batch(db_key.pragma_key_statement().as_str())
        .map_err(map_aux_err)?;
    ensure_sqlcipher(&conn)?;
    conn.busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
        .map_err(map_aux_err)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(map_aux_err)?;
    rusqlite::vtab::array::load_module(&conn).map_err(map_aux_err)?;
    // WAL, verified (W-swap-4-a-4) — see `ensure_wal`.
    ensure_wal(&conn)?;
    // Durability pinned, not assumed (W-swap-4-a-5) — see `ensure_synchronous_full`.
    ensure_synchronous_full(&conn)?;
    // No plaintext temp spill outside the encrypted DB (§4.3 / FR-14) — see
    // `open_keyed_connection`.
    conn.execute_batch("PRAGMA temp_store = MEMORY;")
        .map_err(map_aux_err)?;
    Ok(conn)
}

/// Put the connection's database in WAL journal mode and VERIFY the echo
/// (W-swap-4-a-4). Load-bearing, not a tuning preference: the wallet file
/// carries TWO connections (the engine + the aux store), and in the
/// rollback-journal era ANY engine read window longer than the 5 s
/// `busy_timeout` failed a concurrent aux COMMIT `SQLITE_BUSY` (the
/// device blocker — a single engine `progress_snapshot` measured ~6.5 s, so
/// swap executes died mid-sync). Under WAL readers never block the writer,
/// which retires that whole class; only writer-vs-writer contention remains
/// (carried by `StoreBusy` + [`with_aux_busy_retry`]).
///
/// The pragma is PERSISTENT (stored in the DB header) and idempotent — the
/// first opener flips a legacy rollback-mode wallet, every later open just
/// re-confirms. The echo is checked (`wal`, case-insensitively) and any other
/// mode is fail-closed `StoreCorrupt`: silently running rollback would
/// re-arm the mislabeled-BUSY class without any signal. Sidecar/at-rest
/// posture is unchanged: SQLCipher encrypts WAL frames, `-shm` holds only
/// frame-index metadata (no note/tx plaintext), and every delete path already
/// sweeps `-wal`/`-shm` ([`DB_FILE_SUFFIXES`]). Durability is unchanged too:
/// `synchronous` stays at its FULL default (money data takes no NORMAL
/// downgrade). The rescan aux copy's read-only ATTACH stays sound — its
/// source is only ever a cleanly-quiesced close, which checkpoints and
/// removes the WAL.
///
/// **REKEY SEAM (security MED-1, W-swap-4-a-5).** `PRAGMA rekey` is
/// UNSAFE under a live WAL. Mechanism (vendored `sqlite3_rekey_v2`, corrected
/// by the crypto review): the rekey rewrites every page THROUGH the
/// pager, landing NEW-key frames in the WAL while the MAIN FILE keeps its
/// OLD-key pages until checkpoint — a reader-gated PARTIAL checkpoint can
/// then leave the main file mixed-key the moment anything treats it as the
/// whole database (crash + WAL loss, a rename-shaped step, a main-only
/// backup). The future rekey slice (the `db_key` carriers: wallet.rs
/// `WalletDb`, store.rs `OpenWallet`) MUST bracket the rotation with
/// [`checkpoint_wal_truncate`] + a journal flip (or use `sqlcipher_export`
/// into a fresh keyed DB), and land a gate test proving no `-wal` survives
/// the rotation. Two more pins for that slice: QUIESCE TO ONE CONNECTION
/// first (this file carries two — the sibling's codec keeps the OLD key and
/// would HMAC-fail every read into a `StoreCorrupt` storm), and on a FAILED
/// rekey CLOSE the connection and never write on it again (SQLCipher flips
/// its write context to the new key up front and restores on success only —
/// after a failure, writes self-corrupt).
///
/// **DESKTOP VFS (reliability F4, #371/§9).** A VFS that cannot run WAL
/// (no shared-memory support — e.g. a network mount) keeps echoing the old
/// mode, which the `delete` arm below types `StoreBusy` — a PERMANENT
/// condition wearing a transient's code. Correct for shipping mobile (WAL
/// always works on local storage, and a network `db_dir` was never supported);
/// the desktop GA pass must type the permanent case distinctly.
fn ensure_wal(conn: &Connection) -> Result<(), WalletError> {
    // A pragma ERROR routes through the aux taxonomy (BUSY → the retryable
    // `StoreBusy`, never corruption — review fold); an echo of the OLD mode
    // (`delete`) without an error is SQLite's "could not take the flip lock"
    // signal, the same transient, so it maps `StoreBusy` too. Both are
    // production-unreachable today (the flock + strictly-sequential opens mean no
    // sibling connection exists when a legacy file flips), but mislabeling a busy
    // as corruption is exactly the sin this batch retires — don't resurrect it at
    // the open door. Any OTHER echo (a store that cannot run WAL at all) stays
    // fail-closed `StoreCorrupt`: silently running rollback would re-arm the
    // mislabeled-BUSY class with no signal.
    let mode: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))
        .map_err(map_aux_err)?;
    if mode.eq_ignore_ascii_case("wal") {
        Ok(())
    } else if mode.eq_ignore_ascii_case("delete") {
        Err(WalletError::StoreBusy)
    } else {
        Err(WalletError::StoreCorrupt)
    }
}

/// Turn on `secure_delete` for the AUX connection and VERIFY it took
/// (ADR-0568, crypto audit B1). Since ADR-0568 the aux DB holds a user's
/// custom-server key in the `sync_server` choice row; a replaced or erased key
/// must not survive in a freed page. MEASURED at the build: SQLCipher already
/// turns `secure_delete` on for EVERY keyed handle (`sqlcipherCodecAttach`
/// calls `sqlite3BtreeSecureDelete(pBt, 1)`, `libsqlite3-sys-0.35.0`
/// `sqlcipher/sqlite3.c:109845`), so the engine's connection has it too. This
/// is the EXPLICIT, VERIFIED form on the handle that holds the key
/// (`store.rs`, right after the aux opens): a link that ever lost that
/// default — a plain SQLite under the same symbols — fails closed here
/// rather than keeping erased keys silently. `ON` (1), never `FAST`: the row
/// can outgrow a page (URL + bound URL + value), and FAST promises to zero a
/// freed page only when that costs no extra I/O. Measured: under WAL it
/// zeroed our freed overflow page anyway, so ON is the conservative choice
/// that holds whatever the journal mode or freelist shape — not one a test
/// here proves necessary (see `the_aux_connection_zeroes_an_erased_key`).
/// Pages are SQLCipher ciphertext on
/// disk either way — this zeroes the plaintext they would decrypt to. The WAL
/// is the switch's job (`checkpoint_wal_truncate` after the write).
pub(crate) fn ensure_secure_delete(conn: &Connection) -> Result<(), WalletError> {
    let on: i64 = conn
        .query_row("PRAGMA secure_delete = 1", [], |r| r.get(0))
        .map_err(map_aux_err)?;
    if on == 1 {
        Ok(())
    } else {
        // A build of SQLite that cannot honour it would silently keep erased
        // keys; fail closed rather than claim an erasure that is not done —
        // but as what it is (a store capability), never `StoreCorrupt`'s
        // restore-from-phrase scare over a healthy wallet (crypto audit).
        Err(WalletError::Io(std::io::Error::other(
            "the store cannot zero erased pages (secure_delete unsupported)",
        )))
    }
}

/// Pin `synchronous = FULL` and VERIFY the echo (W-swap-4-a-5 — the 4-a-4
/// review ASSUMED the FULL default). Money durability must not ride a
/// compile-time default: a host-linked SQLite built with
/// `SQLITE_DEFAULT_WAL_SYNCHRONOUS = NORMAL` (common in distro/system builds)
/// would silently downgrade every wallet.db commit under WAL to a
/// power-cut-lossy fsync policy. The set-form pragma returns no row, so the
/// echo is read back explicitly; anything but FULL (`2`) is fail-closed
/// `StoreCorrupt` (a store that cannot honor the durability contract must not
/// open). Faults route via [`map_aux_err`] (busy-shaped → `StoreBusy`),
/// matching `ensure_wal`'s discipline.
fn ensure_synchronous_full(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch("PRAGMA synchronous = FULL;")
        .map_err(map_aux_err)?;
    let level: i64 = conn
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .map_err(map_aux_err)?;
    if level == 2 {
        Ok(())
    } else {
        Err(WalletError::StoreCorrupt)
    }
}

/// Pin the block cache's journal to ROLLBACK (`DELETE`) and VERIFY the echo
/// (W-swap-4-a-5 — the 4-a-4 shared-opener WAL flip silently converted the
/// SINGLE-connection cache, where WAL buys no concurrency and costs 2× write
/// amplification of every compact-block byte through the multi-hour initial
/// sync, plus a `-wal` high-water OUTSIDE the `CACHE_MAX_BYTES` accounting).
/// The pragma is persistent: a cache created inside the 4-a-4 window flips
/// BACK here on first open — the flip checkpoints its WAL and unlinks the
/// sidecars, keeping the cached rows (no spurious re-download). Echo
/// discipline mirrors [`ensure_wal`]: an error routes [`map_aux_err`]; a `wal`
/// echo without one is a lost flip lock ⇒ `StoreBusy` (unreachable on a
/// single-connection file, kept honest); any other echo ⇒ `StoreCorrupt`.
/// Every arm self-heals at the caller — the cache open is validated-or-deleted.
fn ensure_rollback_journal(conn: &Connection) -> Result<(), WalletError> {
    let mode: String = conn
        .query_row("PRAGMA journal_mode = DELETE", [], |r| r.get(0))
        .map_err(map_aux_err)?;
    if mode.eq_ignore_ascii_case("delete") {
        Ok(())
    } else if mode.eq_ignore_ascii_case("wal") {
        Err(WalletError::StoreBusy)
    } else {
        Err(WalletError::StoreCorrupt)
    }
}

/// Open the DISPOSABLE block cache keyed by the wallet's `db_key` — the
/// cache-shaped sibling of [`open_keyed_connection`] (W-swap-4-a-5). Same
/// key-first + forced-probe discipline; deliberately different tail: the
/// journal is pinned ROLLBACK, never WAL ([`ensure_rollback_journal`] — the
/// cache carries exactly ONE connection, so 4-a-4's reader-vs-writer fix does
/// not apply and WAL is pure write amplification), and the rarray vtab is NOT
/// loaded (the cache's SQL is plain SELECT/INSERT/DELETE; only
/// `zcash_client_sqlite` needs the module). `synchronous` pins FULL like the
/// wallet — the disposable contract leans on crash-atomicity, and the pin
/// makes the pre-4-a-4 default explicit. CREATE-allowed by design: the caller
/// (block_cache.rs `open_validated`) is validated-or-deleted.
pub(crate) fn open_keyed_cache_connection(
    path: &Path,
    db_key: &WalletDbKey,
) -> Result<Connection, WalletError> {
    let conn = Connection::open(path).map_err(map_aux_err)?;
    conn.execute_batch(db_key.pragma_key_statement().as_str())
        .map_err(map_aux_err)?;
    ensure_sqlcipher(&conn)?;
    conn.busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
        .map_err(map_aux_err)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(map_aux_err)?;
    // Durability BEFORE the journal flip (W-swap-4-a-6): the flip of a WAL-era
    // cache runs an implicit checkpoint, and that fold must ride the pinned
    // FULL fsync policy, not a maybe-NORMAL compile default.
    ensure_synchronous_full(&conn)?;
    ensure_rollback_journal(&conn)?;
    // Stray WAL sidecars beside an ALREADY-rollback cache are dead bytes (a
    // kill inside a prior flip's window, a hand-copied dir): in rollback mode
    // SQLite never reads them, but they pin disk and carry stale encrypted
    // frames — unlink them (W-swap-4-a-6 defense-in-depth; the flip arm's own
    // checkpoint unlinks its pair, so this is usually a no-op). Absence and
    // planted directories tolerated ([`remove_path_if_present`]); honest IO
    // faults propagate and the caller self-heals (validated-or-deleted).
    for suffix in ["-wal", "-shm"] {
        remove_path_if_present(&with_suffix(path, suffix))?;
    }
    conn.execute_batch("PRAGMA temp_store = MEMORY;")
        .map_err(map_aux_err)?;
    Ok(conn)
}

/// Fold the database's WAL into the main file NOW — `PRAGMA
/// wal_checkpoint(TRUNCATE)` with TYPED faults (W-swap-4-a-5). The LAST-CLOSE
/// checkpoint is silent (on failure SQLite leaves the WAL behind and close
/// still succeeds; rusqlite's `Drop` discards the close result), so any step
/// about to treat the MAIN FILE as the whole database (the rescan's fsync →
/// rename) calls this first on the LAST-standing connection: an ENOSPC
/// surfaces as the honest `DiskFull` at the point of failure instead of
/// [`verify_wal_folded`]'s fail-closed `StoreCorrupt` backstop. (Platform
/// caveat, mirrored from the block-cache `map_write_err` note: some platforms
/// surface a genuine ENOSPC as `SQLITE_IOERR` rather than `SQLITE_FULL`. Same
/// family: iOS Data Protection can fault mid-fold reads/writes with
/// `SQLITE_IOERR` while the device is locked. RESOLVED #371: [`map_aux_err`] now
/// types `SQLITE_IOERR` as the honest generic `Io` — not the `StoreCorrupt`
/// seed-restore scare — so this checkpoint fold surfaces `Io` on an IOERR-masked
/// fault; `verify_wal_folded`'s `StoreCorrupt` stays the fail-closed backstop for a
/// genuinely surviving WAL.) A checkpoint
/// is per-FILE (any connection folds every connection's commits); TRUNCATE
/// additionally zeroes the WAL, so the backstop's non-empty check passes. The
/// pragma reports reader interference as a `busy = 1` ROW (not an error) —
/// typed the transient `StoreBusy`, never a false corruption. On a
/// rollback-mode database the pragma is a no-op (`busy = 0`).
pub(crate) fn checkpoint_wal_truncate(conn: &Connection) -> Result<(), WalletError> {
    let busy: i64 = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))
        .map_err(map_aux_err)?;
    if busy == 0 {
        Ok(())
    } else {
        Err(WalletError::StoreBusy)
    }
}

/// Fail CLOSED if `path` still has a NON-EMPTY `-wal` sidecar (W-swap-4-a-4
/// review fold — the 3-review converged HIGH). Under WAL a COMMIT lands in the WAL
/// file and reaches the main DB only via a checkpoint; the LAST-CLOSE checkpoint's
/// failure (EIO / near-full disk) is SILENT — SQLite leaves the WAL behind and close
/// still succeeds, and rusqlite's `Drop` discards the close result. Any step that is
/// about to treat the MAIN FILE as the whole database (the rescan's fsync → atomic
/// rename) must therefore verify the WAL actually folded: a surviving non-empty WAL
/// means committed data (the copied aux money tables on the temp side; the old
/// wallet's most recent commits on the destination side) is NOT in the file being
/// renamed. A missing or zero-length WAL (a clean close unlinks it; a TRUNCATE
/// checkpoint leaves it empty) passes. Typed `StoreCorrupt` — fail-closed, caller
/// state untouched, a retry re-runs the idempotent rebuild.
pub(crate) fn verify_wal_folded(path: &Path) -> Result<(), WalletError> {
    let wal = with_suffix(path, "-wal");
    match std::fs::metadata(&wal) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(WalletError::from_io(e)),
        Ok(m) if m.len() == 0 => Ok(()),
        Ok(_) => Err(WalletError::StoreCorrupt),
    }
}

/// Append `suffix` to a path's final component (e.g. `wallet.db` + `-wal` →
/// `wallet.db-wal`). Operates on the raw `OsStr` so a non-UTF-8 path round-trips
/// losslessly; an empty suffix returns the path unchanged.
pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    if suffix.is_empty() {
        return path.to_path_buf();
    }
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// The SQLite sidecar suffixes a DB file can spawn (rollback-journal + WAL modes).
/// `""` is the DB file itself; the three sidecars are unlinked alongside it so a
/// recreate never inherits a stale journal/WAL from the old (corrupt/replaced) DB.
/// `pub(crate)` so tests that fabricate a stale DB (e.g. the rescan stale-temp test)
/// reference the ONE canonical set rather than re-listing it (single source of truth).
pub(crate) const DB_FILE_SUFFIXES: [&str; 4] = ["", "-journal", "-wal", "-shm"];

/// Remove a DB file AND its SQLite sidecars (`-journal`/`-wal`/`-shm`), tolerating
/// absence (a missing file is success). The ONE place this sweep lives — the rescan
/// temp-DB clear ([`reset_data_db_keep_seed`](crate::store)), the disposable
/// block-cache reset + its validated-or-deleted recreate
/// ([`BlockCache`](crate::block_cache)), and the provision prior-life hygiene
/// (store.rs, W-swap-4-a-5/-6) all call it, so the suffix set can never silently
/// diverge between them. ENOSPC/permission keep their honest IO code via
/// [`WalletError::from_io`] (→ `DiskFull` on a full disk), never a misleading
/// `StoreCorrupt`.
pub(crate) fn remove_db_and_sidecars(path: &Path) -> Result<(), WalletError> {
    for suffix in DB_FILE_SUFFIXES {
        remove_path_if_present(&with_suffix(path, suffix))?;
    }
    Ok(())
}

/// Remove one path — file, symlink, or (planted) directory — tolerating absence.
/// The ONE NotFound-tolerant unlink (→#374 review fold, all three reviewers):
/// [`remove_db_and_sidecars`], the cache opener's stale-sidecar sweep, and the
/// provisioning hygiene block all consume it, so the tolerance-and-directory
/// posture can never diverge between them. (Destroy's whole-dir sweep keeps its
/// separate warn-and-Ok `io::Result` policy — the crypto-shred there is already
/// complete when cleanup runs.)
///
/// DIRECTORY PARITY: a directory planted where a file belongs (`foo.tmp/`,
/// `block-cache.db-wal/` — hand-tampering or a foreign producer in the
/// SDK-exclusive dir) is removed like destroy's sweep would remove it
/// (`remove_dir_all`); the file-only arm would otherwise fail EPERM/EISDIR
/// identically on every retry — a new brick shape in the very batch that
/// un-bricks provisioning. `remove_file` on Unix unlinks a SYMLINK itself,
/// never its referent, so a link pointing at live data cannot pivot this into
/// deleting the target. Honest IO faults propagate via [`WalletError::from_io`].
pub(crate) fn remove_path_if_present(path: &Path) -> Result<(), WalletError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            // `symlink_metadata` (never follows): only a REAL directory takes
            // the dir arm — a symlink-to-dir was already unlinked above.
            let is_dir = path
                .symlink_metadata()
                .map(|m| m.file_type().is_dir())
                .unwrap_or(false);
            if !is_dir {
                return Err(WalletError::from_io(e));
            }
            match std::fs::remove_dir_all(path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(WalletError::from_io(e)),
            }
        }
    }
}

/// Classify a no-CREATE open error: a missing file (`CannotOpen`) is the
/// recoverable `NotFound`; everything else goes through [`classify_sqlite_error`].
fn map_open_err(e: rusqlite::Error) -> WalletError {
    match e {
        rusqlite::Error::SqliteFailure(err, _) if err.code == rusqlite::ErrorCode::CannotOpen => {
            WalletError::NotFound
        }
        other => classify_sqlite_error(&other),
    }
}

/// The ONE `rusqlite::Error` → SDK taxonomy mapping (#371), the SINGLE SOURCE OF TRUTH
/// every store seam classifies through so a fault can never tell "one condition, two
/// stories": `DiskFull` stays distinct ("free space and retry"); a `SQLITE_BUSY` past
/// `busy_timeout` is the TRANSIENT `StoreBusy` (W-swap-4-a-4 — under WAL an aux write
/// lost a writer-vs-writer race to an engine scan-batch commit; the rollback-journal era
/// mislabeled exactly this as corruption, the device blocker); a `SQLITE_IOERR` —
/// iOS Data Protection can surface a device-locked or ENOSPC-masked fault as IOERR rather
/// than FULL — is the honest generic `Io`, never the `StoreCorrupt` seed-restore scare;
/// everything else (incl. `SQLITE_CORRUPT`/`SQLITE_NOTADB`) folds to the fail-closed
/// `StoreCorrupt`. The engine seam ([`crate::sync::ClassifyStoreFault`]) unwraps its
/// `SqliteClientError` (both the `DbError` door AND the commitment-tree `Storage` door)
/// down to a `rusqlite::Error` and routes it HERE, so the two seams are compiler-shared,
/// not comment-coupled. The two `SQLITE_IOERR` extended codes that ARE corruption
/// (`SQLITE_IOERR_CORRUPTFS`, `_SHORT_READ`) are `StoreCorrupt`, checked before the family:
/// since R10 `Io` renders `Stalled { StorageUnavailable }` ("retrying"), and a failing flash
/// that reads short on every pass would otherwise say "retrying" forever and never name the
/// one remedy that works. `SQLITE_LOCKED` (a same-connection or shared-cache conflict, as
/// transient as BUSY) is `StoreBusy` too (R12 §4.1).
pub(crate) fn classify_sqlite_error(e: &rusqlite::Error) -> WalletError {
    match e {
        rusqlite::Error::SqliteFailure(err, _)
            if matches!(
                err.extended_code,
                rusqlite::ffi::SQLITE_IOERR_CORRUPTFS | rusqlite::ffi::SQLITE_IOERR_SHORT_READ
            ) =>
        {
            WalletError::StoreCorrupt
        }
        rusqlite::Error::SqliteFailure(err, _) => match err.code {
            rusqlite::ErrorCode::DiskFull => WalletError::DiskFull,
            rusqlite::ErrorCode::DatabaseBusy => WalletError::StoreBusy,
            rusqlite::ErrorCode::DatabaseLocked => WalletError::StoreBusy,
            // Static message: never the SQLite string (it can carry a path — §5.4).
            rusqlite::ErrorCode::SystemIoFailure => {
                WalletError::Io(std::io::Error::other("store io failure"))
            }
            _ => WalletError::StoreCorrupt,
        },
        _ => WalletError::StoreCorrupt,
    }
}

/// The by-value door to [`classify_sqlite_error`] for ANY of our own `rusqlite` calls (R12
/// §4.1): aux stores, keyed openers, block cache — never a blind `|_| StoreCorrupt`. User
/// seams retry a `StoreBusy` whole-txn via [`with_aux_busy_retry`]; background writers
/// surface it and rely on their next pass.
pub(crate) fn map_aux_err(e: rusqlite::Error) -> WalletError {
    classify_sqlite_error(&e)
}

/// Run ONE aux-store transaction, retrying a [`WalletError::StoreBusy`] up to
/// [`AUX_BUSY_RETRY_ATTEMPTS`](crate::constants::AUX_BUSY_RETRY_ATTEMPTS) total
/// attempts with a short de-phasing sleep between (W-swap-4-a-4). For the
/// USER-FACING aux seams — issued-quote `persist`/`take` + their
/// destination-watch twins, BOTH enqueue doors (`queue_send`,
/// `queue_deposit_send`), the parked-send cancel/retry taps, and the two quote
/// brackets' refund/destination index reserves — an
/// interactive tap should ride out a couple of engine scan-batch commits
/// instead of surfacing a transient. Background writers (drain marks, purge,
/// watch sweeps, sync/creation stamps) do NOT use this: their next pass is the
/// retry. The interactive two-step enrol's outbox write is the one deliberate
/// user-facing exclusion — it holds BOTH the db and aux guards, which
/// structurally excludes the busy (every engine writer needs the db guard).
///
/// SOUNDNESS CONTRACT: `op` must be ONE `IMMEDIATE` transaction (every aux
/// store fn is) — a failed COMMIT rolls back clean, so a retry re-runs a
/// whole, un-committed txn. NEVER wrap a multi-txn composite (e.g. the
/// `take` + `mark_executed` pair): if the first txn COMMITTED and the second
/// went busy, re-running the composite would re-run the committed one — for
/// `take`, misreading a consumed quote as "already executed". Wrap each txn
/// individually at the call site instead.
///
/// **Blocking** (same contract as the openers): the sleep is a plain
/// `thread::sleep`, callers are already inside `spawn_blocking`, and each
/// attempt internally waits out `busy_timeout` first — worst case ≈ 15.5 s,
/// compile-pinned far below the sync watchdog (constants.rs). Any non-busy
/// result (Ok, or any other error) returns immediately, unretried.
pub(crate) fn with_aux_busy_retry<T>(
    mut op: impl FnMut() -> Result<T, WalletError>,
) -> Result<T, WalletError> {
    let attempts = crate::constants::AUX_BUSY_RETRY_ATTEMPTS.max(1);
    for attempt in 1..=attempts {
        match op() {
            Err(WalletError::StoreBusy) if attempt < attempts => {
                std::thread::sleep(Duration::from_millis(
                    crate::constants::AUX_BUSY_RETRY_BACKOFF_MS,
                ));
            }
            other => return other,
        }
    }
    // Unreachable: the loop's final iteration always returns (the `attempt <
    // attempts` guard is false). Kept as the honest fail-closed fallback.
    Err(WalletError::StoreBusy)
}

/// Provision the wallet DB (§6.3): CREATE the encrypted DB under `db_key`, write
/// the `wallet_provisioning` sentinel row, then build the zcash schema via
/// `init_wallet_db`. IDEMPOTENT — repair re-runs it (the §6.3 resume contract):
/// the sentinel uses `CREATE TABLE IF NOT EXISTS` + a non-clobbering upsert, and
/// the migrator skips already-applied migrations. `network` selects the
/// consensus parameters baked into the schema (an opened DB's network is then
/// pinned by the manifest, store.rs).
///
/// Order: sentinel FIRST (our "this is a wallet DB" claim, written on the
/// bare keyed connection), THEN `WalletDb::from_connection` (which consumes the
/// connection) + `init_wallet_db`. A kill between the two is harmless — repair
/// re-runs the whole idempotent sequence, and the store-level marker (written
/// only after this returns Ok) is the sole Complete signal.
///
/// `created_at`: `Some(unix_secs)` ONLY for a freshly-generated create
/// (`SeedSource::Generate` / host-attested `FreshRawBytes`, threaded from
/// `ProvisionIntent.freshly_generated_at`) — writes the
/// creation stamp (§3.2f #356-F4, `creation_stamp::record_once`; earliest
/// wins across repair re-runs). `None` for restores AND for the rescan
/// rebuild's temp DB (there the real stamp rides `copy_aux_tables`).
pub(crate) fn provision_db(
    path: &Path,
    db_key: &WalletDbKey,
    network: Network,
    created_at: Option<u64>,
) -> Result<(), WalletError> {
    let conn = open_keyed_connection(path, db_key)?;
    // id = 1 single-row sentinel; the CHECK pins it so a second row is impossible.
    conn.execute_batch(
        r#"CREATE TABLE IF NOT EXISTS wallet_provisioning (
               id INTEGER PRIMARY KEY CHECK (id = 1),
               schema_version INTEGER NOT NULL
           );"#,
    )
    .map_err(map_aux_err)?;
    conn.execute(
        r#"INSERT INTO wallet_provisioning (id, schema_version) VALUES (1, ?1)
           ON CONFLICT(id) DO NOTHING"#,
        [WALLET_SCHEMA_VERSION],
    )
    .map_err(map_aux_err)?;
    // The creation stamp (§3.2f #356-F4) — BEFORE `migrate` consumes the
    // connection. Its table must exist first (the ensure inside `migrate` has
    // not run yet on a fresh DB); `ensure_table` is idempotent, so the second
    // run inside `ensure_aux_tables` is a no-op. Write-side faults fail the
    // (user-attended, idempotently retried) create loudly rather than silently
    // dropping a money input.
    if let Some(at) = created_at {
        crate::creation_stamp::ensure_table(&conn)?;
        crate::creation_stamp::record_once(&conn, network, at)?;
    }

    // Build the zcash schema on the SAME keyed connection (sentinel FIRST, on the
    // bare conn, since `migrate` consumes it). `seed = None`: a fresh wallet has no
    // derived account, so every migration applies cleanly. Provisioning discards
    // the wrapped handle — it only needed the schema written to disk; `open`/the
    // handle re-wrap when the wallet is opened.
    let _wdb = migrate(conn, network, None)?;
    Ok(())
}

/// Wrap a bare keyed connection in `WalletDb` and run the idempotent
/// `init_wallet_db` schemerz migrator — the one place OUR connection meets the
/// librustzcash schema. Shared by [`provision_db`] (build the schema at create,
/// `seed = None` — a fresh wallet has no derived account) and [`open_db_migrated`]
/// (migrate-up at open, `seed` = the loaded custody seed). `from_connection`
/// requires `load_module` to have run on the connection (both openers do it);
/// `SystemClock`/`OsRng` are the upstream API's clock/rng capabilities — neither
/// generates any of OUR key material (the DB key is minted in seal.rs).
///
/// **The `Some(seed)` retry loop (§3.2b, now reachable since accounts exist).**
/// We try `init_wallet_db(None)` first; once a DERIVED ACCOUNT exists, an app
/// upgrade that adds a seed-validating migration makes the seed-less pass return
/// `WalletMigrationError::SeedRequired` (downcast through the error source — the
/// upstream-documented pattern). We retry WITH the seed when one is available;
/// with NONE available (the `SeedPersistence::None` path holds none at rest)
/// we surface the typed `SeedRequired` so the host re-supplies it (§4.2 re-derive
/// contract). Steady-state opens (no pending seed-migration) succeed on the first
/// `None` pass even with an account present — the retry is correctness insurance,
/// not a per-open cost. `SecretVec` zeroizes the transient seed copy on drop.
///
/// A non-seed migration failure is CLASSIFIED (T0-7, §4aa), not blanket corruption:
/// the migrator error is unwrapped to the `WalletMigrationError` it carries and, when
/// that is a `DbError`, routed through [`classify_sqlite_error`] like every other store
/// seam — so an ENOSPC at a new wallet's first open is `DiskFull` ("free space and
/// retry") and a held lock is the transient `StoreBusy`, never the red
/// "restore from your recovery phrase". Only a fault that IS the wallet being wrong
/// (`CorruptedData`, `CommitmentTree`, `CannotRevert`, …) keeps the fail-closed
/// `StoreCorrupt`, and nothing is ever auto-wiped. This doc used to say the finer
/// classification was owed because "the schemerz migrator wraps rusqlite errors, so the
/// underlying `io::Error` isn't reachable here" — that was false: `migration_error`
/// below already reaches through `source()`, and `WalletMigrationError::DbError` is a
/// public variant.
fn migrate(
    conn: Connection,
    network: Network,
    seed: Option<&[u8]>,
) -> Result<WalletConn, WalletError> {
    // Ensure OUR aux tables exist on the SAME keyed connection BEFORE `from_connection`
    // consumes it — `migrate` is the single chokepoint both provision (a fresh DB) AND
    // every open route through (so no `WALLET_SCHEMA_VERSION` bump is needed to add an aux
    // store; an existing wallet gains it on next open). The aux RUNNING connection
    // (`Inner.aux_db`) is opened separately; this only guarantees the schema is on disk.
    ensure_aux_tables(&conn)?;
    let mut wdb = WalletDb::from_connection(conn, network.consensus(), SystemClock, OsRng);
    match init_wallet_db(&mut wdb, None) {
        Ok(()) => Ok(wdb),
        // Not reachable against the exact-pinned `=0.22.0` either — RE-CHECKED at the
        // NU6.3 wave rather than inherited: 0.22.0 adds 19 migrations
        // (`ironwood_shardtree`, `note_locking`, the `orchard_ironwood_migration_*`
        // set, …) and none of them validates a seed; `SeedRequired` is still raised
        // only by the pre-existing `full_account_ids` and `ufvk_support`. This arm
        // activates only when an upstream bump ships a seed-validating migration.
        // Built as forward correctness; covered by the pattern, not a test.
        Err(e)
            if matches!(
                migration_error(&e),
                Some(WalletMigrationError::SeedRequired)
            ) =>
        {
            let seed = seed.ok_or(WalletError::SeedRequired)?;
            let secret = SecretVec::new(seed.to_vec());
            init_wallet_db(&mut wdb, Some(secret)).map_err(|e2| {
                // A WRONG re-supplied seed is `SeedNotRelevant` (distinct from
                // corruption) — surface it as `SeedMismatch`, never `StoreCorrupt`
                // ("never auto-wipe"). Unreachable in v1 (open loads the wallet's OWN
                // custodied seed); live when the None-mode re-supply port lands
                // (inc-2c-iv).
                match migration_error(&e2) {
                    Some(WalletMigrationError::SeedNotRelevant) => WalletError::SeedMismatch,
                    other => classify_migration_error(other),
                }
            })?;
            Ok(wdb)
        }
        // CLOSED at T0-4/T0-7 (2026-09-11, §4aa). Every other migration failure USED to
        // fold to `StoreCorrupt` — the RED "restore from your recovery phrase" remedy —
        // including an ENOSPC or a transient BUSY mid-migration, which are "free space
        // and retry" conditions. That is the same mis-attribution #371 fixed on the SCAN
        // path, and it was still open on the FIRST-OPEN path, where 0.22.0 runs 19 new
        // migrations (one of which builds a UNIQUE index over an existing table) on every
        // create and every seed-restore. A brand-new user whose disk filled during
        // provisioning was told their wallet was destroyed.
        //
        // **The comment that used to sit here called it a structural blocker — "the
        // schemerz migrator wraps the rusqlite error, so the underlying code is not
        // reachable from here" — and that was FALSE** (§4aa, verified at the pin).
        // `WalletMigrationError::DbError(rusqlite::Error)` is a public variant
        // (`zcash_client_sqlite-0.22.0 wallet/init.rs:57`), and [`migration_error`] right
        // below already reaches the `WalletMigrationError` through `source()` —
        // the same door the `SeedRequired` arm above has used all along. So the inner
        // error was one match arm away the whole time.
        //
        // Routed through [`classify_sqlite_error`], the same SSOT the scan door
        // ([`crate::sync::ClassifyStoreFault`]) and the aux stores use, so this seam
        // cannot drift from them: FULL → `DiskFull`, BUSY → `StoreBusy`, IOERR → `Io`,
        // everything else → the fail-closed `StoreCorrupt`. `Other(SqliteClientError)`
        // is unwrapped too — upstream wraps some migration faults that way, and a
        // `DbError` inside it is the same condition wearing a second box.
        // A non-SQLite `WalletMigrationError` (`CorruptedData`, `CommitmentTree`,
        // `CannotRevert`, …) keeps `StoreCorrupt`: those ARE the wallet being wrong.
        Err(e) => Err(classify_migration_error(migration_error(&e))),
    }
}

/// The ONE unwrap of a migrator fault (the first try AND the `SeedRequired` retry, R12
/// §4.1 — kept LOCAL: `db.rs` is the classification SSOT and imports nothing from sync).
/// `DbError` → [`classify_sqlite_error`]; `Other(SqliteClientError)`'s `DbError` likewise,
/// and its `Io` (a decode of our bytes, not file I/O — `crate::sync`'s engine-Io rule;
/// probably unreachable, upstream turns it into `CorruptedData` at `init.rs:189`) is
/// `DiskFull` on `StorageFull`, else `StoreCorrupt`. Anything else: `StoreCorrupt`.
fn classify_migration_error(e: Option<&WalletMigrationError>) -> WalletError {
    match e {
        Some(WalletMigrationError::DbError(inner)) => classify_sqlite_error(inner),
        Some(WalletMigrationError::Other(inner)) => match inner.as_ref() {
            SqliteClientError::DbError(inner) => classify_sqlite_error(inner),
            SqliteClientError::Io(io) if io.kind() == std::io::ErrorKind::StorageFull => {
                WalletError::DiskFull
            }
            SqliteClientError::Io(_) => WalletError::StoreCorrupt,
            _ => WalletError::StoreCorrupt,
        },
        _ => WalletError::StoreCorrupt,
    }
}

/// Downcast a migrator error to the `WalletMigrationError` it wraps (the
/// upstream-documented `source()` + `downcast_ref` pattern), so the migrate retry
/// can distinguish `SeedRequired` / `SeedNotRelevant` from generic corruption.
fn migration_error(e: &dyn std::error::Error) -> Option<&WalletMigrationError> {
    e.source()
        .and_then(|s| s.downcast_ref::<WalletMigrationError>())
}

/// Create OUR durable aux tables on `conn` — the SINGLE declaration of the aux-table
/// set. Each `ensure_table` is idempotent (`CREATE TABLE IF NOT EXISTS`, additive
/// columns via `ALTER` on a pre-existing table), so this runs on every provision AND
/// open without a `WALLET_SCHEMA_VERSION` bump: queued-send intents (inc-2d-3-b-i), the
/// refund-index counter (§2.6 HARD-H), the in-flight issued-quote store (§3.2 W2), and —
/// swap-build only — the scoped swap-destination detection set (§3.3b D2, ADR-0530).
///
/// MONEY-SAFETY CONTRACT (ADR-0534): adding an aux store here REQUIRES adding its table
/// name to [`AUX_TABLES_PRESERVED`] (and removing one requires removing it there) — else
/// a rescan would silently drop that store's money state. The
/// `aux_tables_preserved_matches_what_migrate_creates` guard test fails loudly the moment
/// these two lists diverge, so the coupling can never rot unnoticed.
fn ensure_aux_tables(conn: &Connection) -> Result<(), WalletError> {
    crate::intent_store::ensure_table(conn)?;
    crate::refund_index::ensure_table(conn)?;
    // The FR-8 public diversified-receive counter (ADR-0537) — the refund counter's
    // shielded sibling: rides the rescan copy UNCLEARED (never-reuse must survive a
    // rescan; the engine's courtesy address rows drop, harmlessly — trial decryption
    // detects shielded funds at any diversifier).
    crate::diversified_index::ensure_table(conn)?;
    // The engine-registration high-water marker (#368, the ADR-0527 backfill) — the
    // counter's sibling. Rides the rescan copy like every aux table, but its ROW is
    // cleared on the rescan temp DB (the `sync_stamp` copy-then-clear precedent): the
    // engine rebuild drops `get_address_for_index` registrations, so the marker must
    // fall to 0 and the next refresh re-registers the full allocated range.
    crate::refund_index::ensure_registration_table(conn)?;
    crate::issued_quote_store::ensure_table(conn)?;
    // UNCONDITIONAL since W-swap-5 (#366): the swap tables' SCHEMA exists in every
    // build — feature gates scope BEHAVIOR (the DETECT poll, the service), never
    // the durability of money state already written, so a non-swap build's rescan
    // preserves a swap-build's in-flight rows instead of silently dropping them
    // (the cross-build finding, ADR-0534).
    crate::swap_destination_store::ensure_table(conn)?;
    crate::swap_record_store::ensure_table(conn)?;
    crate::sync_stamp::ensure_table(conn)?;
    // The delivery obligation's accepted mark (stage S8 `obligation`): rides a
    // rescan UNCLEARED — every reading starts from the engine's `transactions`
    // row, which the rebuild discards, so a surviving mark is inert (its module doc).
    crate::delivery::ensure_table(conn)?;
    // The consensus verdict + grace anchor (`ironwood-nu63-support.md` §6.2).
    // Rides a rescan UNCLEARED, unlike `sync_stamp`: a rescan rebuilds OUR view
    // of the chain and says nothing about which rules the network runs, so
    // clearing it would drop the grace anchor and re-refuse a wallet whose only
    // act was rebuilding its balance.
    crate::consensus_stamp::ensure_table(conn)?;
    crate::creation_stamp::ensure_table(conn)?;
    // #357 durable aux flags — both TABLES ride the rescan copy (the list below
    // == this ensure set, ADR-0534 guard). The fingerprint's ROW rides
    // uncleared (same seed after a rescan); the everSynced ROW is CLEARED by
    // the rescan rebuild (the `sync_stamp` precedent — see the list's entry).
    crate::ever_synced::ensure_table(conn)?;
    crate::seed_fingerprint::ensure_table(conn)?;
    // #377 s357b-2: the rescan-rebuilding breadcrumb — the INVERSE lifecycle
    // (the rescan SETS its row on the temp DB, reached-tip clears it).
    crate::rescan_rebuilding::ensure_table(conn)?;
    // T0-1a-R: the A11 height bind's rewind ledger — how many chain rewinds this
    // wallet has performed, and what that count stood at when each pool's subtree
    // heights were last written. It carries no money state and no wallet-narrowing
    // number (one integer per scope); its ROWS are CLEARED by the rescan rebuild, the
    // `sync_stamp` posture — a rebuilt wallet has no stale record to relax for
    // (`root_bind`'s rewind-gate section).
    crate::root_bind::ensure_rewind_table(conn)?;
    // BIND-1-R: the A11 height bind's boundary-bound ledger — for each pool and shard
    // index, the two blocks between which this wallet's OWN scan proved that subtree's
    // completion must lie. It carries no money state; a completion height is
    // wallet-narrowing (§5.4), which is why it lives here in the encrypted aux DB and
    // never on a log line. Its ROWS are CLEARED by the rescan rebuild, the
    // `sync_stamp` posture and `ensure_rewind_table`'s — a rebuilt wallet has no
    // recorded height left for a bracket to refute and none of the blocks it was
    // counted from (`root_bind`'s boundary-bound section).
    crate::root_bind::ensure_boundary_bound_table(conn)?;
    // P3-13: the sync-server choice — which server the USER picked. Rides a
    // rescan UNCLEARED (the `consensus_stamp` posture: the choice is about the
    // user's trust, not the chain view); the row stores an id or a URL, never
    // a key (`sync_server` module doc).
    crate::sync_server::ensure_table(conn)?;
    Ok(())
}

/// The durable AUX tables `store::reset_data_db_keep_seed` PRESERVES across a
/// rescan (ADR-0534): money intents + in-flight swap detection state that are NOT
/// re-derivable by re-scanning the chain (unlike the zcash account/notes/witnesses,
/// which the seed regenerates at the lower birthday). Kept in ONE place so the
/// rescan copy and the [`ensure_aux_tables`] chokepoint can never silently diverge on
/// the set (guarded by `aux_tables_preserved_matches_what_migrate_creates`).
/// The swap tables ride UNCONDITIONALLY (W-swap-5 / #366 — the cross-build
/// fold): the pre-#366 `cfg(feature = "swap")` gate here meant a NON-swap build
/// rescanning a swap-build `wallet.db` silently dropped its in-flight swap rows;
/// the set is now build-independent, so the guard test pins ONE set everywhere.
const AUX_TABLES_PRESERVED: &[&str] = &[
    "queued_send_intent",
    "refund_address_index",
    // The FR-8 diversified-receive counter (ADR-0537): rides UNCLEARED — the
    // never-reuse property must survive a rescan (its addresses need no
    // re-registration; shielded detection is diversifier-agnostic).
    "diversified_address_index",
    // The registration marker (#368): the TABLE rides the copy (keeps this list ==
    // the ensure set), but the rescan rebuild CLEARS its row on the temp DB before
    // the rename — the rebuilt engine DB holds no `get_address_for_index`
    // registrations, so a surviving marker would falsely assert them registered
    // (`refund_index::REGISTRATION_TABLE` doc; the `sync_stamp` precedent below) —
    // and RE-SEEDS it there from the copied counter (#382: the quiescent seed point;
    // `store::open` seeds every other path, which is also why migrate itself must
    // NOT seed — a migrate-seeded temp row would collide with this plain-INSERT copy).
    crate::refund_index::REGISTRATION_TABLE,
    "issued_swap_quote",
    "swap_destination",
    crate::swap_record_store::TABLE,
    // Display-only (§2.5 `last_synced`): the TABLE rides the copy (keeps this
    // list == the ensure set, ADR-0534 guard), but the rescan rebuild CLEARS
    // its row on the temp DB before the rename — a stamp that survived a
    // rescan would pair the old height/time with a rebuilding balance
    // (`sync_stamp` module doc).
    crate::sync_stamp::TABLE,
    // The delivery obligation's accepted mark (stage S8 `obligation`): RIDES the
    // copy UNCLEARED — a mark is read only beside a wallet-created `transactions`
    // row, and the rebuild discards those, so a carried-over mark is inert
    // (`delivery` module doc).
    crate::delivery::ACCEPTED_TABLE,
    // The creation stamp (§3.2f #356-F4): the row RIDES the copy UNCLEARED —
    // when the seed came into existence is a fact a rescan cannot change
    // (`creation_stamp` module doc).
    crate::creation_stamp::TABLE,
    // The consensus verdict + grace anchor (`ironwood-nu63-support.md` §6.2):
    // the row RIDES the copy UNCLEARED, the `creation_stamp` posture rather
    // than `sync_stamp`'s — which consensus rules the network runs is a fact
    // about the CHAIN, and a rescan changes only our view of it.
    crate::consensus_stamp::TABLE,
    // #357 durable everSynced flag: the TABLE rides the copy (keeps this list ==
    // the ensure set, ADR-0534 guard), but the rescan rebuild CLEARS its ROW on the
    // temp DB before the rename — the `sync_stamp` precedent, NOT `creation_stamp`'s
    // preserve. A rescan resets the wallet to catching-up, so the catch-up cue must
    // re-show during the rebuild (else `everSynced && lastSynced==null` is ambiguous
    // and an offline relaunch mid-rebuild suppresses the "did I lose funds?"
    // reassurance — the post-ship reliability/UX HIGH). Re-set on the first
    // post-rescan reached-tip (`ever_synced` module doc).
    crate::ever_synced::TABLE,
    // P3-13 sync-server choice: RIDES the copy UNCLEARED — the user's pick of a
    // server survives a rescan the way the consensus stamp does (a rescan
    // rebuilds our view of the chain, not the user's trust). The row is an id
    // or a URL — and for a custom server, the USER's key bound to that URL
    // (ADR-0568), never a host's (`sync_server` module doc). The copy is
    // logical (`INSERT … SELECT *` into a fresh file), so only the current
    // row moves — no freed page carrying an old key does.
    crate::sync_server::TABLE,
    // #357 seed fingerprint: RIDES the copy UNCLEARED — a rescan keeps the same
    // seed, so its fingerprint must survive to keep verifying a None-persistence
    // host-supplied seed after a rescan.
    crate::seed_fingerprint::TABLE,
    // #377 s357b-2 rescan-rebuilding breadcrumb: the TABLE rides the copy
    // (keeps this list == the ensure set), and the rescan rebuild SETS its row
    // on the temp DB before the rename — the INVERSE of the `sync_stamp` /
    // `ever_synced` clears above, same crash-atomicity (a kill leaves
    // intact-old-without or complete-new-with, never torn). Cleared at the
    // first post-rescan reached-tip (`rescan_rebuilding` module doc).
    crate::rescan_rebuilding::TABLE,
    // T0-1a-R the A11 rewind ledger: the TABLE rides the copy (keeps this list ==
    // the ensure set, ADR-0534 guard), but the rescan rebuild CLEARS its rows on the
    // temp DB before the rename — the `sync_stamp` precedent, NOT `creation_stamp`'s
    // preserve. The rebuild drops every shard row and every scanned block, so there is
    // no stale record for a surviving rewind to relax; carrying one across would only
    // hand the endpoint a free unbound pass on the first post-rescan sync
    // (`root_bind::clear_rewind_watch`).
    crate::root_bind::REWIND_TABLE,
    // BIND-1-R the boundary-bound ledger: the TABLE rides the copy (keeps this list ==
    // the ensure set, ADR-0534 guard), but the rescan rebuild CLEARS its rows on the
    // temp DB before the rename — the rewind ledger's posture, for the same reason
    // twice over. A bracket is a statement about a recorded completion height, and the
    // rebuild drops every shard row; it is also derived from `blocks` rows the rebuild
    // drops, so carrying one across would refuse an honest server a height this wallet
    // can no longer prove wrong (`root_bind::clear_boundary_bounds`).
    crate::root_bind::BOUNDARY_BOUND_TABLE,
];

/// Copy the durable AUX tables from `src` (the OLD `wallet.db`) into `dst` (the
/// freshly-provisioned, account-less rescan temp DB), both keyed by `db_key` — the
/// ADR-0534 aux preservation. The zcash data tables are intentionally NOT copied
/// (the seed re-derives them at the lower birthday); only the money intents / swap
/// detection state ([`AUX_TABLES_PRESERVED`]) ride across.
///
/// Mechanism: ATTACH `src` **read-only** (`mode=ro`) under the SAME raw key, then — in
/// ONE deferred transaction — `INSERT … SELECT *` each aux table. The columnless
/// `SELECT *` is sound because BOTH DBs routed their aux schema through
/// [`ensure_aux_tables`], so the column set+order match; [`assert_columns_match`]
/// re-checks that per table BEFORE each copy, so any schema drift fails LOUDLY (typed)
/// rather than silently mis-mapping a money column. COMMIT then DETACH + close, so the
/// temp DB is committed and holds no attached handle for the caller's atomic rename.
///
/// The source is opened **read-only**, so the OLD wallet.db is provably untouched until
/// the rename — the crash-atomicity the seed-recovery depends on, enforced by SQLite
/// (a stray write would be `SQLITE_READONLY`), not merely by "we only wrote a SELECT".
/// A DiskFull stays distinct; any other fault is `StoreCorrupt`.
///
/// **Synchronous + blocking** (same contract as [`open_keyed_connection`]): the
/// rescan path calls this inside `spawn_blocking` under the held single-writer lock.
pub(crate) fn copy_aux_tables(
    dst: &Path,
    src: &Path,
    db_key: &WalletDbKey,
) -> Result<(), WalletError> {
    // A non-UTF-8 db_dir is unsupported on the platforms we ship; reject typed rather
    // than lossily mangle the ATTACH URI.
    let src_str = src.to_str().ok_or(WalletError::StoreCorrupt)?;
    // `SQLITE_OPEN_URI` so the read-only `file:…?mode=ro` ATTACH below is honored; `dst`
    // is a plain path (never `file:`-prefixed), so it stays interpreted literally.
    let conn = open_existing_keyed_with_flags(dst, db_key, OpenFlags::SQLITE_OPEN_URI)?;
    // ATTACH the source read-only under the SAME key (the raw key inlined as SQL; see
    // `WalletDbKey::attach_statement`). The statement string zeroizes on drop.
    let attach = db_key.attach_statement(src_str, "old");
    conn.execute_batch(attach.as_str()).map_err(map_aux_err)?;
    let copy = || -> Result<(), WalletError> {
        // ONE deferred transaction: every aux table copies, or none does (a torn temp
        // DB would be discarded by the rescan abort anyway — keep it all-or-nothing).
        // DEFERRED (not IMMEDIATE) so the write lock is taken lazily on `main` at the
        // first INSERT; the read-only `old` is only ever SHARED-locked for the SELECT.
        conn.execute_batch("BEGIN;").map_err(map_aux_err)?;
        let inserts = || -> Result<(), WalletError> {
            for &table in AUX_TABLES_PRESERVED {
                // `table` is a compile-time constant from `AUX_TABLES_PRESERVED`, never
                // attacker input — no SQL-injection surface in the interpolation.
                assert_columns_match(&conn, table)?;
                let sql = format!("INSERT INTO main.{table} SELECT * FROM old.{table};");
                conn.execute_batch(&sql).map_err(map_aux_err)?;
            }
            Ok(())
        };
        match inserts() {
            Ok(()) => conn.execute_batch("COMMIT;").map_err(map_aux_err),
            Err(e) => {
                // Roll back the partial copy; the original error wins over a rollback fault.
                let _ = conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    };
    let copied = copy();
    // DETACH regardless of the copy outcome so the connection closes cleanly (no
    // lingering attached handle that could hold the temp file); the copy error wins. A
    // DETACH fault is near-unreachable here — the copy's transaction already COMMITted (or
    // rolled back), so no transaction is open, and the connection drop would release the
    // handle anyway — but surface it rather than swallow it (no silent failures).
    let detached = conn.execute_batch("DETACH DATABASE old;");
    copied?;
    detached.map_err(map_aux_err)?;
    Ok(())
}

/// Assert `main.<table>` and `old.<table>` have an IDENTICAL column signature (matching
/// ordinal, name, and declared type), so the columnless `INSERT … SELECT *` in
/// [`copy_aux_tables`] maps money columns 1:1. A drift — e.g. an old DB whose additive
/// `ALTER` the migrate-on-open didn't reconcile — fails typed `StoreCorrupt` rather than
/// silently writing the wrong column. `table` is a compile-time constant (no injection).
fn assert_columns_match(conn: &Connection, table: &str) -> Result<(), WalletError> {
    let signature = |schema: &str| -> Result<Vec<(i64, String, String)>, WalletError> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA {schema}.table_info({table});"))
            .map_err(map_aux_err)?;
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,    // cid (ordinal)
                r.get::<_, String>(1)?, // name
                r.get::<_, String>(2)?, // declared type
            ))
        })
        .map_err(map_aux_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_aux_err)
    };
    let main_sig = signature("main")?;
    // An EMPTY signature means the table is absent — make the existence part of the
    // contract explicit (fail-closed) rather than relying on the later INSERT to error,
    // so the function's name ("columns match") can't lie. Unreachable in practice (both
    // DBs ran `ensure_aux_tables` on open/provision), but cheap defense-in-depth.
    if main_sig.is_empty() || main_sig != signature("old")? {
        return Err(WalletError::StoreCorrupt);
    }
    Ok(())
}

/// Open a PROVISIONED wallet DB (§6.3 layers 2+3): open-existing (no CREATE),
/// then verify the `wallet_provisioning` sentinel is present — a truncated /
/// 0-byte DB (which SQLite would otherwise open as a fresh empty DB) lacks the
/// sentinel and fails typed `StoreCorrupt`, never a silent fresh wallet. The
/// store layer's completion-marker check (layer 1) runs BEFORE this. Returns the
/// bare keyed connection (the sentinel/at-rest tests assert at this layer);
/// [`open_db_migrated`] is the production open path that also migrates.
pub(crate) fn open_db(path: &Path, db_key: &WalletDbKey) -> Result<Connection, WalletError> {
    let conn = open_existing_keyed_connection(path, db_key)?;
    // The sentinel row MUST be present and current. A missing table (fresh /
    // truncated DB) or a future schema version is corruption, fail-closed.
    let version: i64 = conn
        .query_row(
            "SELECT schema_version FROM wallet_provisioning WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .map_err(map_aux_err)?;
    if version != WALLET_SCHEMA_VERSION {
        return Err(WalletError::StoreCorrupt);
    }
    Ok(conn)
}

/// Open a provisioned wallet DB AND migrate it up (§6.3 + inc-2b-ii-B): the
/// production open path the async `Wallet` handle drives. Runs the create-vs-open
/// guard ([`open_db`]: no-CREATE + the sentinel check), THEN re-runs the
/// idempotent `init_wallet_db` migrator so an app upgrade that added migrations
/// brings the schema forward at open time (a no-op on an already-current DB —
/// "re-runs init_wallet_db on open"). `seed` is the loaded custody seed
/// (`Some` for `SealedKeychain`, `None` for `None`-mode) — threaded into the
/// migrate retry loop (§3.2b) so a post-account seed-validating migration can run.
/// Fail-closed `StoreCorrupt` on any guard or non-seed migration failure.
/// **Synchronous + blocking** — the handle calls this inside `spawn_blocking`.
pub(crate) fn open_db_migrated(
    path: &Path,
    db_key: &WalletDbKey,
    network: Network,
    seed: Option<&[u8]>,
) -> Result<WalletConn, WalletError> {
    let conn = open_db(path, db_key)?;
    migrate(conn, network, seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn wallet_db_key_sealed_never_plaintext_on_disk_at_rest() {
        // §8 named test (the at-rest DB-FILE half; the key-seal half is in
        // seal.rs): content written under the DB key is SQLCipher-encrypted on
        // disk — no plaintext SQLite magic, no plaintext content — round-trips
        // with the same key, and FAILS typed with a wrong key (never silently
        // opens an empty/garbage DB).
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();

        // create + write under the key, then close (drop)
        {
            let conn = open_keyed_connection(&path, &key).expect("open new");
            conn.execute_batch(
                "CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('secret-note-xyzzy');",
            )
            .expect("write");
        }

        // on-disk file is SQLCipher-encrypted: not the plaintext SQLite magic,
        // and the known plaintext content does not appear in the raw bytes
        let bytes = std::fs::read(&path).expect("read db file");
        assert!(
            !bytes.starts_with(b"SQLite format 3\0"),
            "header must be encrypted (no plaintext SQLite magic)"
        );
        let needle = b"secret-note-xyzzy";
        assert!(
            bytes.windows(needle.len()).all(|w| w != needle),
            "plaintext content must not appear on disk"
        );

        // reopen with the SAME key → content readable
        {
            let conn = open_keyed_connection(&path, &key).expect("reopen same key");
            let v: String = conn
                .query_row("SELECT v FROM t", [], |r| r.get(0))
                .expect("read back");
            assert_eq!(v, "secret-note-xyzzy");
        }

        // reopen with a WRONG key → fails typed, never opens
        let wrong = WalletDbKey::generate();
        assert!(matches!(
            open_keyed_connection(&path, &wrong),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn open_registers_rarray_vtab() {
        // the rarray table-valued function zcash_client_sqlite needs resolves
        // ONLY because open_keyed_connection ran load_module.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let conn = open_keyed_connection(&dir.path().join("w.db"), &key).expect("open");
        assert!(
            conn.prepare("SELECT value FROM rarray(?1)").is_ok(),
            "rarray vtab must be registered by the open"
        );
    }

    #[test]
    fn open_existing_missing_file_is_not_found_never_created() {
        // §6.3 layer 2: open-existing on a missing path is typed NotFound and
        // does NOT create the file (no SQLITE_OPEN_CREATE).
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        assert!(matches!(
            open_existing_keyed_connection(&path, &key),
            Err(WalletError::NotFound)
        ));
        assert!(
            !path.exists(),
            "open-existing must never CREATE the DB file"
        );
        // open_db (sentinel layer on top) inherits the NotFound.
        assert!(matches!(open_db(&path, &key), Err(WalletError::NotFound)));
    }

    #[test]
    fn provision_then_open_roundtrips_and_is_idempotent() {
        // §6.3 provision → open happy path; provision is idempotent (repair
        // re-runs it) and a wrong key still fails typed after provisioning.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();

        provision_db(&path, &key, Network::Main, None).expect("provision");
        provision_db(&path, &key, Network::Main, None).expect("re-provision is idempotent");
        open_db(&path, &key).expect("open a provisioned DB");

        let wrong = WalletDbKey::generate();
        assert!(matches!(
            open_db(&path, &wrong),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn provision_builds_zcash_schema_and_migration_is_idempotent() {
        // W3-inc-2b-ii-A: provision_db runs init_wallet_db, so the real zcash
        // schema is present (e.g. the `accounts` table) AND schemerz recorded the
        // applied migrations. Re-running provision_db is a no-op at the migration
        // level (the migrator skips applied migrations) — the §6.3 repair contract
        // depends on this.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();

        provision_db(&path, &key, Network::Test, None).expect("provision");
        {
            let conn = open_existing_keyed_connection(&path, &key).expect("open existing");
            // a real zcash_client_sqlite schema table proves init_wallet_db ran
            let accounts_present: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='accounts'",
                    [],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            assert!(
                accounts_present,
                "init_wallet_db must create the zcash schema"
            );
            // the migrator recorded applied migrations in its bookkeeping table
            // (`schemer_migrations`, the name zcash_client_sqlite passes to schemerz)
            let applied: i64 = conn
                .query_row("SELECT count(*) FROM schemer_migrations", [], |r| r.get(0))
                .expect("query the migrator bookkeeping table");
            assert!(applied > 0, "init_wallet_db must record applied migrations");
        }

        // re-provision: idempotent, schema intact, still opens
        provision_db(&path, &key, Network::Test, None).expect("re-provision is idempotent");
        open_db(&path, &key).expect("opens after re-provision");
    }

    #[test]
    fn open_db_migrated_runs_migrator_idempotently_and_guards_truncation() {
        // W3-inc-2b-ii-B: the production open path wraps the keyed connection in
        // WalletDb and re-runs init_wallet_db (idempotent migrate-up at open). It
        // succeeds repeatedly on an up-to-date DB (a no-op migration), and it still
        // inherits open_db's create-vs-open guard — a truncated DB fails typed
        // StoreCorrupt through the migrate wrapper, never a silent fresh wallet.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();

        provision_db(&path, &key, Network::Main, None).expect("provision");
        // open + migrate-up succeeds, and is idempotent across opens (the migrator
        // skips already-applied migrations — a re-open is a no-op). `seed = None`:
        // an account-less wallet never reaches the SeedRequired retry arm.
        drop(open_db_migrated(&path, &key, Network::Main, None).expect("first migrated open"));
        drop(
            open_db_migrated(&path, &key, Network::Main, None)
                .expect("second migrated open is a no-op"),
        );

        // the truncation backstop holds through the migrate wrapper
        std::fs::write(&path, b"").expect("truncate to empty");
        assert!(
            matches!(
                open_db_migrated(&path, &key, Network::Main, None),
                Err(WalletError::StoreCorrupt)
            ),
            "a 0-byte DB must NOT open-and-migrate as a fresh empty wallet"
        );

        // a missing DB is NotFound (no CREATE), inherited from open_db
        let missing = dir.path().join("absent.db");
        assert!(matches!(
            open_db_migrated(&missing, &key, Network::Main, None),
            Err(WalletError::NotFound)
        ));
    }

    #[test]
    fn future_schema_version_sentinel_is_store_corrupt() {
        // Boundary test for WALLET_SCHEMA_VERSION (gate 7): a wallet provisioned
        // by a NEWER app (sentinel schema_version ahead of ours) must fail typed
        // StoreCorrupt on open — an older binary never silently opens a
        // forward-versioned store. This guards the `version != WALLET_SCHEMA_VERSION`
        // branch in open_db, which becomes load-bearing at the first schema bump.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        open_db(&path, &key).expect("opens at the current version");

        // forge a future schema version into the sentinel row
        {
            let conn = open_existing_keyed_connection(&path, &key).expect("open existing");
            conn.execute(
                "UPDATE wallet_provisioning SET schema_version = ?1 WHERE id = 1",
                [WALLET_SCHEMA_VERSION + 1],
            )
            .expect("bump the sentinel version");
        }
        assert!(
            matches!(open_db(&path, &key), Err(WalletError::StoreCorrupt)),
            "a forward-versioned store must NOT open on an older binary"
        );
    }

    #[test]
    fn truncated_or_garbage_db_is_typed_reject_never_fresh_wallet() {
        // §6.3 layer 3 (the truncation arm of the create-vs-open guard): a
        // provisioned DB later truncated to 0 bytes — which SQLite would open as
        // a FRESH empty DB — is caught by the MISSING sentinel and fails typed
        // StoreCorrupt, never a silent fresh wallet. (The store layer's marker is
        // layer 1; this proves the DB-level backstop independently.)
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        open_db(&path, &key).expect("opens before truncation");

        // truncate to 0 bytes (SQLite would treat this as a fresh empty DB)
        std::fs::write(&path, b"").expect("truncate to empty");
        assert!(
            matches!(open_db(&path, &key), Err(WalletError::StoreCorrupt)),
            "a 0-byte DB must NOT open as a fresh empty wallet"
        );

        // a short non-SQLCipher prefix → header decrypt fails → StoreCorrupt
        std::fs::write(&path, b"not a sqlcipher header at all").expect("garbage prefix");
        assert!(matches!(
            open_db(&path, &key),
            Err(WalletError::StoreCorrupt)
        ));
    }

    /// **§4aa OP-1 — DEFECT row, RED before T0-7.** A full disk during the
    /// FIRST OPEN of a new wallet is `DiskFull`, never "your wallet is
    /// destroyed".
    ///
    /// `init_wallet_db` runs the whole migration chain from an empty file on
    /// create AND on seed-restore — 0.22.0 added 19, one of which builds a
    /// UNIQUE index over an existing table. Every failure of that chain other
    /// than `SeedRequired`/`SeedNotRelevant` used to fold to `StoreCorrupt`,
    /// which the host renders as the red **"restore from your recovery
    /// phrase"** — told to a user whose wallet was never written, over a
    /// condition whose real remedy is "free some space and try again".
    ///
    /// The fault is induced by `PRAGMA max_page_count`, which makes SQLite
    /// raise `SQLITE_FULL` exactly as a full filesystem does. It is a
    /// PER-CONNECTION setting, so the row drives the shipped `migrate` directly
    /// on a connection it set the pragma on; `provision_db` opens its own
    /// connection and reaches `migrate` with a plain `?`, so the classification
    /// it inherits is the one measured here.
    ///
    /// The CONTROL is in the same row: the same connection WITHOUT the pragma
    /// migrates clean, so the `Err` above is the disk and not the fixture.
    #[test]
    fn disk_full_during_first_open_is_disk_full_not_corruption() {
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();

        // The control first, on its own file: the same call, no pragma, succeeds.
        let ok_path = dir.path().join("control.db");
        let ok_conn = open_keyed_connection(&ok_path, &key).expect("keyed connection");
        migrate(ok_conn, Network::Test, None).expect(
            "the control: with no page ceiling the whole migration chain runs clean, so the \
             failure below is the disk and not the fixture",
        );

        // A page ceiling low enough that the 19-migration chain overflows it — and
        // HIGH enough that it overflows INSIDE the chain rather than before it.
        //
        // **RAISED FROM 24 BY BIND-1-R, and the reason is a finding and not a
        // fixture nuisance.** [`migrate`] runs [`ensure_aux_tables`] on this same
        // connection BEFORE `init_wallet_db` (the property OP-2 below pins), so every
        // aux table this SDK adds takes pages out of the budget the migration chain
        // then has. BIND-1-R added one (`subtree_boundary_bound`), and at 24 the
        // chain ran out of pages inside upstream's own migrations-TABLE setup, which
        // is an `.expect("Migrations table setup succeeds.")`
        // (`zcash_client_sqlite-0.22.0 wallet/init.rs:610`) — so the run PANICKED
        // instead of returning the `DiskFull` this row exists to assert. The row went
        // red at the panic, which is the honest thing for it to have done, and the
        // ceiling is raised so it once again measures the classification rather than
        // the panic.
        //
        // **What that says about production, and it is owed rather than closed:**
        // there is an early window at the first open of a NEW wallet in which an
        // out-of-disk is an upstream PANIC and not a typed error, and how wide that
        // window is depends on how many pages this SDK's own aux tables have already
        // spent. Nothing in this crate can classify a panic; the FFI boundary turns
        // it into a crash. Reported by this comment and by the run record; not
        // repaired here, because the repair is upstream's or a pre-flight free-space
        // check, and neither belongs in a height-bind item.
        let full_path = dir.path().join("full.db");
        let conn = open_keyed_connection(&full_path, &key).expect("keyed connection");
        conn.pragma_update(None, "max_page_count", 32)
            .expect("set the page ceiling");
        let err = migrate(conn, Network::Test, None)
            .err()
            .expect("the migration chain must fail against a page ceiling this low");
        assert!(
            matches!(err, WalletError::DiskFull),
            "§4aa OP-1: an ENOSPC at the first open of a NEW wallet must reach the host as \
             DiskFull — 'free some space and retry' — never StoreCorrupt, which is the red \
             restore-from-your-seed remedy shown to a user whose wallet was never written. \
             Got {err:?}"
        );
    }

    /// **§4aa OP-2 — a CONTROL, and the contract was wrong about it.** A held
    /// lock at the first open of a new wallet is `StoreBusy`, never corruption —
    /// and it was ALREADY `StoreBusy` before T0-7.
    ///
    /// §4aa's Q2 assumed this reached the host as `StoreCorrupt` like the disk
    /// case did. MEASURED against the base (the blanket `Err(_) =>
    /// StoreCorrupt` arm restored, one mutant per run): this row stays GREEN,
    /// while OP-1 reds. The reason is ordering — [`migrate`] runs
    /// [`ensure_aux_tables`] on the same connection BEFORE `init_wallet_db`,
    /// and that path classifies through `map_aux_err`, which has routed
    /// `SQLITE_BUSY` to `StoreBusy` since #371. So the first write a held lock
    /// meets was already classified correctly, and the arm T0-7 fixed is not
    /// the arm this fixture reaches.
    ///
    /// It is kept, and kept honest, for two reasons: it is the property the
    /// host's `StoreBusy` arm depends on, and it pins the ordering — a refactor
    /// that moved `ensure_aux_tables` after `init_wallet_db`, or dropped its
    /// classification, would land the lock on the migration chain instead.
    /// **The residual T0-7 does close is the window between the two calls**: a
    /// lock acquired after the aux tables and before the chain lands inside
    /// `init_wallet_db`, and that error now takes the same classifier instead
    /// of the blanket corruption. No fixture reaches that window from outside
    /// the function, so it is stated here rather than asserted.
    ///
    /// The lock is a real `BEGIN EXCLUSIVE` on a second keyed connection to the
    /// same file, and it is RELEASED in the second half: the same connection
    /// then migrates clean, which is OP-4's resume property — the condition is
    /// transient and nothing about the wallet was damaged by meeting it.
    #[test]
    fn a_held_lock_at_first_open_is_store_busy_not_corruption() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();

        // Create the file (and its WAL) so a second connection can lock it.
        drop(open_keyed_connection(&path, &key).expect("create the file"));

        let blocker = open_keyed_connection(&path, &key).expect("the blocking connection");
        // A short busy timeout on the wallet connection so the row does not sit
        // out the production one; the CONDITION is what is under test, not the
        // wait. The blocker holds a write lock for the whole migration attempt.
        blocker
            .execute_batch("BEGIN EXCLUSIVE")
            .expect("hold an exclusive write lock");

        let conn = open_keyed_connection(&path, &key).expect("keyed connection");
        conn.busy_timeout(Duration::from_millis(50))
            .expect("a short busy timeout for the row");
        let err = migrate(conn, Network::Test, None)
            .err()
            .expect("the migration chain cannot write through an exclusive lock");
        assert!(
            matches!(err, WalletError::StoreBusy),
            "§4aa OP-2: a held lock at the first open of a NEW wallet must reach the host as \
             StoreBusy — another instance has this wallet, close it and retry — never \
             StoreCorrupt. Got {err:?}"
        );

        // OP-4, the resume: the cause clears and the SAME provisioning finishes.
        blocker.execute_batch("ROLLBACK").expect("release the lock");
        drop(blocker);
        let conn = open_keyed_connection(&path, &key).expect("keyed connection");
        migrate(conn, Network::Test, None).expect(
            "§4aa OP-4: once the lock is gone the same first open completes — the condition \
             was transient and nothing was damaged by meeting it",
        );
    }

    /// **§4aa OP-3 — CONTROL, green before and after T0-7.** A fault that IS the
    /// wallet being wrong is still `StoreCorrupt`.
    ///
    /// T0-7 narrows the blanket `StoreCorrupt` to the faults that deserve it, so
    /// this row is what stops the narrowing from becoming a fail-OPEN: a file
    /// that is not a wallet at all must still refuse, and must still refuse with
    /// the remedy that says so.
    #[test]
    fn a_corrupt_file_at_first_open_is_still_store_corrupt() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Test, None).expect("provision");

        // Not a SQLCipher file any more.
        std::fs::write(&path, b"not a sqlcipher header at all").expect("garbage prefix");
        assert!(
            matches!(
                open_db_migrated(&path, &key, Network::Test, None),
                Err(WalletError::StoreCorrupt)
            ),
            "a file that is not this wallet must still be StoreCorrupt — the narrowing must \
             not turn a real corruption into a retry prompt"
        );

        // And a 0-byte file, which SQLite would open as a FRESH empty DB.
        std::fs::write(&path, b"").expect("truncate to empty");
        assert!(matches!(
            open_db_migrated(&path, &key, Network::Test, None),
            Err(WalletError::StoreCorrupt)
        ));
    }

    /// **R12 §4.5 test 4 — the open path, on an EXISTING wallet.** A held lock at
    /// open is `StoreBusy` (onboarding's `alreadyOpen`: "another instance holds this
    /// wallet"), never `StoreCorrupt` (onboarding's `needsRecovery`). OP-2 above is
    /// a NEW wallet's first open, where `ensure_aux_tables` classified first; an
    /// existing wallet's open meets the lock earlier — at the key probe and the
    /// keyed-connection PRAGMAs (`open_existing_keyed_with_flags`), which mapped it
    /// blind. If this row is green before the sweep it is a CONTROL, not proof.
    ///
    /// The lock is a real one on the WAL file (`test_support::hold_the_wallet_file_locked`:
    /// EXCLUSIVE locking mode + a write transaction, which refuses readers). No
    /// test seam shortens the open's own busy wait — the opener builds its
    /// connection inside — so this row sits out the production wait once (≈5 s).
    #[test]
    fn opening_an_existing_wallet_under_a_held_lock_is_store_busy_not_corruption() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Test, None).expect("provision");

        let blocker = crate::test_support::hold_the_wallet_file_locked(&path, &key);
        match open_db_migrated(&path, &key, Network::Test, None).err() {
            Some(WalletError::StoreBusy) => {}
            other => panic!(
                "R12 §4.5 test 4: a held lock at the open of an EXISTING wallet must reach the \
                 host as StoreBusy (another instance has it — close and retry), never \
                 StoreCorrupt, which onboarding routes to restore-from-seed; got {other:?}"
            ),
        }
        drop(blocker);
        open_db_migrated(&path, &key, Network::Test, None)
            .expect("once the lock is gone the same wallet opens — nothing was damaged");
    }

    /// **R12 §4.1 / §4.5 test 8 (a).** `SQLITE_LOCKED` — a same-connection or
    /// shared-cache conflict — is as transient as BUSY and classifies `StoreBusy`,
    /// at the classifier and through the by-value door every swept `rusqlite` site
    /// uses. It fell to the fail-closed `StoreCorrupt` default before (the security
    /// review). Synthetic on purpose: this pins the CLASSIFIER's table, one row per
    /// code; the doors are pinned under a real lock elsewhere.
    #[test]
    fn sqlite_locked_is_store_busy_at_the_classifier() {
        for code in [
            rusqlite::ffi::SQLITE_LOCKED,
            rusqlite::ffi::SQLITE_LOCKED_SHAREDCACHE,
        ] {
            let locked = || rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
            assert!(
                matches!(classify_sqlite_error(&locked()), WalletError::StoreBusy),
                "SQLITE_LOCKED (extended {code}) is the transient StoreBusy, not corruption; got \
                 {:?}",
                classify_sqlite_error(&locked())
            );
            assert!(
                matches!(map_aux_err(locked()), WalletError::StoreBusy),
                "and the by-value door agrees (extended {code})"
            );
        }
    }

    #[test]
    fn copy_aux_tables_carries_durable_aux_across_a_keyed_rebuild_leaving_src_untouched() {
        // ADR-0534 (the rescan aux copy): ATTACH the OLD wallet.db under the SAME key and
        // INSERT…SELECT the durable aux tables into a freshly-provisioned, account-less DB —
        // money intents + the never-recycle refund counter survive, and the SOURCE is only
        // READ (never written), so the old DB is intact for the crash-atomic rename.
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("wallet.db");
        let dst = dir.path().join("wallet.db.rescan-tmp");
        let key = WalletDbKey::generate();

        // OLD wallet.db: seed a queued-send intent + reserve a refund index (counter → 2).
        provision_db(&src, &key, Network::Main, None).expect("provision src");
        {
            let mut conn = open_existing_keyed_connection(&src, &key).expect("open src");
            crate::intent_store::enqueue(&mut conn, "zcash:carry-me", 0, None, None)
                .expect("enqueue");
            assert_eq!(
                crate::refund_index::reserve_next_index(&mut conn).expect("reserve"),
                1,
                "first reserve is the floor"
            );
        }

        // A fresh, account-less destination (the rescan temp DB), then copy the aux across.
        provision_db(&dst, &key, Network::Main, None).expect("provision dst");
        copy_aux_tables(&dst, &src, &key).expect("copy aux");

        // The durable aux landed in the destination.
        let mut dconn = open_existing_keyed_connection(&dst, &key).expect("open dst");
        let queued = crate::intent_store::list_queued(&dconn).expect("list dst");
        assert_eq!(queued.len(), 1, "the queued send intent was copied across");
        assert_eq!(queued[0].uri, "zcash:carry-me");
        // The refund counter (next_index = 2) was copied: the next reserve returns 2, never 1.
        assert_eq!(
            crate::refund_index::reserve_next_index(&mut dconn).expect("reserve dst"),
            2,
            "a reset that lost the refund counter would recycle index 1 — privacy break",
        );

        // The SOURCE is unmodified by the copy (ATTACH-read only).
        let sconn = open_existing_keyed_connection(&src, &key).expect("reopen src");
        assert_eq!(
            crate::intent_store::list_queued(&sconn)
                .expect("list src")
                .len(),
            1,
            "copy_aux_tables must only READ the source",
        );
    }

    #[test]
    fn copy_aux_tables_attaches_src_read_only_through_a_special_char_path() {
        // ADR-0534 hardening: the source is ATTACHed via a `file:…?mode=ro` URI, so (a) a
        // db_dir containing URI-structural bytes (`%`/`#`/`?`/space) must still copy
        // correctly (the percent-encoding round-trips), and (b) the source file is
        // byte-for-byte UNCHANGED by the copy (read-only attach — not just "we wrote a
        // SELECT"). Both are load-bearing for the crash-atomic rename.
        let dir = tempdir().expect("tempdir");
        // A real-world-hostile dir name: percent, hash, question-mark, and a space.
        let weird = dir.path().join("re%ld #weird?dir");
        std::fs::create_dir(&weird).expect("create weird dir");
        let src = weird.join("wallet.db");
        let dst = weird.join("wallet.db.rescan-tmp");
        let key = WalletDbKey::generate();

        provision_db(&src, &key, Network::Main, None).expect("provision src");
        {
            let mut conn = open_existing_keyed_connection(&src, &key).expect("open src");
            crate::intent_store::enqueue(&mut conn, "zcash:weird-path", 0, None, None)
                .expect("enqueue");
        }
        // Snapshot the source bytes AFTER all writes + close, BEFORE the copy.
        let src_before = std::fs::read(&src).expect("read src before");

        provision_db(&dst, &key, Network::Main, None).expect("provision dst");
        copy_aux_tables(&dst, &src, &key).expect("copy across a URI-structural path");

        let dconn = open_existing_keyed_connection(&dst, &key).expect("open dst");
        assert_eq!(
            crate::intent_store::list_queued(&dconn)
                .expect("list dst")
                .len(),
            1,
            "the aux copied correctly even through a `%`/`#`/`?`/space path",
        );
        // The read-only ATTACH guarantee: the source file is byte-identical.
        assert_eq!(
            std::fs::read(&src).expect("read src after"),
            src_before,
            "a read-only (mode=ro) ATTACH must leave the source byte-for-byte unchanged",
        );
    }

    #[test]
    fn copy_aux_tables_rejects_a_drifted_aux_schema_never_mis_maps() {
        // ADR-0534 hardening (`assert_columns_match`): the columnless `INSERT … SELECT *`
        // is sound ONLY if dst and src agree on the column set+order. If they drift (an
        // additive `ALTER` on one side the other lacks), the copy must fail typed
        // `StoreCorrupt` — never silently shift money columns by one position.
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("wallet.db");
        let dst = dir.path().join("wallet.db.rescan-tmp");
        let key = WalletDbKey::generate();

        // Direction 1 — DESTINATION has a column the source lacks.
        provision_db(&src, &key, Network::Main, None).expect("provision src");
        provision_db(&dst, &key, Network::Main, None).expect("provision dst");
        {
            let conn = open_existing_keyed_connection(&dst, &key).expect("open dst");
            conn.execute_batch("ALTER TABLE queued_send_intent ADD COLUMN drift_col INTEGER;")
                .expect("alter dst");
        }
        assert!(
            matches!(
                copy_aux_tables(&dst, &src, &key),
                Err(WalletError::StoreCorrupt)
            ),
            "dst-has-extra-column drift must abort the copy typed, never mis-map silently",
        );

        // Direction 2 — SOURCE has a column the destination lacks (the real-world hazard:
        // an old wallet.db a newer binary already ALTER'd before the rescan). Fresh DBs so
        // the prior drift can't mask this.
        let src2 = dir.path().join("wallet2.db");
        let dst2 = dir.path().join("wallet2.db.rescan-tmp");
        provision_db(&src2, &key, Network::Main, None).expect("provision src2");
        provision_db(&dst2, &key, Network::Main, None).expect("provision dst2");
        {
            let conn = open_existing_keyed_connection(&src2, &key).expect("open src2");
            conn.execute_batch("ALTER TABLE queued_send_intent ADD COLUMN drift_col INTEGER;")
                .expect("alter src2");
        }
        assert!(
            matches!(
                copy_aux_tables(&dst2, &src2, &key),
                Err(WalletError::StoreCorrupt)
            ),
            "src-has-extra-column drift must ALSO abort the copy typed (the asymmetric hazard)",
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_aux_tables_rejects_a_non_utf8_src_path() {
        // ADR-0534 hardening: a non-UTF-8 source path can't be rendered into the ATTACH
        // URI, so it is rejected typed BEFORE any DB is opened — never lossily mangled.
        use std::os::unix::ffi::OsStrExt;
        let dir = tempdir().expect("tempdir");
        let dst = dir.path().join("wallet.db.rescan-tmp");
        let key = WalletDbKey::generate();
        provision_db(&dst, &key, Network::Main, None).expect("provision dst");

        let bad = Path::new(std::ffi::OsStr::from_bytes(b"/tmp/\xff\xfe-not-utf8.db"));
        assert!(
            matches!(
                copy_aux_tables(&dst, bad, &key),
                Err(WalletError::StoreCorrupt)
            ),
            "a non-UTF-8 src path must be rejected typed, before any DB access",
        );
    }

    #[test]
    fn aux_tables_preserved_matches_what_migrate_creates() {
        // ADR-0534 MONEY-SAFETY guard: `AUX_TABLES_PRESERVED` (the set the rescan copies
        // across a data-DB rebuild) MUST equal EXACTLY the aux tables `ensure_aux_tables`
        // creates. A future aux store added to `ensure_aux_tables`/`migrate` but forgotten
        // in `AUX_TABLES_PRESERVED` would silently DROP its money state on every rescan; a
        // name listed but never created would make `copy_aux_tables` fail. Either drift
        // fails this test loudly, so the two declarations can never rot apart unnoticed.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("aux-only.db");
        let key = WalletDbKey::generate();
        // A bare keyed DB with ONLY our aux schema — no `init_wallet_db`, so NONE of the
        // librustzcash tables are present and `sqlite_master` lists exactly the aux tables
        // under test (indexes are `type='index'`, excluded by the filter).
        let conn = open_keyed_connection(&path, &key).expect("open keyed");
        ensure_aux_tables(&conn).expect("ensure aux tables");

        let mut created: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            )
            .expect("prepare")
            .query_map([], |r| r.get::<_, String>(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("collect table names");
        created.sort();

        let mut preserved: Vec<String> = AUX_TABLES_PRESERVED
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        preserved.sort();

        assert_eq!(
            created, preserved,
            "AUX_TABLES_PRESERVED must list exactly the aux tables ensure_aux_tables creates \
             — a mismatch means a rescan would silently drop (or fail to copy) money state",
        );
    }

    #[test]
    fn migrate_creates_the_queued_send_intent_table_idempotently() {
        // §8 (inc-2d-3-b-i): `provision_db` routes through `migrate`, which ensures OUR
        // aux `queued_send_intent` table on the SAME keyed connection. Both provision and
        // a re-provision (the §6.3 repair-resume idempotency) leave the table present —
        // no `WALLET_SCHEMA_VERSION` bump needed because `migrate` runs on every open.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Test, None).expect("provision");
        provision_db(&path, &key, Network::Test, None).expect("re-provision is idempotent");
        let conn = open_existing_keyed_connection(&path, &key).expect("open");
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='queued_send_intent'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(exists, "migrate must create the queued-send intent table");
    }

    #[test]
    fn concurrent_keyed_connections_serialize_intent_writes_without_busy_failure() {
        // §8 (inc-2d-3-b-i, MOBILE/concurrency — testing-patterns mandates a concurrent
        // test for shared state): the wallet DB has TWO keyed connections to one file
        // (the engine + the aux intent store) — in VERIFIED WAL mode since W-swap-4-a-4
        // (rollback-era wording fixed W-swap-4-a-5) — and WAL allows only ONE writer at
        // a time, so their write transactions still contend. `busy_timeout` (set by the
        // opener) makes them SERIALIZE rather than spuriously fail `SQLITE_BUSY`. N
        // threads, each on its OWN keyed connection, enqueue concurrently after a
        // barrier (max contention); ALL rows must land — no lost write, no error. A
        // `busy_timeout` of 0 would lose writes here. Each write goes through
        // `with_aux_busy_retry`, as production's do, so a loaded test machine
        // cannot fail the claim while the opener's own wait stays the one under test.
        use std::sync::{Arc, Barrier};
        let dir = tempdir().expect("tempdir");
        let path = Arc::new(dir.path().join("wallet.db"));
        // WalletDbKey is deliberately non-Clone (keys are never duplicated) — share the
        // ONE key across threads via Arc so every connection opens the same encrypted DB.
        let key = Arc::new(WalletDbKey::generate());
        provision_db(&path, &key, Network::Main, None).expect("provision");

        const THREADS: usize = 8;
        let barrier = Arc::new(Barrier::new(THREADS));
        let handles: Vec<_> = (0..THREADS)
            .map(|t| {
                let (path, key, barrier) =
                    (Arc::clone(&path), Arc::clone(&key), Arc::clone(&barrier));
                std::thread::spawn(move || {
                    let mut conn = open_existing_keyed_connection(&path, &key).expect("keyed open");
                    barrier.wait();
                    with_aux_busy_retry(|| {
                        crate::intent_store::enqueue(
                            &mut conn,
                            &format!("zcash:t{t}"),
                            t as i64,
                            None,
                            None,
                        )
                    })
                    .expect("enqueue must not fail under cross-connection contention");
                })
            })
            .collect();
        for h in handles {
            h.join().expect("thread");
        }

        let conn = open_existing_keyed_connection(&path, &key).expect("open");
        let n = crate::intent_store::list_queued(&conn).expect("list").len();
        assert_eq!(
            n, THREADS,
            "every concurrent intent write committed (none lost to BUSY)"
        );
    }

    #[test]
    fn aux_immediate_write_commits_while_an_engine_style_deferred_reader_holds_shared() {
        // §8 (inc-2d-3-b-i, MOBILE/concurrency — the REAL production topology the aux-vs-aux
        // test does NOT exercise): the engine writes via rusqlite-default DEFERRED (SHARED
        // first, upgraded later), the aux intent store via `BEGIN IMMEDIATE` (RESERVED
        // upfront). A `queue_send` fired while a scan batch holds a DEFERRED read MUST wait
        // out the lock and COMMIT — never spuriously fail `SQLITE_BUSY` (which `map_db_err`
        // would surface as `StoreCorrupt`, a FALSE corruption on a healthy wallet) and never
        // deadlock. RESERVED coexists with the reader's SHARED; the `busy_timeout` covers the
        // COMMIT's brief EXCLUSIVE wait. Deterministic: a barrier sequences the held read
        // before the writer attempts, and the read releases right after — bounded ≪ timeout.
        use std::sync::{Arc, Barrier};
        let dir = tempdir().expect("tempdir");
        let path = Arc::new(dir.path().join("wallet.db"));
        let key = Arc::new(WalletDbKey::generate());
        provision_db(&path, &key, Network::Main, None).expect("provision");

        let barrier = Arc::new(Barrier::new(2));
        let writer = {
            let (path, key, barrier) = (Arc::clone(&path), Arc::clone(&key), Arc::clone(&barrier));
            std::thread::spawn(move || {
                let mut aux = open_existing_keyed_connection(&path, &key).expect("aux open");
                barrier.wait(); // attempt only after the engine-role reader holds SHARED
                crate::intent_store::enqueue(&mut aux, "zcash:contended", 0, None, None)
                    .expect("aux IMMEDIATE write waits out the reader — never BUSY/StoreCorrupt")
            })
        };
        // Engine-role: a DEFERRED transaction holding a SHARED read lock across the aux write.
        let mut engine = open_existing_keyed_connection(&path, &key).expect("engine open");
        let txn = engine
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .expect("deferred");
        txn.query_row("SELECT count(*) FROM queued_send_intent", [], |_| Ok(()))
            .expect("hold SHARED");
        barrier.wait(); // the aux writer now races our held SHARED
        txn.commit().expect("release SHARED"); // let the aux COMMIT proceed
        let id = writer.join().expect("writer thread");
        assert!(
            id.value() > 0,
            "the contended aux write committed a real row"
        );

        let conn = open_existing_keyed_connection(&path, &key).expect("verify open");
        assert_eq!(
            crate::intent_store::list_queued(&conn).expect("list").len(),
            1,
            "the intent landed despite a concurrent engine-style DEFERRED reader (no deadlock)",
        );
    }

    #[test]
    fn keyed_connections_open_in_wal_mode() {
        // W-swap-4-a-4: BOTH openers set + verify WAL (persistent in the DB header), and
        // a legacy rollback-mode wallet FLIPS on its first open — the load-bearing arm:
        // silently staying in rollback would re-arm the mislabeled-BUSY class.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");

        let mode = |conn: &Connection| -> String {
            conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .expect("journal_mode")
        };
        {
            let conn = open_existing_keyed_connection(&path, &key).expect("open");
            assert!(mode(&conn).eq_ignore_ascii_case("wal"), "opener yields WAL");
            // Regress to the legacy rollback mode (what a pre-4-a-4 wallet file is in).
            let back: String = conn
                .query_row("PRAGMA journal_mode = DELETE", [], |r| r.get(0))
                .expect("regress to rollback");
            assert!(
                back.eq_ignore_ascii_case("delete"),
                "regressed for the flip arm"
            );
        }
        let conn = open_existing_keyed_connection(&path, &key).expect("reopen");
        assert!(
            mode(&conn).eq_ignore_ascii_case("wal"),
            "a legacy rollback-mode wallet flips to WAL on its next open",
        );
    }

    #[test]
    fn aux_commit_succeeds_under_an_engine_read_window_on_wal() {
        // W-swap-4-a-4 — the device blocker, reproduced at store level: the ENGINE
        // connection holds a LONG read window (a single `progress_snapshot` measured
        // ~6.5 s on device) while an aux `IMMEDIATE` txn COMMITs. Under rollback-journal
        // the reader's SHARED lock blocked the COMMIT's EXCLUSIVE past the 5 s
        // `busy_timeout` — the `SQLITE_BUSY` that was mislabeled `StoreCorrupt` ("the
        // wallet couldn't safely record this swap", 2 of 3 live executes). Under WAL the
        // commit lands WHILE the read transaction stays open: no release, no wait.
        // Single-threaded and deterministic — the read txn is held across the write and
        // released only after it committed.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");

        let mut engine = open_existing_keyed_connection(&path, &key).expect("engine open");
        let read_txn = engine
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .expect("deferred read");
        read_txn
            .query_row("SELECT count(*) FROM queued_send_intent", [], |_| Ok(()))
            .expect("materialize the read snapshot (the engine's long read window)");

        let mut aux = open_existing_keyed_connection(&path, &key).expect("aux open");
        let id = crate::intent_store::enqueue(&mut aux, "zcash:walproof", 0, None, None)
            .expect("the aux COMMIT lands INSIDE the open engine read window under WAL");
        assert!(id.value() > 0, "a real row committed");

        // The engine read window is STILL open — release it only now, then verify.
        read_txn.commit().expect("release the read window");
        let conn = open_existing_keyed_connection(&path, &key).expect("verify open");
        assert_eq!(
            crate::intent_store::list_queued(&conn).expect("list").len(),
            1,
            "the intent is durable — committed under a concurrent reader, never lost",
        );
    }

    #[test]
    fn verify_wal_folded_passes_clean_and_refuses_a_surviving_wal() {
        // review fold (the 3-review converged HIGH): the rescan rename guard —
        // a non-empty `-wal` beside the file means a silently-failed close checkpoint
        // left committed data OUT of the main file; renaming that main file would
        // install a wallet missing its aux money tables. Missing or zero-length WALs
        // pass (a clean close unlinks it; a TRUNCATE checkpoint leaves it empty).
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        std::fs::write(&path, b"x").expect("main file");
        assert!(verify_wal_folded(&path).is_ok(), "no sidecar passes");
        let wal = with_suffix(&path, "-wal");
        std::fs::write(&wal, b"").expect("empty wal");
        assert!(
            verify_wal_folded(&path).is_ok(),
            "a zero-length (truncated) wal passes"
        );
        std::fs::write(&wal, b"frames").expect("non-empty wal");
        assert!(
            matches!(verify_wal_folded(&path), Err(WalletError::StoreCorrupt)),
            "a surviving non-empty wal fails closed"
        );
    }

    #[test]
    fn cache_connection_opens_rollback_and_flips_a_wal_era_cache_back() {
        // W-swap-4-a-5: the block cache is SINGLE-connection — WAL buys no
        // concurrency there and costs 2× write amplification of every compact-
        // block byte, so the cache opener pins the ROLLBACK journal. A cache
        // converted inside the 4-a-4 window (the shared-opener WAL flip) flips
        // BACK on first open, KEEPING its rows (no spurious re-download) and
        // unlinking the WAL sidecar.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("block-cache.db");
        let key = WalletDbKey::generate();
        let mode = |conn: &Connection| -> String {
            conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .expect("journal_mode")
        };

        // Fresh create: rollback from the first byte.
        {
            let conn = open_keyed_cache_connection(&path, &key).expect("fresh cache open");
            assert!(
                mode(&conn).eq_ignore_ascii_case("delete"),
                "a fresh cache is rollback"
            );
            conn.execute_batch(
                "CREATE TABLE cached_block (\
                    height INTEGER PRIMARY KEY NOT NULL, \
                    bytes  BLOB    NOT NULL)",
            )
            .expect("schema");
        }
        // The 4-a-4 window: the WALLET opener converts the file to WAL.
        {
            let conn = open_keyed_connection(&path, &key).expect("wal-era open");
            assert!(mode(&conn).eq_ignore_ascii_case("wal"), "wal-era cache");
            conn.execute(
                "INSERT INTO cached_block (height, bytes) VALUES (1, x'aa')",
                [],
            )
            .expect("a cached row from the wal era");
        }
        // The cache opener flips it back; the row survives.
        let conn = open_keyed_cache_connection(&path, &key).expect("cache reopen");
        assert!(
            mode(&conn).eq_ignore_ascii_case("delete"),
            "a wal-era cache flips back to rollback on first open"
        );
        let n: i64 = conn
            .query_row("SELECT count(*) FROM cached_block", [], |r| r.get(0))
            .expect("count");
        assert_eq!(n, 1, "the wal-era row survived the flip");
        assert!(
            !with_suffix(&path, "-wal").exists(),
            "the flip checkpointed and unlinked the wal sidecar"
        );
    }

    #[test]
    fn keyed_connections_pin_synchronous_full() {
        // W-swap-4-a-5: durability is PINNED, not assumed — a host-linked SQLite
        // built with `SQLITE_DEFAULT_WAL_SYNCHRONOUS = NORMAL` would otherwise
        // silently downgrade money commits under WAL to a power-cut-lossy fsync
        // policy. FULL echoes `2`.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        let level = |conn: &Connection| -> i64 {
            conn.query_row("PRAGMA synchronous", [], |r| r.get(0))
                .expect("synchronous")
        };
        {
            let conn = open_keyed_connection(&path, &key).expect("create opener");
            assert_eq!(level(&conn), 2, "the create opener pins FULL");
        }
        let conn = open_existing_keyed_connection(&path, &key).expect("existing opener");
        assert_eq!(level(&conn), 2, "the existing opener pins FULL");
        let cache =
            open_keyed_cache_connection(&dir.path().join("block-cache.db"), &key).expect("cache");
        assert_eq!(level(&cache), 2, "the cache opener pins FULL");
    }

    /// Crypto audit B1 (ADR-0568): an erased custom-server key is GONE from the
    /// file's bytes once the aux connection runs `secure_delete` and the WAL is
    /// folded and truncated — and the control (the same steps WITHOUT
    /// `ensure_secure_delete`) still finds it, so the row can fail. Unkeyed
    /// on purpose: on a SQLCipher file the bytes are ciphertext either way and
    /// a byte scan proves nothing; plain pages show what SQLCipher would
    /// encrypt.
    #[test]
    fn the_aux_connection_zeroes_an_erased_key() {
        // 4096 bytes, the largest key the door admits: with the URL and the
        // bound URL the row passes a table leaf's local-payload maximum
        // (usable size − 35 = 4061 on a 4096-byte page), so the key reaches an
        // OVERFLOW page and the erase frees it. MEASURED: under WAL —
        // the only journal mode the wallet runs — `FAST` zeroed that freed
        // page as well as `ON` did, with and without a pre-existing freelist
        // trunk, so this row cannot tell the two apart and its FAST mutant
        // survives. `ON` stays because it zeroes freed pages whatever the
        // journal mode or freelist shape; it is the conservative setting, not
        // one this test proves necessary.
        let marker = "ERASE-ME-MARKER-".repeat(256);
        let leaves_marker = |secure: bool| -> bool {
            let dir = tempdir().expect("tempdir");
            let path = dir.path().join("aux.db");
            {
                let conn = Connection::open(&path).expect("open");
                conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get::<_, String>(0))
                    .expect("wal");
                if secure {
                    ensure_secure_delete(&conn).expect("secure_delete");
                }
                crate::sync_server::ensure_table(&conn).expect("table");
                let key =
                    crate::config::EndpointAuth::new("x-api-key", marker.as_str()).expect("auth");
                crate::sync_server::write(
                    &conn,
                    &crate::sync_server::SyncServerChoice::Custom {
                        endpoint: crate::config::LightServerEndpoint::new(
                            "https://mine.example:443",
                        )
                        .expect("endpoint"),
                        key: Some(key),
                    },
                    1,
                )
                .expect("keyed write");
                checkpoint_wal_truncate(&conn).expect("fold the keyed write");
                crate::sync_server::write(
                    &conn,
                    &crate::sync_server::SyncServerChoice::Predefined(
                        crate::sync_server::SyncServerId::new("zec-rocks").expect("id"),
                    ),
                    2,
                )
                .expect("erase");
                checkpoint_wal_truncate(&conn).expect("fold the erase");
                // Scanned WHILE the connection is open (crypto audit, the
                // built diff): the last close would checkpoint and delete the
                // WAL by itself, so a scan after the drop could not tell
                // whether the explicit fold ran.
                let mut found = false;
                for file in [path.clone(), with_suffix(&path, "-wal")] {
                    if let Ok(bytes) = std::fs::read(&file) {
                        found |= bytes
                            .windows(marker.len() / 4)
                            .any(|w| w == &marker.as_bytes()[..marker.len() / 4]);
                    }
                }
                found
            }
        };
        assert!(
            leaves_marker(false),
            "the control must find the erased key without secure_delete, or this row proves nothing"
        );
        assert!(
            !leaves_marker(true),
            "secure_delete + the truncating fold must leave no byte of an erased key"
        );
    }

    #[test]
    fn a_keyed_connection_zeroes_freed_pages_by_sqlciphers_default() {
        // B1, MEASURED at the build (the design had assumed it OFF and planned
        // `the_shared_opener_leaves_secure_delete_off`, which went red): SQLCipher
        // turns `secure_delete` on for every KEYED handle in
        // `sqlcipherCodecAttach`, so the engine connection zeroes freed pages
        // too. Pinned, so a link that loses the default is seen here — and
        // `ensure_secure_delete` on the aux handle is then the line that fails
        // closed. The control: an UNKEYED handle reads the plain-SQLite 0.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        drop(open_keyed_connection(&path, &key).expect("create"));
        let conn = open_existing_keyed_connection(&path, &key).expect("open");
        let pragma = |c: &Connection| -> i64 {
            c.query_row("PRAGMA secure_delete", [], |r| r.get(0))
                .expect("pragma")
        };
        assert_eq!(pragma(&conn), 1, "a keyed handle has SQLCipher's default");
        let plain = Connection::open(dir.path().join("plain.db")).expect("plain");
        assert_eq!(pragma(&plain), 0, "the control: an unkeyed handle does not");
    }

    #[test]
    fn checkpoint_wal_truncate_folds_the_wal_and_the_backstop_passes() {
        // W-swap-4-a-5: the HONEST fold — commits land in the WAL; the explicit
        // TRUNCATE checkpoint moves them into the main file and zeroes the WAL,
        // so the rename-side backstop passes without relying on the SILENT
        // last-close checkpoint (whose failure rusqlite's Drop discards).
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        let conn = open_keyed_connection(&path, &key).expect("open");
        conn.execute_batch("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (7);")
            .expect("commit into the WAL");
        let wal = with_suffix(&path, "-wal");
        assert!(
            std::fs::metadata(&wal).expect("wal exists").len() > 0,
            "the commit lives in the WAL before the fold"
        );
        checkpoint_wal_truncate(&conn).expect("fold");
        assert_eq!(
            std::fs::metadata(&wal).expect("wal still present").len(),
            0,
            "TRUNCATE zeroed the WAL"
        );
        verify_wal_folded(&path).expect("the backstop passes after the honest fold");
    }

    #[test]
    fn checkpoint_wal_truncate_types_reader_interference_as_busy() {
        // W-swap-4-a-5: the pragma reports a reader pinning WAL frames as a
        // `busy` result — typed the transient `StoreBusy`, NEVER a false
        // corruption (the sin 4-a-4 retired must not respawn at the fold).
        // Unreachable at the rescan call sites (the fold runs on the LAST-
        // standing connection) — pinned so the taxonomy can't rot.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        let writer = open_keyed_connection(&path, &key).expect("writer open");
        writer
            .execute_batch("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1);")
            .expect("frames");
        // Keep the test fast: the checkpoint waits for readers via the busy
        // handler — shorten it far below the production 5 s.
        writer
            .busy_timeout(Duration::from_millis(25))
            .expect("short test wait");

        let mut reader = open_existing_keyed_connection(&path, &key).expect("reader open");
        let read_txn = reader
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .expect("read txn");
        read_txn
            .query_row("SELECT count(*) FROM t", [], |_| Ok(()))
            .expect("materialize a snapshot that pins WAL frames");
        // Advance the WAL past the reader's mark so the fold cannot complete.
        writer
            .execute("INSERT INTO t VALUES (2)", [])
            .expect("frames past the reader's mark");

        assert!(
            matches!(
                checkpoint_wal_truncate(&writer),
                Err(WalletError::StoreBusy)
            ),
            "a blocking reader types StoreBusy, not corruption"
        );
        read_txn.commit().expect("release the reader");
        checkpoint_wal_truncate(&writer).expect("folds once the reader is gone");
    }

    #[test]
    fn wal_frames_at_rest_carry_no_plaintext() {
        // review fold (crypto NIT): the at-rest surface now includes WAL frames
        // that persist across a crash. Pin that SQLCipher encrypts them: write a
        // needle while a SECOND connection holds an open read snapshot (which blocks
        // the writer's close-time checkpoint, so the `-wal` SURVIVES the close), then
        // scan the raw wal bytes for the needle.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");

        let mut holder = open_existing_keyed_connection(&path, &key).expect("holder open");
        let read_txn = holder
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .expect("read txn");
        read_txn
            .query_row("SELECT count(*) FROM queued_send_intent", [], |_| Ok(()))
            .expect("hold the read snapshot");

        let needle = "zcash:NEEDLE-plaintext-canary";
        {
            let mut writer = open_existing_keyed_connection(&path, &key).expect("writer open");
            crate::intent_store::enqueue(&mut writer, needle, 0, None, None).expect("enqueue");
            // The writer drops here; the held snapshot blocks its close checkpoint,
            // so the frames stay in the wal.
        }
        let wal_bytes =
            std::fs::read(with_suffix(&path, "-wal")).expect("the wal survived the close");
        assert!(!wal_bytes.is_empty(), "frames are present in the wal");
        assert!(
            !wal_bytes
                .windows(needle.len())
                .any(|w| w == needle.as_bytes()),
            "SQLCipher encrypts WAL frames — no plaintext needle at rest"
        );
        read_txn.commit().expect("release the snapshot");
    }

    /// Fabricate the torn mid-txn state a power cut leaves on a rollback-mode
    /// cache: shrink `cache_size` so a multi-page UPDATE SPILLS rewritten pages
    /// into the MAIN file mid-txn (the journal already holds their
    /// before-images — `synchronous = FULL` syncs it before any main write),
    /// then snapshot db + journal while the txn is still open (the copy IS the
    /// power cut). Returns the copies' db path.
    fn fabricate_torn_txn_cache(needle: &[u8], key: &WalletDbKey) -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("block-cache.db");
        {
            let conn = open_keyed_cache_connection(&path, key).expect("create");
            conn.execute_batch(
                "CREATE TABLE cached_block (\
                    height INTEGER PRIMARY KEY NOT NULL, \
                    bytes  BLOB    NOT NULL)",
            )
            .expect("schema");
            let mut insert = conn
                .prepare("INSERT INTO cached_block (height, bytes) VALUES (?1, ?2)")
                .expect("prepare");
            for h in 0i64..40 {
                let mut bytes = vec![h as u8; 4000];
                bytes[..needle.len().min(4000)].copy_from_slice(needle);
                insert.execute(rusqlite::params![h, bytes]).expect("row");
            }
            drop(insert);
        }
        let conn = open_keyed_cache_connection(&path, key).expect("reopen");
        // Two pages of cache against a ~40-page UPDATE ⇒ guaranteed mid-txn spill.
        conn.execute_batch("PRAGMA cache_size = 2; BEGIN;")
            .expect("tiny cache + open txn");
        conn.execute("UPDATE cached_block SET bytes = zeroblob(4000)", [])
            .expect("the torn overwrite");
        let journal = with_suffix(&path, "-journal");
        // Multi-PAGE coverage, not merely non-empty (→#374 crypto fold):
        // a future page-size/cache default change must not quietly shrink the
        // canary's scanned surface toward a header-only journal.
        assert!(
            std::fs::metadata(&journal)
                .expect("hot journal on disk")
                .len()
                > 3 * 4096,
            "the spill journaled multiple pages of before-images mid-txn"
        );
        // The power cut: copy db + journal while the txn is open, then abandon
        // the original (its rollback is irrelevant to the copies).
        let cut = tempdir().expect("cut dir");
        let cut_db = cut.path().join("block-cache.db");
        std::fs::copy(&path, &cut_db).expect("copy main");
        std::fs::copy(&journal, with_suffix(&cut_db, "-journal")).expect("copy journal");
        (cut, cut_db)
    }

    #[test]
    fn cache_opener_replays_a_torn_txn_hot_journal() {
        // W-swap-4-a-6 (reliability — validated empirically, pinned here):
        // crash-atomicity is the ONE property the disposable cache contract
        // leans on (module doc). A power cut mid-txn leaves main-file pages
        // already rewritten and a HOT rollback journal holding their
        // before-images; the keyed cache opener must roll the txn back — every
        // row reads its ORIGINAL bytes, never a torn mix.
        let key = WalletDbKey::generate();
        let (_cut, cut_db) = fabricate_torn_txn_cache(b"torn-txn-fixture", &key);

        let conn = open_keyed_cache_connection(&cut_db, &key).expect("open replays the journal");
        let (n, zeroed): (i64, i64) = conn
            .query_row(
                "SELECT count(*), sum(bytes = zeroblob(4000)) FROM cached_block",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("post-recovery read");
        assert_eq!(n, 40, "every row survived");
        assert_eq!(
            zeroed, 0,
            "no torn page: the mid-txn overwrite rolled back whole"
        );
        assert_eq!(
            std::fs::metadata(with_suffix(&cut_db, "-journal"))
                .map(|m| m.len())
                .unwrap_or(0),
            0,
            "the hot journal was consumed by the rollback"
        );
    }

    #[test]
    fn journal_pages_at_rest_carry_no_plaintext() {
        // W-swap-4-a-6 (security NIT-2) — the `-journal` sibling of
        // `wal_frames_at_rest_carry_no_plaintext`: a rollback journal holds
        // BEFORE-images of SQLCipher pages, and those persist across a mid-txn
        // power cut. Pin that they are ciphertext: plant a needle, overwrite it
        // inside a spilling txn, and scan the surviving journal's raw bytes.
        // (Scope, like the WAL sibling: USER DATA is what's pinned — journal
        // header fields and per-record page numbers are plaintext metadata by
        // design and carry no note/tx content.)
        let needle = b"zcash:NEEDLE-journal-canary";
        let key = WalletDbKey::generate();
        let (_cut, cut_db) = fabricate_torn_txn_cache(needle, &key);

        let journal_bytes =
            std::fs::read(with_suffix(&cut_db, "-journal")).expect("the journal survived the cut");
        assert!(!journal_bytes.is_empty(), "before-images are present");
        assert!(
            !journal_bytes
                .windows(needle.len())
                .any(|w| w == needle.as_slice()),
            "SQLCipher encrypts journal before-images — no plaintext needle at rest"
        );
    }

    #[test]
    fn remove_path_if_present_covers_files_dirs_symlinks_and_absence() {
        // →#374 review fold (all three reviewers): the shared tolerant unlink
        // must remove a FILE, tolerate ABSENCE, remove a planted DIRECTORY
        // (parity with destroy's sweep — the file-only arm was a fresh brick
        // shape), and unlink a SYMLINK itself, never its referent.
        let dir = tempdir().expect("tempdir");

        let file = dir.path().join("a.tmp");
        std::fs::write(&file, b"x").expect("file");
        remove_path_if_present(&file).expect("file removed");
        assert!(!file.exists());
        remove_path_if_present(&file).expect("absence tolerated");

        let planted = dir.path().join("b.tmp");
        std::fs::create_dir(&planted).expect("planted dir");
        std::fs::write(planted.join("child"), b"x").expect("child");
        remove_path_if_present(&planted).expect("planted dir removed whole");
        assert!(!planted.exists());

        #[cfg(unix)]
        {
            let referent = dir.path().join("live-data");
            std::fs::write(&referent, b"keep me").expect("referent");
            let link = dir.path().join("c.tmp");
            std::os::unix::fs::symlink(&referent, &link).expect("link");
            remove_path_if_present(&link).expect("link unlinked");
            assert!(link.symlink_metadata().is_err(), "the link is gone");
            assert!(referent.exists(), "the referent is NEVER followed");
        }
    }

    #[test]
    fn map_aux_err_types_busy_as_retryable_never_corrupt() {
        // W-swap-4-a-4 + #371: the aux error taxonomy — BUSY is the TRANSIENT `StoreBusy`
        // (retryable), a full disk stays `DiskFull`, a device-locked/ENOSPC-masked
        // SQLITE_IOERR is the honest generic `Io` (never the seed-restore scare), and
        // everything else stays the fail-closed `StoreCorrupt`.
        let sq = |code: std::os::raw::c_int| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None)
        };
        assert!(matches!(
            map_aux_err(sq(rusqlite::ffi::SQLITE_BUSY)),
            WalletError::StoreBusy
        ));
        assert!(matches!(
            map_aux_err(sq(rusqlite::ffi::SQLITE_FULL)),
            WalletError::DiskFull
        ));
        assert!(
            matches!(
                map_aux_err(sq(rusqlite::ffi::SQLITE_IOERR)),
                WalletError::Io(_)
            ),
            "SQLITE_IOERR is the honest generic Io (#371), never StoreCorrupt"
        );
        assert!(matches!(
            map_aux_err(sq(rusqlite::ffi::SQLITE_ERROR)),
            WalletError::StoreCorrupt
        ));
    }

    #[test]
    fn the_corruption_class_ioerr_codes_are_store_corrupt_and_the_rest_stay_io() {
        // R10: `Io` now renders "retrying". A short read or a corrupt filesystem is
        // corruption, and on failing flash it repeats every pass — it must reach the
        // restore remedy, never "retrying" forever. The rest of the IOERR family stays `Io`.
        let sq = |code: std::os::raw::c_int| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None)
        };
        for code in [
            rusqlite::ffi::SQLITE_IOERR_SHORT_READ,
            rusqlite::ffi::SQLITE_IOERR_CORRUPTFS,
        ] {
            assert!(
                matches!(classify_sqlite_error(&sq(code)), WalletError::StoreCorrupt),
                "extended code {code} is corruption"
            );
        }
        assert!(
            matches!(
                classify_sqlite_error(&sq(rusqlite::ffi::SQLITE_IOERR_READ)),
                WalletError::Io(_)
            ),
            "a plain read fault stays the transient Io"
        );
    }

    #[test]
    fn with_aux_busy_retry_recovers_a_transient_busy() {
        // The user-facing seam contract: one busy loss to a concurrent writer, then the
        // retry lands — the caller never sees the transient.
        let mut calls = 0u32;
        let got = with_aux_busy_retry(|| {
            calls += 1;
            if calls == 1 {
                Err(WalletError::StoreBusy)
            } else {
                Ok(7)
            }
        })
        .expect("recovered on the retry");
        assert_eq!(got, 7);
        assert_eq!(calls, 2, "exactly one retry consumed");
    }

    #[test]
    fn with_aux_busy_retry_exhausts_typed() {
        // A store contended through EVERY attempt surfaces the honest retryable —
        // never corruption, and never an unbounded spin.
        let mut calls = 0u32;
        let got: Result<(), WalletError> = with_aux_busy_retry(|| {
            calls += 1;
            Err(WalletError::StoreBusy)
        });
        assert!(matches!(got, Err(WalletError::StoreBusy)));
        assert_eq!(
            calls,
            crate::constants::AUX_BUSY_RETRY_ATTEMPTS,
            "bounded: exactly the configured attempts",
        );
    }

    #[test]
    fn with_aux_busy_retry_passes_other_errors_through_unretried() {
        // Only the transient is retried: corruption (or any other fault) returns
        // immediately — retrying a real fault would just triple the failure latency.
        let mut calls = 0u32;
        let got: Result<(), WalletError> = with_aux_busy_retry(|| {
            calls += 1;
            Err(WalletError::StoreCorrupt)
        });
        assert!(matches!(got, Err(WalletError::StoreCorrupt)));
        assert_eq!(calls, 1, "no retry on a non-busy fault");
    }

    #[test]
    fn open_migrates_in_the_intent_table_for_a_pre_existing_wallet() {
        // §8 (inc-2d-3-b-i forward-compat): the no-`WALLET_SCHEMA_VERSION`-bump decision rests
        // on `migrate` (run on EVERY open) creating the aux table idempotently. Prove it for a
        // wallet provisioned BEFORE the table existed: provision, DROP the table (emulate the
        // prior binary), then open via `open_db_migrated` — the table must come back.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        {
            let conn = open_existing_keyed_connection(&path, &key).expect("open to drop");
            conn.execute_batch(
                "DROP INDEX IF EXISTS idx_queued_send_intent_state;\
                 DROP TABLE queued_send_intent;",
            )
            .expect("emulate a pre-table wallet");
            let present: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='queued_send_intent'",
                    [],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            assert!(
                !present,
                "precondition: the table is absent (an old wallet)"
            );
        }
        // The production open path re-runs `migrate` → `ensure_table`.
        open_db_migrated(&path, &key, Network::Main, None).expect("open migrates the table in");
        let conn = open_existing_keyed_connection(&path, &key).expect("verify open");
        let back: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='queued_send_intent'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(
            back,
            "open re-created the intent table for a pre-existing wallet"
        );
    }

    #[test]
    fn refund_index_reserve_persists_across_a_real_keyed_reopen() {
        // §8 (W-swap-3-a, the never-recycle TEETH on the REAL sealed file): the privacy
        // guarantee is that a restart never re-hands-out a refund index. The unit test in
        // refund_index proves this on a plaintext file; prove it HERE on the actual SQLCipher
        // `wallet.db` the wallet uses (PRAGMA key + the production opener), since a
        // cipher-layer/journal quirk could recycle index 0 while every plaintext test stays
        // green — a silent swap-linkage privacy break.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        {
            let mut conn = open_existing_keyed_connection(&path, &key).expect("keyed open");
            // refunds start at index 1 (external index 0 = the receive address, ADR-0528)
            assert_eq!(
                crate::refund_index::reserve_next_index(&mut conn).expect("reserve 1"),
                1
            );
            assert_eq!(
                crate::refund_index::reserve_next_index(&mut conn).expect("reserve 2"),
                2
            );
        } // keyed connection dropped — a process exit/kill
        let mut conn = open_existing_keyed_connection(&path, &key).expect("keyed reopen");
        assert_eq!(
            crate::refund_index::reserve_next_index(&mut conn).expect("reserve after reopen"),
            3,
            "a reopened SEALED wallet recycled a refund index — HARD-H broken on the real file"
        );
    }

    #[test]
    fn refund_index_large_counter_survives_a_real_keyed_reopen_unmangled() {
        // §8 (W-swap-3-a, the high-END persistence path the low-value reopen test skips):
        // a near-max counter must survive a keyed reopen as EXACTLY that value, never silently
        // re-read as 0 (a NULL-vs-0 confusion or an i64/u32 narrowing bug at the top of the
        // non-hardened range would recycle index 0). Seed near the ceiling, reopen, assert the
        // exact value reserves.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        let near_max = crate::refund_index::NON_HARDENED_MAX - 1;
        {
            let conn = open_existing_keyed_connection(&path, &key).expect("keyed open");
            conn.execute(
                "INSERT INTO refund_address_index (singleton, next_index) VALUES (0, ?1)",
                rusqlite::params![i64::from(near_max)],
            )
            .expect("seed a near-max counter");
        }
        let mut conn = open_existing_keyed_connection(&path, &key).expect("keyed reopen");
        assert_eq!(
            crate::refund_index::reserve_next_index(&mut conn).expect("reserve after reopen"),
            near_max,
            "a large counter was mangled across a sealed reopen — a recycle risk at the ceiling"
        );
    }

    #[test]
    fn open_migrates_in_the_refund_index_table_for_a_pre_existing_wallet() {
        // §8 (W-swap-3-a forward-compat, the direct analogue of the intent migrate-in test):
        // the no-`WALLET_SCHEMA_VERSION`-bump decision rests on `migrate` creating the
        // refund-index table on EVERY open. Provision, DROP the table (emulate a wallet from
        // before this slice), reopen via `open_db_migrated` — the table must come back AND the
        // coexisting aux/engine data must be untouched (no rebuild, no loss).
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        {
            let mut conn = open_existing_keyed_connection(&path, &key).expect("open to drop");
            // leave a sibling-aux row to prove the migrate doesn't disturb existing data
            crate::intent_store::enqueue(&mut conn, "zcash:survivor", 0, None, None)
                .expect("seed sibling");
            conn.execute_batch("DROP TABLE refund_address_index;")
                .expect("emulate a pre-table wallet");
            let present: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='refund_address_index'",
                    [],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            assert!(!present, "precondition: the refund-index table is absent");
        }
        open_db_migrated(&path, &key, Network::Main, None).expect("open migrates the table in");
        let conn = open_existing_keyed_connection(&path, &key).expect("verify open");
        let back: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='refund_address_index'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(
            back,
            "open re-created the refund-index table for a pre-existing wallet"
        );
        // the sibling intent survived the migrate (no data-bearing table was rebuilt)
        assert_eq!(
            crate::intent_store::list_queued(&conn).expect("list").len(),
            1,
            "the coexisting intent row must survive the refund-index migrate-in"
        );
    }

    #[test]
    fn issued_quote_persists_across_a_real_keyed_reopen() {
        // §8 (W-swap-3-c-1, the crash-then-requote TEETH on the REAL sealed file): an issued
        // quote must survive a process kill so the post-crash execute consumes the SAME frozen
        // deposit (one deposit), not an empty registry forcing a re-quote (a second deposit).
        // The unit test proves this on a plaintext file; prove it HERE on the actual SQLCipher
        // `wallet.db` (PRAGMA key + the production opener), since a cipher/journal quirk could
        // drop the row while every plaintext test stays green — a silent double-charge window.
        use crate::issued_quote_store::{StoredQuote, persist, take};
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        let quote = StoredQuote {
            refund_address: None,
            refund_index: None,
            id: "q-sealed".into(),
            deposit_address: Some("t1frozen".into()),
            deposit_amount_zat: Some(100_000),
            expires_at_wall: 2_000_000_000,
            destination_address: None,
            destination_index: None,
            binding: None,
            provider_ref: Some("t1frozen".into()),
            term_deposit_address: Some("t1frozen".into()),
            term_deposit_memo: None,
            term_amount_in: Some("1.0".into()),
            term_min_amount_out: Some("41.5".into()),
            term_zec_side_zat: Some(100_000),
            term_refund_to: None,
            term_expires_at: Some(2_000_000_000),
        };
        {
            let mut conn = open_existing_keyed_connection(&path, &key).expect("keyed open");
            persist(&mut conn, &quote, 16, 1_700_000_000).expect("persist on the sealed file");
        } // keyed connection dropped — a process exit/kill between quote and execute
        let mut conn = open_existing_keyed_connection(&path, &key).expect("keyed reopen");
        let got = take(&mut conn, "q-sealed", 0)
            .expect("take")
            .expect("the issued quote must survive a sealed reopen");
        assert_eq!(
            got, quote,
            "a reopened SEALED wallet lost or mangled an in-flight quote — the double-charge window reopened"
        );
        // still one-shot across the restart (the durable single-flight authority)
        assert!(take(&mut conn, "q-sealed", 0).expect("take2").is_none());
    }

    #[test]
    fn open_migrates_in_the_issued_quote_table_for_a_pre_existing_wallet() {
        // §8 (W-swap-3-c-1 forward-compat, the direct analogue of the intent/refund migrate-in
        // tests): the no-`WALLET_SCHEMA_VERSION`-bump decision rests on `migrate` creating the
        // issued-quote table on EVERY open. Provision, DROP the table (a wallet from before this
        // slice), reopen via `open_db_migrated` — the table must come back AND coexisting data
        // is untouched.
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        {
            let mut conn = open_existing_keyed_connection(&path, &key).expect("open to drop");
            // a sibling-aux row to prove the migrate doesn't disturb existing data
            crate::intent_store::enqueue(&mut conn, "zcash:survivor", 0, None, None)
                .expect("seed sibling");
            conn.execute_batch("DROP TABLE issued_swap_quote;")
                .expect("emulate a pre-table wallet");
            let present: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='issued_swap_quote'",
                    [],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            assert!(!present, "precondition: the issued-quote table is absent");
        }
        open_db_migrated(&path, &key, Network::Main, None).expect("open migrates the table in");
        let conn = open_existing_keyed_connection(&path, &key).expect("verify open");
        let back: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='issued_swap_quote'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(
            back,
            "open re-created the issued-quote table for a pre-existing wallet"
        );
        // the sibling intent survived the migrate (no data-bearing table was rebuilt)
        assert_eq!(
            crate::intent_store::list_queued(&conn).expect("list").len(),
            1,
            "the coexisting intent row must survive the issued-quote migrate-in"
        );
    }

    /// FR-5 D-24: `PRAGMA key` is a silent no-op on plain SQLite, so a link that
    /// bound the wallet to one would store it in plaintext. `cipher_version`
    /// answers no row there; that, an empty or a blank answer are "not SQLCipher".
    #[test]
    fn a_connection_without_a_cipher_version_is_not_sqlcipher() {
        assert!(!is_sqlcipher(None), "no row: plain SQLite");
        assert!(!is_sqlcipher(Some("")), "an empty answer is not a version");
        assert!(!is_sqlcipher(Some("  ")), "a blank answer is not a version");
        assert!(is_sqlcipher(Some("4.6.1 community")));
    }

    /// The SQLite this crate links IS SQLCipher (the bundled-sqlcipher build): a
    /// keyed connection passes the guard, and the guard reads real answers — a
    /// non-empty version, and a provider (measured "openssl").
    #[test]
    fn the_linked_sqlite_answers_a_cipher_version() {
        let dir = tempdir().expect("tempdir");
        let conn = Connection::open(dir.path().join("k.db")).expect("open");
        conn.execute_batch(WalletDbKey::generate().pragma_key_statement().as_str())
            .expect("key");
        let version: String = conn
            .query_row("PRAGMA cipher_version", [], |r| r.get(0))
            .expect("SQLCipher answers cipher_version");
        assert!(!version.trim().is_empty(), "cipher_version: {version:?}");
        assert!(ensure_sqlcipher(&conn).is_ok());
    }

    /// The guard also proves THIS connection's key took: `cipher_provider`
    /// answers only once a key is attached, so an opener that forgot its
    /// `PRAGMA key` is refused like a plain-SQLite link (crypto audit).
    #[test]
    fn a_connection_whose_key_did_not_take_is_refused() {
        let dir = tempdir().expect("tempdir");
        let unkeyed = Connection::open(dir.path().join("u.db")).expect("open");
        assert!(matches!(
            ensure_sqlcipher(&unkeyed),
            Err(WalletError::VaultAbsent)
        ));
    }
}
