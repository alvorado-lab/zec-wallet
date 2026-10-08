//! Transaction history (FR-1 — the host's activity-history list + a txid lookup).
//!
//! `TxSummary` (spec §2.5) exists in [`crate::state`] but nothing returned a LIST of
//! them; this module does, paginated. Like [`crate::intent_store`] it is pure SQL over the
//! SECOND SQLCipher-keyed `rusqlite::Connection` to the wallet DB (the `aux_db`), so it
//! never contends the engine's `WalletDb` lock and unit-tests against a bare `&Connection`.
//!
//! The data comes from librustzcash's own **`v_transactions`** view (the audited building
//! brick: `account_balance_delta` = the signed net effect on the wallet, `fee_paid`,
//! `memo_count`, `mined_height`, `expired_unmined`, `block_time`) — we do NOT re-sum notes
//! ourselves. Upstream's `WalletTest::get_tx_history` is explicitly test-only ("production
//! use could return a very large number of results; pagination … necessary"), so we add the
//! `limit` + opaque-keyset-cursor pagination it lacks while reusing its underlying view.
//!
//! §5.4 NEVER-LOG: this module touches txids + amounts + memo presence; it logs NONE of them.

use rusqlite::{Connection, params};

use crate::constants::MAX_TX_PAGE_SIZE;
use crate::db::map_aux_err;
use crate::error::WalletError;
use crate::memo::Memo;
use crate::money::{BlockHeight, TxId, ZatBalance, Zatoshis};
use crate::state::{TxStatus, TxSummary};

/// The columns of `v_transactions` we read, in SELECT order. Kept as one string so the
/// list + single-txid queries can never drift on the column set/order (the columnless
/// `map_row` reads positionally).
///
/// The final column is the §3.2i-3 TRANSPARENCY classifier: a correlated EXISTS over
/// librustzcash's own `v_tx_outputs` view — `output_pool = 0` is the crate's transparent
/// pool code (`wallet::encoding::pool_code(PoolType::Transparent)`), `is_change` filters
/// change back to ourselves. `IFNULL(is_change, 0)` reads a NULL as NOT-change ON PURPOSE:
/// a transparent output of unknown change-ness errs toward FLAGGED (public visibility is a
/// fact of pool 0; over-badging is honest, hiding is not). EXISTS itself is always 0/1.
///
/// The column after it is the outcome-resolution height (stage S2 `outcome`): the
/// engine row's `expiry_height` for a WALLET-CREATED transaction only (`created IS NOT
/// NULL` — the stamp `create_proposed_transactions` writes and the decrypt path never
/// does, the same test `crate::delivery` uses). A decrypted receive's row also holds the
/// tx's own expiry (upstream `put_tx_data` writes it), which is not an outcome the wallet
/// awaits, so it reads NULL here.
const TX_COLUMNS: &str = "txid, mined_height, account_balance_delta, fee_paid, memo_count, block_time, expired_unmined, \
     EXISTS(SELECT 1 FROM v_tx_outputs o WHERE o.txid = v_transactions.txid \
            AND o.output_pool = 0 AND IFNULL(o.is_change, 0) = 0), \
     (SELECT t.expiry_height FROM transactions t WHERE t.txid = v_transactions.txid \
            AND t.created IS NOT NULL)";

/// The stable ORDER BY for the whole history, NEWEST first. Pending (unmined) rows sort
/// above confirmed; confirmed by `mined_height` DESC; and `txid` is the FINAL tiebreaker so
/// the order is a TOTAL order. We DELIBERATELY do NOT order by `tx_index`: librustzcash sets
/// `mined_height` and `tx_index` INDEPENDENTLY (`set_transaction_status(Mined)` fills the
/// height but leaves `tx_index` NULL — the common case for a tx learned via `GetStatus` /
/// the transparent-input path, i.e. exchange withdrawals + swap-in deposits). Keying the
/// cursor on a possibly-NULL `tx_index` would make such a row UNREACHABLE on later pages
/// (NULL comparisons are never true) — it would silently HIDE received funds. `txid` is
/// globally unique, so `(mined_height, txid)` is already a total order; within a single
/// block the order is by txid (deterministic; same-block timestamp ⇒ user-irrelevant).
const TX_ORDER: &str = "ORDER BY (mined_height IS NULL) DESC, mined_height DESC, txid DESC";

/// The max length of an `after` cursor token we accept from the host (size-cap-before-work,
/// invariant 7). A real token is `"v1:" + ≤10-digit height + ":" + 64 hex` ≈ 78 bytes; cap
/// generously. Anything longer is not a token we emitted → typed reject, no parsing work.
const CURSOR_MAX_LEN: usize = 96;

/// The raw `v_transactions` row, before mapping into a [`TxSummary`]. `txid` is in
/// librustzcash INTERNAL (consensus) byte order — [`map_row`] reverses it through the
/// KAT-pinned conversion to our display-order [`TxId`].
struct RawTxRow {
    /// The raw blob as stored (INTERNAL order); [`map_row`] validates it is exactly 32
    /// bytes (fail-closed, never zero-filled) before the conversion.
    txid: Vec<u8>,
    mined_height: Option<u32>,
    account_balance_delta: i64,
    fee_paid: Option<i64>,
    memo_count: i64,
    block_time: Option<i64>,
    expired_unmined: bool,
    has_transparent_output: bool,
    /// The wallet-created transaction's stored expiry (NULL for a receive); `0` is
    /// upstream's "never expires" and [`map_row`] reads it as none.
    created_expiry_height: Option<u32>,
}

/// The keyset cursor — the sort position of the LAST row of a page (the `TX_ORDER` key:
/// `(mined_height, txid)`). The host treats it as an OPAQUE token: it gets one from
/// [`HistoryPage::next`] and passes it straight back to fetch the following page. Encoding
/// the complete sort key (height + the globally-unique txid, NOT just the height) is what
/// closes the height-only-cursor SEAM: a page boundary that falls inside a run of
/// same-height txs no longer skips the remainder, and a full page of pending txs no longer
/// strands the confirmed history behind it.
#[derive(Debug, PartialEq, Eq)]
struct HistoryCursor {
    /// `None` ⇒ the cursor row is PENDING (unmined); `Some` ⇒ its mined height.
    mined_height: Option<u32>,
    /// The cursor row's txid in INTERNAL (stored) byte order — the unique within-height
    /// tiebreaker (and the sole discriminator among pending rows).
    txid_internal: [u8; 32],
}

/// The cursor format version. Bumped if the token layout ever changes, so an old persisted
/// token can never be MIS-parsed into a valid-but-wrong key (it would reject typed instead).
const CURSOR_VERSION: &str = "v1";

impl HistoryCursor {
    /// The cursor for a raw row (its sort position), to hand back as `next`.
    fn of(raw: &RawTxRow) -> Result<Self, WalletError> {
        let txid_internal: [u8; 32] = raw
            .txid
            .as_slice()
            .try_into()
            .map_err(|_| WalletError::StoreCorrupt)?;
        Ok(Self {
            mined_height: raw.mined_height,
            txid_internal,
        })
    }

    /// Encode to the opaque token: `"v1:{mined_height|-}:{txid_internal hex}"`. A readable
    /// structured form (debuggable), but the host MUST treat it as opaque.
    fn encode(&self) -> String {
        let height = self
            .mined_height
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "{CURSOR_VERSION}:{height}:{}",
            hex::encode(self.txid_internal)
        )
    }

    /// Decode an opaque token the host passed back. Strict + PANIC-FREE on ANY `&str`
    /// (invariant 7 — the token crosses the FFI from the host and may be corrupted): a
    /// malformed cursor fails typed ([`WalletError::StoreCorrupt`] — the single corruption
    /// door), NEVER a panic (which, run under the aux-db lock, would poison it) and never a
    /// silent wrong-page (which would HIDE rows — a money-visibility bug).
    fn decode(s: &str) -> Result<Self, WalletError> {
        // Size-cap BEFORE any parsing work (invariant 7); a real token is ≤ ~78 bytes.
        if s.len() > CURSOR_MAX_LEN {
            return Err(WalletError::StoreCorrupt);
        }
        let mut it = s.splitn(3, ':');
        if it.next() != Some(CURSOR_VERSION) {
            return Err(WalletError::StoreCorrupt);
        }
        let mined_height = match it.next().ok_or(WalletError::StoreCorrupt)? {
            "-" => None,
            v => Some(v.parse::<u32>().map_err(|_| WalletError::StoreCorrupt)?),
        };
        let hex = it.next().ok_or(WalletError::StoreCorrupt)?;
        // The audited `hex` brick decodes over BYTES into the fixed 32-byte buffer: it
        // errors (never panics) on a wrong length OR any non-hex byte — including a
        // multibyte UTF-8 char that a naive `&hex[i..]` string-slice would panic on (a
        // panic here runs under the aux-db lock and would poison it). No hand-rolled loop.
        let mut txid_internal = [0u8; 32];
        hex::decode_to_slice(hex, &mut txid_internal).map_err(|_| WalletError::StoreCorrupt)?;
        Ok(Self {
            mined_height,
            txid_internal,
        })
    }
}

/// One page of history plus the cursor to continue (spec §3.3). `next` is `Some(opaque)`
/// when MORE rows may follow (the host shows a "load more" affordance and passes the token
/// straight back), `None` on the last page. The `next` token is OPAQUE — the host stores +
/// returns it verbatim and never parses it (its shape is an internal keyset detail).
pub struct HistoryPage {
    pub rows: Vec<TxSummary>,
    pub next: Option<String>,
}

/// List the wallet's transaction history, newest first, paginated by an opaque KEYSET
/// cursor (FR-1; spec §3.3). Pending (unmined) rows sort ABOVE confirmed ones (see
/// [`TX_ORDER`]). `after = None` is the first page; `after = Some(token)` is the page
/// strictly after the cursor row (the token from the previous [`HistoryPage::next`]).
/// `limit` caps the page and is clamped to [`MAX_TX_PAGE_SIZE`] so a host-supplied huge
/// value can't materialize an unbounded result.
///
/// KEYSET (not height-only): the cursor encodes the last row's full sort key
/// `(mined_height, txid)` (see [`TX_ORDER`] for why NOT `tx_index`), and the next page
/// selects rows strictly after it in that order. This closes the height-only SEAM: a page
/// boundary that lands inside a run of
/// same-height txs no longer skips the remainder (the classic "exchange paid me 60 notes in
/// one block, page 2 hides 10 of them" money-visibility bug), and a full page of pending
/// txs no longer strands the confirmed history behind it. The union of all pages is exactly
/// every row once (pinned by `keyset_pages_union_to_every_row_once_even_with_same_height`).
///
/// Scoped to the PRIMARY account (the first by `accounts.id`) — this MUST stay consistent
/// with [`account::primary_account_id`](crate::account::primary_account_id), which is the
/// `WalletRead` door for the same choice (`get_account_ids().next()`); a future multi-account
/// increment updates BOTH (here the SQL runs on the bare aux `Connection`, which can't take
/// the typed `WalletRead` path). A single-account wallet by design (§2.1). No account
/// imported yet ⇒ an empty page (the subquery yields NULL, matching no rows), never an error.
///
/// `scanned_tip` is the highest block the wallet has scanned (`MAX(blocks.height)`), read
/// once on the SAME connection so the confirmation depths are consistent with the page.
pub(crate) fn list_transactions(
    conn: &Connection,
    limit: u32,
    after: Option<&str>,
) -> Result<HistoryPage, WalletError> {
    let limit = limit.min(MAX_TX_PAGE_SIZE);
    if limit == 0 {
        // A zero-limit page is empty by definition; short-circuit so the `+1` peek and the
        // `rows[limit-1]` cursor index below never underflow.
        return Ok(HistoryPage {
            rows: Vec::new(),
            next: None,
        });
    }
    let scanned_tip = scanned_tip(conn)?;
    let cursor = after.map(HistoryCursor::decode).transpose()?;

    // Build the keyset WHERE fragment + its positional params for the cursor kind. All
    // values are bound (the integers could be inlined safely, but binding matches the
    // crate's no-string-in-SQL discipline). A `+1` peek row tells us whether a NEXT page
    // exists without a second round-trip.
    let fetch = (limit as i64) + 1;
    let mut keyset = String::new();
    let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    match &cursor {
        None => {}
        Some(c) if c.mined_height.is_none() => {
            // Cursor is PENDING: remaining pending rows after it (by txid), then ALL mined.
            keyset.push_str(
                " AND ((mined_height IS NULL AND txid < ?1) OR mined_height IS NOT NULL)",
            );
            binds.push(Box::new(c.txid_internal.to_vec()));
        }
        Some(c) => {
            // Cursor is MINED: only mined rows strictly after (height, txid) DESC. txid is
            // globally unique, so this is exact; crucially it does NOT reference `tx_index`
            // (which librustzcash leaves NULL on many mined rows — a NULL comparison would
            // make those rows unreachable on later pages, hiding received funds).
            keyset.push_str(
                " AND mined_height IS NOT NULL AND (\
                   mined_height < ?1 \
                   OR (mined_height = ?1 AND txid < ?2))",
            );
            binds.push(Box::new(c.mined_height));
            binds.push(Box::new(c.txid_internal.to_vec()));
        }
    }
    binds.push(Box::new(fetch));

    let sql = format!(
        "SELECT {TX_COLUMNS} \
         FROM v_transactions \
         WHERE account_uuid = (SELECT uuid FROM accounts ORDER BY id LIMIT 1){keyset} \
         {TX_ORDER} \
         LIMIT ?{}",
        binds.len()
    );
    let mut stmt = conn.prepare(&sql).map_err(map_aux_err)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(binds.iter()), read_row)
        .map_err(map_aux_err)?;
    let mut raws = Vec::new();
    for row in rows {
        raws.push(row.map_err(map_aux_err)?);
    }

    // The `+1` peek: if present, MORE rows follow → hand back the cursor of the LAST
    // RETURNED row (index limit-1) and drop the peek; otherwise this is the last page.
    let next = if raws.len() as u32 > limit {
        let cursor = HistoryCursor::of(&raws[(limit - 1) as usize])?.encode();
        raws.truncate(limit as usize);
        Some(cursor)
    } else {
        None
    };
    let mut out = Vec::with_capacity(raws.len());
    for raw in raws {
        out.push(map_row(raw, scanned_tip)?);
    }
    Ok(HistoryPage { rows: out, next })
}

