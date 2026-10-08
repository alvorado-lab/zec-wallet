//! The durable never-recycle diversifier-index allocator for PUBLIC diversified
//! receive UAs (FR-8 / #393; spec §3.3a Recv-4; ADR-0537).
//!
//! Every mint hands out a fresh ZIP-32 diversifier index from the **diversified
//! receive region** `[DIVERSIFIED_INDEX_BASE (2^40), BASE + 2^31)` — disjoint by
//! construction from the swap/refund single-use space (`[1, NON_HARDENED_MAX]`,
//! [`crate::refund_index`]), from the default UA's forward search (starts at 0),
//! AND from the engine's own timestamp-based shielded gap allocator (see the BASE
//! doc — the reason the base is 2^40, not 2^32). Disjointness is the crypto
//! property, not hygiene: two UAs minted at the SAME diversifier index carry
//! byte-identical shielded receivers regardless of receiver set, so any overlap
//! across purposes is public linkability (ADR-0537).
//!
//! **Storage.** A single-row counter table on the aux SQLCipher connection, exactly
//! like [`crate::refund_index`] (created idempotently in `db::migrate`, no schema
//! bump; bare `&mut Connection` for engine-free unit tests). The reserve is ONE
//! `BEGIN IMMEDIATE` read-modify-write — atomic, crash-safe, concurrent reserves
//! get DISTINCT indices.
//!
//! **Restore story (ADR-0527 law, day one; RANDOMIZED).** The #387
//! restore-pessimistic first seed ONLY: at `store::open`, counter-row-absent ∧ NOT
//! freshly generated here ⇒ counter = `BASE + k·DIVERSIFIED_RESTORE_BAND` for an
//! OsRng band `k ∈ [1, DIVERSIFIED_RESTORE_BAND_COUNT]`. Nothing else is needed —
//! shielded trial decryption detects funds at ANY diversifier index, so a restored
//! wallet sees every payment to every previously-minted UA with no counter at all.
//! The only restore harm is a random band landing inside a PRIOR life's used span
//! (two contacts end up holding the same UA — an attribution/linkage defect, never
//! a fund defect): ≤ ⌈used/4096⌉/2^18 per prior life, and bands are free of scan
//! cost. There is no bounds/registration table and no deep-scan analog by design.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::db::map_aux_err as map_db_err;
use crate::error::WalletError;
use crate::refund_index::NON_HARDENED_MAX;

/// The first diversifier index of the public diversified receive region — 2^40,
/// strictly above every OTHER diversifier producer on this account:
/// 1. the swap/refund allocator (counter validated `≤ NON_HARDENED_MAX < 2^31`);
/// 2. the default UA's forward search (starts at 0; reaching even 2^32 would take
///    2^32 consecutive sapling-invalid indices, probability ≈ 2^-(2^32));
/// 3. **the engine's own shielded gap allocator**:
///    `zcash_client_sqlite::get_next_available_address` bases shielded diversifiers
///    at `now_secs + MIN_SHIELDED_DIVERSIFIER_OFFSET (2,817,325,936)` — a band that
///    has sat above 2^32 since Zcash genesis (≈ 2^32 + 3.1×10^8 in 2026) and climbs
///    one per second, crossing 2^40 only around year ~36,000. The SDK never calls
///    that allocator (ADR-0530), but the region must hold even if a future engine
///    API adoption — or another consumer of the same wallet DB — does.
///
/// **FROZEN (ADR-0537)** the way derivation labels are frozen: moving it would
/// re-issue shielded receivers already published under the old region. A
/// shielded-only (G7) UA has no transparent receiver, so exceeding the BIP44
/// non-hardened bound is free — the ZIP-32 diversifier space is 88 bits, and
/// 2^40 + the full span stays far below the Dart-exactness bound (2^41 ≪ 2^53).
pub(crate) const DIVERSIFIED_INDEX_BASE: u64 = 1 << 40;
const _: () = assert!(
    DIVERSIFIED_INDEX_BASE > NON_HARDENED_MAX as u64,
    "the diversified receive region must sit strictly above the swap/refund index space"
);

