//! The durable never-recycle HD-index allocator for swap refund addresses
//! (§3.2h W-swap-3-a; §2.6 HARD-H).
//!
//! An `OutOfZec` swap deposits ZEC from the shielded pool, so a failed swap is refunded
//! by the provider to a fresh **transparent** address the wallet controls
//! ([`crate::derivation::derive_transparent_refund_address`]). "Fresh, never recycled"
//! (HARD-H) is a PRIVACY property: reusing one refund t-addr across two swaps would
//! publicly link them on-chain. This module owns the durable, monotonic index the
//! derivation consumes — a single counter that only ever moves FORWARD, so no two swaps
//! (across restarts, crashes, or concurrent calls) can ever share an index.
//!
//! **Storage.** A single-row counter table in the ONE sealed/wiped/backup-excluded
//! `wallet.db`, riding the SECOND SQLCipher-keyed aux connection (`Inner.aux_db`) exactly
//! like [`crate::intent_store`] — the engine's `WalletDb` exposes no connection accessor,
//! and a refund index is durable money-adjacent state that belongs in the single sealed
//! file (a second durable file would replicate the seal/wipe/backup surface). The table
//! is created idempotently in `db::migrate` (the one chokepoint both provision and open
//! route through), so an existing wallet gains it on next open with NO
//! `WALLET_SCHEMA_VERSION` bump. Functions take a bare `&mut Connection` so they unit-test
//! against a plain connection with no `Wallet` (the [`crate::intent_store`] pattern).
//!
//! **Deadlock-freedom + crash-safety.** The reserve is ONE `BEGIN IMMEDIATE` transaction
//! (read-then-write the counter): it takes `RESERVED` atomically and never sits on a bare
//! `SHARED` it intends to upgrade, so it cannot deadlock the engine's `DEFERRED` writer
//! (the load-bearing invariant documented in [`crate::intent_store`]); and a killed
//! reserve leaves the counter at its last committed value — never a torn or skipped index.
//! Concurrent reserves serialize on `busy_timeout` and receive DISTINCT indices.
//!
//! **No recycle, ever.** The counter is validated in `[0, NON_HARDENED_MAX]` on read; a
//! value outside that range (corruption, or the astronomically-unreachable exhaustion of
//! the 2^31 non-hardened index space) is fail-closed `StoreCorrupt`, never a wrapped or
//! recycled index that would re-hand-out a used refund address.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::db::map_aux_err as map_db_err;
use crate::error::WalletError;

/// The maximum ZIP-32 non-hardened child index (2^31 − 1). The refund index lives in the
/// external-scope non-hardened space (`m/44'/coin'/0'/0/i`), so a valid counter value is
/// always in `[0, NON_HARDENED_MAX]`; [`crate::derivation::derive_transparent_refund_address`]
/// rejects anything at or above 2^31 typed. Kept here (the allocator owns the bound) as the
/// one SSOT rather than re-deriving `(1 << 31) - 1` at the call site.
pub(crate) const NON_HARDENED_MAX: u32 = (1 << 31) - 1;

/// The external-scope index of the wallet's transparent RECEIVE address — the account's
/// canonical `m/44'/coin'/0'/0/0` receiver (`current_transparent_address`, Recv-2 / ADR-0528).
/// This is the ONE source of truth for "which external index is the receive address": the refund
/// floor below is derived from it, AND `derivation::encode_transparent_receive_address` asserts
/// the derived receive address lands on exactly this index (re-review DRY fold — the two facts
/// are now algebraically linked, not comment-coupled, so a change here shifts BOTH the guard and
/// the floor together and any drift is a compile-time/KAT failure, never a silent collision).
pub(crate) const RECEIVE_EXTERNAL_INDEX: u32 = 0;

/// The first refund index the allocator hands out = one past the reserved receive index. So
/// refunds (external `RECEIVE_EXTERNAL_INDEX+1`, +2, …) are collision-free from the receive
/// address by construction — a refund landing on the user's main receive address would link them
/// on-chain (a HARD-H privacy break). The custom counter stays UNBOUNDED (no engine gap limit),
/// so a heavy swapper never runs out of fresh refund addresses (the reason ADR-0528 kept it over
/// the engine's gap-limited ephemeral allocator).
pub(crate) const REFUND_INDEX_FLOOR: u32 = RECEIVE_EXTERNAL_INDEX + 1;

/// #387 — the RESTORE-PESSIMISTIC breadth (spec §3.2h item 5; ADR-0527 addendum): the
/// counter AND ceiling value the FIRST-EVER seed writes when the wallet was NOT freshly
/// generated on this device (counter row absent ∧ creation stamp absent — a restore, or
/// a pre-F4 wallet that never allocated). The standard backfill then registers +
/// one-window-watches external indices `[FLOOR, breadth)` — restoring in-app visibility
/// of any pre-restore refund landed within the breadth — and post-restore allocations
/// start AT the breadth, so the swept range is never reused (HARD-H held within it).
/// Sizing: refund AND destination mints share this counter (one index per quote, both
/// directions, abandoned quotes included), so 64 covers a 63-quote pre-restore history;
/// beyond it the disclosed residual applies (≥ breadth invisibility + possible reuse —
/// the seed-only-restore information loss; a third-party BIP44 gap restore remains the
/// §4.5 recovery). Compile-time floor: the breadth must exceed the FLOOR or the sweep
/// range `[FLOOR, breadth)` is empty and the seed degenerates to the pre-#387 behavior.
pub(crate) const RESTORE_BACKFILL_BREADTH: u32 = 64;
const _: () = assert!(
    RESTORE_BACKFILL_BREADTH > REFUND_INDEX_FLOOR,
    "RESTORE_BACKFILL_BREADTH must exceed the floor or the restore sweep is empty"
);

