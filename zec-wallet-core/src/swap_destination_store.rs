//! The durable SCOPED swap-destination detection set (§3.3b D2 / L2-L3; ADR-0530; IZ-1b).
//!
//! IZ-1a minted a fresh per-swap ZEC destination (engine-persisted via `get_address_for_index`)
//! and recorded it in the durable [`crate::issued_quote_store`] so a crash between quote and
//! execute reconstructs the `recipient`. IZ-1b adds the DETECTION half: the transparent
//! `GetAddressUtxos` poll ([`crate::transparent`]) must watch each active swap destination — NOT
//! the wallet's whole gap-pregenerated receiver set (that would disclose a wallet fingerprint to
//! the endpoint — ADR-0530 security BLOCKER) and NOT only the index-0 receive address (a swap
//! delivery lands at a distinct external index). This module owns the durable SCOPED set the poll
//! unions with index-0.
//!
//! **Why a DEDICATED store, not `IssuedQuoteStore::active_destinations()` (the spec L2 wording).**
//! The spec's first cut nominated the issued-quote store as the owner, assuming a durable swap
//! record survives `execute`. It does NOT: [`crate::issued_quote_store::take`] CONSUMES the issued
//! quote at execute (it is the single-flight authority), and the in-memory status poll holds no
//! durable record. But the detection window OUTLIVES the issued-quote row — funds arrive AFTER the
//! user deposits (post-execute) and must be watched THROUGH delivery + shield. So the scoped set
//! needs its OWN lifecycle, distinct from the single-flight claim: the issued-quote store keeps its
//! SRP (claim-once on execute), this store keeps the detection-set SRP (watch quote→deliver→shield).
//! The small address/index overlap between the two is two DIFFERENT facts with DIFFERENT lifetimes,
//! not a DRY violation. The spec's INTENT — a durable store owns the set, crash-recoverable by
//! construction, retiring on shield/lapse/Hard-kill, with NO in-memory `Inner` cache (no lock-order
//! risk against the engine `WalletDb`) — is honored here; only the mechanism (a sibling table)
//! differs (§3.3b L2 records this resolution).
//!
//! **Storage + deadlock-freedom.** A small table in the ONE sealed/wiped/backup-excluded
//! `wallet.db`, on the SECOND SQLCipher aux connection (`Inner.aux_db`), exactly like
//! [`crate::issued_quote_store`] / [`crate::refund_index`] / [`crate::intent_store`]. Every write is
//! ONE `BEGIN IMMEDIATE` transaction — it takes `RESERVED` atomically and never sits on a bare
//! `SHARED` it intends to upgrade, so it cannot deadlock the engine's `DEFERRED` writer (the
//! load-bearing aux invariant). Created idempotently in `db::migrate` (the one chokepoint both
//! provision and open route through), so an existing wallet gains it on next open with NO
//! `WALLET_SCHEMA_VERSION` bump. Functions take a bare `&mut Connection`/`&Connection` so they
//! unit-test against a plain connection with no `Wallet`.
//!
//! **Since #368 the store watches BOTH inbound swap legs** (the ADR-0527 discharge): the IntoZec
//! DESTINATION (an engine-minted UA string) and the OutOfZec REFUND address (the bare BIP44
//! external t-addr handed to the provider as refundTo, engine-registered at quote alongside its
//! raw derivation). One mechanism — watch a swap-attributable transparent address until its money
//! is shielded — two row shapes, both decoded by the shared
//! [`crate::derivation::transparent_receiver_from_stored_address`].
//!
//! **Lifecycle (§3.3b L2/L3; #368).** `swap_id` is the quote/swap id (the issued-quote `id`), or
//! the synthetic `backfill:<index>` for a one-time ADR-0527 backfill row.
//! - [`upsert_quoted`] at QUOTE time — DESTINATIONS ONLY: the address becomes watchable immediately
//!   (1Click already holds it as `recipient` from the quote request), with `expires_at_wall` = the
//!   QUOTE deadline, so an abandoned/comparison quote retires fast ("an idle wallet shrinks back
//!   to index-0"). A refund has NO quote-time row — nothing can land at it before the deposit
//!   leaves the pool, so an abandoned OutOfZec quote adds zero poll traffic.
//! - [`mark_executed`] / [`arm_in_tx`] at EXECUTE — the swap is live; the destination's window is
//!   pushed to SETTLEMENT (never SHRUNK) and the refund watch is ARMED (insert), atomically with
//!   the W-swap-5 home row (record-first is watch-first). The #368 Refunded re-arm re-runs
//!   [`mark_executed`] when the user pins that terminal.
//! - [`arm_backfill`] once per allocated pre-#368 index — the ADR-0527 migration obligation.
//! - retirement (DELETE): [`prune_expired`] (the LAPSE arm, UNFUNDED rows only since #368 — its
//!   far-future arm CLAMPS to one settlement window rather than deleting, so a clock step never
//!   reaps a live watch), [`debounced_retire_and_mark_served`] (the poll's shield signal — a
//!   previously `funded` row that polls EMPTY on [`RETIRE_AFTER_EMPTY_PASSES`] CONSECUTIVE passes
//!   (#386) was spent/shielded; the ONLY exit for a funded row besides Hard-kill — which since
//!   #385 also latches the record's `watch_served` flag in the SAME txn,
//!   ending the retire→re-arm churn without losing the card-visit re-arm heal), and
//!   [`clear_all`] (a §3.5 Hard swap kill drops every row — honest-off; an in-flight delivery
//!   still lands on-chain and surfaces on the next user-initiated full sync).
//!
//! **Trust boundary.** The address reaching a write is either the engine-minted UA string
//! ([`crate::account::mint_swap_destination`], audited derivation) or the cross-checked refund
//! t-addr (two audited derivations byte-compared at mint) — stored verbatim. On the way OUT
//! [`active`] re-checks only the structural invariant a tampered sealed row could violate (the
//! external index in `[0, u32::MAX]`) — a violating row is SKIPPED fail-honest (#385; never a
//! panic at the consumer, and never a pass-killing `StoreCorrupt`: one tampered row must not
//! silence index-0 detection forever — the posture doc at [`active`]).
//!
//! §5.4: the destination `address` is a NEVER-LOG item — this module stores/returns it, never logs
//! it; only COUNTS are observable upstream.

use rusqlite::{Connection, TransactionBehavior};

use crate::db::map_aux_err as map_db_err;
use crate::error::WalletError;

/// The RESERVED synthetic-key namespace for the #368 ADR-0527 backfill rows
/// (`backfill:<index>`). ONE source of truth: [`arm_backfill`] keys its rows with it,
/// and `SwapService::validate_quote` REFUSES any provider id inside it
/// ([`crate::error::ProviderProtocolReason::ReservedId`]) — a hostile id could
/// otherwise pre-claim the PK (silently dropping an owed one-shot watch) or collide
/// `arm_in_tx`'s frozen-address upsert (suppressing a real swap's refund watch).
pub(crate) const BACKFILL_ID_PREFIX: &str = "backfill:";

/// One active scoped-poll entry — the durable detection-set row reconstructed for the poll. The
/// `address` is the engine-persisted UA string handed to 1Click (destinations, backfill rows) OR
/// the bare refund t-addr handed to the provider as refundTo (#368 — both self-minted forms decode
/// via [`crate::derivation::transparent_receiver_from_stored_address`]); the poll decodes it to
/// its transparent receiver and matches returned UTXOs against it (§4.6 M2 re-derivation).
/// `funded` is the poll's "this address has received at least one UTXO" memory — the
/// shield-retirement signal (a `funded` row that later polls EMPTY was shielded/spent).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct ActiveDestination {
    pub(crate) swap_id: String,
    pub(crate) address: String,
    pub(crate) index: u32,
    pub(crate) funded: bool,
}

/// How many CONSECUTIVE empty polls a `funded` row must return before the funded→empty
/// retire fires (#386, the ADR-0530 debounce promoted): the negative proxy ("emptied ⇒
/// shielded/spent") is one endpoint reply, and a single lying/reorged "empty" must not
/// latch a still-funded leg `watch_served` — since #385 that early retire is STICKY (the
/// latch stops the per-pass re-arm; the card-visit heal is `Refunded`-stream-only), which
/// left a sub-threshold parked IntoZec delivery balance-visible but outside the shield
/// source set until a full rescan. Three passes (~2 extra poll intervals after a genuine
/// shield) buy surviving a two-pass endpoint failover/reorg window at the cost of one
/// already-emptied own-address lingering in the query batch — negligible against the
/// rescan-only stranding it prevents. A FLAPPING endpoint (funded/empty alternation — a
/// lagging round-robin backend) resets the streak on every funded reply, so a
/// genuinely-emptied row never retires while the flap persists — directionally correct
/// (an endpoint asserting funds means keep watching), self-healing after N consecutive
/// honest empties, and costing only the lingering query-batch slot.
pub(crate) const RETIRE_AFTER_EMPTY_PASSES: i64 = 3;

// A debounce needs at least ONE below-threshold pass — `= 1` is "retire on the first empty
// pass" = the pre-#386 un-debounced behavior, defeating the whole feature. It would also
// silently gut the `for _ in 1..RETIRE_AFTER_EMPTY_PASSES` below-threshold assertions across
// the store + wallet tests (an empty range ⇒ zero iterations ⇒ a broken retire passes). Pin
// the invariant at compile time so lowering the const can never quietly degrade either (
// review NIT-4).
const _: () = assert!(
    RETIRE_AFTER_EMPTY_PASSES > 1,
    "RETIRE_AFTER_EMPTY_PASSES must be > 1 or the retire is not debounced",
);

