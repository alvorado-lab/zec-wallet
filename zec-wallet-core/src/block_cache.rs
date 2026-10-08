//! The disposable compact-block cache (spec §3.2e, W3-inc-2c-iv-b).
//!
//! Data plane ONLY — **no decryption, no key material**. The streamed
//! `CompactBlock`s (iv-a) land here; the scanner (`scan_cached_blocks`, iv-d)
//! reads them back via the upstream `BlockSource` trait. The crypto is
//! `scan_cached_blocks`, consumed WHOLE (§3.2) — this module never touches a key.
//!
//! Storage is a SEPARATE bounded SQLCipher `block-cache.db` keyed by the wallet's
//! own `db_key` (maintainer decision 2026-06-14, §3.2e): SQLite transactions give
//! free crash-atomicity (no torn blocks on a mid-sync kill — the property the
//! disposable contract leans on) and SQLCipher gives free encryption at rest
//! (principle 5). Both DBs are keyed with the SAME raw 256-bit key by design (one
//! `db_key` seals both): the raw-key `PRAGMA key = "x'…'"` form uses the bytes
//! directly as the page-encryption key and BYPASSES the salt KDF, so the per-DB
//! random salt only diversifies the HMAC subkey — confidentiality rests on
//! SQLCipher's random per-page IVs, NOT on per-DB key diversification (which the
//! raw-key form does not provide; that is fine — sharing one AES key across two
//! DBs with fresh per-page IVs is safe).
//!
//! We implement the UNGATED `BlockSource` trait (the only contract
//! `scan_cached_blocks` needs) plus inherent management methods; we deliberately
//! do NOT implement the `sync`-gated `BlockCache` trait nor use the `unstable`
//! `FsBlockDb` — zero new features. The `CACHE_MAX_BYTES` bound is a PRIMITIVE
//! (`headroom`) iv-d's download loop consults; the fill LOOP itself lives in iv-d.
//!
//! **Journal posture (W-swap-4-a-5): ROLLBACK, never WAL.** The cache carries
//! exactly ONE connection, so the wallet file's WAL rationale (reader-vs-writer
//! under two connections, W-swap-4-a-4) does not apply — WAL here is pure 2×
//! write amplification of every compact-block byte through the multi-hour
//! initial sync, plus a `-wal` high-water OUTSIDE the `CACHE_MAX_BYTES`
//! accounting (`headroom` counts `SUM(length(bytes))` only). The opener pins
//! `journal_mode = DELETE` (echo-verified, self-healing — the open is
//! validated-or-deleted); the rollback journal holds BEFORE-images of the
//! pages a txn touches and is deleted at each commit — on the fill path that
//! transient is one insert batch's page span (≤ ~`DOWNLOAD_BATCH_MAX_BYTES`),
//! but a mass DELETE (a reorg truncate, a prune, a cache clear) journals
//! before-images of every touched page, up to ~the cache FILE size worst-case
//! (W-swap-4-a-6 honesty: still transient, still deleted at commit, but the
//! journal high-water tracks the file — page-count-based, so per-page
//! SQLCipher reserve + b-tree/freelist overhead ride modestly above the
//! `SUM(length(bytes))` accounting — not the batch size).
//
// Consumed by the iv-d scan loop; the §8 tests exercise it meanwhile.

use std::path::Path;
use std::sync::{Arc, Mutex};

use prost::Message;
use rusqlite::Connection;
use zcash_client_backend::data_api::chain::BlockSource;
use zcash_client_backend::data_api::chain::error::Error as ChainError;
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_protocol::consensus::BlockHeight;

use crate::constants::{BLOCK_CACHE_DB_FILE_NAME, CACHE_MAX_BYTES, GRPC_MAX_MESSAGE_BYTES};
use crate::db;
use crate::error::WalletError;
use crate::runtime::run_blocking;
use crate::seal::WalletDbKey;

/// The disposable compact-block cache: a keyed SQLCipher DB behind a `Mutex`
/// (the `!Sync` `rusqlite::Connection` — `with_blocks` is SYNC and locks it in
/// the scanner's own blocking context; the async methods lock it inside
/// `spawn_blocking`, never across an `.await`).
pub(crate) struct BlockCache {
    conn: Arc<Mutex<Connection>>,
    /// The read-ahead byte bound. `CACHE_MAX_BYTES` in production; injectable so
    /// the bound test need not write 256 MiB. Read only by the test-only
    /// `headroom` (P3-7): the download loop enforces the bound with its own
    /// accounting (`sync::download_range_folding`), never through the cache.
    #[cfg_attr(not(test), allow(dead_code))]
    max_bytes: u64,
}

