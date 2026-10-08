//! The durable queued-send intent store (§3.2h inc-2d-3-b-i; §6.2 offline-first).
//!
//! `queue_send` persists the **intent** — the `PaymentRequest` the user wants to
//! send — NOT a signed transaction and NOT transaction state (the §1.7
//! "pending-tx DB removal" lesson: tx state lives chain-derived only). Proposing
//! and signing happen later, at broadcast time, so a queued send can never carry
//! a stale anchor or a burned expiry window (the §6.2 worst-case walk: compose on
//! a plane, land 6 h later, the chain moved ~290 blocks — a compose-time signature
//! would be long-expired; the re-proposed intent is fresh).
//!
//! **Encoding (audited-format-whole).** The intent is stored as its **ZIP-321
//! URI** via the SDK's own audited inverse pair (`payment_uri::{encode,parse}`):
//! the canonical, spec'd, stable on-disk form. Re-loading (inc-2d-3-b-ii)
//! re-validates through the SAME door `propose` uses, so a queued send can never
//! drift from a freshly-typed one. No custom serde of internal struct layout.
//!
//! **Storage.** These rows live in the ONE sealed/wiped/backup-excluded `wallet.db`
//! (durable money state belongs in the single sealed file — a second durable file
//! would replicate the seal/wipe/backup surface, a money-safety hazard). The
//! `zcash_client_sqlite` `WalletDb` exposes no connection accessor, so this module
//! is pure SQL over a SECOND SQLCipher-keyed `rusqlite::Connection` to the same
//! file (`Inner.aux_db`); the table is created idempotently in `db::migrate` (the
//! single chokepoint both provision and open route through). Functions here take a
//! bare `&Connection`/`&mut Connection` so they unit-test against a plain
//! connection with NO `Wallet` (the `send::propose_core` testing pattern).
//!
//! **Crash-safety (§6.3).** Each write is ONE `IMMEDIATE` transaction → crash-atomic
//! (a killed insert leaves a clean committed prefix; never a torn row).
//!
//! **Deadlock-freedom — the load-bearing invariant.** Two connections share this
//! file — the engine's `WalletDb` (whose writes are rusqlite-default `DEFERRED`)
//! and OUR aux connection — in VERIFIED WAL mode since W-swap-4-a-4 (this
//! paragraph was rollback-era; re-justified W-swap-4-a-5). EVERY aux write is
//! `BEGIN IMMEDIATE`: it claims the ONE WAL writer slot up front, honoring
//! `busy_timeout`, and never sits on a read snapshot it intends to upgrade. A
//! `DEFERRED` read-then-write would instead fail `SQLITE_BUSY_SNAPSHOT`
//! IMMEDIATELY (no busy-handler wait) whenever the engine committed after the
//! snapshot was taken — an instant transient in a money path. (In the rollback
//! era the SAME rule prevented the classic two-upgraders deadlock that
//! `busy_timeout` cannot break; under WAL it buys writer-slot fairness and
//! snapshot honesty instead — the rule stands, for a different reason.) **This
//! is why `enqueue`/`delete` MUST use `IMMEDIATE`, never `DEFERRED` — and the
//! inc-2d-3-b-ii resubmission consume (which reads intents AND writes engine
//! state under sync contention) MUST do the same.**
//!
//! **The double-send guard STORAGE (inc-2d-3-b-ii-A).** This module holds the DURABLE
//! half of the §6.3 guard: the lifecycle states (`Queued` → `Submitting` → `Sent`), the
//! nullable `claim` column (the proposal's selected-note set as chain-stable
//! [`NoteClaim`] triples, recorded BEFORE the tx is created) and `txid` column (the
//! created tx's id, recorded after), and the state-guarded transitions
//! ([`mark_submitting`]/`mark_sent`/[`reset_to_queued`]) — each ONE `IMMEDIATE` txn. The
//! guard's LOGIC half — extracting the claim from a proposal, the spend witness, and the
//! reconcile decision — lives in `crate::send` (it needs the upstream proposal/`WalletRead`
//! types); together they let an intent never mint two txs and a tx never orphan its intent,
//! WITHOUT a cross-table transaction (the upstream constraint that ruled the shared txn out;
//! see the spec §3.2h inc-2d-3-b-ii design-lock). The money-moving consume that drives these
//! transitions from the sync loop is 3-b-ii-B's concern.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::db::map_aux_err as map_db_err;
use crate::error::WalletError;
use crate::seed::SpendBinding;
use crate::state::QueuedSendId;

/// `state` value for a fresh, not-yet-attempted intent (offline-first to-do; no
/// `claim`, no `txid`). The resubmission pass (inc-2d-3-b-ii-B) re-proposes these.
const STATE_QUEUED: i64 = 0;
/// `state` value for an intent whose resubmission is IN FLIGHT: the proposal's
/// selected-note `claim` is recorded but the tx is not yet known-created (`txid`
/// still NULL). The crash-window state — recovery reconciles it via the spend
/// witness ([`crate::send::reconcile`]). (inc-2d-3-b-ii-A schema; driven at B.)
const STATE_SUBMITTING: i64 = 1;
/// `state` value for an intent whose tx the engine created + persisted (`claim`
/// AND `txid`/`txids` set). The resubmission pass re-broadcasts the group until the
/// FINAL tx mines, then deletes the row (the chain owns it). (inc-2d-3-b-ii-A
/// schema; driven at B.)
const STATE_SENT: i64 = 2;
/// `state` value for a TERMINAL stranded send (§3.2i-2 slice B / round-2 BLOCKER #1):
/// a multi-step (TEX) `Sent` row whose earlier tx mined (the shielded notes left
/// IRREVERSIBLY) but whose later tx then EXPIRED un-mined — the funds sit on a
/// wallet-controlled ephemeral transparent address, recoverable but NOT via this
/// outbox (re-propose is structurally impossible post-tx0; the expired tx can never
/// mine). The row STOPS being reconciled (excluded from [`list_in_flight`], so it can
/// never busy-loop a doomed re-broadcast) and is handed to the 2e-2b stranded-ephemeral
/// detect, which surfaces the amount. The `claim`/`txids` are KEPT for that surface /
/// audit. A single-step send can never reach this state (an expired single-step re-queues
/// once its expiry buries — `reset_to_queued`); only a partially-mined multi-step strands.
const STATE_STRANDED: i64 = 3;

/// The lifecycle state of an IN-FLIGHT intent (`Submitting` or `Sent`), decoded from the
/// raw `state` column at the [`list_in_flight`] boundary so the 3-b-ii-B reconcile DISPATCH
/// branches EXHAUSTIVELY (the compiler catches a missing arm) instead of comparing raw
/// `i64`s. `Queued` is intentionally absent — it is not an in-flight state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IntentState {
    /// Claim recorded, tx not yet known-created (`txid` NULL) — the crash window.
    Submitting,
    /// Tx created + persisted (`txid` set) — re-broadcast until it mines.
    Sent,
}

impl IntentState {
    /// Decode the stored `state` integer. Only the two in-flight values are valid here
    /// (`list_in_flight` filters to them); anything else is corruption, fail-closed.
    fn from_i64(v: i64) -> Result<Self, WalletError> {
        match v {
            STATE_SUBMITTING => Ok(Self::Submitting),
            STATE_SENT => Ok(Self::Sent),
            _ => Err(WalletError::StoreCorrupt),
        }
    }
}

/// Fixed on-disk width of ONE encoded [`NoteClaim`]: 32 (txid) + 1 (protocol tag) +
/// 4 (output_index, big-endian) = 37 bytes. The `claim` BLOB is exactly
/// `claims.len() * CLAIM_RECORD_LEN` bytes — any other length is corruption (§4.6
/// size-validate-before-use), rejected on decode, never silently truncated.
const CLAIM_RECORD_LEN: usize = 37;
/// Defensive cap on the number of notes one claim may encode/decode — a proposal
/// selects a bounded set of inputs (proof size limits it to well under this), so a
/// `claim` BLOB implying more notes than this is corruption, rejected before alloc.
/// Enforced symmetrically at the PRODUCER too (`crate::send::claim_from_proposal`), so a
/// claim that would fail to decode can never be written in the first place.
pub(crate) const MAX_CLAIM_NOTES: usize = 512;
/// Protocol tag for a Sapling note in an encoded [`NoteClaim`] (storage-stable — the
/// SDK's own byte, NOT the upstream `ShieldedProtocol` discriminant, which is not a
/// guaranteed stable wire value). `crate::send` maps `ShieldedProtocol` ⇆ these tags,
/// so the byte values live here (the storage-format owner) as the one SSOT.
pub(crate) const PROTO_SAPLING: u8 = 0;
/// Protocol tag for an Orchard note in an encoded [`NoteClaim`].
pub(crate) const PROTO_ORCHARD: u8 = 1;
/// Protocol tag for an Ironwood note (NU6.3) in an encoded [`NoteClaim`].
///
/// A DISTINCT byte, not a re-use of [`PROTO_ORCHARD`], even though `zcash_keys`'
/// own comment says Ironwood shares the Orchard receiver. The tag is one third of
/// the `(txid, pool, output_index)` key `InputSource::get_spendable_note` resolves,
/// and `output_index` is PER-POOL: a version 6 transaction may carry both an Orchard
/// and an Ironwood bundle, so index 0 exists twice. Filing an Ironwood note under
/// the Orchard tag can therefore resolve to a DIFFERENT, still-unspent note, the
/// double-send witness reads "our transaction was never created", and the recovery
/// path re-proposes a payment that is already on chain. New byte, new arm.
///
/// ONE-WAY on disk. A build predating this constant reads a claim carrying `2` as
/// `StoreCorrupt` — the stuck-with-money-in-flight state `decode_claims` describes.
/// That is a downgrade concern, not an upgrade one, and it is subsumed by the
/// `zcash_client_sqlite` 0.22.0 schema migration being irreversible anyway; noted
/// here because it is an INDEPENDENT mechanism, so a future migration story that
/// solves the schema half does not automatically solve this one.
pub(crate) const PROTO_IRONWOOD: u8 = 2;

/// Fixed on-disk width of ONE txid in the ordered `txids` BLOB (§3.2i-2 pt 3 — the
/// multi-txid persistence). A `Sent` intent records the txids of EVERY tx the engine
/// created for its proposal, in step order: a single-step send is exactly one, a TEX /
/// ZIP-320 two-step is two (tx0 unshields to the wallet's ephemeral address, tx1 forwards
/// to the exchange). The blob is `txids.len() * TXID_RECORD_LEN` bytes — any other length
/// is corruption (§4.6 size-validate-before-use), rejected on decode, never truncated.
const TXID_RECORD_LEN: usize = 32;
/// Defensive cap on how many txids one intent may record. Today the engine produces 1
/// (ordinary send) or 2 (TEX two-step); this is generous headroom that rejects an absurd
/// blob length BEFORE allocating (corruption-before-alloc, the `MAX_CLAIM_NOTES` precedent)
/// while never refusing a real proposal's tx group. Enforced symmetrically at the PRODUCER
/// (`crate::send` caps the step count it records), so a blob that would fail to decode can
/// never be written.
pub(crate) const MAX_INTENT_TXIDS: usize = 8;

/// Create the queued-send intent table if it is absent. Idempotent
/// (`CREATE TABLE IF NOT EXISTS`), so `db::migrate` calls it on EVERY provision and
/// open — an existing wallet provisioned before this table existed gains it on the
/// next open, with no `WALLET_SCHEMA_VERSION` bump. `state` is indexed because
/// inc-2d-3-b-ii's resubmission pass lists by it every sync pass.
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS queued_send_intent (
             id         INTEGER PRIMARY KEY,
             uri        TEXT    NOT NULL,
             state      INTEGER NOT NULL DEFAULT 0,
             created_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_queued_send_intent_state
             ON queued_send_intent (state);",
    )
    .map_err(map_db_err)?;
    // inc-2d-3-b-ii-A: the double-send-guard columns, added ADDITIVELY so a wallet
    // provisioned under 3-b-i gains them on its next open with NO `WALLET_SCHEMA_VERSION`
    // bump and NO table rebuild (rust-patterns: never rebuild a data-bearing table on the
    // boot path). Both are nullable with no default ⇒ each `ALTER ADD COLUMN` is O(1)
    // metadata. Driven by the resubmission machinery at 3-b-ii-B; the schema lands now so
    // the storage shape is frozen before the money-moving half wires it.
    ensure_guard_columns(conn)
}

/// Idempotently add the nullable `claim`/`txid`/`deposit_deadline` columns. `ALTER TABLE
/// ADD COLUMN` errors if the column already exists, so we consult `PRAGMA table_info` first
/// and add only what is absent — making `ensure_table` safe to re-run on EVERY provision/open
/// (the `db::migrate` chokepoint), across a 3-b-i, a 3-b-ii, an inc-2d-swap, or a fresh
/// wallet alike.
///
/// The read-then-conditional-ALTER is NOT a TOCTOU hazard: in production `migrate` runs
/// only under the exclusive single-opener `WalletLock` flock (see `lifecycle.rs`), so no
/// second connection can race the ALTER; tests each use a fresh in-memory DB. No two
/// connections ever run this concurrently.
fn ensure_guard_columns(conn: &Connection) -> Result<(), WalletError> {
    let existing = table_columns(conn)?;
    // `claim`: the serialized [`NoteClaim`] set the proposal selected (set at
    // `mark_submitting`, before the engine creates the tx). `txid`: the created tx's
    // 32-byte id (set at `mark_sent`, after create). Both NULL until a resubmission
    // attempt populates them.
    if !existing.contains("claim") {
        conn.execute_batch("ALTER TABLE queued_send_intent ADD COLUMN claim BLOB")
            .map_err(map_db_err)?;
    }
    if !existing.contains("txid") {
        conn.execute_batch("ALTER TABLE queued_send_intent ADD COLUMN txid BLOB")
            .map_err(map_db_err)?;
    }
    // `deposit_deadline` (inc-2d-swap-a): the swap quote's wall-clock `expires_at` (unix
    // seconds) for a SWAP-DEPOSIT intent; `NULL` for an ordinary `queue_send`. Tagging the
    // deposit lets the resubmission machinery EXCLUDE a fresh re-propose past the quote
    // deadline (§4.4 "would feed a dead quote"). Additive + nullable, like `claim`/`txid` —
    // a 3-b-ii wallet gains it on next open with NO `WALLET_SCHEMA_VERSION` bump.
    if !existing.contains("deposit_deadline") {
        conn.execute_batch("ALTER TABLE queued_send_intent ADD COLUMN deposit_deadline INTEGER")
            .map_err(map_db_err)?;
    }
    // `txids` (§3.2i-2 slice B pt 3): the ORDERED `[txid(32)]×N` BLOB the engine created for
    // this intent's proposal — one tx for an ordinary send, two for a TEX two-step. This
    // SUPERSEDES the single `txid` column above: from slice B all new writes record `txids`
    // (via `mark_sent_multi`), and reconcile reads ONLY `txids` (never the legacy `txid`) so a
    // multi-step row can't `AwaitWitness` forever on a NULL legacy field. The legacy `txid` is
    // retained read-only as a fallback for a row already in-flight across the upgrade
    // (`list_in_flight` decodes such a row to a one-element `txids`). Additive + nullable, like
    // `claim`/`txid` — a 3-b-ii wallet gains it on next open with NO `WALLET_SCHEMA_VERSION` bump.
    //
    // ADDED LAST, AFTER `deposit_deadline`, ON PURPOSE: `copy_aux_tables` (the keyed-rebuild
    // path) maps columns by ORDINAL (`INSERT … SELECT *` guarded by an ordinal/name/type
    // signature match). Every provenance — fresh, 3-b-i, 3-b-ii, swap-a — must converge to the
    // SAME column ORDER, so a new column is always appended at the end of the migration so it
    // lands at the same ordinal whether the row already had `deposit_deadline` or not. (Were
    // `txids` inserted before `deposit_deadline`, an old swap-a DB — which already has
    // `deposit_deadline` and appends `txids` after it — would diverge in ordinal from a fresh
    // DB and a rebuild would fail-closed `StoreCorrupt`.)
    if !existing.contains("txids") {
        conn.execute_batch("ALTER TABLE queued_send_intent ADD COLUMN txids BLOB")
            .map_err(map_db_err)?;
    }
    // `repropose_attempts` (§3.2i-2 2e-2b-vi, #315 slice 1): how many EPHEMERAL-RESERVING create
    // attempts this intent has made — incremented atomically inside [`mark_submitting_counting`]
    // (the drain funnel EVERY ephemeral-reserving create passes through, whatever requeue path
    // brought the row back) and read by the drain's cap gate
    // (`repropose_attempts >= MAX_TEX_REPROPOSE_ATTEMPTS` ⇒ park, stop leaking). Only a ZIP-320
    // two-step ever increments it (a single-step send reserves nothing), so
    // `repropose_attempts > 0 ⟺ TEX intent`. `NOT NULL DEFAULT 0` (a counter, never nullable; an
    // `ALTER ADD COLUMN` with a CONSTANT default is still O(1) metadata). Appended LAST, after
    // `txids`, for the same ordinal-convergence reason documented above (`copy_aux_tables` maps by
    // ordinal, and its `assert_columns_match` compares only `(cid, name, type)` — the NOT
    // NULL/DEFAULT bits do not participate — so every provenance converges).
    if !existing.contains("repropose_attempts") {
        conn.execute_batch(
            "ALTER TABLE queued_send_intent \
             ADD COLUMN repropose_attempts INTEGER NOT NULL DEFAULT 0",
        )
        .map_err(map_db_err)?;
    }
    // `binding` (FR-17, #396): the 32-byte spend-binding nonce minted at enqueue (or
    // copied from the issued swap quote for a deposit), presented to the host seed
    // port on EVERY sign attempt for this row — so a host re-stage recorded for row X
    // can never be consumed signing row Y. Nullable + additive: a pre-FR-17 row reads
    // NULL ⇒ the pull presents unbound (honest — no binding was ever shown to the
    // host for it). Appended LAST, after `repropose_attempts`, for the SAME
    // ordinal-convergence reason documented on `txids` above.
    if !existing.contains("binding") {
        conn.execute_batch("ALTER TABLE queued_send_intent ADD COLUMN binding BLOB")
            .map_err(map_db_err)?;
    }
    Ok(())
}

/// The set of column names currently on `queued_send_intent` (via `PRAGMA table_info`,
/// whose row column 1 is the name). Surfaces any query error (rust-patterns: never
/// swallow a DB error on a schema read).
fn table_columns(conn: &Connection) -> Result<std::collections::HashSet<String>, WalletError> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(queued_send_intent)")
        .map_err(map_db_err)?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(map_db_err)?;
    names
        .collect::<Result<std::collections::HashSet<_>, _>>()
        .map_err(map_db_err)
}