/// The last counter value accepted on read — one full non-hardened-sized span above
/// the base. Purely a validation ceiling (fail-closed `StoreCorrupt` beyond it, like
/// the refund counter's `NON_HARDENED_MAX`): 2^31 single-use indices exhaust only
/// after ~2^31 mints, unreachable in any real wallet life.
pub(crate) const DIVERSIFIED_INDEX_MAX: u64 = DIVERSIFIED_INDEX_BASE + NON_HARDENED_MAX as u64;

/// #387-pattern restore-pessimistic band width: each wallet LIFE (the original
/// create, and every seed-only restore) mints inside its own band-aligned span, so
/// an index a prior life could plausibly have issued is never re-handed to a new
/// contact. Sizing honesty: under the G7 `Require`-sapling
/// request ~HALF of fixed indices miss and each miss burns a counter value, so a
/// life that mints M addresses consumes ≈2M indices — 4096 covers ≈2048 mints per
/// life, and skipping shielded indices costs NOTHING (no backfill, no watch, no
/// scan; the old 1024 constant was sized in indices but disclosed in mints, a 2×
/// overstatement).
pub(crate) const DIVERSIFIED_RESTORE_BAND: u64 = 4096;

/// Number of selectable restore bands (2^18). The restore seed picks a band
/// UNIFORMLY AT RANDOM in `[1, 2^18]` (never band 0 — that is the original
/// create's band), so the highest possible floor is `BASE + 2^18·4096 = BASE +
/// 2^30`, leaving a ≥2^30-index (≈500M-mint) headroom below
/// [`DIVERSIFIED_INDEX_MAX`] for the restored life itself. WHY random, not a
/// constant (review fold, the chained-restore catch): a CONSTANT floor
/// re-seeds IDENTICALLY on every subsequent seed-only restore, so restore #2
/// re-issues the addresses restore #1's life minted after only a handful of
/// mints — the exact linkage ADR-0537 exists to prevent. A random band makes any
/// two lives collide only if the new band lands inside a prior life's used span:
/// ≤ ⌈used/4096⌉ / 2^18 per prior life (~2^-18 for a normal life), replacing the
/// old CERTAIN re-issue. 2^18 divides 2^32, so masking `next_u32` is bias-free.
pub(crate) const DIVERSIFIED_RESTORE_BAND_COUNT: u64 = 1 << 18;
const _: () = assert!(
    DIVERSIFIED_RESTORE_BAND_COUNT * DIVERSIFIED_RESTORE_BAND
        <= (NON_HARDENED_MAX as u64).div_ceil(2),
    "the highest restore floor must leave at least half the region as mint headroom"
);

/// Bound on the reserve+derive loop (`allocate_single_use_external`): with the G7
/// `Require` sapling receiver, ~half of all FIXED diversifier indices are
/// sapling-invalid (`get_address_for_index` → `Ok(None)`), so misses are a REAL,
/// expected path here — unlike the swap request (`Allow` sapling) where a miss is
/// exotic. 64 attempts ⇒ P(all miss) ≈ 2^-64; exhaustion surfaces as the typed
/// `KeyDerivation`, never a spin or a recycled index.
pub(crate) const DIVERSIFIED_DERIVE_MAX_ATTEMPTS: u32 = 64;

/// A freshly minted public diversified receive UA (FR-8): the encoded address plus
/// the ZIP-32 diversifier index it was derived at — returned together so a host can
/// keep its own contact/invoice ↔ index attribution map ("deterministic per index",
/// the FR-8 acceptance). §5.4: BOTH fields are on the NEVER-log list (the address
/// directly; the index because it re-derives the address); they are returned, never
/// logged.
#[derive(Clone, PartialEq, Eq)]
pub struct MintedDiversifiedAddress {
    /// The canonical encoded UA (G7 receiver set: orchard + sapling, no transparent).
    pub address: String,
    /// The ZIP-32 diversifier index the address was derived at, in
    /// `[DIVERSIFIED_INDEX_BASE, DIVERSIFIED_INDEX_MAX]`.
    pub diversifier_index: u64,
}

/// Redacting `Debug` (the [`crate::parked`] belt): BOTH fields are §5.4 NEVER-log
/// values, so a stray `{:?}` in a future span/panic prints placeholders, never the
/// address or the index. Pure defense-in-depth — no current code formats this type.
impl std::fmt::Debug for MintedDiversifiedAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MintedDiversifiedAddress")
            .field("address", &"<redacted>")
            .field("diversifier_index", &"<redacted>")
            .finish()
    }
}

