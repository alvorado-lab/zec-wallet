//! The durable in-flight issued-swap-quote store (§3.2h W-swap-3-c-1; §3.2 W2).
//!
//! `SwapService` accepts `execute` ONLY for a quote it ISSUED (via `quote`) — the
//! in-memory issued-quote registry is what makes a hand-built `SwapQuote` unable to
//! reach the provider, and what one-shot-consumes a quote so it executes at most once.
//! At W2 that registry was IN-MEMORY: a process kill between `quote` and `execute`
//! emptied it, so the recovery was RE-QUOTING — and a re-quote yields a NEW deposit
//! address, so a host that re-quoted after a dropped `execute` future could queue a
//! SECOND deposit to the new address. No ZEC is ever LOST (every deposit is gated by
//! the durable §4.4 `deposit_gate` before signing), but the user could be double-
//! CHARGED across a crash-then-requote (§3.2 W2, documented). This module owns the
//! DURABLE half that closes that window: an issued quote SURVIVES a kill, so the
//! post-crash `execute` finds the SAME quote (the SAME frozen deposit address) and is
//! idempotent — single-flight keyed on the quote id rather than process memory.
//!
//! **Storage.** A small table in the ONE sealed/wiped/backup-excluded `wallet.db`,
//! riding the SECOND SQLCipher-keyed aux connection (`Inner.aux_db`) exactly like
//! [`crate::intent_store`] / [`crate::refund_index`] — the engine's `WalletDb` exposes
//! no connection accessor, and an in-flight quote (with its committed deposit address +
//! amount) is durable money-adjacent state that belongs in the single sealed file (a
//! second durable file would replicate the seal/wipe/backup surface). The table is
//! created idempotently in `db::migrate` (the one chokepoint both provision and open
//! route through), so an existing wallet gains it on next open with NO
//! `WALLET_SCHEMA_VERSION` bump. Functions take a bare `&mut Connection`/`&Connection`
//! so they unit-test against a plain connection with no `Wallet`.
//!
//! **Deadlock-freedom + crash-safety.** Every write is ONE `BEGIN IMMEDIATE`
//! transaction: it takes `RESERVED` atomically and never sits on a bare `SHARED` it
//! intends to upgrade, so it cannot deadlock the engine's `DEFERRED` writer (the
//! load-bearing invariant documented in [`crate::intent_store`]); a killed write leaves
//! a clean committed prefix, never a torn row. [`take`] is itself ONE `IMMEDIATE` txn
//! (SELECT-then-DELETE), so it is the **atomic single-flight authority**: two
//! concurrent (or post-crash) executes of the same quote can never both observe the row
//! — exactly one removes it, the other gets `None`.
//!
//! **Trust boundary.** The deposit address reaching [`persist`] was already validated by
//! `SwapService::validate_quote` (parseable, transparent, right-network — the swap-b L-1
//! door, the SSOT) and is frozen per quote id, so this layer stores it WITHOUT
//! re-validating (DRY: one validation door). On the way OUT, [`take`] and [`peek`]
//! re-check only the STRUCTURAL invariants a tampered sealed DB could violate — the
//! deposit pair (address ⟺ amount both present or both absent), a non-negative amount,
//! and the S8 terms group (present whole when the provider handle is) — fail-closed
//! `StoreCorrupt`, never a silently half-formed deposit. The amount/deposit-domain
//! reconstruction (`Address`/`Zatoshis`) is the consumer's (the W-swap-3-c-2 hook).
//!
//! **The key (stage S8, R01).** `id` is the SDK-MINTED execution identity, never a value
//! the provider chose; a duplicate is an invariant violation the plain `INSERT`'s
//! primary-key conflict surfaces LOUD (`StoreCorrupt`) — the pre-S8 first-wins
//! short-circuit is gone, because it was the mechanism that let a provider reusing a
//! deposit address bind two quotes' terms to one row. The provider's own handle rides
//! the row as `provider_ref`, data for the status poll.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::db::map_aux_err as map_db_err;
use crate::error::WalletError;
use crate::seed::SpendBinding;

/// The far-future legitimacy cutoff (W-swap-4-a-4, spec §4.4): a row with
/// `expires_at_wall > now + SWAP_DEADLINE_DEFAULT_SECS + FAR_FUTURE_SKEW_ALLOWANCE_SECS`
/// cannot have been written by a post-clamp build — every persist ceiling-clamps the
/// stored wall deadline to at most 24 h ahead (`SwapService::quote`, direction-aware),
/// a bound that only tightens as `now` advances UNDER A NON-DECREASING CLOCK — so such
/// a row is PRE-UPGRADE (a live ~72 h provider echo persisted by an older build) or
/// hostile by construction. Rows past the cutoff are swept by the prunes and refused by
/// [`take`]: left alone they stay executable for days AND hold `MAX_ISSUED_QUOTES`
/// capacity (a wipe-only swap brick at 16). Too-short is the money-safe direction: a
/// swept/refused quote costs a re-quote, never funds.
///
/// THE SKEW ALLOWANCE (review fold — the 3-review converged finding): the wall
/// clock is NOT non-decreasing. The live provider ignores our requested window, so
/// every live IntoZec row sits at EXACTLY `persist_now + 24 h` — with a zero-margin
/// cutoff, a 1-second backward NTP step between quote and any read arm would refuse
/// and delete a live quote into a misleading "not issued" miss. One hour of allowance
/// absorbs real clock corrections; a genuine ~72 h pre-clamp echo still sits ~47 h
/// past the cutoff, so the sweep loses no teeth.
///
/// DISABLED below the clock-plausibility floor (returns `i64::MAX`, which no row can
/// exceed): with a broken/1970 device clock, `now + 24 h` would sit BELOW every live
/// deadline and the far-future arm would mass-sweep the whole table. The lapsed arm
/// (`expires_at_wall <= now`) needs no such guard — a too-low `now` just prunes nothing.
fn far_future_cutoff(now_unix: i64) -> i64 {
    let floor = i64::try_from(crate::constants::CLOCK_PLAUSIBILITY_FLOOR_SECS).unwrap_or(i64::MAX);
    if now_unix < floor {
        return i64::MAX;
    }
    now_unix
        .saturating_add(
            i64::try_from(crate::constants::SWAP_DEADLINE_DEFAULT_SECS).unwrap_or(i64::MAX),
        )
        .saturating_add(
            i64::try_from(crate::constants::FAR_FUTURE_SKEW_ALLOWANCE_SECS).unwrap_or(i64::MAX),
        )
}

