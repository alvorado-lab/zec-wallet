//! WL-4 (`docs/plan/wl4-queued-identity.md`): a queued send is signed only as the row
//! that was read. The engine-side rows, over a FUNDED harness, so "signed" is a real
//! transaction: the resubmission pass lists its rows once, the aux lock is free while the
//! rows ahead prove, and `queued_send_intent.id` is reused (`INTEGER PRIMARY KEY`, no
//! `AUTOINCREMENT`), so a row that was cancelled and replaced keeps the snapshot's id.
//! The pass itself, paused between two rows, is in `wallet/tests/queued_identity.rs`.

use super::*;
use crate::intent_store::{self, QueuedIntent};
// The engine's own function, by a name no helper of `send::tests` can shadow.
use crate::send::prepare_queued as engine_prepare_queued;

/// Above `CLOCK_PLAUSIBILITY_FLOOR_SECS`, so a deposit's gate judges its tag.
const NOW: u64 = CLOCK_PLAUSIBILITY_FLOOR_SECS + 100_000;
/// Every request here pays this much: a fixture with more notes than sends is never the
/// reason a row is not signed.
const AMOUNT: u64 = NOTE_VALUE / 4;
/// The transparent key hash each payment goes to, readable in the signed bytes.
const TO_A: [u8; 20] = [0xa1; 20];
const TO_B: [u8; 20] = [0xb1; 20];
const TO_REPLACEMENT: [u8; 20] = [0xb2; 20];

fn paying(hash: [u8; 20]) -> zip321::TransactionRequest {
    let to = ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, hash);
    request(vec![(to, AMOUNT)])
}

fn pays(raw: &[u8], hash: [u8; 20]) -> bool {
    raw.windows(hash.len()).any(|w| w == hash)
}

/// One row driven as a pass drives it: the row as it was READ, and the request of the
/// payment that read row holds.
fn drive(
    st: &mut HarnessState,
    aux: &mut Connection,
    row: &QueuedIntent,
    request: zip321::TransactionRequest,
) -> Prepared {
    let network = *st.network();
    let account = st.test_account().expect("acct").id();
    let usk = st.test_account().expect("acct").usk().clone();
    let prover = crate::prover::tx_prover();
    engine_prepare_queued(
        st.wallet_mut(),
        &network,
        aux,
        account,
        request,
        usk,
        prover,
        prover,
        row,
        NOW,
        crate::consensus::SigningPermit::assume_current_for_test(),
    )
    .expect("prepare")
}

fn listed(aux: &Connection) -> Vec<QueuedIntent> {
    intent_store::list_queued(aux).expect("list")
}

/// A pass listed A and B and signed A. Then B was cancelled (the call answered true) and
/// another payment queued: it took B's id. Returns the pass's snapshot of B.
fn cancelled_and_replaced(queued_at: i64) -> (HarnessState, Connection, QueuedIntent) {
    let mut st = funded_multi(&[NOTE_VALUE, NOTE_VALUE, NOTE_VALUE]);
    let mut aux = aux_conn();
    let bind = || Some(SpendBinding::mint());
    intent_store::enqueue(&mut aux, "zcash:to-a", 1, None, bind()).expect("queue A");
    let b = intent_store::enqueue(&mut aux, "zcash:to-b", 2, None, bind()).expect("queue B");
    let pass = listed(&aux);
    assert_eq!(pass.len(), 2, "fixture: the pass listed A and B");
    let a = drive(&mut st, &mut aux, &pass[0], paying(TO_A));
    assert!(
        matches!(a, Prepared::Broadcast(_)),
        "fixture: A is signed, got {a:?}"
    );
    assert!(
        intent_store::cancel_queued(&mut aux, b, 2).expect("cancel B"),
        "fixture: the user is told B is cancelled"
    );
    let replacement =
        intent_store::enqueue(&mut aux, "zcash:to-replacement", queued_at, None, bind())
            .expect("queue the replacement");
    assert_eq!(replacement, b, "fixture: the replacement took B's id");
    (st, aux, pass[1].clone())
}

