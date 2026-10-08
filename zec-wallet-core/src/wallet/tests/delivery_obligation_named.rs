//! Stage S8 `obligation` — the TEST AUTHOR's rows that name a SYMBOL the
//! contract leaves to the implementer (`docs/plan/stage-8-payment-identity-
//! and-durable-retry.md` §3.2 rows 3, 5, 6, 9 and 10). They cannot compile at
//! the base commit, so they ride their own commit, written against the names
//! the author declared in the run's `contractFindings` for the adjudicator to
//! join:
//!
//! - `crate::state::DeliveryState::{Persisted, RetryPending, Accepted,
//!   Confirmed}` and `TxSummary::delivery: Option<DeliveryState>` — the
//!   per-transaction delivery state a history row carries (row 10): the bytes
//!   are kept / the wallet is still trying / a server accepted it / it is in
//!   the chain;
//! - `crate::send::unowned_signed_transactions(&rusqlite::Connection)
//!   -> Result<Vec<[u8; 32]>, WalletError>` — the generic phase's enumeration
//!   in INTERNAL txid order: the wallet-created (`created IS NOT NULL`),
//!   unmined, unexpired `transactions` rows no live intent row can own, on
//!   any connection to the wallet file (row 3's attribution; rows 5, 6 and
//!   9's exclusions at the predicate);
//! - `HistoryHarness::create_send_with_claims` — `create_send` returning the
//!   proposal's note claims beside the txid (`test_support.rs`, this commit).
//!
//! The wallet-level rows, and the loopback lightwalletd and funded-wallet
//! fixtures shared here, are the sibling `delivery_obligation.rs`'s.

use super::delivery_obligation::{
    Answer, LoopbackLightwalletd, SYNCED_TIP, config_over, enrol_sent_row, funded_wallet,
    mark_mined, now_unix, persist_shield,
};
use super::*;
use crate::intent_store::{self, NoteClaim};
use crate::state::{DeliveryState, TxStatus};
use crate::test_support::HistoryHarness;

/// A funded history harness holding one RECEIVED transaction (scanned from a
/// compact block — `created IS NULL`) and one REAL single-step send signed over
/// the product path (`propose_core` → `create_signed_core`), plus the second
/// connection the enumeration's SQL rides, with the intent table on it.
struct SendUnderTest {
    harness: HistoryHarness,
    received: zcash_protocol::TxId,
    send: zcash_protocol::TxId,
    claims: Vec<NoteClaim>,
    conn: rusqlite::Connection,
}

fn a_send_under_test() -> SendUnderTest {
    use zcash_address::{ToAddress, ZcashAddress};
    use zcash_protocol::consensus::NetworkType;

    let mut harness = HistoryHarness::new();
    let received = harness.mine_received(100_000);
    harness.mine_empty(10);
    harness.scan();
    let recipient = ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, [0x71; 20]);
    let (send, claims) = harness.create_send_with_claims(recipient, 20_000);
    let conn = harness.read_conn();
    intent_store::ensure_table(&conn).expect("the intent table on the same file");
    SendUnderTest {
        harness,
        received,
        send,
        claims,
        conn,
    }
}

fn enumerated(conn: &rusqlite::Connection) -> Vec<[u8; 32]> {
    // JOIN SPELLING (adjudicator): the author declared
    // `send::unowned_signed_transactions(conn)`; the implementer built
    // `delivery::enumerate(conn, tip)` — the same predicate over the same rows,
    // with the scanned tip as its expiry basis and `Obligation { txid, raw }`
    // rows. The implementer's spelling; every assertion in this file is the
    // author's.
    let tip = crate::history::scanned_tip(conn).expect("tip");
    crate::delivery::enumerate(conn, tip)
        .expect("enumerate")
        .into_iter()
        .map(|o| o.txid)
        .collect()
}

// ── §3.2 row 3 at the predicate: attribution, never the recorded txid alone ──