/// Create the single-row diversified-index counter table if absent. Idempotent
/// (`CREATE TABLE IF NOT EXISTS`), called from `db::migrate` on every provision and
/// open — an existing wallet gains it with NO `WALLET_SCHEMA_VERSION` bump (the
/// [`crate::refund_index`] precedent). The row is created lazily by the first
/// reserve (or by the restore-pessimistic seed).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS diversified_address_index (
             singleton  INTEGER PRIMARY KEY CHECK (singleton = 0),
             next_index INTEGER NOT NULL
         );",
    )
    .map_err(map_db_err)
}

/// The `store::open` seed with the #387 RESTORE-PESSIMISTIC first-seed rule
/// (ADR-0537; the [`crate::refund_index::seed_from_counter_or_restore_floor`]
/// sibling, simpler because this counter has NO bounds/registration table). The
/// restore arm fires exactly when provenance says any mint history died with a
/// previous device: the counter row is ABSENT (nothing minted here) ∧ NOT freshly
/// generated here (`created_at` stamp absent — a restore, or a stamp-read fault
/// degraded conservatively). It writes counter = `BASE + k·BAND` for an OsRng
/// band `k ∈ [1, DIVERSIFIED_RESTORE_BAND_COUNT]` (see the band-count doc for why
/// random) in one idempotent `INSERT OR IGNORE` (concurrent-safe under the
/// exclusive open lock the caller holds; a concurrent double-seed writes ONE of
/// the two candidate floors — either is a valid fresh band). Every other shape —
/// a real counter, a Generate-created wallet — is a no-op: a fresh wallet's first
/// mint starts at the BASE via [`reserve_next_index`]'s floor arm.
///
/// FAULT DIRECTION: a mis-read Generate wallet lands here as the restore arm — the
/// cost is skipping free indices (a band floor), never a re-issued address.
/// §5.4: silent; the chosen band is wallet-local state, never logged.
pub(crate) fn seed_restore_pessimistic(
    conn: &Connection,
    freshly_generated_here: bool,
) -> Result<(), WalletError> {
    if freshly_generated_here {
        return Ok(());
    }
    let counter_row: Option<i64> = conn
        .query_row(
            "SELECT next_index FROM diversified_address_index WHERE singleton = 0",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_db_err)?;
    if counter_row.is_none() {
        // Bias-free: DIVERSIFIED_RESTORE_BAND_COUNT = 2^18 divides 2^32, so the
        // mask is uniform; `+ 1` keeps band 0 (the original create's span) out.
        let band = u64::from(rand_core::RngCore::next_u32(&mut rand_core::OsRng))
            % DIVERSIFIED_RESTORE_BAND_COUNT
            + 1;
        conn.execute(
            "INSERT OR IGNORE INTO diversified_address_index (singleton, next_index) \
                 VALUES (0, ?1)",
            rusqlite::params![diversified_to_i64(
                DIVERSIFIED_INDEX_BASE + band * DIVERSIFIED_RESTORE_BAND
            )],
        )
        .map_err(map_db_err)?;
    }
    Ok(())
}

