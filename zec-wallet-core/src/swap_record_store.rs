//! The durable in-flight swap RECORD — the user-visibility home row (W-swap-5, #366).
//!
//! Three swap stores, three concerns (deliberately NOT merged — each has its own
//! lifecycle): [`crate::issued_quote_store`] is the execute SINGLE-FLIGHT (consumed
//! at execute — nothing survives it), [`crate::swap_destination_store`] is the
//! IntoZec DETECTION set (scoped poll state, cleared on a §3.5 Hard kill), and THIS
//! store is the durable fact "an order was registered at execute" — the re-attach
//! handle a UI home surface lists after a process death, screen re-entry, or
//! session swap. Pre-#366 that fact lived only in host RAM: kill→relaunch
//! mid-armed-window left a live OutOfZec deposit visible NOWHERE while the §4.4
//! in-flight guard refused new swaps.
//!
//! **Honest durable minimum.** `swap_id` is the SDK-minted execution identity (S8;
//! feed it to `watch_swap_status` to re-attach — the poll resolves the provider's
//! own handle, `provider_ref`, from this row before it spawns). The direction is
//! the COARSE arm only — no asset names, no amounts are durably stored (data
//! minimization; the UI polls provider truth live). There is NO LIVE-status column: the row asserts
//! registration — plus, since #367, at most one host-pinned OBSERVED TERMINAL
//! [`SwapOutcome`] (a terminal never goes stale, so pinning it is not the
//! snapshot-staleness violation a live status column would be).
//!
//! **Lifecycle.** `insert` at execute — first-wins, with a floor-gated inline
//! prune (lapsed + far-future rows) and a [`MAX_SWAP_RECORDS`] cap eviction
//! that EXEMPTS the just-inserted id, so the fresh registration survives by
//! construction (see `insert`'s contract for the clock-edge reasoning —
//! display-only records, money-safe either way). [`record_outcome`] on the
//! host's terminal OBSERVATION (#367 — first-wins UPDATE; pre-#367 the host
//! deleted here, which erased outcomes the user never saw: a `Refunded` that
//! landed while the user was away vanished unrendered, and a process death in
//! the gap erased it durably). [`dismiss`] is USER-INTENT only (idempotent
//! DELETE — the terminal card's Done after the outcome rendered, or the home
//! row's explicit Remove). Self-lapse is PINNED-ONLY since #382 (the
//! converged HIGH): a PINNED row self-lapses out of [`list_in_flight`] at
//! `expires_at_wall` (execute-time + the settlement ceiling — the outcome had
//! a full window to be seen) and is physically reaped by the next insert's
//! prune; an UNPINNED row (`outcome IS NULL` at the SQL level) stays listed
//! and is SPARED by the prune's lapse arm — an unresolved swap is money in
//! motion, and pre-#382 the row, its watch, and the Refunded re-arm all died
//! on that one wall-clock bound, so a refund landing >48 h after execute with
//! the app closed landed invisibly. Its only exits are the user's Remove, a
//! later pinned terminal's lapse, and the cap eviction.
//!
//! **Reads are NOT swap traffic (§3.5).** This table is ensured and read
//! UNGATED — records stay readable when swap is disabled, wound down, or
//! Hard-killed (a local read sends zero provider packets), and a Hard kill's
//! `clear_all` on the DETECTION set deliberately does not touch this store:
//! erasing the user's own knowledge of money in motion is not "honest-off".
//! Rescan: rides [`crate::db`]'s `AUX_TABLES_PRESERVED` copy UNCONDITIONALLY
//! (ADR-0534 — schema is build-independent so a non-swap build's rescan never
//! drops a swap-build's in-flight state).
//!
//! **Storage + deadlock-freedom.** A small table in the ONE sealed/wiped/
//! backup-excluded `wallet.db`, on the SECOND SQLCipher aux connection
//! (`Inner.aux_db`), exactly like its two siblings: every write is ONE
//! `BEGIN IMMEDIATE` transaction (takes `RESERVED` atomically — never deadlocks
//! the engine's `DEFERRED` writer). Created idempotently in `db::migrate`, so an
//! existing wallet gains it on next open with NO `WALLET_SCHEMA_VERSION` bump.
//! Functions take a bare `&mut Connection`/`&Connection` so they unit-test
//! against a plain connection with no `Wallet`.
//!
//! §5.4: `provider_ref` is the shipped provider's deposit address — a NEVER-LOG
//! item — and `swap_id` links log lines to one user's swap. This module
//! stores/returns them, never logs them; only COUNTS are observable upstream.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::constants::{
    CLOCK_PLAUSIBILITY_FLOOR_SECS, FAR_FUTURE_SKEW_ALLOWANCE_SECS, MAX_SWAP_RECORDS,
    SWAP_SETTLEMENT_MAX_SECS,
};
use crate::db::map_aux_err as map_db_err;
use crate::error::WalletError;

/// One durable in-flight swap row, reconstructed for the host's home surface
/// (the public return of `Wallet::list_in_flight_swaps` — re-exported at the
/// crate root). `deposit_deadline` is when the DEPOSIT must land (OutOfZec:
/// the clamped §4.4 tag the wallet's own send gates on; IntoZec: the quote's
/// deposit window the USER must beat) — display-only here, `None` if the
/// executing build predates it. `expires_at_wall` is the record's own
/// self-lapse bound — PINNED rows only since #382: an unpinned row lists
/// past it (rendered as "unresolved past the settlement window"), because
/// vanishing on a wall clock alone was the money-visibility HIGH.
/// `outcome` is the host-pinned observed terminal (#367) —
/// `None` while genuinely in flight AND for a stored value this build does
/// not recognize (fail-honest: an unknown outcome renders as in-flight and
/// the live poll re-reveals provider truth, never a wrong terminal).
///
/// **§5.4 NEVER-log:** `swap_id` — render-only, never log. The provider's own
/// handle (the deposit address) is on the row but NOT on this DTO: it is read by
/// [`provider_ref_of`] for the poll and never crosses the FFI.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InFlightSwap {
    /// The SDK-minted execution identity (S8) — THE re-attach handle.
    pub swap_id: String,
    /// The coarse direction arm: `true` = OutOfZec (the wallet sends the
    /// deposit), `false` = IntoZec (the user deposits externally).
    pub out_of_zec: bool,
    pub created_at: i64,
    pub deposit_deadline: Option<i64>,
    pub expires_at_wall: i64,
    pub outcome: Option<SwapOutcome>,
}

/// The observed TERMINAL a host may pin into a record (#367) — the coarse,
/// provider-agnostic terminal arms of `SwapStatus` (`Success`/`Refunded`/
/// `Failed`), no payloads (txids/amounts stay live-polled + never durable —
/// data minimization; the pin exists so the OUTCOME CLASS survives the user
/// being away at observation time, nothing more).
//
// DELIBERATELY NOT #[non_exhaustive] (the G2 policy's documented exception,
// surfaced by the #368 gate run — this shipped un-annotated at #367 and the
// omission is the DESIGN, not a miss): the FFI bridge's convert arms match
// this enum EXHAUSTIVELY in BOTH directions ("no Unknown" — #367), so adding
// an outcome variant is a COMPILE-LOUD workspace event that forces the FFI
// mirror + the Dart rendering to grow in lockstep. `#[non_exhaustive]` would
// force a wildcard arm in the downstream FFI crate — exactly the silent
// mis-render ("some future outcome displays as Failed") the terminal-honesty
// batch exists to prevent. The durable form is already drift-safe the OTHER
// way: `from_db_str` reads unknown stored text as `None` (fail-honest).
//
// The line below is the waiver the `public_enums_non_exhaustive` gate READS
// (P0-9): until an earlier revision that gate accepted this paragraph because it merely
// CONTAINED the word, which meant any prose could satisfy a code gate. The gate
// now accepts the attribute or this exact marker, and pins the set of marked
// enums to a list it names — so a second waiver is a deliberate edit of the
// gate, not a comment.
// G2-WAIVER: exhaustive-by-design (#367, the paragraph above)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwapOutcome {
    Success,
    Refunded,
    Failed,
}