/// A durable issued-quote row — the primitive shape persisted/loaded (the swap-domain
/// `SwapId`/`Address`/`Zatoshis` reconstruction is the consumer's, W-swap-3-c-2). For an
/// `OutOfZec` quote `deposit_address` + `deposit_amount_zat` carry the frozen §4.4 deposit
/// the wallet commits to send on execute; for `IntoZec` both are `None` (ZEC is RECEIVED to
/// our own address — no wallet-side deposit). `expires_at_wall` is the quote's wall-clock
/// deadline (unix seconds) — the CROSS-RESTART deadline authority (a monotonic `Instant`
/// cannot survive a process exit, so the durable form keeps the wall deadline; the
/// within-process monotonic gate stays on the in-memory registry the consumer keeps).
///
/// `destination_address` + `destination_index` carry the fresh per-swap IntoZec DESTINATION
/// (§3.3b D1 / ADR-0530; IZ-1): the engine-persisted UA handed to 1Click as `recipient` plus
/// the single-use external index it was minted at. `Some` for `IntoZec`, `None` for `OutOfZec`
/// — the MIRROR of the deposit pair (a quote carries a deposit OR a destination, by direction;
/// that direction-exclusivity is enforced by the SERVICE in `quote`, where the direction is
/// known — this layer enforces only the structural pair-consistency below). They are persisted
/// so a crash between quote and execute reconstructs the detection-set entry (§3.3b L1/L2): the
/// post-`open()` scoped poll re-derives `active_destinations()` from these rows.
///
/// **Type convention.** `expires_at_wall` is `i64` to match the durable-deadline column the
/// sibling `intent_store` already uses (`deposit_deadline: Option<i64>`) — SQLite INTEGER is
/// signed, so this is the one consistent durable wall-time type. The swap layer's `u64`
/// `SwapQuote::expires_at` converts at the SINGLE -c-2 persist boundary via the SAME
/// saturating `i64::try_from(u64).unwrap_or(0)` `queue_deposit_send` uses (an absurd
/// deadline beyond i64::MAX saturates to 0 = already-expired = fail-CLOSED, never an
/// eternal row), so the conversion is one documented cast, not scattered `as i64`s.
/// `destination_index` is `i64` for the same reason (SQLite signed INTEGER); the consumer's
/// `u32` external index (always in `[1, 2^31-1]`) converts losslessly, and a stored value
/// outside `[0, i64::from(u32::MAX)]` is rejected `StoreCorrupt` on the way out.
///
/// `refund_address` + `refund_index` carry the fresh per-swap OutOfZec REFUND t-addr (#368 /
/// ADR-0527): the provider's refundTo (engine-registered at mint, cross-checked against the raw
/// BIP44 derivation) plus its single-use external index — the SAME shared counter destinations
/// draw from. `Some` for `OutOfZec` (alongside the deposit pair), `None` for `IntoZec` (whose
/// inbound leg is the destination). Persisted so execute can arm the refund watch from the taken
/// record (the durable handle the pre-#368 derive-and-forget path never had). Same pair + range
/// validation as the destination columns.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) struct StoredQuote {
    pub(crate) id: String,
    pub(crate) deposit_address: Option<String>,
    pub(crate) deposit_amount_zat: Option<i64>,
    pub(crate) expires_at_wall: i64,
    pub(crate) destination_address: Option<String>,
    pub(crate) destination_index: Option<i64>,
    pub(crate) refund_address: Option<String>,
    pub(crate) refund_index: Option<i64>,
    /// FR-17 (#396): the spend-binding nonce minted when this quote was issued —
    /// surfaced on the `SwapQuote` DTO (the host records it at its execute-authorize
    /// bracket) and COPIED onto the deposit's `queued_send_intent` row at execute, so
    /// the sign-at-execute pull presents it. `None` for a pre-FR-17 row.
    pub(crate) binding: Option<SpendBinding>,
    /// S8 (R01): the provider's own handle for this quote (1Click: the deposit
    /// address) — the adapter's `id`, kept as DATA once the SDK minted the key. `None`
    /// only on a pre-S8 row, which the consumer treats as a miss (its terms were never
    /// recorded, so nothing can be compared against them). §5.4 NEVER-log: it IS the
    /// provider deposit address, the marker moves with the value (contract §3.1 row 9).
    pub(crate) provider_ref: Option<String>,
    /// S8 (R01): the terms the approval showed, both directions — what the consumer
    /// compares the caller's DTO against before consuming the row. The five
    /// `term_deposit_address` / `term_amount_in` / `term_min_amount_out` /
    /// `term_zec_side_zat` / `term_expires_at` are present whenever `provider_ref` is
    /// (a half-set group is tamper, `StoreCorrupt`); `term_deposit_memo` and
    /// `term_refund_to` are legitimately absent per direction. Distinct from
    /// `deposit_address` / `deposit_amount_zat` (the OutOfZec SEND plan),
    /// `refund_address` (the minted refund watch) and `expires_at_wall` (the
    /// un-margined wall deadline the gate enforces) — those are what the wallet does,
    /// these are what the user saw; `term_expires_at` is the ACTIONABLE deadline the
    /// returned DTO carried (wall − the direction's execute margin), verbatim.
    pub(crate) term_deposit_address: Option<String>,
    pub(crate) term_deposit_memo: Option<String>,
    pub(crate) term_amount_in: Option<String>,
    pub(crate) term_min_amount_out: Option<String>,
    pub(crate) term_zec_side_zat: Option<i64>,
    pub(crate) term_refund_to: Option<String>,
    pub(crate) term_expires_at: Option<i64>,
}

/// Whether [`persist`] stored the quote or refused it at the in-flight cap. Returned (not a
/// typed error) so the consumer maps `AtCapacity` to its own `SwapError` (the in-memory
/// registry already returns `RequestInvalid{"too many in-flight quotes"}`; the durable cap
/// is its crash-loop backstop) without this layer owning a swap error.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) enum PersistOutcome {
    Persisted,
    AtCapacity,
}

/// Create the issued-quote table if absent. Idempotent (`CREATE TABLE IF NOT EXISTS`), so
/// `db::migrate` calls it on EVERY provision and open — a wallet provisioned before this
/// table existed gains it on the next open, with NO `WALLET_SCHEMA_VERSION` bump (the
/// [`crate::intent_store`] / [`crate::refund_index`] precedent).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS issued_swap_quote (
             id                 TEXT PRIMARY KEY,
             deposit_address    TEXT,
             deposit_amount_zat INTEGER,
             expires_at_wall    INTEGER NOT NULL
         );",
    )
    .map_err(map_db_err)?;
    // IZ-1 (§3.3b D1): the IntoZec DESTINATION columns, added ADDITIVELY so a wallet
    // provisioned before IZ-1 gains them on its next open with NO `WALLET_SCHEMA_VERSION`
    // bump and NO table rebuild (the issued-quote table is ephemeral in-flight state — a
    // pre-IZ-1 wallet has at most a handful of unconsumed rows, all `OutOfZec` with NULL
    // destination, so the additive columns are correct for them too). The
    // [`crate::intent_store`] guard-column precedent.
    ensure_destination_columns(conn)
}

/// Idempotently add the nullable `destination_address`/`destination_index` columns (IZ-1 /
/// §3.3b D1). `ALTER TABLE ADD COLUMN` errors if the column already exists, so consult
/// `PRAGMA table_info` first and add only what is absent (the [`crate::intent_store`]
/// `ensure_guard_columns` precedent) — making `ensure_table` safe to re-run on EVERY
/// provision/open (the `db::migrate` chokepoint), across a pre-IZ-1 or a fresh wallet alike.
/// Both nullable with no default ⇒ each `ALTER ADD COLUMN` is O(1) metadata, never a
/// data-bearing rebuild on the boot path (rust-patterns).
///
/// NOT a TOCTOU hazard: in production `migrate` runs only under the exclusive single-opener
/// `WalletLock` flock (see `lifecycle.rs`), so no second connection can race the ALTER; tests
/// each use a fresh DB. No two connections ever run this concurrently.
fn ensure_destination_columns(conn: &Connection) -> Result<(), WalletError> {
    let existing = table_columns(conn)?;
    for (name, decl) in [
        ("destination_address", "destination_address TEXT"),
        ("destination_index", "destination_index INTEGER"),
        // #368 (ADR-0527): the OutOfZec refund pair — additive exactly like the
        // destination pair; a pre-#368 row's NULLs are correct for it (its refund
        // was derive-and-forget, there is nothing to reconstruct).
        ("refund_address", "refund_address TEXT"),
        ("refund_index", "refund_index INTEGER"),
        // FR-17 (#396): the spend-binding nonce — additive; a pre-FR-17 row's NULL is
        // correct (no binding was ever shown to the host for it ⇒ its deposit signs
        // unbound).
        ("binding", "binding BLOB"),
        // S8 (R01): the provider handle + the approval's terms — additive; a pre-S8
        // row's NULLs make it a consumer-side miss (re-quote), the dev-stage drop
        // posture. Appended LAST, in this order (the `copy_aux_tables`
        // ordinal-convergence rule).
        ("provider_ref", "provider_ref TEXT"),
        ("term_deposit_address", "term_deposit_address TEXT"),
        ("term_deposit_memo", "term_deposit_memo TEXT"),
        ("term_amount_in", "term_amount_in TEXT"),
        ("term_min_amount_out", "term_min_amount_out TEXT"),
        ("term_zec_side_zat", "term_zec_side_zat INTEGER"),
        ("term_refund_to", "term_refund_to TEXT"),
        // S8 row 2 (the adjudication's repair): the deadline the approval showed —
        // appended after the group above, same rule.
        ("term_expires_at", "term_expires_at INTEGER"),
    ] {
        if !existing.contains(name) {
            conn.execute_batch(&format!("ALTER TABLE issued_swap_quote ADD COLUMN {decl}"))
                .map_err(map_db_err)?;
        }
    }
    Ok(())
}