/// #390 — the depth ONE user-triggered DEEP SCAN ("Check older swap addresses")
/// widens by (spec §3.2h item 5, the #390 sub-block; ADR-0527 #390 addendum). The
/// #387 first seed is BOUNDED to [`RESTORE_BACKFILL_BREADTH`] to keep the restore-time
/// poll set inside the §4.4 item-7 envelope, so a >63-quote pre-restore history leaves
/// single-use addresses at indices ≥ breadth un-swept and in-app-invisible (residual
/// (a)). The deep scan RAISES counter + ceiling by ONE STEP so the standard backfill
/// registers + one-window-watches the widened band and the scoped poll surfaces any
/// UTXOs — the SDK's own in-app recovery of that residual (previously only the
/// third-party BIP44 §4.5 path). A COMPILED constant, NEVER an FFI parameter: the depth
/// is a privacy/perf envelope (each swept index = one watched address for one settlement
/// window — the Exception-THREE cost), not a caller knob. Reruns ACCUMULATE (each accepted
/// run adds a STEP; the op is deliberately NOT idempotent), paced to one outstanding band
/// per settlement window by the [`crate::wallet`] typed refusal — the cap on elective
/// poll-set growth. ~16× the first-seed breadth ⇒ `REFUND_BACKFILL_MAX_PER_PASS` (256)
/// registers it in ~4 chain-advancing passes.
pub(crate) const REFUND_DEEP_SCAN_STEP: u32 = 1024;
const _: () = assert!(
    REFUND_DEEP_SCAN_STEP > 0,
    "REFUND_DEEP_SCAN_STEP must be positive or the deep scan widens nothing"
);

/// Create the single-row refund-index counter table if absent. Idempotent
/// (`CREATE TABLE IF NOT EXISTS`), so `db::migrate` calls it on EVERY provision and open —
/// a wallet provisioned before this table existed gains it on the next open, with NO
/// `WALLET_SCHEMA_VERSION` bump (the [`crate::intent_store`] precedent). The `CHECK
/// (singleton = 0)` pins the table to exactly one row; the counter starts implicitly at 0
/// (the row is created lazily by the first reserve).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS refund_address_index (
             singleton  INTEGER PRIMARY KEY CHECK (singleton = 0),
             next_index INTEGER NOT NULL
         );",
    )
    .map_err(map_db_err)
}

/// The engine-registration BOUNDS table (#368, the ADR-0527 backfill):
/// `registered_up_to` = the highest single-use external index the backfill has
/// engine-persisted via `get_address_for_index`; `backfill_ceiling` = ONE PAST the
/// highest index the backfill is ever allowed to touch, SNAPSHOTTED from the counter
/// the first time the backfill runs (lazy seed — see [`seed_backfill_bounds`]).
///
/// **The ceiling is the review-converged HIGH fix (3-angle review):** without it
/// the backfill would chase the LIVE counter — every freshly minted index (a pending
/// quote's refund, an abandoned quote, a crash-burned reserve) would be swept into a
/// one-window poll, violating the §4.4 item-7 privacy envelope ("a refund address
/// enters the poll only at EXECUTE") and twinning executed swaps' watch rows. With
/// it, the backfill's domain is EXACTLY the owed population: indices allocated
/// BEFORE the snapshot (pre-#368 history, or the full pre-rescan range after the
/// rescan reset) — post-snapshot mints self-register at mint and are never swept.
/// The sweep is done FOREVER once `registered_up_to + 1 >= backfill_ceiling`.
///
/// RESCAN SEMANTICS (ADR-0534 + #368): the TABLE rides `AUX_TABLES_PRESERVED` (keeping
/// the copy set == the ensure set, the guard-test coupling), but the rescan rebuild
/// CLEARS its row on the temp DB before the rename (the `sync_stamp` copy-then-clear
/// precedent) — the engine rebuild dropped every `get_address_for_index` registration,
/// so surviving bounds would falsely assert them registered; the cleared row makes the
/// next backfill RE-SEED (ceiling = the copied counter's full range) and re-register
/// everything — healing refund AND destination registrations across a rescan.
pub(crate) const REGISTRATION_TABLE: &str = "refund_registration";

pub(crate) fn ensure_registration_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS refund_registration (
             singleton        INTEGER PRIMARY KEY CHECK (singleton = 0),
             registered_up_to INTEGER NOT NULL,
             backfill_ceiling INTEGER NOT NULL
         );",
    )
    .map_err(map_db_err)
}

/// The next index the counter would hand out — `next_index` as stored (0/absent ⇒ the
/// FLOOR: nothing was ever reserved, so nothing needs backfill). Read-only; the backfill
/// compares this against `registered_up_to` to decide whether any registration is owed.
/// An out-of-range stored value is the same fail-closed `StoreCorrupt` as
/// [`reserve_next_index`] (never a wild loop bound).
pub(crate) fn next_index_snapshot(conn: &Connection) -> Result<u32, WalletError> {
    let current: Option<i64> = conn
        .query_row(
            "SELECT next_index FROM refund_address_index WHERE singleton = 0",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_db_err)?;
    match current {
        None => Ok(REFUND_INDEX_FLOOR),
        Some(v)
            if (i64::from(REFUND_INDEX_FLOOR)..=i64::from(NON_HARDENED_MAX) + 1).contains(&v) =>
        {
            Ok(v as u32)
        }
        // Note the `+ 1`: a legitimately-exhausted counter stores 2^31 (one past
        // NON_HARDENED_MAX) — the RESERVE path fails closed on it, but the backfill
        // may still register the fully-allocated range below it.
        Some(_) => Err(WalletError::StoreCorrupt),
    }
}