impl SwapOutcome {
    /// The stored TEXT form — append-only vocabulary (a renamed value would
    /// orphan every previously-pinned row into the unknown→`None` arm).
    fn as_db_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Refunded => "refunded",
            Self::Failed => "failed",
        }
    }

    /// Parse the stored TEXT; unknown (a NEWER build's variant read by this
    /// one) ⇒ `None` = still-in-flight — fail-honest, see [`InFlightSwap`].
    fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Self::Success),
            "refunded" => Some(Self::Refunded),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Create the swap-record table if absent. Idempotent (`CREATE TABLE IF NOT
/// EXISTS`), so `db::migrate` calls it on EVERY provision and open — a wallet
/// provisioned before W-swap-5 gains it on the next open, with NO
/// `WALLET_SCHEMA_VERSION` bump (the sibling-store precedent). UNGATED by the
/// `swap` feature: schema durability is build-independent (ADR-0534).
pub(crate) const TABLE: &str = "swap_record";

pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS swap_record (
             swap_id          TEXT PRIMARY KEY,
             out_of_zec       INTEGER NOT NULL,
             created_at       INTEGER NOT NULL,
             deposit_deadline INTEGER,
             expires_at_wall  INTEGER NOT NULL,
             outcome          TEXT,
             watch_address    TEXT,
             watch_index      INTEGER,
             watch_served     INTEGER NOT NULL DEFAULT 0
         );",
    )
    .map_err(map_db_err)?;
    ensure_additive_columns(conn)
}

/// Idempotently add the additive columns for a table created by an older
/// build: `outcome` (#367), the `watch_address`/`watch_index` pair (#368 —
/// the swap's watched inbound transparent leg, written at record time so the
/// Refunded re-arm can re-create the detection row after the issued-quote row
/// is long consumed; NEVER listed, NEVER crossing FFI), and `watch_served`
/// (#385 — the retired-leg latch: `1` once the leg's detection purpose was
/// served, see [`mark_watch_served_in_tx`]; `NOT NULL DEFAULT 0` is legal on
/// an additive `ALTER` because the default is a constant, and `0` is correct
/// for every pre-#385 row — a retired-but-unlatched pre-#385 leg resurrects
/// as an ORDINARY unresolved watch, polled until its record pins or is
/// dismissed (its money already moved, so it polls empty and the
/// funded-then-empty retire can never re-fire — it latches only if the leg
/// ever re-funds; review precision — the cost is one extra polled
/// own-address, not a loop).
/// `ALTER TABLE ADD COLUMN` errors if the column already exists, so consult
/// `PRAGMA table_info` first (the [`crate::issued_quote_store`]
/// destination-column precedent) — O(1) metadata, never a data-bearing
/// rebuild on the boot path. A pre-#367/#368 row's NULLs
/// are correct for it (nothing was pinned; no watch was recorded). NOT a
/// TOCTOU hazard: `migrate` runs only under the exclusive single-opener
/// `WalletLock` flock. Documented downgrade bound: an older binary's RESCAN
/// of a newer DB fails typed on the column drift (`db::assert_columns_match`)
/// — loud, never a silent column shift.
fn ensure_additive_columns(conn: &Connection) -> Result<(), WalletError> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(swap_record)")
        .map_err(map_db_err)?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(map_db_err)?
        .collect::<Result<std::collections::HashSet<_>, _>>()
        .map_err(map_db_err)?;
    for (name, decl) in [
        ("outcome", "outcome TEXT"),
        ("watch_address", "watch_address TEXT"),
        ("watch_index", "watch_index INTEGER"),
        ("watch_served", "watch_served INTEGER NOT NULL DEFAULT 0"),
        // S8 (R01): the provider's own handle, recorded at execute for the poll to
        // resolve. A pre-S8 row's NULL means "no handle recorded": `watch_status`
        // refuses it typed rather than guess (the dev-stage drop posture).
        ("provider_ref", "provider_ref TEXT"),
    ] {
        if !names.contains(name) {
            conn.execute_batch(&format!("ALTER TABLE swap_record ADD COLUMN {decl}"))
                .map_err(map_db_err)?;
        }
    }
    Ok(())
}

/// Record a swap's registration at EXECUTE. ONE `IMMEDIATE` transaction, three
/// steps:
///
/// 1. **Inline prune** (bound hygiene), gated on the clock-plausibility floor —
///    a sub-floor (unsynced/1970) clock prunes NOTHING, so a bad clock can
///    never mass-destroy the home rows (the `deposit_gate` `Wait`-arm
///    direction). At a plausible clock it reaps two classes, PINNED rows only
///    since #382 (an unpinned row is unresolved money in motion — deleting it
///    on a wall clock was the HIGH's third leg): LAPSED pinned rows
///    (`expires_at_wall <= now AND outcome IS NOT NULL` — an unknown-text
///    outcome counts as pinned here, preserving the #367 documented tail) and
///    FAR-FUTURE pinned rows no sane clock could have written
///    (`expires_at_wall > now + settlement + skew` — every legitimate row's
///    bound is write-time + the settlement ceiling, so anything beyond that is
///    a jumped-clock leftover; unreaped, such rows would list as eternal
///    terminal rows). A far-future UNPINNED row is CLAMPED to one settlement
///    window from now instead of deleted (the `swap_destination` far-future
///    precedent — bounded, never an erased unresolved record). Documented
///    residuals (#385 named the second): a transient FORWARD clock spike
///    exceeding a live PINNED row's remaining window reaps it early (lapse
///    arm), and a BACKWARD step larger than the skew allowance makes a
///    legitimately-pinned live row's bound (write-time + settlement) read as
///    far-future and reaps it through THAT arm — both display-only losses
///    (custody state is elsewhere), the same accepted exposure as the
///    money-path `deposit_gate`'s `Expired` arm. An UNPINNED row is immune
///    to both (lapse-spared; far-future CLAMPS it).
/// 2. **Plain insert** under the SDK-minted `swap_id` (S8): OUR issued-quote
///    single-flight makes a same-quote replay structurally dead, and the key is
///    minted from the OS CSPRNG, never a provider value — so a duplicate here is
///    an INVARIANT VIOLATION surfaced LOUD (`StoreCorrupt` via the PK conflict),
///    not the pre-S8 `OR IGNORE` first-wins that let a provider reusing a deposit
///    address collapse two orders onto one home row. A provider that reuses an
///    address across orders (nothing forbids it — HARD-G: statuses/ids are API
///    data) now yields two rows with one `provider_ref`. The `provider_ref` rides
///    the row as the poll's request value ([`provider_ref_of`]).
/// 3. **Cap eviction** to [`MAX_SWAP_RECORDS`], EXEMPTING the just-inserted id
///    (the load-bearing half of record-first, review H1): the fresh
///    registration ALWAYS survives — by construction, not by timestamp
///    ordering — so a deposit can never be durably queued whose home row this
///    same transaction evicted (a regressed clock + a cap-full table of
///    later-stamped rows would otherwise evict the LIVE row it just wrote).
///    Among the REST, the newest `MAX_SWAP_RECORDS - 1` by `created_at` stay
///    (ties broken by id for determinism); display-only rows, so eviction can
///    never lose money — and the §4.4 guard bounds real concurrency far below
///    the cap.
///
/// **#368 — the watch arm rides the SAME transaction.** `watch` is the swap's
/// watched inbound transparent leg (IntoZec: the destination; OutOfZec: the
/// refund address) with its single-use external index. When present, step 2
/// ALSO writes it onto the row (the Refunded re-arm's durable source) and
/// step 2b arms/extends the `swap_destination` detection row
/// ([`crate::swap_destination_store::arm_in_tx`] — a deliberate, documented
/// cross-store call: both tables live on the one aux connection, and the
/// atomicity is the point). Record-first is therefore WATCH-first: a deposit
/// can never be durably queued whose refund watch this same transaction did
/// not arm — the take↔mark_executed two-txn crash window (and its tolerated
/// residual-busy) is GONE for the execute path. The watch deadline is the
/// row's own `expires_at_wall` (execute-time + the settlement ceiling — one
/// value, one meaning).
///
/// `insert` is the watch-less convenience form (`watch = None`) — same
/// transaction, no detection-row arm. `provider_ref` is the provider's own handle
/// for the swap (the deposit address, §5.4 never-log), stored beside the row —
/// not on [`InFlightSwap`], which is the READ shape a host lists.
pub(crate) fn insert_with_watch(
    conn: &mut Connection,
    row: &InFlightSwap,
    now_unix: i64,
    provider_ref: &str,
    watch: Option<(&str, u32)>,
) -> Result<(), WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let floor = i64::try_from(CLOCK_PLAUSIBILITY_FLOOR_SECS).unwrap_or(i64::MAX);
    if now_unix >= floor {
        let settlement = i64::try_from(SWAP_SETTLEMENT_MAX_SECS).unwrap_or(i64::MAX);
        let far_future_ceiling = now_unix
            .saturating_add(settlement)
            .saturating_add(i64::try_from(FAR_FUTURE_SKEW_ALLOWANCE_SECS).unwrap_or(i64::MAX));
        tx.execute(
            "DELETE FROM swap_record WHERE outcome IS NOT NULL \
                 AND (expires_at_wall <= ?1 OR expires_at_wall > ?2)",
            rusqlite::params![now_unix, far_future_ceiling],
        )
        .map_err(map_db_err)?;
        tx.execute(
            "UPDATE swap_record SET expires_at_wall = ?1 \
                 WHERE outcome IS NULL AND expires_at_wall > ?2",
            rusqlite::params![now_unix.saturating_add(settlement), far_future_ceiling],
        )
        .map_err(map_db_err)?;
    }
    tx.execute(
        "INSERT INTO swap_record \
             (swap_id, out_of_zec, created_at, deposit_deadline, expires_at_wall, \
              watch_address, watch_index, provider_ref) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            row.swap_id,
            i64::from(row.out_of_zec),
            row.created_at,
            row.deposit_deadline,
            row.expires_at_wall,
            watch.map(|(a, _)| a.to_owned()),
            watch.map(|(_, i)| i64::from(i)),
            provider_ref,
        ],
    )
    .map_err(map_db_err)?;
    // #368 step 2b — arm/extend the detection row atomically with the home row. The
    // MAX-deadline upsert is idempotent and the insert above is under a fresh key,
    // so the whole-txn busy retry stays sound (a busy COMMIT rolled the txn back
    // whole; the re-run sees no row).
    if let Some((address, index)) = watch {
        crate::swap_destination_store::arm_in_tx(
            &tx,
            &row.swap_id,
            address,
            index,
            row.expires_at_wall,
            None,
        )?;
    }
    tx.execute(
        // Keep IN-FLIGHT rows preferentially at the cap (#367 money-review):
        // a PINNED terminal is finished business awaiting the user's Remove —
        // it must never evict a LIVE row's only re-attach handle. Ordering:
        // unpinned first, then newest (ties by id for determinism).
        "DELETE FROM swap_record WHERE swap_id != ?2 AND swap_id NOT IN (
             SELECT swap_id FROM swap_record WHERE swap_id != ?2
             ORDER BY (outcome IS NULL) DESC, created_at DESC, swap_id DESC LIMIT ?1
         )",
        rusqlite::params![
            i64::try_from(MAX_SWAP_RECORDS.saturating_sub(1)).unwrap_or(i64::MAX),
            row.swap_id
        ],
    )
    .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(())
}