impl BlockCache {
    /// Open (and sanitize) the cache under `db_dir`. Production bound is
    /// [`CACHE_MAX_BYTES`].
    pub(crate) fn open(db_dir: &Path, db_key: &WalletDbKey) -> Result<Self, WalletError> {
        Self::open_with_bound(db_dir, db_key, CACHE_MAX_BYTES)
    }

    /// Reset the disposable cache for a rescan (ADR-0534): delete the cache DB and
    /// its sidecars, then recreate it empty. A birthday change starts the cache fresh
    /// — it is re-downloaded on the next sync and is never part of the §6.3
    /// usable-wallet contract, so dropping it loses nothing. The caller holds the
    /// single-writer lock and has closed any prior cache handle first (so the unlink
    /// always succeeds). Runs on the blocking pool (file IO + SQLCipher key setup),
    /// like `open`.
    pub(crate) fn reset(db_dir: &Path, db_key: &WalletDbKey) -> Result<Self, WalletError> {
        let path = db_dir.join(BLOCK_CACHE_DB_FILE_NAME);
        db::remove_db_and_sidecars(&path)?;
        Self::open(db_dir, db_key)
    }

    /// Validated-or-deleted (§6.3): if the existing file won't open + validate
    /// under the key + carry our schema, it is DROPPED and recreated empty — the
    /// cache is disposable by design, and a corrupt/foreign DB must never be fed
    /// to the scanner. SQLite atomicity guarantees the surviving rows are a clean
    /// height-prefix (no torn blocks), so a valid cache is kept across a clean
    /// restart (saving a re-download on a flaky link).
    fn open_with_bound(
        db_dir: &Path,
        db_key: &WalletDbKey,
        max_bytes: u64,
    ) -> Result<Self, WalletError> {
        let path = db_dir.join(BLOCK_CACHE_DB_FILE_NAME);
        let conn = match open_validated(&path, db_key) {
            Ok(conn) => conn,
            // DELIBERATELY every error — including a `DiskFull`/`StoreBusy`
            // from the W-swap-4-a-5 rollback flip's implicit checkpoint: the
            // cache is disposable, deleting it (+ sidecars) FREES the space a
            // mid-flip ENOSPC needs, and busy is unreachable on this
            // single-connection file. Only the recreate's own fault surfaces.
            Err(_) => {
                db::remove_db_and_sidecars(&path)?;
                open_validated(&path, db_key)?
            }
        };
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            max_bytes,
        })
    }

    /// Insert blocks into the cache (the iv-d fill loop's write). Non-contiguous
    /// allowed; idempotent per height (`ON CONFLICT DO UPDATE` — re-inserting a
    /// height overwrites, never duplicates). One transaction = crash-atomic (a
    /// killed insert leaves a clean committed prefix). A full disk surfaces typed
    /// [`WalletError::DiskFull`] (transient — "free space and retry", §6.1), never
    /// `StoreCorrupt`, so iv-d can back off rather than nuke the cache.
    pub(crate) async fn insert(&self, blocks: Vec<CompactBlock>) -> Result<(), WalletError> {
        if blocks.is_empty() {
            return Ok(());
        }
        let conn = Arc::clone(&self.conn);
        run_blocking(move || {
            let mut guard = conn.lock().expect("block cache mutex poisoned");
            let tx = guard.transaction().map_err(map_write_err)?;
            {
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO cached_block (height, bytes) VALUES (?1, ?2) \
                         ON CONFLICT(height) DO UPDATE SET bytes = excluded.bytes",
                    )
                    .map_err(map_write_err)?;
                for block in &blocks {
                    // Zcash heights are u32 by definition (principle 7 — every byte
                    // is hostile): reject an out-of-range height at the boundary, so
                    // it can never be mis-keyed under a divergent i64 AND can never
                    // reach the scanner's `CompactBlock::height()` u32 `unwrap` panic.
                    let height =
                        u32::try_from(block.height).map_err(|_| WalletError::StoreCorrupt)?;
                    let bytes = block.encode_to_vec();
                    stmt.execute(rusqlite::params![i64::from(height), bytes])
                        .map_err(map_write_err)?;
                }
            }
            tx.commit().map_err(map_write_err)?;
            Ok(())
        })
        .await
    }

    /// Read the CONTIGUOUS run of blocks from `start` (inclusive) up to `end`
    /// (exclusive), stopping at the first height gap — the `BlockCache::read`
    /// contract (short reads allowed, but always contiguous from `start`).
    #[cfg(test)]
    pub(crate) async fn read(
        &self,
        start: u64,
        end: u64,
    ) -> Result<Vec<CompactBlock>, WalletError> {
        let conn = Arc::clone(&self.conn);
        run_blocking(move || {
            let guard = conn.lock().expect("block cache mutex poisoned");
            let mut stmt = guard
                .prepare(
                    "SELECT height, length(bytes), bytes FROM cached_block \
                     WHERE height >= ?1 AND height < ?2 ORDER BY height ASC",
                )
                .map_err(|_| WalletError::StoreCorrupt)?;
            let mut rows = stmt
                .query(rusqlite::params![height_to_i64(start), height_to_i64(end)])
                .map_err(|_| WalletError::StoreCorrupt)?;
            let mut out = Vec::new();
            let mut expected = start;
            while let Some(row) = rows.next().map_err(|_| WalletError::StoreCorrupt)? {
                let h: i64 = row.get(0).map_err(|_| WalletError::StoreCorrupt)?;
                if h as u64 != expected {
                    break; // gap → contiguous prefix only
                }
                out.push(decode_block(&row_block_bytes(row)?)?);
                expected += 1;
            }
            Ok(out)
        })
        .await
    }

    /// Delete the blocks in `[start, end)` — iv-d evicts a range once the scanner
    /// has consumed it (keeping the cache bounded behind the scan frontier).
    pub(crate) async fn delete(&self, start: u64, end: u64) -> Result<(), WalletError> {
        let conn = Arc::clone(&self.conn);
        run_blocking(move || {
            let guard = conn.lock().expect("block cache mutex poisoned");
            guard
                .execute(
                    "DELETE FROM cached_block WHERE height >= ?1 AND height < ?2",
                    rusqlite::params![height_to_i64(start), height_to_i64(end)],
                )
                .map_err(map_write_err)?;
            Ok(())
        })
        .await
    }

    /// Remove every block ABOVE `height` (reorg rewind — the chain reorged below
    /// what we cached, so the read-ahead past `height` is stale).
    pub(crate) async fn truncate(&self, height: u64) -> Result<(), WalletError> {
        let conn = Arc::clone(&self.conn);
        run_blocking(move || {
            let guard = conn.lock().expect("block cache mutex poisoned");
            guard
                .execute(
                    "DELETE FROM cached_block WHERE height > ?1",
                    rusqlite::params![height_to_i64(height)],
                )
                .map_err(map_write_err)?;
            Ok(())
        })
        .await
    }

    /// The highest cached height, or `None` if the cache is empty.
    #[cfg(test)]
    pub(crate) async fn tip_height(&self) -> Result<Option<u64>, WalletError> {
        let conn = Arc::clone(&self.conn);
        run_blocking(move || {
            let guard = conn.lock().expect("block cache mutex poisoned");
            let max: Option<i64> = guard
                .query_row("SELECT MAX(height) FROM cached_block", [], |r| r.get(0))
                .map_err(|_| WalletError::StoreCorrupt)?;
            Ok(max.map(|h| h as u64))
        })
        .await
    }

    /// Total bytes currently cached (the `CACHE_MAX_BYTES` accounting).
    #[cfg(test)]
    pub(crate) async fn total_bytes(&self) -> Result<u64, WalletError> {
        let conn = Arc::clone(&self.conn);
        run_blocking(move || {
            let guard = conn.lock().expect("block cache mutex poisoned");
            sum_bytes(&guard)
        })
        .await
    }

    /// Bytes the cache may still accept before the bound. Saturates at 0 once the
    /// cache is at/over the bound. Test-only (P3-7): the iv-d download loop stops
    /// reading ahead on its OWN byte accounting (`sync::download_range_folding`),
    /// so nothing in production consults this; the gate-7 boundary row does.
    #[cfg(test)]
    pub(crate) async fn headroom(&self) -> Result<u64, WalletError> {
        let used = self.total_bytes().await?;
        Ok(self.max_bytes.saturating_sub(used))
    }
}

