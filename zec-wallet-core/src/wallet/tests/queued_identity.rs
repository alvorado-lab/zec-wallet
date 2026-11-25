//! WL-4 (`docs/plan/wl4-queued-identity.md`): a queued send is signed only as the row
//! that was read, through the wallet's own doors: the resubmission pass, the deposit sign
//! at execute and the parked "Send now". The wallet here holds no notes, so nothing is
//! signed either way; what the doors answer and what the rows read afterwards is asserted.
//! The signed bytes are `send/tests/queued_identity.rs`, over a funded harness.
//!
//! THE PAUSE is the seed pull: every caller reads its row, releases the aux lock and asks
//! the host for the seed before it takes the locks of the claim. The port below runs one
//! closure inside that pull, so the row is replaced at a fixed point and nothing races.

use std::sync::{Arc, Mutex};

use super::*;
use crate::intent_store::{self, QueuedIntent};
use crate::seed::SeedSupplyError;

const ANCHOR: u64 = 280_900;
const TIP: u64 = 281_000;

struct ReplacingPort {
    seed: Zeroizing<Vec<u8>>,
    during_the_next_pull: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl ReplacingPort {
    fn arm(&self, replace: impl FnOnce() + Send + 'static) {
        *self.during_the_next_pull.lock().expect("armed") = Some(Box::new(replace));
    }
}

impl WalletSeedPort for ReplacingPort {
    fn provide_seed(
        &self,
        _binding: Option<&SpendBinding>,
    ) -> Result<Zeroizing<Vec<u8>>, SeedSupplyError> {
        let armed = self.during_the_next_pull.lock().expect("armed").take();
        if let Some(replace) = armed {
            replace();
        }
        Ok(Zeroizing::new(self.seed.to_vec()))
    }
}

/// A synced wallet with no notes whose seed lives with the port.
async fn port_wallet(dir: &std::path::Path) -> (Wallet, Arc<ReplacingPort>) {
    let vault = test_vault();
    let port = Arc::new(ReplacingPort {
        seed: Zeroizing::new(FR12_SENTINEL_SEED.to_vec()),
        during_the_next_pull: Mutex::new(None),
    });
    let w = create_none_then_reopen_with_port(
        dir,
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    let birthday = account::birthday_from_treestate(sync::testing::chain_tree_state(ANCHOR))
        .expect("the empty-frontier anchor decodes");
    w.import_account(birthday).await.expect("import via port");
    w.sync_once(
        &mut sync::testing::FakeChain::to_tip(TIP),
        &sync::CancelToken::new(),
        NOOP_PROGRESS,
    )
    .await
    .expect("sync records the chain tip");
    w.seed_current_consensus_for_test();
    (w, port)
}

fn listed(w: &Wallet) -> Vec<QueuedIntent> {
    let aux = w.inner.aux_db.lock().expect("aux db");
    intent_store::list_queued(&aux).expect("list")
}

/// The URI of a payment this wallet would queue, and nothing left queued.
async fn another_uri(w: &Wallet) -> String {
    let id = w
        .queue_send(one_payment(Some(60_000)))
        .await
        .expect("queue");
    let row = listed(w).into_iter().find(|r| r.id == id).expect("queued");
    assert!(
        w.cancel_parked_send(id.value(), row.created_at)
            .await
            .expect("cancel")
    );
    row.uri
}

/// What happens inside the pull: `row` is deleted and a payment of `uri` queued at
/// `queued_at`, which takes its id.
fn replaced(w: &Wallet, row: &QueuedIntent, uri: &str, queued_at: i64) -> impl FnOnce() + use<> {
    let inner = Arc::clone(&w.inner);
    let (id, uri) = (row.id, uri.to_owned());
    move || {
        let mut aux = inner.aux_db.lock().expect("aux db");
        assert!(intent_store::delete(&mut aux, id).expect("delete"));
        let binding = Some(SpendBinding::mint());
        let taken =
            intent_store::enqueue(&mut aux, &uri, queued_at, None, binding).expect("enqueue");
        assert_eq!(taken, id, "fixture: the replacement took its id");
    }
}

/// A payment the engine can propose. `one_payment`'s Sapling recipient is no valid point,
/// so its propose stops at the address (`Retry`) whether or not the row is still the one
/// read; this recipient reaches note selection, which finds the wallet short (`Stale`).
fn short_of_funds(amount: i64) -> PaymentRequest {
    use crate::memo::{Memo, Payment};
    PaymentRequest {
        payments: vec![
            Payment::new(
                transparent_test_addr(),
                Some(Zatoshis::new(amount).expect("amount")),
                Memo::Empty,
                None,
                None,
            )
            .expect("payment"),
        ],
    }
}

fn assert_only_the_replacement_is_queued(w: &Wallet, uri: &str) {
    let left = listed(w);
    let uris: Vec<&str> = left.iter().map(|r| r.uri.as_str()).collect();
    assert_eq!(uris, [uri], "the replacement is still queued, as itself");
}

/// A pass lists A and B; inside A's pull B is cancelled and another payment takes its id.
/// At B's turn the pass must see it no longer holds the row it read: one row was short of
/// funds (A), one is retried (the replacement, which the next pass lists as itself).
#[tokio::test]
async fn a_pass_paused_between_its_rows_retries_the_replacement() {
    // The same second as B, then a later one.
    for later in [0, 1] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (w, port) = port_wallet(dir.path()).await;
        for amount in [40_000, 50_000] {
            w.queue_send(short_of_funds(amount)).await.expect("queue");
        }
        let uri = another_uri(&w).await;
        let rows = listed(&w);
        let b = rows[1].clone();
        port.arm(replaced(&w, &b, &uri, b.created_at + later));
        let pass = w
            .resubmit_queued_sends(&sync::CancelToken::new())
            .await
            .expect("the pass runs");
        assert_eq!(
            (pass.stale, pass.retried),
            (1, 1),
            "the cancelled row is not driven under the id another send holds (later: {later})"
        );
        let left = listed(&w);
        assert_eq!(left.len(), 2, "A and the replacement");
        assert_eq!(left[1].uri, uri, "the replacement is queued as itself");
        w.close().await.expect("close");
    }
}

async fn queue_deposit(w: &Wallet, deadline: u64) -> QueuedIntent {
    let id = w
        .queue_deposit_send(
            transparent_test_addr(),
            Zatoshis::new(250_000).expect("amount"),
            deadline,
            Some(SpendBinding::mint()),
        )
        .await
        .expect("queue the deposit");
    listed(w).into_iter().find(|r| r.id == id).expect("queued")
}

#[tokio::test]
async fn a_deposit_sign_whose_row_was_replaced_answers_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, port) = port_wallet(dir.path()).await;
    let uri = another_uri(&w).await;
    let deposit = queue_deposit(&w, 1_900_000_000).await;
    port.arm(replaced(&w, &deposit, &uri, deposit.created_at));
    let out = w
        .try_sign_queued_intent(deposit.id, SignGuard::Deposit)
        .await
        .expect("sign");
    assert!(
        matches!(out, SignQueuedOutcome::NotFound),
        "the deposit this call queued is gone; the row under its id is another send"
    );
    assert_only_the_replacement_is_queued(&w, &uri);
    w.close().await.expect("close");
}