/// One persisted queued-send intent (what [`list_queued`]/[`load`] return). Carries
/// the opaque id, the ZIP-321 URI to re-propose from, and the queue timestamp — NOT
/// a decoded recipient/amount (decoding is the resubmission path's job, through the
/// audited `parse_payment_uri`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct QueuedIntent {
    pub(crate) id: QueuedSendId,
    pub(crate) uri: String,
    /// Unix seconds at enqueue — injected by the caller (no wall-clock in storage),
    /// for observability / future age display, NOT for ordering (id is the FIFO key).
    pub(crate) created_at: i64,
    /// The swap quote's wall-clock `expires_at` (unix seconds) for a SWAP-DEPOSIT intent;
    /// `None` for an ordinary send (inc-2d-swap-a). The resubmission core excludes a fresh
    /// re-propose past this (§4.4 — never feed a dead quote).
    pub(crate) deposit_deadline: Option<i64>,
    /// How many EPHEMERAL-RESERVING create attempts this intent has made (#315 slice 1) —
    /// incremented by [`mark_submitting_counting`] (the drain, for a ZIP-320 two-step only), read
    /// by the drain's cap gate (`>= MAX_TEX_REPROPOSE_ATTEMPTS` ⇒ `Prepared::CappedParked`) and by
    /// the parked classifier (`paused`). `0` for every single-step intent, always.
    pub(crate) repropose_attempts: i64,
    /// FR-17: the row's spend-binding nonce, presented on every sign pull for this
    /// intent. `None` for a legacy pre-FR-17 row (or a wrong-length blob —
    /// validate-never-truncate ⇒ treated as absent, the fail-closed direction).
    pub(crate) binding: Option<SpendBinding>,
}

/// One selected input note of a proposal, as the CHAIN-STABLE triple — the §6.3
/// double-send-guard CLAIM (inc-2d-3-b-ii). `(txid, protocol, output_index)` survives a
/// rescan/rebuild (it names a position on the chain), unlike the engine's session-local
/// `ReceivedNoteId` rowid. The resubmission path records the claim BEFORE creating the tx
/// and, on crash-recovery, asks the engine whether these exact notes are now spent
/// (`crate::send::note_spendable`) — spent ⟺ the tx was created (don't re-propose).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct NoteClaim {
    /// The id of the transaction that PRODUCED this note (its on-chain receiving tx) —
    /// NOT the tx we are about to create. Pairs with `output_index` + `protocol` to
    /// index the note via `InputSource::get_spendable_note`.
    pub(crate) txid: [u8; 32],
    /// Shielded pool tag: [`PROTO_SAPLING`], [`PROTO_ORCHARD`] or [`PROTO_IRONWOOD`]
    /// (the SDK's own stable byte; `crate::send` maps it ⇆ the upstream
    /// `ShieldedPool`). Only the three
    /// shielded tags ever occur — NOT because transparent-inputs are off (they are ON,
    /// ADR-0528/Recv-2b), but because the double-send CLAIM is extracted over the
    /// proposal's SHIELDED note inputs only (`claim_from_proposal`): a TEX / ZIP-320
    /// two-step spends its shielded notes in step0 (claimed here) and its ephemeral
    /// transparent output in step1 via a `prior_step_inputs` StepOutput — NOT a claimed
    /// `transparent_input` — so no transparent tag is ever recorded (pt 7 site 3).
    pub(crate) protocol: u8,
    /// The note's output index within its producing transaction's shielded bundle.
    pub(crate) output_index: u32,
}

/// Encode a claim set to its fixed-width on-disk BLOB (`CLAIM_RECORD_LEN` bytes per
/// note: 32 txid ‖ 1 protocol ‖ 4 output_index big-endian). Deterministic, no serde,
/// preserves order. An empty set encodes to an empty BLOB (a claim must be non-empty in
/// practice — a real proposal always selects ≥1 note — but the codec stays total).
pub(crate) fn encode_claims(claims: &[NoteClaim]) -> Vec<u8> {
    let mut out = Vec::with_capacity(claims.len() * CLAIM_RECORD_LEN);
    for c in claims {
        out.extend_from_slice(&c.txid);
        out.push(c.protocol);
        out.extend_from_slice(&c.output_index.to_be_bytes());
    }
    out
}

/// Decode a claim BLOB, VALIDATING before trusting (§4.6 — even our own at-rest bytes are
/// validated, never defaulted: a torn/garbled `claim` is corruption, propagated as
/// `StoreCorrupt`, never a silently-wrong note set that could mis-witness a spend). Rejects
/// a length that is not a whole number of records, an absurd note count (> `MAX_CLAIM_NOTES`),
/// or an unknown protocol tag. The codec stays TOTAL on length: an empty blob decodes to an
/// empty `Vec` — the "an in-flight row carries ≥1 claim" invariant is enforced by the
/// producer ([`crate::send::claim_from_proposal`]) + the in-flight reader ([`list_in_flight`]),
/// NOT here, so no caller should rely on `decode_claims` to reject empty.
pub(crate) fn decode_claims(blob: &[u8]) -> Result<Vec<NoteClaim>, WalletError> {
    if !blob.len().is_multiple_of(CLAIM_RECORD_LEN) {
        return Err(WalletError::StoreCorrupt);
    }
    let count = blob.len() / CLAIM_RECORD_LEN;
    if count > MAX_CLAIM_NOTES {
        return Err(WalletError::StoreCorrupt);
    }
    let mut claims = Vec::with_capacity(count);
    for chunk in blob.as_chunks::<CLAIM_RECORD_LEN>().0 {
        let mut txid = [0u8; 32];
        txid.copy_from_slice(&chunk[..32]);
        let protocol = chunk[32];
        // Must admit every tag the PRODUCER can write, or a claim writes fine and
        // then fails every crash-recovery read as `StoreCorrupt` — the intent is
        // stuck in flight, the recovery never runs, and the failure appears at the
        // worst possible moment (after a crash, on a wallet with money in flight).
        if protocol != PROTO_SAPLING && protocol != PROTO_ORCHARD && protocol != PROTO_IRONWOOD {
            return Err(WalletError::StoreCorrupt);
        }
        let output_index = u32::from_be_bytes([chunk[33], chunk[34], chunk[35], chunk[36]]);
        claims.push(NoteClaim {
            txid,
            protocol,
            output_index,
        });
    }
    Ok(claims)
}

/// Encode an ordered txid list to its fixed-width on-disk BLOB (`TXID_RECORD_LEN` bytes per
/// txid, concatenated in step order). Deterministic, no serde, preserves order — so the
/// re-broadcast path replays the engine's tx group tx0→tx1 in the same order it built them
/// (§3.2i-2 pt 4). The on-disk byte order matches whatever the caller passes; the caller
/// records the engine's INTERNAL-order txid bytes (the form `WalletRead::get_transaction`
/// keys on), so the round trip never display↔internal-flips.
pub(crate) fn encode_txids(txids: &[[u8; 32]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(txids.len() * TXID_RECORD_LEN);
    for t in txids {
        out.extend_from_slice(t);
    }
    out
}

/// Decode a `txids` BLOB, VALIDATING before trusting (§4.6 — even our own at-rest bytes are
/// validated, never defaulted: a torn/garbled blob is corruption, never a silently-wrong
/// txid set that could mis-broadcast or mis-key a delete). Rejects a length that is not a
/// whole number of 32-byte records, or an absurd count (> [`MAX_INTENT_TXIDS`], rejected
/// BEFORE alloc). An empty blob decodes to an empty `Vec` (the codec stays TOTAL on length —
/// the "a `Sent` row carries ≥1 txid" invariant is enforced at [`list_in_flight`], not here).
pub(crate) fn decode_txids(blob: &[u8]) -> Result<Vec<[u8; 32]>, WalletError> {
    if !blob.len().is_multiple_of(TXID_RECORD_LEN) {
        return Err(WalletError::StoreCorrupt);
    }
    let count = blob.len() / TXID_RECORD_LEN;
    if count > MAX_INTENT_TXIDS {
        return Err(WalletError::StoreCorrupt);
    }
    let mut txids = Vec::with_capacity(count);
    for chunk in blob.as_chunks::<TXID_RECORD_LEN>().0 {
        let mut txid = [0u8; 32];
        txid.copy_from_slice(chunk);
        txids.push(txid);
    }
    Ok(txids)
}

/// One in-flight intent (state `Submitting` or `Sent`) the resubmission machinery
/// (inc-2d-3-b-ii-B) reconciles: its decoded `claims` drive the spend witness, and `txids`
/// (the ordered created-tx group, present once the engine created the tx(s)) drive
/// re-broadcast. `state` distinguishes the create-window (`Submitting`, `txids` EMPTY) from a
/// created send (`Sent`, `txids` non-empty in step order).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct InFlightIntent {
    pub(crate) id: QueuedSendId,
    pub(crate) uri: String,
    pub(crate) claims: Vec<NoteClaim>,
    /// The ordered created-tx group (§3.2i-2 pt 3). EMPTY ⟺ `Submitting` (no tx yet);
    /// non-empty ⟺ `Sent` (one txid for an ordinary send, the tx0→tx1 pair for a TEX
    /// two-step). Decoded from the `txids` BLOB, or a legacy single `txid` column for a row
    /// in-flight across the slice-B upgrade. The `state`/`txids`-emptiness invariant is
    /// enforced at the [`list_in_flight`] boundary (fail-closed `StoreCorrupt` otherwise).
    pub(crate) txids: Vec<[u8; 32]>,
    /// `Submitting` (claim recorded, no tx yet) or `Sent` (tx created, `txids` set) — a typed
    /// enum so the B reconcile dispatch matches exhaustively.
    pub(crate) state: IntentState,
    /// The swap quote's wall-clock `expires_at` (unix seconds) for a SWAP-DEPOSIT intent;
    /// `None` for an ordinary send. `reconcile_inflight` deletes a lapsed never-created deposit
    /// (`ReProposeFresh`); a created one's broadcast is HELD and it requeues into the drain's
    /// delete once its expiry buries (inc-2d-swap-a / §4.4, S7 C1).
    pub(crate) deposit_deadline: Option<i64>,
}

/// Persist a send intent (its ZIP-321 `uri`) and return its opaque id. ONE
/// `IMMEDIATE` transaction (crash-atomic). `created_at` is supplied by the caller
/// (the system clock in production, a fixed value in tests). `deposit_deadline` is
/// `Some(expires_at)` for a SWAP-DEPOSIT intent (inc-2d-swap-a — the resubmission
/// core excludes a fresh re-propose past it) and `None` for an ordinary `queue_send`.
/// A deposit-tagged enqueue additionally passes the ONE-deposit-in-flight guard
/// (typed [`WalletError::SwapDepositInFlight`] — §4.4 W-swap-4-a-2, below).
/// The `uri` is already validated + encoded by the caller (`queue_send` lowers the
/// `PaymentRequest` through the SAME `to_transaction_request` door `propose` uses);
/// this layer does not re-validate (DRY — one validation chokepoint).
pub(crate) fn enqueue(
    conn: &mut Connection,
    uri: &str,
    created_at: i64,
    deposit_deadline: Option<i64>,
    binding: Option<SpendBinding>,
) -> Result<QueuedSendId, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    // THE ONE-DEPOSIT-IN-FLIGHT GUARD (§4.4 W-swap-4-a-2 — the cross-quote
    // double-deposit refusal). A deposit-tagged enqueue is REFUSED while ANY
    // deadline-tagged row exists, in ANY state: `Queued` (unsigned — a sign-at-
    // execute failure left it for the drain), `Submitting`/`Sent` (signed, notes
    // spent, draining), and defensively `Stranded` (structurally unreachable for a
    // single-step deposit; if it ever existed its money would still be committed).
    // Checked INSIDE the same `IMMEDIATE` transaction as the insert, so two racing
    // `swapExecute`s (each with its OWN quote id — the per-quote single-flight
    // cannot see the other) serialize here and exactly ONE deposit is ever
    // persisted; the loser gets the typed refusal, whose port mapping
    // (`SwapAlreadyInFlight`) tells the host to TRACK, not re-quote. Deadline-
    // bounded, never a wedge: a signed deposit dies at mined-buried delete or
    // expiry→requeue, and a Queued one at the drain's SEEDLESS purge
    // (`send::purge_lapsed_queued_deposits` — unconditional every pass; the
    // in-gate delete is unreachable at a host-custody/portless background drain),
    // so the guard self-clears with the quote lifetime at EVERY custody tier
    // (while resubmission passes run — the purge rides the drain).
    // Ordinary sends pass untouched in both directions — a
    // deposit never blocks a `queue_send` (this branch is deadline-tagged-only)
    // and queued sends never count against a deposit (the EXISTS is
    // deadline-tagged-only). GUARD BEFORE CAP (W-swap-4-a-3): when the outbox is
    // ALSO full, the deposit refusal must win — `QueuedSendsFull`'s "let the
    // queue drain, then retry" copy invites exactly the re-quote this guard
    // exists to refuse, so surfacing the cap first would invert the remedy.
    if deposit_deadline.is_some() {
        let inflight: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM queued_send_intent \
                 WHERE deposit_deadline IS NOT NULL)",
                [],
                |r| r.get(0),
            )
            .map_err(map_db_err)?;
        if inflight {
            return Err(WalletError::SwapDepositInFlight);
        }
    }
    // Defensive cap (§6.2 / inc-2d-3-b-ii-A), checked INSIDE the IMMEDIATE txn so the
    // count→insert is atomic — two concurrent enqueues can never both pass a full-table
    // check and overflow. Counts rows that consume FUTURE outbox work (`state != Stranded`:
    // Queued + Submitting + Sent — every row the resubmission pass still acts on), so a
    // flaky/censoring link that STRANDS sends cannot fill the cap with terminal rows and
    // wedge new sends (an availability regression, money-safe — §3.2i-2 cap-hygiene /
    // ADR-0535 Decision 4). A terminal `Stranded` row owes no further outbox work (it left
    // `list_in_flight`, its amount is surfaced from the engine balance, not this row), and
    // the 2e-2b reap clears it on tx0 burial; it costs a real on-chain tx0 + fees to create,
    // so exempting it is no cheap DoS. At-or-over the cap ⇒ typed `QueuedSendsFull` (honest
    // "let the queue drain"), never a silent drop of a real send.
    let live: i64 = tx
        .query_row(
            "SELECT count(*) FROM queued_send_intent WHERE state != ?1",
            [STATE_STRANDED],
            |r| r.get(0),
        )
        .map_err(map_db_err)?;
    if live >= crate::constants::QUEUED_SEND_INTENTS_MAX as i64 {
        return Err(WalletError::QueuedSendsFull);
    }
    tx.execute(
        "INSERT INTO queued_send_intent (uri, state, created_at, deposit_deadline, binding) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            uri,
            STATE_QUEUED,
            created_at,
            deposit_deadline,
            binding.map(|b| b.as_bytes().to_vec()),
        ],
    )
    .map_err(map_db_err)?;
    let id = tx.last_insert_rowid();
    tx.commit().map_err(map_db_err)?;
    Ok(QueuedSendId::new(id))
}

/// List every still-queued intent in FIFO order (`id` ascending — a monotonic
/// rowid). The resubmission pass (inc-2d-3-b-ii) drives this each sync pass; in this
/// slice it backs the crash-survival tests. Reads only `state = Queued` so
/// inc-2d-3-b-ii's `Submitting`/`Sent` rows are naturally excluded.
pub(crate) fn list_queued(conn: &Connection) -> Result<Vec<QueuedIntent>, WalletError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uri, created_at, deposit_deadline, repropose_attempts, binding \
             FROM queued_send_intent WHERE state = ?1 ORDER BY id ASC",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map([STATE_QUEUED], |r| {
            Ok(QueuedIntent {
                id: QueuedSendId::new(r.get::<_, i64>(0)?),
                uri: r.get::<_, String>(1)?,
                created_at: r.get::<_, i64>(2)?,
                deposit_deadline: r.get::<_, Option<i64>>(3)?,
                repropose_attempts: r.get::<_, i64>(4)?,
                binding: decode_binding(r.get::<_, Option<Vec<u8>>>(5)?),
            })
        })
        .map_err(map_db_err)?;
    // Surface a row error (rust-patterns: never `filter_map(Result::ok)` on a query).
    rows.collect::<Result<Vec<_>, _>>().map_err(map_db_err)
}

/// List every intent the PARKED SURFACE must show, FIFO (`id` ascending), paired with whether
/// the row is mid-signature (`state = Submitting`).
///
/// Wider than [`list_queued`] on purpose (#400 R2, the reliability HIGH). `mark_submitting`
/// commits its atomic claim BEFORE the ZK prove — the unbounded, memory-hungry step whose
/// archetypal mobile failure is an OOM kill — so a process death in that window leaves a durable
/// `Submitting` row that appeared on ZERO host surfaces: [`list_queued`] filters it out, the
/// in-flight reader wants `Sent` + multi-tx, and — until the engine's create commits — no
/// transaction exists for the activity list either. That is the #331 silent-hide shape (a
/// committed spend the user cannot see, so they re-enter it and both send — a DOUBLE PAY), and
/// since FR-23-b a user TAP reaches it: "Send now" claims the row and proves inside the bracket,
/// exactly while the user is watching.
///
/// The recovery itself already existed and is unchanged: [`list_in_flight`] includes `Submitting`,
/// so the §6.3 reconcile re-queues an unfinished claim on the next pass. This reader closes the
/// VISIBILITY hole in the meantime — which for a host with no sync passes is not a window but a
/// permanent state. A `Submitting` row is presented as PREPARING, never as failed or as sendable.
///
/// `Submitting` SPANS THE CREATE, so the row's other properties are NOT constant across it
/// (#401 R2c): the claim commits before `create_signed_core`, and the create commits the
/// transaction to the wallet DB before `mark_sent_multi` flips the row to `Sent`. After that
/// commit the SAME payment can render as a Pending ACTIVITY row while it is still listed here,
/// and its notes have already left the spendable balance — so a host surface must not assert
/// the earmark of a `sending` row, nor tell the user it is absent from activity.
pub(crate) fn list_parkable(conn: &Connection) -> Result<Vec<(QueuedIntent, bool)>, WalletError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uri, created_at, deposit_deadline, repropose_attempts, binding, state \
             FROM queued_send_intent WHERE state IN (?1, ?2) ORDER BY id ASC",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map([STATE_QUEUED, STATE_SUBMITTING], |r| {
            Ok((
                QueuedIntent {
                    id: QueuedSendId::new(r.get::<_, i64>(0)?),
                    uri: r.get::<_, String>(1)?,
                    created_at: r.get::<_, i64>(2)?,
                    deposit_deadline: r.get::<_, Option<i64>>(3)?,
                    repropose_attempts: r.get::<_, i64>(4)?,
                    binding: decode_binding(r.get::<_, Option<Vec<u8>>>(5)?),
                },
                r.get::<_, i64>(6)? == STATE_SUBMITTING,
            ))
        })
        .map_err(map_db_err)?;
    // Surface a row error (rust-patterns: never `filter_map(Result::ok)` on a query).
    rows.collect::<Result<Vec<_>, _>>().map_err(map_db_err)
}