/// `BlockSource` is the one upstream contract `scan_cached_blocks` reads through
/// (`with_blocks`). It is SYNC — the scanner already runs inside iv-d's
/// `spawn_blocking`, so we lock + query directly here (no nested blocking).
impl BlockSource for BlockCache {
    type Error = WalletError;

    /// Deliver the CONTIGUOUS run of blocks from `from_height` ascending, up to
    /// `limit`, STOPPING at the first height gap.
    ///
    /// CRITICAL (money — crypto audit fold): this stops at a gap rather than
    /// returning blocks ACROSS it. `scan_cached_blocks`'s own continuity check is
    /// SKIPPED for the first block of a run (when `prior_block_metadata` is `None`
    /// — the first batch from the wallet birthday), so if the cache were missing
    /// `from_height` but held a later block, returning that later block first would
    /// make the scanner record `[from_height..later]` as scanned while the gap was
    /// never scanned — silently missed funds. Stopping at the gap yields an empty
    /// (or short) read, the scanner makes no false progress, and iv-d re-fills.
    fn with_blocks<F, WalletErrT>(
        &self,
        from_height: Option<BlockHeight>,
        limit: Option<usize>,
        mut with_block: F,
    ) -> Result<(), ChainError<WalletErrT, Self::Error>>
    where
        F: FnMut(CompactBlock) -> Result<(), ChainError<WalletErrT, Self::Error>>,
    {
        let guard = self.conn.lock().expect("block cache mutex poisoned");
        // `expected` tracks the next contiguous height. `Some(from)` ⇒ the run MUST
        // start exactly at `from` (a start gap yields an empty read — the money fix
        // below). `None` ⇒ the trait contract "begin at the first available block":
        // anchor to the first row, then require contiguity from there.
        let mut expected: Option<i64> = from_height.map(|h| i64::from(u32::from(h)));
        let from_param = expected.unwrap_or(0);
        // SQLite: LIMIT -1 means unbounded.
        let lim = limit.map_or(-1i64, |l| i64::try_from(l).unwrap_or(i64::MAX));
        let mut stmt = guard
            .prepare(
                "SELECT height, length(bytes), bytes FROM cached_block \
                 WHERE height >= ?1 ORDER BY height ASC LIMIT ?2",
            )
            .map_err(|e| ChainError::BlockSource(db::map_aux_err(e)))?;
        let mut rows = stmt
            .query(rusqlite::params![from_param, lim])
            .map_err(|e| ChainError::BlockSource(db::map_aux_err(e)))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| ChainError::BlockSource(db::map_aux_err(e)))?
        {
            let h: i64 = row
                .get(0)
                .map_err(|e| ChainError::BlockSource(db::map_aux_err(e)))?;
            if matches!(expected, Some(e) if h != e) {
                break; // gap (or start gap) → never feed across it (the money fix)
            }
            let bytes = row_block_bytes(row).map_err(ChainError::BlockSource)?;
            let block = decode_block(&bytes).map_err(ChainError::BlockSource)?;
            with_block(block)?;
            expected = Some(h + 1);
        }
        Ok(())
    }
}