#[test]
fn a_drained_send_cancelled_and_replaced_mid_pass_is_never_signed() {
    // The same second as B, then a later one.
    for queued_at in [2, 3] {
        let (mut st, mut aux, stale_b) = cancelled_and_replaced(queued_at);
        let out = drive(&mut st, &mut aux, &stale_b, paying(TO_B));
        assert!(
            !matches!(out, Prepared::Broadcast(_)),
            "the cancelled payment is never signed (queued at {queued_at}), got {out:?}"
        );
        assert_eq!(
            intent_store::list_in_flight(&aux).expect("in flight").len(),
            1,
            "only A went out: the replacement is not marked sent (queued at {queued_at})"
        );
        let left = listed(&aux);
        assert_eq!(left.len(), 1, "the replacement is still queued");
        assert_eq!(
            (left[0].uri.as_str(), left[0].created_at),
            ("zcash:to-replacement", queued_at),
            "as itself, untouched"
        );
    }
}

#[test]
fn the_next_pass_signs_the_replacement_as_itself() {
    let (mut st, mut aux, stale_b) = cancelled_and_replaced(2);
    let _ = drive(&mut st, &mut aux, &stale_b, paying(TO_B));
    let next = listed(&aux);
    assert_eq!(next.len(), 1, "the next pass lists the replacement");
    let Prepared::Broadcast(raws) = drive(&mut st, &mut aux, &next[0], paying(TO_REPLACEMENT))
    else {
        panic!("the next pass signs the replacement");
    };
    assert_eq!(raws.len(), 1);
    assert!(
        pays(&raws[0], TO_REPLACEMENT) && !pays(&raws[0], TO_B),
        "the replacement's own recipient is paid, the cancelled one is not"
    );
    assert_eq!(
        intent_store::list_in_flight(&aux).expect("in flight").len(),
        2,
        "A and the replacement"
    );
}

/// A row is read, then it is deleted and `replace` writes another under its id that
/// differs from it in ONE field. The read row's request must not be signed under it.
fn never_signed_after(
    field: &str,
    bound: Option<SpendBinding>,
    replace: impl FnOnce(&mut Connection),
) {
    let mut st = funded(NOTE_VALUE);
    let mut aux = aux_conn();
    let id = intent_store::enqueue(&mut aux, "zcash:to-b", 2, None, bound).expect("queue");
    let read = listed(&aux).remove(0);
    assert!(intent_store::delete(&mut aux, id).expect("delete"));
    replace(&mut aux);
    let left = listed(&aux);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, id, "fixture: the id was reused");
    let out = drive(&mut st, &mut aux, &read, paying(TO_B));
    assert!(
        !matches!(out, Prepared::Broadcast(_)),
        "a row with another {field} is not the row that was read, got {out:?}"
    );
    assert_eq!(listed(&aux).len(), 1, "it stays queued");
}

#[test]
fn a_row_with_another_created_at_is_never_signed_as_the_one_read() {
    let bound = Some(SpendBinding::mint());
    never_signed_after("created_at", bound, |aux| {
        intent_store::enqueue(aux, "zcash:to-b", 3, None, bound).expect("queue");
    });
}

#[test]
fn a_row_with_another_uri_is_never_signed_as_the_one_read() {
    let bound = Some(SpendBinding::mint());
    never_signed_after("uri", bound, |aux| {
        intent_store::enqueue(aux, "zcash:to-replacement", 2, None, bound).expect("queue");
    });
}

#[test]
fn a_row_with_another_deadline_is_never_signed_as_the_one_read() {
    let bound = Some(SpendBinding::mint());
    let live = i64::try_from(NOW).expect("clock") + 10_000;
    never_signed_after("deposit_deadline", bound, |aux| {
        intent_store::enqueue(aux, "zcash:to-b", 2, Some(live), bound).expect("queue");
    });
}

#[test]
fn a_row_with_another_binding_is_never_signed_as_the_one_read() {
    let mint = SpendBinding::mint;
    never_signed_after("binding", Some(mint()), |aux| {
        intent_store::enqueue(aux, "zcash:to-b", 2, None, Some(mint())).expect("queue");
    });
}

/// The stored blob is compared, not what it decodes to: a NULL binding and one of the
/// wrong length both decode to none.
#[test]
fn a_null_binding_and_a_broken_one_are_two_different_rows() {
    never_signed_after("stored binding", None, |aux| {
        intent_store::enqueue(aux, "zcash:to-b", 2, None, None).expect("queue");
        aux.execute("UPDATE queued_send_intent SET binding = zeroblob(31)", [])
            .expect("a wrong-length blob");
    });
}