/// Row 3 — the create-committed-but-unrecorded window. The intent recorded its
/// CLAIM (the notes the send spends) and the process died before the txid was
/// recorded: no row carries the txid, and the transaction is owned all the
/// same — by attribution through the claim — because the intent path
/// deliberately never broadcasts it (it waits for expiry). A recorded txid
/// owns it too; a deleted row frees it.
#[test]
fn the_generic_phase_never_touches_a_transaction_an_intent_row_owns() {
    let SendUnderTest {
        harness: _harness,
        send,
        claims,
        mut conn,
        ..
    } = a_send_under_test();
    let internal = *send.as_ref();
    assert_eq!(
        enumerated(&conn),
        vec![internal],
        "control: with no intent row the signed send is the phase's to deliver"
    );

    let id = intent_store::enqueue(&mut conn, "zcash:owner", 0, None, None).expect("enqueue");
    assert!(intent_store::mark_submitting(&mut conn, id, &claims).expect("submitting"));
    assert!(
        enumerated(&conn).is_empty(),
        "Submitting — the claim recorded, NO txid: owned by attribution, never broadcast here"
    );

    assert!(intent_store::mark_sent_multi(&mut conn, id, &[internal]).expect("sent"));
    assert!(
        enumerated(&conn).is_empty(),
        "Sent — the txid recorded: the intent machinery's group, never this phase's"
    );

    assert!(intent_store::delete(&mut conn, id).expect("delete"));
    assert_eq!(
        enumerated(&conn),
        vec![internal],
        "no live row can own it: unowned again"
    );
}

// ── §3.2 rows 5, 6 and 9 at the predicate ───────────────────────────────────

/// Row 9 at the predicate: a transaction the wallet RECEIVED (`created IS
/// NULL` on the pinned upstream) is never in the enumeration. The harness's
/// scanned receipt is MINED, which `mined_height IS NULL` excludes whatever
/// `created` says — so the guard against an upstream bump that starts stamping
/// `created` on received rows is the UNMINED received row planted beside it:
/// the decrypt path's shape (`created IS NULL`, no expiry), the send's own
/// serialized bytes under another txid, which only the `created` discriminant
/// keeps out — the row would then list two. (Repaired at the adjudication,
/// ruling.md §6/§9: the row survived the very mutation its doc claimed to pin.)
#[test]
fn a_received_transaction_is_never_enumerated() {
    let SendUnderTest {
        harness,
        received,
        send,
        conn,
        ..
    } = a_send_under_test();
    let raw: Vec<u8> = conn
        .query_row(
            "SELECT raw FROM transactions WHERE txid = ?1",
            rusqlite::params![send.as_ref()],
            |r| r.get(0),
        )
        .expect("the send's persisted bytes");
    let unmined_received = [0x5a_u8; 32];
    conn.execute(
        "INSERT INTO transactions (txid, raw, expiry_height, min_observed_height, created) \
         VALUES (?1, ?2, NULL, ?3, NULL)",
        rusqlite::params![unmined_received.as_slice(), raw, harness.current_tip()],
    )
    .expect("insert an unmined received row");

    let listed = enumerated(&conn);
    assert_eq!(listed, vec![*send.as_ref()], "exactly the created send");
    assert!(
        !listed.contains(received.as_ref()),
        "the received (and mined) transaction is not an obligation"
    );
    assert!(
        !listed.contains(&unmined_received),
        "the received UNMINED transaction is not an obligation: `created` is the discriminant"
    );
}

/// Row 6 at the predicate: mined, the send leaves the enumeration.
#[test]
fn a_mined_transaction_leaves_the_enumeration() {
    let SendUnderTest {
        mut harness,
        send,
        conn,
        ..
    } = a_send_under_test();
    assert_eq!(
        enumerated(&conn),
        vec![*send.as_ref()],
        "control: unmined, listed"
    );
    let tip = harness.current_tip();
    harness.mark_mined_at(send, tip);
    assert!(enumerated(&conn).is_empty(), "mined: never rebroadcast");
}