#[tokio::test]
async fn a_deposit_sign_past_its_quote_never_deletes_the_row_that_took_its_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, port) = port_wallet(dir.path()).await;
    let uri = another_uri(&w).await;
    let deposit = queue_deposit(&w, 1_700_000_000).await;
    port.arm(replaced(&w, &deposit, &uri, deposit.created_at));
    let out = w
        .try_sign_queued_intent(deposit.id, SignGuard::Deposit)
        .await
        .expect("sign");
    assert_only_the_replacement_is_queued(&w, &uri);
    assert!(
        matches!(out, SignQueuedOutcome::NotFound),
        "the lapsed deposit is gone already: nothing of this call's was left to expire"
    );
    w.close().await.expect("close");
}

async fn parked_sign_after_a_replacement(later: i64) {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, port) = port_wallet(dir.path()).await;
    let uri = another_uri(&w).await;
    w.queue_send(one_payment(Some(40_000)))
        .await
        .expect("queue");
    let row = listed(&w).remove(0);
    port.arm(replaced(&w, &row, &uri, row.created_at + later));
    let out = w
        .authorize_parked_send(row.id.value(), row.created_at)
        .await
        .expect("authorize");
    assert_eq!(
        out,
        crate::parked::ParkedAuthorization::NotFound,
        "the payment the user authorized is gone; another send holds its id"
    );
    assert_only_the_replacement_is_queued(&w, &uri);
    w.close().await.expect("close");
}

/// The control: the parked sign already held this before WL-4, for a later second.
#[tokio::test]
async fn a_parked_sign_whose_row_was_replaced_answers_not_found() {
    parked_sign_after_a_replacement(1).await;
}

#[tokio::test]
async fn a_parked_sign_whose_row_was_replaced_in_the_same_second_answers_not_found() {
    parked_sign_after_a_replacement(0).await;
}