/// Create the scoped swap-destination table if absent. Idempotent (`CREATE TABLE IF NOT EXISTS`),
/// so `db::migrate` calls it on EVERY provision and open — a wallet provisioned before IZ-1b gains
/// it on the next open, with NO `WALLET_SCHEMA_VERSION` bump (the [`crate::issued_quote_store`] /
/// [`crate::refund_index`] precedent).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS swap_destination (
             swap_id         TEXT PRIMARY KEY,
             address         TEXT NOT NULL,
             address_index   INTEGER NOT NULL,
             expires_at_wall INTEGER NOT NULL,
             funded          INTEGER NOT NULL DEFAULT 0,
             empty_streak    INTEGER NOT NULL DEFAULT 0
         );",
    )
    .map_err(map_db_err)?;
    ensure_additive_columns(conn)
}

/// Idempotently add the additive column for a table created by an older build:
/// `empty_streak` (#386 — the retire-debounce counter; `NOT NULL DEFAULT 0` is legal on
/// an additive `ALTER` because the default is a constant, and `0` is correct for every
/// pre-#386 row: no empty passes were noted yet, so a pre-upgrade funded row needs
/// [`RETIRE_AFTER_EMPTY_PASSES`] fresh consecutive empties post-upgrade — strictly the
/// safer direction). `ALTER TABLE ADD COLUMN` errors if the column already exists, so
/// consult `PRAGMA table_info` first (the [`crate::swap_record_store`] idiom) — O(1)
/// metadata, never a data-bearing rebuild on the boot path. NOT a TOCTOU hazard:
/// `migrate` runs only under the exclusive single-opener `WalletLock` flock. Documented
/// downgrade bound: an older binary's RESCAN of a newer DB fails typed on the column
/// drift (`db::assert_columns_match`) — loud, never a silent column shift.
fn ensure_additive_columns(conn: &Connection) -> Result<(), WalletError> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(swap_destination)")
        .map_err(map_db_err)?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(map_db_err)?
        .collect::<Result<std::collections::HashSet<_>, _>>()
        .map_err(map_db_err)?;
    // A list of one ON PURPOSE: the next additive column is appended here, in
    // order (the column-order rule above), not hand-written as a second `if`.
    #[allow(clippy::single_element_loop)]
    for (name, decl) in [("empty_streak", "empty_streak INTEGER NOT NULL DEFAULT 0")] {
        if !names.contains(name) {
            conn.execute_batch(&format!("ALTER TABLE swap_destination ADD COLUMN {decl}"))
                .map_err(map_db_err)?;
        }
    }
    Ok(())
}

/// Add (or refresh) a destination in the scoped set at QUOTE time. ONE `IMMEDIATE` transaction.
/// FIRST-WINS on the minted `(address, address_index)` — a re-quote that somehow reused the same
/// id can never relink the destination to a different address (the issued-quote first-wins mirror)
/// — but it REFRESHES `expires_at_wall` (a re-quote extends the watch window) and PRESERVES
/// `funded` (a destination already seen funded stays funded). `index` is `u32` (the single-use
/// external counter domain, always ≤ 2^31-1) → `i64` losslessly.
pub(crate) fn upsert_quoted(
    conn: &mut Connection,
    swap_id: &str,
    address: &str,
    index: u32,
    expires_at_wall: i64,
) -> Result<(), WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    // ON CONFLICT updates ONLY the deadline — address/address_index/funded are frozen first-wins
    // (the address is the immutable minted UA; funded is the poll's own memory, never reset here).
    tx.execute(
        "INSERT INTO swap_destination (swap_id, address, address_index, expires_at_wall, funded) \
             VALUES (?1, ?2, ?3, ?4, 0) \
         ON CONFLICT(swap_id) DO UPDATE SET expires_at_wall = excluded.expires_at_wall",
        rusqlite::params![swap_id, address, i64::from(index), expires_at_wall],
    )
    .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(())
}

/// Push a watched address's retirement deadline to the SETTLEMENT window when its swap EXECUTES
/// (§3.3b L3; since #368 this arms BOTH inbound legs — the IntoZec destination AND the OutOfZec
/// refund address): pre-execute a destination row expires at the QUOTE deadline (a refund has no
/// pre-execute row at all — nothing can land before the deposit leaves); at execute the swap is
/// live and money can arrive far later, so extend/arm to the settlement window. NEVER SHRINKS the
/// deadline (`MAX(existing, settlement)`) — a long quote window is preserved, and `funded` is
/// preserved.
///
/// UPSERT, not a bare UPDATE: if the row is ABSENT it is INSERTED from the caller's `address`/`index`
/// (the durable issued-quote record the consume returns carries both). This CLOSES the crash window
/// where `persist` recorded the issued quote but the quote-time `upsert_quoted` did not commit
/// (separate txns) — the execute-time arm re-creates the detection-set row from the durable truth,
/// so a watched address is never silently absent from the poll for an executed swap. The statement
/// body is [`arm_in_tx`] so the #368 record-sink can compose it ATOMICALLY with the swap-record
/// insert (one aux txn: home row + watch arm); this standalone form wraps it in its own
/// `IMMEDIATE` txn (the #368 Refunded re-arm rides it).
pub(crate) fn mark_executed(
    conn: &mut Connection,
    swap_id: &str,
    address: &str,
    index: u32,
    settlement_deadline_wall: i64,
) -> Result<(), WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    arm_in_tx(&tx, swap_id, address, index, settlement_deadline_wall, None)?;
    tx.commit().map_err(map_db_err)?;
    Ok(())
}

/// The [`mark_executed`] UPSERT body over a caller-owned transaction — the composition point for
/// the #368 record-sink's ONE-txn "home row + watch arm" write (`crate::swap_record_store::insert`
/// calls this inside its own `IMMEDIATE` txn; a deliberate, documented cross-store call — both
/// tables live on the one aux connection, and atomicity here is what makes record-first also mean
/// watch-first). Same semantics as the standalone form: insert-if-absent, `MAX` deadline, `funded`
/// preserved. `extend_floor` (#385, ONE SSOT for the throttled and unthrottled arms
/// review: the first cut duplicated this SQL): when `Some(floor)`, the EXTEND arm is skipped for
/// an existing row whose deadline is already `>= floor` (a guarded-out `DO UPDATE` dirties no
/// page — the [`rearm_unresolved`] write throttle); the INSERT arm is always unconditional.
/// `None` (every execute-path caller) keeps the plain `MAX` extend.
pub(crate) fn arm_in_tx(
    tx: &rusqlite::Transaction<'_>,
    swap_id: &str,
    address: &str,
    index: u32,
    settlement_deadline_wall: i64,
    extend_floor: Option<i64>,
) -> Result<(), WalletError> {
    // #390 DUAL-ROW RULE (spec §3.2h item 5): a #390 deep-scan widen may have armed a
    // synthetic `backfill:<index>` row for THIS index during the mid-issuance race (a
    // quote reserved mid-widen whose issued row had not yet persisted, then executed).
    // The real execute arm SUPERSEDES that synthetic twin — delete it in the SAME txn so
    // one receiver never carries two rows (which would double-attribute the funded/retire
    // signal, breaking the one-row-per-receiver rule). PK-exact and no-op (dirties
    // no page) when absent; it can never hit the row being armed — a real `swap_id` can
    // never equal the RESERVED `backfill:` namespace (`validate_quote` refuses any
    // provider id inside it).
    tx.execute(
        "DELETE FROM swap_destination WHERE swap_id = ?1",
        rusqlite::params![format!("{BACKFILL_ID_PREFIX}{index}")],
    )
    .map_err(map_db_err)?;
    tx.execute(
        "INSERT INTO swap_destination (swap_id, address, address_index, expires_at_wall, funded) \
             VALUES (?1, ?2, ?3, ?4, 0) \
         ON CONFLICT(swap_id) DO UPDATE \
             SET expires_at_wall = MAX(expires_at_wall, excluded.expires_at_wall) \
             WHERE ?5 IS NULL OR swap_destination.expires_at_wall < ?5",
        rusqlite::params![
            swap_id,
            address,
            i64::from(index),
            settlement_deadline_wall,
            extend_floor
        ],
    )
    .map_err(map_db_err)?;
    Ok(())
}

/// Arm the one-time BACKFILL watch rows (#368, the ADR-0527 migration obligation) and advance the
/// registration marker, ATOMICALLY (one `IMMEDIATE` txn — a crash re-runs the whole idempotent
/// batch, never a marker past unarmed rows). Each entry is an already-allocated single-use external
/// index + its freshly engine-registered address, watched for ONE settlement window under the
/// synthetic key `backfill:<index>` (a RESERVED namespace — `validate_quote` refuses any
/// provider id that would collide with it, so no hostile id can pre-claim or extend these
/// rows) so an ALREADY-LANDED pre-#368 refund is detected once, then the row retires/prunes
/// like any other. An index that ALREADY has a live watch row, or that a LIVE issued quote
/// still references, is SKIPPED — keyed on the INDEX, not the address string (review
/// fix: a refund watch row stores the bare t-addr while a backfill entry carries the UA
/// form of the SAME receiver, so a string comparison could never match; a second row on one
/// receiver would double-attribute the funded/retire signal, and a live quote's index will
/// arm properly at its own execute). Returns how many rows were armed.
pub(crate) fn arm_backfill(
    conn: &mut Connection,
    entries: &[(u32, String)],
    deadline_wall: i64,
    marker_up_to: u32,
) -> Result<usize, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let mut n = 0;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO swap_destination \
                     (swap_id, address, address_index, expires_at_wall, funded) \
                 SELECT ?1, ?2, ?3, ?4, 0 \
                 WHERE NOT EXISTS \
                     (SELECT 1 FROM swap_destination WHERE address_index = ?3) \
                   AND NOT EXISTS \
                     (SELECT 1 FROM issued_swap_quote \
                      WHERE destination_index = ?3 OR refund_index = ?3)",
            )
            .map_err(map_db_err)?;
        for (index, address) in entries {
            n += stmt
                .execute(rusqlite::params![
                    format!("{BACKFILL_ID_PREFIX}{index}"),
                    address,
                    i64::from(*index),
                    deadline_wall
                ])
                .map_err(map_db_err)?;
        }
    }
    crate::refund_index::set_registered_up_to(&tx, marker_up_to)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n)
}