/// Row 5 at the predicate: once the scanned tip passes the row's own
/// `expiry_height` (read off the row, never a constant), the send leaves the
/// enumeration — expiry ends it.
#[test]
fn an_expired_transaction_leaves_the_enumeration() {
    let SendUnderTest {
        mut harness,
        send,
        conn,
        ..
    } = a_send_under_test();
    assert_eq!(
        enumerated(&conn),
        vec![*send.as_ref()],
        "control: unexpired, listed"
    );
    let expiry = harness.expiry_height_of(send);
    let tip = harness.current_tip();
    assert!(
        expiry > tip,
        "fixture: a fresh send expires ahead of the tip"
    );
    harness.mine_empty((expiry - tip + 1) as usize);
    harness.scan();
    assert!(enumerated(&conn).is_empty(), "expired: never rebroadcast");
}

// ── §3.2 row 10: the four states, on the per-transaction status ─────────────

async fn delivery_of(w: &Wallet, txid: zcash_protocol::TxId) -> Option<DeliveryState> {
    w.transaction(TxId::from(txid))
        .await
        .expect("history read")
        .expect("the shield is a history row")
        .delivery
}

/// Row 10, the core half: the four states on the history row, read where the
/// activity list reads transaction status, beside `TxStatus`. An unowned
/// persisted transaction is OWED from the moment it is persisted (the next pass
/// attempts it), so its first reading is `RetryPending` — still trying.
/// `Persisted` — kept, no promise — is reached through a shape the contract
/// names: a `Sent` intent row holding the transaction under a lapsed
/// `deposit_deadline` (the deposit hold's `Expired` arm, row 3), which the
/// intent path keeps and the generic phase never touches; the row deleted, the
/// send is owed again. Then a no-verdict attempt (still trying), acceptance,
/// and the chain. (Repaired at the adjudication, ruling.md §6: the first leg
/// presumed an "attempted" mark the contract never names.)
#[tokio::test]
async fn the_four_delivery_states_cross_the_bridge() {
    let (peer, endpoint) = LoopbackLightwalletd::serve(Answer::HangUp).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();

    let w = funded_wallet(&vault, config_over(dir.path(), TorPolicy::Off, endpoint)).await;
    let txid = persist_shield(&w).await;
    assert_eq!(
        delivery_of(&w, txid).await,
        Some(DeliveryState::RetryPending),
        "signed and kept, no attempt yet: owed from the persist — the wallet is still trying"
    );

    let held = enrol_sent_row(&w, &[*txid.as_ref()], Some(now_unix() - 3_600));
    assert_eq!(
        delivery_of(&w, txid).await,
        Some(DeliveryState::Persisted),
        "held by the intent path: the bytes are kept, nothing is retrying them"
    );
    {
        let mut aux = w.inner.aux_db.lock().expect("aux db");
        assert!(intent_store::delete(&mut aux, held).expect("delete"));
    }
    assert_eq!(
        delivery_of(&w, txid).await,
        Some(DeliveryState::RetryPending),
        "fixture control: the row gone, the send is the generic phase's again"
    );

    let miss = w.broadcast_persisted(vec![txid]).await.expect("broadcast");
    assert!(matches!(
        miss.as_slice(),
        [TxSubmitResult::GrpcFailure { .. }]
    ));
    assert_eq!(
        delivery_of(&w, txid).await,
        Some(DeliveryState::RetryPending),
        "no verdict: the wallet is still trying"
    );

    peer.set_answer(Answer::Accept);
    let ok = w.broadcast_persisted(vec![txid]).await.expect("broadcast");
    assert!(matches!(ok.as_slice(), [TxSubmitResult::Success { .. }]));
    assert_eq!(
        delivery_of(&w, txid).await,
        Some(DeliveryState::Accepted),
        "a server accepted it"
    );

    mark_mined(&w, txid, SYNCED_TIP as u32);
    assert_eq!(
        delivery_of(&w, txid).await,
        Some(DeliveryState::Confirmed),
        "it is in the chain"
    );
    let row = w
        .transaction(TxId::from(txid))
        .await
        .expect("history read")
        .expect("row");
    assert!(
        matches!(row.status, TxStatus::Confirmed { .. }),
        "and the existing status agrees: {:?}",
        row.status
    );
    w.close().await.expect("close");
}