/// Open the cache DB keyed, ensure our schema, and PROVE it is usable (a probe
/// over our columns). Any failure ⇒ "not a valid cache" → the caller
/// drops + recreates. The probe catches a foreign DB that happens to decrypt
/// under our key but lacks the `cached_block` shape.
fn open_validated(path: &Path, db_key: &WalletDbKey) -> Result<Connection, WalletError> {
    // The CACHE opener, not the wallet's (W-swap-4-a-5): journal pinned
    // ROLLBACK — a single-connection disposable DB takes no WAL (4-a-4's
    // shared-opener flip silently converted it; a WAL-era cache flips back
    // here, rows kept). See `db::open_keyed_cache_connection`.
    let conn = db::open_keyed_cache_connection(path, db_key)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cached_block (\
            height INTEGER PRIMARY KEY NOT NULL, \
            bytes  BLOB    NOT NULL)",
    )
    .map_err(db::map_aux_err)?;
    sum_bytes(&conn)?; // probe our columns; errors on a foreign/partial schema
    Ok(conn)
}

fn sum_bytes(conn: &Connection) -> Result<u64, WalletError> {
    let total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(length(bytes)), 0) FROM cached_block",
            [],
            |r| r.get(0),
        )
        .map_err(db::map_aux_err)?;
    Ok(total.max(0) as u64)
}

/// Pull a row's block bytes, rejecting an oversized BLOB BEFORE the Rust-side
/// copy (§4.6 size-cap-before-alloc). A legitimately-cached block passed the gRPC
/// `GRPC_MAX_MESSAGE_BYTES` decode cap at insert, so a stored blob larger than
/// that is a substituted/corrupt DB — reject typed (→ validated-or-deleted drops
/// the cache + re-downloads) rather than materialize a multi-hundred-MB `Vec` and
/// risk an OOM/jetsam on mobile. (Unreachable without `db_key` compromise — the
/// SQLCipher page HMAC authenticates the bytes — but cheap read-path insurance.)
/// Row columns: `0 = height`, `1 = length(bytes)`, `2 = bytes`.
fn row_block_bytes(row: &rusqlite::Row) -> Result<Vec<u8>, WalletError> {
    let blen: i64 = row.get(1).map_err(db::map_aux_err)?;
    if blen < 0 || blen as u64 > GRPC_MAX_MESSAGE_BYTES as u64 {
        return Err(WalletError::StoreCorrupt);
    }
    row.get(2).map_err(db::map_aux_err)
}

