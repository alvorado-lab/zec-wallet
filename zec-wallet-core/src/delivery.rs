//! The delivery obligation (stage S8 `obligation`, the 2026-09-20 review's R02).
//!
//! A signed transaction the wallet PERSISTED is the wallet's obligation to deliver
//! it. The obligation is not a second write a crash could lose: it is DERIVED from
//! what the signing persistence already committed — the engine's `transactions` row
//! (raw bytes, `expiry_height`, `created`, `mined_height`), written in the SAME
//! engine transaction that marks the spent notes — so a death anywhere after
//! `create_proposed_transactions` returns leaves a row this module finds on the next
//! sync pass and after a reopen. Before this module, only an INTENT row
//! (`queued_send_intent`) enrolled a send for retry; an ordinary single-step transfer
//! or shield wrote none, so a signed transaction whose broadcast failed was never
//! retried while the UI said "saved — your wallet will send it on a later sync"
//! (`docs/plan/stage-8-payment-identity-and-durable-retry.md` §3.0, §3.2).
//!
//! **What the generic phase enumerates** ([`enumerate`]): every wallet-created
//! (`created IS NOT NULL` — the upstream stamp `create_proposed_transactions` writes
//! and the decrypt path never does), unmined, unexpired `transactions` row with its
//! raw bytes, MINUS every transaction an intent row can own. Ownership is decided by
//! ATTRIBUTION ([`DeliveryView::load`]): a recorded `txids` group (a `Sent` or
//! `Stranded` row) AND the notes a live claim names (a `Submitting` row's `claim`,
//! recorded BEFORE the engine creates the tx — `intent_store::mark_submitting`).
//! The second leg is load-bearing: in the create-committed-but-unrecorded window (a
//! death between the engine's commit and `mark_sent_multi`) the tx exists, spends the
//! claimed notes, and no row carries its txid; the intent path deliberately never
//! broadcasts it (`send::Recovery::AwaitWitness`) and neither may this phase, so
//! exclusion by recorded txid alone would be wrong exactly there. The intent
//! machinery keeps sole ownership of ordered groups, deposit deadlines and the
//! deposit hold (`send::rebroadcast_group`'s `DepositBroadcastHold`).
//!
//! **The accepted mark** ([`mark_accepted`]): a best-effort aux write recording that
//! an endpoint accepted a transaction (txid, the scanned tip, the wall clock). ONE
//! broadcast decision reads it (the 2026-10-05 review F01): for a swap deposit, a leg
//! whose own mark exists, or whose predecessor is mined or marked, CONTINUES under the
//! shorter continue gate instead of the start gate (`send::rebroadcast_group`, and
//! [`DeliveryView::state_of`] mirroring it). Its LOSS MODE is the conservative gate:
//! a lost mark (a `broadcast_one` future dropped between the submit and the write, a
//! swallowed write error) makes that leg a START, which can hold a second leg inside
//! the start margin. Otherwise the generic phase rebroadcasts an accepted-but-unmined
//! transaction on every later pass exactly like the intent path does (a duplicate
//! submit is `Rejected` by the endpoint and moves no money — the priced "bounded
//! redundant broadcast traffic between acceptance and confirmation"), and a lost
//! mark costs the per-transaction READING: it says *retry-pending* instead of
//! *accepted* until the next pass's success re-writes it.
//!
//! **The four states** ([`DeliveryView::state_of`]): *persisted* (the bytes are kept
//! and the wallet is not going to broadcast them on its own right now — a swap
//! deposit held past its quote, the unrecorded create window), *retry-pending* (the
//! wallet owes a broadcast and the next pass attempts it), *accepted* (an endpoint
//! took it, not yet in the chain), *confirmed* (mined). A received transaction and
//! an expired one have NO delivery state (`None`) — `TxStatus` already says so —
//! unless a live intent will replace it: an expired transaction a live `Sent` row
//! owns reads *retry-pending*, because the intent path WILL send the payment again
//! once the expiry is buried beyond reorg reach (`send::rebroadcast_group`'s
//! requeue, S7 C1). Saying nothing there would read "expired, the amount is still
//! yours" through the ~`REORG_MAX_BLOCKS` wait, and a user who re-sent by hand
//! would pay twice. A `Stranded` row's transaction, a group with a MINED leg (a
//! two-step whose tx0 mined strands, never re-sends) and a swap deposit's (by
//! burial its quote has lapsed and the drain deletes it) stay `None` once expired.
//!
//! **Storage and connection.** Like [`crate::history`] this is pure SQL over the
//! aux SQLCipher connection (`Inner.aux_db`) to the ONE `wallet.db` file: the
//! engine's `WalletDb` exposes no connection accessor, and the engine's own tables
//! (`transactions`, the received-note and note-spend tables) live in the same file
//! the aux handle reads. The tip every expiry judgment uses is
//! [`crate::history::scanned_tip`] — `MAX(blocks.height)`, the SAME quantity
//! upstream's `v_transactions.expired_unmined` compares against — so a history row
//! that reads `Expired` reads a delivery state beside it ONLY in the replaced case
//! above (*retry-pending*: this attempt is dead, the payment will be sent again).
//!
//! §5.4 NEVER-LOG: txids and raw bytes pass through here and none is logged.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension};
use zcash_protocol::consensus::BranchId;