/// The set of column names currently on `issued_swap_quote` (via `PRAGMA table_info`, whose
/// row column 1 is the name). Surfaces any query error (rust-patterns: never swallow a DB
/// error on a schema read).
fn table_columns(conn: &Connection) -> Result<std::collections::HashSet<String>, WalletError> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(issued_swap_quote)")
        .map_err(map_db_err)?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(map_db_err)?;
    names
        .collect::<Result<std::collections::HashSet<_>, _>>()
        .map_err(map_db_err)
}

/// Durably persist an issued quote. ONE `IMMEDIATE` transaction: prune any wall-expired
/// rows (a quote-looping host never grows the table unboundedly), refuse a DUPLICATE id
/// before anything else is decided, enforce the in-flight `cap` (a crash-loop backstop —
/// at capacity a NEW quote is refused `AtCapacity`, the existing rows untouched), then a
/// plain INSERT under the SDK-minted id. A duplicate id is an INVARIANT VIOLATION (S8:
/// the key is 128 bits from the OS CSPRNG, never a provider value), so it fails LOUD —
/// `StoreCorrupt` — rather than first-wins: the pre-S8 `already_present` short-circuit
/// was what let a provider reusing a deposit address persist two quotes' terms as one
/// row (principle 10, and R01). It is detected BEFORE the cap is counted on purpose: an
/// existing id at capacity is still the invariant violation, never `AtCapacity` — the
/// consumer maps `AtCapacity` to "too many in-flight quotes — execute or let one
/// expire", which would be a lie about why the write was refused and the wrong next
/// step (contract §3.1 row 8). The INSERT's primary-key conflict stays as the second
/// door (`StoreCorrupt` via `map_db_err`).
///
/// `now_unix` is the caller's monotonic-floor-checked wall clock (the §4.4 deadline-tag
/// convention); the deposit pair (`deposit_address` ⟺ `deposit_amount_zat`) MUST be both
/// present or both absent — a half-set argument is a caller bug, rejected `StoreCorrupt`
/// before any write.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn persist(
    conn: &mut Connection,
    quote: &StoredQuote,
    cap: usize,
    now_unix: i64,
) -> Result<PersistOutcome, WalletError> {
    // Structural pre-check (defense-in-depth): never write a half-formed deposit. The
    // -c-2 consumer builds the pair atomically from one `Option<DepositPlan>` (both-or-
    // neither by construction), so a half-set argument is an unreachable caller-invariant
    // violation — surfaced fail-closed + typed (NOT a panic: a panic across the eventual
    // FFI boundary is a crash, the whole §4.2 typed-not-panic posture) rather than writing
    // an inconsistent row. `StoreCorrupt` is the closest fail-closed "data is inconsistent"
    // signal; it never fires in practice.
    if quote.deposit_address.is_some() != quote.deposit_amount_zat.is_some() {
        return Err(WalletError::StoreCorrupt);
    }
    // Destination pair (IZ-1): the IntoZec destination address ⟺ its index, both-or-neither
    // (the deposit-pair invariant's mirror). The -c-2 consumer builds the pair atomically from
    // one `Option<IssuedDestination>`, so a half-set argument is an unreachable caller-invariant
    // violation — fail-closed + typed, never a half-formed row.
    if quote.destination_address.is_some() != quote.destination_index.is_some() {
        return Err(WalletError::StoreCorrupt);
    }
    // Refund pair (#368): both-or-neither, the destination-pair mirror — the consumer
    // builds it atomically from one `Option<IssuedRefund>`.
    if quote.refund_address.is_some() != quote.refund_index.is_some() {
        return Err(WalletError::StoreCorrupt);
    }
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    // Prune wall-expired AND far-future rows first — keeps the table bounded for a
    // quote-polling host, means the cap counts only still-live LEGITIMATE quotes, and
    // frees capacity held by pre-clamp/hostile far-future rows ([`far_future_cutoff`];
    // the sweep is what un-bricks a capacity-full pre-upgrade wallet on its next quote).
    tx.execute(
        "DELETE FROM issued_swap_quote WHERE expires_at_wall <= ?1 OR expires_at_wall > ?2",
        rusqlite::params![now_unix, far_future_cutoff(now_unix)],
    )
    .map_err(map_db_err)?;
    // The duplicate door, FIRST (S8 row 8): the id is SDK-minted, so a row already
    // carrying it is an invariant violation (a broken CSPRNG, a logic bypass
    // re-persisting a live row) whatever the table's occupancy — refused typed
    // `StoreCorrupt` before the cap can answer for it. Inside the same IMMEDIATE tx as
    // the INSERT, so no writer can slip a row in between the check and the write.
    let already_present = tx
        .query_row(
            "SELECT 1 FROM issued_swap_quote WHERE id = ?1",
            rusqlite::params![quote.id],
            |_| Ok(()),
        )
        .optional()
        .map_err(map_db_err)?
        .is_some();
    if already_present {
        return Err(WalletError::StoreCorrupt);
    }
    let live: i64 = tx
        .query_row("SELECT COUNT(*) FROM issued_swap_quote", [], |r| r.get(0))
        .map_err(map_db_err)?;
    // `COUNT(*)` is never negative; compare in i64 like the `intent_store` cap (it casts
    // the constant up, not the count down) so a huge `cap` can't truncate.
    let outcome = if live >= cap as i64 {
        PersistOutcome::AtCapacity
    } else {
        // Plain INSERT (NOT `ON CONFLICT DO NOTHING`): a PK conflict the door above did
        // not see is still the invariant violation and still LOUD (`StoreCorrupt` via
        // `map_db_err`), never silently swallowed (no silent failures, principle 10).
        tx.execute(
            "INSERT INTO issued_swap_quote \
                 (id, deposit_address, deposit_amount_zat, expires_at_wall, \
                  destination_address, destination_index, refund_address, refund_index, \
                  binding, provider_ref, term_deposit_address, term_deposit_memo, \
                  term_amount_in, term_min_amount_out, term_zec_side_zat, term_refund_to, \
                  term_expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, \
                     ?17)",
            rusqlite::params![
                quote.id,
                quote.deposit_address,
                quote.deposit_amount_zat,
                quote.expires_at_wall,
                quote.destination_address,
                quote.destination_index,
                quote.refund_address,
                quote.refund_index,
                quote.binding.map(|b| b.as_bytes().to_vec()),
                quote.provider_ref,
                quote.term_deposit_address,
                quote.term_deposit_memo,
                quote.term_amount_in,
                quote.term_min_amount_out,
                quote.term_zec_side_zat,
                quote.term_refund_to,
                quote.term_expires_at,
            ],
        )
        .map_err(map_db_err)?;
        PersistOutcome::Persisted
    };
    // Commit either way — the prune is valid cleanup regardless of the insert outcome.
    tx.commit().map_err(map_db_err)?;
    Ok(outcome)
}

/// Read one row by id — the SELECT shared by [`take`] and [`peek`] (one column list,
/// one decode). No validation here; [`check_row`] is the shared door.
fn read_row(conn: &Connection, id: &str) -> Result<Option<StoredQuote>, WalletError> {
    conn.query_row(
        "SELECT id, deposit_address, deposit_amount_zat, expires_at_wall, \
                destination_address, destination_index, refund_address, refund_index, \
                binding, provider_ref, term_deposit_address, term_deposit_memo, \
                term_amount_in, term_min_amount_out, term_zec_side_zat, term_refund_to, \
                term_expires_at \
         FROM issued_swap_quote WHERE id = ?1",
        rusqlite::params![id],
        |r| {
            Ok(StoredQuote {
                id: r.get(0)?,
                deposit_address: r.get(1)?,
                deposit_amount_zat: r.get(2)?,
                expires_at_wall: r.get(3)?,
                destination_address: r.get(4)?,
                destination_index: r.get(5)?,
                refund_address: r.get(6)?,
                refund_index: r.get(7)?,
                // FR-17: NULL or a wrong-length blob decodes to `None`
                // (validate-never-truncate — an unbound pull fail-closes at a
                // bound host; corruption can never pad into a "valid" token).
                binding: r
                    .get::<_, Option<Vec<u8>>>(8)?
                    .as_deref()
                    .and_then(SpendBinding::from_slice),
                provider_ref: r.get(9)?,
                term_deposit_address: r.get(10)?,
                term_deposit_memo: r.get(11)?,
                term_amount_in: r.get(12)?,
                term_min_amount_out: r.get(13)?,
                term_zec_side_zat: r.get(14)?,
                term_refund_to: r.get(15)?,
                term_expires_at: r.get(16)?,
            })
        },
    )
    .optional()
    .map_err(map_db_err)
}