/// Load one intent by id, or `None` if absent (rust-patterns: a missing row is
/// `Ok(None)`, never a silenced error). Used by `Wallet::send_deposit_now` to re-read the
/// just-enqueued deposit intent for the synchronous sign-at-execute drive (FR-23-a); absent ⇒
/// a racing background drain already claimed it. (The resubmission LOOP drives the queue via
/// [`list_queued`]/[`list_in_flight`], not by id.)
pub(crate) fn load(
    conn: &Connection,
    id: QueuedSendId,
) -> Result<Option<QueuedIntent>, WalletError> {
    conn.query_row(
        "SELECT id, uri, created_at, deposit_deadline, repropose_attempts, binding \
         FROM queued_send_intent WHERE id = ?1",
        [id.value()],
        |r| {
            Ok(QueuedIntent {
                id: QueuedSendId::new(r.get::<_, i64>(0)?),
                uri: r.get::<_, String>(1)?,
                created_at: r.get::<_, i64>(2)?,
                deposit_deadline: r.get::<_, Option<i64>>(3)?,
                repropose_attempts: r.get::<_, i64>(4)?,
                binding: decode_binding(r.get::<_, Option<Vec<u8>>>(5)?),
            })
        },
    )
    .optional()
    .map_err(map_db_err)
}

/// Load one PARKED intent by its full host-visible identity (FR-23-b / #361): the same row
/// `list_parked_sends` would surface for this `(id, created_at)` pair, or `None` if no such row
/// exists RIGHT NOW.
///
/// All four FR-23-b guards live in the WHERE clause, so the read cannot return a row the verb
/// would then have to reject — and, critically, the SAME query can be re-run under the aux lock
/// the claim is about to take, making the guard atomic with `mark_submitting` (crypto audit
/// H1: a guard evaluated on a released-lock snapshot is defeated by a delete + rowid-reusing
/// re-enqueue inside the window, which for this verb spans a HUMAN-latency host credential
/// prompt):
/// 1. `id` — the row the host named.
/// 2. `created_at` — the identity pin against SQLite rowid REUSE (`id INTEGER PRIMARY KEY` has no
///    `AUTOINCREMENT`), the same pin [`cancel_queued`] and [`reset_repropose_attempts`] carry.
/// 3. `state = Queued` — nothing signed yet. A `Submitting`/`Sent` row must NOT reach the sign: it
///    would burn the host's take-once credential and then report "still waiting" over a payment
///    that is actually IN FLIGHT (crypto audit M1).
/// 4. `deposit_deadline IS NULL` — deposit rows are excluded from the parked surface, so they are
///    excluded here, exactly as in the two sibling verbs.
pub(crate) fn load_parked(
    conn: &Connection,
    id: QueuedSendId,
    created_at: i64,
) -> Result<Option<QueuedIntent>, WalletError> {
    conn.query_row(
        "SELECT id, uri, created_at, deposit_deadline, repropose_attempts, binding \
         FROM queued_send_intent \
         WHERE id = ?1 AND created_at = ?2 AND state = ?3 AND deposit_deadline IS NULL",
        rusqlite::params![id.value(), created_at, STATE_QUEUED],
        |r| {
            Ok(QueuedIntent {
                id: QueuedSendId::new(r.get::<_, i64>(0)?),
                uri: r.get::<_, String>(1)?,
                created_at: r.get::<_, i64>(2)?,
                deposit_deadline: r.get::<_, Option<i64>>(3)?,
                repropose_attempts: r.get::<_, i64>(4)?,
                binding: decode_binding(r.get::<_, Option<Vec<u8>>>(5)?),
            })
        },
    )
    .optional()
    .map_err(map_db_err)
}

/// FR-17 blob → [`SpendBinding`]: `None` on NULL AND on any wrong-length blob
/// (validate-never-truncate — a corrupt binding presents as an unbound pull, which
/// a bound host fail-closes; it can never silently pad into a "valid" token).
fn decode_binding(blob: Option<Vec<u8>>) -> Option<SpendBinding> {
    blob.as_deref().and_then(SpendBinding::from_slice)
}

/// Remove an intent by id; returns whether a row was deleted (`false` ⇒ already
/// gone — idempotent, never an error). ONE `IMMEDIATE` transaction. Unguarded by state by
/// design (it removes a row in any state).
///
/// This is the `Sent` → done edge: the resubmission hook (3-b-ii-B) calls it once it
/// confirms the `Sent` intent's tx is MINED (the chain owns the send; no more resubmission
/// owed). **B OBLIGATION:** B must detect "mined" (e.g. `WalletRead::get_tx_height(txid)` is
/// `Some`) before deleting — a `Sent` row whose tx is merely re-broadcast-rejected must NOT
/// be deleted (the spend hasn't necessarily landed). Distinct from [`reset_to_queued`], the
/// money-PRESERVING exit when the notes freed (re-attempt the send); `delete` is the
/// terminal exit when the send SUCCEEDED.
pub(crate) fn delete(conn: &mut Connection, id: QueuedSendId) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute("DELETE FROM queued_send_intent WHERE id = ?1", [id.value()])
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// State-guarded DELETE for the §3.2i-2 2e-2b reap (operational-round HARDENING): removes the
/// row ONLY if it is still `Stranded` (`WHERE id = ?1 AND state = Stranded`). The reap snapshots
/// terminal-`Stranded` ids, then deletes them per-row a pass later; guarding on the state makes the
/// delete LOCAL-safe — `id INTEGER PRIMARY KEY` has no `AUTOINCREMENT`, so SQLite may REUSE a freed
/// rowid, and a snapshotted `Stranded` id that was reaped-and-reused by a fresh `enqueue` for a live
/// `Queued` send can NEVER be deleted here (it is no longer `Stranded`). Without this guard the
/// reap's money-safety would rest on the GLOBAL "only the reap deletes a `Stranded` row" invariant —
/// which the host-facing cancel affordance (landed 2e-2b-v-4a as the state-guarded sibling
/// [`cancel_queued`], which deletes ONLY `Queued`) would otherwise have silently broken, dropping a
/// not-yet-sent payment. Returns whether a row was deleted (`false` ⇒ already gone or no longer
/// `Stranded` — idempotent, never an error).
pub(crate) fn delete_stranded(
    conn: &mut Connection,
    id: QueuedSendId,
) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "DELETE FROM queued_send_intent WHERE id = ?1 AND state = ?2",
            rusqlite::params![id.value(), STATE_STRANDED],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// State+identity-guarded CANCEL of a PARKED queued send (§3.2i-2 2e-2b-v-4a — the user-facing cancel
/// affordance). Deletes the row IFF it is still the SAME `Queued`, non-deposit intent the host surfaced
/// via `list_parked_sends`, pinned by BOTH the `id` AND the immutable `created_at`. Returns whether a row
/// was deleted (`false` ⇒ already gone / began draining / its rowid was reused — idempotent, never an
/// error). The host-facing `false`-handling contract (re-read the list; never present `false` as
/// "cancelled" nor invite a re-send) lives on the public
/// [`Wallet::cancel_parked_send`](crate::Wallet::cancel_parked_send). ONE `IMMEDIATE` transaction
/// (crash-atomic).
///
/// **Money-safety — two load-bearing guards (the §3.2i-2 2e-2b-v-4a CONTRACT proofs):**
/// 1. `state = Queued` (the [`STATE_QUEUED`] guard): cancel touches ONLY a `Queued` row, which has NO
///    `claim`, NO `txid`/`txids`, NO tx0 — no note reserved, nothing signed, nothing on-chain — so removing
///    it is STRUCTURALLY incapable of stranding funds; it discards a not-yet-attempted intent. And it makes
///    cancel vs. the post-v-5 drain MUTUALLY EXCLUSIVE: [`mark_submitting`] (`Queued`→`Submitting`, the
///    first money-touching step) is ALSO `BEGIN IMMEDIATE` on this SAME aux connection, so for any one
///    intent EXACTLY ONE of {this delete, mark_submitting} wins — cancel can never delete a send whose tx
///    might exist (it loses that race to a `false` no-op). This is why the guard is on `Queued`, never an
///    unguarded [`delete`].
/// 2. `created_at = ?2` (the identity pin): `id INTEGER PRIMARY KEY` has NO `AUTOINCREMENT`, so SQLite may
///    REUSE a freed rowid (the exact hazard [`delete_stranded`]'s doc named). A reused rowid is a fresh
///    `enqueue` with a fresh `created_at`, so pinning the immutable enqueue timestamp the host read from
///    `ParkedSend` fails CLOSED rather than cancel a DIFFERENT send the user never saw.
///
/// `deposit_deadline IS NULL` mirrors `list_parked_sends`'s swap-deposit exclusion (a swap deposit is
/// owned by the swap status surface).
///
/// ⚠ SURFACE ALIGNMENT (updated at #331, RE-updated at #400 R2): the guard set is `Queued` +
/// non-deposit, which is a STRICT SUBSET of the parked surface again. #331 had made the two exactly
/// equal; #400 R2 widened the surface to `Queued ∪ Submitting` so a spend claimed-but-not-recorded
/// (an OOM kill during proving) stops being invisible — but cancel MUST NOT follow it there. A
/// `Submitting` row has been claimed, and past `create` its notes are already locally spent, so
/// deleting the intent would discard the user's record of a payment that may exist. The guard
/// therefore stays `Queued` and the PRESENTATION carries the alignment instead: a `sending` row
/// offers no cancel affordance at all (`ParkedSend::sending`), because a `false` return plus a
/// confirm dialog promising "nothing leaves your wallet" would be two contradictory statements
/// about the same money. Cancel deliberately does NOT re-parse the stored URI here, so the escape hatch can never
/// FAIL-CLOSED on a torn at-rest URI — a corrupt URI must not trap the user's funds in an un-cancellable
/// parked send (classify's own torn-URI arm errors LOUD instead). `state = Queued` (never the recipient
/// kind) is the money-safety.
/// EVERY row this can match is `Queued` ⇒ structurally fundless (no claim / no txid / no tx0). The
/// identity pin is second-granularity, so a freed-then-same-second-reused rowid is the one residual
/// collision — still money-safe (the reused row is `Queued`).
/// Sibling of [`delete_stranded`]: the same state-guarded IMMEDIATE delete, plus the identity pin.
pub(crate) fn cancel_queued(
    conn: &mut Connection,
    id: QueuedSendId,
    created_at: i64,
) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "DELETE FROM queued_send_intent \
             WHERE id = ?1 AND created_at = ?2 AND state = ?3 AND deposit_deadline IS NULL",
            rusqlite::params![id.value(), created_at, STATE_QUEUED],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// Transition `Queued` → `Submitting`, recording the proposal's selected-note `claims`
/// BEFORE the tx is created (§6.3 double-send guard, inc-2d-3-b-ii). ONE `IMMEDIATE`
/// transaction (crash-atomic — the claim is durable the instant this returns, so a kill
/// during the subsequent create is recoverable). State-guarded (`WHERE state = Queued`,
/// rust-patterns: a transition method guards its current state): returns `false` if the
/// intent is no longer `Queued` (already in flight, or gone) — a no-op the caller reads as
/// "another pass owns it", never a corruption. Wired by the resubmission hook at 3-b-ii-B.
///
/// For a SINGLE-STEP proposal only (reserves no ephemeral). An ephemeral-reserving (ZIP-320
/// two-step) create goes through [`mark_submitting_counting`], which additionally increments the
/// #315 `repropose_attempts` leak counter in the SAME atomic write.
pub(crate) fn mark_submitting(
    conn: &mut Connection,
    id: QueuedSendId,
    claims: &[NoteClaim],
) -> Result<bool, WalletError> {
    mark_submitting_inner(conn, id, claims, false)
}

/// [`mark_submitting`] for an EPHEMERAL-RESERVING (ZIP-320 two-step) create (#315 slice 1): the
/// same state-guarded `Queued` → `Submitting` claim write, PLUS `repropose_attempts += 1` — in
/// ONE `IMMEDIATE` UPDATE. The fold is load-bearing: the subsequent `create` reserves a fresh
/// ephemeral index the engine never un-reserves (not on tx0 expiry, not on a create fault), so
/// this counter is the ONLY record that a leak-risking attempt happened. A separate
/// transition-then-increment pair would have a kill window between the two writes → an
/// under-count → a persistently-faulting intent whose cap NEVER trips (the exact per-pass leak
/// the cap exists to stop). Counting here — the one funnel every ephemeral-reserving create
/// passes through, whichever requeue path brought the row back — also covers the fast
/// create-fault leak (`Err(_) => reset_to_queued; Retry`), which no requeue-site counter sees.
/// Only ever called for a two-step, so `repropose_attempts > 0 ⟺ TEX intent` stays invariant.
pub(crate) fn mark_submitting_counting(
    conn: &mut Connection,
    id: QueuedSendId,
    claims: &[NoteClaim],
) -> Result<bool, WalletError> {
    mark_submitting_inner(conn, id, claims, true)
}