use crate::db::map_aux_err;
use crate::error::WalletError;
use crate::intent_store::{
    self, IntentState, NoteClaim, PROTO_IRONWOOD, PROTO_ORCHARD, PROTO_SAPLING,
};
use crate::send::{DepositGate, deposit_gate, deposit_gate_continue, past_expiry};
use crate::state::DeliveryState;

/// The accepted-mark table — referenced by `db::AUX_TABLES_PRESERVED` (ADR-0534).
/// It rides a rescan UNCLEARED: a rescan discards the `transactions` rows the marks
/// describe, and every reading starts from that row, so a surviving mark for a
/// txid the rebuilt store no longer holds as wallet-created is inert.
pub(crate) const ACCEPTED_TABLE: &str = "tx_delivery_accepted";

/// Create the accepted-mark table (idempotent; runs in `db::migrate` on every
/// provision AND open, so existing wallets gain it with no schema bump).
pub(crate) fn ensure_table(conn: &Connection) -> Result<(), WalletError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tx_delivery_accepted (
             txid         BLOB    PRIMARY KEY,
             height       INTEGER,
             at_unix_secs INTEGER NOT NULL
         );",
    )
    .map_err(map_aux_err)
}

/// Record that an endpoint ACCEPTED `txid` (INTERNAL byte order — the form the
/// `transactions.txid` column stores). First-wins: the mark names the FIRST
/// acceptance, and a later duplicate submit's `Accepted` changes nothing. One
/// statement, so it takes `RESERVED` atomically (the aux-write invariant). Loss
/// mode: conservative — see the module doc.
pub(crate) fn mark_accepted(
    conn: &Connection,
    txid: &[u8; 32],
    height: Option<u32>,
    at_unix_secs: u64,
) -> Result<(), WalletError> {
    let at = i64::try_from(at_unix_secs).unwrap_or(i64::MAX);
    conn.execute(
        "INSERT OR IGNORE INTO tx_delivery_accepted (txid, height, at_unix_secs) \
         VALUES (?1, ?2, ?3)",
        rusqlite::params![txid.as_slice(), height.map(i64::from), at],
    )
    .map_err(map_aux_err)?;
    Ok(())
}

/// Whether an accepted mark exists for `txid` (INTERNAL order).
pub(crate) fn is_accepted(conn: &Connection, txid: &[u8; 32]) -> Result<bool, WalletError> {
    conn.query_row(
        "SELECT 1 FROM tx_delivery_accepted WHERE txid = ?1",
        rusqlite::params![txid.as_slice()],
        |_| Ok(()),
    )
    .optional()
    .map_err(map_aux_err)
    .map(|row| row.is_some())
}

/// The txid of a consensus-serialized transaction, or `None` if the bytes do not
/// parse. The accepted mark is keyed by txid, but the broadcast machinery carries
/// raw bytes only (a `BroadcastGroup` is `Vec<Vec<u8>>`), so the mark site derives
/// the id from what it just sent. The branch-id argument matters ONLY to v4-and-
/// earlier transactions, which this SDK never signs (every created transaction is
/// v5+, where the branch id is read from the bytes themselves). A parse failure —
/// or a parser panic on bytes that are not ours, caught the way `enhance` catches
/// it — yields `None`: the caller then writes no mark, the conservative direction (the
/// stricter start gate for a swap-deposit leg).
pub(crate) fn txid_of_raw(raw: &[u8]) -> Option<[u8; 32]> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zcash_primitives::transaction::Transaction::read(raw, BranchId::Nu5).ok()
    }))
    .ok()
    .flatten()
    .map(|tx| *tx.txid().as_ref())
}

/// One transaction the generic phase owes a broadcast: its INTERNAL-order txid
/// and the exact bytes the engine persisted at signing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Obligation {
    pub(crate) txid: [u8; 32],
    pub(crate) raw: Vec<u8>,
}