/// Validate-before-trust on a DB-sourced row (a tampered/corrupt sealed row): the
/// STRUCTURAL invariants [`take`] and [`peek`] both re-check on the way out —
/// fail-closed `StoreCorrupt`, never a silently half-formed deposit, destination,
/// refund or terms group.
fn check_row(row: &StoredQuote) -> Result<(), WalletError> {
    if row.deposit_address.is_some() != row.deposit_amount_zat.is_some()
        || row.deposit_amount_zat.is_some_and(|a| a < 0)
        || row.destination_address.is_some() != row.destination_index.is_some()
        // the external index must fit the consumer's `u32` domain (the `StoredQuote` doc): a
        // negative OR a `> u32::MAX` value is a tampered sealed row — fail-closed HERE (the
        // validation door), never a panic at the consumer's `u32::try_from` across the FFI boundary.
        || row
            .destination_index
            .is_some_and(|i| i < 0 || i > i64::from(u32::MAX))
        // the refund pair + index domain (#368) — the destination checks' mirror
        || row.refund_address.is_some() != row.refund_index.is_some()
        || row
            .refund_index
            .is_some_and(|i| i < 0 || i > i64::from(u32::MAX))
        // S8: the terms group rides whole with the provider handle — a half-set group,
        // a negative amount or a negative shown deadline is tamper (the memo/refund
        // terms are per-direction optional and not checked here)
        || row.provider_ref.is_some()
            && (row.term_deposit_address.is_none()
                || row.term_amount_in.is_none()
                || row.term_min_amount_out.is_none()
                || row.term_zec_side_zat.is_none_or(|z| z < 0)
                || row.term_expires_at.is_none_or(|t| t < 0))
    {
        return Err(WalletError::StoreCorrupt);
    }
    Ok(())
}

/// Read an issued quote WITHOUT consuming it — the non-claiming read the consumer's
/// caller-DTO terms compare runs on BEFORE [`take`] (S8): a DTO whose terms differ is
/// refused with the row intact, so a later honest execute still claims it. Same
/// structural door as `take` ([`check_row`]) and the same [`far_future_cutoff`] refusal
/// (reported as a miss, the row left for the prune sweeps). No transaction: a single
/// SELECT on a row that is immutable once written.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn peek(
    conn: &Connection,
    id: &str,
    now_unix: i64,
) -> Result<Option<StoredQuote>, WalletError> {
    let Some(row) = read_row(conn, id)? else {
        return Ok(None);
    };
    check_row(&row)?;
    if row.expires_at_wall > far_future_cutoff(now_unix) {
        return Ok(None);
    }
    Ok(Some(row))
}

/// Atomically CONSUME (remove) an issued quote — the cross-restart one-shot single-flight
/// authority. ONE `IMMEDIATE` transaction reads the row then deletes it, so two concurrent
/// (or a post-crash duplicate) `take`s of the same id can never both succeed: exactly one
/// gets `Some(quote)` (and removes it), the other gets `None` (⇒ the consumer returns
/// `RequestInvalid` — not issued, or already executed). Returns `None` for an unknown id.
///
/// On the way out the STRUCTURAL invariants a tampered sealed DB could violate are
/// re-checked (validate-before-trust, §4.6): the deposit pair must be both-present or
/// both-absent, and the amount non-negative — either violation is fail-closed
/// `StoreCorrupt`, never a silently half-formed or negative-amount deposit.
///
/// `now_unix` (the caller's wall clock, W-swap-4-a-4) feeds the [`far_future_cutoff`]
/// refusal: a row no post-clamp build could have written is DELETED and reported as a
/// miss (`None` ⇒ the consumer's "not issued" remedy: re-quote) rather than handed to
/// the execute path — the read-side twin of the prune sweeps, so a pre-upgrade ~72 h
/// row can neither execute days past the honest window nor survive its first touch.
#[cfg_attr(not(feature = "swap"), allow(dead_code))]
pub(crate) fn take(
    conn: &mut Connection,
    id: &str,
    now_unix: i64,
) -> Result<Option<StoredQuote>, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let row = match read_row(&tx, id)? {
        None => {
            // Nothing to consume — commit the (no-op) txn and report a miss.
            tx.commit().map_err(map_db_err)?;
            return Ok(None);
        }
        Some(row) => row,
    };
    // Validate-before-trust on DB-sourced data (a tampered/corrupt sealed row).
    check_row(&row)?;
    tx.execute(
        "DELETE FROM issued_swap_quote WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    // Far-future refusal AFTER the structural checks (tamper is corruption regardless of
    // the clock) and AFTER the delete (the row must not survive its first touch either
    // way): a pre-clamp/hostile deadline is a MISS, never an executable quote.
    if row.expires_at_wall > far_future_cutoff(now_unix) {
        return Ok(None);
    }
    Ok(Some(row))
}

/// Delete every quote whose wall deadline has lapsed (`expires_at_wall <= now_unix`) OR
/// sits past the [`far_future_cutoff`] (pre-clamp/hostile, W-swap-4-a-4), returning how
/// many were removed. ONE `IMMEDIATE` transaction. Standalone AND wired at wallet open
/// (W-swap-4-a-4): the open-time sweep is what frees `MAX_ISSUED_QUOTES` capacity held
/// by pre-upgrade far-future rows WITHOUT waiting for the next quote's inline prune — a
/// capacity-bricked wallet un-bricks on its next launch. A dead quote can never be
/// executed (its `take` re-checks both the deadline at the consumer and the far-future
/// cutoff), so pruning stays bound hygiene, never money-load-bearing.
pub(crate) fn prune_expired(conn: &mut Connection, now_unix: i64) -> Result<usize, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let removed = tx
        .execute(
            "DELETE FROM issued_swap_quote WHERE expires_at_wall <= ?1 OR expires_at_wall > ?2",
            rusqlite::params![now_unix, far_future_cutoff(now_unix)],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(removed)
}