/// Look up ONE transaction by its (display-order) txid — the FR-1 txid-keyed accessor.
/// `Ok(None)` if this wallet has no row for it (not its tx, or not yet detected).
pub(crate) fn transaction_by_txid(
    conn: &Connection,
    txid: &TxId,
) -> Result<Option<TxSummary>, WalletError> {
    let scanned_tip = scanned_tip(conn)?;
    // The view stores INTERNAL byte order; our `TxId` is DISPLAY order — reverse back to
    // match the stored blob (the inverse of `map_row`'s conversion).
    let mut internal = *txid.as_bytes();
    internal.reverse();
    let sql = format!(
        "SELECT {TX_COLUMNS} \
         FROM v_transactions \
         WHERE account_uuid = (SELECT uuid FROM accounts ORDER BY id LIMIT 1) \
           AND txid = ?1 \
         LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql).map_err(map_aux_err)?;
    let mut rows = stmt
        .query_map(params![internal.as_slice()], read_row)
        .map_err(map_aux_err)?;
    match rows.next() {
        Some(row) => Ok(Some(map_row(row.map_err(map_aux_err)?, scanned_tip)?)),
        None => Ok(None),
    }
}

/// Read the MEMOS the primary account can view in one transaction (FR-1 memo-content read) —
/// the recovered message text/bytes a [`TxSummary::has_memo`] only flags as present. Returns
/// the non-empty memos in output order, parsed through the audited ZIP-302 classifier
/// ([`Memo::from_wire`]); an empty memo (`0xF6`) is filtered out (it carries no message, matching
/// the `has_memo`/`memo_count` semantics). Reads `v_tx_outputs.memo` — populated by tx-ENHANCEMENT
/// (`Wallet::enhance_transactions`), NOT by compact scan (compact blocks omit memo bytes), so a
/// scanned-but-not-yet-enhanced receive returns an empty list until enhancement runs.
///
/// Scoped to the primary account, including BOTH received-output memos (paid TO us) and our own
/// outgoing-output memos (sent FROM us, decryptable via our OVK) — the full set
/// `memo_count` counts. `txid` is DISPLAY order (reversed to the view's INTERNAL storage).
/// **The two failure modes are DIFFERENT and are treated differently** (FR-27
/// post-fold security review). A memo row can go wrong two ways, and the first
/// version of this fix collapsed them:
///
/// - **An oversized blob (> 512 bytes) is LOCAL CORRUPTION** — librustzcash
///   stores what it decrypted, so no counterparty can put one there. It stays
///   FATAL: swallowing it would hide a broken store behind a short list, which
///   is a silent failure (principle 10) and the `.ok()`-on-a-DB-read shape
///   rust-patterns bans outright.
/// - **A classifier rejection is HOSTILE INPUT** — ZIP-302's one upstream-invalid
///   shape (a text lead over non-UTF-8 bytes), and the party paying you chooses
///   every byte of every memo in their own transaction. It is SKIPPED and
///   COUNTED, never fatal: failing the whole read would let one dust output deny
///   every other memo in the transaction, which is the "wrote it, then made it
///   unreadable" failure FR-27 exists to end.
///
/// **Both lanes agree on the readable set, and that is deliberate.** An earlier
/// fold gave the display lane `Fail` and the machine lane `Skip` over the same
/// rows. That let a payer attach one malformed dust memo and have the HOST act
/// on their envelope while the USER's display read returned nothing for that
/// transaction at all — a payer-controlled divergence between what the screen
/// says and what the host recorded, which is precisely the class FR-26's review
/// found dangerous on the send side. One readable set, one truth; the count is
/// how a caller degrades honestly (§6 "the UI tells the truth about what works").
pub struct TransactionMemos {
    /// The readable, non-empty memos in output order.
    pub memos: Vec<Memo>,
    /// How many outputs carried a memo the audited classifier REJECTED. A caller
    /// rendering to a human should say so ("1 memo could not be read") rather
    /// than present a short list as complete; a machine caller can fail closed
    /// on it. Non-zero is attacker-reachable and is NOT a corruption signal.
    pub unreadable: usize,
}

pub(crate) fn transaction_memos(
    conn: &Connection,
    txid: &TxId,
) -> Result<TransactionMemos, WalletError> {
    use zcash_protocol::memo::MemoBytes;
    let mut internal = *txid.as_bytes();
    internal.reverse();
    // The primary-account uuid (one source of truth with `primary_account_id` / the list scope).
    // `output_pool` is the tiebreaker so two outputs at the same `output_index` in different pools
    // (Sapling vs Orchard) have a DETERMINISTIC display order (`v_tx_outputs` is unique on
    // `(transaction_id, output_pool, output_index)`).
    let sql = "SELECT memo FROM v_tx_outputs \
               WHERE txid = ?1 AND memo IS NOT NULL \
                 AND (to_account_uuid = (SELECT uuid FROM accounts ORDER BY id LIMIT 1) \
                   OR from_account_uuid = (SELECT uuid FROM accounts ORDER BY id LIMIT 1)) \
               ORDER BY output_index, output_pool";
    let mut stmt = conn.prepare(sql).map_err(map_aux_err)?;
    let rows = stmt
        .query_map(params![internal.as_slice()], |r| r.get::<_, Vec<u8>>(0))
        .map_err(map_aux_err)?;
    let mut memos = Vec::new();
    let mut unreadable = 0usize;
    for blob in rows {
        let blob = blob.map_err(map_aux_err)?;
        // librustzcash stores the memo TRIMMED of trailing zeros (`memo_repr`/`MemoBytes::as_slice`)
        // — the empty memo is a single `0xF6` byte, "hi" is 3 bytes, etc. — so a stored blob is
        // almost always SHORTER than 512: non-512 is the NORM, not corruption. `MemoBytes::from_bytes`
        // losslessly re-pads any blob ≤ 512 back to the canonical field (the exact inverse of the
        // trim), and rejects ONLY a > 512 blob — the one true corruption signal — as `MemoInvalid`.
        // DO NOT add a `len == 512` guard: it would reject every real (trimmed) memo.
        //
        // FATAL, and separately from the classifier below: nothing a counterparty
        // can do puts an oversized blob here (the engine stores what it
        // decrypted), so this is our own store being wrong and it must not be
        // swallowed into a silently-short list.
        let mb = MemoBytes::from_bytes(&blob).map_err(|_| WalletError::MemoInvalid)?;
        // `from_wire` RELABELS the audited upstream classification (never panics on hostile
        // bytes — A4). Its ONE error is the attacker-reachable shape, so it is COUNTED, not
        // fatal — see the type doc for why one dust output must not deny the transaction.
        match Memo::from_wire(&mb) {
            Err(_) => unreadable += 1,
            Ok(Memo::Empty) => {} // no message — filtered (matches has_memo/memo_count)
            Ok(memo) => memos.push(memo),
        }
    }
    Ok(TransactionMemos { memos, unreadable })
}

/// The highest block height the wallet has scanned, for confirmation-depth math. `None`
/// (no blocks scanned yet) ⇒ a mined tx reports depth 1 (it is at least in a known block).
pub(crate) fn scanned_tip(conn: &Connection) -> Result<Option<u32>, WalletError> {
    conn.query_row("SELECT MAX(height) FROM blocks", [], |r| {
        r.get::<_, Option<u32>>(0)
    })
    .map_err(map_aux_err)
}

/// The FULLY-SCANNED frontier (R1): the highest height H such that every
/// block at or below H has been scanned — the ONLY height an incoming-funds
/// watermark may honestly claim. `MAX(blocks.height)` alone OVER-claims under
/// the sync plan's PRIORITY ORDER (Verify-near-tip ranges scan FIRST during a
/// restore/rescan, so the scan maximum runs far ahead of pending LOWER ranges;
/// a cursor stamped from it would suppress replay of arrivals in ranges never
/// diffed — the backgrounded-mid-restore miss). Reads `scan_queue` —
/// internal zcash_client_sqlite schema, bounded by the workspace's EXACT
/// `=0.21.0` pin (priority code: `Scanned` = 10, pending work > 10; see
/// zcash_client_sqlite::wallet::scanning). Fails only in the safe direction: a
/// stale/lower answer WIDENS the next replay (duplicates — the documented
/// at-least-once direction), never narrows it. `None` ⇒ nothing scanned yet.
pub(crate) fn fully_scanned_frontier(conn: &Connection) -> Result<Option<u32>, WalletError> {
    let Some(tip) = scanned_tip(conn)? else {
        return Ok(None);
    };
    let pending_start: Option<u32> = conn
        .query_row(
            "SELECT MIN(block_range_start) FROM scan_queue WHERE priority > 10",
            [],
            |r| r.get::<_, Option<u32>>(0),
        )
        .map_err(map_aux_err)?;
    Ok(Some(match pending_start {
        // No pending ranges: the scan maximum IS the frontier.
        None => tip,
        // Pending work exists: nothing above (start − 1) is fully covered.
        Some(p) => tip.min(p.saturating_sub(1)),
    }))
}

// ── The incoming-funds detect/replay reads (ADR-0536 #392) ───────────────────────

/// The incoming-funds WATERMARK cursor: `"w1:<height>"` — "arrivals mined at or
/// below `<height>` are accounted". DELIBERATELY a different version prefix from
/// [`HistoryCursor`]'s keyset token (`"v1:…"`), so a host that accidentally feeds a
/// `HistoryPage.next` token to `watch_incoming_funds` gets a TYPED reject, never a
/// silently-wrong replay window (the money-visibility direction of that mistake
/// would be a suppressed catch-up). Opaque to the host, like every cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IncomingCursor(pub(crate) u32);

/// The incoming-cursor format version (see [`CURSOR_VERSION`] for the bump rule).
const INCOMING_CURSOR_VERSION: &str = "w1";

impl IncomingCursor {
    /// Encode to the opaque token. Readable-but-opaque, like the keyset token.
    pub(crate) fn encode(self) -> String {
        format!("{INCOMING_CURSOR_VERSION}:{}", self.0)
    }

    /// Decode a host-supplied token. Strict + panic-free on ANY `&str` (invariant 7:
    /// it crosses the FFI and may be corrupted): size-cap before parsing work, exact
    /// version match, a bare `u32` height, NOTHING trailing. Malformed ⇒ typed
    /// [`WalletError::StoreCorrupt`] (the single corruption door, matching
    /// [`HistoryCursor::decode`]), never a panic and never a silent wrong-window.
    pub(crate) fn decode(s: &str) -> Result<Self, WalletError> {
        if s.len() > CURSOR_MAX_LEN {
            return Err(WalletError::StoreCorrupt);
        }
        let mut it = s.splitn(2, ':');
        if it.next() != Some(INCOMING_CURSOR_VERSION) {
            return Err(WalletError::StoreCorrupt);
        }
        let height = it
            .next()
            .ok_or(WalletError::StoreCorrupt)?
            .parse::<u32>()
            .map_err(|_| WalletError::StoreCorrupt)?;
        Ok(Self(height))
    }
}

/// Count + mined-height span of the primary account's INCOMING rows (positive
/// `account_balance_delta`) in a height window — the ADR-0536 detect/replay truth
/// read. `count == 0` ⇒ `span == None` (SQL MIN/MAX of no rows are NULL).
pub(crate) struct IncomingSpan {
    pub(crate) count: u32,
    pub(crate) span: Option<(u32, u32)>,
}

/// The scan-edge truth diff: incoming rows mined in `[from, to]` (both INCLUSIVE —
/// the caller converts the batch's `[from, end)` to `(from, end-1)`). Runs on the
/// bare aux connection like every history read; positive-delta is the same
/// "the wallet got richer" predicate the activity UI renders as received. A
/// change-only / self-send batch (the `sync::ScanOutcome::Scanned` gate's
/// over-approximation) finds zero rows here and stays silent.
pub(crate) fn incoming_in_span(
    conn: &Connection,
    from: u32,
    to: u32,
) -> Result<IncomingSpan, WalletError> {
    incoming_where(conn, "mined_height BETWEEN ?1 AND ?2", params![from, to])
}

/// The high-water `id` of `ironwood_received_notes` — `0` on a wallet that has
/// received none.
///
/// **Why this exists (T0-3, §4z).** ADR-0536's incoming-funds gate is driven by
/// `ScanSummary::received_*_note_count`, and upstream's `ScanSummary`
/// (`zcash_client_backend-0.24.0 data_api/chain.rs:443-500`) counts Sapling and
/// Orchard and **nothing else** — `scan_cached_blocks`'s accumulation loop
/// (`:679-687`) never reads `wtx.ironwood_outputs()`, though the accessor
/// exists. So an Ironwood-only batch reported `received_notes == 0`, the gate
/// stayed shut, `detect_incoming` never ran, no `IncomingFundsEvent` was
/// emitted and the host could not notify. The truth view `v_transactions`
/// already SEES Ironwood rows (`v_received_outputs`'s third `UNION`), so the
/// blindness was in the gate alone.
///
/// A delta of this value across one `scan_cached_blocks` is the Ironwood
/// receipt count for that batch. `id INTEGER PRIMARY KEY` is a rowid alias, so
/// `MAX(id)` is the rowid B-tree's rightmost entry — O(1), not a table scan,
/// which is what lets the gate stay CHEAP (ADR-0536's whole reason for gating
/// was that the `v_transactions` truth read measured ~74 % of a batch on a deep
/// restore). Two of these per batch is two index seeks.
///
/// The delta is only ever taken across a SUCCEEDED scan, which can insert but
/// never delete: a continuity error takes `scan_batch`'s reorg arm and computes
/// no delta. So the subtraction cannot see a rewind's deletions, and it
/// saturates anyway.
///
/// **What a rewind actually does to this value, which is not deletion** (
/// crypto pass): `truncate_to_height_internal` UN-MINES rather than deletes —
/// `UPDATE transactions SET mined_height = NULL WHERE mined_height > :height`
/// — so the `transactions` rows survive, the `ON DELETE CASCADE` never fires,
/// and these rows survive with them. Re-scanning the rewound span then hits
/// `ON CONFLICT (transaction_id, action_index) DO UPDATE` and REUSES the
/// existing rowid, so `MAX(id)` does not move and the re-scanned batch reports
/// zero Ironwood receipts where the Sapling and Orchard terms report their full
/// re-scanned counts. That asymmetry is wanted: it suppresses a duplicate
/// arrival notification for money the user was already told about. A genuinely
/// NEW Ironwood note in the reorganised chain has a new `transactions` row and
/// does move the high water, so first-arrival detection is intact.
pub(crate) fn ironwood_received_high_water(conn: &Connection) -> Result<u64, WalletError> {
    let hw: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(id), 0) FROM ironwood_received_notes",
            [],
            |r| r.get(0),
        )
        .map_err(map_aux_err)?;
    Ok(u64::try_from(hw).unwrap_or(0))
}

/// The replay read: incoming rows mined STRICTLY ABOVE `watermark` (`None` ⇒ every
/// mined incoming row — the full-history catch-up). UNMINED positive rows are
/// deliberately excluded: a watermark cursor can only account MINED heights, and a
/// pending row would re-count on every replay until it mines (churn, not truth) —
/// it fires a `Live` event at its mining batch instead.
pub(crate) fn incoming_above(
    conn: &Connection,
    watermark: Option<u32>,
) -> Result<IncomingSpan, WalletError> {
    incoming_where(conn, "mined_height > COALESCE(?1, -1)", params![watermark])
}

/// The shared shape of the two incoming reads: one aggregate row over
/// `v_transactions`, primary-account-scoped exactly like [`list_transactions`]
/// (the same subquery — a future multi-account increment updates all three
/// together). `mined_height IS NOT NULL` is implied by both callers' predicates
/// (BETWEEN / >), so no explicit clause.
fn incoming_where(
    conn: &Connection,
    height_pred: &str,
    binds: impl rusqlite::Params,
) -> Result<IncomingSpan, WalletError> {
    let sql = format!(
        "SELECT COUNT(*), MIN(mined_height), MAX(mined_height) \
         FROM v_transactions \
         WHERE account_uuid = (SELECT uuid FROM accounts ORDER BY id LIMIT 1) \
           AND account_balance_delta > 0 \
           AND {height_pred}"
    );
    let (count, lo, hi) = conn
        .query_row(&sql, binds, |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, Option<u32>>(1)?,
                r.get::<_, Option<u32>>(2)?,
            ))
        })
        .map_err(map_aux_err)?;
    Ok(IncomingSpan {
        count,
        span: match (lo, hi) {
            (Some(lo), Some(hi)) => Some((lo, hi)),
            _ => None,
        },
    })
}

/// Read a `v_transactions` row positionally (the [`TX_COLUMNS`] order).
///
/// `expired_unmined` is the view expression
/// `(mined_height IS NULL AND expiry_height BETWEEN 1 AND max_height)`. For an UN-MINED
/// RECEIVED tx the wallet has no `expiry_height` (only wallet-CREATED sends set one), so
/// SQLite three-valued logic makes the whole expression **NULL**, not 0/1 — e.g. a payment
/// detected before it confirms, or a received tx orphaned by a reorg. Read it as
/// `Option<i64>` and treat NULL as NOT expired (the row is Pending, which is exactly right):
/// reading it as a bare `i64` errors the ENTIRE page to `StoreCorrupt` the moment one such
/// row exists — the whole history would vanish behind an error on a single unconfirmed
/// receive (regression: `a_reorg_orphans_a_received_tx_then_a_rescan_restores_it` +
/// `pending_received_tx_with_null_expired_unmined_maps_to_pending`).
fn read_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RawTxRow> {
    Ok(RawTxRow {
        // Read the raw blob; the 32-byte length is enforced in `map_row` (fail typed, never
        // truncate/zero-fill — the rust-patterns "propagate corruption, never default" rule).
        txid: r.get(0)?,
        mined_height: r.get(1)?,
        // `account_balance_delta` (col 2) and `memo_count` (col 4) are read as bare `i64`
        // ON PURPOSE: the view's `GROUP BY notes.account_id, notes.transaction_id` over the
        // INNER-joined `notes` CTE guarantees ≥1 non-NULL note per emitted row, so these
        // `SUM(...)` columns are non-NULL by construction. If an upstream view change ever
        // made them nullable, a bare read would error the WHOLE page to `StoreCorrupt` — the
        // same class of bug the `expired_unmined` read below just fixed; revisit them together.
        account_balance_delta: r.get(2)?,
        fee_paid: r.get(3)?,
        memo_count: r.get(4)?,
        block_time: r.get(5)?,
        expired_unmined: r.get::<_, Option<i64>>(6)?.unwrap_or(0) != 0,
        // EXISTS is 0/1 by construction (never NULL); the Option read is the same
        // belt-and-suspenders as col 6 — a NULL must degrade one flag, never error
        // the whole page to `StoreCorrupt`. NULL ⇒ un-flagged (the plain default).
        has_transparent_output: r.get::<_, Option<i64>>(7)?.unwrap_or(0) != 0,
        // NULL for a receive (the column's `created` gate) or a missing row.
        created_expiry_height: r.get(8)?,
    })
}