/// [`insert_with_watch`] with no watch — the full 3-step contract above, no
/// detection-row arm — and the row's own id as its provider handle (the pre-S8
/// shape, where the two were one value). Test-only convenience (production
/// always records the watch when the record carries one).
#[cfg(test)]
pub(crate) fn insert(
    conn: &mut Connection,
    row: &InFlightSwap,
    now_unix: i64,
) -> Result<(), WalletError> {
    insert_with_watch(conn, row, now_unix, &row.swap_id, None)
}

/// The provider's own handle recorded for `swap_id` at execute — the value a
/// status poll must carry — or `None` for an absent row OR a pre-S8 row that
/// recorded none (`watch_status` refuses both typed; guessing a handle for a
/// row that never recorded one is exactly the identity/handle conflation S8
/// removes). Read-only, no transaction. §5.4 NEVER-log.
pub(crate) fn provider_ref_of(
    conn: &Connection,
    swap_id: &str,
) -> Result<Option<String>, WalletError> {
    conn.query_row(
        "SELECT provider_ref FROM swap_record WHERE swap_id = ?1",
        rusqlite::params![swap_id],
        |r| r.get::<_, Option<String>>(0),
    )
    .optional()
    .map(Option::flatten)
    .map_err(map_db_err)
}

/// The live in-flight set: every UNPINNED row (unresolved money in motion
/// lists regardless of its lapse bound — #382; pre-#382 an unresolved row
/// vanished at `expires_at_wall`, taking the user's only re-attach handle
/// and the Refunded re-arm trigger with it) plus every PINNED row whose
/// self-lapse bound is still ahead of the caller's clock, newest-first (the
/// home surface order). `outcome IS NULL` is the SQL-level test: a stored
/// outcome TEXT this build does not recognize still counts as pinned for
/// lapse purposes (the #367 documented tail — it renders in-flight but reaps
/// at lapse). Read-only — a lapsed pinned row is FILTERED here and physically
/// reaped by the next `insert`'s prune (reads stay reads; residue is
/// cap-bounded and lives inside the sealed DB).
pub(crate) fn list_in_flight(
    conn: &Connection,
    now_unix: i64,
) -> Result<Vec<InFlightSwap>, WalletError> {
    let mut stmt = conn
        .prepare(
            "SELECT swap_id, out_of_zec, created_at, deposit_deadline, expires_at_wall, \
                    outcome \
             FROM swap_record WHERE outcome IS NULL OR expires_at_wall > ?1 \
             ORDER BY created_at DESC, swap_id DESC",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![now_unix], |r| {
            Ok(InFlightSwap {
                swap_id: r.get(0)?,
                out_of_zec: r.get::<_, i64>(1)? != 0,
                created_at: r.get(2)?,
                deposit_deadline: r.get(3)?,
                expires_at_wall: r.get(4)?,
                // Unknown stored text ⇒ `None` (fail-honest, see the type doc).
                outcome: r
                    .get::<_, Option<String>>(5)?
                    .as_deref()
                    .and_then(SwapOutcome::from_db_str),
            })
        })
        .map_err(map_db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_db_err)?;
    Ok(rows)
}

/// Pin the host's OBSERVED terminal into the record (#367) — FIRST-WINS
/// (`outcome IS NULL` guard): a terminal is forever, so the first observation
/// is the truth and a later conflicting pin (a hostile provider flipping a
/// "terminal" — HARD-G: statuses are API data) cannot re-write it. Idempotent
/// and total: pinning an absent/already-pinned row is `Ok(false)` (the caller
/// treats it as "nothing to do" — the row may have been user-dismissed between
/// observation and pin; display-only either way).
///
/// **The pin REFRESHES the self-lapse bound (#382)** to EXACTLY one settlement
/// window from `now_unix` at a plausible clock: unresolved rows now outlive
/// the execute-time window, so a pin can land on an already-LAPSED row, and
/// pinning-then-instantly-delisting would erase the outcome un-rendered — the
/// exact #367 regression at the new boundary. The outcome therefore always
/// gets one full window to be SEEN, measured from observation. EXACT, not
/// `MAX`: every legitimate prior bound is its own
/// write-time + the settlement ceiling ≤ this pin's window, so replacement
/// can only ever collapse a jumped-clock FAR-FUTURE artifact that `MAX` would
/// preserve as an eternal-looking terminal until the next insert's reap
/// (documented residual: a backward clock step between write and pin shrinks
/// the window by the step — bounded, display-only). Below the
/// clock-plausibility floor the update keeps `MAX` semantics instead — never
/// shrink on a clock we don't trust (the prune floor-gate direction). ONE
/// `IMMEDIATE` transaction (single statement, aux-invariant shape).
pub(crate) fn record_outcome(
    conn: &mut Connection,
    swap_id: &str,
    outcome: SwapOutcome,
    now_unix: i64,
) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = pin_in_tx(
        &tx,
        "UPDATE swap_record SET outcome = ?2, expires_at_wall = ?3 \
             WHERE swap_id = ?1 AND outcome IS NULL",
        "UPDATE swap_record SET outcome = ?2, \
                 expires_at_wall = MAX(expires_at_wall, ?3) \
             WHERE swap_id = ?1 AND outcome IS NULL",
        swap_id,
        outcome,
        now_unix,
    )?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// The shared pin write: the plausible-clock arm sets the display bound EXACT,