/// The backfill bounds `(registered_up_to, backfill_ceiling)` — `None` if never
/// seeded (a fresh wallet, or post-rescan before the first backfill run). Read-only.
pub(crate) fn backfill_bounds(conn: &Connection) -> Result<Option<(u32, u32)>, WalletError> {
    let row: Option<(i64, i64)> = conn
        .query_row(
            "SELECT registered_up_to, backfill_ceiling FROM refund_registration \
             WHERE singleton = 0",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(map_db_err)?;
    match row {
        None => Ok(None),
        Some((m, c))
            if (0..=i64::from(NON_HARDENED_MAX)).contains(&m)
                && (0..=i64::from(NON_HARDENED_MAX) + 1).contains(&c) =>
        {
            Ok(Some((m as u32, c as u32)))
        }
        Some(_) => Err(WalletError::StoreCorrupt),
    }
}

/// The marker half of [`backfill_bounds`] (0/absent ⇒ nothing registered yet) —
/// the tests' convenience read.
#[cfg(test)]
pub(crate) fn registered_up_to(conn: &Connection) -> Result<u32, WalletError> {
    Ok(backfill_bounds(conn)?.map_or(0, |(m, _)| m))
}

/// Seed the backfill bounds ONCE (first-wins — `OR IGNORE` on the singleton): marker 0,
/// ceiling = the counter snapshot the caller read on the same connection. Seeded at
/// QUIESCENT points ONLY since #382 (the seed-race MED — the claim that the
/// reserve→persist gap was "closed by the index-keyed exclusions" OVERSTATED it: a
/// reserve whose issued row had not yet persisted was invisible to `arm_backfill`'s
/// `NOT EXISTS`): `store::open` seeds right after the migrate-up, under the exclusive
/// open lock, BEFORE the wallet can serve a quote — so every index this build reserves
/// is ≥ the ceiling BY CONSTRUCTION — and the rescan rebuild re-seeds on the temp DB
/// from the COPIED counter, atomically with the marker clear (quote issuance is fenced
/// across a rescan). NOT seeded by `ensure_registration_table`/migrate itself: the
/// rescan temp DB routes through migrate too, and a migrate-seeded row would collide
/// with the plain-INSERT aux copy (`db::copy_aux_tables`). The backfill's own lazy seed
/// remains as a defensive fallback only (unreachable by construction). The index-keyed
/// live-row exclusions remain as the guard for PRE-seed live quotes, whose rows DO
/// exist at snapshot time.
pub(crate) fn seed_backfill_bounds(conn: &Connection, ceiling: u32) -> Result<(), WalletError> {
    conn.execute(
        "INSERT OR IGNORE INTO refund_registration \
             (singleton, registered_up_to, backfill_ceiling) VALUES (0, 0, ?1)",
        rusqlite::params![i64::from(ceiling)],
    )
    .map_err(map_db_err)?;
    Ok(())
}

/// Advance the registration marker to `index` (monotonic — never regresses; an
/// out-of-order write from a stale batch is a no-op via `MAX`). Plain UPDATE — the row
/// is guaranteed by [`seed_backfill_bounds`] (the backfill seeds before it arms); an
/// absent row here is a caller-contract break, surfaced fail-closed. The caller
/// composes this into the same aux txn as the backfill watch rows so a batch's
/// registrations and its marker advance commit atomically.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn set_registered_up_to(
    tx: &rusqlite::Transaction<'_>,
    index: u32,
) -> Result<(), WalletError> {
    let n = tx
        .execute(
            "UPDATE refund_registration \
                 SET registered_up_to = MAX(registered_up_to, ?1) WHERE singleton = 0",
            rusqlite::params![i64::from(index)],
        )
        .map_err(map_db_err)?;
    if n == 0 {
        return Err(WalletError::StoreCorrupt);
    }
    Ok(())
}

/// Clear the registration marker — the RESCAN reset (see [`REGISTRATION_TABLE`]): the
/// rebuilt engine DB holds none of the prior `get_address_for_index` registrations, so
/// the marker must fall to 0 and the next refresh re-registers the full allocated range.
/// The rescan rebuild follows this immediately with [`seed_from_counter`] on the same
/// temp connection (#382), so a post-rescan DB never carries an UNSEEDED bounds row a
/// live quote could race.
pub(crate) fn clear_registration(conn: &Connection) -> Result<(), WalletError> {
    conn.execute("DELETE FROM refund_registration", [])
        .map_err(map_db_err)?;
    Ok(())
}

/// Read the counter and seed the backfill bounds from it, first-wins (#382) — the ONE
/// composition every quiescent seed site uses (`store::open` after the migrate-up; the
/// rescan rebuild's temp DB after [`clear_registration`]). Two statements, NO txn: both
/// sites hold exclusive access to the file (the open lock / the never-shared temp DB),
/// so nothing can interleave the read and the `OR IGNORE` write. Idempotent — an
/// already-seeded row wins (`seed_backfill_bounds`' `OR IGNORE`), so every subsequent
/// open is a no-op read+skip.
pub(crate) fn seed_from_counter(conn: &Connection) -> Result<(), WalletError> {
    let ceiling = next_index_snapshot(conn)?;
    seed_backfill_bounds(conn, ceiling)
}

/// The `store::open` seed with the #387 RESTORE-PESSIMISTIC first-seed rule (spec §3.2h
/// item 5). `freshly_generated_here` = the creation stamp is present (`created_at =
/// Some` ⇔ a freshly-generated create on THIS device — `SeedSource::Generate`, or the
/// host-attested `FreshRawBytes` whose §3.2f lying-host contract carries the trust
/// shift: a mis-attested restore skips this sweep too, recovered by the #390
/// user-triggered deep scan — §3.2f #356-F4).
///
/// The restore arm fires exactly when provenance says the allocation history (if any)
/// died with a previous device: the COUNTER ROW IS ABSENT (nothing ever allocated
/// here — distinguished from a stored value, which [`next_index_snapshot`] cannot do)
/// ∧ NOT freshly generated here ∧ the BOUNDS ROW IS ABSENT (first-wins preserved: a
/// wallet already seeded by a pre-#387 build keeps its seed — the disclosed residual).
/// It then writes counter = ceiling = [`RESTORE_BACKFILL_BREADTH`] — both `OR IGNORE`
/// (idempotent; concurrent-safe under the exclusive open lock the caller holds, same
/// contract as [`seed_from_counter`]) — so the standard backfill sweeps `[FLOOR,
/// breadth)` and the next [`reserve_next_index`] returns the breadth (never a swept
/// index). THE WRITE ORDER IS LOAD-BEARING: the COUNTER is written
/// FIRST — a crash between the two statements leaves counter=breadth/bounds-absent,
/// which the next open converges (the arm skips on the present counter;
/// `seed_from_counter` ceilings at the breadth). The REVERSE order's crash window
/// would leave a widened ceiling over an UNBURNED counter: the first reserve would
/// return the FLOOR — an index inside the swept, watched range (in-breadth reuse +
/// a §4.4 envelope violation). Every other shape (a real counter, a Generate-created
/// wallet, an already-seeded bounds row) falls through to [`seed_from_counter`]
/// unchanged.
///
/// FAULT DIRECTION: a stamp-READ fault upstream degrades `created_at` to `None`, which
/// lands HERE as the restore arm — a sweep, never a missed refund
/// (visibility-conservative; the cost is one bounded sweep on a mis-read Generate
/// wallet). §5.4: no logging (the seed is silent either way; the backfill's own pass
/// logs counts).
pub(crate) fn seed_from_counter_or_restore_floor(
    conn: &Connection,
    freshly_generated_here: bool,
) -> Result<(), WalletError> {
    let counter_row: Option<i64> = conn
        .query_row(
            "SELECT next_index FROM refund_address_index WHERE singleton = 0",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_db_err)?;
    if counter_row.is_none() && !freshly_generated_here && backfill_bounds(conn)?.is_none() {
        conn.execute(
            "INSERT OR IGNORE INTO refund_address_index (singleton, next_index) \
                 VALUES (0, ?1)",
            rusqlite::params![i64::from(RESTORE_BACKFILL_BREADTH)],
        )
        .map_err(map_db_err)?;
        return seed_backfill_bounds(conn, RESTORE_BACKFILL_BREADTH);
    }
    seed_from_counter(conn)
}