/// What the live intent rows OWN, read once per pass: every txid an intent row
/// recorded (`Sent`/`Stranded` groups), every txid that SPENDS a note a live claim
/// names (the unrecorded create window), and — for the recorded ones — the row's
/// deposit deadline, which decides whether the intent path is retrying it or
/// holding it.
pub(crate) struct DeliveryView {
    /// Recorded txid → the owning row's `deposit_deadline` (`None` for an ordinary send).
    recorded: HashMap<[u8; 32], Option<i64>>,
    /// Recorded txid → the leg before it in its live group (from `intent.txids` order),
    /// for the start-vs-continue choice the drain makes (F01, `send::rebroadcast_group`).
    predecessor: HashMap<[u8; 32], [u8; 32]>,
    /// Txids attributed through a claim only (a `Submitting` row's notes, spent by a
    /// tx no row recorded) — the intent path waits on these; so does this phase.
    claimed: HashSet<[u8; 32]>,
    /// Recorded txids of a group the wallet WILL send again once it is dead and buried
    /// (S7 C1): the row is LIVE (`Sent`, in flight — not `Stranded`), NO leg of its group
    /// is mined (a two-step whose tx0 mined strands, never re-sends), and it is no swap
    /// deposit (its re-send comes at burial, ~2.9 h on, long after the 15-minute quote has
    /// lapsed, so the drain deletes it instead). Two narrow residuals, stated: a `Submitting`
    /// row's claim-owned txid (the create-committed-but-unrecorded window) reads `None` once
    /// expired, for about one pass before its requeue; and a deposit's "no re-send" leans on
    /// the wall clock — a backward jump of more than ~3 h could revive its quote at burial.
    live: HashSet<[u8; 32]>,
}

impl DeliveryView {
    /// Build the ownership view from the live intent rows. A corrupt in-flight row
    /// fails the whole read (`list_in_flight`'s fail-closed rule): this phase must
    /// not broadcast on "could not prove no row owns it".
    pub(crate) fn load(conn: &Connection) -> Result<Self, WalletError> {
        let mut recorded = HashMap::new();
        let mut predecessor = HashMap::new();
        let mut claimed = HashSet::new();
        let mut live = HashSet::new();
        for intent in intent_store::list_in_flight(conn)? {
            for pair in intent.txids.windows(2) {
                predecessor.insert(pair[1], pair[0]);
            }
            let mut any_leg_mined = false;
            for txid in &intent.txids {
                any_leg_mined |= read_tx_row(conn, txid)?.is_some_and(|r| r.mined_height.is_some());
            }
            let resends = intent.deposit_deadline.is_none() && !any_leg_mined;
            for txid in &intent.txids {
                recorded.insert(*txid, intent.deposit_deadline);
                if resends {
                    live.insert(*txid);
                }
            }
            for claim in &intent.claims {
                for txid in spenders_of(conn, claim)? {
                    claimed.insert(txid);
                }
            }
            // A `Sent` row's claims are spent by its own recorded group; anything
            // else spending them is a reorg-era double we would not broadcast either.
            debug_assert!(
                intent.state != IntentState::Sent || !intent.txids.is_empty(),
                "list_in_flight enforces the Sent ⇒ txids invariant"
            );
        }
        for stranded in intent_store::list_stranded_intents(conn)? {
            for txid in &stranded.txids {
                recorded.insert(*txid, None);
            }
        }
        Ok(Self {
            recorded,
            predecessor,
            claimed,
            live,
        })
    }

    /// Can an intent row own `txid`? (Row 3's attribution rule: by recorded group OR
    /// by claim, never by recorded txid alone.)
    pub(crate) fn owned(&self, txid: &[u8; 32]) -> bool {
        self.recorded.contains_key(txid) || self.claimed.contains(txid)
    }

    /// The per-transaction delivery state of `txid` (INTERNAL order) — the four
    /// readings the module doc names — or `None` for a transaction that is not in
    /// delivery at all: not wallet-created, expired (unless a live `Sent` row will send
    /// the payment again — then `RetryPending`), or not in the store. `tip` is
    /// [`crate::history::scanned_tip`]; `now_unix` gates a held deposit.
    pub(crate) fn state_of(
        &self,
        conn: &Connection,
        tip: Option<u32>,
        now_unix: u64,
        txid: &[u8; 32],
    ) -> Result<Option<DeliveryState>, WalletError> {
        let Some(row) = read_tx_row(conn, txid)? else {
            return Ok(None);
        };
        if !row.wallet_created {
            return Ok(None);
        }
        if row.mined_height.is_some() {
            return Ok(Some(DeliveryState::Confirmed));
        }
        if let Some(tip) = tip
            && past_expiry(row.expiry_height, tip)
        {
            // This attempt is dead. Checked BEFORE the `None`: a group the wallet WILL send
            // again (`live` — see the field) requeues once the expiry buries (S7 C1), so the
            // honest reading is `RetryPending`, never "expired, the amount is yours". A
            // stranded, partly mined or deposit group promises no re-send: `None`.
            return Ok(self
                .live
                .contains(txid)
                .then_some(DeliveryState::RetryPending));
        }
        if is_accepted(conn, txid)? {
            return Ok(Some(DeliveryState::Accepted));
        }
        if let Some(deadline) = self.recorded.get(txid) {
            // Intent-owned: the intent path rebroadcasts it unless the deposit gate
            // holds it (`DepositBroadcastHold`, both the `Expired` and the `Wait` arm).
            // The SAME gate the drain picks (F01): this leg is unmarked (a marked one
            // read `Accepted` above), so it continues only when the leg before it is
            // mined or marked accepted. The same rule as `send::rebroadcast_group`
            // and `wallet::DepositCheck::passes`; change the three together.
            let continues = match self.predecessor.get(txid) {
                Some(prev) => {
                    read_tx_row(conn, prev)?.is_some_and(|r| r.mined_height.is_some())
                        || is_accepted(conn, prev)?
                }
                None => false,
            };
            let gate = if continues {
                deposit_gate_continue(*deadline, now_unix)
            } else {
                deposit_gate(*deadline, now_unix)
            };
            return Ok(Some(match gate {
                DepositGate::Proceed if row.has_raw => DeliveryState::RetryPending,
                _ => DeliveryState::Persisted,
            }));
        }
        if self.claimed.contains(txid) || !row.has_raw {
            // The unrecorded create window (the intent path waits for expiry), or
            // bytes the store no longer holds: kept, not retried.
            return Ok(Some(DeliveryState::Persisted));
        }
        Ok(Some(DeliveryState::RetryPending))
    }
}