/// Reserve the next fresh diversifier index, advancing the durable counter by one.
/// The returned index is unique for the life of the wallet (BURNED the moment it is
/// reserved — never recycled, even across a crash between this commit and the caller
/// using it; a sapling-invalid miss burns it too). Concurrent callers each get a
/// DISTINCT index (the `IMMEDIATE` txn serializes the read-modify-write).
///
/// The first reserve returns [`DIVERSIFIED_INDEX_BASE`] and persists `BASE + 1`;
/// thereafter it returns the stored value and persists value+1. A stored value
/// outside `[BASE, DIVERSIFIED_INDEX_MAX]` — corruption (a zeroed/foreign row would
/// otherwise alias the swap space or the default UA's neighborhood), or the
/// unreachable exhaustion of the span — is fail-closed `StoreCorrupt`, never a
/// wrapped, recycled, or below-region index.
pub(crate) fn reserve_next_index(conn: &mut Connection) -> Result<u64, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let current: Option<i64> = tx
        .query_row(
            "SELECT next_index FROM diversified_address_index WHERE singleton = 0",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_db_err)?;
    let reserved = match current {
        None => DIVERSIFIED_INDEX_BASE,
        // Validate before trust (§4.6): a legitimately-stored value is always a
        // previously-reserved index + 1, i.e. in `(BASE, MAX + 1]` — accepting only
        // `[BASE, MAX]` therefore never rejects a real counter mid-span and fails
        // closed at exhaustion (the stored `MAX + 1` reads as out-of-range).
        Some(v)
            if (diversified_to_i64(DIVERSIFIED_INDEX_BASE)
                ..=diversified_to_i64(DIVERSIFIED_INDEX_MAX))
                .contains(&v) =>
        {
            v as u64
        }
        Some(_) => return Err(WalletError::StoreCorrupt),
    };
    // `reserved <= DIVERSIFIED_INDEX_MAX < i64::MAX`, so the +1 cannot overflow; at
    // exhaustion the SUBSEQUENT reserve reads `MAX + 1` as out-of-range ⇒ fail closed.
    let next = diversified_to_i64(reserved) + 1;
    tx.execute(
        "INSERT INTO diversified_address_index (singleton, next_index) VALUES (0, ?1) \
         ON CONFLICT(singleton) DO UPDATE SET next_index = ?1",
        rusqlite::params![next],
    )
    .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(reserved)
}