/// #390 — is any one-time `backfill:` watch row still UNEXPIRED (`expires_at_wall >
/// now`)? The [`crate::wallet`] deep-scan pacing gate reads this for typed-refusal (ii):
/// at most ONE outstanding band per settlement window (retry-safety + the elective-growth
/// cap). A funded `backfill:` row whose deadline has PASSED is NOT "unexpired" — any funds
/// it found already park + shield via the normal balance path, so it never blocks a
/// rerun; only a still-open watch window does. Keyed on the RESERVED namespace prefix
/// ([`BACKFILL_ID_PREFIX`], the one SSOT — `%` is the sole LIKE metacharacter and the
/// prefix contains none), so it matches exactly the synthetic backfill rows and no real
/// swap id. Read-only (no txn).
#[cfg(feature = "swap")]
pub(crate) fn has_unexpired_backfill_row(
    conn: &Connection,
    now_unix: i64,
) -> Result<bool, WalletError> {
    let pattern = format!("{BACKFILL_ID_PREFIX}%");
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM swap_destination \
                 WHERE swap_id LIKE ?1 AND expires_at_wall > ?2)",
            rusqlite::params![pattern, now_unix],
            |r| r.get(0),
        )
        .map_err(map_db_err)?;
    Ok(exists)
}

/// Re-arm the watched leg of every UNRESOLVED swap record for a fresh settlement window
/// (#382 (b) — the converged HIGH's per-pass invariant): the poll's set-build calls
/// this BEFORE [`prune_expired`], so a watch row that lapsed — or was already pruned —
/// while its record's outcome is still unpinned is extended or RESURRECTED and polled by
/// the SAME pass; the first foreground after ANY absence detects a waiting refund, instead
/// of the watch/list/re-arm all dying on one execute + 48 h bound. [`arm_in_tx`]'s
/// insert-if-absent + `MAX`-deadline + frozen-address semantics, with ONE addition
/// (#385, the write-throttle MED): the extend is SKIPPED while the existing row's
/// deadline is still at least HALF a settlement window out (`existing >= deadline − ½
/// settlement` ⇔ `existing >= now + ½ settlement`) — without the guard this txn
/// rewrote+fsynced EVERY watch row on EVERY ~75 s pass for the entire (now unbounded)
/// life of any unresolved record (~1.1k aux fsyncs/day from one stuck row). No semantic
/// change: the skipped row still has ≥ 24 h of window, this same call extends it back to
/// a full window on the first pass after the midpoint, and an ABSENT row (the resurrect
/// arm) always inserts. A resurrected row starts `funded = 0` and is re-marked by the
/// same pass's put attribution; an existing row only ever EXTENDS — never relinks. The
/// caller sources `watches` from [`crate::swap_record_store::unresolved_watches`]
/// (cap-bounded ≤ `MAX_SWAP_RECORDS`; a `watch_served` retired leg is excluded THERE) and
/// gates on the §3.5 Hard kill exactly like the poll itself (honest-off must not re-arm
/// what `clear_all` drops); a DISMISSED record stops appearing here, so its watch lapses
/// naturally within one settlement window (spec §4.4 item 8b — the engine registration
/// survives, a post-dismiss refund stays rescan-recoverable). ONE `IMMEDIATE` txn; an
/// empty input is a no-op (no txn). Privacy: this is the spec item-7 exception-TWO
/// widening — an unresolved swap's provider-disclosed leg stays in the scoped poll until
/// pinned or dismissed.
pub(crate) fn rearm_unresolved(
    conn: &mut Connection,
    watches: &[(String, String, u32)],
    deadline_wall: i64,
) -> Result<(), WalletError> {
    if watches.is_empty() {
        return Ok(());
    }
    // Half a settlement window below the fresh deadline (== now + ½ settlement at the
    // caller's `deadline_wall = now + settlement`). Integer halving is exact for the
    // shipped even constant and merely shifts the skip boundary by ≤ 1 s otherwise.
    let settlement = i64::try_from(crate::constants::SWAP_SETTLEMENT_MAX_SECS).unwrap_or(i64::MAX);
    let threshold = deadline_wall.saturating_sub(settlement / 2);
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    for (swap_id, address, index) in watches {
        // [`arm_in_tx`] with the throttle floor on the UPDATE arm only — the INSERT
        // (resurrect) arm is unconditional, and a guarded-out UPDATE dirties no page
        // (SQLite skips the journal on a no-write commit).
        arm_in_tx(
            &tx,
            swap_id,
            address,
            *index,
            deadline_wall,
            Some(threshold),
        )?;
    }
    tx.commit().map_err(map_db_err)?;
    Ok(())
}