/// The generic phase's enumeration: every wallet-created, unmined, unexpired
/// `transactions` row with its raw bytes that no intent row can own, in insertion
/// order. `tip` is [`crate::history::scanned_tip`]; with NO tip (nothing scanned)
/// expiry cannot be judged and every candidate counts as owed — the conservative
/// direction for the rescan fence (a wallet that never scanned cannot have created
/// a transaction, so the drain never meets this arm).
pub(crate) fn enumerate(
    conn: &Connection,
    tip: Option<u32>,
) -> Result<Vec<Obligation>, WalletError> {
    let view = DeliveryView::load(conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT txid, raw, expiry_height FROM transactions \
             WHERE created IS NOT NULL AND mined_height IS NULL AND raw IS NOT NULL \
             ORDER BY id_tx ASC",
        )
        .map_err(map_aux_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Option<u32>>(2)?,
            ))
        })
        .map_err(map_aux_err)?;
    let mut out = Vec::new();
    for row in rows {
        let (txid, raw, expiry) = row.map_err(map_aux_err)?;
        // Validate-never-truncate: a non-32-byte txid blob is corruption, fail typed.
        let txid: [u8; 32] = txid
            .as_slice()
            .try_into()
            .map_err(|_| WalletError::StoreCorrupt)?;
        if let Some(tip) = tip
            && past_expiry(expiry.unwrap_or(0), tip)
        {
            continue;
        }
        if view.owned(&txid) {
            continue;
        }
        out.push(Obligation { txid, raw });
    }
    Ok(out)
}

/// The columns of one `transactions` row a delivery reading needs.
struct TxRow {
    wallet_created: bool,
    mined_height: Option<u32>,
    /// `0` for a NULL expiry (no expiry), like `send::read_tx_expiry`'s callers.
    expiry_height: u32,
    has_raw: bool,
}

fn read_tx_row(conn: &Connection, txid: &[u8; 32]) -> Result<Option<TxRow>, WalletError> {
    conn.query_row(
        "SELECT created IS NOT NULL, mined_height, expiry_height, raw IS NOT NULL \
         FROM transactions WHERE txid = ?1",
        rusqlite::params![txid.as_slice()],
        |r| {
            Ok(TxRow {
                wallet_created: r.get::<_, i64>(0)? != 0,
                mined_height: r.get(1)?,
                expiry_height: r.get::<_, Option<u32>>(2)?.unwrap_or(0),
                has_raw: r.get::<_, i64>(3)? != 0,
            })
        },
    )
    .optional()
    .map_err(map_aux_err)
}