/// the sub-floor arm keeps `MAX` (see [`record_outcome`]'s contract — both pin
/// paths share it).
fn pin_in_tx(
    tx: &rusqlite::Transaction<'_>,
    exact_sql: &str,
    max_sql: &str,
    swap_id: &str,
    outcome: SwapOutcome,
    now_unix: i64,
) -> Result<usize, WalletError> {
    let floor = i64::try_from(CLOCK_PLAUSIBILITY_FLOOR_SECS).unwrap_or(i64::MAX);
    let display_until =
        now_unix.saturating_add(i64::try_from(SWAP_SETTLEMENT_MAX_SECS).unwrap_or(i64::MAX));
    let sql = if now_unix >= floor {
        exact_sql
    } else {
        max_sql
    };
    tx.execute(
        sql,
        rusqlite::params![swap_id, outcome.as_db_str(), display_until],
    )
    .map_err(map_db_err)
}

/// The record's watched inbound transparent leg (#368) — `(address, index,
/// out_of_zec)` — the Refunded re-arm's durable source: by observation time
/// the issued-quote row is long consumed, so this row is the ONE place the
/// refund address/index survive. `None` for an absent/dismissed row, a
/// pre-#368 row (NULL columns), or a half-set pair (tamper — fail-honest
/// None, the re-arm is hygiene, never worth failing a pin over). An index
/// outside `[0, u32::MAX]` is the same fail-honest `None`. Deliberately
/// IGNORES the `watch_served` latch (#385): this reader feeds the CARD-VISIT
/// `Refunded` re-arm, which is exactly the recovery path for a
/// lying-endpoint EARLY retire — the latch stops only the unattended
/// per-pass re-arm ([`unresolved_watches`]). Read-only.
pub(crate) fn watch_of(
    conn: &Connection,
    swap_id: &str,
) -> Result<Option<(String, u32, bool)>, WalletError> {
    let row = conn
        .query_row(
            "SELECT watch_address, watch_index, out_of_zec FROM swap_record WHERE swap_id = ?1",
            rusqlite::params![swap_id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<i64>>(1)?,
                    r.get::<_, i64>(2)? != 0,
                ))
            },
        )
        .optional()
        .map_err(map_db_err)?;
    Ok(match row {
        Some((Some(address), Some(index_i64), out_of_zec)) => u32::try_from(index_i64)
            .ok()
            .map(|i| (address, i, out_of_zec)),
        _ => None,
    })
}