/// The current active scoped set (§3.3b D2): every watched address whose retirement deadline has
/// not yet passed (`expires_at_wall > now`), PLUS every `funded` row regardless of deadline (#368,
/// the [`prune_expired`] funded-spare's twin — a spared row that `active` still excluded would be
/// unpollable and unshieldable, the exact stranding the spare exists to prevent; a funded row
/// leaves only via [`debounced_retire_and_mark_served`]/[`clear_all`]). The poll unions this with the
/// index-0 receiver. Read-only (no txn). Ordered by `swap_id` so callers/tests see a
/// deterministic sequence (rust-patterns: never compare an unordered SQL result).
///
/// **Corrupt-row posture (#385, the set-build MED — was fail-closed `StoreCorrupt`):** a
/// row whose stored fields cannot READ because of TAMPER — `address_index` outside
/// `[0, u32::MAX]`, or a type-tampered column under SQLite's dynamic typing
/// (`InvalidColumnType`) — is SKIPPED fail-honest, exactly like
/// [`crate::swap_record_store::unresolved_watches`]'s posture on the same-tier tamper, and the
/// skips are COUNTED into a payload-free `wallet.swap_destination_scan { skipped }` warn (the
/// review's observability half: a silent skip made sealed-DB tamper indistinguishable
/// from row-never-existed). Failing closed made ONE tampered row (only reachable via the sealed
/// DB) kill EVERY transparent poll pass forever — index-0 detection included — silently and
/// RESCAN-SURVIVINGLY (the aux tables are rescan-preserved), a money-visibility denial an
/// attacker with sealed-DB write access shouldn't get for one flipped byte. Skipping costs
/// exactly the tampered row's own coverage (the same as if it never existed; its money, if any,
/// stays engine-tracked and rescan-recoverable) while every other destination and index-0 keep
/// polling. Any OTHER per-row error is a genuine step/page fault (I/O, HMAC) and PROPAGATES
/// loud — real corruption must never read as an honestly-empty set.
pub(crate) fn active(
    conn: &Connection,
    now_unix: i64,
) -> Result<Vec<ActiveDestination>, WalletError> {
    let mut stmt = conn
        .prepare(
            "SELECT swap_id, address, address_index, funded FROM swap_destination \
             WHERE expires_at_wall > ?1 OR funded = 1 ORDER BY swap_id",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![now_unix], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .map_err(map_db_err)?;
    let mut out = Vec::new();
    let mut skipped: u64 = 0;
    for row in rows {
        // TAMPER shapes skip + count; genuine faults propagate (posture doc above).
        let (swap_id, address, index_i64, funded) = match row {
            Ok(fields) => fields,
            Err(rusqlite::Error::InvalidColumnType(..)) => {
                skipped += 1;
                continue;
            }
            Err(e) => return Err(map_db_err(e)),
        };
        let Ok(index) = u32::try_from(index_i64) else {
            skipped += 1;
            continue;
        };
        out.push(ActiveDestination {
            swap_id,
            address,
            index,
            funded: funded != 0,
        });
    }
    if skipped > 0 {
        // §5.4: a COUNT only — never a row payload (`skipped` is allowlisted).
        tracing::warn!(target: "zec_wallet_core", skipped, "wallet.swap_destination_scan");
    }
    Ok(out)
}

/// Mark the given destinations `funded` AND reset their retire-debounce streak (#386) — the poll
/// calls this for every active destination that returned a UTXO this pass: seeing the row funded
/// both sets the memory flag and breaks any [`RETIRE_AFTER_EMPTY_PASSES`] run in progress (the
/// streak counts CONSECUTIVE empties only). Idempotent, and a re-mark of an already-funded,
/// streak-0 row is a ZERO-WRITE no-op (the guard, #385 — the write-throttle sibling of the re-arm
/// guard: a parked sub-threshold funded row otherwise re-dirtied+fsynced its page on every ~75 s
/// pass for as long as it stayed parked). Returns the number of rows CHANGED (newly funded, or a
/// streak broken). ONE `IMMEDIATE` txn. An empty input is a no-op (no txn).
pub(crate) fn mark_funded(
    conn: &mut Connection,
    swap_ids: &[String],
) -> Result<usize, WalletError> {
    if swap_ids.is_empty() {
        return Ok(0);
    }
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let mut n = 0;
    {
        let mut stmt = tx
            .prepare(
                "UPDATE swap_destination SET funded = 1, empty_streak = 0 \
                 WHERE swap_id = ?1 AND (funded = 0 OR empty_streak != 0)",
            )
            .map_err(map_db_err)?;
        for id in swap_ids {
            n += stmt.execute(rusqlite::params![id]).map_err(map_db_err)?;
        }
    }
    tx.commit().map_err(map_db_err)?;
    Ok(n)
}

/// Note one EMPTY poll pass for each given `funded` destination and retire (DELETE) those whose
/// empty run reaches [`RETIRE_AFTER_EMPTY_PASSES`] — the poll's DEBOUNCED SHIELD-retirement
/// signal (§3.3b L3a + #386): a destination previously seen `funded` that polls EMPTY on N
/// consecutive passes had its delivery spent/shielded, so it leaves the scoped set (the set
/// shrinks back toward index-0) — AND, ATOMICALLY in the SAME `IMMEDIATE` txn as both the streak
/// bump and the DELETE, each retired id latches its `swap_record.watch_served` flag
/// ([`crate::swap_record_store::mark_watch_served_in_tx`] — the [`arm_in_tx`] cross-store
/// precedent, direction reversed: both tables live on the one aux connection, and the atomicity
/// is the point). The latch is what stops the #382 per-pass re-arm from resurrecting the
/// just-retired leg (retire deletes, re-arm re-inserts, polls empty, retires again — unbounded
/// churn, IntoZec deliveries especially, which have no chain pin); composing it with the DELETE
/// (#385, the headline MED) closes the two-txn kill/fault window that durably re-opened
/// the churn — and a retire that FAULTS rolls the whole txn back, bumps included, so the next
/// empty pass recomputes and retries (the id re-enters via its preserved streak). The DEBOUNCE
/// (#386, the ADR-0530 hardening promoted) is why the input is CANDIDATES, not retirees: one
/// lying/reorged "empty" reply must not latch a still-funded leg served — see
/// [`RETIRE_AFTER_EMPTY_PASSES`]; [`mark_funded`] resets the streak on any funded pass.
/// Idempotent whole (retry-safe under the caller's busy retry): a re-run within one pass
/// re-bumps a candidate at most once per call and a threshold-crossing re-run re-deletes nothing
/// and re-latches harmlessly; a `backfill:<index>` synthetic id has no record row and the latch
/// touches zero rows; an absent or unfunded id is skipped whole. Returns the number of
/// destination rows removed. An empty input is a no-op (no txn).
///
/// **Accepted residual (funded-then-empty is a NEGATIVE proxy, not a positive shield signal).**
/// `GetAddressUtxos` returns only UNSPENT outputs, so under an honest endpoint an emptied funded
/// destination IS shielded/spent. An endpoint lying empty for N CONSECUTIVE passes (or a reorg
/// outlasting them) can still early-retire a still-funded destination (crypto/security MINOR —
/// narrowed from ONE pass by the #386 debounce). This is bounded + NEVER a fund loss: the
/// delivery UTXO was already `put` to the engine and its address is engine-registered, so a
/// user-initiated full transparent rescan recovers it — and since #385 the served LATCH (a flag,
/// not a column clear) keeps the record's watch columns intact, so the #368 card-visit `Refunded`
/// re-arm remains the cheaper recovery for the OutOfZec arm. Documented in ADR-0530 Consequences
/// + STATUS (the #386 HARDENED note).
///
/// **Tamper posture (coarser than [`active`]'s per-row skip, deliberately):** a sealed-DB
/// `empty_streak` tampered to a non-INTEGER READS as `InvalidColumnType` here and fails the whole
/// call typed — the caller is best-effort, so the POLL PASS survives and every candidate in the
/// batch simply stays active (fail-open toward money visibility; a tamperer with sealed-DB write
/// access could DELETE the rows outright, so no capability is granted). TEXT/BLOB tamper
/// self-heals via the bump's numeric coercion, and a funded pass heals it through
/// [`mark_funded`]'s streak write. The one exception (review precision — no money impact):
/// a NULL-tampered `empty_streak` heals via NEITHER path — `NULL + 1 = NULL` (the bump) and
/// `mark_funded`'s `(funded = 0 OR empty_streak != 0)` guard evaluates `(false OR NULL) → NULL`,
/// which SQLite treats as non-matching, so the reset is skipped. That row simply never retires
/// (`active` never reads `empty_streak`, so it stays balance-visible AND a shield source — the
/// most fail-open outcome); NULL is unreachable without sealed-DB tamper of a `NOT NULL DEFAULT 0`
/// column, and it grants no capability a direct DELETE didn't.
pub(crate) fn debounced_retire_and_mark_served(
    conn: &mut Connection,
    empty_funded_ids: &[String],
) -> Result<usize, WalletError> {
    if empty_funded_ids.is_empty() {
        return Ok(0);
    }
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let mut n = 0;
    {
        // `AND funded = 1`: the streak is defined only for funded rows (the negative
        // proxy needs a prior positive); an absent/unfunded candidate — possible if a
        // concurrent `clear_all`/retire won the row between the caller's snapshot and
        // this txn — bumps nothing and is skipped whole below.
        let mut bump = tx
            .prepare(
                "UPDATE swap_destination SET empty_streak = empty_streak + 1 \
                 WHERE swap_id = ?1 AND funded = 1",
            )
            .map_err(map_db_err)?;
        let mut read = tx
            .prepare("SELECT empty_streak FROM swap_destination WHERE swap_id = ?1")
            .map_err(map_db_err)?;
        let mut del = tx
            .prepare("DELETE FROM swap_destination WHERE swap_id = ?1")
            .map_err(map_db_err)?;
        for id in empty_funded_ids {
            if bump.execute(rusqlite::params![id]).map_err(map_db_err)? == 0 {
                continue;
            }
            let streak: i64 = read
                .query_row(rusqlite::params![id], |r| r.get(0))
                .map_err(map_db_err)?;
            if streak >= RETIRE_AFTER_EMPTY_PASSES {
                n += del.execute(rusqlite::params![id]).map_err(map_db_err)?;
                crate::swap_record_store::mark_watch_served_in_tx(&tx, id)?;
            }
        }
    }
    tx.commit().map_err(map_db_err)?;
    Ok(n)
}

/// Delete every UNFUNDED destination whose retirement deadline has lapsed
/// (`expires_at_wall <= now`) — the §3.3b L3b lapse sweep (an abandoned/comparison quote past its
/// quote deadline, or a live swap past its settlement window with nothing ever delivered) — and
/// CLAMP any deadline that sits impossibly FAR-FUTURE down to one settlement window from now
/// (#368; the issued-quote far-future arm's WATCH-store form: every legitimate row's bound is
/// write-time + the settlement ceiling, so anything beyond `now + settlement + skew` is a
/// pre-IZ-1b-clamp leftover or a tampered row; unclamped it would poll its address
/// indefinitely — the "idle shrinks to index-0" defeat the quote-time clamp exists to prevent).
///
/// **CLAMP, not DELETE:** the issued-quote store DELETES far-future
/// rows because a swept quote costs only a re-quote; deleting a far-future WATCH row costs
/// poll coverage of inbound money — a >1 h backward clock step would reap a LIVE executed
/// swap's watch mid-flight. (Since #382 an UNRESOLVED record's leg is re-armed every pass
/// — [`rearm_unresolved`] — so a reaped watch is no longer unrecoverable; the clamp stays
/// as the cheaper, immediate bound, and a PINNED record's leg still has no re-arm path.)
/// Clamping bounds a tampered row to exactly one honest settlement window while a live row
/// keeps watching through it. Gated on the clock-plausibility floor (a sub-floor clock
/// cannot classify anything as far-future — the `deposit_gate` `Wait`-arm direction).
///
/// **`funded = 1` rows are SPARED by BOTH arms (#368):** a funded-but-unshielded delivery or
/// refund (honestly parked below `SHIELDING_THRESHOLD_ZAT`) must keep its poll slot and its
/// `propose_shield` source-set membership until [`debounced_retire_and_mark_served`]
/// (funded-then-empty, N consecutive passes) or a Hard-kill
/// [`clear_all`] removes it — a window lapse alone must never orphan engine-visible money out of
/// the shield's source set (there is no full-receiver-set rescan to recover it; pre-#368 this
/// pruned funded rows, silently stranding a sub-threshold delivery's shieldability at lapse).
/// The cost is bounded and honest: the row is OUR OWN funded address, polled until emptied.
///
/// Returns how many were REMOVED (the lapse arm; clamps don't count). ONE `IMMEDIATE` txn.
/// Bound hygiene for the unfunded sweep: a swept delivery that DID land but was never marked
/// funded (landed after its last poll) is still on-chain + engine-tracked and is caught by the
/// next user-initiated full sync — and, for a refund, re-armed when the user pins the
/// `Refunded` terminal (the #368 reactive re-arm).
pub(crate) fn prune_expired(conn: &mut Connection, now_unix: i64) -> Result<usize, WalletError> {
    let floor = i64::try_from(crate::constants::CLOCK_PLAUSIBILITY_FLOOR_SECS).unwrap_or(i64::MAX);
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    if now_unix >= floor {
        let settlement =
            i64::try_from(crate::constants::SWAP_SETTLEMENT_MAX_SECS).unwrap_or(i64::MAX);
        let far_future_ceiling = now_unix.saturating_add(settlement).saturating_add(
            i64::try_from(crate::constants::FAR_FUTURE_SKEW_ALLOWANCE_SECS).unwrap_or(i64::MAX),
        );
        tx.execute(
            "UPDATE swap_destination SET expires_at_wall = ?1 \
                 WHERE funded = 0 AND expires_at_wall > ?2",
            rusqlite::params![now_unix.saturating_add(settlement), far_future_ceiling],
        )
        .map_err(map_db_err)?;
    }
    // else: implausible (unsynced/1970) clock — the far-future arm is OFF (nothing can be
    // classified relative to a clock we don't trust); the lapse arm below is safe at any
    // clock (a low `now` lapses nothing).
    let removed = tx
        .execute(
            "DELETE FROM swap_destination WHERE funded = 0 AND expires_at_wall <= ?1",
            rusqlite::params![now_unix],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(removed)
}

/// Drop EVERY swap destination — the §3.5 Hard-kill honest-off (§3.3b D2): the scoped set falls
/// back to index-0 only, so zero swap-attributable traffic leaves from this instant. Returns how
/// many were removed. ONE `IMMEDIATE` txn.
pub(crate) fn clear_all(conn: &mut Connection) -> Result<usize, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let removed = tx
        .execute("DELETE FROM swap_destination", [])
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(removed)
}

/// Count the stored destinations (live + any not-yet-pruned). Test/observability helper — the
/// `open()` recovery span reads it to report `swap_destination_recovered { count }`.
pub(crate) fn count(conn: &Connection) -> Result<usize, WalletError> {
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM swap_destination", [], |r| r.get(0))
        .map_err(map_db_err)?;
    // `COUNT(*)` is non-negative; the clamp is defensive-only.
    Ok(n.max(0) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn rearm_unresolved_resurrects_a_pruned_row_and_never_shrinks_or_relinks() {
        // #382 (b): the per-pass re-arm must (1) RESURRECT a watch row a prior
        // pass's prune already deleted (the >48 h-absence shape: the row lapsed
        // and was reaped before its swap resolved), (2) only ever EXTEND a live
        // row's deadline (MAX semantics — a stale caller can't shrink a fresher
        // window), and (3) never relink a frozen address.
        let mut conn = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&conn).expect("schema");
        // A lapsed, unfunded watch row → pruned (the pre-#382 terminal state).
        mark_executed(&mut conn, "swap-a", "t1refund", 3, 1_000).expect("arm");
        prune_expired(&mut conn, 1_700_000_000).expect("prune");
        assert!(
            active(&conn, 1_700_000_000).expect("active").is_empty(),
            "the lapsed row was pruned — the exact hole the re-arm closes"
        );
        // The re-arm RESURRECTS it for a fresh window; the same pass polls it.
        rearm_unresolved(
            &mut conn,
            &[("swap-a".to_owned(), "t1refund".to_owned(), 3)],
            1_700_172_800,
        )
        .expect("rearm");
        let rows = active(&conn, 1_700_000_000).expect("active");
        assert_eq!(rows.len(), 1, "resurrected into the scoped set");
        assert_eq!(rows[0].swap_id, "swap-a");
        assert_eq!(rows[0].address, "t1refund");
        assert!(!rows[0].funded, "a resurrected row starts unfunded");
        // MAX semantics: a stale (smaller) deadline never shrinks the window…
        rearm_unresolved(
            &mut conn,
            &[("swap-a".to_owned(), "t1refund".to_owned(), 3)],
            1_700_000_100,
        )
        .expect("stale rearm");
        assert_eq!(
            active(&conn, 1_700_172_000).expect("active").len(),
            1,
            "the larger window held (never shrunk by a stale re-arm)"
        );
        // …and the frozen address is never relinked by a conflicting re-arm.
        rearm_unresolved(
            &mut conn,
            &[("swap-a".to_owned(), "t1EVIL".to_owned(), 3)],
            1_700_200_000,
        )
        .expect("conflicting rearm");
        assert_eq!(
            active(&conn, 1_700_000_000).expect("active")[0].address,
            "t1refund",
            "the address stays frozen (extend-only upsert)"
        );
        // An empty input is a no-op (no txn).
        rearm_unresolved(&mut conn, &[], 1_700_300_000).expect("empty no-op");
    }

    /// A fresh in-memory connection with the table applied — the store logic is orthogonal to
    /// SQLCipher (the [`crate::issued_quote_store`] / [`crate::refund_index`] pattern). The
    /// `swap_record` sibling table rides along since #385: [`debounced_retire_and_mark_served`]
    /// latches the record's `watch_served` in the same txn (production migrate ensures both).
    fn mem() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&conn).expect("schema");
        crate::swap_record_store::ensure_table(&conn).expect("record schema");
        conn
    }

    /// A file-backed connection for durability/concurrency, waiting out locks for `CONTENTION_BUSY_WAIT_MS`.
    fn open_file(path: &std::path::Path) -> Connection {
        let conn = Connection::open(path).expect("open file db");
        conn.busy_timeout(std::time::Duration::from_millis(
            crate::test_support::CONTENTION_BUSY_WAIT_MS,
        ))
        .expect("busy_timeout");
        ensure_table(&conn).expect("schema");
        crate::swap_record_store::ensure_table(&conn).expect("record schema");
        conn
    }

    fn addrs(active: &[ActiveDestination]) -> Vec<String> {
        active.iter().map(|a| a.address.clone()).collect()
    }

    #[test]
    fn upsert_then_active_round_trips_the_destination() {
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-1", "u1dest1", 7, 2_000_000_000).expect("upsert");
        let active = active(&conn, 1_700_000_000).expect("active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].swap_id, "swap-1");
        assert_eq!(active[0].address, "u1dest1");
        assert_eq!(active[0].index, 7);
        assert!(!active[0].funded, "a fresh quote is not yet funded");
    }

    #[test]
    fn active_excludes_a_lapsed_destination() {
        // The §3.3b D2 "idle wallet shrinks back to index-0" property: a destination past its
        // deadline is NOT in the active set (even before an explicit prune — `active` filters live).
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-live", "u1live", 1, 2_000_000_000).expect("p1");
        upsert_quoted(&mut conn, "swap-dead", "u1dead", 2, 1_000).expect("p2");
        let active = active(&conn, 1_500).expect("active");
        assert_eq!(
            addrs(&active),
            vec!["u1live".to_string()],
            "only the live one"
        );
    }

    #[test]
    fn mark_executed_extends_the_window_but_never_shrinks_it() {
        // §3.3b L3: a destination quoted with a short deadline that EXECUTES must keep watching long
        // past the quote window; and a destination with an ALREADY-longer deadline is not shrunk.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-x", "u1x", 1, 1_000).expect("quote (short deadline)");
        assert!(
            active(&conn, 2_000).expect("a").is_empty(),
            "lapsed at the quote deadline"
        );
        mark_executed(&mut conn, "swap-x", "u1x", 1, 9_000).expect("execute extends");
        assert_eq!(
            addrs(&active(&conn, 2_000).expect("a")),
            vec!["u1x".to_string()],
            "executed ⇒ watched past the quote deadline (settlement window)"
        );
        // a second mark_executed with a SHORTER settlement must not shrink the live window
        mark_executed(&mut conn, "swap-x", "u1x", 1, 3_000).expect("no-shrink");
        assert_eq!(
            addrs(&active(&conn, 5_000).expect("a")),
            vec!["u1x".to_string()],
            "MAX(existing, new) — the longer window is preserved"
        );
    }

    #[test]
    fn mark_executed_inserts_if_the_quote_time_upsert_was_lost() {
        // §3.3b L2 crash-window close: `persist` committed the issued quote but the quote-time
        // `upsert_quoted` did NOT (separate txns, a crash between). Execute then re-creates the
        // detection-set row from the durable issued-quote record, so the destination is never
        // silently absent from the poll for an executed swap.
        let mut conn = mem();
        assert_eq!(count(&conn).expect("count"), 0, "no quote-time row");
        mark_executed(&mut conn, "swap-lost", "u1lost", 4, 9_000).expect("insert-if-absent");
        let active = active(&conn, 5_000).expect("active");
        assert_eq!(
            addrs(&active),
            vec!["u1lost".to_string()],
            "re-created at execute"
        );
        assert_eq!(active[0].index, 4, "with the durable record's index");
    }

    #[test]
    fn mark_funded_then_consecutive_empty_passes_model_the_shield_signal() {
        // §3.3b L3a + #386: the poll marks a destination funded on the pass it sees a UTXO;
        // it retires only once RETIRE_AFTER_EMPTY_PASSES consecutive passes poll it EMPTY
        // (shielded/spent) — the debounce means the passes BEFORE the threshold change
        // nothing observable but the streak.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-f", "u1f", 1, 2_000_000_000).expect("quote");
        assert_eq!(
            mark_funded(&mut conn, &["swap-f".to_string()]).expect("fund"),
            1
        );
        assert!(
            active(&conn, 1_700_000_000).expect("a")[0].funded,
            "now funded"
        );
        for pass in 1..RETIRE_AFTER_EMPTY_PASSES {
            assert_eq!(
                debounced_retire_and_mark_served(&mut conn, &["swap-f".to_string()])
                    .expect("empty pass below threshold"),
                0,
                "empty pass {pass} must not retire yet (debounce)"
            );
            assert!(
                active(&conn, 1_700_000_000).expect("a")[0].funded,
                "still funded + polled below the threshold"
            );
        }
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &["swap-f".to_string()])
                .expect("threshold pass"),
            1,
            "the RETIRE_AFTER_EMPTY_PASSES-th consecutive empty pass retires"
        );
        assert!(
            active(&conn, 1_700_000_000).expect("a").is_empty(),
            "retired on shield"
        );
    }

    #[test]
    fn a_funded_pass_resets_the_empty_streak() {
        // #386 KAT (the consecutiveness property): two empty passes, then a pass that sees
        // the row funded again (a lying endpoint healed / a reorg un-wound), then two more
        // empty passes — the row must survive ALL of it; only a FULL run of
        // RETIRE_AFTER_EMPTY_PASSES consecutive empties retires. Without the reset a
        // non-consecutive accumulator would retire on the 3rd empty overall — exactly the
        // one-lie stickiness the debounce exists to absorb.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-blip", "u1blip", 1, 2_000_000_000).expect("quote");
        mark_funded(&mut conn, &["swap-blip".to_string()]).expect("fund");
        for _ in 1..RETIRE_AFTER_EMPTY_PASSES {
            assert_eq!(
                debounced_retire_and_mark_served(&mut conn, &["swap-blip".to_string()])
                    .expect("empty"),
                0
            );
        }
        // The healed pass: the poll sees the UTXO again → mark_funded resets the streak
        // (and reports the row CHANGED — the streak write, not a funded flip).
        assert_eq!(
            mark_funded(&mut conn, &["swap-blip".to_string()]).expect("healed pass"),
            1,
            "a streak-carrying re-mark is a real write (the reset)"
        );
        for pass in 1..RETIRE_AFTER_EMPTY_PASSES {
            assert_eq!(
                debounced_retire_and_mark_served(&mut conn, &["swap-blip".to_string()])
                    .expect("empty after heal"),
                0,
                "post-heal empty pass {pass} starts the run over"
            );
        }
        assert!(
            !active(&conn, 1_700_000_000).expect("a").is_empty(),
            "the interrupted run never accumulated to the threshold"
        );
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &["swap-blip".to_string()])
                .expect("full run"),
            1,
            "a FULL consecutive run still retires"
        );
    }

    #[test]
    fn debounce_ignores_a_never_funded_row() {
        // The streak is defined only for funded rows (the negative proxy needs a prior
        // positive): a never-funded candidate — unreachable from the caller's own filter,
        // but the store guard must hold on its own — bumps nothing and never retires.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-unfunded", "u1uf", 1, 2_000_000_000).expect("quote");
        for _ in 0..RETIRE_AFTER_EMPTY_PASSES + 1 {
            assert_eq!(
                debounced_retire_and_mark_served(&mut conn, &["swap-unfunded".to_string()])
                    .expect("no-op"),
                0
            );
        }
        assert_eq!(
            addrs(&active(&conn, 1_700_000_000).expect("a")),
            vec!["u1uf".to_string()],
            "the unfunded row is untouched — its exit is the lapse prune"
        );
    }

    #[test]
    fn rearm_preserves_a_mid_streak_row() {
        // review pin: the per-pass re-arm (`arm_in_tx` on an EXISTING row)
        // must never touch `empty_streak` — its `DO UPDATE` lists ONLY
        // `expires_at_wall`. A re-arm resetting the streak would make an
        // unresolved record's watch UNRETIRABLE (re-armed every pass ⇒ streak
        // forever 0); a re-arm bumping it would fake empty observations.
        let mut conn = mem();
        let now = 1_700_000_000_i64;
        mark_executed(&mut conn, "swap-mid", "t1mid", 1, now + 100).expect("arm");
        mark_funded(&mut conn, &["swap-mid".to_string()]).expect("fund");
        for _ in 1..RETIRE_AFTER_EMPTY_PASSES {
            debounced_retire_and_mark_served(&mut conn, &["swap-mid".to_string()]).expect("bump");
        }
        let read_streak = |conn: &Connection| -> i64 {
            conn.query_row(
                "SELECT empty_streak FROM swap_destination WHERE swap_id = 'swap-mid'",
                [],
                |r| r.get(0),
            )
            .expect("row")
        };
        let mid = read_streak(&conn);
        assert_eq!(mid, RETIRE_AFTER_EMPTY_PASSES - 1, "mid-streak setup");
        rearm_unresolved(
            &mut conn,
            &[("swap-mid".to_owned(), "t1mid".to_owned(), 1)],
            now + 200_000,
        )
        .expect("rearm");
        assert_eq!(
            read_streak(&conn),
            mid,
            "the re-arm extend preserves the debounce streak verbatim"
        );
    }

    #[test]
    fn ensure_table_adds_empty_streak_to_a_pre_386_table_defaulting_zero() {
        // The additive migration (#386): a table created by a pre-#386 build gains
        // `empty_streak` on the next open, defaulting 0 for every existing row — correct
        // (no empty passes were noted yet), and strictly the safer direction: a
        // pre-upgrade funded row needs a FULL fresh run of consecutive empties.
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE swap_destination (
                 swap_id         TEXT PRIMARY KEY,
                 address         TEXT NOT NULL,
                 address_index   INTEGER NOT NULL,
                 expires_at_wall INTEGER NOT NULL,
                 funded          INTEGER NOT NULL DEFAULT 0
             );
             INSERT INTO swap_destination (swap_id, address, address_index, expires_at_wall, funded)
                 VALUES ('swap-old', 'u1old', 5, 2000000000, 1);",
        )
        .expect("a pre-#386 table with a funded row");
        ensure_table(&conn).expect("migrate additively");
        crate::swap_record_store::ensure_table(&conn).expect("record schema");
        let streak: i64 = conn
            .query_row(
                "SELECT empty_streak FROM swap_destination WHERE swap_id = 'swap-old'",
                [],
                |r| r.get(0),
            )
            .expect("the migrated column reads");
        assert_eq!(streak, 0, "a pre-#386 row starts its streak at zero");
        let mut conn = conn;
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &["swap-old".to_string()])
                .expect("first post-upgrade empty pass"),
            0,
            "the migrated funded row gets the full debounce window"
        );
    }

    #[test]
    fn mark_funded_and_retire_are_noops_on_empty_input() {
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-1", "u1a", 1, 2_000_000_000).expect("quote");
        assert_eq!(mark_funded(&mut conn, &[]).expect("fund none"), 0);
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &[]).expect("retire none"),
            0
        );
        assert_eq!(count(&conn).expect("count"), 1, "nothing touched");
    }

    #[test]
    fn mark_funded_and_retire_are_noops_on_an_absent_id() {
        // The poll computes its mark/retire id lists from a snapshot read under an EARLIER aux lock
        // that has since been released (wallet.rs `advance_destination_store`). A concurrent
        // `prune_expired` (a lapse sweep) or a Hard-kill `clear_all` can DELETE a row in that gap, so
        // the post-put advance can name a `swap_id` no longer present. Both primitives must treat an
        // absent id as a 0-row no-op (never an error) — a "missed-by-one-tick update self-heals next
        // pass" (the documented benign race), not a poisoned poll. Distinct from the empty-input
        // no-op: here a NON-empty list names an id that simply isn't in the table.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-present", "u1present", 1, 2_000_000_000).expect("quote");
        assert_eq!(
            mark_funded(&mut conn, &["swap-GONE".to_string()]).expect("fund absent"),
            0,
            "marking an absent id touches zero rows, no error"
        );
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &["swap-GONE".to_string()])
                .expect("retire absent"),
            0,
            "an absent id bumps nothing and removes zero rows, no error"
        );
        // the present row is wholly untouched by either no-op
        let active = active(&conn, 1_700_000_000).expect("active");
        assert_eq!(addrs(&active), vec!["u1present".to_string()]);
        assert!(
            !active[0].funded,
            "the present row was NOT collaterally funded by a mark of an absent id"
        );
    }

    #[test]
    fn mark_funded_mixed_present_and_absent_ids_marks_only_the_present() {
        // A poll pass whose funded set names one live destination AND one already-pruned id (the
        // benign gap above): only the live row flips to funded, the absent id is silently skipped,
        // and the returned count reflects ONLY the rows actually touched.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-live", "u1live", 1, 2_000_000_000).expect("quote");
        let touched = mark_funded(
            &mut conn,
            &["swap-live".to_string(), "swap-vanished".to_string()],
        )
        .expect("mark mixed");
        assert_eq!(touched, 1, "only the present row was marked");
        assert!(
            active(&conn, 1_700_000_000).expect("a")[0].funded,
            "the live destination is funded; the absent id was a no-op"
        );
    }

    #[test]
    fn prune_expired_boundary_a_destination_expiring_exactly_at_now_is_pruned() {
        // The `<=` retirement edge (the issued_quote_store prune-boundary mirror): a deadline that
        // EQUALS now is pruned, one tick later survives.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-at", "u1at", 1, 1_000).expect("p1");
        upsert_quoted(&mut conn, "swap-after", "u1after", 2, 1_001).expect("p2");
        assert_eq!(prune_expired(&mut conn, 1_000).expect("prune"), 1);
        assert_eq!(
            addrs(&active(&conn, 0).expect("a")),
            vec!["u1after".to_string()],
            "the now+1 destination survives the `<=` boundary"
        );
    }

    #[test]
    fn clear_all_drops_every_destination_hard_kill() {
        // §3.5 Hard-kill honest-off (§3.3b D2): the set falls back to index-0 only.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-1", "u1a", 1, 2_000_000_000).expect("p1");
        upsert_quoted(&mut conn, "swap-2", "u1b", 2, 2_000_000_000).expect("p2");
        assert_eq!(clear_all(&mut conn).expect("clear"), 2);
        assert!(
            active(&conn, 1_700_000_000).expect("a").is_empty(),
            "all dropped"
        );
    }

    #[test]
    fn upsert_is_first_wins_on_address_but_refreshes_the_deadline() {
        // A re-quote that reused the id can never relink the address (the issued-quote first-wins
        // mirror — an attacker that flipped the recipient could redirect a delivery), but it DOES
        // extend the watch window, and `funded` is preserved.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-1", "u1original", 1, 1_000).expect("first");
        mark_funded(&mut conn, &["swap-1".to_string()]).expect("fund");
        upsert_quoted(&mut conn, "swap-1", "u1ATTACKER", 2, 9_000).expect("re-quote");
        let active = active(&conn, 5_000).expect("active");
        assert_eq!(active.len(), 1);
        assert_eq!(
            active[0].address, "u1original",
            "first-wins — never relinked"
        );
        assert_eq!(active[0].index, 1, "the original index is frozen too");
        assert!(active[0].funded, "funded is preserved across a re-upsert");
    }

    #[test]
    fn active_skips_a_tampered_row_and_keeps_serving_the_rest() {
        // #385 (the set-build MED): a row that cannot READ — an index beyond the
        // u32 domain, or a TEXT-typed index under SQLite's dynamic typing (both only via
        // sealed-DB tamper; the allocator floors at 1, caps at 2^31-1) — is SKIPPED
        // fail-honest, exactly like `unresolved_watches`' posture on the same tamper.
        // Pre-#385 this was fail-closed `StoreCorrupt`, which let ONE tampered row kill
        // EVERY transparent poll pass forever (index-0 detection included), silently,
        // rescan-survivingly. Never a panic at the consumer either way. The ids order
        // the TYPE-tampered row FIRST (`ORDER BY swap_id`) so the test also pins that
        // iteration proceeds PAST a row-read error to healthy rows behind it.
        let conn = mem();
        conn.execute(
            "INSERT INTO swap_destination (swap_id, address, address_index, expires_at_wall, funded) \
                 VALUES ('swap-big', 'u1dest', 4294967296, 2000000000, 0)",
            [],
        )
        .expect("seed an over-u32 index");
        conn.execute(
            "INSERT INTO swap_destination (swap_id, address, address_index, expires_at_wall, funded) \
                 VALUES ('swap-0text', 'u1text', 'not-a-number', 2000000000, 0)",
            [],
        )
        .expect("seed a type-tampered index (sorts FIRST)");
        conn.execute(
            "INSERT INTO swap_destination (swap_id, address, address_index, expires_at_wall, funded) \
                 VALUES ('swap-good', 'u1good', 7, 2000000000, 0)",
            [],
        )
        .expect("seed a healthy sibling");
        let active = active(&conn, 1_700_000_000).expect("skip, never a pass-killing error");
        assert_eq!(
            addrs(&active),
            vec!["u1good".to_string()],
            "both tampered rows are skipped; the healthy sibling keeps polling"
        );
    }

    #[test]
    fn retire_latches_watch_served_atomically_and_preserves_the_columns() {
        // #385 (the headline MED): retire + the record's served latch are ONE aux
        // txn — a kill between two separate txns durably re-opened the retire→re-arm
        // churn (the id never re-enters a retire list, so the latch was never retried).
        // The latch is a FLAG, not a column clear: `unresolved_watches` (the per-pass
        // re-arm source) must drop the leg while `watch_of` (the card-visit re-arm's
        // source — the lying-endpoint early-retire heal) must still return it.
        let mut conn = mem();
        crate::swap_record_store::insert_with_watch(
            &mut conn,
            &crate::swap_record_store::InFlightSwap {
                swap_id: "swap-served".into(),
                out_of_zec: false,
                created_at: 1_700_000_000,
                deposit_deadline: None,
                expires_at_wall: 1_700_172_800,
                outcome: None,
            },
            1_700_000_000,
            "0xswapserved",
            Some(("u1dest", 9)),
        )
        .expect("record + watch");
        mark_funded(&mut conn, &["swap-served".to_string()]).expect("fund");
        assert_eq!(
            crate::swap_record_store::unresolved_watches(&conn)
                .expect("watches")
                .len(),
            1,
            "the unresolved leg re-arms before retire"
        );
        // Drive the #386 debounce to its threshold — the leg must stay unserved (and so
        // keep re-arming) on every below-threshold empty pass.
        for _ in 1..RETIRE_AFTER_EMPTY_PASSES {
            assert_eq!(
                debounced_retire_and_mark_served(&mut conn, &["swap-served".to_string()])
                    .expect("below threshold"),
                0
            );
            assert_eq!(
                crate::swap_record_store::unresolved_watches(&conn)
                    .expect("watches")
                    .len(),
                1,
                "a below-threshold empty pass must NOT latch the leg served"
            );
        }
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &["swap-served".to_string()])
                .expect("retire"),
            1
        );
        assert!(
            active(&conn, 1_700_000_000).expect("active").is_empty(),
            "the destination row is gone"
        );
        assert!(
            crate::swap_record_store::unresolved_watches(&conn)
                .expect("watches")
                .is_empty(),
            "the served leg stops per-pass re-arming — the churn ends"
        );
        assert_eq!(
            crate::swap_record_store::watch_of(&conn, "swap-served").expect("watch_of"),
            Some(("u1dest".to_string(), 9, false)),
            "the watch COLUMNS survive the latch — the card-visit re-arm heal remains"
        );
        // Idempotent whole (the caller's busy retry re-runs the txn): nothing re-deletes,
        // the latch re-marks harmlessly (the deleted row can't bump, so the guard skips).
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &["swap-served".to_string()])
                .expect("re-run"),
            0
        );
    }

    #[test]
    fn rearm_skips_a_row_with_at_least_half_a_window_left() {
        // #385 (the write-throttle MED): the per-pass re-arm must NOT rewrite a
        // watch row whose deadline is still >= half a settlement window out — pre-#385
        // the MAX upsert dirtied+fsynced every watch row on every ~75 s pass for the
        // unbounded life of a stuck unresolved record. Below the midpoint it extends;
        // an ABSENT row (the resurrect arm) always inserts.
        let mut conn = mem();
        let now = 1_700_000_000_i64;
        let settlement = crate::constants::SWAP_SETTLEMENT_MAX_SECS as i64;
        let deadline = now + settlement; // the caller's now + settlement
        // Above the midpoint (>= now + ½ settlement): the extend is SKIPPED.
        mark_executed(
            &mut conn,
            "swap-fresh",
            "t1fresh",
            1,
            now + settlement / 2 + 10,
        )
        .expect("arm fresh");
        // Below the midpoint: the extend RUNS (MAX semantics).
        mark_executed(&mut conn, "swap-stale", "t1stale", 2, now + 100).expect("arm stale");
        rearm_unresolved(
            &mut conn,
            &[
                ("swap-fresh".to_owned(), "t1fresh".to_owned(), 1),
                ("swap-stale".to_owned(), "t1stale".to_owned(), 2),
                ("swap-absent".to_owned(), "t1absent".to_owned(), 3),
            ],
            deadline,
        )
        .expect("rearm");
        let read = |id: &str| -> i64 {
            conn.query_row(
                "SELECT expires_at_wall FROM swap_destination WHERE swap_id = ?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .expect("row")
        };
        assert_eq!(
            read("swap-fresh"),
            now + settlement / 2 + 10,
            "a row with >= half a window left is untouched (zero-write skip)"
        );
        assert_eq!(
            read("swap-stale"),
            deadline,
            "a row below the midpoint extends to the fresh window"
        );
        assert_eq!(
            read("swap-absent"),
            deadline,
            "an absent row is resurrected unconditionally"
        );
    }

    #[test]
    fn mark_funded_re_mark_is_a_zero_write_noop() {
        // #385 write-throttle sibling: a parked sub-threshold funded row is re-attributed
        // on every pass — the re-mark must touch zero rows (no page dirty, no fsync).
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-parked", "u1parked", 1, 2_000_000_000).expect("quote");
        assert_eq!(
            mark_funded(&mut conn, &["swap-parked".to_string()]).expect("first mark"),
            1,
            "the first mark flips the row"
        );
        assert_eq!(
            mark_funded(&mut conn, &["swap-parked".to_string()]).expect("re-mark"),
            0,
            "the re-mark is a zero-write no-op"
        );
        assert!(
            active(&conn, 1_700_000_000).expect("active")[0].funded,
            "still funded — the guard changed the write count, not the state"
        );
    }

    #[test]
    fn destinations_survive_reopen_for_crash_recovery() {
        // §3.3b L2 crash recovery BY CONSTRUCTION: a destination upserted before a kill is STILL
        // there after a reopen, so the first post-open poll re-derives the scoped set from it.
        let file = tempfile::NamedTempFile::new().expect("temp db");
        let path = file.path().to_owned();
        {
            let mut conn = open_file(&path);
            upsert_quoted(&mut conn, "swap-survive", "u1survive", 3, 2_000_000_000)
                .expect("upsert");
            mark_executed(&mut conn, "swap-survive", "u1survive", 3, 2_000_000_100)
                .expect("execute");
        } // connection dropped — a simulated process kill mid-swap
        {
            let conn = open_file(&path);
            let active = active(&conn, 1_700_000_000).expect("active after reopen");
            assert_eq!(
                addrs(&active),
                vec!["u1survive".to_string()],
                "survived the kill"
            );
        }
    }

    #[test]
    fn ensure_table_is_idempotent_and_preserves_rows() {
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-keep", "u1keep", 1, 2_000_000_000).expect("upsert");
        ensure_table(&conn).expect("re-ensure"); // every open re-runs this
        assert_eq!(
            count(&conn).expect("count"),
            1,
            "re-running ensure_table dropped a row — a lost detection-set entry"
        );
    }

    #[test]
    fn concurrent_upserts_of_distinct_ids_never_collide() {
        // The aux `IMMEDIATE` discipline under real contention (the refund_index / issued_quote
        // concurrency mirror): N threads each upserting a DISTINCT destination all land, no torn
        // rows, no lost writes, no deadlock against each other.
        const N: usize = 12;
        let file = tempfile::NamedTempFile::new().expect("temp db");
        let path = Arc::new(file.path().to_owned());
        // create the schema once up front
        {
            let _ = open_file(&path);
        }
        let barrier = Arc::new(Barrier::new(N));
        let handles: Vec<_> = (0..N)
            .map(|i| {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let mut conn = open_file(&path);
                    barrier.wait();
                    upsert_quoted(
                        &mut conn,
                        &format!("swap-{i}"),
                        &format!("u1dest{i}"),
                        (i + 1) as u32,
                        2_000_000_000,
                    )
                    .expect("upsert under contention");
                })
            })
            .collect();
        for h in handles {
            h.join().expect("join");
        }
        let conn = open_file(&path);
        assert_eq!(
            count(&conn).expect("count"),
            N,
            "every distinct destination landed under contention"
        );
    }

    #[test]
    fn prune_and_active_spare_a_funded_row_until_retire() {
        // #368: a funded-but-unshielded row (a sub-threshold delivery/refund honestly parked
        // below the shielding threshold) keeps its poll slot AND its `propose_shield`
        // source-set membership past ANY deadline — a window lapse alone must never orphan
        // engine-visible money out of the shield's source set (pre-#368 this pruned funded
        // rows, silently stranding shieldability). The funded exits are retire + clear_all.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-f", "u1funded", 1, 1_700_000_100).expect("quote");
        mark_funded(&mut conn, &["swap-f".to_string()]).expect("fund");
        assert_eq!(
            prune_expired(&mut conn, 1_900_000_000).expect("prune"),
            0,
            "a funded row is spared long past its lapsed deadline"
        );
        let a = active(&conn, 1_900_000_000).expect("active");
        assert_eq!(
            addrs(&a),
            vec!["u1funded".to_string()],
            "still polled + shieldable past lapse (the prune-spare's twin in active())"
        );
        assert!(a[0].funded);
        for _ in 1..RETIRE_AFTER_EMPTY_PASSES {
            assert_eq!(
                debounced_retire_and_mark_served(&mut conn, &["swap-f".to_string()])
                    .expect("below threshold"),
                0
            );
        }
        assert_eq!(
            debounced_retire_and_mark_served(&mut conn, &["swap-f".to_string()]).expect("retire"),
            1
        );
        assert!(
            active(&conn, 1_900_000_000).expect("a").is_empty(),
            "retire (funded-then-empty, debounced) is the funded row's exit"
        );
    }

    #[test]
    fn prune_clamps_a_far_future_unfunded_row_only_at_a_plausible_clock() {
        // #368 far-future arm, WATCH-store form: a deadline no sane
        // clock could have written (a pre-IZ-1b-clamp leftover or a tampered row) is CLAMPED
        // to one settlement window at a plausible clock — never DELETED, because a >1 h
        // backward clock step must not reap a LIVE executed swap's watch mid-flight (the
        // issued-quote store deletes because a swept quote costs only a re-quote; a swept
        // WATCH costs poll coverage of inbound money). A sub-floor clock disables the arm.
        let mut conn = mem();
        upsert_quoted(&mut conn, "swap-ff", "u1ff", 1, i64::MAX - 10).expect("tampered");
        upsert_quoted(&mut conn, "swap-live", "u1live", 2, 1_700_003_600).expect("live");
        assert_eq!(
            prune_expired(&mut conn, 1_000).expect("prune at 1970"),
            0,
            "the far-future arm is OFF at an implausible clock"
        );
        let a = active(&conn, 1_700_000_000).expect("still both");
        assert_eq!(a.len(), 2, "nothing deleted at the implausible clock");
        assert_eq!(
            prune_expired(&mut conn, 1_700_000_000).expect("prune"),
            0,
            "the far-future arm CLAMPS — it never deletes"
        );
        let a = active(&conn, 1_700_000_000).expect("active");
        assert_eq!(
            addrs(&a),
            vec!["u1ff".to_string(), "u1live".to_string()],
            "the tampered row keeps watching — bounded, not reaped"
        );
        // …bounded to EXACTLY one settlement window from the clamp instant: past it, the
        // clamped row lapses out via the ordinary arm while the live row survives.
        let one_window = 1_700_000_000 + crate::constants::SWAP_SETTLEMENT_MAX_SECS as i64;
        assert_eq!(
            prune_expired(&mut conn, one_window).expect("prune past the clamp"),
            2,
            "the clamped row AND the (long-lapsed) live row reap via the lapse arm"
        );
        // a FUNDED far-future row is spared by BOTH arms (money-first — retire is its exit)
        upsert_quoted(&mut conn, "swap-ff2", "u1ff2", 3, i64::MAX - 10).expect("tampered 2");
        mark_funded(&mut conn, &["swap-ff2".to_string()]).expect("fund");
        assert_eq!(
            prune_expired(&mut conn, 1_700_000_000).expect("prune funded"),
            0,
            "a funded far-future row is spared — retire is its exit"
        );
        let a = active(&conn, 1_700_000_000).expect("active");
        assert_eq!(a.len(), 1);
        assert_eq!(
            a[0].address, "u1ff2",
            "and its deadline was NOT clamped (funded rows are untouched by both arms)"
        );
    }

    #[test]
    fn arm_backfill_arms_synthetic_rows_and_advances_the_marker_atomically() {
        // #368 / ADR-0527 backfill: owed indices get one-window synthetic watch rows; an
        // INDEX a live watch row already covers — or a LIVE issued quote still references —
        // is SKIPPED (keyed on the index, NOT the address string: a refund watch row stores
        // the bare t-addr while the backfill entry carries the UA form of the SAME receiver,
        // the review fix); the registration marker advances in the SAME txn; a re-run
        // is a no-op (idempotent).
        let mut conn = mem();
        crate::refund_index::ensure_registration_table(&conn).expect("marker schema");
        crate::issued_quote_store::ensure_table(&conn).expect("quote schema");
        crate::refund_index::seed_backfill_bounds(&conn, 5).expect("seed bounds");
        // index 2 has a LIVE watch row in a DIFFERENT string form (t-addr vs the entry's
        // UA form) — the index-keyed skip must still catch it
        upsert_quoted(&mut conn, "swap-live", "t1addr2", 2, 2_000_000_000).expect("live row");
        // index 4 is referenced by a LIVE issued quote (a pending refund) — skipped too:
        // it arms properly at its own execute, and polling it pre-execute would leak
        crate::issued_quote_store::persist(
            &mut conn,
            &crate::issued_quote_store::StoredQuote {
                id: "q-pending".into(),
                deposit_address: Some("t1dep".into()),
                deposit_amount_zat: Some(100_000),
                expires_at_wall: 2_000_000_000,
                destination_address: None,
                destination_index: None,
                refund_address: Some("t1refund4".into()),
                refund_index: Some(4),
                binding: None,
                provider_ref: Some("t1dep".into()),
                term_deposit_address: Some("t1dep".into()),
                term_deposit_memo: None,
                term_amount_in: Some("1.0".into()),
                term_min_amount_out: Some("41.5".into()),
                term_zec_side_zat: Some(100_000),
                term_refund_to: Some("t1refund4".into()),
                term_expires_at: Some(2_000_000_000),
            },
            16,
            1_700_000_000,
        )
        .expect("pending quote");
        let entries = vec![
            (1u32, "u1back1".to_string()),
            (2u32, "u1back2-ua-form-of-t1addr2".to_string()),
            (3u32, "u1back3".to_string()),
            (4u32, "u1back4-ua-form-of-t1refund4".to_string()),
        ];
        let armed = arm_backfill(&mut conn, &entries, 1_900_000_000, 4).expect("backfill");
        assert_eq!(
            armed, 2,
            "the live-watched index AND the live-quote index were skipped"
        );
        let a = active(&conn, 1_700_000_000).expect("active");
        assert_eq!(a.len(), 3, "backfill:1 + backfill:3 + the live row");
        assert!(
            a.iter().any(|d| d.swap_id == "backfill:1" && d.index == 1),
            "synthetic key carries the index"
        );
        assert!(a.iter().any(|d| d.swap_id == "backfill:3"));
        assert!(
            !a.iter().any(|d| d.index == 4),
            "the pending quote's refund index is NOT polled pre-execute"
        );
        assert_eq!(
            crate::refund_index::registered_up_to(&conn).expect("marker"),
            4,
            "the marker advanced atomically with the batch (past skipped indices too)"
        );
        assert_eq!(
            arm_backfill(&mut conn, &entries, 1_900_000_000, 4).expect("re-run"),
            0,
            "a re-run double-arms nothing"
        );
        assert_eq!(count(&conn).expect("count"), 3);
    }

    #[test]
    fn arm_in_tx_deletes_a_same_index_backfill_twin() {
        // #390 DUAL-ROW RULE: a synthetic `backfill:<index>` row left by a deep-scan widen
        // during the mid-issuance race is SUPERSEDED when that same index later EXECUTES —
        // the execute arm (`mark_executed` → `arm_in_tx`) deletes the twin in the SAME txn,
        // restoring the one-row-per-receiver rule (no double-attribution of
        // funded/retire signals).
        let mut conn = mem();
        // the leftover synthetic backfill row at index 5
        upsert_quoted(&mut conn, "backfill:5", "u1b5", 5, 2_000_000_000).expect("backfill twin");
        // the real swap at index 5 executes
        mark_executed(&mut conn, "swap-real", "t1refund5", 5, 2_100_000_000).expect("execute arm");
        let a = active(&conn, 1_700_000_000).expect("active");
        let at_index_5: Vec<_> = a.iter().filter(|d| d.index == 5).collect();
        assert_eq!(
            at_index_5.len(),
            1,
            "exactly one row per receiver after the execute arm — the twin is gone"
        );
        assert_eq!(
            at_index_5[0].swap_id, "swap-real",
            "the real execute row supersedes the synthetic twin"
        );
        assert!(
            !a.iter().any(|d| d.swap_id == "backfill:5"),
            "the same-index backfill twin was deleted in the arm txn"
        );
        // A DIFFERENT-index backfill row is untouched (the delete is index-exact).
        upsert_quoted(&mut conn, "backfill:6", "u1b6", 6, 2_000_000_000).expect("other backfill");
        mark_executed(&mut conn, "swap-real2", "t1refund7", 7, 2_100_000_000).expect("execute 7");
        assert!(
            active(&conn, 1_700_000_000)
                .expect("active")
                .iter()
                .any(|d| d.swap_id == "backfill:6"),
            "a backfill row at a DIFFERENT index must survive an unrelated execute arm"
        );
    }

    #[cfg(feature = "swap")]
    #[test]
    fn has_unexpired_backfill_row_paces_deep_scan_reruns() {
        // #390 refusal-ii: the deep-scan pacing gate reads TRUE only while a prior sweep's
        // `backfill:` watch row is still UNEXPIRED (one outstanding band per settlement
        // window). Real (non-`backfill:`) rows never count; a past-deadline row — funded or
        // not — never blocks a rerun.
        let mut conn = mem();
        let now = 1_700_000_000;
        assert!(
            !has_unexpired_backfill_row(&conn, now).expect("empty"),
            "no backfill rows ⇒ no outstanding band"
        );
        // a real swap row, unexpired, is NOT in the reserved namespace ⇒ does not pace
        upsert_quoted(&mut conn, "swap-x", "u1x", 7, now + 1_000).expect("real row");
        assert!(
            !has_unexpired_backfill_row(&conn, now).expect("real-only"),
            "a real swap row must never be read as an outstanding deep-scan band"
        );
        // an UNEXPIRED backfill row ⇒ a band is outstanding
        upsert_quoted(&mut conn, "backfill:9", "u1b9", 9, now + 1_000).expect("unexpired backfill");
        assert!(
            has_unexpired_backfill_row(&conn, now).expect("unexpired"),
            "an unexpired backfill row must pace reruns"
        );
        // past its deadline it no longer paces — even once FUNDED (the window, not funded
        // state, governs the elective-growth cap; found money parks via the balance path)
        mark_funded(&mut conn, &["backfill:9".to_string()]).expect("fund");
        assert!(
            !has_unexpired_backfill_row(&conn, now + 2_000).expect("expired-funded"),
            "a past-deadline backfill row must not block a rerun, funded or not"
        );
    }
}