/// The control of the row above: a row whose own blob is broken still equals itself.
#[test]
fn a_row_with_a_broken_binding_is_signed_as_itself() {
    let mut st = funded(NOTE_VALUE);
    let mut aux = aux_conn();
    intent_store::enqueue(&mut aux, "zcash:to-b", 2, None, None).expect("queue");
    aux.execute("UPDATE queued_send_intent SET binding = zeroblob(31)", [])
        .expect("a wrong-length blob");
    let row = listed(&aux).remove(0);
    let out = drive(&mut st, &mut aux, &row, paying(TO_B));
    assert!(matches!(out, Prepared::Broadcast(_)), "got {out:?}");
}

#[test]
fn a_lapsed_snapshot_never_deletes_the_row_that_took_its_id() {
    let mut st = funded(NOTE_VALUE);
    let mut aux = aux_conn();
    let lapsed = i64::try_from(NOW).expect("clock") - 10_000;
    let id = intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(lapsed), None)
        .expect("queue the deposit");
    let read = listed(&aux).remove(0);
    assert_eq!(
        purge_lapsed_queued_deposits(&mut aux, NOW).expect("purge"),
        1,
        "fixture: a pass purged the lapsed deposit"
    );
    let taken = intent_store::enqueue(&mut aux, "zcash:ordinary", 1, None, None).expect("queue");
    assert_eq!(taken, id, "fixture: an ordinary send took its id");
    let out = drive(&mut st, &mut aux, &read, paying(TO_B));
    let left = listed(&aux);
    assert_eq!(
        left.len(),
        1,
        "the ordinary send is not deleted, got {out:?}"
    );
    assert_eq!(left[0].uri, "zcash:ordinary");
}

/// The state half of the pin: the snapshot's deposit was claimed and signed by another
/// caller, and the snapshot's own clock then reads its quote lapsed. A signed row's
/// record is never deleted by it.
#[test]
fn a_lapsed_snapshot_never_deletes_a_row_signed_since() {
    let mut st = funded(NOTE_VALUE);
    let mut aux = aux_conn();
    let lapsed = i64::try_from(NOW).expect("clock") - 10_000;
    let id = intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(lapsed), None)
        .expect("queue the deposit");
    let read = listed(&aux).remove(0);
    let proposal = propose_on(&mut st, paying(TO_B)).expect("propose");
    let claims = claim_from_proposal(&proposal).expect("claim");
    assert!(intent_store::mark_submitting(&mut aux, id, &claims).expect("submitting"));
    assert!(intent_store::mark_sent_multi(&mut aux, id, &[[0x5e; 32]]).expect("sent"));
    let out = drive(&mut st, &mut aux, &read, paying(TO_B));
    assert_eq!(
        intent_store::list_in_flight(&aux).expect("in flight").len(),
        1,
        "the signed row keeps its record, got {out:?}"
    );
}

/// The same rule one phase earlier (the plan's I3): a reconcile left running by a pass
/// whose future was dropped acts on a row another pass has finished and another send
/// replaced.
#[test]
fn a_stale_reconcile_never_deletes_the_row_that_took_its_id() {
    let mut st = funded(NOTE_VALUE);
    let mut aux = aux_conn();
    let lapsed = i64::try_from(NOW).expect("clock") - 10_000;
    let id = intent_store::enqueue(&mut aux, "zcash:deposit", 1, Some(lapsed), None)
        .expect("queue the deposit");
    let proposal = propose_on(&mut st, paying(TO_B)).expect("propose");
    let claims = claim_from_proposal(&proposal).expect("claim");
    assert!(intent_store::mark_submitting(&mut aux, id, &claims).expect("submitting"));
    let read = intent_store::list_in_flight(&aux).expect("in flight")[0].clone();
    assert_eq!(
        reconcile_inflight(st.wallet(), st.network(), &mut aux, &read, NOW).expect("reconcile"),
        Reconciled::DepositExpired,
        "fixture: the running pass ends the never-created lapsed deposit"
    );
    let taken = intent_store::enqueue(&mut aux, "zcash:ordinary", 1, None, None).expect("queue");
    assert_eq!(taken, id, "fixture: an ordinary send took its id");
    let out =
        reconcile_inflight(st.wallet(), st.network(), &mut aux, &read, NOW).expect("reconcile");
    assert_eq!(
        listed(&aux).len(),
        1,
        "the ordinary send is not deleted, got {out:?}"
    );
}