/// Reserve the next fresh refund HD index, advancing the durable counter by one. The
/// returned index is unique for the life of the wallet (never recycled, even across a
/// crash between this commit and the caller using the address — the index is BURNED the
/// moment it is reserved). Concurrent callers each get a DISTINCT index (the `IMMEDIATE`
/// txn serializes the read-modify-write).
///
/// The first reserve returns [`REFUND_INDEX_FLOOR`] (1; external index 0 is the receive
/// address — ADR-0528) and persists `next_index = 2`; thereafter it returns the stored value
/// and persists value+1. A stored value outside `[0, NON_HARDENED_MAX]` — corruption, or the
/// unreachable exhaustion of the index space — is fail-closed `StoreCorrupt` (never a
/// wrapped/recycled index).
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn reserve_next_index(conn: &mut Connection) -> Result<u32, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    // Read the current counter (NULL ⇒ never reserved ⇒ start at the FLOOR, index 1 —
    // external index 0 is reserved for the receive address, ADR-0528). The whole
    // read-modify-write is inside the IMMEDIATE txn, so two concurrent reserves can never
    // both read the same value and collide.
    let current: Option<i64> = tx
        .query_row(
            "SELECT next_index FROM refund_address_index WHERE singleton = 0",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_db_err)?;
    let reserved = match current {
        None => REFUND_INDEX_FLOOR,
        // Validate before trust (§4.6): a counter must be in `[FLOOR, NON_HARDENED_MAX]`.
        // Below the FLOOR (a corrupt or zeroed row → index 0) would alias the RECEIVE address
        // (crypto audit HARDENING fold), above the space is exhaustion — both fail closed, NEVER
        // a recycled/aliasing index. A legitimately-stored value is always ≥ FLOOR+1 (it is a
        // previously-reserved index + 1), so this never rejects a real counter.
        Some(v) if (i64::from(REFUND_INDEX_FLOOR)..=i64::from(NON_HARDENED_MAX)).contains(&v) => {
            v as u32
        }
        Some(_) => return Err(WalletError::StoreCorrupt),
    };
    // Persist reserved+1 as the next value. `reserved <= NON_HARDENED_MAX`, so this i64 add
    // cannot overflow; when `reserved == NON_HARDENED_MAX` the stored `next_index` becomes
    // 2^31, and the SUBSEQUENT reserve reads it as out-of-range ⇒ `StoreCorrupt` (the index
    // space is exhausted — fail closed, no wrap). Single-row upsert keyed on the singleton.
    let next = i64::from(reserved) + 1;
    tx.execute(
        "INSERT INTO refund_address_index (singleton, next_index) VALUES (0, ?1) \
         ON CONFLICT(singleton) DO UPDATE SET next_index = ?1",
        rusqlite::params![next],
    )
    .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(reserved)
}

/// #390 — the result of a user-triggered "Check older swap addresses" deep scan
/// (spec §3.2h item 5). COUNTS ONLY — never an index value (§5.4): `widened_by` is how
/// many single-use indices this run added to the checked range, `covered_swaps` /
/// `pending` are the post-widen [`deep_scan_coverage`] read.
#[cfg(feature = "swap")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SwapAddressCheckReport {
    /// How many indices this run added to the checked band (`new − previous`).
    pub widened_by: u32,
    /// How many of the wallet's swaps the checked range now spans (`counter − FLOOR`).
    pub covered_swaps: u32,
    /// Indices not yet registered + polled (`ceiling − 1 − marker`); `0` = done.
    pub pending: u32,
}

/// #390 — the RENDER-ONLY deep-scan coverage read result (spec §3.2h item 5). COUNTS
/// ONLY (§5.4): `covered_swaps` = how many swaps (quotes, either direction, abandoned
/// included) the checked range spans; `pending` = indices not yet registered + polled
/// (`0` = the honest "done").
#[cfg(feature = "swap")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SwapAddressCheckCoverage {
    pub covered_swaps: u32,
    pub pending: u32,
}