/// The shared `Queued` → `Submitting` body — one SQL chokepoint so the counting and
/// non-counting doors can never drift in guard or claim handling.
fn mark_submitting_inner(
    conn: &mut Connection,
    id: QueuedSendId,
    claims: &[NoteClaim],
    count_ephemeral_attempt: bool,
) -> Result<bool, WalletError> {
    let blob = encode_claims(claims);
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let sql = if count_ephemeral_attempt {
        "UPDATE queued_send_intent \
         SET state = ?1, claim = ?2, repropose_attempts = repropose_attempts + 1 \
         WHERE id = ?3 AND state = ?4"
    } else {
        "UPDATE queued_send_intent SET state = ?1, claim = ?2 \
         WHERE id = ?3 AND state = ?4"
    };
    let n = tx
        .execute(
            sql,
            rusqlite::params![STATE_SUBMITTING, blob, id.value(), STATE_QUEUED],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// Transition `Submitting` → `Sent`, recording the ORDERED `txids` group the engine created
/// (§3.2i-2 pt 3 — the multi-txid record). ONE `IMMEDIATE` transaction. Called right AFTER
/// `create_proposed_transactions` persists ALL of the proposal's txs (it is all-or-nothing),
/// in step order: `[tx0]` for an ordinary send, `[tx0, tx1]` for a TEX two-step. From here the
/// resubmission pass re-broadcasts the group (skip-mined, ordered) until the FINAL tx mines,
/// then [`delete`]s the row. Writes the new `txids` column (NOT the legacy single `txid`), so
/// reconcile reads one source of truth. State-guarded (`WHERE state = Submitting`): `false` if
/// it is not in the submit window. A caller must pass a NON-EMPTY group (a `Sent` row carries
/// ≥1 txid — the producer guarantees it); an empty slice is a structural fault, fail-closed.
pub(crate) fn mark_sent_multi(
    conn: &mut Connection,
    id: QueuedSendId,
    txids: &[[u8; 32]],
) -> Result<bool, WalletError> {
    // A `Sent` row MUST carry ≥1 txid (else the witness/delete have nothing to key on). The
    // producer always passes the engine's non-empty tx group; an empty slice is a bug, never
    // a silently-Sent-with-no-tx row. Fail closed rather than persist an un-reconcilable Sent.
    if txids.is_empty() || txids.len() > MAX_INTENT_TXIDS {
        return Err(WalletError::StoreCorrupt);
    }
    let blob = encode_txids(txids);
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "UPDATE queued_send_intent SET state = ?1, txids = ?2 \
             WHERE id = ?3 AND state = ?4",
            rusqlite::params![STATE_SENT, blob, id.value(), STATE_SUBMITTING],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// Transition `Submitting` → `Sent`, recording a SINGLE created tx via a one-element group.
/// Thin sugar over [`mark_sent_multi`] for the unit tests that assert the single-txid case;
/// the PRODUCTION send paths (single-step AND TEX two-step) both call [`mark_sent_multi`]
/// directly through [`crate::send::drain_multi`] / `create_two_step_enrolled` with the whole
/// ordered group, so there is ONE write path regardless of step count. Test-only
/// (`#[cfg(test)]`, P3-7): production never calls it.
#[cfg(test)]
pub(crate) fn mark_sent(
    conn: &mut Connection,
    id: QueuedSendId,
    txid: &[u8; 32],
) -> Result<bool, WalletError> {
    mark_sent_multi(conn, id, std::slice::from_ref(txid))
}

/// Reset a capped (paused) queued send's `repropose_attempts` to 0 — the #315 slice-1 RETRY
/// affordance (`Wallet::retry_parked_send`). A capped intent parks honestly
/// (`Prepared::CappedParked`, surfaced `paused`) after `MAX_TEX_REPROPOSE_ATTEMPTS`
/// (crate::constants) ephemeral-reserving attempts; when the user believes conditions changed (the
/// link recovered, the exchange is reachable again) this re-arms the SAME intent for another
/// attempt round WITHOUT cancel+re-enqueue — a fresh intent would restart at 0 anyway but ALSO
/// re-leak up to the cap as a NEW row, so resuming in place is the leak-cheapest path the design
/// deliberately offers (the multi-intent cap-evasion note). Money-safe by construction: it touches
/// ONLY the counter of a still-`Queued`, non-deposit row (no claim, no txid, nothing signed).
///
/// Guards mirror [`cancel_queued`] (the same host-surfaced `(id, created_at)` pair) plus a
/// counter guard: `state = Queued` (a draining/in-flight/terminal row is never re-armed), the
/// `created_at` identity pin (a reused rowid — a DIFFERENT send the user never saw — must not
/// have its cap silently lifted), `deposit_deadline IS NULL` (a swap deposit has its own
/// lifecycle), and `repropose_attempts > 0` (an untouched row has no budget to reset — a stale
/// or misdirected retry against it is a true no-op `false`, never a silent mid-budget refill;
/// security review NIT-3). Returns whether a row was re-armed (`false` ⇒ gone / draining /
/// identity mismatch / nothing to reset — idempotent, never an error; the host re-reads the
/// parked list either way). ONE `IMMEDIATE` txn.
pub(crate) fn reset_repropose_attempts(
    conn: &mut Connection,
    id: QueuedSendId,
    created_at: i64,
) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "UPDATE queued_send_intent SET repropose_attempts = 0 \
             WHERE id = ?1 AND created_at = ?2 AND state = ?3 \
               AND deposit_deadline IS NULL AND repropose_attempts > 0",
            rusqlite::params![id.value(), created_at, STATE_QUEUED],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// [`reset_to_queued`] for the TYPED gap-limit-ceiling refusal (#315, red-team F1): release
/// the `Submitting` claim back to `Queued` AND REFUND the attempt counted at
/// [`mark_submitting_counting`] — in ONE `IMMEDIATE` UPDATE. The counter's contract is
/// "ephemeral-RESERVING attempts": the count is taken pessimistically BEFORE `create` (the engine
/// reserves before construction and never rolls back, so a fault must stay counted), but a
/// [`TexSendLimitReached`](crate::error::WalletError) refusal is the engine's own proof that this
/// attempt reserved NOTHING (`reserve_next_n_ephemeral_addresses` refused) — leaving it counted
/// would pause a healthy TEX behind a transiently-full window in ~3 poll passes (~1 min), long
/// before the ~40-block self-heal, the exact transient→persistent over-correction the review
/// forbade. Guarded to `Submitting` ONLY (the just-refused create window — the sole caller is the
/// drain's typed-ceiling arm; a `Sent` row was never refused). `MAX(…, 0)` floors a defensive
/// refund-without-count at zero. Kill window: a death between the refusal and this write leaves
/// the attempt counted (the witness requeues via the plain non-refunding [`reset_to_queued`]) —
/// an OVER-count, conservative and money-safe, never an under-count.
pub(crate) fn reset_to_queued_refunding_attempt(
    conn: &mut Connection,
    id: QueuedSendId,
) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "UPDATE queued_send_intent \
             SET state = ?1, claim = NULL, txid = NULL, txids = NULL, \
                 repropose_attempts = MAX(repropose_attempts - 1, 0) \
             WHERE id = ?2 AND state = ?3",
            rusqlite::params![STATE_QUEUED, id.value(), STATE_SUBMITTING],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// Transition an IN-FLIGHT intent (`Submitting` OR `Sent`) → `Queued`, CLEARING the stale
/// `claim` AND the recorded tx group (BOTH the new `txids` and the legacy `txid`) so the next
/// pass re-proposes FRESH (it may select different notes, and any old txids are dead). Two
/// callers: a `Submitting` row whose claimed notes read spendable (the tx was never created),
/// and — the ONLY exit that re-sends a `Sent` row — the group-side all-expired terminal (v-5c
/// finding #1, `rebroadcast_group`): a `Sent` group with NO tx mined and EVERY tx past an
/// expiry BURIED beyond `REORG_MAX_BLOCKS` is dead whole, whatever the note witness reads
/// (S7 C1: the engine frees the notes at BARE expiry, while a reorg could still revive the
/// signed group), so the user's intended payment did NOT land and must be re-attempted, NOT
/// dropped. This is the
/// money-PRESERVING exit from `Sent` — without it a reorg-orphaned send would have only
/// `delete` (losing the queued payment). ONE `IMMEDIATE` transaction. State-guarded to the two
/// in-flight states (a `Queued` row is already there; a no-op `false`). Wired at 3-b-ii-B.
pub(crate) fn reset_to_queued(
    conn: &mut Connection,
    id: QueuedSendId,
) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "UPDATE queued_send_intent SET state = ?1, claim = NULL, txid = NULL, txids = NULL \
             WHERE id = ?2 AND state IN (?3, ?4)",
            rusqlite::params![STATE_QUEUED, id.value(), STATE_SUBMITTING, STATE_SENT],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// Transition a `Sent` intent → the TERMINAL `Stranded` disposition (§3.2i-2 slice B /
/// round-2 BLOCKER #1). The reconcile action when a multi-step (TEX) send partially mined —
/// an earlier tx mined (the shielded notes left irreversibly) but a later tx EXPIRED un-mined
/// — so the funds sit on a wallet-controlled ephemeral address that re-broadcast can never
/// recover (the expired tx can never enter the mempool; re-propose is structurally impossible
/// post-tx0). Moving to `Stranded` STOPS the doomed re-broadcast (the row leaves
/// [`list_in_flight`]) and hands the ephemeral off to the 2e-2b detect; the `claim`/`txids` are
/// KEPT so that surface / an audit can read the group. ONE `IMMEDIATE` transaction.
/// State-guarded `WHERE state = Sent` (only a created+sent send can strand; a never-created
/// `Submitting` cannot — its notes are unspent and it re-queues): `false` if not `Sent`.
pub(crate) fn mark_stranded(conn: &mut Connection, id: QueuedSendId) -> Result<bool, WalletError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_db_err)?;
    let n = tx
        .execute(
            "UPDATE queued_send_intent SET state = ?1 WHERE id = ?2 AND state = ?3",
            rusqlite::params![STATE_STRANDED, id.value(), STATE_SENT],
        )
        .map_err(map_db_err)?;
    tx.commit().map_err(map_db_err)?;
    Ok(n > 0)
}

/// List every IN-FLIGHT intent (`Submitting` or `Sent`) in FIFO order, decoding the `claim`
/// and `txid` the resubmission pass reconciles. An in-flight row MUST carry a `claim` (it
/// was set at `mark_submitting`); a NULL claim or a mis-sized `txid` is corruption,
/// propagated as `StoreCorrupt` (never a silently-wrong note set — a mis-witnessed spend is
/// a double-send hazard). The collect-then-decode split keeps the row error surfaced
/// (rust-patterns) while letting the decode return the typed `WalletError`. Wired at 3-b-ii-B.
pub(crate) fn list_in_flight(conn: &Connection) -> Result<Vec<InFlightIntent>, WalletError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, uri, state, claim, txids, txid, deposit_deadline FROM queued_send_intent \
             WHERE state IN (?1, ?2) ORDER BY id ASC",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![STATE_SUBMITTING, STATE_SENT], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<Vec<u8>>>(3)?,
                r.get::<_, Option<Vec<u8>>>(4)?, // txids (the slice-B canonical group)
                r.get::<_, Option<Vec<u8>>>(5)?, // txid (legacy single — read-only fallback)
                r.get::<_, Option<i64>>(6)?,
            ))
        })
        .map_err(map_db_err)?;
    let raw = rows.collect::<Result<Vec<_>, _>>().map_err(map_db_err)?;
    let mut out = Vec::with_capacity(raw.len());
    for (id, uri, state, claim_blob, txids_blob, legacy_txid_blob, deposit_deadline) in raw {
        // An in-flight row without a claim is a broken invariant — fail closed, never
        // reconcile against an empty/defaulted note set.
        let claim_blob = claim_blob.ok_or(WalletError::StoreCorrupt)?;
        let claims = decode_claims(&claim_blob)?;
        let state = IntentState::from_i64(state)?;
        // The recorded tx group: prefer the slice-B `txids` column; fall back to the legacy
        // single `txid` (a row created+recorded by a pre-slice-B binary, still in-flight across
        // the upgrade) decoded to a one-element group. A row carrying BOTH is corruption (the
        // two writers are mutually exclusive — `mark_sent_multi` only ever sets `txids`).
        let txids = match (txids_blob, legacy_txid_blob) {
            (Some(_), Some(_)) => return Err(WalletError::StoreCorrupt),
            (Some(blob), None) => decode_txids(&blob)?,
            (None, Some(b)) => {
                vec![<[u8; 32]>::try_from(b.as_slice()).map_err(|_| WalletError::StoreCorrupt)?]
            }
            (None, None) => Vec::new(),
        };
        // The state↔txids invariant (fail-closed both ways): a `Submitting` row is the
        // pre-create window so it carries NO txid; a `Sent` row recorded its group so it carries
        // ≥1. A violation is corruption — never reconcile a Sent row with nothing to broadcast,
        // nor a Submitting row that claims a tx.
        match state {
            IntentState::Submitting if !txids.is_empty() => return Err(WalletError::StoreCorrupt),
            IntentState::Sent if txids.is_empty() => return Err(WalletError::StoreCorrupt),
            _ => {}
        }
        out.push(InFlightIntent {
            id: QueuedSendId::new(id),
            uri,
            claims,
            txids,
            state,
            deposit_deadline,
        });
    }
    Ok(out)
}

/// One terminal-`Stranded` intent the §3.2i-2 2e-2b reap inspects (ADR-0535 Decision 3): its id +
/// the ORDERED tx group it recorded. `txids[0]` is tx0 — the tx that unshielded to the wallet's
/// ephemeral address — and the reap deletes the row once THAT tx is buried beyond `REORG_MAX_BLOCKS`.
/// Distinct from [`InFlightIntent`]: a `Stranded` row is NOT reconciled/re-broadcast (it left
/// [`list_in_flight`]), so this carries only what the reap needs.
#[derive(Debug)]
pub(crate) struct StrandedIntent {
    pub(crate) id: QueuedSendId,
    pub(crate) txids: Vec<[u8; 32]>,
}

/// List every terminal-`Stranded` intent (the 2e-2b reap input), decoding the recorded slice-B
/// `txids` group. A `Stranded` row MUST carry ≥1 txid (it reached `Stranded` only FROM `Sent`, which
/// recorded its group at `mark_sent_multi`); a NULL / mis-sized / empty `txids` is corruption,
/// propagated as `StoreCorrupt` (never a silently-empty group — the reap must read tx0). Reads only
/// the canonical `txids` (a `Stranded` row is post-slice-B by construction; the legacy single `txid`
/// never reaches this state). FIFO (`id` ascending) for a deterministic reap order.
pub(crate) fn list_stranded_intents(conn: &Connection) -> Result<Vec<StrandedIntent>, WalletError> {
    let mut stmt = conn
        .prepare("SELECT id, txids FROM queued_send_intent WHERE state = ?1 ORDER BY id ASC")
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map([STATE_STRANDED], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<Vec<u8>>>(1)?))
        })
        .map_err(map_db_err)?;
    let raw = rows.collect::<Result<Vec<_>, _>>().map_err(map_db_err)?;
    let mut out = Vec::with_capacity(raw.len());
    for (id, txids_blob) in raw {
        let blob = txids_blob.ok_or(WalletError::StoreCorrupt)?;
        let txids = decode_txids(&blob)?;
        if txids.is_empty() {
            return Err(WalletError::StoreCorrupt);
        }
        out.push(StrandedIntent {
            id: QueuedSendId::new(id),
            txids,
        });
    }
    Ok(out)
}

/// One `Sent` MULTI-STEP intent still owned by the outbox — the §3.2i-2 #309 in-flight
/// two-step surface's row read. `Sent` + a ≥2-tx group means tx0 (the unshield to the
/// wallet's ephemeral one-time address) was CREATED and the send is not yet complete.
/// The row leaves `Sent` by exactly THREE exits: delete-at-FINAL-BURIED (the whole
/// chain mined — done), `Stranded` (tx0 mined+buried, tx1 expired — the recoverable
/// surface takes over), or `reset_to_queued` (EVERY tx expired unmined, e.g. a deep
/// reorg — no money moved; the row returns to `Queued` and the PARKED surface, where it
/// is cancellable again). So this row's existence IS the "payment in motion through a
/// one-time address" window the host's durable don't-re-send cue renders. Carries the
/// URI (the caller parses the gross amount AFTER the lock drops) + `created_at`
/// (display-only age) + `deposit_deadline` (a swap deposit is excluded by the caller —
/// its own lifecycle).
pub(crate) struct SentMultiStepIntent {
    pub(crate) uri: String,
    pub(crate) created_at: i64,
    pub(crate) deposit_deadline: Option<i64>,
}

// §5.4 belt: the stored URI carries the RECIPIENT ADDRESS + amount — it must never
// `{:?}`-leak into a log (mirrors the surface DTOs' redacting `Debug`s).
impl std::fmt::Debug for SentMultiStepIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentMultiStepIntent")
            .field("uri", &"<redacted §5.4>")
            .field("created_at", &self.created_at)
            .field("deposit_deadline", &self.deposit_deadline)
            .finish()
    }
}