/// Map a raw `v_transactions` row into a `TxSummary` (spec §2.5). PURE (no IO) so the
/// money-display logic — signed net amount, fee, confirmation depth, and the
/// Expired/Confirmed/Pending status — is unit-tested directly, without a populated DB.
///
/// `scanned_tip` is the confirmation basis (highest scanned block); depth is
/// `tip − mined_height + 1` (saturating). `Queued`/`Failed` are SDK-side states from the
/// intent/submit path, never the chain view, so they don't appear here.
fn map_row(raw: RawTxRow, scanned_tip: Option<u32>) -> Result<TxSummary, WalletError> {
    // Validate-never-truncate: a non-32-byte txid blob is DB corruption — fail typed
    // (`StoreCorrupt`), never zero-fill (a phantom all-zero id would render + falsely match
    // a lookup). librustzcash stores INTERNAL order; THE audited reversing door (the
    // `From<zcash_protocol::TxId>` impl) yields our display order.
    let txid_bytes: [u8; 32] = raw
        .txid
        .as_slice()
        .try_into()
        .map_err(|_| WalletError::StoreCorrupt)?;
    let txid: TxId = zcash_protocol::TxId::from_bytes(txid_bytes).into();
    let mined_height = raw.mined_height.map(BlockHeight::new);
    let status = if raw.expired_unmined {
        // Unmined past its expiry — funds returned to spendable (§6.1), a normal history row.
        TxStatus::Expired
    } else if let Some(h) = raw.mined_height {
        let depth = scanned_tip
            .map(|tip| tip.saturating_sub(h).saturating_add(1))
            .unwrap_or(1);
        TxStatus::Confirmed { depth }
    } else {
        TxStatus::Pending
    };
    // Validate-never-truncate: a delta/fee outside money bounds is corruption, surfaced
    // typed (`AmountOutOfRange`), never silently clamped.
    let net_amount = ZatBalance::new(raw.account_balance_delta)?;
    let fee = raw.fee_paid.map(Zatoshis::new).transpose()?;
    Ok(TxSummary {
        txid,
        batch_id: None,
        mined_height,
        status,
        net_amount,
        fee,
        has_memo: raw.memo_count > 0,
        has_transparent_output: raw.has_transparent_output,
        // Display-only unix seconds; clamp a (impossible) negative to 0 rather than wrap.
        timestamp: raw.block_time.map(|t| t.max(0) as u64),
        // The delivery reading is not the view's to give: `Wallet::transactions`
        // annotates the page from `crate::delivery` on the same connection.
        delivery: None,
        // `0` is "never expires": no height resolves such an outcome, so none is named.
        expiry_height: raw
            .created_expiry_height
            .filter(|&h| h > 0)
            .map(BlockHeight::new),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        mined_height: Option<u32>,
        delta: i64,
        fee: Option<i64>,
        memo_count: i64,
        block_time: Option<i64>,
        expired_unmined: bool,
    ) -> RawTxRow {
        RawTxRow {
            txid: (0..32).map(|i| i as u8).collect(),
            mined_height,
            account_balance_delta: delta,
            fee_paid: fee,
            memo_count,
            block_time,
            expired_unmined,
            has_transparent_output: false,
            created_expiry_height: None,
        }
    }

    #[test]
    fn map_row_reads_a_never_expiring_send_as_no_resolution_height() {
        // Upstream stores `0` for a tx built with no expiry: no block resolves its outcome,
        // so the row names none rather than a height every tip is already past.
        let mut never = row(None, -50_000, Some(1_000), 0, None, false);
        never.created_expiry_height = Some(0);
        assert_eq!(map_row(never, Some(100)).expect("map").expiry_height, None);
        let mut expiring = row(None, -50_000, Some(1_000), 0, None, false);
        expiring.created_expiry_height = Some(140);
        assert_eq!(
            map_row(expiring, Some(100)).expect("map").expiry_height,
            Some(BlockHeight::new(140))
        );
    }

    #[test]
    fn map_row_carries_the_transparent_output_flag_through() {
        // The §3.2i-3 transparency classifier is computed in SQL (the TX_COLUMNS EXISTS);
        // map_row must pass it through verbatim — both ways.
        let mut flagged = row(Some(100), -50_000, Some(1_000), 0, None, false);
        flagged.has_transparent_output = true;
        assert!(
            map_row(flagged, Some(100))
                .expect("map")
                .has_transparent_output
        );
        let plain = row(Some(100), -50_000, Some(1_000), 0, None, false);
        assert!(
            !map_row(plain, Some(100))
                .expect("map")
                .has_transparent_output
        );
    }

    #[test]
    fn map_row_confirmed_incoming_with_depth_and_memo() {
        // A received tx (positive net, no fee, a memo), mined 4 blocks below a tip at 1000 →
        // depth 5; the txid is reversed from internal to display order.
        let s = map_row(
            row(Some(996), 250_000, None, 1, Some(1_700_000_000), false),
            Some(1000),
        )
        .expect("map");
        assert_eq!(s.status, TxStatus::Confirmed { depth: 5 });
        assert_eq!(s.net_amount.zat(), 250_000, "incoming is positive");
        assert_eq!(s.fee, None, "a received tx carries no fee we paid");
        assert!(s.has_memo);
        assert_eq!(s.mined_height, Some(BlockHeight::new(996)));
        assert_eq!(s.timestamp, Some(1_700_000_000));
        // internal 00..1f reversed → display starts 1f
        assert!(s.txid.to_string().starts_with("1f"));
    }

    #[test]
    fn map_row_confirmed_outgoing_is_negative_with_fee() {
        // A sent tx: negative net effect, a fee, no memo.
        let s = map_row(
            row(
                Some(900),
                -1_010_000,
                Some(10_000),
                0,
                Some(1_699_000_000),
                false,
            ),
            Some(950),
        )
        .expect("map");
        assert_eq!(s.status, TxStatus::Confirmed { depth: 51 });
        assert_eq!(s.net_amount.zat(), -1_010_000, "outgoing is negative");
        assert_eq!(s.fee.map(Zatoshis::zat), Some(10_000));
        assert!(!s.has_memo);
    }

    #[test]
    fn map_row_pending_when_unmined_and_not_expired() {
        let s = map_row(row(None, 90_000, None, 0, None, false), Some(1000)).expect("map");
        assert_eq!(s.status, TxStatus::Pending);
        assert_eq!(s.mined_height, None);
        assert_eq!(s.timestamp, None, "no block time for an unmined tx");
    }

    #[test]
    fn map_row_expired_takes_precedence_over_pending() {
        // expired_unmined wins even though mined_height is None (it would otherwise be Pending).
        let s = map_row(row(None, 0, None, 0, None, true), Some(1000)).expect("map");
        assert_eq!(s.status, TxStatus::Expired);
    }

    #[test]
    fn read_row_treats_a_null_expired_unmined_as_not_expired() {
        // The REAL `v_transactions` view yields `expired_unmined = NULL` for an UN-MINED
        // RECEIVED tx: it has no `expiry_height`, so the view's
        // `(mined_height IS NULL AND expiry_height BETWEEN 1 AND max)` collapses under
        // SQLite three-valued logic to NULL, not 0/1 (a payment seen before it confirms, or
        // a reorg-orphaned receive). Reading column 6 as a bare `i64` errors the ENTIRE page
        // to `StoreCorrupt` — the whole history would vanish behind an error on one such row.
        // `read_row` must read it as `Option` and map NULL → not-expired (the row is Pending).
        // This pins the fix at the read layer; the funded-harness reorg test exercises it
        // against the real view end-to-end.
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE r (txid BLOB, mined_height INTEGER, account_balance_delta INTEGER, \
             fee_paid INTEGER, memo_count INTEGER, block_time INTEGER, expired_unmined INTEGER, \
             has_transparent_output INTEGER, created_expiry_height INTEGER); \
             INSERT INTO r VALUES (zeroblob(32), NULL, 123000, NULL, 0, NULL, NULL, NULL, NULL);",
        )
        .expect("fixture");
        let raw = conn
            .query_row(
                "SELECT txid, mined_height, account_balance_delta, fee_paid, memo_count, \
                 block_time, expired_unmined, has_transparent_output, created_expiry_height \
                 FROM r",
                [],
                read_row,
            )
            .expect("read_row must not error on a NULL expired_unmined");
        assert!(
            !raw.expired_unmined,
            "a NULL expired_unmined reads as NOT expired"
        );
        // Col 7 (the transparency flag) gets the same NULL tolerance: EXISTS can't yield
        // NULL, but if it ever did, ONE flag degrades — never the whole page to an error.
        assert!(
            !raw.has_transparent_output,
            "a NULL flag reads as un-flagged, not an error"
        );
        let s = map_row(raw, Some(1000)).expect("map");
        assert_eq!(
            s.status,
            TxStatus::Pending,
            "un-mined + not-expired ⇒ Pending, never an error"
        );
        assert_eq!(s.net_amount.zat(), 123_000, "the received amount is intact");
    }

    #[test]
    fn map_row_depth_is_one_when_no_blocks_scanned() {
        // No scanned blocks (tip None) → depth floors to 1 (it is at least in a known block).
        let s = map_row(row(Some(500), 1, None, 0, None, false), None).expect("map");
        assert_eq!(s.status, TxStatus::Confirmed { depth: 1 });
    }

    #[test]
    fn map_row_depth_floors_to_one_when_tip_below_mined_height() {
        // tip below mined_height (can't happen, but saturating) → depth 1, never underflow.
        let s = map_row(row(Some(500), 1, None, 0, None, false), Some(400)).expect("map");
        assert_eq!(s.status, TxStatus::Confirmed { depth: 1 });
    }

    #[test]
    fn map_row_rejects_an_out_of_range_delta_typed() {
        // A delta beyond max supply is corruption — surfaced typed, never clamped.
        let bad = map_row(row(Some(1), i64::MAX, None, 0, None, false), Some(1));
        assert!(matches!(bad, Err(WalletError::AmountOutOfRange)));
    }

    #[test]
    fn map_row_rejects_a_non_32_byte_txid_typed() {
        // A wrong-length txid blob is DB corruption — fail typed `StoreCorrupt`, never
        // zero-fill into a phantom all-zero id (rust-patterns: propagate, never default).
        let mut bad = row(Some(1), 1, None, 0, None, false);
        bad.txid = vec![0xab; 31]; // one byte short
        assert!(matches!(
            map_row(bad, Some(1)),
            Err(WalletError::StoreCorrupt)
        ));
    }

    #[test]
    fn queries_run_against_the_real_schema_and_are_empty_on_a_fresh_db() {
        // The list + lookup SQL must COMPILE + RUN against librustzcash's REAL
        // `v_transactions` view (a column-name typo — `tx_index`, `expired_unmined`,
        // `account_balance_delta`, … — would fail HERE, not silently in production). A
        // freshly provisioned, account-less DB has no history, so every shape is empty.
        use crate::db::{open_existing_keyed_connection, provision_db};
        use crate::money::Network;
        use crate::seal::WalletDbKey;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let key = WalletDbKey::generate();
        provision_db(&path, &key, Network::Main, None).expect("provision");
        let conn = open_existing_keyed_connection(&path, &key).expect("open");

        let first = list_transactions(&conn, 50, None).expect("list runs");
        assert!(first.rows.is_empty(), "a fresh DB has no history");
        assert!(first.next.is_none(), "no next page on an empty DB");
        // A cursor page on an empty DB also COMPILES + RUNS + is empty (the keyset WHERE
        // fragment must be valid against the real view).
        let cursor = HistoryCursor {
            mined_height: Some(100),
            txid_internal: [0u8; 32],
        }
        .encode();
        let paged = list_transactions(&conn, 10, Some(&cursor)).expect("paged list runs");
        assert!(
            paged.rows.is_empty(),
            "the cursor page also runs + is empty"
        );
        let txid = TxId::from_display_order([7u8; 32]);
        assert!(
            transaction_by_txid(&conn, &txid)
                .expect("lookup runs")
                .is_none(),
            "an unknown txid yields None, not an error"
        );
        // The memo read must COMPILE + RUN against the REAL `v_tx_outputs` view (a column-name
        // typo — `to_account_uuid`/`from_account_uuid`/`memo`/`output_index` — would fail HERE).
        // A fresh DB has no outputs ⇒ no memos.
        assert!(
            transaction_memos(&conn, &txid)
                .expect("memo read runs")
                .memos
                .is_empty(),
            "a fresh DB has no memos"
        );
    }

    #[test]
    fn transaction_memos_parses_filters_empty_and_scopes_to_the_primary_account() {
        // The memo-content read LOGIC against a CONTROLLED `v_tx_outputs`-shaped fixture (the
        // real-schema test above pins the column NAMES): a received text memo + our own outgoing
        // memo surface in output order; an EMPTY memo is filtered (no message); another account's
        // memo is excluded; machine bytes classify as Arbitrary. The blobs are built through the
        // SAME audited ZIP-302 wire codec the reader parses.
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, uuid BLOB);
             CREATE TABLE v_tx_outputs (txid BLOB, output_index INTEGER, output_pool INTEGER, \
                to_account_uuid BLOB, from_account_uuid BLOB, memo BLOB);",
        )
        .expect("fixture schema");
        let acct_a = vec![0xAAu8; 16];
        let acct_b = vec![0xBBu8; 16];
        conn.execute(
            "INSERT INTO accounts (id, uuid) VALUES (1, ?1), (2, ?2)",
            params![acct_a, acct_b],
        )
        .expect("accounts");

        // Store the memo blob the way librustzcash does — TRIMMED of trailing zeros
        // (`MemoBytes::as_slice`), NOT the full 512-byte array — so the read path's re-pad via
        // `from_bytes` is actually exercised (empty ⇒ a single `0xF6` byte; "gm ☕" ⇒ 6 bytes;
        // arbitrary([1,2,3]) ⇒ `0xFF`+3 = 4 bytes). A regression to a strict `len == 512` guard
        // would break against these real shapes — which a full-array fixture would have hidden.
        let wire = |m: Memo| m.to_wire().expect("sendable").as_slice().to_vec();
        let text = wire(Memo::text("gm ☕").expect("text"));
        let empty = wire(Memo::Empty);
        assert_eq!(
            empty,
            vec![0xF6],
            "the empty memo stores as a single 0xF6 byte (the real shape)"
        );
        let arb = wire(Memo::arbitrary(vec![1, 2, 3]).expect("arb"));
        assert!(
            arb.len() < 512,
            "a short arbitrary memo stores trimmed, not zero-padded to 512"
        );
        let txid_internal = vec![0x11u8; 32]; // all-same ⇒ display == internal (byte order is
        // covered by transaction_by_txid's distinct-byte test)
        let ins = "INSERT INTO v_tx_outputs (txid, output_index, output_pool, to_account_uuid, \
                   from_account_uuid, memo) VALUES (?1,?2,?3,?4,?5,?6)";
        let none = Option::<Vec<u8>>::None;
        let pool = 2i64; // sapling — the pool value is immaterial here (distinct output_index)
        // out 0: received text (to us) · out 1: empty (filtered) · out 2: our outgoing arbitrary
        // (from us) · out 3: account B's memo (excluded). Inserted out of order to test ORDER BY.
        conn.execute(ins, params![txid_internal, 2i64, pool, none, acct_a, arb])
            .expect("o2");
        conn.execute(ins, params![txid_internal, 0i64, pool, acct_a, none, text])
            .expect("o0");
        conn.execute(ins, params![txid_internal, 1i64, pool, acct_a, none, empty])
            .expect("o1");
        conn.execute(
            ins,
            params![
                txid_internal,
                0i64,
                pool,
                acct_b,
                none,
                wire(Memo::text("nope").unwrap())
            ],
        )
        .expect("acctB");

        let read = transaction_memos(&conn, &TxId::from_display_order([0x11u8; 32])).expect("read");
        assert_eq!(
            read.memos.len(),
            2,
            "empty filtered + account B excluded ⇒ two memos"
        );
        assert_eq!(
            read.unreadable, 0,
            "every memo here classifies — the count is 0 on the ordinary path, \
             so a non-zero can only mean the hostile shape"
        );
        assert_eq!(
            read.memos[0],
            Memo::text("gm ☕").unwrap(),
            "received text first (output_index 0)"
        );
        assert!(
            matches!(read.memos[1], Memo::Arbitrary(_)),
            "our outgoing machine memo second (index 2)"
        );

        // An unknown txid ⇒ empty, never an error.
        let missing =
            transaction_memos(&conn, &TxId::from_display_order([0x99u8; 32])).expect("read");
        assert!(missing.memos.is_empty());
        assert_eq!(missing.unreadable, 0);
    }

    #[test]
    fn machine_memo_lane_survives_a_sibling_memo_the_classifier_rejects() {
        // FR-27 review, found INDEPENDENTLY by the crypto audit and the
        // security review — the strongest signal a finding can carry.
        //
        // The party paying you chooses every byte of every memo in their own
        // transaction. ZIP-302 has exactly one upstream-invalid shape (a TEXT
        // lead whose payload is not valid UTF-8), and `v_tx_outputs.memo` is
        // stored unvalidated, so that shape is reachable on-chain for the price
        // of one dust output. Under the strict reader it denied the WHOLE
        // transaction — including the valid envelope sitting beside it, which
        // is the exact "the SDK helped write it onto a permanent ledger and
        // then made it unreadable" failure FR-27 exists to end, handed back to
        // a hostile counterparty.
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, uuid BLOB);
             CREATE TABLE v_tx_outputs (txid BLOB, output_index INTEGER, output_pool INTEGER, \
                to_account_uuid BLOB, from_account_uuid BLOB, memo BLOB);",
        )
        .expect("fixture schema");
        let wire = |m: Memo| m.to_wire().expect("sendable").as_slice().to_vec();
        let acct = vec![0xA1u8; 16];
        conn.execute(
            "INSERT INTO accounts (id, uuid) VALUES (1, ?1)",
            params![acct],
        )
        .expect("account");
        let mut internal = [0x11u8; 32];
        internal.reverse();
        let none: Option<Vec<u8>> = None;
        // output 0 — the POISON: a text lead (0x41 <= 0xF4) over non-UTF-8
        // bytes. Written as a raw blob, exactly as an unvalidated store holds
        // it; the audited classifier returns Err for this and only this shape.
        conn.execute(
            "INSERT INTO v_tx_outputs VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                internal.as_slice(),
                0i64,
                0i64,
                acct,
                none,
                vec![0x41u8, 0xFF, 0xFE]
            ],
        )
        .expect("poison row");
        // output 1 — the machine memo that must survive it.
        conn.execute(
            "INSERT INTO v_tx_outputs VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                internal.as_slice(),
                1i64,
                0i64,
                acct,
                none,
                wire(Memo::arbitrary(b"RLM\x01envelope".to_vec()).expect("machine memo"))
            ],
        )
        .expect("machine row");

        let txid = TxId::from_display_order([0x11u8; 32]);
        let read = transaction_memos(&conn, &txid).expect("the poison row is not fatal");

        // The readable memo survives the poison sitting beside it.
        assert_eq!(read.memos.len(), 1, "the poison row is skipped, not fatal");
        let Memo::Arbitrary(bytes) = &read.memos[0] else {
            panic!("expected the machine memo to survive");
        };
        assert_eq!(
            &bytes[..12],
            b"RLM\x01envelope",
            "and it survives BYTE-FOR-BYTE (the 0xFF field zero-pads to 511)",
        );

        // NOT SILENT — the whole reason this is a skip and not a drop. A caller
        // rendering to a human must be able to say "1 memo could not be read"
        // instead of presenting a short list as complete (principle 10).
        assert_eq!(
            read.unreadable, 1,
            "the skipped row is COUNTED, never silent"
        );

        // ONE readable set, both consumers. An earlier fold gave the display
        // lane a hard failure here and the machine lane a skip, which let a
        // payer make the HOST act on an envelope the USER could not see on the
        // same transaction — the FR-26 divergence class, on the read side
        // (FR-27 post-fold security review). There is now one reader.
        let via_verb = crate::memo::filter_machine_memos(
            transaction_memos(&conn, &txid).expect("same reader").memos,
            &[crate::memo::MachineMemoPrefix::new(b"RLM\x01".to_vec()).expect("prefix")],
        );
        assert_eq!(via_verb.len(), 1, "the machine lane reads the same set");
        assert_eq!(
            &via_verb[0][..12],
            b"RLM\x01envelope",
            "and gets the same bytes the display lane saw",
        );
    }

    #[test]
    fn transaction_memos_rejects_an_oversized_memo_blob_typed() {
        // gate-7 boundary: a stored memo blob > 512 bytes is genuine corruption (the ZIP-302
        // field is 512). `MemoBytes::from_bytes` rejects it; the reader fails typed `MemoInvalid`
        // — never truncates into a phantom memo. (A ≤512 trimmed blob is the NORM and re-pads
        // losslessly — proven by the parse/filter test above.)
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, uuid BLOB);
             CREATE TABLE v_tx_outputs (txid BLOB, output_index INTEGER, output_pool INTEGER, \
                to_account_uuid BLOB, from_account_uuid BLOB, memo BLOB);",
        )
        .expect("fixture schema");
        let acct = vec![0xAAu8; 16];
        conn.execute(
            "INSERT INTO accounts (id, uuid) VALUES (1, ?1)",
            params![acct],
        )
        .expect("account");
        let oversized = vec![0x01u8; 513]; // one byte past the ZIP-302 field
        conn.execute(
            "INSERT INTO v_tx_outputs (txid, output_index, output_pool, to_account_uuid, \
             from_account_uuid, memo) VALUES (?1,0,2,?2,NULL,?3)",
            params![vec![0x11u8; 32], acct, oversized],
        )
        .expect("insert oversized");
        assert!(
            matches!(
                transaction_memos(&conn, &TxId::from_display_order([0x11u8; 32])),
                Err(WalletError::MemoInvalid)
            ),
            "a > 512-byte memo blob is corruption — fail typed, never a phantom memo",
        );
    }

    #[test]
    fn transaction_memos_retains_a_reserved_memo() {
        // GAP-7: the ACCESSOR's empty-filter must drop ONLY the canonical empty memo
        // (`Memo::Empty`), never a ZIP-302 RESERVED memo (first byte 0xF5 / 0xF7..=0xFE, or a
        // non-canonical 0xF6 = the empty tag with a non-zero tail). A reserved memo from a newer
        // sender EXISTS on-chain; dropping it would lie (per `memo::from_wire`). The classification
        // itself is pinned in `memo.rs::memo_512_byte_boundary_invalid_utf8_and_reserved_range_handled`;
        // THIS pins that `transaction_memos` SURFACES it — i.e. the filter keys on the
        // classification (`Memo::Empty`), not a naive "first byte 0xF6 ⇒ drop".
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, uuid BLOB);
             CREATE TABLE v_tx_outputs (txid BLOB, output_index INTEGER, output_pool INTEGER, \
                to_account_uuid BLOB, from_account_uuid BLOB, memo BLOB);",
        )
        .expect("fixture schema");
        let acct = vec![0xAAu8; 16];
        conn.execute(
            "INSERT INTO accounts (id, uuid) VALUES (1, ?1)",
            params![acct],
        )
        .expect("account");
        let ins = "INSERT INTO v_tx_outputs (txid, output_index, output_pool, to_account_uuid, \
                   from_account_uuid, memo) VALUES (?1,?2,2,?3,NULL,?4)";
        let txid = vec![0x11u8; 32];
        // out 0: a reserved memo (first byte 0xF7, in 0xF5..=0xFE; stored TRIMMED to its tag byte).
        conn.execute(ins, params![txid, 0i64, acct, vec![0xF7u8]])
            .expect("reserved memo");
        // out 1: a NON-canonical 0xF6 — the empty tag with a non-zero tail. NOT the empty memo
        // (the empty arm requires 0xF6 + ALL zeros), so it must NOT be dropped. Stored trimmed.
        conn.execute(ins, params![txid, 1i64, acct, vec![0xF6u8, 0x01]])
            .expect("non-canonical 0xF6");

        let read = transaction_memos(&conn, &TxId::from_display_order([0x11u8; 32])).expect("read");
        assert_eq!(
            read.memos.len(),
            2,
            "both reserved memos are RETAINED, not filtered as empty"
        );
        assert_eq!(
            read.unreadable, 0,
            "RESERVED framing classifies fine — it is readable-but-not-renderable, \
             a different thing from the unclassifiable shape the count tracks"
        );
        assert!(
            matches!(read.memos[0], Memo::Reserved(_)),
            "0xF7 ⇒ Memo::Reserved — surfaced, not dropped",
        );
        assert!(
            matches!(read.memos[1], Memo::Reserved(_)),
            "a 0xF6 tag with a non-zero tail is NOT the empty memo ⇒ Reserved, never silently dropped",
        );
    }

    #[test]
    fn list_orders_pending_first_then_newest_and_paginates_by_height() {
        // The MAJOR pagination invariants — ORDER (pending above confirmed, then
        // newest-first), the `before` cursor, the primary-account scope, confirmation
        // depth, the signed money, AND the txid byte-order round-trip — tested against a
        // CONTROLLED `v_transactions`-shaped fixture. (The `queries_run_against_the_real_
        // schema…` test pins the column NAMES to librustzcash's real view; THIS test pins
        // the query LOGIC against rows we control — no proving/scan harness needed.)
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, uuid BLOB);
             CREATE TABLE blocks (height INTEGER);
             CREATE TABLE v_transactions (
                 account_uuid BLOB, txid BLOB, mined_height INTEGER, tx_index INTEGER,
                 account_balance_delta INTEGER, fee_paid INTEGER, memo_count INTEGER,
                 block_time INTEGER, expired_unmined INTEGER
             );
             CREATE TABLE v_tx_outputs (txid BLOB, output_pool INTEGER, is_change INTEGER);
             CREATE TABLE transactions (txid BLOB, expiry_height INTEGER, created TEXT);",
        )
        .expect("fixture schema");
        let acct_a = vec![0xAAu8; 16];
        let acct_b = vec![0xBBu8; 16];
        conn.execute(
            "INSERT INTO accounts (id, uuid) VALUES (1, ?1), (2, ?2)",
            params![acct_a, acct_b],
        )
        .expect("accounts");
        conn.execute_batch("INSERT INTO blocks (height) VALUES (100), (200);")
            .expect("blocks");

        // Distinct-byte INTERNAL txid for the looked-up row so the display reversal is
        // VISIBLE (an all-same-byte id would round-trip even with a broken reversal).
        let tx_high: Vec<u8> = (0..32).map(|i| i as u8).collect(); // mined 200
        let tx_low = vec![0x20u8; 32]; // mined 100
        let tx_pending = vec![0x30u8; 32]; // unmined
        let tx_other = vec![0x40u8; 32]; // account B — must be excluded
        let ins = "INSERT INTO v_transactions (account_uuid, txid, mined_height, tx_index, \
                   account_balance_delta, fee_paid, memo_count, block_time, expired_unmined) \
                   VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)";
        let none = Option::<i64>::None;
        conn.execute(
            ins,
            params![
                acct_a,
                tx_high,
                200i64,
                0i64,
                500_000i64,
                none,
                1i64,
                1_700_000_200i64,
                0i64
            ],
        )
        .expect("high");
        conn.execute(
            ins,
            params![
                acct_a,
                tx_low,
                100i64,
                0i64,
                -10_000i64,
                1_000i64,
                0i64,
                1_700_000_100i64,
                0i64
            ],
        )
        .expect("low");
        conn.execute(
            ins,
            params![
                acct_a, tx_pending, none, 0i64, 90_000i64, none, 0i64, none, 0i64
            ],
        )
        .expect("pending");
        conn.execute(
            ins,
            params![
                acct_b,
                tx_other,
                150i64,
                0i64,
                1i64,
                none,
                0i64,
                1_700_000_150i64,
                0i64
            ],
        )
        .expect("account B");

        // Page 1 (whole, limit 50): pending first, then newest confirmed; account B
        // excluded; fewer than the limit ⇒ no next page.
        let page1 = list_transactions(&conn, 50, None).expect("page1");
        assert_eq!(
            page1.rows.len(),
            3,
            "account B's tx is excluded — only A's three"
        );
        assert!(page1.next.is_none(), "all three fit ⇒ no next cursor");
        assert_eq!(
            page1.rows[0].status,
            TxStatus::Pending,
            "pending sorts first"
        );
        assert_eq!(
            page1.rows[1].mined_height,
            Some(BlockHeight::new(200)),
            "then newest confirmed"
        );
        assert_eq!(page1.rows[2].mined_height, Some(BlockHeight::new(100)));
        // depth uses MAX(blocks.height)=200: h200→1, h100→101.
        assert_eq!(page1.rows[1].status, TxStatus::Confirmed { depth: 1 });
        assert_eq!(page1.rows[2].status, TxStatus::Confirmed { depth: 101 });
        // signed money + fee carried through.
        assert_eq!(page1.rows[1].net_amount.zat(), 500_000, "incoming positive");
        assert_eq!(page1.rows[2].net_amount.zat(), -10_000, "outgoing negative");
        assert_eq!(page1.rows[2].fee.map(Zatoshis::zat), Some(1_000));

        // KEYSET pagination across the cursor: limit 2 ⇒ [pending, h200] + a next cursor;
        // following it ⇒ [h100] + no next. The two pages union to all three rows, once each.
        let p1 = list_transactions(&conn, 2, None).expect("keyset p1");
        assert_eq!(p1.rows.len(), 2);
        assert_eq!(p1.rows[0].status, TxStatus::Pending);
        assert_eq!(p1.rows[1].mined_height, Some(BlockHeight::new(200)));
        let cursor = p1.next.expect("a third row remains ⇒ a next cursor");
        let p2 = list_transactions(&conn, 2, Some(&cursor)).expect("keyset p2");
        assert_eq!(p2.rows.len(), 1, "only h100 remains after the cursor");
        assert_eq!(p2.rows[0].mined_height, Some(BlockHeight::new(100)));
        assert!(p2.next.is_none(), "last page ⇒ no further cursor");

        // limit caps the page (pending is the single newest row) and yields a next cursor.
        let one = list_transactions(&conn, 1, None).expect("limit 1");
        assert_eq!(one.rows.len(), 1);
        assert_eq!(one.rows[0].status, TxStatus::Pending);
        assert!(one.next.is_some(), "more rows remain ⇒ a cursor");

        // txid byte-order round-trip: the list emits DISPLAY order; looking up BY that
        // display-order txid reverses back to internal and finds the SAME row.
        let high_display = page1.rows[1].txid.clone();
        assert!(
            high_display.to_string().starts_with("1f"),
            "internal 00..1f → display 1f..00"
        );
        let found = transaction_by_txid(&conn, &high_display)
            .expect("lookup runs")
            .expect("the looked-up tx exists");
        assert_eq!(found.mined_height, Some(BlockHeight::new(200)));
        assert_eq!(
            found.txid, high_display,
            "round-trips to the SAME display-order txid"
        );
        // a txid we never inserted → None (not an error).
        let missing = TxId::from_display_order([0x99u8; 32]);
        assert!(
            transaction_by_txid(&conn, &missing)
                .expect("lookup runs")
                .is_none()
        );
    }

    #[test]
    fn list_clamps_limit_to_the_max_page_size() {
        // gate-7 boundary: `limit` is host-supplied and flows straight to SQL `LIMIT`, so
        // it MUST clamp to MAX_TX_PAGE_SIZE — else a huge value materializes an unbounded
        // Vec<TxSummary> and pressures mobile memory on a large history. Fill JUST over the
        // cap and prove a huge limit returns exactly the ceiling; a small limit caps below.
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, uuid BLOB);
             CREATE TABLE blocks (height INTEGER);
             CREATE TABLE v_transactions (
                 account_uuid BLOB, txid BLOB, mined_height INTEGER, tx_index INTEGER,
                 account_balance_delta INTEGER, fee_paid INTEGER, memo_count INTEGER,
                 block_time INTEGER, expired_unmined INTEGER
             );
             CREATE TABLE v_tx_outputs (txid BLOB, output_pool INTEGER, is_change INTEGER);
             CREATE TABLE transactions (txid BLOB, expiry_height INTEGER, created TEXT);
             INSERT INTO blocks (height) VALUES (1000);",
        )
        .expect("fixture schema");
        let acct = vec![0xAAu8; 16];
        conn.execute(
            "INSERT INTO accounts (id, uuid) VALUES (1, ?1)",
            params![acct],
        )
        .expect("account");
        // MAX_TX_PAGE_SIZE + 50 distinct rows (in-memory, so unbatched inserts are fine).
        let total = MAX_TX_PAGE_SIZE + 50;
        for i in 0..total {
            let mut txid = vec![0u8; 32];
            txid[..4].copy_from_slice(&i.to_le_bytes());
            conn.execute(
                "INSERT INTO v_transactions (account_uuid, txid, mined_height, tx_index, \
                 account_balance_delta, fee_paid, memo_count, block_time, expired_unmined) \
                 VALUES (?1, ?2, 1000, ?3, 1, NULL, 0, 1700000000, 0)",
                params![acct, txid, i as i64],
            )
            .expect("insert row");
        }

        assert_eq!(
            list_transactions(&conn, u32::MAX, None)
                .expect("clamped")
                .rows
                .len(),
            MAX_TX_PAGE_SIZE as usize,
            "a huge limit clamps to MAX_TX_PAGE_SIZE, never the full set",
        );
        assert_eq!(
            list_transactions(&conn, MAX_TX_PAGE_SIZE, None)
                .expect("exact")
                .rows
                .len(),
            MAX_TX_PAGE_SIZE as usize,
            "exactly the ceiling returns the ceiling",
        );
        assert_eq!(
            list_transactions(&conn, 2, None).expect("small").rows.len(),
            2,
            "a small limit still caps below the ceiling",
        );
    }

    /// Build a v_transactions-shaped fixture DB with one account + a scanned tip; the
    /// caller inserts rows. Shared by the keyset tests below.
    fn keyset_fixture(tip: i64) -> (Connection, Vec<u8>) {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, uuid BLOB);
             CREATE TABLE blocks (height INTEGER);
             CREATE TABLE v_transactions (
                 account_uuid BLOB, txid BLOB, mined_height INTEGER, tx_index INTEGER,
                 account_balance_delta INTEGER, fee_paid INTEGER, memo_count INTEGER,
                 block_time INTEGER, expired_unmined INTEGER
             );
             CREATE TABLE v_tx_outputs (txid BLOB, output_pool INTEGER, is_change INTEGER);
             CREATE TABLE transactions (txid BLOB, expiry_height INTEGER, created TEXT);",
        )
        .expect("fixture schema");
        let acct = vec![0xAAu8; 16];
        conn.execute(
            "INSERT INTO accounts (id, uuid) VALUES (1, ?1)",
            params![acct],
        )
        .expect("account");
        conn.execute("INSERT INTO blocks (height) VALUES (?1)", params![tip])
            .expect("tip block");
        (conn, acct)
    }

    /// The §3.2i-3 transparency classifier semantics, against a CONTROLLED
    /// `v_tx_outputs`-shaped fixture (the real-schema test pins the column NAMES; this
    /// pins the LOGIC): a non-change transparent output flags the tx; shielded outputs
    /// (pool 2/3) never flag; transparent CHANGE (is_change=1) does not flag; a NULL
    /// `is_change` on a transparent output DOES flag (errs toward honest disclosure).
    /// The single-txid lookup carries the same flag.
    #[test]
    fn list_flags_non_change_transparent_outputs_only() {
        let (conn, acct) = keyset_fixture(1000);
        let tx_deshield = vec![0x51u8; 32]; // transparent recipient output → flagged
        let tx_shielded = vec![0x52u8; 32]; // orchard output only → not flagged
        let tx_t_change = vec![0x53u8; 32]; // transparent output but is_change=1 → not flagged
        let tx_null_chg = vec![0x54u8; 32]; // transparent output, is_change NULL → flagged
        insert_tx(&conn, &acct, &tx_deshield, Some(500), None);
        insert_tx(&conn, &acct, &tx_shielded, Some(501), None);
        insert_tx(&conn, &acct, &tx_t_change, Some(502), None);
        insert_tx(&conn, &acct, &tx_null_chg, Some(503), None);
        let out = "INSERT INTO v_tx_outputs (txid, output_pool, is_change) VALUES (?1, ?2, ?3)";
        conn.execute(out, params![tx_deshield, 0i64, 0i64])
            .expect("deshield out");
        conn.execute(out, params![tx_shielded, 3i64, 0i64])
            .expect("orchard out");
        conn.execute(out, params![tx_t_change, 0i64, 1i64])
            .expect("t-change out");
        conn.execute(out, params![tx_null_chg, 0i64, Option::<i64>::None])
            .expect("null-chg out");

        let page = list_transactions(&conn, 50, None).expect("list");
        assert_eq!(page.rows.len(), 4);
        // TX_ORDER is newest-first: 503, 502, 501, 500.
        assert!(
            page.rows[0].has_transparent_output,
            "NULL is_change transparent ⇒ flagged"
        );
        assert!(
            !page.rows[1].has_transparent_output,
            "transparent CHANGE ⇒ not flagged"
        );
        assert!(
            !page.rows[2].has_transparent_output,
            "shielded-only ⇒ not flagged"
        );
        assert!(
            page.rows[3].has_transparent_output,
            "non-change transparent ⇒ flagged"
        );
        // The txid-keyed lookup reads the SAME classifier.
        let looked = transaction_by_txid(&conn, &page.rows[3].txid)
            .expect("lookup runs")
            .expect("row exists");
        assert!(looked.has_transparent_output, "lookup carries the flag");
    }

    fn insert_tx(
        conn: &Connection,
        acct: &[u8],
        txid: &[u8],
        mined_height: Option<i64>,
        tx_index: Option<i64>,
    ) {
        conn.execute(
            "INSERT INTO v_transactions (account_uuid, txid, mined_height, tx_index, \
             account_balance_delta, fee_paid, memo_count, block_time, expired_unmined) \
             VALUES (?1, ?2, ?3, ?4, 1, NULL, 0, 1700000000, 0)",
            params![acct, txid, mined_height, tx_index],
        )
        .expect("insert tx");
    }

    /// THE keyset correctness proof (the deep-e2e P0 — the height-only-cursor SEAM closed).
    /// A run of txs at the SAME `mined_height` straddles a page boundary; the union of all
    /// keyset pages must be EVERY row exactly once — no drop (the old `before = h` bug
    /// hid the same-height remainder), no duplicate. Also covers a page of all-pending rows.
    #[test]
    fn keyset_pages_union_to_every_row_once_even_with_same_height() {
        let (conn, acct) = keyset_fixture(1000);
        // 5 txs ALL mined at height 500 (an exchange paying 5 notes in one block) +
        // 2 pending. Distinct txids so we can prove the union by identity.
        let mut expected: Vec<Vec<u8>> = Vec::new();
        for idx in 0..5i64 {
            let txid = vec![(0x10 + idx) as u8; 32];
            insert_tx(&conn, &acct, &txid, Some(500), Some(idx));
            expected.push(txid);
        }
        for n in 0..2u8 {
            let txid = vec![0x80 + n; 32];
            insert_tx(&conn, &acct, &txid, None, None);
            expected.push(txid);
        }

        // Walk EVERY page at limit 2 (so same-height rows straddle boundaries), following
        // the cursor, and collect the (display-order) txids seen.
        let mut seen: Vec<String> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut guard = 0;
        loop {
            let page = list_transactions(&conn, 2, cursor.as_deref()).expect("page");
            for r in &page.rows {
                seen.push(r.txid.to_string());
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => break,
            }
            guard += 1;
            assert!(guard < 100, "pagination must terminate");
        }

        // Every inserted row appears EXACTLY once (no same-height drop, no duplicate).
        assert_eq!(seen.len(), 7, "all 7 rows surfaced across the pages");
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 7, "no row paginated twice");
        for txid_internal in &expected {
            // The list emits DISPLAY order (reversed); reverse our internal fixture id.
            let mut disp = txid_internal.clone();
            disp.reverse();
            let display = TxId::from_display_order(disp.try_into().unwrap()).to_string();
            assert!(seen.contains(&display), "row {display} must appear once");
        }
    }

    #[test]
    fn keyset_pending_only_page_still_reaches_the_confirmed_history() {
        // The other half of the SEAM: MORE pending than a page. The old height-only cursor
        // (`before = NULL height`) could never page PAST the pending rows, stranding the
        // confirmed history. The keyset pending-cursor (txid tiebreaker) reaches it.
        let (conn, acct) = keyset_fixture(1000);
        for n in 0..3u8 {
            insert_tx(&conn, &acct, &[0x90 + n; 32], None, None); // pending
        }
        insert_tx(&conn, &acct, &[0x40u8; 32], Some(300), Some(0)); // confirmed, behind

        // Page at limit 2 through the pending rows; the confirmed row MUST eventually show.
        let mut reached_confirmed = false;
        let mut cursor: Option<String> = None;
        let mut guard = 0;
        loop {
            let page = list_transactions(&conn, 2, cursor.as_deref()).expect("page");
            if page
                .rows
                .iter()
                .any(|r| r.mined_height == Some(BlockHeight::new(300)))
            {
                reached_confirmed = true;
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => break,
            }
            guard += 1;
            assert!(guard < 100, "pagination must terminate");
        }
        assert!(
            reached_confirmed,
            "the confirmed row behind a full pending page is reachable"
        );
    }

    #[test]
    fn cursor_round_trips_and_rejects_malformed_tokens() {
        // The opaque cursor is host-round-tripped; a token we never emitted must fail typed
        // (StoreCorrupt), never silently page from the wrong place (a money-visibility risk),
        // and NEVER panic (it decodes host-supplied input under the aux-db lock).
        let mined = HistoryCursor {
            mined_height: Some(500),
            txid_internal: [7u8; 32],
        };
        assert_eq!(
            HistoryCursor::decode(&mined.encode()).expect("round-trips"),
            mined
        );
        let pending = HistoryCursor {
            mined_height: None,
            txid_internal: [9u8; 32],
        };
        assert_eq!(
            HistoryCursor::decode(&pending.encode()).expect("round-trips"),
            pending
        );

        let hex64 = "0".repeat(64);
        for bad in [
            "".to_string(),                       // empty
            "v1:500".to_string(),                 // too few parts
            format!("v2:500:{hex64}"),            // wrong version
            format!("500:{hex64}"),               // missing version
            format!("v1:x:{hex64}"),              // non-numeric height
            "v1:500:zz".to_string(),              // non-hex txid
            format!("v1:500:{}", "0".repeat(63)), // wrong hex length (short)
            format!("v1:500:{}", "0".repeat(66)), // wrong hex length (long)
            // 64-BYTE hex containing a multibyte char — the byte-length check passes, so a
            // bare slice would PANIC on the non-char-boundary; the is_ascii guard rejects it.
            format!("v1:500:a\u{e9}{}", "b".repeat(61)),
            "v1:9999999999999999999:".to_string() + &hex64, // height overflows u32
            "x".repeat(CURSOR_MAX_LEN + 1),                 // over the size cap
        ] {
            assert!(
                matches!(HistoryCursor::decode(&bad), Err(WalletError::StoreCorrupt)),
                "malformed cursor {bad:?} must fail typed (not panic)",
            );
        }
    }

    #[test]
    fn cursor_decode_never_panics_on_adversarial_strings() {
        // A poor-man's fuzz (no proptest dep here): decode MUST return Result on every input
        // — a panic would run under the aux-db lock and poison it (a session-wide brick). Mix
        // multibyte chars, colons, near-valid shapes, and oversize at every even/odd boundary.
        let mut cases: Vec<String> = vec![
            String::new(),
            ":".to_string(),
            "::::".to_string(),
            "v1::".to_string(),
            "v1:0:".to_string(),
            "\u{e9}".repeat(40),
            "v1:0:\u{e9}".repeat(30),
        ];
        for n in 0..130usize {
            cases.push("é".repeat(n)); // multibyte at every length (incl. 64-byte regions)
            cases.push(format!("v1:1:{}", "z".repeat(n)));
            cases.push(format!("v1:{}:{}", "9".repeat(n), "a".repeat(n)));
        }
        for c in cases {
            // The ONLY contract: it returns (Ok or Err), never unwinds.
            let _ = HistoryCursor::decode(&c);
        }
    }

    #[test]
    fn keyset_surfaces_mined_rows_with_null_tx_index_no_drop() {
        // REGRESSION (the BLOCKER): librustzcash's `set_transaction_status(Mined)` fills
        // `mined_height` but leaves `tx_index` NULL — the common exchange-withdrawal /
        // swap-in-deposit case. A cursor keyed on `tx_index` would make these rows
        // UNREACHABLE on later pages (NULL comparisons are never true), silently hiding
        // received funds. Insert same-height rows with NULL tx_index straddling page
        // boundaries and prove every one surfaces exactly once.
        let (conn, acct) = keyset_fixture(1000);
        let mut expected: Vec<Vec<u8>> = Vec::new();
        for idx in 0..6u8 {
            let txid = vec![0x10 + idx; 32];
            // ALL mined at height 500, ALL with NULL tx_index (the GetStatus shape).
            insert_tx(&conn, &acct, &txid, Some(500), None);
            expected.push(txid);
        }

        let mut seen: Vec<String> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut guard = 0;
        loop {
            let page = list_transactions(&conn, 2, cursor.as_deref()).expect("page");
            for r in &page.rows {
                seen.push(r.txid.to_string());
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => break,
            }
            guard += 1;
            assert!(guard < 100, "pagination must terminate");
        }

        assert_eq!(seen.len(), 6, "every NULL-tx_index mined row surfaced");
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 6, "no row paginated twice");
    }

    #[test]
    fn keyset_exact_multiple_of_page_size_has_no_trailing_empty_page() {
        // BOUNDARY on the `+1` peek (gate-7): when the history length is an EXACT
        // multiple of the page size, the FINAL full page must report `next: None`
        // — never a dangling cursor whose follow-up fetch is empty. The union
        // tests use ODD totals (the last page returns FEWER than `limit`, hitting
        // the `len < limit` arm); the `len == limit` boundary on a TERMINAL page
        // (peek fetches limit+1, only limit exist ⇒ `len > limit` is false) is
        // otherwise unexercised. An off-by-one here (`>=` for `>`) would surface a
        // "Load more" affordance that pages into nothing on every full history —
        // a recurring dead-end on the most common page-aligned case. 6 same-height
        // NULL-tx_index rows at limit 3 ⇒ exactly two full pages, the second
        // terminal, AND lands the page boundary inside a same-height run.
        let (conn, acct) = keyset_fixture(1000);
        for idx in 0..6u8 {
            insert_tx(&conn, &acct, &[0x10 + idx; 32], Some(500), None);
        }

        let mut pages = 0;
        let mut seen = 0;
        let mut cursor: Option<String> = None;
        loop {
            let page = list_transactions(&conn, 3, cursor.as_deref()).expect("page");
            assert!(
                !page.rows.is_empty(),
                "a non-null cursor must never lead to an empty page",
            );
            pages += 1;
            seen += page.rows.len();
            match page.next {
                Some(c) => cursor = Some(c),
                None => break,
            }
            assert!(pages < 10, "pagination must terminate");
        }
        assert_eq!(
            pages, 2,
            "6 rows / page 3 = exactly two pages, no trailing empty"
        );
        assert_eq!(
            seen, 6,
            "every row surfaced once across the exact-multiple walk"
        );
    }

    #[test]
    fn keyset_walks_pending_plus_null_tx_index_batch_plus_normal_in_order() {
        // The realistic exchange scenario with ALL three row kinds in ONE walk: 2
        // PENDING sends, a 3-note same-height NULL-tx_index deposit batch (the
        // `set_transaction_status(Mined)` / GetStatus shape — exchange withdrawal /
        // swap-in), and 2 normal distinct-height confirmed rows. The separate tests
        // pin each kind alone; this pins their INTERACTION through the keyset cursor
        // — the union is every row once AND the global TX_ORDER (pending first,
        // then mined newest-first) holds across the kinds, with a page boundary
        // landing INSIDE the NULL-tx_index batch (the kind most at risk of being
        // stranded). A reorder/drop here would scramble or hide an exchange deposit.
        let (conn, acct) = keyset_fixture(1000);
        let mut expected = 0;
        for n in 0..2u8 {
            insert_tx(&conn, &acct, &[0xA0 + n; 32], None, None); // pending
            expected += 1;
        }
        for n in 0..3u8 {
            insert_tx(&conn, &acct, &[0x50 + n; 32], Some(500), None); // batch, NULL tx_index
            expected += 1;
        }
        insert_tx(&conn, &acct, &[0x60u8; 32], Some(600), Some(0)); // normal, newer
        insert_tx(&conn, &acct, &[0x40u8; 32], Some(400), Some(0)); // normal, older
        expected += 2;

        // Walk at limit 2 so a boundary straddles the same-height batch; record
        // (is_pending, height) in walk order plus the txid set for the union check.
        let mut walk: Vec<(bool, Option<u32>)> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut guard = 0;
        loop {
            let page = list_transactions(&conn, 2, cursor.as_deref()).expect("page");
            for r in &page.rows {
                seen.push(r.txid.to_string());
                walk.push((
                    matches!(r.status, TxStatus::Pending),
                    r.mined_height.map(BlockHeight::value),
                ));
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => break,
            }
            guard += 1;
            assert!(guard < 100, "pagination must terminate");
        }

        // Union: every inserted row exactly once.
        assert_eq!(seen.len(), expected, "every row of every kind surfaced");
        let mut uniq = seen.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), expected, "no row paginated twice");

        // Order: all pending precede all mined; no mined-then-pending inversion.
        let first_mined = walk
            .iter()
            .position(|(pending, _)| !*pending)
            .expect("there are mined rows");
        assert!(
            walk[..first_mined].iter().all(|(pending, _)| *pending),
            "pending rows sort strictly first",
        );
        assert!(
            walk[first_mined..].iter().all(|(pending, _)| !*pending),
            "no pending row appears after a mined row",
        );
        // Order: mined heights are non-increasing (newest-first) across the walk,
        // so the NULL-tx_index batch stays grouped between 600 and 400.
        let heights: Vec<u32> = walk[first_mined..]
            .iter()
            .map(|(_, h)| h.expect("a mined row has a height"))
            .collect();
        assert!(
            heights.windows(2).all(|w| w[0] >= w[1]),
            "mined rows stay newest-first across the walk: {heights:?}",
        );
        // The whole 3-note batch surfaced contiguously (none stranded by a boundary).
        assert_eq!(
            heights.iter().filter(|&&h| h == 500).count(),
            3,
            "all three NULL-tx_index batch notes reached the walk",
        );
    }

    // ── Deep-e2e: the queries against the REAL `v_transactions` populated by a REAL scan ──
    //
    // The tests above pin the query LOGIC against rows WE control. These drive the actual
    // librustzcash scan/decrypt path (the funded harness) so the REAL view's semantics —
    // signed deltas, NULL `mined_height`/`tx_index` on the `GetStatus`/reorg path, the
    // account scoping, depth at the scanned frontier — are exercised, not assumed.
    mod real_view {
        use super::*;
        use crate::test_support::HistoryHarness;
        use std::collections::BTreeSet;
        use zcash_protocol::TxId as ZTxId;

        /// Cross the harness's upstream (INTERNAL-order) txid through the SDK's reversing door
        /// to the DISPLAY-order [`TxId`] that [`TxSummary`]/[`transaction_by_txid`] speak.
        fn disp(t: ZTxId) -> TxId {
            t.into()
        }

        /// Walk the keyset cursor to exhaustion at the given page size, returning the rows in
        /// page order. Guards against a non-terminating cursor.
        fn walk_all(conn: &Connection, page_size: u32) -> Vec<TxSummary> {
            let mut out = Vec::new();
            let mut cursor: Option<String> = None;
            for _ in 0..1000 {
                let page = list_transactions(conn, page_size, cursor.as_deref()).expect("page");
                assert!(
                    page.rows.len() as u32 <= page_size,
                    "the limit is honored per page"
                );
                out.extend(page.rows);
                match page.next {
                    Some(c) => cursor = Some(c),
                    None => return out,
                }
            }
            panic!("keyset cursor did not terminate");
        }

        /// **§4z AR-1's mechanism and AR-3's price, on the REAL engine and the
        /// REAL schema.** An Ironwood receipt is invisible to upstream's
        /// `ScanSummary` and visible to the high-water read, and that read is an
        /// index seek rather than a table scan.
        ///
        /// Three clauses:
        /// 1. a real mined Ironwood note scans, and `v_transactions` holds the
        ///    incoming row — the truth view was NEVER blind, which is what made
        ///    INC-016 a gate bug rather than a scanner bug;
        /// 2. `ironwood_received_high_water` moves 0 → 1 across that scan, while
        ///    the scan of a SAPLING note leaves it where it is — so the delta
        ///    counts Ironwood and only Ironwood;
        /// 3. the price: SQLite plans the read without a full table scan
        ///    (`id INTEGER PRIMARY KEY` is a rowid alias, so `MAX(id)` is the
        ///    rowid B-tree's rightmost entry). ADR-0536 gated the truth diff
        ///    because a per-batch `v_transactions` read measured ~74 % of a
        ///    batch on a deep restore; a gate that cost a table scan per batch
        ///    would have given that back.
        #[test]
        fn an_ironwood_receipt_is_invisible_to_the_scan_summary_and_visible_to_the_high_water() {
            let mut h = HistoryHarness::new();
            let before = {
                let conn = h.read_conn();
                ironwood_received_high_water(&conn).expect("high water")
            };
            assert_eq!(before, 0, "precondition: nothing received yet");

            // A SAPLING receive first — the control for clause 2.
            h.mine_received(100_000);
            h.mine_empty(2);
            h.scan();
            let after_sapling = {
                let conn = h.read_conn();
                ironwood_received_high_water(&conn).expect("high water")
            };
            assert_eq!(
                after_sapling, 0,
                "a Sapling receipt must not move the Ironwood high water — otherwise the delta \
                 would double-count what ScanSummary already reports"
            );

            let ironwood_txid = h.mine_received_ironwood(75_000);
            h.mine_empty(2);
            h.scan();
            let conn = h.read_conn();
            let after_ironwood = ironwood_received_high_water(&conn).expect("high water");
            assert!(
                after_ironwood > after_sapling,
                "§4z: a real mined Ironwood note must move the high water — the delta across a \
                 scan is the ONLY Ironwood receipt count available, because upstream's \
                 ScanSummary has none ({after_sapling} → {after_ironwood})"
            );

            // Clause 1: the truth view sees it, and always did.
            let rows = walk_all(&conn, 50);
            assert!(
                rows.iter().any(|r| r.txid == disp(ironwood_txid)),
                "v_transactions must hold the Ironwood receive row — the blindness was in the \
                 ADR-0536 GATE, never in the view. Rows: {:?}",
                rows.iter().map(|r| r.txid.clone()).collect::<Vec<_>>()
            );

            // Clause 3: the price. The plan must not be a full table scan.
            let plan: Vec<String> = conn
                .prepare(
                    "EXPLAIN QUERY PLAN SELECT COALESCE(MAX(id), 0) FROM ironwood_received_notes",
                )
                .expect("prepare")
                .query_map([], |r| r.get::<_, String>(3))
                .expect("query plan")
                .collect::<Result<_, _>>()
                .expect("plan rows");
            assert!(
                plan.iter()
                    .all(|step| !step.contains("SCAN ironwood_received_notes")),
                "§4z AR-3: the gate's new read must be an index seek, not a table scan — every \
                 batch pays for it. SQLite planned: {plan:?}"
            );
        }

        #[test]
        fn probe_funded_scan_surfaces_received_rows_over_the_aux_connection() {
            // The harness pipeline smoke: mine three received notes, scan to fill the real
            // view, then page over a SECOND connection (the production aux pattern). The
            // deep-e2e cases below stand on a real scan, not a fixture.
            let mut h = HistoryHarness::new();
            let a = h.mine_received(100_000);
            h.mine_received(200_000);
            h.mine_received(300_000);
            h.mine_empty(5);
            h.scan();

            let conn = h.read_conn();
            let page = list_transactions(&conn, 50, None).expect("list runs on real view");
            assert_eq!(page.rows.len(), 3, "all three received txs surface");
            assert!(page.next.is_none(), "three rows fit one page ⇒ no cursor");
            assert!(
                page.rows.iter().all(|r| r.net_amount.zat() > 0),
                "received notes are positive net",
            );
            assert!(
                page.rows
                    .iter()
                    .all(|r| matches!(r.status, TxStatus::Confirmed { .. })),
                "all mined + buried ⇒ confirmed",
            );
            // The internal→display txid door round-trips against a REAL scanned row (not just
            // the fixture): the mined txid looks itself up.
            let found = transaction_by_txid(&conn, &disp(a))
                .expect("lookup")
                .expect("present");
            assert_eq!(
                found.txid,
                disp(a),
                "the mined txid round-trips through the real view"
            );
            // Independent byte-order cross-check (not routed through `From` on both sides, so
            // it would catch a dropped reversal even if the door regressed): the RAW stored
            // `v_transactions.txid` blob is INTERNAL order = `ZTxId::as_ref()`, and the
            // DISPLAY-order txid we surface is its exact reverse.
            let stored: Vec<u8> = conn
                .query_row(
                    "SELECT txid FROM v_transactions WHERE account_uuid = \
                     (SELECT uuid FROM accounts ORDER BY id LIMIT 1) AND txid = ?1",
                    params![a.as_ref().as_slice()],
                    |r| r.get(0),
                )
                .expect("the mined row is stored under its INTERNAL txid");
            assert_eq!(
                stored.as_slice(),
                a.as_ref(),
                "the view stores INTERNAL byte order"
            );
            let mut display_bytes = *found.txid.as_bytes();
            display_bytes.reverse();
            assert_eq!(
                display_bytes,
                *a.as_ref(),
                "our display txid is the internal blob reversed"
            );
        }

        #[test]
        fn a_multi_note_receive_surfaces_one_row_with_the_summed_delta() {
            // A single transaction can pay us in SEVERAL notes (e.g. a payer splitting the amount
            // across output notes — all EXTERNAL receives here, not a self-send, which would be a
            // spend netting to ≈ −fee). `v_transactions` aggregates them into ONE row whose signed
            // `account_balance_delta` is the SUM of the notes — UNDER-reporting it (showing only
            // one note) would HIDE funds from the activity list. This pins the summed delta over
            // the REAL scanned view, and that a sibling single-note receive in another block is
            // NOT conflated into it. (This is the feasible money-relevant cousin of the
            // same-height-multi-ROW GAP-1, which the testing harness can't synthesize: every
            // `FakeCompactOutput` lands in ONE `CompactTx`, so N notes ⇒ 1 tx ⇒ 1 row.)
            let mut h = HistoryHarness::new();
            // One tx, three notes = 400_000; a separate single-note receive in the next block.
            let multi = h.mine_received_multi(&[100_000, 250_000, 50_000]);
            let single = h.mine_received(70_000);
            h.mine_empty(5);
            h.scan();

            let conn = h.read_conn();
            let page = list_transactions(&conn, 50, None).expect("list runs on real view");
            assert_eq!(
                page.rows.len(),
                2,
                "two txs ⇒ two rows; the three-note tx is ONE row, never three",
            );

            let multi_row = page
                .rows
                .iter()
                .find(|r| r.txid == disp(multi))
                .expect("the multi-note row is present");
            assert_eq!(
                multi_row.net_amount.zat(),
                400_000,
                "the row shows the SUM of the three notes — never just one (which would hide funds)",
            );
            let single_row = page
                .rows
                .iter()
                .find(|r| r.txid == disp(single))
                .expect("the single-note row is present");
            assert_eq!(
                single_row.net_amount.zat(),
                70_000,
                "the sibling single-note receive keeps its own value — not conflated with the multi",
            );
            assert!(
                page.rows
                    .iter()
                    .all(|r| matches!(r.status, TxStatus::Confirmed { .. })),
                "both mined + buried ⇒ confirmed",
            );
        }

        #[test]
        fn engine_written_transparent_rows_drive_has_transparent_output() {
            // The owed test (#317): the controlled fixture pins the classifier LOGIC
            // (`EXISTS(... output_pool = 0 AND IFNULL(is_change,0) = 0)`); THIS pins the
            // column NAMES + the engine's own `is_change` semantics against rows the ENGINE
            // wrote — both transparent doors:
            //   (a) a RECEIVED transparent output (`put_received_transparent_utxo` — the only
            //       inbound door; compact blocks carry no transparent data), whose
            //       `v_received_outputs` transparent arm hardcodes `is_change = 0` in 0.21
            //       (verified against the vendored SQL, exercised end-to-end here);
            //   (b) a wallet-CREATED de-shield (a t-addr RECIPIENT output stored by
            //       `create_proposed_transactions` — the sent-outputs door).
            // Control: a shielded-only receive must NOT flag (§3.2i-3 (c) — the flag's
            // absence is the no-public-output-leg claim the history row renders).
            let mut h = HistoryHarness::new();
            let shielded = h.mine_received(500_000);
            h.mine_empty(10); // bury past the spendable depth for the de-shield below
            h.scan();

            // (a) The engine-written transparent RECEIVE.
            let t_receive = h.fund_transparent(70_000);

            // (b) The engine-written de-shield: spend the shielded note to a plain
            // transparent recipient (one real proving run).
            use zcash_address::ToAddress;
            let deshield = h.create_send(
                zcash_address::ZcashAddress::from_transparent_p2pkh(
                    zcash_protocol::consensus::NetworkType::Regtest,
                    [0x05; 20],
                ),
                20_000,
            );

            let conn = h.read_conn();

            let shielded_row = transaction_by_txid(&conn, &disp(shielded))
                .expect("lookup")
                .expect("the shielded receive is present");
            assert!(
                !shielded_row.has_transparent_output,
                "a shielded-only receive must NOT claim a public output leg",
            );

            let t_row = transaction_by_txid(&conn, &disp(t_receive))
                .expect("lookup")
                .expect("the transparent receive surfaces as a row");
            assert!(
                t_row.has_transparent_output,
                "an engine-written received transparent output flags the row \
                 (is_change hardcoded 0 in the view's transparent arm)",
            );
            assert!(
                t_row.net_amount.zat() > 0,
                "the transparent receive is an inbound row",
            );

            let d_row = transaction_by_txid(&conn, &disp(deshield))
                .expect("lookup")
                .expect("the created de-shield is present (stored at create)");
            assert!(
                d_row.has_transparent_output,
                "an engine-written t-addr RECIPIENT output flags the created row",
            );
            assert!(
                d_row.net_amount.zat() < 0,
                "the de-shield is an outbound row (spend + fee)",
            );
        }

        #[test]
        fn keyset_pages_over_real_scanned_rows_union_to_every_row_once() {
            // The FR-1 correctness seam, now over the REAL view: walk the cursor at a small
            // page size and prove the union of pages is EXACTLY the mined set — no row dropped
            // at a boundary, none duplicated — and newest-first across the whole walk.
            let mut h = HistoryHarness::new();
            let mut want = BTreeSet::new();
            for v in [10_000u64, 20_000, 30_000, 40_000, 50_000] {
                want.insert(disp(h.mine_received(v)).to_string());
            }
            h.mine_empty(3);
            h.scan();
            let conn = h.read_conn();

            let rows = walk_all(&conn, 2);
            let seen: Vec<String> = rows.iter().map(|r| r.txid.to_string()).collect();
            let uniq: BTreeSet<String> = seen.iter().cloned().collect();
            assert_eq!(
                seen.len(),
                want.len(),
                "no row dropped or duplicated across pages"
            );
            assert_eq!(
                uniq, want,
                "the union of all pages is exactly the mined txs, once each"
            );

            // Newest-first holds across the page seams (all confirmed ⇒ heights non-increasing).
            let heights: Vec<u32> = rows
                .iter()
                .map(|r| r.mined_height.expect("all confirmed").value())
                .collect();
            assert!(
                heights.windows(2).all(|w| w[0] >= w[1]),
                "newest-first preserved across the cursor: {heights:?}",
            );
        }

        #[test]
        fn reorg_takes_a_confirmed_receive_to_pending_then_a_rescan_reconfirms_it() {
            // Money-visibility across a REAL chain reorg (`truncate_to_height` on the SQLite
            // store — the genuine primitive; `set_transaction_status` does NOT un-mine a
            // scanned tx). A confirmed received tx whose block is orphaned must RE-SURFACE as
            // pending (sorted to the top), never vanish, money intact; replaying the retained
            // cache re-confirms it. Crucially the un-mined row has `expired_unmined = NULL`
            // (a received tx has no expiry_height) — a regression guard that one such row no
            // longer errors the WHOLE page to `StoreCorrupt`.
            let mut h = HistoryHarness::new();
            let keep = h.mine_received(500_000); // an earlier tx, below the reorg point
            let safe_height = h.current_tip();
            let orphan = h.mine_received(123_000); // the tx the reorg will un-mine
            h.mine_empty(4);
            h.scan();
            let full_tip = h.current_tip();
            let conn = h.read_conn();

            let before = list_transactions(&conn, 50, None).expect("list before reorg");
            let orphan_row = before
                .rows
                .iter()
                .find(|r| r.txid == disp(orphan))
                .expect("orphan tx starts visible");
            assert!(
                matches!(orphan_row.status, TxStatus::Confirmed { .. }),
                "starts confirmed"
            );
            assert_eq!(orphan_row.net_amount.zat(), 123_000);

            // Reorg back below the orphan's block: it is un-mined → pending, NOT dropped.
            h.reorg_to_height(safe_height);
            let mid = list_transactions(&conn, 50, None).expect(
                "the page still loads — a NULL expired_unmined row must not StoreCorrupt it",
            );
            let pending = mid
                .rows
                .iter()
                .find(|r| r.txid == disp(orphan))
                .expect("the un-mined received tx is NOT dropped — it stays visible");
            assert_eq!(pending.status, TxStatus::Pending, "orphaned ⇒ pending");
            assert_eq!(pending.mined_height, None, "no block height while un-mined");
            assert_eq!(
                pending.net_amount.zat(),
                123_000,
                "the money survives the reorg"
            );
            assert_eq!(
                mid.rows[0].txid,
                disp(orphan),
                "pending sorts above the confirmed history"
            );
            assert!(
                mid.rows.iter().any(|r| r.txid == disp(keep)),
                "a tx below the reorg point is untouched",
            );

            // Replay the retained cache: the orphan re-confirms, money intact — never lost.
            h.rescan_to_tip(full_tip);
            let after = list_transactions(&conn, 50, None).expect("list after rescan");
            let restored =
                after.rows.iter().find(|r| r.txid == disp(orphan)).expect(
                    "the un-mined tx is RE-CONFIRMED by the rescan — never permanently lost",
                );
            assert!(
                matches!(restored.status, TxStatus::Confirmed { .. }),
                "restored ⇒ confirmed"
            );
            assert_eq!(
                restored.net_amount.zat(),
                123_000,
                "the money survives the round-trip"
            );
        }

        #[test]
        fn a_multi_note_receive_keeps_its_summed_delta_across_a_reorg_and_reconfirm() {
            // Money-visibility for a MULTI-NOTE receive across a real reorg: the summed
            // `account_balance_delta` must survive un-mine → Pending → reconfirm INTACT — never a
            // PARTIAL (e.g. 400k → 100k, which a per-note retention bug in truncation could
            // produce). The single-note reorg test above cannot catch a subset-retention defect;
            // this one pins the WHOLE sum through the round-trip.
            let mut h = HistoryHarness::new();
            let _keep = h.mine_received(500_000); // an earlier tx, below the reorg point
            let safe_height = h.current_tip();
            let multi = h.mine_received_multi(&[100_000, 250_000, 50_000]); // one tx, 400_000
            h.mine_empty(4);
            h.scan();
            let full_tip = h.current_tip();
            let conn = h.read_conn();

            let before = list_transactions(&conn, 50, None).expect("list before reorg");
            let row = before
                .rows
                .iter()
                .find(|r| r.txid == disp(multi))
                .expect("the multi-note tx starts visible");
            assert!(
                matches!(row.status, TxStatus::Confirmed { .. }),
                "starts confirmed"
            );
            assert_eq!(
                row.net_amount.zat(),
                400_000,
                "confirmed at the full summed delta"
            );

            // Reorg below the multi-note block: un-mined → pending, the WHOLE sum intact.
            h.reorg_to_height(safe_height);
            let mid = list_transactions(&conn, 50, None).expect("the page still loads after reorg");
            let pending = mid
                .rows
                .iter()
                .find(|r| r.txid == disp(multi))
                .expect("the un-mined multi-note tx is NOT dropped");
            assert_eq!(pending.status, TxStatus::Pending, "orphaned ⇒ pending");
            assert_eq!(pending.mined_height, None, "no block height while un-mined");
            assert_eq!(
                pending.net_amount.zat(),
                400_000,
                "the FULL sum survives un-mining — never a partial (no note silently dropped)",
            );

            // Replay the retained cache: re-confirms at the full sum, never lost or shrunk.
            h.rescan_to_tip(full_tip);
            let after = list_transactions(&conn, 50, None).expect("list after rescan");
            let restored = after
                .rows
                .iter()
                .find(|r| r.txid == disp(multi))
                .expect("the multi-note tx is RE-CONFIRMED by the rescan");
            assert!(
                matches!(restored.status, TxStatus::Confirmed { .. }),
                "restored ⇒ confirmed"
            );
            assert_eq!(
                restored.net_amount.zat(),
                400_000,
                "the full summed delta survives the reorg round-trip",
            );
        }

        #[test]
        fn a_scanned_receive_yields_an_enhancement_request_that_select_targets_picks() {
            // The §3.3 enhancement loop's core assumption — that a compact-scanned shielded
            // RECEIVE surfaces as an `Enhancement(txid)` request (compact blocks omit the memo,
            // so the full tx must be fetched) and that `select_enhancement_targets` selects it —
            // validated against REAL upstream `transaction_data_requests()` output, not the
            // hand-built requests the `enhance.rs` unit test uses. Closes the biggest "does the
            // loop's filter match reality" gap without a device.
            use crate::constants::MAX_ENHANCEMENTS_PER_PASS;
            use zcash_client_backend::data_api::TransactionDataRequest;
            let mut h = HistoryHarness::new();
            let a = h.mine_received(42_000);
            h.mine_empty(2);
            h.scan();

            let reqs = h.pending_requests();
            assert!(
                reqs.iter()
                    .any(|r| matches!(r, TransactionDataRequest::Enhancement(t) if *t == a)),
                "a real scanned receive surfaces as an Enhancement(txid) request",
            );
            // The loop's selection picks exactly it (Enhancement kept; the transparent variant,
            // if any, deferred) — over REAL output.
            let targets = crate::enhance::select_enhancement_targets(
                reqs,
                MAX_ENHANCEMENTS_PER_PASS,
                &Default::default(),
            );
            assert!(
                targets.contains(&(a, crate::enhance::RequestKind::Enhancement)),
                "select_enhancement_targets picks the scanned receive, AS an Enhancement — \
                 a real scanned receive wants its DATA (the memo), and answering it with a \
                 status instead would recover nothing (INC-009's mirror image)",
            );
        }

        #[test]
        fn a_not_found_reply_never_unscans_a_received_note() {
            // The §3.3 enhancement loop records `TxidNotRecognized` when the endpoint returns an
            // empty body for a request. A LYING endpoint could answer "not found" for a tx we
            // ALREADY scanned (a real receive). That must NOT un-detect the note / hide the
            // money — the chain scan is the source of truth, enhancement only ADDS the memo.
            // Pins the upstream behavior the loop relies on (matches the reorg finding:
            // set_transaction_status does not override scanned block linkage).
            let mut h = HistoryHarness::new();
            let a = h.mine_received(456_000);
            h.mine_empty(3);
            h.scan();
            let conn = h.read_conn();
            let before = transaction_by_txid(&conn, &disp(a))
                .expect("lookup")
                .expect("present");
            assert!(matches!(before.status, TxStatus::Confirmed { .. }));
            assert_eq!(before.net_amount.zat(), 456_000);

            // The hostile/empty not-found reply.
            h.mark_not_recognized(a);

            let after = transaction_by_txid(&conn, &disp(a))
                .expect("lookup")
                .expect("present");
            assert!(
                matches!(after.status, TxStatus::Confirmed { .. }),
                "a not-found reply must not un-mine a scanned receive",
            );
            assert_eq!(
                after.net_amount.zat(),
                456_000,
                "the received amount is intact"
            );
            // And it still appears in the history page (never silently dropped).
            let page = list_transactions(&conn, 50, None).expect("list");
            assert!(
                page.rows.iter().any(|r| r.txid == disp(a)),
                "the receive stays visible"
            );
        }

        /// **INC-019 (phase-2 P2-3) — a send the network never accepted stays VISIBLE,
        /// with an honest terminal state.** On the device proof five "Expired"
        /// rows left the Activity list across an in-place migration, at the moment the
        /// enhancement pass logged `status_set=5`. **The registry row (INC-019,
        /// `evals/incidents.tsv`) is the ONE source of truth for what a status write can
        /// and cannot do, which arm reproduces the symptom, why the first attribution
        /// was refuted, and what stays uncovered** — this doc carries only what the
        /// clauses need. Two arms: `NotInMainChain` (a no-height answer) deletes
        /// nothing and cannot take a row away — clauses 1–4; `Mined(h)` (a height
        /// answer, ACCEPTED for a never-mined send under the residual — the
        /// reasoning is at `enhance::chain_status`) sets `mined_height`, so the row
        /// re-reads `Confirmed` and `TX_ORDER` sorts it OUT of the unmined group at the
        /// top of the list — clause 5: moved and re-read, never gone.
        ///
        /// Fresh wallet, the row planted through the REAL store (`create_send`: propose,
        /// prove, sign, store — never mined), the status written through the REAL
        /// `set_transaction_status`, and the expiry crossed by scanning real blocks:
        ///
        /// 1. unmined and unexpired, the send is a `Pending` row — never gone;
        /// 2. after the `NotInMainChain` write it is STILL `Pending` (the write is a
        ///    fact about the chain, not a verdict: the tx can still be mined before its
        ///    expiry) — this clause pins UPSTREAM's arm, not this crate: neither
        ///    registered mutant reaches it, and it would red on a `zcash_client_sqlite`
        ///    bump that made the write hide or expire the row;
        /// 3. once the scanned frontier reaches the row's own `expiry_height` it reads
        ///    `Expired` — at the edge itself and one block past it — never `Pending`,
        ///    never gone, with the outgoing amount. "Terminal" is relative to the
        ///    scanned frontier: the view's `expired_unmined` reads `MAX(blocks.height)`,
        ///    and a rewind that drops blocks back below the expiry reads `Pending`
        ///    again until the frontier returns (the security pass's row 8);
        /// 4. a SECOND `NotInMainChain` write on the already-expired row (the
        ///    pass's shape, if that is what it wrote) changes nothing: still `Expired`,
        ///    still on the first page, still found by txid, exactly once in the walk;
        /// 5. a `Mined(h)` write on the expired send (the -accepted arm, `h` inside
        ///    the accepted window): the row is STILL in the walk exactly once, reads
        ///    `Confirmed { depth }` at `h`, and has LEFT the pending-first top of the
        ///    list for its height's place — moved and re-read, never gone.
        ///
        /// Watched against (one per run, the base restored between): `map_row`'s
        /// `if raw.expired_unmined` → `if false` (clause 3 reds: the row reads
        /// `Pending`), and `list_transactions`' base `WHERE` gaining
        /// `AND mined_height IS NOT NULL` (clauses 1 and 3 red: the row is gone —
        /// the incident's own shape).
        #[test]
        fn a_send_the_network_never_accepted_stays_visible_and_reads_expired() {
            let mut h = HistoryHarness::new();
            h.mine_received(1_000_000);
            h.mine_empty(12); // bury past the spendable depth for the send below
            h.scan();
            use zcash_address::ToAddress;
            let send = h.create_send(
                zcash_address::ZcashAddress::from_transparent_p2pkh(
                    zcash_protocol::consensus::NetworkType::Regtest,
                    [0x07; 20],
                ),
                20_000,
            );
            let expiry = h.expiry_height_of(send);
            let tip = h.current_tip();
            assert!(
                expiry > tip,
                "harness: the created send expires above the tip it was built at \
                 (expiry {expiry}, tip {tip})"
            );

            let conn = h.read_conn();
            let read = |conn: &Connection| -> TxSummary {
                transaction_by_txid(conn, &disp(send))
                    .expect("lookup")
                    .expect("INC-019: the send row is present")
            };
            let on_first_page = |conn: &Connection| -> bool {
                list_transactions(conn, 50, None)
                    .expect("list")
                    .rows
                    .iter()
                    .any(|r| r.txid == disp(send))
            };

            // 1. Unmined, unexpired: a Pending row, on the page.
            let before = read(&conn);
            assert_eq!(
                before.status,
                TxStatus::Pending,
                "an unmined, unexpired wallet-created send reads Pending"
            );
            assert!(
                before.net_amount.zat() < 0,
                "the send is an outgoing row: {}",
                before.net_amount.zat()
            );
            assert!(
                on_first_page(&conn),
                "the pending send is on the first page"
            );

            // 2. The enhancement pass's status write — the chain does not hold it.
            h.mark_not_in_main_chain(send);
            let after_status = read(&conn);
            assert_eq!(
                after_status.status,
                TxStatus::Pending,
                "INC-019: a NotInMainChain write before the expiry leaves the row Pending — \
                 a fact about the chain now, not a verdict; the tx can still be mined"
            );
            assert!(
                on_first_page(&conn),
                "INC-019: the status write must not take the row off the page"
            );

            // 3. The scanned frontier reaches the row's OWN expiry height — the
            //    boundary itself first (the view's `expiry_height BETWEEN 1 AND
            //    max_height` is inclusive: a tx with expiry_height H cannot be mined
            //    at H, so the row is expired the moment block H is scanned), then one
            //    block past it (the arch pass's row 9: a test that lands only past
            //    the edge cannot see an off-by-one at the edge).
            let to_mine = usize::try_from(expiry - tip).expect("a small block count");
            h.mine_empty(to_mine);
            h.scan();
            assert_eq!(
                h.current_tip(),
                expiry,
                "harness: the scanned tip IS the expiry"
            );
            let at_edge = read(&conn);
            assert_eq!(
                at_edge.status,
                TxStatus::Expired,
                "INC-019: at the expiry height itself an unmined send reads Expired — \
                 upstream's view is inclusive at the edge"
            );
            // One block past the edge — mined as a RECEIVE, so a confirmed row exists
            // ABOVE the height clause 5 will claim: without it clause 5's "moved out
            // of the top" could not fail (the code reviewer's row 1: two confirmed
            // rows and no pending one made `position >= 0` a tautology).
            let receive_above = h.mine_received(30_000);
            h.scan();
            let expired = read(&conn);
            assert_eq!(
                expired.status,
                TxStatus::Expired,
                "INC-019: past its expiry an unmined send reads Expired — the honest terminal \
                 state (the funds are the user's again) — never Pending"
            );
            assert_eq!(
                expired.net_amount, before.net_amount,
                "the expired row still shows what the send would have moved"
            );
            assert!(
                on_first_page(&conn),
                "INC-019: an Expired send stays on the first page (pending-first order)"
            );

            // 4. The shape: a status write on the ALREADY-expired row.
            h.mark_not_in_main_chain(send);
            let again = read(&conn);
            assert_eq!(
                again.status,
                TxStatus::Expired,
                "INC-019: a second NotInMainChain write on an expired row changes nothing"
            );
            assert!(
                on_first_page(&conn),
                "INC-019: the row a user could see yesterday is on the page today"
            );
            let rows = walk_all(&conn, 2);
            assert_eq!(
                rows.iter().filter(|r| r.txid == disp(send)).count(),
                1,
                "the expired send appears exactly once across the whole keyset walk: {:?}",
                rows.iter()
                    .map(|r| (r.txid.clone(), r.status))
                    .collect::<Vec<_>>()
            );
            // The "top of the list" the observer read is the UNMINED group `TX_ORDER`
            // puts first (`(mined_height IS NULL) DESC` — not the Pending STATUS, which
            // an expired unmined row does not carry). The expired send sits inside it,
            // above the two confirmed receives.
            let unmined_prefix = rows.iter().take_while(|r| r.mined_height.is_none()).count();
            let position = rows
                .iter()
                .position(|r| r.txid == disp(send))
                .expect("present");
            assert!(
                position < unmined_prefix,
                "INC-019: an expired unmined send is in the unmined group at the TOP of the \
                 list (position {position}, unmined prefix {unmined_prefix}): {:?}",
                rows.iter()
                    .map(|r| (r.txid.clone(), r.mined_height, r.status))
                    .collect::<Vec<_>>()
            );

            // 5. The OTHER arm: a `Mined(h)` write on the expired send, `h` inside the
            //    window `chain_status` accepts for a tx the wallet holds no height for
            //    The row MOVES — out of the unmined group at the
            //    top, to its height's place BELOW the receive mined above it — and
            //    re-reads Confirmed; it does not vanish.
            let claimed = expiry - 1;
            h.mark_mined_at(send, claimed);
            let mined = read(&conn);
            assert!(
                matches!(mined.status, TxStatus::Confirmed { .. }),
                "INC-019 (the Mined arm): a status answer naming a height re-reads the row \
                 as Confirmed — upstream sets mined_height unconditionally; got {:?}",
                mined.status
            );
            assert_eq!(
                mined.mined_height,
                Some(BlockHeight::new(claimed)),
                "the row carries the claimed height"
            );
            let rows = walk_all(&conn, 2);
            assert_eq!(
                rows.iter().filter(|r| r.txid == disp(send)).count(),
                1,
                "INC-019 (the Mined arm): the row is still in the walk exactly once — moved, \
                 never gone: {:?}",
                rows.iter()
                    .map(|r| (r.txid.clone(), r.status))
                    .collect::<Vec<_>>()
            );
            let position = rows
                .iter()
                .position(|r| r.txid == disp(send))
                .expect("present");
            assert!(
                position > 0 && rows[0].txid == disp(receive_above),
                "INC-019 (the Mined arm): the row has LEFT the top of the list for its \
                 height's place — the receive mined one block past the expiry now sorts \
                 above it, which is what an observer reading the top saw as \"vanished\"; \
                 position {position}: {:?}",
                rows.iter()
                    .map(|r| (r.txid.clone(), r.mined_height, r.status))
                    .collect::<Vec<_>>()
            );
        }

        /// Stage S2 `outcome` (§3.5c, FR-41's residual) — the core half. A
        /// wallet-created send's history row carries its OWN `expiry_height` (read off
        /// the row, never a constant), a receive carries none — even one whose
        /// `transactions` row holds the tx's expiry, as a decrypted receive's does —
        /// and the reading rule holds against the real view, both cases:
        ///
        /// 1. `expiry_height > tip` — unresolved on chain: the row reads `Pending`;
        /// 2. the chain has passed `expiry_height` but the wallet has not scanned to
        ///    it — the row STILL reads `Pending` (the status is judged against the
        ///    wallet's own scan, never the chain tip); a sync to ≥ `expiry_height`
        ///    resolves it to `Expired`.
        #[test]
        fn a_wallet_created_send_reports_the_height_its_unknown_outcome_resolves_at_and_the_rule_reads_it()
         {
            let mut h = HistoryHarness::new();
            let received = h.mine_received(1_000_000);
            h.mine_empty(12); // bury past the spendable depth for the send below
            h.scan();
            use zcash_address::ToAddress;
            let send = h.create_send(
                zcash_address::ZcashAddress::from_transparent_p2pkh(
                    zcash_protocol::consensus::NetworkType::Regtest,
                    [0x07; 20],
                ),
                20_000,
            );
            let expiry = h.expiry_height_of(send);
            let conn = h.read_conn();
            let read = |conn: &Connection, txid: ZTxId| -> TxSummary {
                transaction_by_txid(conn, &disp(txid))
                    .expect("lookup")
                    .expect("the row is present")
            };

            // A decrypted receive's `transactions` row holds the tx's own expiry
            // (upstream `put_tx_data` writes it); plant one so the gate is on
            // who CREATED the tx, not on whether a height happens to be stored.
            conn.execute(
                "UPDATE transactions SET expiry_height = ?1 WHERE txid = ?2",
                rusqlite::params![expiry, received.as_ref()],
            )
            .expect("plant a receive's expiry");
            assert_eq!(
                read(&conn, received).expiry_height,
                None,
                "a received transaction has no outcome-resolution height"
            );

            // Case 1: expiry above the tip — unresolved on chain, wait for block N.
            let tip = h.current_tip();
            assert_eq!(
                scanned_tip(&conn).expect("tip"),
                Some(tip),
                "harness: scanned to the chain tip"
            );
            assert!(
                expiry > tip,
                "harness: the send expires above the tip (expiry {expiry}, tip {tip})"
            );
            let pending = read(&conn, send);
            assert_eq!(
                pending.expiry_height,
                Some(BlockHeight::new(expiry)),
                "the send reports its own expiry height"
            );
            assert_eq!(pending.status, TxStatus::Pending);

            // Case 2: the chain passes the expiry, the wallet has not looked yet.
            let to_mine = usize::try_from(expiry - tip).expect("a small block count");
            h.mine_empty(to_mine);
            assert!(
                h.current_tip() >= expiry,
                "harness: the chain is at or past the expiry"
            );
            let scanned = scanned_tip(&conn).expect("tip").expect("scanned");
            assert!(
                scanned < expiry,
                "harness: the wallet's scan is still below the expiry ({scanned} < {expiry})"
            );
            assert_eq!(
                read(&conn, send).status,
                TxStatus::Pending,
                "not looked yet: the status is judged against the wallet's own scan"
            );
            h.scan();
            let resolved = read(&conn, send);
            assert_eq!(
                resolved.status,
                TxStatus::Expired,
                "a sync to ≥ expiryHeight resolves the unknown outcome"
            );
            assert_eq!(
                resolved.expiry_height,
                Some(BlockHeight::new(expiry)),
                "the height stays on the resolved row"
            );
        }

        #[test]
        fn confirmation_depth_tracks_the_scanned_frontier() {
            // The depth math at the scanned frontier, against a real scan: a tx mined at the
            // tip reads depth 1; burying it N blocks deeper reads depth N+1.
            let mut h = HistoryHarness::new();
            let a = h.mine_received(70_000);
            h.scan(); // tip == h_a
            let conn = h.read_conn();
            let s = transaction_by_txid(&conn, &disp(a))
                .expect("lookup")
                .expect("present");
            assert_eq!(
                s.status,
                TxStatus::Confirmed { depth: 1 },
                "mined at the scanned frontier ⇒ depth 1",
            );

            h.mine_empty(9);
            h.scan(); // tip == h_a + 9
            let s = transaction_by_txid(&conn, &disp(a))
                .expect("lookup")
                .expect("present");
            assert_eq!(
                s.status,
                TxStatus::Confirmed { depth: 10 },
                "nine blocks on top ⇒ depth 10",
            );
        }

        #[test]
        fn a_history_read_sees_a_consistent_snapshot_during_an_in_flight_write() {
            // The aux read runs on its OWN connection. While another connection holds an
            // UNCOMMITTED write tx, a history page must (a) not BUSY-fail — `busy_timeout`
            // absorbs rollback-journal contention, proven generally by
            // `db.rs::concurrent_keyed_connections_serialize_intent_writes_without_busy_failure`
            // — and (b) see a CONSISTENT committed snapshot, never a torn half-written row.
            let mut h = HistoryHarness::new();
            h.mine_received(60_000);
            h.mine_received(70_000);
            h.mine_empty(3);
            h.scan();

            // A separate connection opens a write tx (RESERVED lock) and holds it.
            let writer = Connection::open(h.path()).expect("writer connection");
            rusqlite::vtab::array::load_module(&writer).expect("module");
            writer
                .execute_batch(
                    "BEGIN IMMEDIATE; CREATE TABLE _probe_lock(x); INSERT INTO _probe_lock VALUES (1);",
                )
                .expect("acquire a write lock");

            // The reader still pages successfully and sees exactly the two committed rows.
            let conn = h.read_conn();
            let page = list_transactions(&conn, 50, None)
                .expect("the history read does not BUSY-fail under an in-flight write");
            assert_eq!(
                page.rows.len(),
                2,
                "a consistent committed snapshot — the in-flight write is invisible, not torn",
            );

            writer.execute_batch("ROLLBACK").expect("release the lock");
        }

        // ── ADR-0536 #392: the incoming-funds detect/replay reads over the REAL view ──

        /// The mined height of the single positive row the harness produced (the tests
        /// below need real heights; the harness doesn't expose them, so read the row).
        fn only_incoming_height(conn: &Connection) -> u32 {
            let page = list_transactions(conn, 50, None).expect("page");
            let mined: Vec<u32> = page
                .rows
                .iter()
                .filter(|r| r.net_amount.zat() > 0)
                .filter_map(|r| r.mined_height.map(|h| h.value()))
                .collect();
            assert_eq!(mined.len(), 1, "exactly one mined incoming row");
            mined[0]
        }

        #[test]
        fn incoming_in_span_counts_a_real_receive_only_inside_its_window() {
            // The scan-edge truth diff (gate-8): a real scanned receive is counted with
            // an exact span when the window covers its height, and is INVISIBLE to a
            // window below it (the batch that didn't mine it never fires an event).
            let mut h = HistoryHarness::new();
            h.mine_received(80_000);
            h.mine_empty(4);
            h.scan();
            let conn = h.read_conn();
            let at = only_incoming_height(&conn);

            let hit = incoming_in_span(&conn, at, at + 2).expect("covering window");
            assert_eq!(hit.count, 1, "the receive is in the window");
            assert_eq!(hit.span, Some((at, at)), "span is the exact mined height");

            let miss = incoming_in_span(&conn, at + 1, at + 4).expect("window above");
            assert_eq!(miss.count, 0, "a window that missed the height is silent");
            assert!(miss.span.is_none(), "no rows ⇒ no span (NULL MIN/MAX)");
        }

        #[test]
        fn incoming_reads_skip_sends_and_replay_the_transparent_receive() {
            // Two truths through the funded engine pipeline:
            //   (a) an engine-written transparent RECEIVE (positive delta, mined at
            //       tip via `put_received_transparent_utxo`) IS counted by the
            //       replay read — a restore/catch-up covers the transparent leg
            //       even though v1 `Live` spans are shielded-scan-edge only
            //       (ADR-0536 scope note);
            //   (b) a wallet-CREATED send (negative delta; created+stored, unmined)
            //       never counts — not in a replay (a NULL `mined_height` can't
            //       clear a watermark) and not in any span. The "funds arrived on
            //       your own send" false-fire stays impossible at the SQL layer,
            //       whatever the note-count gate said.
            let mut h = HistoryHarness::new();
            h.mine_received(1_000_000);
            h.mine_empty(12); // bury past spendable depth for the send below
            h.scan();
            let conn = h.read_conn();
            let shielded_at = only_incoming_height(&conn);

            h.fund_transparent(70_000); // put at the CURRENT tip ⇒ mined ABOVE shielded_at
            use zcash_address::ToAddress;
            let _send = h.create_send(
                zcash_address::ZcashAddress::from_transparent_p2pkh(
                    zcash_protocol::consensus::NetworkType::Regtest,
                    [0x05; 20],
                ),
                20_000,
            );

            let full = incoming_above(&conn, None).expect("full catch-up");
            assert_eq!(
                full.count, 2,
                "the shielded + the transparent receives; NEVER the send",
            );
            let span = full.span.expect("two mined incoming rows span");
            assert_eq!(span.0, shielded_at, "span floor = the shielded receive");
            assert!(
                span.1 > shielded_at,
                "span top = the transparent put at tip"
            );

            let above = incoming_above(&conn, Some(shielded_at)).expect("watermark");
            assert_eq!(
                above.count, 1,
                "only the transparent receive replays above the shielded watermark",
            );
        }

        #[test]
        fn fully_scanned_frontier_tracks_pending_scan_ranges() {
            // §8 (R1): the frontier equals the scan max ONLY when no
            // pending ranges remain; a pending LOWER range (the priority-order
            // restore shape — near-tip scanned first) caps it at start − 1, so
            // an incoming-funds watermark can never claim undiffed heights.
            let mut h = HistoryHarness::new();
            {
                let conn = h.read_conn();
                assert_eq!(
                    fully_scanned_frontier(&conn).expect("read"),
                    None,
                    "nothing scanned yet"
                );
            }
            h.mine_received(50_000);
            h.mine_empty(3);
            h.scan();
            let conn = h.read_conn();
            let tip = scanned_tip(&conn).expect("tip").expect("scanned");
            assert_eq!(
                fully_scanned_frontier(&conn).expect("read"),
                Some(tip),
                "no pending lower ranges ⇒ frontier = scan max"
            );
            // Hand-insert a PENDING range below the tip (priority > 10 =
            // pending in the exact-pinned zcash_client_sqlite 0.21 schema).
            conn.execute(
                "INSERT INTO scan_queue (block_range_start, block_range_end, priority) \
                 VALUES (?1, ?2, 20)",
                rusqlite::params![tip - 2, tip],
            )
            .expect("insert a pending range");
            assert_eq!(
                fully_scanned_frontier(&conn).expect("read"),
                Some(tip - 3),
                "a pending lower range caps the frontier at its start − 1"
            );
        }

        #[test]
        fn incoming_above_replays_strictly_above_the_watermark() {
            // The replay read (gate-8): `None` = full catch-up; a watermark below the
            // height replays it; AT the height suppresses it (strictly-above — the
            // cursor means "accounted THROUGH this height", so an equal-height replay
            // would double-notify every wallet-open).
            let mut h = HistoryHarness::new();
            h.mine_received(90_000);
            h.mine_empty(3);
            h.scan();
            let conn = h.read_conn();
            let at = only_incoming_height(&conn);

            let full = incoming_above(&conn, None).expect("no watermark");
            assert_eq!(full.count, 1, "None replays all mined incoming rows");
            assert_eq!(full.span, Some((at, at)));

            let below = incoming_above(&conn, Some(at - 1)).expect("below");
            assert_eq!(below.count, 1, "a watermark below the height replays it");

            let exact = incoming_above(&conn, Some(at)).expect("exact");
            assert_eq!(exact.count, 0, "the accounted height does not replay");
            assert!(exact.span.is_none());
        }
    }

    // ── ADR-0536 #392: the incoming watermark cursor (strict, panic-free) ──

    #[test]
    fn incoming_cursor_round_trips() {
        let c = IncomingCursor(1_234_567);
        assert_eq!(c.encode(), "w1:1234567");
        assert_eq!(IncomingCursor::decode(&c.encode()).expect("round trip"), c);
        assert_eq!(
            IncomingCursor::decode("w1:0").expect("zero is a valid baseline"),
            IncomingCursor(0),
        );
    }

    #[test]
    fn incoming_cursor_rejects_garbage_and_foreign_tokens_typed() {
        // Invariant 7: the token crosses the FFI and may be anything. Every reject is
        // the TYPED single corruption door, never a panic or a silent wrong-window.
        let cases: &[&str] = &[
            "",
            "w1",
            "w1:",
            "w1:abc",
            "w1:-5",
            "w1:1:extra",
            "w2:100",
            "w1:99999999999999999999", // u32 overflow
            // A HistoryPage KEYSET token: the deliberate cross-API confusion reject —
            // mis-reading it as a watermark would silently suppress a catch-up window.
            "v1:100:00000000000000000000000000000000000000000000000000000000000000ff",
        ];
        for s in cases {
            assert!(
                matches!(IncomingCursor::decode(s), Err(WalletError::StoreCorrupt)),
                "{s:?} must reject typed",
            );
        }
        let overlong = format!("w1:{}", "9".repeat(200));
        assert!(
            matches!(
                IncomingCursor::decode(&overlong),
                Err(WalletError::StoreCorrupt)
            ),
            "size-cap before parsing work",
        );
    }
}