/// #390 — the user-triggered DEEP SCAN widen (spec §3.2h item 5): raise the counter AND
/// the backfill ceiling to `min(counter + REFUND_DEEP_SCAN_STEP, NON_HARDENED_MAX + 1)`
/// in ONE `IMMEDIATE` transaction, returning `(previous_counter, new_counter)`. The
/// standard backfill then registers + one-window-watches the widened `(marker, counter)`
/// band (the `registered_up_to` marker is PRESERVED here — this fn never touches it) and
/// the scoped poll surfaces any refund/delivery UTXOs; NO chain rescan (the addresses are
/// transparent).
///
/// **Why one txn, not the two-bare-statement [`seed_from_counter`] shape:** the raise
/// must be atomic across BOTH singleton tables and must SERIALIZE with
/// [`reserve_next_index`] (also `IMMEDIATE`) so a concurrent reserve can never observe a
/// half-widened state and hand out an index inside the sweep — the never-reuse HARD-H
/// property across the widen. Read-modify-write of the counter inside the txn: a killed
/// widen leaves the last committed values (never a torn raise). Both writes are
/// MAX-monotonic (`ONLY widens — never lowers` — the bounds row's first-wins-at-seed rule
/// now reads "raisable ONLY by this op, atomically with an equal-or-greater counter
/// raise"); the ceiling UPDATE is fail-closed on a MISSING bounds row (the
/// [`set_registered_up_to`] contract — `store::open` always seeds it before the wallet can
/// serve a quote, so an absent row is a caller-contract break, surfaced typed).
///
/// **`ceiling <= counter` holds by construction:** both land on `new`, so the item-5
/// backfill cross-check never trips. Raising the counter BURNS `[previous, new)` → post-
/// widen allocations start at `new` (closing residual (a)'s reuse half over the widened
/// width). At the exhaustion sentinel (`next_index` already past `NON_HARDENED_MAX`) the
/// op fails closed `StoreCorrupt` — mirroring [`reserve_next_index`]'s exhaustion arm; the
/// 2^31 index space is astronomically unreachable in a wallet's life. §5.4: no secret
/// crosses here (indices are not on the never-log list, but this fn logs nothing; the
/// caller emits width/counts only).
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn widen_for_deep_scan(conn: &mut Connection) -> Result<(u32, u32), WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    // Current counter (NULL ⇒ never reserved ⇒ FLOOR). Validate BEFORE trust exactly like
    // `reserve_next_index`: a value in `[FLOOR, NON_HARDENED_MAX]` is a live counter; the
    // exhaustion sentinel (`NON_HARDENED_MAX + 1`, written by the final reserve/widen) and
    // any other out-of-range value fail closed `StoreCorrupt` — the deep scan refuses at
    // exhaustion, never wraps or recycles.
    let current: Option<i64> = tx
        .query_row(
            "SELECT next_index FROM refund_address_index WHERE singleton = 0",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_db_err)?;
    let previous = match current {
        None => REFUND_INDEX_FLOOR,
        Some(v) if (i64::from(REFUND_INDEX_FLOOR)..=i64::from(NON_HARDENED_MAX)).contains(&v) => {
            v as u32
        }
        Some(_) => return Err(WalletError::StoreCorrupt),
    };
    // `previous <= NON_HARDENED_MAX` and `STEP >= 1`, so `new > previous` always (the
    // `min` clamps at the sentinel `NON_HARDENED_MAX + 1` = 2^31, which fits u32). The
    // widen therefore never no-ops past the exhaustion guard above.
    let new = previous
        .saturating_add(REFUND_DEEP_SCAN_STEP)
        .min(NON_HARDENED_MAX + 1);
    // Burn the band: raise the counter (MAX-monotonic — a no-op regress under the held
    // IMMEDIATE lock, but written MAX for the same defensive contract as the ceiling).
    tx.execute(
        "INSERT INTO refund_address_index (singleton, next_index) VALUES (0, ?1) \
         ON CONFLICT(singleton) DO UPDATE SET next_index = MAX(next_index, ?1)",
        rusqlite::params![i64::from(new)],
    )
    .map_err(map_db_err)?;
    // Open the ceiling to the same `new` (the ONLY writer that may raise the ceiling past
    // its first-wins seed). Fail-closed on a missing bounds row — `store::open` seeds it.
    let n = tx
        .execute(
            "UPDATE refund_registration \
                 SET backfill_ceiling = MAX(backfill_ceiling, ?1) WHERE singleton = 0",
            rusqlite::params![i64::from(new)],
        )
        .map_err(map_db_err)?;
    if n == 0 {
        return Err(WalletError::StoreCorrupt);
    }
    tx.commit().map_err(map_db_err)?;
    Ok((previous, new))
}