/// List every `Sent` intent whose recorded tx group has ≥2 txs (a ZIP-320 TEX two-step —
/// the queued-path shape guard admits no other multi-step to signing). READ-ONLY and
/// SIDE-EFFECT-FREE; distinct from [`list_in_flight`] (the resubmission machinery's
/// money-path reader) so this surface can never perturb reconcile inputs. Decodes the
/// canonical `txids` group only to COUNT it (the txids themselves are §5.4 never-render
/// and are not returned). A `Sent` row with NO tx group at all — both columns NULL, or
/// an empty canonical group — is corruption (the [`list_in_flight`] invariant),
/// propagated as `StoreCorrupt` — a broken row must not silently hide an in-motion
/// send. Single-tx `Sent` rows (a queued ordinary send, or a pre-slice-B legacy
/// single-`txid` row) are excluded — no ephemeral hop, the activity list's pending tx
/// presents them fully. FIFO (`id` ascending) for a deterministic render order.
pub(crate) fn list_sent_multi_step(
    conn: &Connection,
) -> Result<Vec<SentMultiStepIntent>, WalletError> {
    let mut stmt = conn
        .prepare(
            "SELECT uri, created_at, txids, txid, deposit_deadline FROM queued_send_intent \
             WHERE state = ?1 ORDER BY id ASC",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map([STATE_SENT], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<Vec<u8>>>(2)?,
                r.get::<_, Option<Vec<u8>>>(3)?,
                r.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(map_db_err)?;
    let raw = rows.collect::<Result<Vec<_>, _>>().map_err(map_db_err)?;
    let mut out = Vec::new();
    for (uri, created_at, txids_blob, legacy_txid_blob, deposit_deadline) in raw {
        // Mirror `list_in_flight`'s tx-group invariants (fail-closed, never a silent
        // skip of a broken in-motion row): BOTH columns set is corruption (the writers
        // are mutually exclusive); NEITHER set on a `Sent` row is corruption (a Sent row
        // recorded its group at `mark_sent_multi`); a legacy single-`txid` row is by
        // definition single-tx (pre-slice-B — two-step signing postdates the `txids`
        // column) and simply out of scope here.
        let tx_count = match (txids_blob, legacy_txid_blob) {
            (Some(_), Some(_)) => return Err(WalletError::StoreCorrupt),
            (Some(blob), None) => match decode_txids(&blob)?.len() {
                // A Sent row recorded ≥1 txid at `mark_sent_multi` — an empty group is
                // the same broken invariant `list_in_flight` fails closed on.
                0 => return Err(WalletError::StoreCorrupt),
                n => n,
            },
            // A legacy row's single txid must still BE a txid — a mis-sized blob is
            // corruption (the same `try_from` check `list_in_flight` fails closed on).
            (None, Some(b)) if b.len() == 32 => 1,
            (None, Some(_)) => return Err(WalletError::StoreCorrupt),
            (None, None) => return Err(WalletError::StoreCorrupt),
        };
        if tx_count < 2 {
            continue;
        }
        out.push(SentMultiStepIntent {
            uri,
            created_at,
            deposit_deadline,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memo::{Memo, Payment, PaymentRequest};
    use crate::money::{Network, Zatoshis};
    use crate::payment_uri::{encode_payment_uri, parse_payment_uri};
    use zcash_address::{ToAddress, ZcashAddress};
    use zcash_protocol::consensus::NetworkType;

    /// A fresh in-memory connection with the schema applied — the storage logic is
    /// orthogonal to SQLCipher (encryption is db.rs's concern), so unit tests run
    /// against a plain connection: fast, deterministic, no `Wallet`.
    fn fresh() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        ensure_table(&conn).expect("ensure table");
        conn
    }

    /// Build a test `PaymentRequest` through the SAME audited address encoder the
    /// parser uses (checksum real, payload bytes arbitrary — `zcash_address` is pure
    /// encoding). A Sapling (memo-capable) recipient so the memo round-trips.
    fn sapling_addr() -> crate::memo::Address {
        let encoded = ZcashAddress::from_sapling(NetworkType::Test, [0xAB; 43]).encode();
        crate::memo::Address::parse(&encoded, Network::Test).expect("valid sapling")
    }

    /// A transparent (P2PKH) recipient — the de-shield / exchange-withdrawal case.
    fn transparent_addr() -> crate::memo::Address {
        let encoded = ZcashAddress::from_transparent_p2pkh(NetworkType::Test, [0xCD; 20]).encode();
        crate::memo::Address::parse(&encoded, Network::Test).expect("valid p2pkh")
    }

    /// A unified (Orchard+Sapling) recipient — the default modern pool.
    fn unified_addr() -> crate::memo::Address {
        use zcash_address::unified::{Address as Ua, Encoding, Receiver};
        let encoded = Ua::try_from_items(vec![
            Receiver::Orchard([0xEF; 43]),
            Receiver::Sapling([0xAB; 43]),
        ])
        .expect("valid receiver set")
        .encode(&NetworkType::Test);
        crate::memo::Address::parse(&encoded, Network::Test).expect("valid ua")
    }

    #[test]
    fn ensure_table_is_idempotent() {
        let conn = fresh();
        // A second call is a no-op (the migrate chokepoint re-runs it on every open).
        ensure_table(&conn).expect("idempotent re-create");
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                ["queued_send_intent"],
                |_| Ok(true),
            )
            .expect("query");
        assert!(exists, "the intent table must exist");
    }

    #[test]
    fn enqueue_then_load_round_trips_the_uri() {
        let mut conn = fresh();
        let id = enqueue(
            &mut conn,
            "zcash:example?amount=1",
            1_700_000_000,
            None,
            None,
        )
        .expect("enqueue");
        let got = load(&conn, id).expect("load").expect("present");
        assert_eq!(got.id, id);
        assert_eq!(got.uri, "zcash:example?amount=1");
        assert_eq!(got.created_at, 1_700_000_000);
    }

    #[test]
    fn enqueue_assigns_distinct_ascending_ids() {
        let mut conn = fresh();
        let a = enqueue(&mut conn, "zcash:a", 1, None, None).expect("a");
        let b = enqueue(&mut conn, "zcash:b", 2, None, None).expect("b");
        assert_ne!(a, b, "each intent gets its own id");
        assert!(b.value() > a.value(), "rowids are monotonic (FIFO key)");
    }

    #[test]
    fn list_queued_returns_fifo_order() {
        let mut conn = fresh();
        // Enqueue with created_at DESCENDING to prove ordering is by id (insert
        // order), not by timestamp — a later-but-smaller-timestamp row stays last.
        let first = enqueue(&mut conn, "zcash:first", 300, None, None).expect("1");
        let second = enqueue(&mut conn, "zcash:second", 200, None, None).expect("2");
        let third = enqueue(&mut conn, "zcash:third", 100, None, None).expect("3");
        let listed = list_queued(&conn).expect("list");
        let ids: Vec<_> = listed.iter().map(|i| i.id).collect();
        assert_eq!(
            ids,
            vec![first, second, third],
            "FIFO by id, not by created_at"
        );
        assert_eq!(listed[0].uri, "zcash:first");
    }

    #[test]
    fn list_parkable_surfaces_a_mid_signature_row_that_list_queued_hides() {
        // #400 R2 — the OOM-during-proving shape. `mark_submitting` commits the claim BEFORE the
        // ZK prove, so a kill in that window leaves a `Submitting` row: invisible to `list_queued`,
        // to `list_in_flight_sends` (which wants `Sent` + multi-tx) and to the activity list (no
        // tx exists). That is a committed spend on NO surface — the #331 double-pay shape.
        let mut conn = fresh();
        let queued = enqueue(&mut conn, "zcash:queued", 100, None, None).expect("1");
        let claimed = enqueue(&mut conn, "zcash:claimed", 200, None, None).expect("2");
        assert!(mark_submitting(&mut conn, claimed, &[]).expect("claim"));

        assert_eq!(
            list_queued(&conn).expect("queued").len(),
            1,
            "the claimed row is (still) invisible to the drain's own reader"
        );
        let parkable = list_parkable(&conn).expect("parkable");
        assert_eq!(
            parkable
                .iter()
                .map(|(i, sending)| (i.id, *sending))
                .collect::<Vec<_>>(),
            vec![(queued, false), (claimed, true)],
            "both rows surface, FIFO, each tagged with its own state"
        );
    }

    #[test]
    fn load_parked_refuses_a_row_that_is_no_longer_queued() {
        // The THIRD of `load_parked`'s four guards (#400 R8d), previously untested and
        // money-critical: it is what stops a user-paced authorization from re-entering a row
        // some other path has ALREADY CLAIMED. The atomic `mark_submitting` is the ultimate
        // double-sign authority, but this guard is what makes the surface answer `NotFound`
        // (idempotent, no oracle) instead of racing into it.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:claimed", 1_700_000_000, None, None).expect("1");
        assert!(
            load_parked(&conn, id, 1_700_000_000)
                .expect("load")
                .is_some(),
            "a Queued row loads"
        );
        assert!(mark_submitting(&mut conn, id, &[]).expect("claim"));
        assert!(
            load_parked(&conn, id, 1_700_000_000)
                .expect("load")
                .is_none(),
            "a claimed row is NOT authorizable — the claimer owns it"
        );
    }

    #[test]
    fn list_parkable_excludes_a_sent_row() {
        // The upper bound of the widening: only `Queued` + `Submitting`. A `Sent` row HAS a
        // transaction, so it belongs to activity / the in-flight surface — showing it here would
        // double-render a payment that already left the queue.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:sent", 100, None, None).expect("1");
        assert!(mark_submitting(&mut conn, id, &[]).expect("claim"));
        mark_sent(&mut conn, id, &[0xAB; 32]).expect("sent");
        assert!(list_parkable(&conn).expect("parkable").is_empty());
    }

    #[test]
    fn load_missing_id_is_none() {
        let conn = fresh();
        assert_eq!(load(&conn, QueuedSendId::new(999)).expect("load"), None);
    }

    #[test]
    fn delete_removes_the_row_and_is_idempotent() {
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:gone", 1, None, None).expect("enqueue");
        assert!(
            delete(&mut conn, id).expect("delete"),
            "first delete removes it"
        );
        assert_eq!(load(&conn, id).expect("load"), None, "gone after delete");
        assert!(
            !delete(&mut conn, id).expect("delete again"),
            "deleting an absent row is a no-op false, never an error"
        );
    }

    #[test]
    fn payment_request_round_trips_through_the_intent_store() {
        // The MONEY-FIDELITY test: a multi-payment request with a Unicode memo must
        // survive encode → persist → load → parse byte-for-byte, so the send that
        // eventually fires is EXACTLY the one queued. Uses the public audited
        // inverse pair `encode_payment_uri`/`parse_payment_uri`.
        let original = PaymentRequest {
            payments: vec![
                Payment::new(
                    sapling_addr(),
                    Some(Zatoshis::new(100_000).expect("amount 1")),
                    Memo::text("séND 🧧 私").expect("valid memo"),
                    None,
                    None,
                )
                .expect("payment 1"),
                Payment::new(
                    sapling_addr(),
                    Some(Zatoshis::new(250_000).expect("amount 2")),
                    Memo::Empty,
                    None,
                    None,
                )
                .expect("payment 2"),
            ],
        };
        let uri = encode_payment_uri(&original).expect("encode");

        let mut conn = fresh();
        let id = enqueue(&mut conn, &uri, 42, None, None).expect("enqueue");
        let stored = load(&conn, id).expect("load").expect("present").uri;

        let restored = parse_payment_uri(&stored, Network::Test).expect("parse stored intent");
        assert_eq!(
            restored, original,
            "the queued intent re-proposes the EXACT send"
        );
    }

    #[test]
    fn transparent_and_unified_intents_round_trip_through_the_store() {
        // MONEY + PRIVACY fidelity through the PERSISTENCE door (not just the codec): a
        // de-shield (transparent) leg and a unified leg must re-propose the EXACT recipients
        // + pools after store→load. A queued de-shield that re-proposed a different recipient
        // pool would be a privacy AND money bug. (The simple round-trip test only covers
        // Sapling; this pins the two address types that carry the disclosure consequence.)
        let original = PaymentRequest {
            payments: vec![
                Payment::new(
                    transparent_addr(),
                    Some(Zatoshis::new(150_000).expect("amount t")),
                    Memo::Empty, // transparent recipients cannot carry a memo
                    None,
                    None,
                )
                .expect("de-shield leg"),
                Payment::new(
                    unified_addr(),
                    Some(Zatoshis::new(400_000).expect("amount u")),
                    Memo::text("to my UA").expect("memo"),
                    None,
                    None,
                )
                .expect("unified leg"),
            ],
        };
        let uri = encode_payment_uri(&original).expect("encode");
        let mut conn = fresh();
        let id = enqueue(&mut conn, &uri, 7, None, None).expect("enqueue");
        let stored = load(&conn, id).expect("load").expect("present").uri;
        let restored = parse_payment_uri(&stored, Network::Test).expect("parse");
        assert_eq!(
            restored, original,
            "de-shield + UA payment re-proposes EXACTLY"
        );
        assert!(
            restored.payments[0].recipient.transparent_only(),
            "the de-shield recipient stays transparent across persistence",
        );
    }

    #[test]
    fn duplicate_intent_uri_enqueues_two_distinct_rows() {
        // MONEY-SAFETY (the double-tap): two IDENTICAL intents must enqueue as TWO distinct
        // rows — paying the same person the same amount twice is TWO sends. A silent dedup
        // (e.g. a future `INSERT OR IGNORE` + a UNIQUE(uri) index) would drop a real second
        // payment with the whole suite green; this pins the no-dedup/no-clobber contract.
        let mut conn = fresh();
        let a = enqueue(&mut conn, "zcash:dup?amount=1", 0, None, None).expect("first");
        let b =
            enqueue(&mut conn, "zcash:dup?amount=1", 0, None, None).expect("second (identical)");
        assert_ne!(a, b, "an identical intent queued twice gets its own id");
        let listed = list_queued(&conn).expect("list");
        assert_eq!(
            listed.len(),
            2,
            "two identical taps = two queued sends, never merged"
        );
        assert_eq!(listed[0].uri, listed[1].uri, "both carry the identical URI");
    }

    #[test]
    fn enqueue_rolled_back_persists_no_row() {
        // CRASH-ATOMICITY made observable: a kill BETWEEN the IMMEDIATE insert and the
        // commit must leave NO row (the rollback journal restores). We simulate the kill by
        // opening the exact `enqueue` IMMEDIATE write and dropping it un-committed; the row
        // is visible inside the txn, gone after the abort.
        let mut conn = fresh();
        {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("immediate");
            tx.execute(
                "INSERT INTO queued_send_intent (uri, state, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params!["zcash:aborted", STATE_QUEUED, 0_i64],
            )
            .expect("insert");
            let in_txn: i64 = tx
                .query_row("SELECT count(*) FROM queued_send_intent", [], |r| r.get(0))
                .expect("count");
            assert_eq!(in_txn, 1, "the row is visible inside the open transaction");
            // drop `tx` WITHOUT commit ⇒ rollback (the mid-enqueue kill)
        }
        assert!(
            list_queued(&conn).expect("list").is_empty(),
            "an aborted (killed-before-commit) enqueue leaves no committed intent",
        );
    }

    // ── inc-2d-3-b-ii-A: the double-send-guard storage half ─────────────────────

    #[test]
    fn ensure_table_adds_guard_columns_to_a_pre_existing_3bi_table() {
        // A wallet provisioned under 3-b-i has the 4-column table with NO claim/txid.
        let conn = Connection::open_in_memory().expect("db");
        conn.execute_batch(
            "CREATE TABLE queued_send_intent (
                 id INTEGER PRIMARY KEY, uri TEXT NOT NULL,
                 state INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL);",
        )
        .expect("3-b-i table");
        // An existing row must survive the ADDITIVE migration (no rebuild, no data loss).
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at) VALUES ('zcash:x', 0, 1)",
            [],
        )
        .expect("seed row");

        ensure_table(&conn).expect("migrate up adds the guard columns");

        let cols = table_columns(&conn).expect("cols");
        assert!(cols.contains("claim"), "claim column added");
        assert!(cols.contains("txid"), "txid column added");
        // The pre-existing row is preserved with NULL guard columns (additive).
        let (claim, txid): (Option<Vec<u8>>, Option<Vec<u8>>) = conn
            .query_row(
                "SELECT claim, txid FROM queued_send_intent WHERE uri='zcash:x'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("read back");
        assert!(
            claim.is_none() && txid.is_none(),
            "additive: old row gets NULL guard cols"
        );
        // Idempotent: a second migrate (every open re-runs it) is a clean no-op.
        ensure_table(&conn).expect("idempotent re-migrate");
    }

    // ── inc-2d-swap-a: the deposit-deadline tag ──────────────────────────────────

    #[test]
    fn enqueue_persists_a_deposit_deadline_and_an_ordinary_send_is_none() {
        // The §4.4 tag round-trips through the persistence door: a swap deposit carries
        // `Some(expires_at)`, an ordinary `queue_send` carries `None`. The resubmission core
        // reads it back to decide the deadline exclusion.
        let mut conn = fresh();
        let deposit =
            enqueue(&mut conn, "zcash:deposit", 1, Some(1_900_000_000), None).expect("deposit");
        let ordinary = enqueue(&mut conn, "zcash:plain", 2, None, None).expect("plain");

        assert_eq!(
            load(&conn, deposit)
                .expect("load")
                .expect("present")
                .deposit_deadline,
            Some(1_900_000_000),
            "a deposit intent persists its quote deadline",
        );
        assert_eq!(
            load(&conn, ordinary)
                .expect("load")
                .expect("present")
                .deposit_deadline,
            None,
            "an ordinary send has no deadline",
        );
        // It also rides the FIFO list the resubmission pass drives.
        let listed = list_queued(&conn).expect("list");
        assert_eq!(listed[0].deposit_deadline, Some(1_900_000_000));
        assert_eq!(listed[1].deposit_deadline, None);
    }

    #[test]
    fn enqueue_persists_the_binding_and_reads_it_back() {
        // FR-17 (#396): the spend-binding nonce minted at enqueue rides the durable row —
        // the drain's sign pull presents the ROW's binding (never a re-mint, never another
        // row's), so it must survive persist → list/load BYTE-EQUAL. An ordinary
        // pre-binding-shaped enqueue (`None`) reads back `None` (an honest unbound pull).
        let mut conn = fresh();
        let binding = SpendBinding::mint();
        let bound = enqueue(&mut conn, "zcash:bound", 1, None, Some(binding)).expect("bound");
        let unbound = enqueue(&mut conn, "zcash:unbound", 2, None, None).expect("unbound");

        let listed = list_queued(&conn).expect("list");
        assert_eq!(
            listed[0].binding.as_ref().map(SpendBinding::as_bytes),
            Some(binding.as_bytes()),
            "list_queued returns the enqueue-minted binding byte-equal",
        );
        assert_eq!(listed[1].binding, None, "an unbound enqueue lists None");
        assert_eq!(
            load(&conn, bound).expect("load").expect("present").binding,
            Some(binding),
            "load returns the SAME binding the enqueue persisted",
        );
        assert_eq!(
            load(&conn, unbound)
                .expect("load")
                .expect("present")
                .binding,
            None,
            "an unbound row loads None",
        );
    }

    #[test]
    fn legacy_null_and_wrong_length_binding_rows_read_none() {
        // FR-17 migration honesty: a pre-FR-17 row (NULL binding — no binding was ever
        // shown to the host for it) and a CORRUPT row (wrong-length blob) BOTH decode to
        // `None` — the pull presents unbound, which a bound host fail-closes
        // (validate-never-truncate: corruption can never pad into a "valid" token).
        let conn = fresh();
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at, binding) \
             VALUES (?1, ?2, ?3, NULL)",
            rusqlite::params!["zcash:legacy", STATE_QUEUED, 1],
        )
        .expect("insert legacy NULL row");
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at, binding) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["zcash:corrupt", STATE_QUEUED, 2, vec![0xAAu8; 5]],
        )
        .expect("insert wrong-length blob row");

        let listed = list_queued(&conn).expect("list");
        assert_eq!(listed.len(), 2, "both raw rows list as Queued");
        assert_eq!(listed[0].binding, None, "a NULL binding reads None");
        assert_eq!(
            listed[1].binding, None,
            "a wrong-length (5-byte) blob reads None, never a padded token",
        );
        // The by-id door decodes through the SAME `decode_binding` chokepoint.
        for intent in &listed {
            assert_eq!(
                load(&conn, intent.id)
                    .expect("load")
                    .expect("present")
                    .binding,
                None,
                "load mirrors the list decode: absent/corrupt ⇒ None",
            );
        }
    }

    #[test]
    fn deposit_deadline_rides_into_the_in_flight_row() {
        // The tag must survive the Queued→Submitting transition so `reconcile_inflight` can read
        // it (the in-flight reconcile is where a created-but-freed deposit's deadline is checked).
        let mut conn = fresh();
        let id =
            enqueue(&mut conn, "zcash:deposit", 1, Some(1_888_000_000), None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [7; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submit");
        let in_flight = list_in_flight(&conn).expect("in flight");
        assert_eq!(in_flight.len(), 1);
        assert_eq!(
            in_flight[0].deposit_deadline,
            Some(1_888_000_000),
            "the deadline tag rides into the Submitting/Sent in-flight row",
        );
    }

    // ── W-swap-4-a-2: the ONE-deposit-in-flight guard at the enqueue door ──────────

    #[test]
    fn enqueue_refuses_a_second_deposit_while_one_is_in_flight() {
        // The cross-quote double-deposit refusal (§4.4): a second deadline-tagged enqueue
        // is typed-refused while ANY deposit row exists — in EVERY state it can occupy
        // (Queued: an at-execute sign failure left it for the drain; Submitting/Sent: the
        // deposit signed, notes spent, draining). Cleared by delete (the mined-buried /
        // lapsed-deadline terminals), after which a new swap enqueues normally.
        let mut conn = fresh();
        let first = enqueue(&mut conn, "zcash:dep1", 1, Some(1_800_000_000), None).expect("first");

        // Queued blocks.
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:dep2", 2, Some(1_800_000_500), None),
                Err(WalletError::SwapDepositInFlight)
            ),
            "a Queued deposit blocks a second deposit enqueue",
        );

        // Submitting blocks.
        assert!(
            mark_submitting(
                &mut conn,
                first,
                &[NoteClaim {
                    txid: [7; 32],
                    protocol: PROTO_SAPLING,
                    output_index: 0,
                }],
            )
            .expect("submit")
        );
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:dep2", 3, Some(1_800_000_500), None),
                Err(WalletError::SwapDepositInFlight)
            ),
            "a Submitting deposit blocks a second deposit enqueue",
        );

        // Sent blocks.
        assert!(mark_sent_multi(&mut conn, first, &[[9; 32]]).expect("sent"));
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:dep2", 4, Some(1_800_000_500), None),
                Err(WalletError::SwapDepositInFlight)
            ),
            "a Sent deposit blocks a second deposit enqueue",
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM queued_send_intent WHERE deposit_deadline IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0),
            )
            .expect("count"),
            1,
            "exactly ONE deposit row ever exists — every refusal left nothing behind",
        );

        // Delete clears the guard — the lockout is bounded by the deposit's lifetime.
        assert!(delete(&mut conn, first).expect("delete"));
        enqueue(&mut conn, "zcash:dep3", 5, Some(1_800_001_000), None)
            .expect("a new deposit enqueues once the in-flight one resolved");
    }

    #[test]
    fn enqueue_ordinary_send_and_deposit_never_block_each_other() {
        // The guard is deposit-vs-deposit ONLY: a user can send while a swap drains, and
        // queued ordinary sends never lock the swap door (both directions pinned).
        let mut conn = fresh();

        // Ordinary rows first — a deposit still enqueues.
        enqueue(&mut conn, "zcash:plain1", 1, None, None).expect("plain 1");
        enqueue(&mut conn, "zcash:plain2", 2, None, None).expect("plain 2");
        let dep = enqueue(&mut conn, "zcash:dep", 3, Some(1_800_000_000), None)
            .expect("queued ordinary sends never block a deposit");

        // Deposit in flight — an ordinary send still enqueues.
        enqueue(&mut conn, "zcash:plain3", 4, None, None)
            .expect("an in-flight deposit never blocks an ordinary send");

        // And the guard still holds for deposits underneath the mixed table.
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:dep2", 5, Some(1_800_000_500), None),
                Err(WalletError::SwapDepositInFlight)
            ),
            "the mixed table still refuses a second deposit",
        );
        assert!(delete(&mut conn, dep).expect("delete"));
        enqueue(&mut conn, "zcash:dep2", 6, Some(1_800_000_500), None)
            .expect("cleared once the deposit row is gone");
    }

    #[test]
    fn enqueue_guard_precedes_the_cap_on_a_full_outbox() {
        // W-swap-4-a-3 (e): when a deposit is in flight AND the outbox is full, the
        // deposit refusal must be `SwapDepositInFlight` — `QueuedSendsFull`'s "let the
        // queue drain, then retry" copy invites exactly the re-quote the guard refuses,
        // so surfacing the cap first would invert the remedy at the worst moment. The
        // order must NOT exempt deposits from the cap either: with no deposit in
        // flight, a full outbox still refuses a deposit on the cap (arm 2).
        let mut conn = fresh();

        // Arm 1: one in-flight deposit + ordinary sends up to the cap.
        enqueue(&mut conn, "zcash:dep", 1, Some(1_800_000_000), None).expect("the deposit");
        for i in 1..crate::constants::QUEUED_SEND_INTENTS_MAX {
            enqueue(&mut conn, &format!("zcash:plain{i}"), 2, None, None).expect("fill to cap");
        }
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:dep2", 3, Some(1_800_000_500), None),
                Err(WalletError::SwapDepositInFlight)
            ),
            "guard wins over the cap — never the re-quote-inviting QueuedSendsFull",
        );
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:plain-late", 4, None, None),
                Err(WalletError::QueuedSendsFull)
            ),
            "an ordinary send at the cap still meets the cap (no deadline tag ⇒ no guard)",
        );

        // Arm 2: a full outbox with NO deposit in flight still caps a deposit enqueue —
        // the reorder must not have exempted deposits from the table bound.
        let mut conn = fresh();
        for i in 0..crate::constants::QUEUED_SEND_INTENTS_MAX {
            enqueue(&mut conn, &format!("zcash:plain{i}"), 5, None, None).expect("fill to cap");
        }
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:dep", 6, Some(1_800_000_000), None),
                Err(WalletError::QueuedSendsFull)
            ),
            "with no in-flight deposit the cap still bounds a deposit enqueue",
        );
    }

    #[test]
    fn ensure_table_adds_deposit_deadline_to_a_pre_existing_guard_table() {
        // A wallet provisioned under 3-b-ii (the 6-column claim/txid table, NO deposit_deadline)
        // gains the column ADDITIVELY on its next open — no rebuild, existing rows preserved.
        let conn = Connection::open_in_memory().expect("db");
        conn.execute_batch(
            "CREATE TABLE queued_send_intent (
                 id INTEGER PRIMARY KEY, uri TEXT NOT NULL,
                 state INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL,
                 claim BLOB, txid BLOB);",
        )
        .expect("3-b-ii table");
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at) VALUES ('zcash:y', 0, 1)",
            [],
        )
        .expect("seed row");

        ensure_table(&conn).expect("migrate adds deposit_deadline");

        assert!(
            table_columns(&conn)
                .expect("cols")
                .contains("deposit_deadline"),
            "deposit_deadline column added",
        );
        let deadline: Option<i64> = conn
            .query_row(
                "SELECT deposit_deadline FROM queued_send_intent WHERE uri='zcash:y'",
                [],
                |r| r.get(0),
            )
            .expect("read back");
        assert!(
            deadline.is_none(),
            "additive: the pre-existing row gets a NULL deposit_deadline (an ordinary send)",
        );
        ensure_table(&conn).expect("idempotent re-migrate");
    }

    #[test]
    fn guard_columns_converge_to_one_ordinal_order_across_provenances() {
        // `copy_aux_tables` (the keyed-rebuild / re-key path) maps columns by ORDINAL via
        // `INSERT … SELECT *` under an ordinal/name/type signature match. So EVERY historical
        // provenance MUST converge to the SAME column order after migrate-on-open — else a
        // rebuild of an old wallet would fail-closed `StoreCorrupt` and could not re-key. This
        // pins that convergence (the reason `txids` is appended LAST, after `deposit_deadline`).
        let ordered_columns = |conn: &Connection| -> Vec<String> {
            let mut stmt = conn
                .prepare("PRAGMA table_info(queued_send_intent)")
                .expect("pragma");
            let mut cols: Vec<(i64, String)> = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                .expect("map")
                .collect::<Result<_, _>>()
                .expect("collect");
            cols.sort_by_key(|(cid, _)| *cid);
            cols.into_iter().map(|(_, name)| name).collect()
        };
        // The reference order: a freshly-created table, migrated.
        let want = ordered_columns(&fresh());
        // Each historical provenance = the exact column set THAT era's `CREATE TABLE` produced.
        let provenances = [
            // 3-b-i: the 4 base columns, no guard columns.
            "CREATE TABLE queued_send_intent (id INTEGER PRIMARY KEY, uri TEXT NOT NULL,
                 state INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL);",
            // 3-b-ii: + claim + txid.
            "CREATE TABLE queued_send_intent (id INTEGER PRIMARY KEY, uri TEXT NOT NULL,
                 state INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL,
                 claim BLOB, txid BLOB);",
            // swap-a: + deposit_deadline ALREADY present (the case `txids` must land AFTER, not
            // before — else this provenance would diverge in ordinal from a fresh DB).
            "CREATE TABLE queued_send_intent (id INTEGER PRIMARY KEY, uri TEXT NOT NULL,
                 state INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL,
                 claim BLOB, txid BLOB, deposit_deadline INTEGER);",
            // slice-B: + txids ALREADY present (the case `repropose_attempts` — #315 — must land
            // AFTER txids, not before, for the same ordinal reason).
            "CREATE TABLE queued_send_intent (id INTEGER PRIMARY KEY, uri TEXT NOT NULL,
                 state INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL,
                 claim BLOB, txid BLOB, deposit_deadline INTEGER, txids BLOB);",
        ];
        for ddl in provenances {
            let conn = Connection::open_in_memory().expect("db");
            conn.execute_batch(ddl).expect("era table");
            ensure_table(&conn).expect("migrate up");
            assert_eq!(
                ordered_columns(&conn),
                want,
                "every provenance converges to the same column ORDER (copy_aux_tables ordinal map)",
            );
        }
    }

    #[test]
    fn claim_codec_round_trips_sapling_and_orchard() {
        let claims = vec![
            NoteClaim {
                txid: [0x11; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            },
            NoteClaim {
                txid: [0xAB; 32],
                protocol: PROTO_ORCHARD,
                output_index: 7,
            },
            NoteClaim {
                txid: [0xFF; 32],
                protocol: PROTO_SAPLING,
                output_index: u32::MAX,
            },
        ];
        let blob = encode_claims(&claims);
        assert_eq!(
            blob.len(),
            claims.len() * CLAIM_RECORD_LEN,
            "fixed-width records"
        );
        assert_eq!(
            decode_claims(&blob).expect("decode"),
            claims,
            "byte-exact round trip"
        );
    }

    #[test]
    fn empty_claim_set_round_trips_empty() {
        assert!(encode_claims(&[]).is_empty());
        assert_eq!(
            decode_claims(&[]).expect("decode empty"),
            Vec::<NoteClaim>::new()
        );
    }

    #[test]
    fn decode_rejects_a_truncated_blob() {
        // A length that is not a whole number of records is corruption, never truncated.
        assert!(matches!(
            decode_claims(&[0u8; CLAIM_RECORD_LEN - 1]),
            Err(WalletError::StoreCorrupt)
        ));
        assert!(matches!(
            decode_claims(&[0u8; CLAIM_RECORD_LEN + 1]),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn decode_rejects_an_unknown_protocol_tag() {
        let claim = |protocol: u8| {
            let mut blob = encode_claims(&[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }]);
            blob[32] = protocol;
            blob
        };
        // Every KNOWN tag decodes. Asserted first and explicitly, because the
        // interesting failure of this test is not a missed rejection — it is a
        // rejection that becomes wrong. This test previously planted `2` as "the
        // unknown tag"; `2` became `PROTO_IRONWOOD` with the NU6.3 wave, and the
        // test would then have been asserting that the wallet's own new pool is
        // corrupt, failing loudly if we were lucky and passing silently if the
        // producer had not yet emitted one.
        for known in [PROTO_SAPLING, PROTO_ORCHARD, PROTO_IRONWOOD] {
            assert!(
                decode_claims(&claim(known)).is_ok(),
                "tag {known} is a pool the producer writes — it must decode"
            );
        }
        // ... and an unknown one is corruption, never a silent default. `3` is the
        // first free byte; `u8::MAX` covers the far end.
        for unknown in [3u8, 4, 200, u8::MAX] {
            assert!(
                matches!(
                    decode_claims(&claim(unknown)),
                    Err(WalletError::StoreCorrupt)
                ),
                "tag {unknown} is not a pool this build knows"
            );
        }
    }

    #[test]
    fn decode_rejects_an_absurd_note_count_before_alloc() {
        // A length implying more notes than any real proposal could select is corruption,
        // rejected on the count BEFORE iterating (§4.6 size-validate-before-use).
        let blob = vec![0u8; (MAX_CLAIM_NOTES + 1) * CLAIM_RECORD_LEN];
        assert!(matches!(
            decode_claims(&blob),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn mark_submitting_records_the_claim_and_guards_state() {
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:s", 1, None, None).expect("enqueue");
        let claims = vec![NoteClaim {
            txid: [9; 32],
            protocol: PROTO_SAPLING,
            output_index: 3,
        }];
        assert!(
            mark_submitting(&mut conn, id, &claims).expect("submit"),
            "Queued→Submitting"
        );

        let in_flight = list_in_flight(&conn).expect("in flight");
        assert_eq!(in_flight.len(), 1);
        assert_eq!(in_flight[0].id, id);
        assert_eq!(in_flight[0].claims, claims, "claim persisted + decoded");
        assert!(
            in_flight[0].txids.is_empty(),
            "no txids until created (Submitting carries an empty group)"
        );
        assert_eq!(in_flight[0].state, IntentState::Submitting);
        assert!(
            list_queued(&conn).expect("q").is_empty(),
            "left the Queued list"
        );

        // The transition is state-guarded: a second submit (no longer Queued) is a no-op.
        assert!(
            !mark_submitting(&mut conn, id, &claims).expect("again"),
            "guard: not Queued"
        );
    }

    #[test]
    fn mark_sent_records_the_txid_and_guards_state() {
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:s", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_ORCHARD,
                output_index: 0,
            }],
        )
        .expect("submit");
        let txid = [0x42u8; 32];
        assert!(
            mark_sent(&mut conn, id, &txid).expect("sent"),
            "Submitting→Sent"
        );

        let in_flight = list_in_flight(&conn).expect("in flight");
        assert_eq!(
            in_flight[0].txids,
            vec![txid],
            "the single txid persists + decodes as a one-element group"
        );
        assert_eq!(in_flight[0].state, IntentState::Sent);
        // Guarded: mark_sent again (not Submitting), and mark_submitting on a Sent row.
        assert!(
            !mark_sent(&mut conn, id, &txid).expect("again"),
            "guard: not Submitting"
        );
        assert!(
            !mark_submitting(
                &mut conn,
                id,
                &[NoteClaim {
                    txid: [2; 32],
                    protocol: PROTO_SAPLING,
                    output_index: 1
                }],
            )
            .expect("submit on sent"),
            "guard: not Queued"
        );
    }

    #[test]
    fn reset_to_queued_clears_the_stale_claim() {
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:s", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submit");
        assert!(
            reset_to_queued(&mut conn, id).expect("reset"),
            "Submitting→Queued"
        );

        assert!(
            list_in_flight(&conn).expect("none").is_empty(),
            "no longer in flight"
        );
        assert_eq!(
            list_queued(&conn).expect("q").len(),
            1,
            "back in the FIFO queue"
        );
        let claim: Option<Vec<u8>> = conn
            .query_row(
                "SELECT claim FROM queued_send_intent WHERE id=?1",
                [id.value()],
                |r| r.get(0),
            )
            .expect("read claim");
        assert!(
            claim.is_none(),
            "claim cleared (a fresh re-propose may select other notes)"
        );
        // Guarded: reset on a Queued row is a no-op.
        assert!(
            !reset_to_queued(&mut conn, id).expect("again"),
            "guard: not Submitting"
        );
    }

    #[test]
    fn enqueue_at_the_cap_is_queued_sends_full() {
        let mut conn = fresh();
        for i in 0..crate::constants::QUEUED_SEND_INTENTS_MAX {
            enqueue(&mut conn, "zcash:fill", i as i64, None, None).expect("under the cap");
        }
        let err = enqueue(&mut conn, "zcash:overflow", 0, None, None).expect_err("at the cap");
        assert!(
            matches!(err, WalletError::QueuedSendsFull),
            "the cap is typed, not a silent drop"
        );
    }

    #[test]
    fn cap_counts_inflight_rows_and_delete_frees_a_slot() {
        // The cap bounds the TRUE table size, not just the Queued backlog — a Submitting/Sent
        // in-flight row still costs resubmission work, so it occupies a slot. The real-world
        // shape: a long offline stretch queues sends that advance to in-flight on reconnect but
        // haven't mined+deleted yet; the table must still be bounded. And `delete` (the mined
        // cleanup) must reclaim a slot.
        let mut conn = fresh();
        let claim = [NoteClaim {
            txid: [3; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        }];
        // Fill exactly to the cap; advance the first two rows OUT of Queued (one Submitting,
        // one Sent) to prove in-flight rows still count.
        let mut ids = Vec::new();
        for i in 0..crate::constants::QUEUED_SEND_INTENTS_MAX {
            ids.push(
                enqueue(&mut conn, "zcash:fill", i as i64, None, None).expect("under the cap"),
            );
        }
        mark_submitting(&mut conn, ids[0], &claim).expect("submit");
        mark_submitting(&mut conn, ids[1], &claim).expect("submit");
        mark_sent(&mut conn, ids[1], &[7; 32]).expect("sent");
        // At the cap with two rows in-flight ⇒ still full.
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:over", 0, None, None),
                Err(WalletError::QueuedSendsFull)
            ),
            "an in-flight (Submitting/Sent) row still occupies a cap slot",
        );
        // Deleting the mined (Sent) row frees exactly one slot.
        assert!(
            delete(&mut conn, ids[1]).expect("delete"),
            "removed the Sent row"
        );
        enqueue(&mut conn, "zcash:now-fits", 0, None, None).expect("delete freed a slot");
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:over2", 0, None, None),
                Err(WalletError::QueuedSendsFull)
            ),
            "back at the cap after the one free slot is taken",
        );
    }

    #[test]
    fn reset_to_queued_recovers_a_sent_row_after_reorg_and_expiry() {
        // MONEY-PRESERVATION: a Sent intent (tx created + txid recorded) whose tx a deep reorg
        // un-mined AND that then EXPIRED has its notes freed — the user's payment did NOT land,
        // so it must be RE-ATTEMPTED, never dropped. `reset_to_queued` is the money-preserving
        // exit from Sent (without it, only `delete` would apply, losing the queued send). It
        // clears BOTH the stale claim and the dead txid so the re-propose is fresh.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:reorged", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [5; 32],
                protocol: PROTO_ORCHARD,
                output_index: 2,
            }],
        )
        .expect("submit");
        mark_sent(&mut conn, id, &[0xAB; 32]).expect("sent");
        // The reorg+expiry recovery: Sent → Queued.
        assert!(
            reset_to_queued(&mut conn, id).expect("reset from Sent"),
            "Sent→Queued"
        );

        assert!(
            list_in_flight(&conn).expect("none").is_empty(),
            "no longer in flight"
        );
        assert_eq!(
            list_queued(&conn).expect("q").len(),
            1,
            "re-queued for a fresh re-propose"
        );
        let (claim_null, txid_null, txids_null): (bool, bool, bool) = conn
            .query_row(
                "SELECT claim IS NULL, txid IS NULL, txids IS NULL \
                 FROM queued_send_intent WHERE id=?1",
                [id.value()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("read");
        assert!(
            claim_null && txid_null && txids_null,
            "the stale claim AND the dead tx group (both columns) cleared"
        );
    }

    #[test]
    fn list_in_flight_rejects_a_mis_sized_txid() {
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:s", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submit");
        // Inject a 16-byte txid (corruption) directly — never a panic, never a wrong [u8;32].
        conn.execute(
            "UPDATE queued_send_intent SET txid = ?1 WHERE id = ?2",
            rusqlite::params![&[0u8; 16][..], id.value()],
        )
        .expect("inject bad txid");
        assert!(matches!(
            list_in_flight(&conn),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn list_in_flight_rejects_a_null_claim_on_an_inflight_row() {
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:s", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submit");
        // A Submitting row MUST carry a claim; a NULL claim is a broken invariant ⇒ fail closed.
        conn.execute(
            "UPDATE queued_send_intent SET claim = NULL WHERE id = ?1",
            [id.value()],
        )
        .expect("null the claim");
        assert!(matches!(
            list_in_flight(&conn),
            Err(WalletError::StoreCorrupt)
        ));
    }

    // ── §3.2i-2 slice B pt 3: the multi-txid `txids` group ───────────────────────

    #[test]
    fn txid_codec_round_trips_and_preserves_order() {
        // The ordered group must survive encode→decode byte-exact and IN ORDER — the
        // re-broadcast path replays tx0 before tx1, so a reorder would broadcast the
        // pool-crossing pair backwards. Two distinct txids, deliberately not sorted.
        let txids = vec![[0xAAu8; 32], [0x11u8; 32]];
        let blob = encode_txids(&txids);
        assert_eq!(
            blob.len(),
            txids.len() * TXID_RECORD_LEN,
            "fixed-width records"
        );
        assert_eq!(
            decode_txids(&blob).expect("decode"),
            txids,
            "byte+order exact"
        );
        // The single-step group (the ordinary send) round-trips too.
        assert_eq!(
            decode_txids(&encode_txids(&[[7u8; 32]])).expect("one"),
            vec![[7u8; 32]]
        );
        // Empty stays total (the invariant "Sent ⇒ ≥1" lives at list_in_flight, not the codec).
        assert!(encode_txids(&[]).is_empty());
        assert_eq!(decode_txids(&[]).expect("empty"), Vec::<[u8; 32]>::new());
    }

    #[test]
    fn decode_txids_rejects_a_misaligned_or_absurd_blob() {
        // A length that is not a whole number of 32-byte records is corruption, never truncated.
        assert!(matches!(
            decode_txids(&[0u8; TXID_RECORD_LEN - 1]),
            Err(WalletError::StoreCorrupt)
        ));
        assert!(matches!(
            decode_txids(&[0u8; TXID_RECORD_LEN + 1]),
            Err(WalletError::StoreCorrupt)
        ));
        // A count past the cap is rejected BEFORE alloc (§4.6 size-validate-before-use)…
        let absurd = vec![0u8; (MAX_INTENT_TXIDS + 1) * TXID_RECORD_LEN];
        assert!(matches!(
            decode_txids(&absurd),
            Err(WalletError::StoreCorrupt)
        ));
        // …but EXACTLY the cap is the last valid count (boundary: accept, not reject).
        let at_cap = vec![0u8; MAX_INTENT_TXIDS * TXID_RECORD_LEN];
        assert_eq!(
            decode_txids(&at_cap)
                .expect("MAX_INTENT_TXIDS is valid")
                .len(),
            MAX_INTENT_TXIDS,
            "the cap count itself decodes (the reject is strictly above it)",
        );
    }

    #[test]
    fn mark_sent_multi_records_the_ordered_group_and_guards_state() {
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:tex", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submit");
        // The TEX two-step group: tx0 (unshield to ephemeral) then tx1 (forward to exchange).
        let group = [[0xA0u8; 32], [0xB1u8; 32]];
        assert!(
            mark_sent_multi(&mut conn, id, &group).expect("sent multi"),
            "Submitting→Sent records the whole group"
        );
        let in_flight = list_in_flight(&conn).expect("in flight");
        assert_eq!(in_flight.len(), 1);
        assert_eq!(
            in_flight[0].txids,
            group.to_vec(),
            "ordered group persists + decodes"
        );
        assert_eq!(in_flight[0].state, IntentState::Sent);
        // State-guarded: a second mark_sent_multi (no longer Submitting) is a no-op.
        assert!(
            !mark_sent_multi(&mut conn, id, &group).expect("again"),
            "guard: not Submitting"
        );
        // An EMPTY group is a structural fault (a Sent row must carry ≥1 txid), fail-closed —
        // never a silently-Sent row with nothing to broadcast/delete.
        let other = enqueue(&mut conn, "zcash:x", 2, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            other,
            &[NoteClaim {
                txid: [2; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submit");
        assert!(matches!(
            mark_sent_multi(&mut conn, other, &[]),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn list_sent_multi_step_returns_only_sent_rows_with_multi_tx_groups() {
        // The §3.2i-2 #309 in-flight surface's row read: a Queued row (no tx yet), a Sent
        // SINGLE-tx row (an ordinary queued send — the activity list presents it fully), and a
        // Sent TWO-tx row (the TEX two-step). Only the two-tx row surfaces, carrying its uri +
        // created_at + deposit_deadline (the caller's classifier parses/excludes from those).
        let mut conn = fresh();
        let claim = [NoteClaim {
            txid: [7; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        }];
        // Queued only — never surfaced here (the parked surface owns it).
        let _queued = enqueue(&mut conn, "zcash:queued", 10, None, None).expect("enqueue");
        // Sent single-tx — excluded (no ephemeral hop).
        let single = enqueue(&mut conn, "zcash:single", 20, None, None).expect("enqueue");
        mark_submitting(&mut conn, single, &claim).expect("submit");
        mark_sent_multi(&mut conn, single, &[[0xC0; 32]]).expect("sent single");
        // Sent two-tx — the in-flight two-step; surfaces.
        let pair = enqueue(&mut conn, "zcash:pair", 30, Some(99), None).expect("enqueue");
        mark_submitting(&mut conn, pair, &claim).expect("submit");
        mark_sent_multi(&mut conn, pair, &[[0xA0; 32], [0xB1; 32]]).expect("sent pair");

        let rows = list_sent_multi_step(&conn).expect("list");
        assert_eq!(rows.len(), 1, "only the Sent multi-tx row surfaces");
        assert_eq!(rows[0].uri, "zcash:pair");
        assert_eq!(rows[0].created_at, 30);
        assert_eq!(
            rows[0].deposit_deadline,
            Some(99),
            "the deadline threads to the classifier"
        );

        // §5.4 belt: the row carries the raw ZIP-321 URI (recipient + amount) —
        // its Debug must redact it.
        let dbg = format!("{:?}", rows[0]);
        assert!(!dbg.contains("zcash:pair"), "uri redacted: {dbg}");
        assert!(dbg.contains("redacted"), "redaction marker present: {dbg}");
    }

    #[test]
    fn list_sent_multi_step_fails_closed_on_every_corrupt_tx_group_shape() {
        // A `Sent` row must carry EXACTLY ONE well-formed tx-group representation —
        // the reader mirrors `list_in_flight`'s invariants and fails LOUD on every
        // corrupt shape (never silently hiding a possibly-in-motion send). All shapes
        // fabricated via raw SQL (no writer can produce them). One shape per assert,
        // row deleted between cases so each is judged alone.
        let conn = fresh();
        let corrupt_shapes: [(&str, &[&str]); 4] = [
            // NEITHER column: a Sent row recorded ≥1 txid at mark_sent_multi.
            (
                "INSERT INTO queued_send_intent (uri, state, created_at) VALUES ('zcash:x', 2, 1)",
                &[],
            ),
            // BOTH columns: the two writers are mutually exclusive.
            (
                "INSERT INTO queued_send_intent (uri, state, created_at, txids, txid) VALUES ('zcash:x', 2, 1, X'00', ?1)",
                &["txid32"],
            ),
            // EMPTY canonical group.
            (
                "INSERT INTO queued_send_intent (uri, state, created_at, txids) VALUES ('zcash:x', 2, 1, X'')",
                &[],
            ),
            // MIS-SIZED legacy txid (31 bytes).
            (
                "INSERT INTO queued_send_intent (uri, state, created_at, txid) VALUES ('zcash:x', 2, 1, ?1)",
                &["txid31"],
            ),
        ];
        for (sql, params) in corrupt_shapes {
            match params {
                ["txid32"] => conn.execute(sql, [vec![0u8; 32]]).expect("insert"),
                ["txid31"] => conn.execute(sql, [vec![0u8; 31]]).expect("insert"),
                _ => conn.execute(sql, []).expect("insert"),
            };
            assert!(
                matches!(list_sent_multi_step(&conn), Err(WalletError::StoreCorrupt)),
                "shape must fail closed: {sql}"
            );
            conn.execute("DELETE FROM queued_send_intent", [])
                .expect("clear");
        }
        // Sanity: a WELL-FORMED legacy single-txid row is skipped (single-tx), not an error.
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at, txid) VALUES ('zcash:x', 2, 1, ?1)",
            [vec![0u8; 32]],
        )
        .expect("legacy insert");
        assert!(
            list_sent_multi_step(&conn)
                .expect("legacy row is valid")
                .is_empty()
        );
    }

    #[test]
    fn mark_sent_delegates_to_a_one_element_group() {
        // The single-step sugar writes the SAME `txids` column the multi path does (not the
        // legacy `txid`), so reconcile has one source of truth.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:s", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_ORCHARD,
                output_index: 0,
            }],
        )
        .expect("submit");
        assert!(
            mark_sent(&mut conn, id, &[0x42; 32]).expect("sent"),
            "sugar transitions"
        );
        // It populated `txids`, NOT the legacy `txid` (the slice-B canonical column).
        let (txids, legacy): (Option<Vec<u8>>, Option<Vec<u8>>) = conn
            .query_row(
                "SELECT txids, txid FROM queued_send_intent WHERE id=?1",
                [id.value()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("read");
        assert_eq!(
            txids,
            Some(vec![0x42u8; 32]),
            "mark_sent writes the txids column"
        );
        assert!(
            legacy.is_none(),
            "the legacy single column is left untouched by new writes"
        );
    }

    #[test]
    fn list_in_flight_decodes_a_legacy_single_txid_to_one_element() {
        // A row created+recorded by a PRE-slice-B binary (legacy `txid` set, `txids` NULL) and
        // still in-flight across the upgrade must reconcile: it decodes to a one-element group.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:legacy", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            }],
        )
        .expect("submit");
        // Simulate the old binary's mark_sent: set state=Sent + the LEGACY txid column directly.
        conn.execute(
            "UPDATE queued_send_intent SET state = ?1, txid = ?2 WHERE id = ?3",
            rusqlite::params![STATE_SENT, &[0xCDu8; 32][..], id.value()],
        )
        .expect("legacy sent");
        let in_flight = list_in_flight(&conn).expect("list");
        assert_eq!(
            in_flight[0].txids,
            vec![[0xCD; 32]],
            "a legacy single txid decodes to a one-element group (upgrade-safe)"
        );
    }

    #[test]
    fn list_in_flight_rejects_corrupt_txid_group_shapes() {
        // Four corruption shapes, each fail-closed StoreCorrupt — never a wrong/empty group on
        // a money path. (a) a Sent row with NO txid group; (b) a Submitting row that claims a
        // group; (c) a row carrying BOTH the new and legacy columns; (d) a mis-sized `txids` blob.
        let setup = |uri: &str| {
            let mut conn = fresh();
            let id = enqueue(&mut conn, uri, 1, None, None).expect("enqueue");
            mark_submitting(
                &mut conn,
                id,
                &[NoteClaim {
                    txid: [1; 32],
                    protocol: PROTO_SAPLING,
                    output_index: 0,
                }],
            )
            .expect("submit");
            (conn, id)
        };
        // (a) Sent with no group.
        let (conn, id) = setup("zcash:a");
        conn.execute(
            "UPDATE queued_send_intent SET state = ?1 WHERE id = ?2",
            rusqlite::params![STATE_SENT, id.value()],
        )
        .expect("force sent, no txids");
        assert!(
            matches!(list_in_flight(&conn), Err(WalletError::StoreCorrupt)),
            "Sent needs ≥1 txid"
        );
        // (b) Submitting with a group.
        let (conn, id) = setup("zcash:b");
        conn.execute(
            "UPDATE queued_send_intent SET txids = ?1 WHERE id = ?2",
            rusqlite::params![encode_txids(&[[9u8; 32]]), id.value()],
        )
        .expect("inject group on submitting");
        assert!(
            matches!(list_in_flight(&conn), Err(WalletError::StoreCorrupt)),
            "Submitting carries none"
        );
        // (c) Both columns set.
        let (conn, id) = setup("zcash:c");
        conn.execute(
            "UPDATE queued_send_intent SET state = ?1, txids = ?2, txid = ?3 WHERE id = ?4",
            rusqlite::params![
                STATE_SENT,
                encode_txids(&[[9u8; 32]]),
                &[8u8; 32][..],
                id.value()
            ],
        )
        .expect("inject both");
        assert!(
            matches!(list_in_flight(&conn), Err(WalletError::StoreCorrupt)),
            "both columns = corruption"
        );
        // (d) Mis-sized txids blob (not a multiple of 32).
        let (conn, id) = setup("zcash:d");
        conn.execute(
            "UPDATE queued_send_intent SET state = ?1, txids = ?2 WHERE id = ?3",
            rusqlite::params![STATE_SENT, &[0u8; 31][..], id.value()],
        )
        .expect("inject misaligned");
        assert!(
            matches!(list_in_flight(&conn), Err(WalletError::StoreCorrupt)),
            "misaligned blob"
        );
    }

    #[test]
    fn reset_to_queued_clears_the_txids_group() {
        // The reorg+expiry recovery from a multi-txid Sent row must clear the dead group so the
        // fresh re-propose starts clean (a stale TEX group would re-broadcast dead txs).
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:tex-reorg", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [5; 32],
                protocol: PROTO_ORCHARD,
                output_index: 2,
            }],
        )
        .expect("submit");
        mark_sent_multi(&mut conn, id, &[[0xA0; 32], [0xB1; 32]]).expect("sent group");
        assert!(
            reset_to_queued(&mut conn, id).expect("reset"),
            "Sent→Queued"
        );
        let txids: Option<Vec<u8>> = conn
            .query_row(
                "SELECT txids FROM queued_send_intent WHERE id=?1",
                [id.value()],
                |r| r.get(0),
            )
            .expect("read");
        assert!(txids.is_none(), "the dead tx group is cleared on re-queue");
        assert_eq!(
            list_queued(&conn).expect("q").len(),
            1,
            "back in the FIFO queue"
        );
    }

    #[test]
    fn mark_stranded_terminates_a_sent_row_and_leaves_the_reconcile_loop() {
        // A partially-mined multi-step send → terminal Stranded: it must LEAVE the in-flight
        // reconcile set (so it can never busy-loop a doomed re-broadcast) yet NOT rejoin the
        // Queued FIFO (re-propose is impossible), and KEEP its group for the detect surface.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:stranded", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [4; 32],
                protocol: PROTO_ORCHARD,
                output_index: 1,
            }],
        )
        .expect("submit");
        // Guarded: can't strand a Submitting (never-created) row — its notes are unspent.
        assert!(
            !mark_stranded(&mut conn, id).expect("guard"),
            "only a Sent row can strand"
        );
        mark_sent_multi(&mut conn, id, &[[0xA0; 32], [0xB1; 32]]).expect("sent group");
        assert!(
            mark_stranded(&mut conn, id).expect("strand"),
            "Sent→Stranded"
        );

        assert!(
            list_in_flight(&conn).expect("inflight").is_empty(),
            "a stranded row leaves the reconcile loop (never re-broadcast again)"
        );
        assert!(
            list_queued(&conn).expect("queued").is_empty(),
            "a stranded row does NOT rejoin the Queued FIFO (re-propose is impossible)"
        );
        // The group is preserved for the 2e-2b detect / audit surface.
        let txids: Option<Vec<u8>> = conn
            .query_row(
                "SELECT txids FROM queued_send_intent WHERE id=?1",
                [id.value()],
                |r| r.get(0),
            )
            .expect("read");
        assert_eq!(
            txids.map(|b| decode_txids(&b).expect("decode")),
            Some(vec![[0xA0; 32], [0xB1; 32]]),
            "the stranded group is kept for the detect surface"
        );
        // Idempotent guard: stranding again is a no-op (no longer Sent); delete still frees it.
        assert!(
            !mark_stranded(&mut conn, id).expect("again"),
            "guard: not Sent"
        );
        assert!(
            delete(&mut conn, id).expect("delete"),
            "a stranded row is deletable"
        );
    }

    /// Build ONE terminal `Stranded` row (Queued → Submitting → Sent → Stranded) with the given
    /// ordered tx group, for the cap-hygiene + reap-input tests.
    fn make_stranded(conn: &mut Connection, uri: &str, txids: &[[u8; 32]]) -> QueuedSendId {
        let id = enqueue(conn, uri, 0, None, None).expect("enqueue");
        mark_submitting(
            conn,
            id,
            &[NoteClaim {
                txid: [9; 32],
                protocol: PROTO_ORCHARD,
                output_index: 0,
            }],
        )
        .expect("submit");
        mark_sent_multi(conn, id, txids).expect("sent group");
        assert!(mark_stranded(conn, id).expect("strand"), "Sent→Stranded");
        id
    }

    #[test]
    fn a_terminal_stranded_row_is_exempt_from_the_enqueue_cap() {
        // §3.2i-2 cap-hygiene / ADR-0535 Decision 4: a flaky/censoring link that STRANDS sends must
        // not fill the QUEUED_SEND_INTENTS_MAX cap with terminal rows and wedge NEW sends. With one
        // terminal Stranded row already present, a FULL cap of live Queued rows must still all fit.
        let mut conn = fresh();
        make_stranded(&mut conn, "zcash:stranded", &[[0xA0; 32]]);
        for i in 0..crate::constants::QUEUED_SEND_INTENTS_MAX {
            enqueue(&mut conn, &format!("zcash:q{i}"), i as i64, None, None).unwrap_or_else(|e| {
                panic!("live enqueue {i} must fit (Stranded is exempt): {e:?}")
            });
        }
        // The cap (live rows only) is now full ⇒ the next live enqueue is the honest terminal.
        assert!(
            matches!(
                enqueue(&mut conn, "zcash:over", 0, None, None),
                Err(WalletError::QueuedSendsFull)
            ),
            "the cap counts live rows; a Stranded row never steals a slot",
        );
        // The table physically holds CAP live + the 1 exempt Stranded row.
        let total: i64 = conn
            .query_row("SELECT count(*) FROM queued_send_intent", [], |r| r.get(0))
            .expect("count");
        assert_eq!(total, crate::constants::QUEUED_SEND_INTENTS_MAX as i64 + 1);
    }

    #[test]
    fn list_stranded_intents_returns_terminal_rows_with_their_ordered_tx_group() {
        let mut conn = fresh();
        // A non-terminal (Queued) row must NOT appear in the reap input.
        enqueue(&mut conn, "zcash:queued", 0, None, None).expect("queued");
        let s = make_stranded(&mut conn, "zcash:stranded", &[[0xC0; 32], [0xD1; 32]]);
        let stranded = list_stranded_intents(&conn).expect("list");
        assert_eq!(stranded.len(), 1, "only the terminal Stranded row");
        assert_eq!(stranded[0].id, s);
        assert_eq!(
            stranded[0].txids,
            vec![[0xC0; 32], [0xD1; 32]],
            "tx0 first (the reap checks tx0's burial)",
        );
    }

    #[test]
    fn list_stranded_intents_rejects_a_stranded_row_with_no_txids() {
        // A `Stranded` row reached that state from `Sent` (which always recorded a group), so a NULL
        // `txids` is corruption — fail-closed, never a silently-empty reap input (the reap must read
        // tx0). Force the broken shape directly.
        let conn = fresh();
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at, txids) \
             VALUES ('zcash:null', ?1, 0, NULL)",
            [STATE_STRANDED],
        )
        .expect("insert null");
        assert!(
            matches!(list_stranded_intents(&conn), Err(WalletError::StoreCorrupt)),
            "a NULL txids on a Stranded row is corruption",
        );
        // An EMPTY (0-byte) txids blob decodes to an empty group — the separate `is_empty` guard
        // (distinct from the NULL `ok_or` above) must ALSO fail-close it.
        let conn = fresh();
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at, txids) \
             VALUES ('zcash:empty', ?1, 0, X'')",
            [STATE_STRANDED],
        )
        .expect("insert empty");
        assert!(
            matches!(list_stranded_intents(&conn), Err(WalletError::StoreCorrupt)),
            "an empty txids group on a Stranded row is corruption (the reap has no tx0 to read)",
        );
    }

    #[test]
    fn delete_stranded_removes_only_a_terminal_stranded_row_never_a_live_id() {
        // hardening: the reap's state-guarded delete must NEVER drop a live `Queued` send, even
        // if a freed Stranded rowid is reused for it (the SQLite rowid-reuse class). A `Queued` row at
        // the same id is immune; a terminal `Stranded` row is reaped; the delete is idempotent.
        let mut conn = fresh();
        let q = enqueue(&mut conn, "zcash:queued", 0, None, None).expect("enqueue");
        assert!(
            !delete_stranded(&mut conn, q).expect("guard"),
            "a live Queued row is NOT Stranded — the reap leaves it untouched"
        );
        assert_eq!(
            list_queued(&conn).expect("q").len(),
            1,
            "the not-yet-sent payment survives the reap"
        );
        let s = make_stranded(&mut conn, "zcash:stranded", &[[0xA0; 32]]);
        assert!(
            delete_stranded(&mut conn, s).expect("strand"),
            "a terminal Stranded row IS reaped"
        );
        assert!(
            list_stranded_intents(&conn).expect("list").is_empty(),
            "the Stranded row is gone"
        );
        assert!(
            !delete_stranded(&mut conn, s).expect("again"),
            "idempotent — a second reap of the same id is a no-op, never an error"
        );
    }

    // ── 2e-2b-v-4a: the parked-send CANCEL affordance ────────────────────────────

    #[test]
    fn cancel_queued_removes_a_parked_row_and_is_idempotent() {
        // The happy path: cancel a Queued (parked) send pinned by its (id, created_at) — the row is
        // gone, and a second cancel of the now-absent row is a no-op false, never an error.
        let mut conn = fresh();
        let id =
            enqueue(&mut conn, "zcash:park?amount=1", 1_700_000_500, None, None).expect("enqueue");
        assert!(
            cancel_queued(&mut conn, id, 1_700_000_500).expect("cancel"),
            "a parked Queued row is cancelled"
        );
        assert_eq!(load(&conn, id).expect("load"), None, "the intent is gone");
        assert!(
            !cancel_queued(&mut conn, id, 1_700_000_500).expect("again"),
            "idempotent — re-cancelling an absent row is a no-op false, never an error"
        );
    }

    #[test]
    fn cancel_queued_with_a_mismatched_created_at_is_a_noop() {
        // The rowid-reuse re-verify: `id INTEGER PRIMARY KEY` has no AUTOINCREMENT, so a freed rowid
        // can be REUSED by a later enqueue carrying a DIFFERENT created_at. A cancel whose created_at
        // does not match the live row must delete NOTHING — else a stale host id could drop a send the
        // user never saw. The identity pin fails CLOSED.
        let mut conn = fresh();
        let id =
            enqueue(&mut conn, "zcash:keep?amount=1", 1_700_000_600, None, None).expect("enqueue");
        assert!(
            !cancel_queued(&mut conn, id, 1_700_000_599).expect("cancel"),
            "a wrong created_at cancels nothing (the identity pin)"
        );
        assert!(
            load(&conn, id).expect("load").is_some(),
            "the live row survives a mismatched-identity cancel"
        );
    }

    #[test]
    fn cancel_queued_never_touches_an_in_flight_or_terminal_row() {
        // MONEY-CRITICAL: cancel is guarded to `state = Queued`. A row that began draining (Submitting
        // — claim recorded), a created send (Sent — its tx exists), or a terminal Stranded row must
        // NEVER be cancellable, even with the EXACT (id, created_at): deleting a Submitting/Sent row
        // would orphan a tx that may already be on-chain (a double-pay / lost-money hazard). Each
        // stays put. (This is also the post-v-5 cancel-vs-drain race: a send that began draining wins.)
        let claim = [NoteClaim {
            txid: [7; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        }];

        // Submitting (Queued→Submitting): the drain began, the claim is recorded, no tx yet.
        let mut conn = fresh();
        let submitting = enqueue(&mut conn, "zcash:sub", 10, None, None).expect("enqueue");
        mark_submitting(&mut conn, submitting, &claim).expect("submit");
        assert!(
            !cancel_queued(&mut conn, submitting, 10).expect("cancel submitting"),
            "a Submitting row is never cancelled (the state guard)"
        );
        assert!(
            load(&conn, submitting).expect("load").is_some(),
            "the Submitting row survives"
        );

        // Sent (the tx is created + persisted): cancelling would orphan an on-chain tx.
        let mut conn = fresh();
        let sent = enqueue(&mut conn, "zcash:sent", 20, None, None).expect("enqueue");
        mark_submitting(&mut conn, sent, &claim).expect("submit");
        mark_sent(&mut conn, sent, &[0x11; 32]).expect("sent");
        assert!(
            !cancel_queued(&mut conn, sent, 20).expect("cancel sent"),
            "a Sent row is never cancelled (its tx exists)"
        );
        assert!(
            load(&conn, sent).expect("load").is_some(),
            "the Sent row survives"
        );

        // Stranded (terminal): owned by the sweep/recover surface, not cancellable through this path.
        let mut conn = fresh();
        let stranded = enqueue(&mut conn, "zcash:str", 30, None, None).expect("enqueue");
        mark_submitting(&mut conn, stranded, &claim).expect("submit");
        mark_sent(&mut conn, stranded, &[0x22; 32]).expect("sent");
        mark_stranded(&mut conn, stranded).expect("strand");
        assert!(
            !cancel_queued(&mut conn, stranded, 30).expect("cancel stranded"),
            "a Stranded row is never cancelled (the state guard)"
        );
        assert!(
            load(&conn, stranded).expect("load").is_some(),
            "the Stranded row survives"
        );
    }

    #[test]
    fn cancel_queued_never_touches_a_swap_deposit() {
        // A swap-deposit intent (deposit_deadline set) is EXCLUDED from list_parked_sends and owned by
        // the swap status surface — cancel mirrors that exclusion (`deposit_deadline IS NULL`), so even
        // a Queued deposit with the exact (id, created_at) is never dropped through this path.
        let mut conn = fresh();
        let deposit =
            enqueue(&mut conn, "zcash:deposit", 40, Some(1_900_000_000), None).expect("enqueue");
        assert!(
            !cancel_queued(&mut conn, deposit, 40).expect("cancel deposit"),
            "a swap-deposit Queued row is never cancelled (the deposit guard)"
        );
        assert!(
            load(&conn, deposit).expect("load").is_some(),
            "the deposit intent survives"
        );
    }

    #[test]
    fn cancel_queued_same_second_rowid_reuse_removes_the_new_row_money_safely() {
        // The DOCUMENTED residual of the second-granularity identity pin: `id INTEGER PRIMARY KEY` (no
        // AUTOINCREMENT) reuses a freed rowid, so after a cancel empties the table the NEXT enqueue gets
        // the SAME id. If that re-enqueue also lands in the same unix second, a STALE cancel(id, ts) now
        // matches the NEW row. This is MONEY-SAFE (the new row is Queued ⇒ no claim/txid/tx0 — a
        // not-yet-attempted intent, never funds), but it removes a send the user did not target and
        // returns true. Pinned as the regression marker: a future move to sub-second created_at or
        // AUTOINCREMENT eliminates exactly this case (flip the final cancel to a no-op false).
        let mut conn = fresh();
        let first =
            enqueue(&mut conn, "zcash:first?amount=1", 1_700_000_000, None, None).expect("first");
        assert!(
            cancel_queued(&mut conn, first, 1_700_000_000).expect("cancel"),
            "the first cancel frees the rowid"
        );
        // The table is now empty ⇒ SQLite reuses the freed rowid for the next insert.
        let second = enqueue(
            &mut conn,
            "zcash:second?amount=2",
            1_700_000_000,
            None,
            None,
        )
        .expect("second");
        assert_eq!(
            second, first,
            "SQLite reuses the freed non-AUTOINCREMENT rowid"
        );
        // A stale cancel with the OLD (now-equal) id + the SAME created_at second removes the NEW row.
        assert!(
            cancel_queued(&mut conn, first, 1_700_000_000).expect("stale cancel"),
            "the documented same-second rowid-reuse collision removes the new (Queued, fundless) row"
        );
        assert_eq!(
            load(&conn, second).expect("load"),
            None,
            "the new row is gone — money-safe, it was Queued (no funds)"
        );
    }

    // ── #315 slice 1: the repropose_attempts leak counter ─────────────────────────

    /// Read one row's raw counter directly (the tests' oracle — the production readers go
    /// through `list_queued`/`load`).
    fn attempts(conn: &Connection, id: QueuedSendId) -> i64 {
        conn.query_row(
            "SELECT repropose_attempts FROM queued_send_intent WHERE id = ?1",
            [id.value()],
            |r| r.get(0),
        )
        .expect("read counter")
    }

    #[test]
    fn ensure_table_adds_repropose_attempts_defaulting_existing_rows_to_zero() {
        // A pre-#315 wallet (slice-B era: txids present, no counter) gains the column additively
        // on next open; its existing rows read 0 (never NULL — the counter is NOT NULL DEFAULT 0),
        // so a legacy queued TEX starts a fresh, uncapped attempt budget.
        let conn = Connection::open_in_memory().expect("db");
        conn.execute_batch(
            "CREATE TABLE queued_send_intent (
                 id INTEGER PRIMARY KEY, uri TEXT NOT NULL,
                 state INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL,
                 claim BLOB, txid BLOB, deposit_deadline INTEGER, txids BLOB);",
        )
        .expect("slice-B table");
        conn.execute(
            "INSERT INTO queued_send_intent (uri, state, created_at) VALUES ('zcash:old', 0, 1)",
            [],
        )
        .expect("seed row");
        ensure_table(&conn).expect("migrate adds repropose_attempts");
        let listed = list_queued(&conn).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].repropose_attempts, 0,
            "a pre-#315 row defaults to 0 attempts"
        );
        ensure_table(&conn).expect("idempotent re-migrate");
    }

    #[test]
    fn mark_submitting_counting_increments_in_the_same_guarded_write() {
        // The counting door increments the leak counter ATOMICALLY with the Queued→Submitting
        // transition (one UPDATE — no kill window between transition and count), and ONLY when
        // the state guard passes (a lost race must not phantom-count an attempt that will not
        // create).
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:tex", 1, None, None).expect("enqueue");
        let claims = [NoteClaim {
            txid: [9; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        }];
        assert!(
            mark_submitting_counting(&mut conn, id, &claims).expect("submit"),
            "Queued→Submitting"
        );
        assert_eq!(attempts(&conn, id), 1, "the attempt is counted atomically");
        // Guard miss (no longer Queued): NO transition AND NO phantom count.
        assert!(
            !mark_submitting_counting(&mut conn, id, &claims).expect("again"),
            "guard: not Queued"
        );
        assert_eq!(attempts(&conn, id), 1, "a guard miss never counts");
        // The full requeue cycle counts once per drain attempt: reset → count → reset → count.
        for expected in 2..=3 {
            assert!(reset_to_queued(&mut conn, id).expect("requeue"));
            assert!(mark_submitting_counting(&mut conn, id, &claims).expect("re-drain"));
            assert_eq!(
                attempts(&conn, id),
                expected,
                "each ephemeral-reserving drain attempt counts exactly once"
            );
        }
    }

    #[test]
    fn plain_mark_submitting_never_counts_an_attempt() {
        // The single-step door leaves the counter at 0 — the `attempts > 0 ⟺ TEX` invariant the
        // drain's cap gate relies on (a single-step send reserves no ephemeral, so it must never
        // consume TEX attempt budget).
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:plain", 1, None, None).expect("enqueue");
        mark_submitting(
            &mut conn,
            id,
            &[NoteClaim {
                txid: [1; 32],
                protocol: PROTO_ORCHARD,
                output_index: 0,
            }],
        )
        .expect("submit");
        assert_eq!(
            attempts(&conn, id),
            0,
            "a single-step attempt is not a leak"
        );
    }

    #[test]
    fn reset_to_queued_refunding_attempt_refunds_exactly_one_and_guards_state() {
        // The typed-ceiling exit (red-team F1): release the Submitting claim AND refund the
        // pessimistic count in ONE write — net zero budget for a refusal that provably reserved
        // nothing. Guarded to Submitting only (the just-refused create window); floors at 0.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:tex", 1, None, None).expect("enqueue");
        let claims = [NoteClaim {
            txid: [8; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        }];
        mark_submitting_counting(&mut conn, id, &claims).expect("submit");
        assert_eq!(attempts(&conn, id), 1);
        assert!(
            reset_to_queued_refunding_attempt(&mut conn, id).expect("refund"),
            "Submitting→Queued with the refund"
        );
        assert_eq!(attempts(&conn, id), 0, "the ceiling attempt is refunded");
        assert_eq!(
            list_queued(&conn).expect("q").len(),
            1,
            "back in the FIFO queue with a clean claim"
        );
        // Guarded: a Queued row (already exited) refunds nothing — no double-refund window.
        assert!(
            !reset_to_queued_refunding_attempt(&mut conn, id).expect("again"),
            "guard: not Submitting"
        );
        assert_eq!(attempts(&conn, id), 0, "still floored at 0, never negative");
        // A Sent row is NOT refundable through this exit (its create was not refused).
        mark_submitting_counting(&mut conn, id, &claims).expect("submit");
        mark_sent_multi(&mut conn, id, &[[0xA0; 32], [0xB1; 32]]).expect("sent");
        assert!(
            !reset_to_queued_refunding_attempt(&mut conn, id).expect("sent"),
            "guard: a Sent row keeps its counted attempt"
        );
        assert_eq!(attempts(&conn, id), 1);
    }

    #[test]
    fn reset_to_queued_preserves_the_attempt_counter() {
        // The counter's whole job is surviving the requeue exits (#310's amplifier cycles) — a
        // reset that cleared it would re-arm the infinite leak loop the cap exists to stop. Only
        // the claim/tx group clear; the count persists.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:tex", 1, None, None).expect("enqueue");
        let claims = [NoteClaim {
            txid: [5; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        }];
        mark_submitting_counting(&mut conn, id, &claims).expect("submit");
        mark_sent_multi(&mut conn, id, &[[0xA0; 32], [0xB1; 32]]).expect("sent");
        assert!(reset_to_queued(&mut conn, id).expect("requeue (all-expired exit)"));
        assert_eq!(
            attempts(&conn, id),
            1,
            "the leak count survives the Sent→Queued requeue"
        );
        assert_eq!(
            list_queued(&conn).expect("q")[0].repropose_attempts,
            1,
            "and rides the queued-row read the drain's cap gate consumes"
        );
    }

    #[test]
    fn reset_repropose_attempts_rearms_only_the_pinned_queued_row() {
        // The retry affordance: resets a capped row's counter to 0 under cancel's guard set
        // (Queued + identity pin + non-deposit) PLUS the counter guard — a draining row, a
        // reused rowid, a swap deposit, or an untouched (attempts = 0) row is never silently
        // re-armed.
        let mut conn = fresh();
        let id = enqueue(&mut conn, "zcash:tex", 100, None, None).expect("enqueue");
        let claims = [NoteClaim {
            txid: [3; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        }];
        // A FRESH row (attempts = 0) has no budget to reset — a stale/misdirected retry is a
        // true no-op false (security review NIT-3), never a silent refill.
        assert!(
            !reset_repropose_attempts(&mut conn, id, 100).expect("fresh row"),
            "an untouched row re-arms nothing"
        );
        // Reach the cap the production way: count → requeue, three times.
        for _ in 0..crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS {
            mark_submitting_counting(&mut conn, id, &claims).expect("submit");
            reset_to_queued(&mut conn, id).expect("requeue");
        }
        assert_eq!(
            attempts(&conn, id),
            crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS
        );
        // Identity mismatch: nothing re-armed (fails closed, like cancel).
        assert!(
            !reset_repropose_attempts(&mut conn, id, 99).expect("wrong created_at"),
            "a mismatched identity pin re-arms nothing"
        );
        assert_eq!(
            attempts(&conn, id),
            crate::constants::MAX_TEX_REPROPOSE_ATTEMPTS
        );
        // The pinned row re-arms to 0 — the drain will attempt again next pass.
        assert!(
            reset_repropose_attempts(&mut conn, id, 100).expect("retry"),
            "the pinned Queued row is re-armed"
        );
        assert_eq!(attempts(&conn, id), 0, "the attempt budget is fresh");
        // A row that began draining is NOT re-armable (state guard).
        mark_submitting_counting(&mut conn, id, &claims).expect("submit");
        assert!(
            !reset_repropose_attempts(&mut conn, id, 100).expect("draining"),
            "a Submitting row is never re-armed"
        );
        // A swap deposit is NOT re-armable (its own lifecycle), even with the exact identity.
        let deposit =
            enqueue(&mut conn, "zcash:dep", 200, Some(1_900_000_000), None).expect("deposit");
        assert!(
            !reset_repropose_attempts(&mut conn, deposit, 200).expect("deposit"),
            "a swap deposit is never re-armed through this path"
        );
    }

    // The claim codec is a money-safety parser (a mis-decoded claim could mis-witness a
    // spend), so it earns property tests (testing-patterns): never-panic on arbitrary bytes
    // + round-trip over generated claim sets.
    proptest::proptest! {
        /// The `txids` codec is equally a money-safety parser — a garbled at-rest blob must
        /// return a typed error, never panic.
        #[test]
        fn decode_txids_never_panics_on_arbitrary_bytes(
            bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..1024)
        ) {
            let _ = decode_txids(&bytes);
        }

        /// Round-trip over generated groups: `decode_txids(encode_txids(g)) == g` byte+order
        /// exact for any valid group up to the cap (the testing-patterns parser round-trip rule).
        #[test]
        fn txids_codec_round_trips_over_generated_groups(
            group in proptest::collection::vec(proptest::prelude::any::<[u8; 32]>(), 0..=MAX_INTENT_TXIDS)
        ) {
            let decoded = decode_txids(&encode_txids(&group)).expect("a valid group round-trips");
            proptest::prop_assert_eq!(decoded, group);
        }

        #[test]
        fn decode_claims_never_panics_on_arbitrary_bytes(
            bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..2048)
        ) {
            // A garbled/hostile at-rest blob must return a typed error, never panic.
            let _ = decode_claims(&bytes);
        }

        #[test]
        fn claim_codec_round_trips_over_generated_sets(
            raw in proptest::collection::vec(
                (
                    proptest::prelude::any::<[u8; 32]>(),
                    proptest::prelude::any::<bool>(),
                    proptest::prelude::any::<u32>(),
                ),
                0..64,
            )
        ) {
            let claims: Vec<NoteClaim> = raw
                .into_iter()
                .map(|(txid, orchard, output_index)| NoteClaim {
                    txid,
                    protocol: if orchard { PROTO_ORCHARD } else { PROTO_SAPLING },
                    output_index,
                })
                .collect();
            let decoded = decode_claims(&encode_claims(&claims)).expect("a valid set round-trips");
            proptest::prop_assert_eq!(decoded, claims);
        }
    }
}