/// Every UNRESOLVED record's watched inbound leg — `(swap_id, address,
/// index)` for rows with `outcome IS NULL` (SQL-level — an unknown-text
/// outcome is treated as pinned, the fail-safe bound: this build never
/// extends a watch for a state a newer build already resolved), a present,
/// valid watch pair, AND `watch_served = 0` (#385 — a retired leg's latch,
/// [`mark_watch_served_in_tx`]: its detection purpose is served, so the
/// per-pass re-arm must stop resurrecting it; the card-visit re-arm reads
/// [`watch_of`], which deliberately ignores the latch). The #382 per-pass
/// re-arm's source: the poll's set-build re-arms each of these for a fresh
/// settlement window, so "watchable while unresolved" survives ANY absence
/// instead of dying at execute + 48 h. Both directions are returned — the
/// OutOfZec refund AND the IntoZec destination share the late-settlement
/// tail. A half-set or out-of-range pair (tamper) is SKIPPED fail-honest,
/// like [`watch_of`] — hygiene, never worth failing the poll over.
/// Cap-bounded output (≤ `MAX_SWAP_RECORDS`). Read-only.
pub(crate) fn unresolved_watches(
    conn: &Connection,
) -> Result<Vec<(String, String, u32)>, WalletError> {
    let mut stmt = conn
        .prepare(
            "SELECT swap_id, watch_address, watch_index FROM swap_record \
             WHERE outcome IS NULL AND watch_served = 0 \
               AND watch_address IS NOT NULL AND watch_index IS NOT NULL \
             ORDER BY swap_id",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(map_db_err)?;
    let mut out = Vec::new();
    let mut skipped: u64 = 0;
    for row in rows {
        // A row that cannot READ because of TAMPER (type-tampered watch columns
        // — SQLite's dynamic typing admits TEXT in an INTEGER column — or an
        // out-of-range index) is SKIPPED + counted (review fold
        // telemetry parity with `swap_destination_store::active`): fail-honest
        // hygiene, per this reader's own contract — one tampered row must never
        // starve every OTHER unresolved swap's re-arm, pass after pass. Any
        // OTHER per-row error is a genuine step/page fault and PROPAGATES (the
        // caller's re-arm block is already best-effort).
        let (swap_id, address, index_i64) = match row {
            Ok(fields) => fields,
            Err(rusqlite::Error::InvalidColumnType(..)) => {
                skipped += 1;
                continue;
            }
            Err(e) => return Err(map_db_err(e)),
        };
        match u32::try_from(index_i64) {
            Ok(index) => out.push((swap_id, address, index)),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        // §5.4: a COUNT only — never a row payload (`skipped` is allowlisted).
        tracing::warn!(target: "zec_wallet_core", skipped, "wallet.swap_watch_scan");
    }
    Ok(out)
}

/// Mark the record's watched leg SERVED — the RETIRED-leg convergence (#382
/// fold, redesigned by #385): a funded-then-empty leg was
/// delivered/refunded AND shielded/spent, so its detection purpose is SERVED
/// — without a durable latch, the per-pass re-arm resurrected the
/// just-retired row forever (retire deletes, the next pass re-inserts, polls
/// empty, retires again — an unbounded churn for any unresolved record whose
/// money already moved, IntoZec deliveries especially, which have no chain
/// pin). [`unresolved_watches`] skips a served leg, so the churn ends; the
/// record itself keeps LISTING until pinned or dismissed (the outcome label
/// still comes from the provider poll or the card visit).
///
/// A FLAG, not a column clear (#385 — the first cut NULLed
/// `watch_address`/`watch_index` here, which also destroyed [`watch_of`]'s
/// source and silently narrowed the lying-endpoint early-retire recovery
/// from "open the card" to "full rescan"): the columns survive, so the #368
/// item-4 card-visit `Refunded` re-arm still re-creates the detection row —
/// an early retire (an endpoint lying empty for N consecutive passes / a
/// reorg outlasting them — debounced since #386) stays recoverable exactly
/// as the retire's documented residual promises. A re-armed-then-retired
/// leg simply re-latches here (idempotent).
///
/// Transaction-scoped BY DESIGN: the ONLY production caller is
/// [`crate::swap_destination_store::debounced_retire_and_mark_served`], which composes
/// this with the destination-row DELETE in ONE aux `IMMEDIATE` txn (#385 —
/// two separate transactions left a kill/fault window between retire and the
/// latch that durably re-opened the churn; the id never re-enters a retire
/// list, so the latch write was never re-attempted). An absent id (a
/// `backfill:<index>` synthetic, a dismissed record) touches zero rows.
pub(crate) fn mark_watch_served_in_tx(
    tx: &rusqlite::Transaction<'_>,
    swap_id: &str,
) -> Result<bool, WalletError> {
    let n = tx
        .execute(
            "UPDATE swap_record SET watch_served = 1 WHERE swap_id = ?1",
            rusqlite::params![swap_id],
        )
        .map_err(map_db_err)?;
    Ok(n > 0)
}

/// Pin `Refunded` from a CHAIN observation (#382 (c)): the poll saw the
/// previously-unfunded OutOfZec watch row turn funded — the refund UTXO is
/// consensus truth, which outranks the provider's status API (HARD-G:
/// statuses are API data). Guarded in SQL exactly like [`record_outcome`]'s
/// first-wins (`outcome IS NULL`) PLUS the direction (`out_of_zec = 1` — an
/// IntoZec destination turning funded is a DELIVERY, not a refund) so a
/// misplaced call can never mislabel. Idempotent and total: absent /
/// IntoZec / already-pinned rows are `Ok(false)`; a `backfill:<index>`
/// synthetic id has no record row and lands there too. Refreshes the
/// self-lapse bound exactly like [`record_outcome`] (#382 — the chain pin is
/// the path MOST likely to land on an already-lapsed row). ONE `IMMEDIATE`
/// transaction. Disclosed residual (spec §4.4 item 8c): a party who knows
/// the refund address — by construction the PROVIDER, and equally the
/// LIGHTWALLETD ENDPOINT, which is handed every watched address in the
/// `GetAddressUtxos` query itself and whose asserted records pass the M2
/// script check for any address it was queried on —
/// can dust/fabricate a funding into a false `Refunded` display, first-wins
/// durable, which also stops the unresolved re-arm for that swap.
/// Display-only either way: the balance facts stay chain/engine-derived, a
/// REAL later refund is still engine-put and balance-visible, and both
/// parties already sat inside the display-trust boundary (the provider via
/// the status API, the endpoint via every balance/UTXO answer it serves).
pub(crate) fn pin_chain_observed_refund(
    conn: &mut Connection,
    swap_id: &str,
    now_unix: i64,
) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = pin_in_tx(
        &tx,
        "UPDATE swap_record SET outcome = ?2, expires_at_wall = ?3 \
             WHERE swap_id = ?1 AND outcome IS NULL AND out_of_zec = 1",
        "UPDATE swap_record SET outcome = ?2, \
                 expires_at_wall = MAX(expires_at_wall, ?3) \
             WHERE swap_id = ?1 AND outcome IS NULL AND out_of_zec = 1",
        swap_id,
        SwapOutcome::Refunded,
        now_unix,
    )?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// Remove one record — USER INTENT only since #367 (the terminal card's Done
/// after the outcome rendered, or the home row's explicit Remove; a terminal
/// OBSERVATION pins via [`record_outcome`] instead — deleting there erased
/// outcomes the user never saw). Idempotent: deleting an absent id is
/// `Ok(false)`. ONE `IMMEDIATE` transaction (single statement, aux-invariant
/// shape).
pub(crate) fn dismiss(conn: &mut Connection, swap_id: &str) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "DELETE FROM swap_record WHERE swap_id = ?1",
            rusqlite::params![swap_id],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// Total rows (lapsed included) — tests + hygiene observability (a COUNT, never
/// a §5.4 payload).
#[cfg(test)]
pub(crate) fn count(conn: &Connection) -> Result<u64, WalletError> {
    conn.query_row("SELECT COUNT(*) FROM swap_record", [], |r| {
        r.get::<_, i64>(0)
    })
    .map(|n| u64::try_from(n).unwrap_or(0))
    .map_err(map_db_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh in-memory connection with the table applied — the store logic is
    /// orthogonal to SQLCipher (the sibling-store pattern).
    fn mem() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&conn).expect("schema");
        conn
    }

    fn row(id: &str, out_of_zec: bool, created_at: i64, expires: i64) -> InFlightSwap {
        InFlightSwap {
            swap_id: id.to_owned(),
            out_of_zec,
            created_at,
            deposit_deadline: Some(created_at + 900),
            expires_at_wall: expires,
            outcome: None,
        }
    }

    #[test]
    fn insert_then_list_round_trips_the_record() {
        let mut conn = mem();
        let r = row("swap-1", true, 1_700_000_000, 1_700_172_800);
        insert(&mut conn, &r, 1_700_000_000).expect("insert");
        let live = list_in_flight(&conn, 1_700_000_001).expect("list");
        assert_eq!(live, vec![r]);
    }

    #[test]
    fn list_excludes_a_lapsed_pinned_record_and_orders_newest_first() {
        // #382: self-lapse is PINNED-ONLY, and the pin REFRESHES the bound to one
        // settlement window from observation — so the pinned row filters exactly
        // one window AFTER the pin, not at its old execute-time bound (an unpinned
        // lapsed row keeps listing, see the sibling).
        let mut conn = mem();
        let base = 1_700_000_000_i64;
        insert(&mut conn, &row("old", false, base + 100, base + 200), base).expect("insert old");
        assert!(record_outcome(&mut conn, "old", SwapOutcome::Failed, base + 300).expect("pin"));
        insert(
            &mut conn,
            &row("live-a", true, base + 300, base + 999_000),
            base,
        )
        .expect("insert a");
        insert(
            &mut conn,
            &row("live-b", false, base + 400, base + 999_000),
            base,
        )
        .expect("insert b");
        let settlement = i64::try_from(SWAP_SETTLEMENT_MAX_SECS).expect("fits");
        let live = list_in_flight(&conn, base + 250).expect("list");
        assert_eq!(
            live.len(),
            3,
            "within the post-pin display window the pinned row still lists"
        );
        let live = list_in_flight(&conn, base + 301 + settlement).expect("list");
        let ids: Vec<&str> = live.iter().map(|r| r.swap_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["live-b", "live-a"],
            "newest-first, the PINNED row filtered one window after its pin"
        );
    }

    #[test]
    fn unpinned_lapsed_rows_stay_listed_and_are_spared_by_the_prune() {
        // #382 (a) — the converged HIGH's listing leg: an UNRESOLVED swap
        // (outcome unpinned) is money in motion and must neither vanish from
        // `list_in_flight` at `expires_at_wall` nor be physically reaped by the
        // next insert's prune — pre-#382 both happened at execute + 48 h, so a
        // >48 h-absent user lost the re-attach handle AND the Refunded re-arm
        // trigger while the refund landed invisibly.
        let mut conn = mem();
        let base = 1_700_000_000_i64;
        insert(&mut conn, &row("unresolved", true, base, base + 200), base).expect("insert");
        // WAY past the lapse bound: still listed…
        let live = list_in_flight(&conn, base + 1_000_000).expect("list");
        assert_eq!(
            live.iter().map(|r| r.swap_id.as_str()).collect::<Vec<_>>(),
            vec!["unresolved"],
            "an unpinned row lists past its lapse bound"
        );
        // …and a later insert's inline prune spares it (only PINNED lapsed reap).
        insert(
            &mut conn,
            &row("fresh", true, base + 1_000_000, base + 1_172_800),
            base + 1_000_000,
        )
        .expect("insert fresh");
        assert_eq!(
            count(&conn).expect("count"),
            2,
            "the unresolved lapsed row survived the prune"
        );
        // Once PINNED, the same row lapses like any terminal — one settlement
        // window after the OBSERVATION (the #382 display refresh), so the late
        // outcome is never delisted un-rendered.
        let pin_at = base + 1_000_000;
        assert!(
            record_outcome(&mut conn, "unresolved", SwapOutcome::Refunded, pin_at).expect("pin")
        );
        let settlement = i64::try_from(SWAP_SETTLEMENT_MAX_SECS).expect("fits");
        let live = list_in_flight(&conn, pin_at + settlement - 1).expect("list");
        assert!(
            live.iter()
                .any(|r| r.swap_id == "unresolved" && r.outcome == Some(SwapOutcome::Refunded)),
            "the freshly-pinned outcome gets a full window to be seen"
        );
        let live = list_in_flight(&conn, pin_at + settlement + 1).expect("list");
        assert_eq!(
            live.iter().map(|r| r.swap_id.as_str()).collect::<Vec<_>>(),
            vec!["fresh"],
            "a pinned row filters one window after its pin"
        );
    }

    #[test]
    fn a_duplicate_sdk_id_through_the_bare_insert_is_the_same_typed_error() {
        // S8 `identity` row 8 (rewrites `insert_is_first_wins_on_the_id` in its own span —
        // a third first-wins row the contract's rewrite list did not name): the id is
        // SDK-minted, so a second `insert` under the SAME id is an invariant violation —
        // refused TYPED, never absorbed by `OR IGNORE` — and the first row's fields survive.
        // The `insert_with_watch` leg has its own row at the end of this module. Reddening
        // mutation: `OR IGNORE` (the base commit — the second insert was a silent no-op).
        let mut conn = mem();
        insert(&mut conn, &row("swap-1", true, 100, 10_000), 50).expect("first");
        let mut second = row("swap-1", false, 999, 20_000);
        second.deposit_deadline = None;
        assert!(
            insert(&mut conn, &second, 50).is_err(),
            "a duplicate SDK id is a typed error, never a silent first-wins"
        );
        let live = list_in_flight(&conn, 200).expect("list");
        assert_eq!(live.len(), 1);
        assert!(live[0].out_of_zec, "the FIRST row's fields survive");
        assert_eq!(live[0].created_at, 100);
    }

    #[test]
    fn insert_prunes_lapsed_pinned_rows_inline() {
        // At a PLAUSIBLE clock (>= the floor) the inline prune reaps lapsed rows —
        // PINNED ones only since #382 (the unpinned spare has its own test).
        let mut conn = mem();
        let base = 1_700_000_000_i64;
        insert(&mut conn, &row("dead", true, base, base + 200), base).expect("insert dead");
        assert!(record_outcome(&mut conn, "dead", SwapOutcome::Success, 0).expect("pin dead"));
        assert_eq!(count(&conn).expect("count"), 1);
        insert(
            &mut conn,
            &row("fresh", true, base + 300, base + 10_000),
            base + 300,
        )
        .expect("insert fresh");
        assert_eq!(
            count(&conn).expect("count"),
            1,
            "the lapsed pinned row was physically reaped by the next insert"
        );
    }

    #[test]
    fn cap_eviction_never_evicts_the_just_inserted_row_under_a_regressed_clock() {
        // Review H1 (empirically reproduced pre-fix): a cap-full table of
        // LATER-stamped rows (a clock that ran ahead within plausible skew)
        // must never evict the row this same transaction just wrote —
        // record-first would otherwise silently lose its OWN home row and an
        // armed deposit would be durably queued with no re-attach handle.
        let mut conn = mem();
        let now = 1_700_000_000_i64;
        let cap = i64::try_from(MAX_SWAP_RECORDS).expect("cap fits");
        for i in 0..cap {
            insert(
                &mut conn,
                &row(&format!("ahead{i:03}"), false, now + 100 + i, now + 100_000),
                now,
            )
            .expect("seed ahead-stamped row");
        }
        assert_eq!(count(&conn).expect("count"), MAX_SWAP_RECORDS as u64);
        // The live registration carries an OLDER created_at than every seeded
        // row — by timestamp ordering alone it would lose the eviction contest.
        insert(&mut conn, &row("live-now", true, now, now + 100_000), now).expect("live insert");
        let live = list_in_flight(&conn, now + 1).expect("list");
        assert!(
            live.iter().any(|r| r.swap_id == "live-now"),
            "the just-inserted row ALWAYS survives the cap — by construction"
        );
        assert_eq!(count(&conn).expect("count"), MAX_SWAP_RECORDS as u64);
    }

    #[test]
    fn far_future_pinned_leftovers_are_swept_at_a_plausible_clock() {
        // A jumped-clock write leaves rows whose self-lapse bound exceeds any
        // legitimate write (now + settlement + skew) — the prune's second arm
        // reaps PINNED ones (#382: an unpinned far-future row CLAMPS instead,
        // see the sibling) so they can neither list as eternal terminal rows
        // nor squat the cap forever.
        let mut conn = mem();
        let now = 1_700_000_000_i64;
        insert(
            &mut conn,
            &row("poisoned", true, 4_100_000_000, 4_100_172_800),
            0,
        )
        .expect("hostile-clock write lands (sub-floor prune skips)");
        assert!(record_outcome(&mut conn, "poisoned", SwapOutcome::Failed, 0).expect("pin"));
        insert(&mut conn, &row("live", false, now, now + 100_000), now).expect("live");
        let live = list_in_flight(&conn, now + 1).expect("list");
        assert_eq!(live.len(), 1, "the poisoned pinned row never lists");
        assert_eq!(live[0].swap_id, "live");
        assert_eq!(
            count(&conn).expect("count"),
            1,
            "the poisoned pinned row was physically reaped"
        );
    }

    #[test]
    fn far_future_unpinned_rows_clamp_to_one_settlement_window() {
        // #382 — the unpinned mirror of the pinned sweep: deleting an UNRESOLVED
        // row over a clock artifact would erase money in motion (the exact class
        // the listing fix closes), so the prune CLAMPS its bound to one
        // settlement window from now instead (the `swap_destination` far-future
        // precedent) — bounded, still listed, never erased.
        let mut conn = mem();
        let now = 1_700_000_000_i64;
        insert(
            &mut conn,
            &row("jumped", true, 4_100_000_000, 4_100_172_800),
            0,
        )
        .expect("hostile-clock write lands (sub-floor prune skips)");
        insert(&mut conn, &row("live", false, now, now + 100_000), now).expect("live");
        let live = list_in_flight(&conn, now + 1).expect("list");
        assert_eq!(
            live.len(),
            2,
            "the unpinned far-future row still lists (clamped, not deleted)"
        );
        let jumped = live
            .iter()
            .find(|r| r.swap_id == "jumped")
            .expect("clamped row present");
        let settlement = i64::try_from(SWAP_SETTLEMENT_MAX_SECS).expect("fits");
        assert_eq!(
            jumped.expires_at_wall,
            now + settlement,
            "clamped to exactly one settlement window from the pruning clock"
        );
    }

    #[test]
    fn pin_at_a_plausible_clock_replaces_a_far_future_bound() {
        // review fold — EXACT, not MAX, at a plausible clock: a jumped-clock
        // far-future bound on an UNPINNED row would otherwise survive the pin via
        // MAX and list as an eternal terminal until the next insert's reap; the
        // pin collapses it to exactly one display window from observation. (The
        // sub-floor arm keeps MAX — never shrink on an untrusted clock — covered
        // by `far_future_pinned_leftovers_are_swept_at_a_plausible_clock`, whose
        // clock-0 pin preserves the far-future value for the sweep.)
        let mut conn = mem();
        let now = 1_700_000_000_i64;
        // Land the far-future row under a sub-floor clock so no prune touches it.
        insert(
            &mut conn,
            &row("jumped", true, 4_100_000_000, 4_100_172_800),
            0,
        )
        .expect("hostile-clock write lands");
        assert!(record_outcome(&mut conn, "jumped", SwapOutcome::Refunded, now).expect("pin"));
        let settlement = i64::try_from(SWAP_SETTLEMENT_MAX_SECS).expect("fits");
        let live = list_in_flight(&conn, now + settlement - 1).expect("list");
        assert!(
            live.iter()
                .any(|r| r.swap_id == "jumped" && r.outcome == Some(SwapOutcome::Refunded)),
            "the pinned outcome gets its full window from observation"
        );
        let live = list_in_flight(&conn, now + settlement + 1).expect("list");
        assert!(
            live.is_empty(),
            "the far-future artifact collapsed to one window at the pin"
        );
    }

    #[test]
    fn insert_never_prunes_on_a_low_unsynced_clock() {
        let mut conn = mem();
        insert(&mut conn, &row("live", true, 100, 10_000), 50).expect("insert");
        insert(&mut conn, &row("later", false, 200, 10_000), 0).expect("zero clock");
        assert_eq!(
            count(&conn).expect("count"),
            2,
            "a zero clock prunes nothing"
        );
    }

    #[test]
    fn cap_eviction_prefers_pinned_terminals_over_live_rows() {
        // #367 money-review: at the cap, a PINNED terminal (finished business
        // awaiting the user's Remove) must lose the eviction contest to a LIVE
        // in-flight row — even an OLDER one — so a terminal squatter can never
        // erase a live swap's only re-attach handle.
        let mut conn = mem();
        let now = 1_700_000_000_i64;
        let cap = i64::try_from(MAX_SWAP_RECORDS).expect("cap fits");
        // Fill the table: one OLD live row + (cap-1) NEWER pinned terminals.
        insert(&mut conn, &row("live-old", true, now, now + 100_000), now).expect("live");
        for i in 0..cap - 1 {
            let id = format!("done{i:03}");
            insert(
                &mut conn,
                &row(&id, false, now + 10 + i, now + 100_000),
                now,
            )
            .expect("seed pinned");
            assert!(record_outcome(&mut conn, &id, SwapOutcome::Success, now).expect("pin"));
        }
        assert_eq!(count(&conn).expect("count"), MAX_SWAP_RECORDS as u64);
        // A fresh insert forces one eviction: a pinned terminal goes, the old
        // live row stays.
        insert(
            &mut conn,
            &row("live-new", true, now + 500, now + 100_000),
            now,
        )
        .expect("fresh insert");
        let live = list_in_flight(&conn, now + 1).expect("list");
        assert!(
            live.iter().any(|r| r.swap_id == "live-old"),
            "the OLD live row survives — pinned terminals evict first"
        );
        assert!(
            live.iter().any(|r| r.swap_id == "live-new"),
            "the just-inserted row survives (exemption unchanged)"
        );
        assert_eq!(count(&conn).expect("count"), MAX_SWAP_RECORDS as u64);
    }

    #[test]
    fn cap_evicts_the_oldest_rows_only() {
        let mut conn = mem();
        let cap = i64::try_from(MAX_SWAP_RECORDS).expect("cap fits");
        for i in 0..=cap {
            insert(&mut conn, &row(&format!("s{i:03}"), false, i, 100_000), 1)
                .expect("insert under cap");
        }
        assert_eq!(count(&conn).expect("count"), MAX_SWAP_RECORDS as u64);
        let live = list_in_flight(&conn, 2).expect("list");
        assert!(
            !live.iter().any(|r| r.swap_id == "s000"),
            "the OLDEST row was evicted"
        );
        assert!(
            live.iter().any(|r| r.swap_id == format!("s{cap:03}")),
            "the newest row survives the cap"
        );
    }

    #[test]
    fn record_outcome_pins_first_wins_and_survives_list() {
        let mut conn = mem();
        insert(&mut conn, &row("swap-1", true, 100, 10_000), 50).expect("insert");
        assert!(
            record_outcome(&mut conn, "swap-1", SwapOutcome::Refunded, 50).expect("pin"),
            "first pin lands"
        );
        assert!(
            !record_outcome(&mut conn, "swap-1", SwapOutcome::Success, 60).expect("second pin"),
            "a later conflicting pin is a no-op (first-wins — a terminal is forever)"
        );
        let live = list_in_flight(&conn, 200).expect("list");
        assert_eq!(
            live[0].outcome,
            Some(SwapOutcome::Refunded),
            "the FIRST observed terminal is what lists"
        );
    }

    #[test]
    fn record_outcome_on_absent_row_is_ok_false() {
        let mut conn = mem();
        assert!(
            !record_outcome(&mut conn, "ghost", SwapOutcome::Failed, 50).expect("absent pin"),
            "pinning an absent/lapsed row reports false, never errors"
        );
    }

    #[test]
    fn unknown_stored_outcome_reads_as_in_flight() {
        // Fail-honest forward-compat: a NEWER build's outcome vocabulary read by
        // this one must render as still-in-flight (the live poll re-reveals
        // truth), never as a wrong terminal.
        let mut conn = mem();
        insert(&mut conn, &row("swap-1", true, 100, 10_000), 50).expect("insert");
        conn.execute(
            "UPDATE swap_record SET outcome = 'partially-settled' WHERE swap_id = 'swap-1'",
            [],
        )
        .expect("simulate a newer build's pin");
        let live = list_in_flight(&conn, 200).expect("list");
        assert_eq!(live[0].outcome, None, "unknown outcome maps to None");
        assert!(
            !record_outcome(&mut conn, "swap-1", SwapOutcome::Failed, 70).expect("re-pin"),
            "the stored (unknown) pin still wins the IS NULL guard — no overwrite"
        );
    }

    #[test]
    fn ensure_table_migrates_an_older_table_additively() {
        // A pre-#367 build's 5-column table gains the nullable outcome column
        // (#367) AND the watch pair (#368) on the next ensure (the
        // issued_quote_store destination-column precedent); existing rows read
        // back with outcome = None and no watch (fail-honest — nothing was
        // recorded for them).
        let mut conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE swap_record (
                 swap_id          TEXT PRIMARY KEY,
                 out_of_zec       INTEGER NOT NULL,
                 created_at       INTEGER NOT NULL,
                 deposit_deadline INTEGER,
                 expires_at_wall  INTEGER NOT NULL
             );
             INSERT INTO swap_record VALUES ('old-row', 1, 100, 1000, 10000);",
        )
        .expect("pre-#367 shape");
        ensure_table(&conn).expect("migrate");
        ensure_table(&conn).expect("second ensure is idempotent");
        let live = list_in_flight(&conn, 200).expect("list");
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].outcome, None);
        assert_eq!(
            watch_of(&conn, "old-row").expect("watch"),
            None,
            "a pre-#368 row has no watch to re-arm"
        );
        assert!(
            record_outcome(&mut conn, "old-row", SwapOutcome::Success, 100).expect("pin"),
            "the migrated row accepts a pin"
        );
    }

    #[test]
    fn ensure_table_adds_watch_served_to_a_pre_385_table_defaulting_unserved() {
        // A #368..#382-era 8-column table (watch pair present, no `watch_served`)
        // gains the served latch on the next ensure, DEFAULT 0 — an existing
        // unresolved watch keeps re-arming exactly as before the upgrade (a
        // retired-but-unlatched pre-#385 leg resurrects as an ordinary
        // unresolved watch, polled until its record pins or is dismissed —
        // the documented upgrade bound; see the ensure_additive_columns doc).
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE swap_record (
                 swap_id          TEXT PRIMARY KEY,
                 out_of_zec       INTEGER NOT NULL,
                 created_at       INTEGER NOT NULL,
                 deposit_deadline INTEGER,
                 expires_at_wall  INTEGER NOT NULL,
                 outcome          TEXT,
                 watch_address    TEXT,
                 watch_index      INTEGER
             );
             INSERT INTO swap_record VALUES ('pre-385', 1, 100, 1000, 10000, NULL, 't1w', 4);",
        )
        .expect("pre-#385 shape");
        ensure_table(&conn).expect("migrate");
        assert_eq!(
            unresolved_watches(&conn).expect("watches"),
            vec![("pre-385".to_owned(), "t1w".to_owned(), 4)],
            "the migrated unresolved watch still re-arms (served defaults 0)"
        );
        // and the latch round-trips on the migrated table
        {
            let mut conn = conn;
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("txn");
            assert!(mark_watch_served_in_tx(&tx, "pre-385").expect("latch"));
            tx.commit().expect("commit");
            assert!(
                unresolved_watches(&conn).expect("watches").is_empty(),
                "the latched leg leaves the re-arm source"
            );
            assert_eq!(
                watch_of(&conn, "pre-385").expect("watch_of"),
                Some(("t1w".to_owned(), 4, true)),
                "the columns survive the latch"
            );
        }
    }

    #[test]
    fn insert_with_watch_arms_the_detection_row_atomically_and_round_trips() {
        // #368: ONE txn writes the home row (watch columns included) AND the
        // swap_destination detection row — record-first is watch-first. The
        // detection row carries the record's own settlement bound.
        let mut conn = mem();
        crate::swap_destination_store::ensure_table(&conn).expect("dest schema");
        insert_with_watch(
            &mut conn,
            &row("swap-w", true, 100, 10_000),
            50,
            "t1swapw",
            Some(("t1refund", 7)),
        )
        .expect("insert+arm");
        assert_eq!(
            watch_of(&conn, "swap-w").expect("watch"),
            Some(("t1refund".to_owned(), 7, true)),
            "the watch pair round-trips off the record row"
        );
        let active = crate::swap_destination_store::active(&conn, 5_000).expect("active");
        assert_eq!(
            active.len(),
            1,
            "the detection row was armed in the same txn"
        );
        assert_eq!(active[0].swap_id, "swap-w");
        assert_eq!(active[0].address, "t1refund");
        assert_eq!(active[0].index, 7);
        // watch-less rows stay watch-less (the wrapper form)
        insert(&mut conn, &row("swap-plain", false, 200, 10_000), 50).expect("plain");
        assert_eq!(watch_of(&conn, "swap-plain").expect("watch"), None);
        assert_eq!(
            crate::swap_destination_store::active(&conn, 5_000)
                .expect("active")
                .len(),
            1,
            "no detection row for a watch-less insert"
        );
    }

    #[test]
    fn watch_of_is_fail_honest_on_a_half_set_or_out_of_range_pair() {
        // A half-set pair or an over-u32 index (tamper-only) reads as None —
        // the re-arm is hygiene, never worth failing the pin path over.
        let conn = mem();
        conn.execute_batch(
            "INSERT INTO swap_record \
                 (swap_id, out_of_zec, created_at, deposit_deadline, expires_at_wall, \
                  watch_address, watch_index) \
             VALUES ('half', 1, 100, NULL, 10000, 't1x', NULL),
                    ('big',  1, 100, NULL, 10000, 't1y', 4294967296);",
        )
        .expect("seed tampered rows");
        assert_eq!(watch_of(&conn, "half").expect("half"), None);
        assert_eq!(watch_of(&conn, "big").expect("big"), None);
        assert_eq!(watch_of(&conn, "absent").expect("absent"), None);
    }

    #[test]
    fn unresolved_watches_returns_unpinned_watch_pairs_only() {
        // #382 (b) — the per-pass re-arm's source read: UNPINNED rows with a
        // present, valid watch pair, BOTH directions; pinned rows, watch-less
        // rows, and tampered pairs (half-set / out-of-range) are excluded
        // fail-honest.
        let mut conn = mem();
        crate::swap_destination_store::ensure_table(&conn).expect("dest schema");
        insert_with_watch(
            &mut conn,
            &row("out-live", true, 100, 10_000),
            50,
            "t1outlive",
            Some(("t1refund", 7)),
        )
        .expect("out-of-zec live");
        insert_with_watch(
            &mut conn,
            &row("in-live", false, 200, 10_000),
            50,
            "0xinlive",
            Some(("u1dest", 9)),
        )
        .expect("into-zec live");
        insert_with_watch(
            &mut conn,
            &row("pinned", true, 300, 10_000),
            50,
            "t1pinned",
            Some(("t1done", 11)),
        )
        .expect("pinned row");
        assert!(record_outcome(&mut conn, "pinned", SwapOutcome::Refunded, 50).expect("pin"));
        insert(&mut conn, &row("watchless", true, 400, 10_000), 50).expect("no watch");
        conn.execute_batch(
            "INSERT INTO swap_record \
                 (swap_id, out_of_zec, created_at, deposit_deadline, expires_at_wall, \
                  watch_address, watch_index) \
             VALUES ('tampered', 1, 500, NULL, 10000, 't1z', 4294967296),
                    ('typecorrupt', 1, 550, NULL, 10000, 't1q', 'not-a-number');",
        )
        .expect("seed tampered rows");
        let watches = unresolved_watches(&conn).expect("read");
        assert_eq!(
            watches,
            vec![
                ("in-live".to_owned(), "u1dest".to_owned(), 9),
                ("out-live".to_owned(), "t1refund".to_owned(), 7),
            ],
            "both directions' UNPINNED pairs only, deterministic order; the \
             out-of-range AND the type-corrupt row are each SKIPPED (S196: one \
             poison row must never starve every other swap's re-arm)"
        );
    }

    #[test]
    fn pin_chain_observed_refund_guards_direction_and_first_wins() {
        // #382 (c): the chain pin lands ONLY on an unpinned OutOfZec row — an
        // IntoZec funded destination is a DELIVERY (no-op here), an existing
        // pin is forever (first-wins, even against the chain observation —
        // idempotent re-observation), and an absent id (a `backfill:` synthetic
        // row has no record) is Ok(false).
        let mut conn = mem();
        insert(&mut conn, &row("out", true, 100, 10_000), 50).expect("out-of-zec");
        insert(&mut conn, &row("into", false, 200, 10_000), 50).expect("into-zec");
        assert!(
            pin_chain_observed_refund(&mut conn, "out", 100).expect("pin out"),
            "an unpinned OutOfZec row takes the chain pin"
        );
        assert_eq!(
            list_in_flight(&conn, 300)
                .expect("list")
                .iter()
                .find(|r| r.swap_id == "out")
                .expect("row")
                .outcome,
            Some(SwapOutcome::Refunded),
            "the pinned outcome is Refunded"
        );
        assert!(
            !pin_chain_observed_refund(&mut conn, "out", 110).expect("re-pin"),
            "first-wins: a second observation is a no-op"
        );
        assert!(
            !pin_chain_observed_refund(&mut conn, "into", 100).expect("into-zec"),
            "an IntoZec funded destination never mislabels as Refunded"
        );
        assert_eq!(
            list_in_flight(&conn, 300)
                .expect("list")
                .iter()
                .find(|r| r.swap_id == "into")
                .expect("row")
                .outcome,
            None,
            "the IntoZec row stays unpinned"
        );
        assert!(
            !pin_chain_observed_refund(&mut conn, "backfill:3", 100).expect("absent"),
            "a synthetic backfill id has no record — Ok(false)"
        );
    }

    #[test]
    fn dismiss_is_idempotent() {
        let mut conn = mem();
        insert(&mut conn, &row("swap-1", true, 100, 10_000), 50).expect("insert");
        assert!(dismiss(&mut conn, "swap-1").expect("first dismiss"));
        assert!(!dismiss(&mut conn, "swap-1").expect("second dismiss is Ok(false)"));
        assert!(list_in_flight(&conn, 200).expect("list").is_empty());
    }

    #[test]
    fn ensure_table_is_idempotent_and_preserves_rows() {
        let mut conn = mem();
        insert(&mut conn, &row("swap-1", true, 100, 10_000), 50).expect("insert");
        ensure_table(&conn).expect("second ensure");
        assert_eq!(count(&conn).expect("count"), 1);
    }

    #[test]
    fn injection_shaped_id_is_stored_verbatim_never_executed() {
        // §4.6: provider strings reach SQL as bind values only. An id crafted as
        // SQL must round-trip as DATA and delete nothing else.
        let mut conn = mem();
        let hostile = "x'; DROP TABLE swap_record; --";
        insert(&mut conn, &row(hostile, true, 100, 10_000), 50).expect("insert");
        insert(&mut conn, &row("victim", false, 200, 10_000), 50).expect("insert victim");
        let live = list_in_flight(&conn, 300).expect("list");
        assert_eq!(live.len(), 2, "table intact, hostile id stored as data");
        // The #367 outcome pin is bind-only too — a SQL-shaped id pins its OWN
        // row and touches nothing else (security review fold: every write path
        // on this table stays under the §4.6 named-test contract).
        assert!(
            record_outcome(&mut conn, hostile, SwapOutcome::Failed, 250)
                .expect("pin the hostile id"),
            "the hostile id's own row accepts the pin"
        );
        let live = list_in_flight(&conn, 300).expect("list");
        assert_eq!(live.len(), 2, "table intact after the pin");
        assert!(
            live.iter()
                .any(|r| r.swap_id == "victim" && r.outcome.is_none()),
            "the victim row is untouched by the hostile pin"
        );
        assert!(dismiss(&mut conn, hostile).expect("dismiss the hostile id"));
        assert_eq!(
            list_in_flight(&conn, 300).expect("list")[0].swap_id,
            "victim"
        );
        // #368: the watch columns + the composed detection-row arm are bind-only
        // too — a SQL-shaped ADDRESS round-trips as data through BOTH tables.
        crate::swap_destination_store::ensure_table(&conn).expect("dest schema");
        let hostile_addr = "t1'; DROP TABLE swap_destination; --";
        insert_with_watch(
            &mut conn,
            &row("swap-inj", true, 400, 10_000),
            50,
            hostile_addr,
            Some((hostile_addr, 3)),
        )
        .expect("insert+arm with a hostile address");
        assert_eq!(
            watch_of(&conn, "swap-inj").expect("watch"),
            Some((hostile_addr.to_owned(), 3, true)),
            "the hostile address is data, not SQL"
        );
        let active = crate::swap_destination_store::active(&conn, 5_000).expect("active");
        assert_eq!(active.len(), 1, "swap_destination intact + armed");
        assert_eq!(active[0].address, hostile_addr);
    }

    #[test]
    fn a_duplicate_sdk_id_is_a_typed_invariant_error_never_first_wins_in_the_swap_record_store() {
        // S8 `identity` row 8: the home row is keyed by the SDK-minted identity, so a second
        // insert under the SAME id is an invariant violation — refused TYPED (the base commit's
        // `INSERT OR IGNORE` absorbed it and kept the older stamps) — and the first row's
        // stamps AND its armed watch survive; the duplicate's watch is never armed, so nothing
        // relinks. Reddening mutation: `OR IGNORE` (the base commit).
        let mut conn = mem();
        crate::swap_destination_store::ensure_table(&conn).expect("watch schema");
        let base = 1_700_000_000_i64;
        insert_with_watch(
            &mut conn,
            &row("swap-1", true, base, base + 999_000),
            base,
            "t1provider",
            Some(("t1refund-first", 1)),
        )
        .expect("first");
        assert!(
            insert_with_watch(
                &mut conn,
                &row("swap-1", false, base + 500, base + 999_500),
                base,
                "t1provider",
                Some(("t1refund-second", 2)),
            )
            .is_err(),
            "a duplicate SDK id is a typed error, never a silent first-wins"
        );
        let live = list_in_flight(&conn, base + 1).expect("list");
        assert_eq!(
            live,
            vec![row("swap-1", true, base, base + 999_000)],
            "the first row's stamps survive"
        );
        let watches = crate::swap_destination_store::active(&conn, base + 1).expect("active");
        assert_eq!(
            watches
                .iter()
                .map(|w| (w.swap_id.as_str(), w.address.as_str(), w.index))
                .collect::<Vec<_>>(),
            vec![("swap-1", "t1refund-first", 1)],
            "the first watch stays armed at its own address; the duplicate armed nothing"
        );
    }
}