/// Count the stored quotes (live + any not-yet-pruned expired). Test/observability helper;
/// the cap enforcement in [`persist`] counts inside its own txn.
#[cfg(test)]
pub(crate) fn count(conn: &Connection) -> Result<usize, WalletError> {
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM issued_swap_quote", [], |r| r.get(0))
        .map_err(map_db_err)?;
    // `COUNT(*)` is non-negative; the clamp is defensive-only (a negative is unreachable).
    Ok(n.max(0) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MAX_ISSUED_QUOTES;
    use std::sync::{Arc, Barrier};
    use std::thread;

    /// The production in-flight cap — the SAME constant the -c-2 consumer passes (SSOT, not a
    /// test-local literal), so a tuning bump to `MAX_ISSUED_QUOTES` is reflected here.
    const CAP: usize = MAX_ISSUED_QUOTES;

    /// A fresh in-memory connection with the table applied — the store logic is orthogonal
    /// to SQLCipher (encryption is db.rs's concern), so unit tests run on a plain connection
    /// (the [`crate::refund_index`] / [`crate::intent_store`] pattern).
    fn mem() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&conn).expect("schema");
        conn
    }

    /// A file-backed connection (busy wait `CONTENTION_BUSY_WAIT_MS`), for the durability tests
    /// where a row must survive a connection drop (a simulated process kill).
    fn open_file(path: &std::path::Path) -> Connection {
        let conn = Connection::open(path).expect("open file db");
        conn.busy_timeout(std::time::Duration::from_millis(
            crate::test_support::CONTENTION_BUSY_WAIT_MS,
        ))
        .expect("busy_timeout");
        ensure_table(&conn).expect("schema");
        conn
    }

    fn out_quote(id: &str, addr: &str, amount: i64, deadline: i64) -> StoredQuote {
        StoredQuote {
            refund_address: None,
            refund_index: None,
            id: id.into(),
            deposit_address: Some(addr.into()),
            deposit_amount_zat: Some(amount),
            expires_at_wall: deadline,
            destination_address: None,
            destination_index: None,
            binding: None,
            // S8: the provider handle IS the deposit address on the 1Click rail, and
            // the OutOfZec terms mirror the send plan; the shown deadline is the wall
            // deadline itself (this layer knows no margin, and nothing here compares it)
            provider_ref: Some(addr.into()),
            term_deposit_address: Some(addr.into()),
            term_deposit_memo: None,
            term_amount_in: Some("1.0".into()),
            term_min_amount_out: Some("41.5".into()),
            term_zec_side_zat: Some(amount),
            term_refund_to: None,
            term_expires_at: Some(deadline),
        }
    }

    /// An IntoZec row (IZ-1): no deposit, a fresh engine-persisted DESTINATION + its
    /// single-use external index — the MIRROR of [`out_quote`].
    fn into_quote(id: &str, dest: &str, index: i64, deadline: i64) -> StoredQuote {
        StoredQuote {
            refund_address: None,
            refund_index: None,
            id: id.into(),
            deposit_address: None,
            deposit_amount_zat: None,
            expires_at_wall: deadline,
            destination_address: Some(dest.into()),
            destination_index: Some(index),
            binding: None,
            // S8: an IntoZec quote's provider handle is the provider's SOURCE-chain
            // deposit address; its terms carry the ZEC it will deliver
            provider_ref: Some("0xsourcechaindeposit".into()),
            term_deposit_address: Some("0xsourcechaindeposit".into()),
            term_deposit_memo: None,
            term_amount_in: Some("1.0".into()),
            term_min_amount_out: Some("0.024".into()),
            term_zec_side_zat: Some(2_400_000),
            term_refund_to: Some("user-refund".into()),
            term_expires_at: Some(deadline),
        }
    }

    #[test]
    fn persist_then_take_round_trips_the_deposit_exactly() {
        let mut conn = mem();
        let q = out_quote("q-1", "t1deadbeef", 100_000, 2_000_000_000);
        assert_eq!(
            persist(&mut conn, &q, CAP, 1_700_000_000).expect("persist"),
            PersistOutcome::Persisted
        );
        let got = take(&mut conn, "q-1", 0).expect("take").expect("present");
        assert_eq!(got, q, "the consumed quote must round-trip byte-for-byte");
        // taken = gone: a second take is a miss (the one-shot consume)
        assert!(take(&mut conn, "q-1", 0).expect("take2").is_none());
    }

    #[test]
    fn take_is_the_atomic_one_shot_no_double_consume() {
        // The single-flight property: only ONE take of a given id ever returns the row.
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-x", "t1addr", 50_000, 2_000_000_000),
            CAP,
            1_700_000_000,
        )
        .expect("persist");
        assert!(take(&mut conn, "q-x", 0).expect("first").is_some());
        assert!(
            take(&mut conn, "q-x", 0).expect("second").is_none(),
            "a quote can be consumed at most once — the double-deposit guard"
        );
    }

    #[test]
    fn intozec_quote_persists_with_no_deposit_and_round_trips_the_destination() {
        // IntoZec receives ZEC to our own address ⇒ no wallet-side deposit (both deposit fields
        // None), and it CARRIES a fresh engine-persisted destination + index (IZ-1 / §3.3b D1)
        // that must round-trip byte-for-byte so the post-crash scoped poll re-derives the
        // detection-set entry (§3.3b L1/L2).
        let mut conn = mem();
        let q = into_quote("q-in", "u1destination", 7, 2_000_000_000);
        assert_eq!(
            persist(&mut conn, &q, CAP, 1_700_000_000).expect("persist"),
            PersistOutcome::Persisted
        );
        let got = take(&mut conn, "q-in", 0).expect("take").expect("present");
        assert_eq!(
            got, q,
            "the IntoZec row round-trips the destination exactly"
        );
        assert_eq!(got.deposit_address, None);
        assert_eq!(got.deposit_amount_zat, None);
        assert_eq!(got.destination_address.as_deref(), Some("u1destination"));
        assert_eq!(got.destination_index, Some(7));
    }

    #[test]
    fn binding_round_trips_persist_take() {
        // FR-17 (#396): the quote-issue-minted spend binding must survive
        // persist → take BYTE-EQUAL — execute copies THIS value onto the deposit's
        // `queued_send_intent` row, so a lossy round-trip here would make the
        // sign-at-execute pull present a binding the host never reviewed. And the
        // pre-FR-17 shape (`None` — no binding was ever shown to the host) reads
        // back `None`, so a legacy quote's deposit signs honestly unbound.
        let mut conn = mem();
        let binding = SpendBinding::mint();
        let bound = StoredQuote {
            binding: Some(binding),
            ..out_quote("q-bound", "t1deadbeef", 100_000, 2_000_000_000)
        };
        persist(&mut conn, &bound, CAP, 1_700_000_000).expect("persist bound");
        let got = take(&mut conn, "q-bound", 0)
            .expect("take")
            .expect("present");
        assert_eq!(
            got.binding.as_ref().map(SpendBinding::as_bytes),
            Some(binding.as_bytes()),
            "the taken quote carries the persist-time binding byte-equal",
        );
        assert_eq!(got, bound, "the whole row (binding included) round-trips");

        // The None case — `out_quote` is binding-less by construction (the legacy shape).
        let unbound = out_quote("q-unbound", "t1deadbeef", 100_000, 2_000_000_000);
        persist(&mut conn, &unbound, CAP, 1_700_000_000).expect("persist unbound");
        assert_eq!(
            take(&mut conn, "q-unbound", 0)
                .expect("take")
                .expect("present")
                .binding,
            None,
            "a binding-less row takes back None (honest unbound sign)",
        );
    }

    #[test]
    fn take_rejects_a_tampered_half_set_or_negative_destination_typed() {
        // The destination pair's StoreCorrupt door (the deposit-pair analogue): a half-set
        // destination (address present, index NULL — only reachable via tamper) and a negative
        // index both fail closed, never a half-formed detection-set entry.
        let mut conn = mem();
        conn.execute(
            "INSERT INTO issued_swap_quote \
                 (id, deposit_address, deposit_amount_zat, expires_at_wall, \
                  destination_address, destination_index) \
             VALUES ('q-dhalf', NULL, NULL, 2000000000, 'u1dest', NULL)",
            [],
        )
        .expect("seed half-set destination");
        assert!(matches!(
            take(&mut conn, "q-dhalf", 0),
            Err(WalletError::StoreCorrupt)
        ));
        conn.execute(
            "INSERT INTO issued_swap_quote \
                 (id, deposit_address, deposit_amount_zat, expires_at_wall, \
                  destination_address, destination_index) \
             VALUES ('q-dneg', NULL, NULL, 2000000000, 'u1dest', -1)",
            [],
        )
        .expect("seed negative index");
        assert!(matches!(
            take(&mut conn, "q-dneg", 0),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn take_rejects_a_destination_index_above_u32_max_typed() {
        // The FFI-panic guard: a POSITIVE tampered index beyond the consumer's u32 domain (only
        // reachable via tamper — the allocator floors at 1 and caps at 2^31-1) passes the `< 0`
        // check but must STILL fail closed HERE, so the consumer's `u32::try_from` never has to
        // panic on a sealed-row reconstruction. `u32::MAX as i64 + 1 = 4_294_967_296`.
        let mut conn = mem();
        conn.execute(
            "INSERT INTO issued_swap_quote \
                 (id, deposit_address, deposit_amount_zat, expires_at_wall, \
                  destination_address, destination_index) \
             VALUES ('q-dbig', NULL, NULL, 2000000000, 'u1dest', 4294967296)",
            [],
        )
        .expect("seed an over-u32 index");
        assert!(
            matches!(take(&mut conn, "q-dbig", 0), Err(WalletError::StoreCorrupt)),
            "an index above u32::MAX is fail-closed at the store door, never panicked downstream",
        );
        // exactly at u32::MAX is still valid (the boundary — a real key never reaches it, but the
        // edge must not over-reject)
        conn.execute(
            "INSERT INTO issued_swap_quote \
                 (id, deposit_address, deposit_amount_zat, expires_at_wall, \
                  destination_address, destination_index) \
             VALUES ('q-dmax', NULL, NULL, 2000000000, 'u1dest', 4294967295)",
            [],
        )
        .expect("seed exactly u32::MAX");
        assert_eq!(
            take(&mut conn, "q-dmax", 0)
                .expect("take")
                .expect("present")
                .destination_index,
            Some(i64::from(u32::MAX)),
            "exactly u32::MAX round-trips (the boundary is inclusive)",
        );
    }

    #[test]
    fn persist_rejects_a_half_set_destination_argument_before_writing() {
        let mut conn = mem();
        let half = StoredQuote {
            destination_address: Some("u1dest".into()),
            destination_index: None, // address without index — caller bug
            ..into_quote("q-dbad", "u1dest", 1, 2_000_000_000)
        };
        assert!(matches!(
            persist(&mut conn, &half, CAP, 1_700_000_000),
            Err(WalletError::StoreCorrupt)
        ));
        assert_eq!(count(&conn).expect("count"), 0, "nothing was written");
    }

    #[test]
    fn ensure_table_adds_destination_columns_to_a_pre_existing_wallet() {
        // A wallet provisioned BEFORE IZ-1 has `issued_swap_quote` WITHOUT the destination columns.
        // `ensure_table` (re-run on every open via `db::migrate`) must ADD them idempotently and
        // preserve existing rows — the additive `intent_store` guard-column precedent. Without this,
        // a pre-IZ-1 wallet's next persist would hit "no such column" on first open.
        let mut conn = Connection::open_in_memory().expect("in-memory db");
        // the pre-IZ-1 4-column schema + a seeded OutOfZec row
        conn.execute_batch(
            "CREATE TABLE issued_swap_quote (
                 id TEXT PRIMARY KEY, deposit_address TEXT, deposit_amount_zat INTEGER,
                 expires_at_wall INTEGER NOT NULL
             );
             INSERT INTO issued_swap_quote (id, deposit_address, deposit_amount_zat, expires_at_wall)
             VALUES ('q-pre', 't1frozen', 100000, 2000000000);",
        )
        .expect("seed the pre-IZ-1 schema + an existing row");
        ensure_table(&conn).expect("ensure_table adds the destination columns");
        let cols = table_columns(&conn).expect("columns");
        assert!(
            cols.contains("destination_address") && cols.contains("destination_index"),
            "the destination columns are added to a pre-IZ-1 wallet on open",
        );
        // re-running is idempotent (no duplicate-column error)
        ensure_table(&conn).expect("re-ensure is idempotent");
        // the existing OutOfZec row SURVIVES the migration and reads back with a NULL destination
        let got = take(&mut conn, "q-pre", 0)
            .expect("take")
            .expect("the pre-IZ-1 row survives the additive migration");
        assert_eq!(got.deposit_address.as_deref(), Some("t1frozen"));
        assert_eq!(
            got.destination_address, None,
            "the migrated row has a NULL destination"
        );
        assert_eq!(got.destination_index, None);
    }

    #[test]
    fn a_duplicate_sdk_id_is_a_typed_invariant_error_never_first_wins_in_the_issued_quote_store() {
        // S8 `identity` row 8 (rewrites the first-wins row it replaces, same span): the key is
        // SDK-minted, so a second persist of the SAME id is an invariant violation — refused
        // TYPED, the first row untouched. `Persisted` for a replay was first-wins; `AtCapacity`
        // would be a lie about the cap. Reddening mutation: the `already_present`
        // short-circuit (the base commit).
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-frozen", "t1original", 100_000, 1_700_050_000),
            CAP,
            1_700_000_000,
        )
        .expect("first");
        // the SAME id again with other terms
        assert!(
            persist(
                &mut conn,
                &out_quote("q-frozen", "t1ATTACKER", 1, 1_700_050_000),
                CAP,
                1_700_000_000,
            )
            .is_err(),
            "a duplicate id is a typed error, never a silent first-wins"
        );
        let got = take(&mut conn, "q-frozen", 0)
            .expect("take")
            .expect("the first row survives");
        assert_eq!(got.deposit_address.as_deref(), Some("t1original"));
        assert_eq!(got.deposit_amount_zat, Some(100_000));
    }

    #[test]
    fn persist_refuses_a_new_quote_at_capacity_and_a_duplicate_id_is_the_invariant_error_not_the_cap()
     {
        // The cap half is the PINNED crash-loop backstop, unchanged. The re-persist half is
        // S8 `identity` row 8's (rewritten in its own span, like the first-wins row above —
        // the contract's rewrite list did not name this one): the id is SDK-minted, so an
        // EXISTING id persisted again is an invariant violation — a typed error, never
        // `AtCapacity` (the cap is not why it was refused, and the cap's remedy — wait,
        // consume — is the wrong next step) and never `Persisted` (the base commit's
        // first-wins no-op). Reddening mutation: the `already_present` short-circuit.
        let mut conn = mem();
        let small_cap = 2;
        for i in 0..small_cap {
            assert_eq!(
                persist(
                    &mut conn,
                    &out_quote(&format!("q-{i}"), "t1a", 1, 1_700_050_000),
                    small_cap,
                    1_700_000_000,
                )
                .expect("persist"),
                PersistOutcome::Persisted
            );
        }
        // a NEW id at capacity is refused — the crash-loop backstop
        assert_eq!(
            persist(
                &mut conn,
                &out_quote("q-overflow", "t1a", 1, 1_700_050_000),
                small_cap,
                1_700_000_000,
            )
            .expect("persist"),
            PersistOutcome::AtCapacity
        );
        // an EXISTING id again, at capacity: the invariant error — not the cap's answer
        assert!(
            persist(
                &mut conn,
                &out_quote("q-0", "t1a", 1, 1_700_050_000),
                small_cap,
                1_700_000_000,
            )
            .is_err(),
            "a duplicate SDK id is a typed error — never `AtCapacity`, never a silent first-wins"
        );
    }

    #[test]
    fn persist_prunes_expired_so_capacity_counts_only_live_quotes() {
        let mut conn = mem();
        let small_cap = 2;
        // two quotes that expire at t=1000
        persist(
            &mut conn,
            &out_quote("q-old-1", "t1a", 1, 1_000),
            small_cap,
            500,
        )
        .expect("persist");
        persist(
            &mut conn,
            &out_quote("q-old-2", "t1a", 1, 1_000),
            small_cap,
            500,
        )
        .expect("persist");
        assert_eq!(count(&conn).expect("count"), 2);
        // a new quote at now=2000 (past both deadlines) prunes the two expired, then inserts —
        // capacity is NOT exceeded because the expired ones no longer count
        assert_eq!(
            persist(
                &mut conn,
                &out_quote("q-fresh", "t1a", 1, 9_000),
                small_cap,
                2_000,
            )
            .expect("persist"),
            PersistOutcome::Persisted
        );
        assert_eq!(
            count(&conn).expect("count"),
            1,
            "the two expired quotes were pruned; only the fresh one remains"
        );
    }

    #[test]
    fn prune_expired_removes_only_lapsed_rows() {
        let mut conn = mem();
        persist(&mut conn, &out_quote("q-live", "t1a", 1, 9_000), CAP, 500).expect("p1");
        persist(&mut conn, &out_quote("q-dead", "t1a", 1, 1_000), CAP, 500).expect("p2");
        let removed = prune_expired(&mut conn, 2_000).expect("prune");
        assert_eq!(removed, 1, "exactly the lapsed quote is pruned");
        assert!(take(&mut conn, "q-live", 0).expect("take").is_some());
        assert!(take(&mut conn, "q-dead", 0).expect("take").is_none());
    }

    #[test]
    fn take_rejects_a_tampered_half_set_or_negative_deposit_typed() {
        let mut conn = mem();
        // a half-set row (address present, amount NULL) — only reachable via tamper
        conn.execute(
            "INSERT INTO issued_swap_quote (id, deposit_address, deposit_amount_zat, expires_at_wall) \
             VALUES ('q-half', 't1a', NULL, 2000000000)",
            [],
        )
        .expect("seed half-set");
        assert!(matches!(
            take(&mut conn, "q-half", 0),
            Err(WalletError::StoreCorrupt)
        ));
        // a negative amount — only reachable via tamper
        conn.execute(
            "INSERT INTO issued_swap_quote (id, deposit_address, deposit_amount_zat, expires_at_wall) \
             VALUES ('q-neg', 't1a', -1, 2000000000)",
            [],
        )
        .expect("seed negative");
        assert!(matches!(
            take(&mut conn, "q-neg", 0),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn persist_rejects_a_half_set_deposit_argument_before_writing() {
        let mut conn = mem();
        let half = StoredQuote {
            deposit_amount_zat: None, // address without amount — caller bug
            ..out_quote("q-bad", "t1a", 1, 2_000_000_000)
        };
        assert!(matches!(
            persist(&mut conn, &half, CAP, 1_700_000_000),
            Err(WalletError::StoreCorrupt)
        ));
        assert_eq!(count(&conn).expect("count"), 0, "nothing was written");
    }

    #[test]
    fn issued_quotes_persist_across_reopen_until_consumed() {
        // The crash-then-requote window's teeth: a quote issued before a kill is STILL there
        // after a reopen, so the post-crash execute consumes the SAME frozen deposit (one
        // deposit), instead of an empty registry forcing a re-quote (a second deposit).
        let file = tempfile::NamedTempFile::new().expect("temp db");
        let path = file.path().to_owned();
        {
            let mut conn = open_file(&path);
            persist(
                &mut conn,
                &out_quote("q-survive", "t1frozen", 100_000, 2_000_000_000),
                CAP,
                1_700_000_000,
            )
            .expect("persist");
        } // connection dropped — simulates a process kill between quote and execute
        {
            let mut conn = open_file(&path);
            let got = take(&mut conn, "q-survive", 0)
                .expect("take")
                .expect("the issued quote survived the kill");
            assert_eq!(got.deposit_address.as_deref(), Some("t1frozen"));
            // and it is one-shot even across the restart
            assert!(take(&mut conn, "q-survive", 0).expect("take2").is_none());
        }
    }

    #[test]
    fn ensure_table_is_idempotent_and_preserves_rows() {
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-keep", "t1a", 1, 2_000_000_000),
            CAP,
            1_700_000_000,
        )
        .expect("persist");
        ensure_table(&conn).expect("re-ensure"); // every open re-runs this
        assert!(
            take(&mut conn, "q-keep", 0).expect("take").is_some(),
            "re-running ensure_table dropped a row — a lost in-flight quote"
        );
    }

    // ── re-review (S77): real-world-edge — the no-double-deposit single-flight under real
    // CONCURRENCY, the cap under a race, and the prune/deadline `<=` boundary ──────────────

    #[test]
    fn concurrent_takes_of_one_quote_yield_exactly_one_some() {
        // THE no-double-deposit money invariant under real contention (the `refund_index`
        // `concurrent_reserves_never_collide` analogue — but the failure mode is WORSE: a
        // double-CHARGE, not a privacy link): N threads racing to consume the SAME issued quote
        // must yield EXACTLY ONE `Some` (the rest `None`), so one quote can never fund two
        // deposits. Provable HERE on a file connection (the atomic `IMMEDIATE` SELECT-then-DELETE
        // is the cross-connection single-flight authority), not a -c-2 concern.
        let file = tempfile::NamedTempFile::new().expect("temp db");
        let path = Arc::new(file.path().to_owned());
        {
            let mut conn = open_file(&path);
            persist(
                &mut conn,
                &out_quote("q-race", "t1frozen", 100_000, 2_000_000_000),
                CAP,
                1_700_000_000,
            )
            .expect("seed the one quote");
        }
        const N: usize = 12;
        let barrier = Arc::new(Barrier::new(N));
        let handles: Vec<_> = (0..N)
            .map(|_| {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let mut conn = open_file(&path);
                    barrier.wait();
                    // the long test busy wait serializes all N, even on a loaded machine
                    take(&mut conn, "q-race", 0).expect("take under contention")
                })
            })
            .collect();
        let results: Vec<Option<StoredQuote>> = handles
            .into_iter()
            .map(|h| h.join().expect("join"))
            .collect();
        let somes: Vec<&StoredQuote> = results.iter().filter_map(|r| r.as_ref()).collect();
        assert_eq!(
            somes.len(),
            1,
            "exactly ONE concurrent take consumed the quote — a second would be a double-deposit"
        );
        assert_eq!(
            somes[0].deposit_address.as_deref(),
            Some("t1frozen"),
            "the single winner carries the frozen deposit"
        );
        let conn = open_file(&path);
        assert_eq!(
            count(&conn).expect("count"),
            0,
            "the quote is gone after the race"
        );
    }

    #[test]
    fn concurrent_persist_of_distinct_ids_at_cap_boundary_never_exceeds_cap() {
        // The cap is a crash-loop backstop. Two threads each persisting a DISTINCT new id at
        // cap-1 must NOT both read `live = cap-1` and both insert (landing at cap+1): the
        // `IMMEDIATE` txn serializes the COUNT-then-INSERT, so exactly one is `Persisted` + one
        // `AtCapacity`, and the table never exceeds the cap.
        let small_cap = 4;
        let file = tempfile::NamedTempFile::new().expect("temp db");
        let path = Arc::new(file.path().to_owned());
        {
            let mut conn = open_file(&path);
            for i in 0..(small_cap - 1) {
                persist(
                    &mut conn,
                    &out_quote(&format!("seed-{i}"), "t1a", 1, 1_700_050_000),
                    small_cap,
                    1_700_000_000,
                )
                .expect("seed cap-1 live rows");
            }
        }
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|i| {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let mut conn = open_file(&path);
                    barrier.wait();
                    persist(
                        &mut conn,
                        &out_quote(&format!("race-{i}"), "t1a", 1, 1_700_050_000),
                        small_cap,
                        1_700_000_000,
                    )
                    .expect("persist under contention")
                })
            })
            .collect();
        let outcomes: Vec<PersistOutcome> = handles
            .into_iter()
            .map(|h| h.join().expect("join"))
            .collect();
        let persisted = outcomes
            .iter()
            .filter(|o| matches!(o, PersistOutcome::Persisted))
            .count();
        let at_cap = outcomes
            .iter()
            .filter(|o| matches!(o, PersistOutcome::AtCapacity))
            .count();
        assert_eq!(
            (persisted, at_cap),
            (1, 1),
            "the IMMEDIATE txn serializes the cap check — exactly one of the racers fits"
        );
        let conn = open_file(&path);
        assert_eq!(
            count(&conn).expect("count"),
            small_cap,
            "the table never exceeded the cap under the race"
        );
    }

    #[test]
    fn prune_boundary_a_quote_expiring_exactly_at_now_is_pruned() {
        // The pruning predicate is `expires_at_wall <= now_unix`; the "exactly at now" case is
        // the load-bearing edge — an off-by-one to `<` would keep an expired quote one tick too
        // long (and a `take` of it could feed a dead quote downstream).
        let mut conn = mem();
        persist(&mut conn, &out_quote("q-at-now", "t1a", 1, 1_000), CAP, 500).expect("p1");
        persist(&mut conn, &out_quote("q-after", "t1a", 1, 1_001), CAP, 500).expect("p2");
        assert_eq!(
            prune_expired(&mut conn, 1_000).expect("prune at exactly the deadline"),
            1,
            "a quote whose deadline EQUALS now is pruned (the `<=` edge)"
        );
        assert!(take(&mut conn, "q-at-now", 0).expect("take").is_none());
        assert!(
            take(&mut conn, "q-after", 0).expect("take").is_some(),
            "the now+1 quote survives the `<=` boundary"
        );
    }

    #[test]
    fn expires_at_wall_extreme_values_round_trip_and_prune_correctly() {
        // The column is a raw i64 with no domain bound; 0 and i64::MAX are reachable (the doc's
        // "an absurd deadline beyond i64::MAX saturates to 0 = already-expired"). i64::MAX must
        // round-trip with no truncation, and it survives a prune only on an IMPLAUSIBLE clock
        // (the floor-guarded far-future arm, W-swap-4-a-4) — on a plausible clock it is exactly
        // the hostile far-future shape the sweep retires. A 0 deadline is the bottom `<=` edge
        // (pruned at now=0).
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-eternal", "t1a", 1, i64::MAX),
            CAP,
            0,
        )
        .expect("persist max");
        assert_eq!(
            prune_expired(&mut conn, 500).expect("prune implausible"),
            0,
            "an i64::MAX deadline survives a prune on an implausible clock (floor-guarded arm)"
        );
        assert_eq!(
            take(&mut conn, "q-eternal", 0)
                .expect("take")
                .expect("present")
                .expires_at_wall,
            i64::MAX,
            "the extreme deadline round-trips with no overflow/truncation"
        );
        persist(
            &mut conn,
            &out_quote("q-eternal", "t1a", 1, i64::MAX),
            CAP,
            0,
        )
        .expect("re-persist max");
        assert_eq!(
            prune_expired(&mut conn, 1_700_000_000).expect("prune plausible"),
            1,
            "on a plausible clock an i64::MAX deadline IS the far-future hostile shape — swept"
        );
        persist(
            &mut conn,
            &out_quote("q-zero", "t1a", 1, 0),
            CAP,
            1_700_000_000,
        )
        .expect("persist zero");
        assert_eq!(
            prune_expired(&mut conn, 0).expect("prune at 0"),
            1,
            "a deadline of exactly 0 is pruned at now=0 (the bottom `<=` edge)"
        );
    }

    /// A plausible wall clock for the W-swap-4-a-4 far-future tests — just past the
    /// `CLOCK_PLAUSIBILITY_FLOOR_SECS` the cutoff self-disables under.
    const PLAUSIBLE_NOW: i64 = 1_750_000_000;

    #[test]
    fn prune_sweeps_a_far_future_row_on_a_plausible_clock() {
        // W-swap-4-a-4 (2): a pre-clamp row (~72 h echo, persisted un-clamped by an older
        // build — fabricated here by persisting on an implausible clock, which disables
        // the inline arm) is swept by the standalone prune on a plausible clock; a
        // legitimately-clamped live row survives the same sweep.
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-pre", "t1a", 1, PLAUSIBLE_NOW + 259_200), // ~72 h out
            CAP,
            0,
        )
        .expect("fabricate the pre-clamp row");
        persist(
            &mut conn,
            &out_quote("q-live", "t1a", 1, PLAUSIBLE_NOW + 3_600), // inside the 24 h ceiling
            CAP,
            0,
        )
        .expect("persist the live row");
        assert_eq!(
            prune_expired(&mut conn, PLAUSIBLE_NOW).expect("prune"),
            1,
            "exactly the far-future row is swept"
        );
        assert!(
            take(&mut conn, "q-live", PLAUSIBLE_NOW)
                .expect("take")
                .is_some(),
            "the clamped live row survived the sweep"
        );
    }

    #[test]
    fn persist_inline_prune_sweeps_far_future_rows_on_a_plausible_clock() {
        // The same law at the OTHER prune door: a new quote's inline prune (the path a
        // capacity-bricked wallet hits on its next quote attempt) frees the pre-clamp row
        // in the same IMMEDIATE txn that admits the new one.
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-pre", "t1a", 1, PLAUSIBLE_NOW + 259_200),
            CAP,
            0,
        )
        .expect("fabricate the pre-clamp row");
        persist(
            &mut conn,
            &out_quote("q-new", "t1a", 1, PLAUSIBLE_NOW + 3_600),
            CAP,
            PLAUSIBLE_NOW,
        )
        .expect("the new persist prunes inline and admits");
        assert_eq!(count(&conn).expect("count"), 1, "only the new row remains");
        assert!(
            take(&mut conn, "q-pre", PLAUSIBLE_NOW)
                .expect("take")
                .is_none(),
            "the pre-clamp row is gone"
        );
    }

    #[test]
    fn prune_keeps_a_far_future_row_on_an_implausible_clock() {
        // The floor guard's whole point: a broken/1970 device clock must never mass-sweep
        // live quotes — with `now` below the plausibility floor the far-future arm is off,
        // and the lapsed arm prunes nothing at now=0 either.
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-pre", "t1a", 1, PLAUSIBLE_NOW + 259_200),
            CAP,
            0,
        )
        .expect("persist");
        assert_eq!(
            prune_expired(&mut conn, 0).expect("prune at an implausible now"),
            0,
            "nothing is swept on a clock that cannot judge far-future"
        );
        assert_eq!(count(&conn).expect("count"), 1);
    }

    #[test]
    fn take_refuses_a_far_future_row_and_deletes_it() {
        // W-swap-4-a-4 (2), the read-side twin: a pre-clamp row can neither execute days
        // past the honest window nor survive its first touch — `take` deletes it and
        // reports a MISS (the consumer's "not issued" remedy: re-quote). A clamped live
        // row takes normally on the same clock.
        let mut conn = mem();
        persist(
            &mut conn,
            &out_quote("q-pre", "t1a", 1, PLAUSIBLE_NOW + 259_200),
            CAP,
            0,
        )
        .expect("fabricate the pre-clamp row");
        assert!(
            take(&mut conn, "q-pre", PLAUSIBLE_NOW)
                .expect("take")
                .is_none(),
            "refused as a miss, never handed to the execute path"
        );
        assert_eq!(
            count(&conn).expect("count"),
            0,
            "and deleted — the row does not survive its first touch"
        );
        persist(
            &mut conn,
            &out_quote("q-ok", "t1a", 1, PLAUSIBLE_NOW + 3_600),
            CAP,
            PLAUSIBLE_NOW,
        )
        .expect("persist the live control");
        assert!(
            take(&mut conn, "q-ok", PLAUSIBLE_NOW)
                .expect("take")
                .is_some(),
            "a legitimately-clamped row takes normally on the same clock"
        );
    }

    #[test]
    fn take_keeps_a_ceiling_exact_row_across_a_backward_clock_step() {
        // review fold (the 3-review converged finding): the live provider ignores
        // our requested window, so a live IntoZec row is ceiling-clamped to EXACTLY
        // `persist_now + 24 h`. A routine backward NTP step between quote and execute
        // must NOT read it as far-future — the skew allowance absorbs up to
        // `FAR_FUTURE_SKEW_ALLOWANCE_SECS` of regress; past it the sweep still wins
        // (documented residual, money-safe: re-quote).
        let mut conn = mem();
        let ceiling = i64::try_from(crate::constants::SWAP_DEADLINE_DEFAULT_SECS).expect("fits");
        let skew = i64::try_from(crate::constants::FAR_FUTURE_SKEW_ALLOWANCE_SECS).expect("fits");
        persist(
            &mut conn,
            &out_quote("q-ceil", "t1a", 1, PLAUSIBLE_NOW + ceiling),
            CAP,
            PLAUSIBLE_NOW,
        )
        .expect("persist at the exact ceiling");
        assert!(
            take(&mut conn, "q-ceil", PLAUSIBLE_NOW - skew)
                .expect("take")
                .is_some(),
            "a live ceiling-exact row survives a backward clock step within the allowance"
        );
        persist(
            &mut conn,
            &out_quote("q-ceil-2", "t1a", 1, PLAUSIBLE_NOW + ceiling),
            CAP,
            PLAUSIBLE_NOW,
        )
        .expect("persist the past-allowance control");
        assert!(
            take(&mut conn, "q-ceil-2", PLAUSIBLE_NOW - skew - 1)
                .expect("take")
                .is_none(),
            "one second past the allowance the far-future arm still wins"
        );
    }

    #[test]
    fn persist_refuses_at_the_named_in_flight_cap() {
        // Gate-7 named-constant boundary (the `intent_store` `QUEUED_SEND_INTENTS_MAX` analogue):
        // exactly `MAX_ISSUED_QUOTES` live quotes fit; the next NEW id is refused at the named cap.
        let mut conn = mem();
        for i in 0..MAX_ISSUED_QUOTES {
            assert_eq!(
                persist(
                    &mut conn,
                    &out_quote(&format!("q-{i}"), "t1a", 1, 1_700_050_000),
                    MAX_ISSUED_QUOTES,
                    1_700_000_000,
                )
                .expect("persist"),
                PersistOutcome::Persisted,
                "the first MAX_ISSUED_QUOTES quotes all fit"
            );
        }
        assert_eq!(
            persist(
                &mut conn,
                &out_quote("q-over", "t1a", 1, 1_700_050_000),
                MAX_ISSUED_QUOTES,
                1_700_000_000,
            )
            .expect("persist"),
            PersistOutcome::AtCapacity,
            "the (MAX_ISSUED_QUOTES + 1)th NEW quote is refused at the named cap"
        );
        assert_eq!(count(&conn).expect("count"), MAX_ISSUED_QUOTES);
    }
}