/// The one u64 → SQLite-i64 conversion, safe by construction: every value this
/// module stores is `≤ DIVERSIFIED_INDEX_MAX + 1 = 2^40 + 2^31`, far below
/// `i64::MAX`. Const-asserted so the bound can never silently drift.
const fn diversified_to_i64(v: u64) -> i64 {
    v as i64
}
const _: () = assert!(
    DIVERSIFIED_INDEX_MAX + 1 < i64::MAX as u64,
    "the diversified index span must fit SQLite's signed 64-bit storage"
);

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        ensure_table(&conn).expect("ensure table");
        conn
    }

    #[test]
    fn diversified_reserve_starts_at_base_and_is_monotonic() {
        let mut c = conn();
        assert_eq!(reserve_next_index(&mut c).unwrap(), DIVERSIFIED_INDEX_BASE);
        assert_eq!(
            reserve_next_index(&mut c).unwrap(),
            DIVERSIFIED_INDEX_BASE + 1
        );
        assert_eq!(
            reserve_next_index(&mut c).unwrap(),
            DIVERSIFIED_INDEX_BASE + 2
        );
    }

    #[test]
    fn diversified_reserve_persists_across_reopen() {
        // A file-backed db so a reopen reads the committed counter (the
        // refund_index reopen-test pattern, temp dir auto-cleaned).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("counter.db");
        {
            let mut c = Connection::open(&path).expect("open");
            ensure_table(&c).expect("ensure");
            assert_eq!(reserve_next_index(&mut c).unwrap(), DIVERSIFIED_INDEX_BASE);
            assert_eq!(
                reserve_next_index(&mut c).unwrap(),
                DIVERSIFIED_INDEX_BASE + 1
            );
        }
        let mut c = Connection::open(&path).expect("reopen");
        ensure_table(&c).expect("ensure");
        assert_eq!(
            reserve_next_index(&mut c).unwrap(),
            DIVERSIFIED_INDEX_BASE + 2
        );
    }

    #[test]
    fn diversified_counter_out_of_region_is_store_corrupt() {
        for bad in [
            0i64,
            1,
            i64::from(NON_HARDENED_MAX),
            diversified_to_i64(DIVERSIFIED_INDEX_BASE) - 1,
            diversified_to_i64(DIVERSIFIED_INDEX_MAX) + 1,
            -5,
        ] {
            let mut c = conn();
            c.execute(
                "INSERT INTO diversified_address_index (singleton, next_index) VALUES (0, ?1)",
                rusqlite::params![bad],
            )
            .unwrap();
            assert!(
                matches!(reserve_next_index(&mut c), Err(WalletError::StoreCorrupt)),
                "value {bad} must fail closed, never alias another index space"
            );
        }
    }

    #[test]
    fn diversified_restore_pessimistic_seed_skips_into_a_random_band() {
        let mut c = conn();
        seed_restore_pessimistic(&c, false).expect("seed");
        let floor = reserve_next_index(&mut c).unwrap();
        assert!(
            floor >= DIVERSIFIED_INDEX_BASE + DIVERSIFIED_RESTORE_BAND,
            "a restored wallet must never start inside the original create's band"
        );
        assert!(
            floor
                <= DIVERSIFIED_INDEX_BASE
                    + DIVERSIFIED_RESTORE_BAND_COUNT * DIVERSIFIED_RESTORE_BAND,
            "the floor must stay inside the selectable band range"
        );
        assert_eq!(
            (floor - DIVERSIFIED_INDEX_BASE) % DIVERSIFIED_RESTORE_BAND,
            0,
            "the floor must be band-aligned"
        );
    }

    #[test]
    fn diversified_restore_floors_differ_across_restores() {
        // The chained-restore fix's teeth: independent restores land in
        // independent bands. 8 fresh stores all colliding is P = 2^-126 —
        // structurally non-flaky.
        let mut floors = std::collections::HashSet::new();
        for _ in 0..8 {
            let mut c = conn();
            seed_restore_pessimistic(&c, false).expect("seed");
            floors.insert(reserve_next_index(&mut c).unwrap());
        }
        assert!(
            floors.len() >= 2,
            "restore floors must be randomized, not a constant re-seed"
        );
    }

    #[test]
    fn diversified_seed_is_noop_for_a_fresh_create() {
        let mut c = conn();
        seed_restore_pessimistic(&c, true).expect("seed");
        assert_eq!(
            reserve_next_index(&mut c).unwrap(),
            DIVERSIFIED_INDEX_BASE,
            "a Generate-created wallet starts at the BASE — no breadth skip"
        );
    }

    #[test]
    fn diversified_seed_first_wins_over_an_existing_counter() {
        let mut c = conn();
        assert_eq!(reserve_next_index(&mut c).unwrap(), DIVERSIFIED_INDEX_BASE);
        // A later open that looks like a restore (stamp lost) must NOT clobber the
        // real counter — the row-present guard, plus INSERT OR IGNORE as the belt.
        seed_restore_pessimistic(&c, false).expect("seed");
        assert_eq!(
            reserve_next_index(&mut c).unwrap(),
            DIVERSIFIED_INDEX_BASE + 1
        );
    }

    #[test]
    fn diversified_seed_is_idempotent() {
        // The second seed must not re-roll the band: one row, one floor.
        let mut c = conn();
        seed_restore_pessimistic(&c, false).expect("first");
        seed_restore_pessimistic(&c, false).expect("second");
        let floor = reserve_next_index(&mut c).unwrap();
        assert_eq!(
            (floor - DIVERSIFIED_INDEX_BASE) % DIVERSIFIED_RESTORE_BAND,
            0,
            "still exactly the first seed's band-aligned floor"
        );
        assert_eq!(reserve_next_index(&mut c).unwrap(), floor + 1);
    }

    #[test]
    fn minted_dto_debug_is_redacted() {
        let m = MintedDiversifiedAddress {
            address: "u1visibleaddress".into(),
            diversifier_index: DIVERSIFIED_INDEX_BASE,
        };
        let s = format!("{m:?}");
        assert!(
            !s.contains("u1visible"),
            "address must never appear in Debug"
        );
        assert!(
            !s.contains(&DIVERSIFIED_INDEX_BASE.to_string()),
            "the index must never appear in Debug"
        );
        assert!(s.contains("<redacted>"));
    }

    #[test]
    fn diversified_reserve_concurrent_mints_never_collide() {
        // Two connections to ONE file-backed db reserving interleaved: the
        // IMMEDIATE txn serializes them; all indices distinct (HARD-H analog).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("counter.db");
        let mut a = Connection::open(&path).expect("open a");
        a.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
        ensure_table(&a).expect("ensure");
        let mut b = Connection::open(&path).expect("open b");
        b.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..4 {
            assert!(seen.insert(reserve_next_index(&mut a).unwrap()));
            assert!(seen.insert(reserve_next_index(&mut b).unwrap()));
        }
        assert_eq!(seen.len(), 8);
    }
}