fn decode_block(bytes: &[u8]) -> Result<CompactBlock, WalletError> {
    CompactBlock::decode(bytes).map_err(|_| WalletError::StoreCorrupt)
}

/// Zcash block heights are u32 on the wire; the proto carries them as u64 and we
/// store i64 (SQLite INTEGER). Heights never approach i64::MAX, so the cast is
/// lossless for every real value.
fn height_to_i64(height: u64) -> i64 {
    height as i64
}

/// A write error: a full disk is the transient [`WalletError::DiskFull`] ("free
/// space and retry", §6.1) — DISTINCT from `StoreCorrupt` so iv-d backs off
/// rather than nuking a healthy cache; everything else is [`db::classify_sqlite_error`]'s.
///
/// Since R12 the fallback is the shared classifier, not a blind `StoreCorrupt`: a
/// busy/locked cache is `StoreBusy`, and a platform that surfaces a genuine
/// `ENOSPC` as `SQLITE_IOERR` (extended `IOERR_WRITE`) rather than `SQLITE_FULL`
/// gets the honest generic `Io` (the classifier's IOERR rule, #371) — never the
/// restore scare. Only a real corruption code stays `StoreCorrupt`.
fn map_write_err(e: rusqlite::Error) -> WalletError {
    match e {
        rusqlite::Error::SqliteFailure(err, _) if err.code == rusqlite::ErrorCode::DiskFull => {
            WalletError::DiskFull
        }
        other => db::classify_sqlite_error(&other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn block_at(height: u64) -> CompactBlock {
        CompactBlock {
            height,
            ..Default::default()
        }
    }

    /// A block padded to ~`pad` bytes (via its header field) so the byte-bound
    /// test can cross a small injected bound with a handful of blocks.
    fn fat_block(height: u64, pad: usize) -> CompactBlock {
        CompactBlock {
            height,
            header: vec![0xAB; pad],
            ..Default::default()
        }
    }

    fn heights(blocks: &[CompactBlock]) -> Vec<u64> {
        blocks.iter().map(|b| b.height).collect()
    }

    #[tokio::test]
    async fn cache_insert_read_delete_round_trips() {
        // §8 `cache_insert_read_delete_round_trips`: blocks in → contiguous read
        // back → delete a range → gone (the contiguous-from-start read contract).
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");

        cache
            .insert(vec![block_at(100), block_at(101), block_at(102)])
            .await
            .expect("insert");
        assert_eq!(
            heights(&cache.read(100, 103).await.expect("read")),
            vec![100, 101, 102]
        );

        cache.delete(100, 102).await.expect("delete"); // removes 100, 101
        assert!(
            cache.read(100, 103).await.expect("read").is_empty(),
            "a gap at the start ⇒ empty contiguous read"
        );
        assert_eq!(
            heights(&cache.read(102, 103).await.expect("read")),
            vec![102]
        );
    }

    #[tokio::test]
    async fn with_blocks_feeds_scan_in_height_order() {
        // §8 `with_blocks_feeds_scan_in_height_order`: the `BlockSource` contract
        // `scan_cached_blocks` relies on — ascending from `from_height`, `limit`
        // honored — even when inserted out of order.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");
        cache
            .insert(vec![
                block_at(202),
                block_at(200),
                block_at(201),
                block_at(203),
            ])
            .await
            .expect("insert");

        let mut seen = Vec::new();
        cache
            .with_blocks::<_, ()>(Some(BlockHeight::from(201u32)), Some(2), |b| {
                seen.push(b.height);
                Ok(())
            })
            .expect("with_blocks");
        assert_eq!(
            seen,
            vec![201, 202],
            "ascending from from_height, limit honored"
        );
    }

    // ── R12 §4.5 test 3: the scanner's cache doors under a REAL held lock ──────
    //
    // The cache is rollback-journal (`db::ensure_rollback_journal`) and its one
    // connection holds no lock between statements, so a second keyed connection's
    // `BEGIN EXCLUSIVE` is the real lock here (unlike the WAL wallet file). The
    // cache's own connection is set to `test_support::R12_BUSY_WAIT` so the row
    // does not sit out the production wait.

    /// Hold an EXCLUSIVE lock on the cache file under `dir` until the returned
    /// connection drops.
    fn hold_the_cache_locked(dir: &Path, key: &WalletDbKey) -> Connection {
        let blocker = db::open_keyed_cache_connection(&dir.join(BLOCK_CACHE_DB_FILE_NAME), key)
            .expect("the blocking connection");
        blocker
            .execute_batch("BEGIN EXCLUSIVE;")
            .expect("hold an exclusive lock on the cache");
        blocker
    }

    fn with_short_busy_wait(cache: &BlockCache) {
        cache
            .conn
            .lock()
            .expect("cache mutex")
            .busy_timeout(crate::test_support::R12_BUSY_WAIT)
            .expect("busy_timeout");
    }

    /// The scanner's `BlockSource` read under a held lock is `StoreBusy` — never
    /// the blind `StoreCorrupt` that reached `stall_for` on every pass — and the
    /// same cache feeds the block once the lock is gone.
    #[tokio::test]
    async fn with_blocks_under_a_held_cache_lock_is_store_busy_not_corruption() {
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");
        cache.insert(vec![block_at(100)]).await.expect("insert");
        with_short_busy_wait(&cache);

        let blocker = hold_the_cache_locked(dir.path(), &key);
        let res = cache.with_blocks::<_, ()>(Some(BlockHeight::from(100u32)), None, |_| Ok(()));
        assert!(
            matches!(res, Err(ChainError::BlockSource(WalletError::StoreBusy))),
            "R12: a held lock at the scanner's cache read is BlockSource(StoreBusy), never the \
             blind StoreCorrupt; got {res:?}"
        );

        drop(blocker);
        let mut seen = Vec::new();
        cache
            .with_blocks::<_, ()>(Some(BlockHeight::from(100u32)), None, |b| {
                seen.push(b.height);
                Ok(())
            })
            .expect("released, the same cache reads");
        assert_eq!(seen, vec![100], "the cached block survived the busy read");
    }

    /// The cache WRITE door (`map_write_err`'s fallback) under a held lock is
    /// `StoreBusy`, and the same insert lands once the lock is gone.
    #[tokio::test]
    async fn an_insert_under_a_held_cache_lock_is_store_busy_not_corruption() {
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");
        cache.insert(vec![block_at(100)]).await.expect("insert");
        with_short_busy_wait(&cache);

        let blocker = hold_the_cache_locked(dir.path(), &key);
        let err = cache.insert(vec![block_at(101)]).await.err();
        assert!(
            matches!(err, Some(WalletError::StoreBusy)),
            "R12: a held lock at the cache write is StoreBusy (`map_write_err` classifies), \
             never StoreCorrupt; got {err:?}"
        );

        drop(blocker);
        cache
            .insert(vec![block_at(101)])
            .await
            .expect("released, the same insert lands");
        assert_eq!(
            heights(&cache.read(100, 102).await.expect("read")),
            vec![100, 101]
        );
    }

    #[tokio::test]
    async fn headroom_reaches_zero_at_the_cache_byte_bound() {
        // §8 `headroom_reaches_zero_at_the_cache_byte_bound` (gate 7): with an
        // INJECTED small bound (256 MiB is impractical to fill in a test),
        // `headroom()` saturates at 0 at the boundary — the primitive iv-d's loop
        // consults to stop reading ahead (so a full disk never spins — M4).
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open_with_bound(dir.path(), &key, 250).expect("open");
        assert_eq!(
            cache.headroom().await.expect("headroom"),
            250,
            "empty ⇒ full headroom"
        );

        cache
            .insert(vec![
                fat_block(1, 100),
                fat_block(2, 100),
                fat_block(3, 100),
            ])
            .await
            .expect("insert");
        assert!(
            cache.total_bytes().await.expect("total") >= 250,
            "cache is at/over the injected bound"
        );
        assert_eq!(
            cache.headroom().await.expect("headroom"),
            0,
            "headroom saturates at 0 past the bound — iv-d stops reading ahead"
        );
    }

    #[tokio::test]
    async fn corrupt_cache_is_deleted_and_recreated_on_open() {
        // §8 `corrupt_cache_is_deleted_and_recreated_on_open`: validated-or-deleted.
        // A corrupt file (and, below, a wrong-key file) is DROPPED + recreated
        // empty on open — never fed to the scanner. The cache is disposable.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let path = dir.path().join(BLOCK_CACHE_DB_FILE_NAME);
        {
            let cache = BlockCache::open(dir.path(), &key).expect("open");
            cache.insert(vec![block_at(500)]).await.expect("insert");
        }
        std::fs::write(&path, b"not a valid sqlcipher database at all").expect("corrupt");

        let cache = BlockCache::open(dir.path(), &key).expect("reopen recreates");
        assert_eq!(
            cache.tip_height().await.expect("tip"),
            None,
            "corrupt cache was dropped + recreated empty"
        );
        cache
            .insert(vec![block_at(600)])
            .await
            .expect("usable after recreate");
        assert_eq!(cache.tip_height().await.expect("tip"), Some(600));

        // wrong-key arm: a cache from a DIFFERENT key is unreadable → dropped.
        let dir2 = tempdir().expect("tempdir");
        {
            let cache =
                BlockCache::open(dir2.path(), &WalletDbKey::generate()).expect("open key A");
            cache.insert(vec![block_at(700)]).await.expect("insert");
        }
        let cache2 =
            BlockCache::open(dir2.path(), &WalletDbKey::generate()).expect("key B recreates");
        assert_eq!(
            cache2.tip_height().await.expect("tip"),
            None,
            "wrong-key cache dropped + recreated empty"
        );
    }

    #[tokio::test]
    async fn cache_keeps_rows_across_the_wal_era_flip() {
        // W-swap-4-a-5 wiring: a cache converted to WAL inside the 4-a-4 window
        // (the shared-opener flip) is NOT treated as invalid — `open` flips it
        // back to the rollback journal IN PLACE, keeping every cached block (no
        // spurious re-download on the upgrade), and the WAL sidecar is gone.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let path = dir.path().join(BLOCK_CACHE_DB_FILE_NAME);
        {
            let cache = BlockCache::open(dir.path(), &key).expect("open");
            cache.insert(vec![block_at(800)]).await.expect("insert");
        }
        {
            // Simulate the 4-a-4 window: the WALLET opener converts the file.
            let conn = db::open_keyed_connection(&path, &key).expect("wal-era open");
            let mode: String = conn
                .query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .expect("journal_mode");
            assert!(mode.eq_ignore_ascii_case("wal"), "the file really flipped");
        }
        let cache = BlockCache::open(dir.path(), &key).expect("reopen");
        assert_eq!(
            cache.tip_height().await.expect("tip"),
            Some(800),
            "the wal-era cache kept its rows across the flip back"
        );
        assert!(
            !db::with_suffix(&path, "-wal").exists(),
            "the flip back checkpointed and unlinked the wal sidecar"
        );
    }

    #[tokio::test]
    async fn mid_batch_kill_leaves_a_clean_height_prefix() {
        // §8 `mid_batch_kill_leaves_a_clean_height_prefix` (disposability §6.3):
        // SQLite atomicity ⇒ a committed batch is durable and contiguous after a
        // kill (handle dropped = process exit AFTER commit). A VALID cache is KEPT
        // across reopen — never needlessly dropped (saves re-download).
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        {
            let cache = BlockCache::open(dir.path(), &key).expect("open");
            cache
                .insert(vec![block_at(100), block_at(101), block_at(102)])
                .await
                .expect("insert");
        }
        let cache = BlockCache::open(dir.path(), &key).expect("reopen keeps valid cache");
        assert_eq!(
            heights(&cache.read(100, 200).await.expect("read")),
            vec![100, 101, 102],
            "a committed batch survives kill+reopen, contiguous — not dropped"
        );
        assert_eq!(cache.tip_height().await.expect("tip"), Some(102));
    }

    #[tokio::test]
    async fn truncate_drops_blocks_above_height() {
        // §8 `truncate_drops_blocks_above_height`: reorg rewind — blocks ABOVE the
        // rewind height are stale and removed; ≤ height kept.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");
        cache
            .insert((100u64..110).map(block_at).collect())
            .await
            .expect("insert");
        cache.truncate(105).await.expect("truncate");
        assert_eq!(
            heights(&cache.read(100, 200).await.expect("read")),
            vec![100, 101, 102, 103, 104, 105],
            "blocks above 105 dropped, ≤105 kept"
        );
        assert_eq!(cache.tip_height().await.expect("tip"), Some(105));
    }

    #[tokio::test]
    async fn with_blocks_stops_at_a_gap_never_feeds_across_it() {
        // §8 `with_blocks_stops_at_a_gap_never_feeds_across_it` (crypto audit MAJOR
        // fold — the load-bearing money property): with a START gap (from_height
        // absent, a later block present), `with_blocks` must deliver NOTHING — never
        // the later block as the first row, which the scanner's first-block
        // continuity blind spot would record as scanned, silently missing the gap.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");
        // 300 missing; 301, 302 present, then a gap, then 305.
        cache
            .insert(vec![block_at(301), block_at(302), block_at(305)])
            .await
            .expect("insert");

        let mut seen = Vec::new();
        cache
            .with_blocks::<_, ()>(Some(BlockHeight::from(300u32)), None, |b| {
                seen.push(b.height);
                Ok(())
            })
            .expect("with_blocks");
        assert!(
            seen.is_empty(),
            "a start gap ⇒ empty read; never feed a later block as if it were from_height"
        );

        // from 301: deliver 301, 302, then STOP at the 303/304 gap (not 305).
        let mut seen2 = Vec::new();
        cache
            .with_blocks::<_, ()>(Some(BlockHeight::from(301u32)), None, |b| {
                seen2.push(b.height);
                Ok(())
            })
            .expect("with_blocks");
        assert_eq!(
            seen2,
            vec![301, 302],
            "contiguous run only — stops at the gap before 305"
        );

        // `None` from_height ⇒ the trait contract "begin at the first available
        // block" (NOT height 0): anchor to 301, deliver 301, 302, stop at the gap.
        let mut seen3 = Vec::new();
        cache
            .with_blocks::<_, ()>(None, None, |b| {
                seen3.push(b.height);
                Ok(())
            })
            .expect("with_blocks");
        assert_eq!(
            seen3,
            vec![301, 302],
            "None starts at the first available block, not 0"
        );
    }

    #[tokio::test]
    async fn insert_rejects_out_of_range_height() {
        // §8 `insert_rejects_out_of_range_height` (crypto audit fold): a hostile
        // block whose height exceeds u32 (Zcash heights are u32) is rejected typed
        // at the boundary — never mis-keyed, never reaching the scanner's u32
        // `unwrap` panic.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");
        let bad = block_at(u64::from(u32::MAX) + 1);
        assert!(
            matches!(
                cache.insert(vec![bad]).await,
                Err(WalletError::StoreCorrupt)
            ),
            "out-of-u32-range height is a typed reject, never stored"
        );
        // a valid block in the same call's absence is unaffected — nothing stored.
        assert_eq!(cache.tip_height().await.expect("tip"), None);
    }

    #[tokio::test]
    async fn oversized_cached_block_is_rejected_not_read() {
        // §8 `oversized_cached_block_is_rejected_not_read` (security fold,
        // defense-in-depth §4.6): a substituted DB row whose BLOB exceeds the gRPC
        // cap is rejected typed BEFORE the Rust-side copy — never materialized
        // (OOM/jetsam), never fed to the scanner.
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let cache = BlockCache::open(dir.path(), &key).expect("open");
        // Inject an oversized raw row directly (simulating a tampered cache file).
        {
            let guard = cache.conn.lock().expect("poisoned");
            let oversized = vec![0u8; GRPC_MAX_MESSAGE_BYTES + 1];
            guard
                .execute(
                    "INSERT INTO cached_block (height, bytes) VALUES (?1, ?2)",
                    rusqlite::params![400i64, oversized],
                )
                .expect("raw insert");
        }
        assert!(
            matches!(cache.read(400, 401).await, Err(WalletError::StoreCorrupt)),
            "oversized blob is rejected on read (size-capped before alloc)"
        );
        let mut seen = Vec::new();
        let r = cache.with_blocks::<_, ()>(Some(BlockHeight::from(400u32)), None, |b| {
            seen.push(b.height);
            Ok(())
        });
        assert!(
            r.is_err(),
            "oversized blob is rejected by with_blocks too, never fed to the scanner"
        );
    }

    #[tokio::test]
    async fn cache_db_is_encrypted_at_rest_and_holds_no_key_material() {
        // §8 `cache_db_is_encrypted_at_rest_and_holds_no_key_material` (principle
        // 5): the cache file is SQLCipher-encrypted — no plaintext SQLite magic,
        // and a recognizable plaintext block marker never appears on disk. (The
        // "no key material" half is structural: the module stores only public
        // CompactBlock bytes — no seed/key ever touches it.)
        let dir = tempdir().expect("tempdir");
        let key = WalletDbKey::generate();
        let path = dir.path().join(BLOCK_CACHE_DB_FILE_NAME);
        let needle: [u8; 8] = [0xC0, 0xFF, 0xEE, 0x5A, 0xB0, 0xDE, 0x12, 0x34];
        {
            let cache = BlockCache::open(dir.path(), &key).expect("open");
            let mut b = block_at(900);
            b.hash = needle.repeat(4); // 32 recognizable plaintext bytes
            cache.insert(vec![b]).await.expect("insert");
        }
        let bytes = std::fs::read(&path).expect("read cache file");
        assert!(
            !bytes.starts_with(b"SQLite format 3\0"),
            "header is SQLCipher-encrypted (no plaintext SQLite magic)"
        );
        assert!(
            bytes.windows(needle.len()).all(|w| w != needle),
            "the block's plaintext marker never appears on disk — encrypted at rest"
        );
    }
}