/// #390 — the RENDER-ONLY deep-scan coverage read (spec §3.2h item 5): `covered_swaps`
/// = how many single-use indices the counter has burned (`counter − FLOOR`; each swap
/// quote — either direction, abandoned included — consumes one, so this doubles as the
/// lifetime quote count and as "we've checked addresses for your first N swaps"), and
/// `pending` = `ceiling − 1 − marker`, the count of indices the backfill has NOT yet
/// registered + polled. `pending == 0` is the ONLY honest "done": the widened band is
/// registered and has been polled at least once (found funds then surface via the normal
/// balance path — this read never asserts money exists). Read-only (no txn); a fresh /
/// unseeded wallet reads `pending = 0` (nothing owed). The WalletSpendIntent treatment —
/// counts only, never an index value (§5.4).
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn deep_scan_coverage(conn: &Connection) -> Result<(u32, u32), WalletError> {
    let coverage = next_index_snapshot(conn)?;
    let covered_swaps = coverage.saturating_sub(REFUND_INDEX_FLOOR);
    let pending = match backfill_bounds(conn)? {
        Some((marker, ceiling)) => ceiling.saturating_sub(1).saturating_sub(marker),
        None => 0,
    };
    Ok((covered_swaps, pending))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    /// A fresh in-memory connection with the counter table applied — the allocator logic is
    /// orthogonal to SQLCipher (encryption is db.rs's concern), so unit tests run on a plain
    /// connection (the [`crate::intent_store`] test pattern).
    fn mem() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&conn).expect("schema");
        conn
    }

    /// A file-backed connection (for the durability/concurrency tests, where a value must
    /// survive a connection drop), with a `busy_timeout` (`CONTENTION_BUSY_WAIT_MS`) long
    /// enough that concurrent `IMMEDIATE` txns serialize even on a loaded machine.
    fn open_file(path: &std::path::Path) -> Connection {
        let conn = Connection::open(path).expect("open file db");
        conn.busy_timeout(std::time::Duration::from_millis(
            crate::test_support::CONTENTION_BUSY_WAIT_MS,
        ))
        .expect("busy_timeout");
        ensure_table(&conn).expect("schema");
        conn
    }

    #[test]
    fn reserve_yields_monotonic_indices_from_the_floor() {
        // External index 0 is the receive address (ADR-0528), so refunds start at the FLOOR (1)
        // and advance monotonically with no gaps — never index 0.
        let mut conn = mem();
        for expected in REFUND_INDEX_FLOOR..(REFUND_INDEX_FLOOR + 10) {
            assert_eq!(
                reserve_next_index(&mut conn).expect("reserve"),
                expected,
                "refund indices must be monotonic from the floor (1) with no gaps"
            );
        }
    }

    #[test]
    fn ensure_table_is_idempotent_and_preserves_the_counter() {
        let mut conn = mem();
        assert_eq!(reserve_next_index(&mut conn).unwrap(), 1);
        assert_eq!(reserve_next_index(&mut conn).unwrap(), 2);
        // re-running ensure_table (every open does) must not reset the counter
        ensure_table(&conn).expect("re-ensure");
        assert_eq!(
            reserve_next_index(&mut conn).unwrap(),
            3,
            "re-running ensure_table reset the counter — a recycled refund index"
        );
    }

    #[test]
    fn reserved_indices_persist_across_reopen() {
        // The never-recycle property's teeth: a process restart must NOT re-hand-out index 1.
        let file = tempfile::NamedTempFile::new().expect("temp db");
        let path = file.path().to_owned();
        {
            let mut conn = open_file(&path);
            assert_eq!(reserve_next_index(&mut conn).unwrap(), 1);
            assert_eq!(reserve_next_index(&mut conn).unwrap(), 2);
        } // connection dropped — simulates a process exit/kill
        {
            let mut conn = open_file(&path);
            assert_eq!(
                reserve_next_index(&mut conn).unwrap(),
                3,
                "a reopened wallet recycled a refund index — HARD-H broken"
            );
        }
    }

    #[test]
    fn concurrent_reserves_never_collide() {
        // rust-patterns: thread-safe state needs a concurrent test (N threads, barrier start,
        // assert no collisions). Each thread opens its OWN connection to the same file (the
        // aux-connection-per-caller model) and reserves once; the IMMEDIATE txn + busy_timeout
        // must hand every thread a DISTINCT index forming the contiguous [FLOOR, FLOOR+N) set.
        let file = tempfile::NamedTempFile::new().expect("temp db");
        let path = Arc::new(file.path().to_owned());
        open_file(&path); // ensure the table exists before the race
        const N: usize = 12;
        let barrier = Arc::new(Barrier::new(N));
        let handles: Vec<_> = (0..N)
            .map(|_| {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let mut conn = open_file(&path);
                    barrier.wait();
                    reserve_next_index(&mut conn).expect("reserve under contention")
                })
            })
            .collect();
        let mut got: Vec<u32> = handles
            .into_iter()
            .map(|h| h.join().expect("join"))
            .collect();
        got.sort_unstable();
        assert_eq!(
            got,
            (REFUND_INDEX_FLOOR..REFUND_INDEX_FLOOR + N as u32).collect::<Vec<_>>(),
            "concurrent reserves collided or skipped — two swaps could share a refund t-addr"
        );
    }

    #[test]
    fn out_of_range_counter_is_typed_corruption_never_recycled() {
        // A stored value above the non-hardened space (corruption or exhaustion) is fail-closed
        // `StoreCorrupt` — never wrapped to 0 (which would recycle index 0's refund address).
        let mut conn = mem();
        conn.execute(
            "INSERT INTO refund_address_index (singleton, next_index) VALUES (0, ?1)",
            rusqlite::params![i64::from(NON_HARDENED_MAX) + 1],
        )
        .expect("seed out-of-range counter");
        match reserve_next_index(&mut conn) {
            Err(WalletError::StoreCorrupt) => {}
            other => panic!("expected StoreCorrupt for an out-of-range counter, got {other:?}"),
        }
        // a negative counter (impossible via reserve, only via tamper) is also corruption
        conn.execute(
            "UPDATE refund_address_index SET next_index = -1 WHERE singleton = 0",
            [],
        )
        .expect("tamper negative");
        assert!(matches!(
            reserve_next_index(&mut conn),
            Err(WalletError::StoreCorrupt)
        ));
        // a counter BELOW the floor (0 — would alias the receive address at external index 0) is
        // fail-closed corruption, never handed out (crypto audit HARDENING fold). Legitimately
        // unreachable (the first persisted value is FLOOR+1 = 2), but a corrupt/zeroed row must
        // never recycle index 0.
        conn.execute(
            "UPDATE refund_address_index SET next_index = 0 WHERE singleton = 0",
            [],
        )
        .expect("tamper to the receive index");
        assert!(matches!(
            reserve_next_index(&mut conn),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn restore_pessimistic_seed_burns_the_breadth_for_a_restored_wallet() {
        // #387: counter row ABSENT + NOT freshly generated here ⇒ the first seed writes
        // counter = ceiling = RESTORE_BACKFILL_BREADTH — the standard backfill sweeps
        // [FLOOR, breadth) and the next reserve returns the breadth (never a swept index).
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_from_counter_or_restore_floor(&conn, false).expect("restore seed");
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, RESTORE_BACKFILL_BREADTH)),
            "the restore seed must open the full breadth to the backfill"
        );
        assert_eq!(
            reserve_next_index(&mut conn).unwrap(),
            RESTORE_BACKFILL_BREADTH,
            "post-restore allocations must start AT the breadth — a swept index would \
             be reused across the restore (HARD-H within the breadth)"
        );
    }

    #[test]
    fn restore_pessimistic_seed_is_idempotent_across_reopens() {
        // Second and later opens are no-ops (both writes are OR IGNORE): the counter
        // keeps advancing normally and the bounds row keeps its first-seeded values.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_from_counter_or_restore_floor(&conn, false).expect("first");
        assert_eq!(
            reserve_next_index(&mut conn).unwrap(),
            RESTORE_BACKFILL_BREADTH
        );
        seed_from_counter_or_restore_floor(&conn, false).expect("reopen");
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, RESTORE_BACKFILL_BREADTH)),
            "a reopen must not re-seed"
        );
        assert_eq!(
            reserve_next_index(&mut conn).unwrap(),
            RESTORE_BACKFILL_BREADTH + 1,
            "a reopen must not reset the counter"
        );
    }

    #[test]
    fn freshly_generated_wallet_keeps_the_floor_seed() {
        // A Generate-created wallet has no pre-existing indices by construction — the
        // seed stays at the floor ("done forever", no sweep) and the first reserve is
        // the floor, exactly the pre-#387 behavior.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_from_counter_or_restore_floor(&conn, true).expect("generate seed");
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, REFUND_INDEX_FLOOR)),
            "a freshly-generated wallet must not open a sweep range"
        );
        assert_eq!(reserve_next_index(&mut conn).unwrap(), REFUND_INDEX_FLOOR);
    }

    #[test]
    fn existing_counter_seeds_from_the_counter_not_the_breadth() {
        // The upgrade-in-place shape: a real counter (allocations happened here) wins —
        // the restore arm must never fire over live history, stamp or no stamp.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        assert_eq!(reserve_next_index(&mut conn).unwrap(), 1);
        assert_eq!(reserve_next_index(&mut conn).unwrap(), 2);
        seed_from_counter_or_restore_floor(&conn, false).expect("upgrade seed");
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, 3)),
            "an existing counter must seed the ceiling from the counter (the #368/#382 \
             behavior), never the restore breadth"
        );
        assert_eq!(reserve_next_index(&mut conn).unwrap(), 3);
    }

    #[test]
    fn pre_387_seeded_bounds_row_is_never_overwritten() {
        // The disclosed residual (spec §3.2h item 5 (b)): a wallet restored on a
        // PRE-#387 build already holds the first-wins floor seed — the restore arm
        // must respect it (first-wins is the #382 quiescent-seed contract), not
        // retro-widen a ceiling live quotes may already rely on.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_backfill_bounds(&conn, REFUND_INDEX_FLOOR).expect("old-build seed");
        seed_from_counter_or_restore_floor(&conn, false).expect("387 open");
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, REFUND_INDEX_FLOOR)),
            "an already-seeded bounds row is first-wins — the documented residual"
        );
        // And the counter was NOT burned either (the arm is all-or-nothing).
        assert_eq!(reserve_next_index(&mut conn).unwrap(), REFUND_INDEX_FLOOR);
    }

    #[test]
    fn reserve_at_the_exhaustion_boundary_fails_closed() {
        // Seed the counter at the LAST valid index: that reserve succeeds (returns MAX, a
        // derivable index) and persists 2^31; the NEXT reserve reads the out-of-range value
        // and fails closed — the index space is exhausted, never recycled.
        let mut conn = mem();
        conn.execute(
            "INSERT INTO refund_address_index (singleton, next_index) VALUES (0, ?1)",
            rusqlite::params![i64::from(NON_HARDENED_MAX)],
        )
        .expect("seed at max");
        assert_eq!(
            reserve_next_index(&mut conn).unwrap(),
            NON_HARDENED_MAX,
            "the last valid index must still be reservable"
        );
        assert!(matches!(
            reserve_next_index(&mut conn),
            Err(WalletError::StoreCorrupt)
        ));
    }

    // ── #390 deep-scan widen ────────────────────────────────────────────────

    /// A file-backed helper seeded like a RESTORED wallet (counter = ceiling = breadth,
    /// marker 0) — the population the deep scan exists to recover past.
    fn restored_mem() -> Connection {
        let conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_from_counter_or_restore_floor(&conn, false).expect("restore seed");
        conn
    }

    /// Advance the registration marker in its own txn (the poll's `arm_backfill` shape) —
    /// the test convenience for "the backfill registered up to N".
    fn advance_marker(conn: &mut Connection, up_to: u32) {
        let tx = conn.transaction().expect("txn");
        set_registered_up_to(&tx, up_to).expect("marker");
        tx.commit().expect("commit");
    }

    #[test]
    fn deep_scan_widen_raises_counter_and_ceiling_by_one_step() {
        // The core mechanism: one widen raises BOTH the counter and the ceiling to
        // counter + STEP, leaving the marker untouched, so the standard backfill's owed
        // band becomes (marker, counter).
        let mut conn = restored_mem();
        let (previous, new) = widen_for_deep_scan(&mut conn).expect("widen");
        assert_eq!(previous, RESTORE_BACKFILL_BREADTH);
        assert_eq!(new, RESTORE_BACKFILL_BREADTH + REFUND_DEEP_SCAN_STEP);
        assert_eq!(
            next_index_snapshot(&conn).unwrap(),
            new,
            "counter raised to new"
        );
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, new)),
            "the ceiling must open to `new`; the marker is PRESERVED at 0"
        );
    }

    #[test]
    fn deep_scan_widen_holds_ceiling_le_counter() {
        // The item-5 cross-check invariant: after every widen ceiling == counter, so
        // `ceiling <= counter` holds and `backfill_swap_address_registrations` never
        // trips its fail-closed `ceiling > counter` guard.
        let mut conn = restored_mem();
        for _ in 0..3 {
            widen_for_deep_scan(&mut conn).expect("widen");
            let counter = next_index_snapshot(&conn).unwrap();
            let (_, ceiling) = backfill_bounds(&conn).unwrap().unwrap();
            assert_eq!(ceiling, counter, "ceiling must equal counter after a widen");
        }
    }

    #[test]
    fn deep_scan_widen_burns_the_band_so_no_index_is_reused() {
        // HARD-H across the widen: raising the counter to `new` means the very next
        // reserve returns `new` — never an index inside the swept, watched band (which a
        // pre-restore refund may occupy). This is the reuse-half fix of residual (a).
        let mut conn = restored_mem();
        let (_, new) = widen_for_deep_scan(&mut conn).expect("widen");
        assert_eq!(
            reserve_next_index(&mut conn).unwrap(),
            new,
            "a post-widen allocation must start AT the widened counter, never re-hand a swept index"
        );
    }

    #[test]
    fn deep_scan_widen_is_not_idempotent_reruns_accumulate() {
        // Each accepted run adds a STEP (the deliberate non-idempotency — reruns go
        // deeper). The wallet-level typed refusal, not this fn, paces reruns.
        let mut conn = restored_mem();
        widen_for_deep_scan(&mut conn).expect("first");
        let (previous, new) = widen_for_deep_scan(&mut conn).expect("second");
        assert_eq!(previous, RESTORE_BACKFILL_BREADTH + REFUND_DEEP_SCAN_STEP);
        assert_eq!(new, RESTORE_BACKFILL_BREADTH + 2 * REFUND_DEEP_SCAN_STEP);
    }

    #[test]
    fn deep_scan_widen_preserves_the_marker() {
        // The marker is NEVER touched by the widen — only the backfill advances it. So
        // the owed band grows from (marker, old_ceiling) to (marker, new), and the
        // already-registered [0, marker] range is not redundantly re-swept.
        let mut conn = restored_mem();
        advance_marker(&mut conn, 40);
        widen_for_deep_scan(&mut conn).expect("widen");
        assert_eq!(
            registered_up_to(&conn).unwrap(),
            40,
            "the widen must preserve the registration marker"
        );
    }

    #[test]
    fn deep_scan_widen_heals_a_floor_seeded_wallet() {
        // Residual (b): a wallet seeded on a PRE-#387 build holds the first-wins FLOOR
        // ceiling ("done forever"). The deep scan is the ONE writer allowed to raise the
        // ceiling past that hollow seed — the first run heals it.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_backfill_bounds(&conn, REFUND_INDEX_FLOOR).expect("old-build floor seed");
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, REFUND_INDEX_FLOOR))
        );
        let (_, new) = widen_for_deep_scan(&mut conn).expect("widen");
        assert_eq!(
            backfill_bounds(&conn).unwrap(),
            Some((0, new)),
            "the widen must raise the ceiling past the first-wins floor seed (residual (b) heal)"
        );
    }

    #[test]
    fn deep_scan_widen_clamps_at_the_exhaustion_sentinel() {
        // A counter within one STEP of the top clamps to the sentinel (2^31) rather than
        // overflowing — the last widen the space allows.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_backfill_bounds(&conn, NON_HARDENED_MAX - 10).expect("seed near-max ceiling");
        conn.execute(
            "INSERT INTO refund_address_index (singleton, next_index) VALUES (0, ?1)",
            rusqlite::params![i64::from(NON_HARDENED_MAX - 10)],
        )
        .expect("seed near-max counter");
        let (previous, new) = widen_for_deep_scan(&mut conn).expect("widen");
        assert_eq!(previous, NON_HARDENED_MAX - 10);
        assert_eq!(
            new,
            NON_HARDENED_MAX + 1,
            "widen must clamp at the sentinel, not overflow"
        );
    }

    #[test]
    fn deep_scan_widen_refuses_at_exhaustion() {
        // The last reservable index widens once (to the sentinel); the NEXT widen reads
        // the out-of-range sentinel and fails closed — mirroring `reserve_next_index`.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_backfill_bounds(&conn, NON_HARDENED_MAX).expect("seed max ceiling");
        conn.execute(
            "INSERT INTO refund_address_index (singleton, next_index) VALUES (0, ?1)",
            rusqlite::params![i64::from(NON_HARDENED_MAX)],
        )
        .expect("seed max counter");
        let (_, new) = widen_for_deep_scan(&mut conn).expect("last widen");
        assert_eq!(new, NON_HARDENED_MAX + 1);
        assert!(
            matches!(
                widen_for_deep_scan(&mut conn),
                Err(WalletError::StoreCorrupt)
            ),
            "a widen at the exhaustion sentinel must fail closed, never wrap"
        );
    }

    #[test]
    fn deep_scan_widen_fails_closed_on_a_missing_bounds_row() {
        // The ceiling raise is fail-closed on an absent registration row (the
        // `set_registered_up_to` contract) — `store::open` always seeds it, so an absent
        // row is a caller-contract break, surfaced typed rather than silently no-op'd.
        let mut conn = mem();
        ensure_registration_table(&conn).expect("bounds schema"); // table exists, NO row
        assert!(
            matches!(
                widen_for_deep_scan(&mut conn),
                Err(WalletError::StoreCorrupt)
            ),
            "a widen with no seeded bounds row must fail closed"
        );
        // And the counter must NOT have been raised (all-or-nothing — the txn rolled back).
        assert!(
            next_index_snapshot(&conn).unwrap() == REFUND_INDEX_FLOOR,
            "a failed widen must not leave a raised counter over an un-raised ceiling"
        );
    }

    #[test]
    fn deep_scan_coverage_reports_covered_and_pending() {
        // The render-only read: a restored wallet covers `breadth - FLOOR` swaps with the
        // full band still pending; as the backfill advances the marker, pending falls to
        // 0 (the honest "done").
        let mut conn = restored_mem();
        let (covered, pending) = deep_scan_coverage(&conn).unwrap();
        assert_eq!(covered, RESTORE_BACKFILL_BREADTH - REFUND_INDEX_FLOOR);
        assert_eq!(
            pending,
            RESTORE_BACKFILL_BREADTH - 1,
            "the whole [FLOOR, breadth) band is pending"
        );
        // The backfill registers the whole band → pending 0.
        advance_marker(&mut conn, RESTORE_BACKFILL_BREADTH - 1);
        let (_, pending_done) = deep_scan_coverage(&conn).unwrap();
        assert_eq!(pending_done, 0, "pending == 0 is the only honest 'done'");
        // A widen re-opens pending by the STEP.
        widen_for_deep_scan(&mut conn).expect("widen");
        let (covered_after, pending_after) = deep_scan_coverage(&conn).unwrap();
        assert_eq!(
            covered_after,
            RESTORE_BACKFILL_BREADTH + REFUND_DEEP_SCAN_STEP - REFUND_INDEX_FLOOR
        );
        assert_eq!(
            pending_after, REFUND_DEEP_SCAN_STEP,
            "a widen re-opens exactly one STEP of pending over the preserved marker"
        );
    }

    #[test]
    fn deep_scan_coverage_on_a_fresh_wallet_is_zero_pending() {
        // A freshly-generated wallet (floor seed, nothing owed) reads pending 0 and
        // covers 0 swaps — the coverage line shows "nothing to check" honestly.
        let conn = mem();
        ensure_registration_table(&conn).expect("bounds schema");
        seed_from_counter_or_restore_floor(&conn, true).expect("generate seed");
        let (covered, pending) = deep_scan_coverage(&conn).unwrap();
        assert_eq!(covered, 0);
        assert_eq!(pending, 0);
    }
}