/// Every transaction that SPENDS the note a claim names, through the engine's own
/// received-note ⇄ note-spend junction for the claim's pool. The claim's `txid` is
/// the PRODUCING transaction (INTERNAL order) and `output_index` the note's index
/// in that transaction's bundle — the same two coordinates `send::note_spendable`
/// asks the engine about.
fn spenders_of(conn: &Connection, claim: &NoteClaim) -> Result<Vec<[u8; 32]>, WalletError> {
    let (notes, spends, note_id, index) = match claim.protocol {
        PROTO_SAPLING => (
            "sapling_received_notes",
            "sapling_received_note_spends",
            "sapling_received_note_id",
            "output_index",
        ),
        PROTO_ORCHARD => (
            "orchard_received_notes",
            "orchard_received_note_spends",
            "orchard_received_note_id",
            "action_index",
        ),
        PROTO_IRONWOOD => (
            "ironwood_received_notes",
            "ironwood_received_note_spends",
            "ironwood_received_note_id",
            "action_index",
        ),
        // `decode_claims` admits only the three tags; a fourth here is corruption.
        _ => return Err(WalletError::StoreCorrupt),
    };
    // Table and column NAMES come from the fixed match above, never from data; the
    // two VALUES are bound.
    let sql = format!(
        "SELECT ct.txid FROM {notes} n \
         JOIN transactions pt ON pt.id_tx = n.transaction_id \
         JOIN {spends} s ON s.{note_id} = n.id \
         JOIN transactions ct ON ct.id_tx = s.transaction_id \
         WHERE pt.txid = ?1 AND n.{index} = ?2"
    );
    let mut stmt = conn.prepare(&sql).map_err(map_aux_err)?;
    let rows = stmt
        .query_map(
            rusqlite::params![claim.txid.as_slice(), claim.output_index],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map_err(map_aux_err)?;
    let mut out = Vec::new();
    for row in rows {
        let txid: [u8; 32] = row
            .map_err(map_aux_err)?
            .as_slice()
            .try_into()
            .map_err(|_| WalletError::StoreCorrupt)?;
        out.push(txid);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent_store::{enqueue, mark_sent_multi, mark_submitting};
    use crate::test_support::HistoryHarness;
    use zcash_address::{ToAddress, ZcashAddress};
    use zcash_protocol::consensus::NetworkType;

    /// The engine's persisted bytes for `txid`, straight off the `transactions` row.
    fn stored_raw(conn: &Connection, txid: &[u8; 32]) -> Vec<u8> {
        conn.query_row(
            "SELECT raw FROM transactions WHERE txid = ?1",
            rusqlite::params![txid.as_slice()],
            |r| r.get(0),
        )
        .expect("the created send has a row")
    }

    /// A funded, scanned harness with ONE real signed-and-persisted single-step
    /// send (never broadcast — the R02 shape), plus an aux connection to the same
    /// file carrying the intent and accepted-mark tables. Returns the aux
    /// connection, the send's INTERNAL-order txid and the harness (kept alive so
    /// the temp file stays).
    fn harness_with_one_send() -> (Connection, [u8; 32], HistoryHarness) {
        let mut h = HistoryHarness::new();
        h.mine_received(100_000);
        h.mine_empty(10);
        h.scan();
        let recipient = ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, [0x71; 20]);
        let send = h.create_send(recipient, 20_000);
        let aux = h.read_conn();
        intent_store::ensure_table(&aux).expect("intent table");
        ensure_table(&aux).expect("mark table");
        (aux, *send.as_ref(), h)
    }

    /// The scanned tip the way the wallet reads it.
    fn tip(conn: &Connection) -> Option<u32> {
        crate::history::scanned_tip(conn).expect("tip")
    }

    #[test]
    fn a_persisted_unbroadcast_send_is_owed_with_its_exact_bytes_and_reads_retry_pending() {
        let (aux, send, _h) = harness_with_one_send();
        let owed = enumerate(&aux, tip(&aux)).expect("enumerate");
        assert_eq!(owed.len(), 1, "exactly the one created send is owed");
        assert_eq!(owed[0].txid, send);
        assert_eq!(
            owed[0].raw,
            stored_raw(&aux, &send),
            "the obligation carries the bytes the engine persisted, never a re-sign"
        );
        assert_eq!(
            txid_of_raw(&owed[0].raw),
            Some(send),
            "the mark site derives the same txid from the bytes it sends"
        );
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), 1_800_000_000, &send)
                .expect("state"),
            Some(DeliveryState::RetryPending)
        );
    }

    #[test]
    fn a_received_transaction_is_never_owed_and_has_no_delivery_state() {
        // The decrypt path stores a row with `created IS NULL` — the discriminant
        // the module pins (an upstream that stamps `created` on received rows would
        // fail here before it could broadcast someone else's transaction).
        let mut h = HistoryHarness::new();
        let received = h.mine_received(50_000);
        h.scan();
        let aux = h.read_conn();
        intent_store::ensure_table(&aux).expect("intent table");
        ensure_table(&aux).expect("mark table");
        assert!(enumerate(&aux, tip(&aux)).expect("enumerate").is_empty());
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), 0, received.as_ref())
                .expect("state"),
            None
        );
    }

    #[test]
    fn the_accepted_mark_is_first_wins_and_only_changes_the_reading() {
        let (aux, send, _h) = harness_with_one_send();
        assert!(!is_accepted(&aux, &send).expect("read"));
        mark_accepted(&aux, &send, Some(7), 1_800_000_000).expect("mark");
        mark_accepted(&aux, &send, Some(9), 1_800_000_100).expect("mark again");
        assert!(is_accepted(&aux, &send).expect("read"));
        let (height, at): (Option<i64>, i64) = aux
            .query_row(
                "SELECT height, at_unix_secs FROM tx_delivery_accepted WHERE txid = ?1",
                rusqlite::params![send.as_slice()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("one row");
        assert_eq!(
            (height, at),
            (Some(7), 1_800_000_000),
            "first acceptance wins"
        );
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), 0, &send).expect("state"),
            Some(DeliveryState::Accepted)
        );
        // The LOSS MODE, stated in code: the mark decides no broadcast — an
        // accepted-but-unmined transaction is still owed on every pass.
        let owed = enumerate(&aux, tip(&aux)).expect("enumerate");
        assert_eq!(
            owed.len(),
            1,
            "an accepted, unmined send is still rebroadcast"
        );
    }

    #[test]
    fn a_mined_transaction_leaves_the_obligation_and_reads_confirmed() {
        let (aux, send, _h) = harness_with_one_send();
        // The column the predicate reads, set the way the engine's own CHECK allows
        // (`min_observed_consistency`: a tx mines no lower than it was first observed);
        // the engine's mined-ness semantics are pinned at the wallet level by the
        // stage's named rows.
        aux.execute(
            "UPDATE transactions SET mined_height = min_observed_height WHERE txid = ?1",
            rusqlite::params![send.as_slice()],
        )
        .expect("mine");
        assert!(enumerate(&aux, tip(&aux)).expect("enumerate").is_empty());
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), 0, &send).expect("state"),
            Some(DeliveryState::Confirmed)
        );
    }

    #[test]
    fn an_expired_transaction_leaves_the_obligation_and_has_no_delivery_state() {
        let (aux, send, _h) = harness_with_one_send();
        let expiry: u32 = aux
            .query_row(
                "SELECT expiry_height FROM transactions WHERE txid = ?1",
                rusqlite::params![send.as_slice()],
                |r| r.get(0),
            )
            .expect("expiry");
        assert!(expiry > 0, "a created send carries an expiry");
        // Exactly at the expiry the tx can no longer enter the next block
        // (`past_expiry`: `expiry <= tip`) — the same edge `v_transactions` uses.
        assert!(enumerate(&aux, Some(expiry - 1)).expect("live").len() == 1);
        assert!(enumerate(&aux, Some(expiry)).expect("expired").is_empty());
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, Some(expiry), 0, &send).expect("state"),
            None
        );
        // No tip at all: expiry cannot be judged, so the send counts as owed (the
        // fence's conservative direction).
        assert_eq!(enumerate(&aux, None).expect("no tip").len(), 1);
    }

    #[test]
    fn a_transaction_an_intent_row_can_own_is_excluded_by_claim_and_by_recorded_txid() {
        let (mut aux, send, _h) = harness_with_one_send();
        // The note the send spent, in the claim's coordinates: the PRODUCING tx and
        // the note's output index, read back through the engine's own junction.
        let (producer, output_index): (Vec<u8>, u32) = aux
            .query_row(
                "SELECT pt.txid, n.output_index FROM sapling_received_notes n \
                 JOIN transactions pt ON pt.id_tx = n.transaction_id \
                 JOIN sapling_received_note_spends s ON s.sapling_received_note_id = n.id \
                 JOIN transactions ct ON ct.id_tx = s.transaction_id \
                 WHERE ct.txid = ?1",
                rusqlite::params![send.as_slice()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("the send spent one sapling note");
        let claim = NoteClaim {
            txid: producer.as_slice().try_into().expect("32 bytes"),
            protocol: PROTO_SAPLING,
            output_index,
        };
        let id = enqueue(&mut aux, "zcash:x", 1, None, None).expect("enqueue");
        // A Queued row (no claim yet) owns nothing.
        assert_eq!(enumerate(&aux, tip(&aux)).expect("queued").len(), 1);

        // The create-committed-but-unrecorded window: the claim is recorded, no
        // txid is — attribution through the spent note excludes the send anyway.
        assert!(mark_submitting(&mut aux, id, &[claim]).expect("submitting"));
        assert!(
            enumerate(&aux, tip(&aux)).expect("submitting").is_empty(),
            "a tx that spends a claimed note is the intent path's, even with no txid recorded"
        );
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), 1_800_000_000, &send)
                .expect("state"),
            Some(DeliveryState::Persisted),
            "the intent path waits for expiry here; the bytes are kept, not retried"
        );

        // Recorded on the row: still excluded, and the intent path retries it.
        assert!(mark_sent_multi(&mut aux, id, &[send]).expect("sent"));
        assert!(enumerate(&aux, tip(&aux)).expect("sent").is_empty());
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), 1_800_000_000, &send)
                .expect("state"),
            Some(DeliveryState::RetryPending)
        );
    }

    #[test]
    fn a_held_deposit_reads_persisted_under_both_hold_arms() {
        let (mut aux, send, _h) = harness_with_one_send();
        // A deposit-tagged row recording the send: the HELD shape (row alive, gate
        // `Expired`, tx unexpired) and the `Wait` arm (untimeable clock).
        let deadline: i64 = 1_800_000_000;
        let id = enqueue(&mut aux, "zcash:x", 1, Some(deadline), None).expect("enqueue");
        let claim = NoteClaim {
            txid: [7; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        };
        assert!(mark_submitting(&mut aux, id, &[claim]).expect("submitting"));
        assert!(mark_sent_multi(&mut aux, id, &[send]).expect("sent"));
        assert!(enumerate(&aux, tip(&aux)).expect("owned").is_empty());
        let view = DeliveryView::load(&aux).expect("view");
        let live = crate::constants::CLOCK_PLAUSIBILITY_FLOOR_SECS + 1;
        assert_eq!(
            view.state_of(&aux, tip(&aux), live, &send).expect("live"),
            Some(DeliveryState::RetryPending),
            "a live-deadline deposit is the intent path's to retry"
        );
        assert_eq!(
            view.state_of(&aux, tip(&aux), 1_900_000_000, &send)
                .expect("lapsed"),
            Some(DeliveryState::Persisted),
            "the Expired arm holds: kept, not retried"
        );
        assert_eq!(
            view.state_of(&aux, tip(&aux), 0, &send).expect("unsynced"),
            Some(DeliveryState::Persisted),
            "the Wait arm holds too"
        );
    }

    /// F01 (the 2026-10-05 review, plan §2.3 item 6): the view picks the SAME gate the drain
    /// does for a deposit's second leg, 150 s before the deadline (inside the 240 s start margin,
    /// outside the 90 s continue margin). Predecessor marked or mined ⇒ it continues and reads
    /// `RetryPending`; neither ⇒ a start, held, `Persisted`. (The drain's own cases are
    /// `send::tests::the_drain_*`.)
    #[test]
    fn the_delivery_view_agrees_with_the_drain() {
        let deadline: i64 = 1_900_000_000;
        let now = (deadline - 150) as u64;
        let record_pair = |aux: &mut Connection, pred: [u8; 32], send: [u8; 32]| {
            let id = enqueue(aux, "zcash:tex", 1, Some(deadline), None).expect("enqueue");
            let claim = NoteClaim {
                txid: [7; 32],
                protocol: PROTO_SAPLING,
                output_index: 0,
            };
            assert!(mark_submitting(aux, id, &[claim]).expect("submitting"));
            assert!(mark_sent_multi(aux, id, &[pred, send]).expect("sent"));
        };

        // Neither: a start, held inside the start margin.
        let (mut aux, send, _h) = harness_with_one_send();
        let pred = [0x55; 32];
        record_pair(&mut aux, pred, send);
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), now, &send).expect("state"),
            Some(DeliveryState::Persisted),
            "nothing on the network: the start gate holds"
        );

        // Predecessor marked accepted: it continues.
        mark_accepted(&aux, &pred, None, 1).expect("mark");
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), now, &send).expect("state"),
            Some(DeliveryState::RetryPending),
            "an accepted predecessor: the continue gate proceeds"
        );

        // Predecessor mined: it continues.
        let mut h = HistoryHarness::new();
        let mined_leg = h.mine_received(100_000);
        h.mine_empty(10);
        h.scan();
        let recipient = ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, [0x71; 20]);
        let send = *h.create_send(recipient, 20_000).as_ref();
        let mut aux = h.read_conn();
        intent_store::ensure_table(&aux).expect("intent table");
        ensure_table(&aux).expect("mark table");
        record_pair(&mut aux, *mined_leg.as_ref(), send);
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, tip(&aux), now, &send).expect("state"),
            Some(DeliveryState::RetryPending),
            "a mined predecessor: the continue gate proceeds"
        );
    }

    #[test]
    fn txid_of_raw_rejects_bytes_that_are_not_a_transaction() {
        assert_eq!(txid_of_raw(&[0x05, 0x00, 0x00, 0x80]), None);
        assert_eq!(txid_of_raw(&[]), None);
    }

    /// THE DEATH-POINT SEAM, PINNED (the `connect_unchecked` rule, stage S1): the
    /// persist half of a send is `Wallet::sign_proposal_upstream`, reached from
    /// exactly two places — the production `send_by_id` (which always broadcasts
    /// next) and the `cfg(test)` `sign_proposal` door, which is how a test dies
    /// "after the engine's persist, before `broadcast_persisted`". A third caller
    /// is a production path that can persist a transaction and never broadcast it,
    /// and this test refuses it. Textual, like its precedent: the worker is
    /// private to `wallet.rs`, so any new caller lands in this file.
    #[test]
    fn the_persist_half_of_a_send_has_exactly_its_two_callers() {
        let source = include_str!("wallet.rs");
        let needle = concat!("sign_proposal_upstream", "(");
        assert_eq!(
            source.matches(needle).count(),
            3,
            "one definition + two callers of the persist-without-broadcast worker"
        );
    }

    /// The expiry height the engine persisted for `txid`.
    fn expiry_of(conn: &Connection, txid: &[u8; 32]) -> u32 {
        conn.query_row(
            "SELECT expiry_height FROM transactions WHERE txid = ?1",
            rusqlite::params![txid.as_slice()],
            |r| r.get(0),
        )
        .expect("expiry")
    }

    /// Record `send` on a fresh intent row as `Sent` (a synthetic claim — the reading keys on
    /// the recorded group, not on the note).
    fn record_sent(
        aux: &mut Connection,
        send: [u8; 32],
        deadline: Option<i64>,
    ) -> crate::state::QueuedSendId {
        let id = enqueue(aux, "zcash:x", 1, deadline, None).expect("enqueue");
        let claim = NoteClaim {
            txid: [7; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        };
        assert!(mark_submitting(aux, id, &[claim]).expect("submitting"));
        assert!(mark_sent_multi(aux, id, &[send]).expect("sent"));
        id
    }

    /// S7 C1: past its expiry, a transaction a LIVE `Sent` row owns reads `RetryPending` — the
    /// group requeue WILL send the payment again once the expiry buries, and "expired, the
    /// amount is yours" beside it would invite a second payment by hand. Once the row requeued
    /// (the txid no longer recorded) or stranded, the dead attempt reads `None` again.
    #[test]
    fn an_expired_send_a_live_intent_will_replace_reads_retry_pending() {
        let (mut aux, send, _h) = harness_with_one_send();
        let expiry = expiry_of(&aux, &send);
        let id = record_sent(&mut aux, send, None);
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, Some(expiry), 1_800_000_000, &send)
                .expect("state"),
            Some(DeliveryState::RetryPending),
            "a live Sent row will send the payment again — never None at bare expiry"
        );

        // The burial requeue clears the group: the dead attempt is nobody's now.
        assert!(intent_store::reset_to_queued(&mut aux, id).expect("requeue"));
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, Some(expiry), 1_800_000_000, &send)
                .expect("state"),
            None,
            "requeued: the old txid is no longer the row's"
        );

        // A Stranded row never re-sends: its expired transaction reads None.
        let id = record_sent(&mut aux, send, None);
        assert!(intent_store::mark_stranded(&mut aux, id).expect("strand"));
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, Some(expiry), 1_800_000_000, &send)
                .expect("state"),
            None,
            "a Stranded row's expired transaction promises no re-send"
        );
    }

    /// S7 C1, the deposit arm (the S7 fold): an expired deposit is NEVER re-sent — its re-send
    /// would come at burial (~2.9 h on), when the ≤ 15-minute quote has always lapsed and the
    /// drain deletes the row. So it reads `None` under every clock, a live deadline included.
    #[test]
    fn an_expired_deposit_never_promises_a_resend() {
        let (mut aux, send, _h) = harness_with_one_send();
        let expiry = expiry_of(&aux, &send);
        let deadline: i64 = 1_800_000_000;
        record_sent(&mut aux, send, Some(deadline));
        let view = DeliveryView::load(&aux).expect("view");
        let live = crate::constants::CLOCK_PLAUSIBILITY_FLOOR_SECS + 1;
        assert_eq!(
            view.state_of(&aux, Some(expiry), live, &send)
                .expect("live"),
            None,
            "even a live-deadline deposit lapses before its burial re-send — no promise"
        );
        assert_eq!(
            view.state_of(&aux, Some(expiry), 1_900_000_000, &send)
                .expect("lapsed"),
            None,
            "a lapsed deposit is deleted by the drain, never re-sent"
        );
        assert_eq!(
            view.state_of(&aux, Some(expiry), 0, &send)
                .expect("unsynced"),
            None,
            "an untimeable clock promises nothing"
        );
    }

    /// S7 C1 (the S7 fold): a group with a MINED leg is never re-sent — a two-step whose tx0
    /// mined strands once its later leg is dead, so that expired later leg reads `None`, not
    /// a promised re-send.
    #[test]
    fn an_expired_leg_of_a_group_with_a_mined_leg_promises_no_resend() {
        let mut h = HistoryHarness::new();
        let mined_leg = h.mine_received(100_000);
        h.mine_empty(10);
        h.scan();
        let recipient = ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, [0x71; 20]);
        let send = *h.create_send(recipient, 20_000).as_ref();
        let mut aux = h.read_conn();
        intent_store::ensure_table(&aux).expect("intent table");
        ensure_table(&aux).expect("mark table");
        let expiry = expiry_of(&aux, &send);
        let id = enqueue(&mut aux, "zcash:tex", 1, None, None).expect("enqueue");
        let claim = NoteClaim {
            txid: [7; 32],
            protocol: PROTO_SAPLING,
            output_index: 0,
        };
        assert!(mark_submitting(&mut aux, id, &[claim]).expect("submitting"));
        assert!(mark_sent_multi(&mut aux, id, &[*mined_leg.as_ref(), send]).expect("sent"));
        let view = DeliveryView::load(&aux).expect("view");
        assert_eq!(
            view.state_of(&aux, Some(expiry), 1_800_000_000, &send)
                .expect("state"),
            None,
            "leg 0 mined + leg 1 at bare expiry: the group strands, it never re-sends"
        );
    }
}
