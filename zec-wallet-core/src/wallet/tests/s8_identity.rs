//! Stage S8 `identity` (R01) — the test author's WALLET-level rows over the REAL
//! sealed stores: the SQLCipher aux-db home rows and refund watches, the engine-
//! registered refund index, a real keyed reopen. Contract
//! `docs/plan/stage-8-payment-identity-and-durable-retry.md` §3.1, written blind
//! (IT-2a) against `dc57cb2d`. A child of `wallet::tests` so it uses that module's
//! fixtures (`cfg`, `raw_seed`, `test_vault`, `chain_birthday`, the swap request
//! builders, the aux peeks) without adding to it.
//!
//! The service-seam rows live in `swap::service::tests`; these prove the same
//! premise where the money is kept: the home row, the watch and the queued deposit
//! are keyed by the SDK-minted identity, and a reopened wallet polls a REAL
//! provider status through the handle its home row carries.

use std::collections::VecDeque;

use super::*;
use crate::swap::{SwapId, SwapPort, SwapStatus, SwapStatusSink};

/// A provider answering every quote with ONE reused deposit address
/// (`honest_swap_quote`'s `0x22` Main t-addr) at the request's own ZEC amount — the
/// R01 shape — and RECORDING the argument each status poll carried. Statuses are
/// served FIFO; empty ⇒ `Processing`.
///
/// COUPLING DECLARED FOR THE JOIN (the port shape is the adjudicator's ruling): at
/// the base commit `status` takes the `SwapId` alone and `polled` records it; once
/// the adapter is handed the provider address as DATA, `polled` records that
/// argument — the value the request URL is built from — whatever the port's spelling.
struct ReusingProvider {
    polled: Mutex<Vec<String>>,
    statuses: Mutex<VecDeque<SwapStatus>>,
}

impl ReusingProvider {
    fn new(statuses: Vec<SwapStatus>) -> Arc<Self> {
        Arc::new(Self {
            polled: Mutex::new(Vec::new()),
            statuses: Mutex::new(statuses.into()),
        })
    }

    fn polled(&self) -> Vec<String> {
        self.polled.lock().expect("polled poisoned").clone()
    }
}

#[async_trait]
impl SwapPort for ReusingProvider {
    fn name(&self) -> &'static str {
        "reusing"
    }

    async fn quote(
        &self,
        req: crate::swap::QuoteRequest,
    ) -> Result<crate::swap::SwapQuote, crate::error::SwapError> {
        let crate::swap::ExactSide::In(crate::swap::SwapAmount::Zec(amount)) = &req.exact else {
            return Err(crate::error::SwapError::ProviderUnavailable);
        };
        let mut q = honest_swap_quote(&req, amount.zat(), 3_600);
        // the shipped adapter's shape (`map.rs:437`): the provider id IS the deposit
        // address — `honest_swap_quote`'s `q-wire` would make "id ≠ address" vacuous
        q.id = SwapId::new(q.deposit_address.clone());
        Ok(q)
    }

    async fn execute(
        &self,
        _quote: &crate::swap::SwapQuote,
    ) -> Result<(), crate::error::SwapError> {
        Ok(())
    }

    async fn status(
        &self,
        _id: &SwapId,
        provider_ref: &str,
    ) -> Result<SwapStatus, crate::error::SwapError> {
        // the ruled port shape (both values): `polled` records the PROVIDER-ADDRESS
        // argument — the value the request URL is built from
        self.polled
            .lock()
            .expect("polled poisoned")
            .push(provider_ref.to_owned());
        Ok(self
            .statuses
            .lock()
            .expect("statuses poisoned")
            .pop_front()
            .unwrap_or(SwapStatus::Processing))
    }

    async fn list_tokens(&self) -> Result<crate::swap::TokenList, crate::error::SwapError> {
        Err(crate::error::SwapError::ProviderUnavailable)
    }
}

/// A status sink over a channel — the reference UI's stream shape; the sender
/// drops when the poll ends, closing the receiver.
struct StatusChannel(tokio::sync::mpsc::UnboundedSender<SwapStatus>);

impl SwapStatusSink for StatusChannel {
    fn emit(&mut self, status: SwapStatus) -> bool {
        self.0.send(status).is_ok()
    }
}

/// A Main wallet with a chain account — the refund mint engine-registers its
/// address, so the account must exist (the `cross_restart_execute_reconstructs…`
/// fixture shape).
async fn main_wallet_with_account(dir: &std::path::Path, vault: &Arc<dyn KeychainPort>) -> Wallet {
    let w = Wallet::create_with_vault(
        cfg(dir, Network::Main, SeedPersistence::SealedKeychain),
        raw_seed(),
        Arc::clone(vault),
    )
    .await
    .expect("create");
    w.import_account(chain_birthday()).await.expect("import");
    w
}

fn now_unix_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Row 6's poll leg over a REAL keyed reopen: a swap executed under an SDK-minted
/// identity is homed under that identity (the re-attach handle), and a
/// `watch_status` opened AFTER the reopen with nothing but that handle polls a REAL
/// provider status — the provider's own terminal, never a synthesized
/// `Failed(NotFound)` — carrying the provider ADDRESS resolved from the durable home
/// row before the poll task was spawned (an in-memory id→address map died with the
/// first process). Reddening mutations: the base keying (the handle IS the
/// address); resolving the address from an in-memory cache; polling with the SDK
/// handle in the address position.
#[tokio::test]
#[cfg(feature = "swap")]
async fn a_swap_executed_under_an_sdk_minted_identity_still_polls_to_a_real_provider_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (handle, address) = {
        let w = main_wallet_with_account(dir.path(), &vault).await;
        w.enable_swap(ReusingProvider::new(Vec::new()))
            .expect("enable swap");
        let svc = w.swap().expect("swap enabled");
        let q = svc.quote(out_of_zec_request(100_000)).await.expect("quote");
        let id = svc
            .execute(&q)
            .await
            .expect("execute — the deposit is enqueued durably; signing is best-effort");
        assert_ne!(
            id.as_str(),
            q.deposit_address,
            "the handle execute returns is not the provider address"
        );
        let rows = w.list_in_flight_swaps().await.expect("list");
        assert_eq!(rows.len(), 1, "one home row");
        assert_eq!(
            rows[0].swap_id,
            id.as_str(),
            "the home row is keyed by the SDK identity — the re-attach handle"
        );
        w.close().await.expect("close");
        (rows[0].swap_id.clone(), q.deposit_address)
    };

    // Reopen: the host has nothing but the handle the home row lists.
    let w2 = Wallet::open_with_vault(
        cfg(dir.path(), Network::Main, SeedPersistence::SealedKeychain),
        Arc::clone(&vault),
    )
    .await
    .expect("reopen");
    let provider = ReusingProvider::new(vec![SwapStatus::Success {
        out_txid: None,
        realized_slippage_bps: None,
    }]);
    w2.enable_swap(Arc::clone(&provider) as Arc<dyn SwapPort>)
        .expect("enable swap after the reopen");
    let rows = w2.list_in_flight_swaps().await.expect("list after reopen");
    assert_eq!(
        rows.iter().map(|r| r.swap_id.as_str()).collect::<Vec<_>>(),
        vec![handle.as_str()],
        "the home row survived the reopen under the same handle"
    );
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    w2.swap()
        .expect("swap")
        .watch_status(SwapId::new(handle.clone()), StatusChannel(tx))
        .await
        .expect("the handle resolved from the home row and the poll spawned");
    let first = rx.recv().await.expect("the stream delivers a status");
    assert_eq!(
        first,
        SwapStatus::Success {
            out_txid: None,
            realized_slippage_bps: None,
        },
        "a REAL provider status — the provider's own terminal, not a synthesized NotFound"
    );
    assert!(rx.recv().await.is_none(), "the stream ends at the terminal");
    assert_eq!(
        provider.polled(),
        vec![address],
        "the poll carried the provider address resolved from the durable home row, never the SDK handle"
    );
    w2.close().await.expect("close");
}

/// Row 4 in the sealed store: two quotes under ONE provider address are two home
/// rows under two SDK identities (neither the address) and two ARMED refund
/// watches at two single-use refund addresses; the one deposit the wallet enqueued
/// is the FIRST approval's. The second execute registers and homes its own order —
/// its deposit leg is then refused by the enqueue's one-deposit-in-flight guard
/// (`intent_store::enqueue`, any deadline-tagged row, `Queued` included — beside
/// `obligation`'s spans, not in them), so exactly one deposit is queued, and the
/// second is NOT a take-miss for the first's identity. Reddening mutation: the
/// base keying (the second execute finds no claim; one home row, one watch).
#[tokio::test]
#[cfg(feature = "swap")]
async fn two_swaps_under_one_provider_address_are_two_home_rows_and_two_refund_watches_in_the_sealed_store()
 {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = main_wallet_with_account(dir.path(), &vault).await;
    w.enable_swap(ReusingProvider::new(Vec::new()))
        .expect("enable swap");
    let svc = w.swap().expect("swap enabled");
    let q1 = svc
        .quote(out_of_zec_request(100_000))
        .await
        .expect("first quote");
    let q2 = svc
        .quote(out_of_zec_request(900_000))
        .await
        .expect("second quote");
    assert_eq!(
        q1.deposit_address, q2.deposit_address,
        "one provider address"
    );
    assert_ne!(q1.id, q2.id, "two SDK identities");
    svc.execute(&q1).await.expect("the first executes");
    let second = svc.execute(&q2).await;
    assert!(
        !matches!(second, Err(crate::error::SwapError::QuoteExpired)),
        "the second quote has its own claim — it is not a replay of the first, got {:?}",
        second.as_ref().err().map(|e| e.code())
    );

    let rows = w.list_in_flight_swaps().await.expect("list");
    let ids: Vec<&str> = rows.iter().map(|r| r.swap_id.as_str()).collect();
    assert_eq!(rows.len(), 2, "two home rows: {ids:?}");
    assert!(
        ids.contains(&q1.id.as_str()) && ids.contains(&q2.id.as_str()),
        "each home row under its own SDK identity: {ids:?}"
    );
    assert!(
        !ids.contains(&q1.deposit_address.as_str()),
        "no home row is keyed by the provider address"
    );

    let watches = {
        let guard = w.inner.aux_db.lock().expect("aux db");
        crate::swap_destination_store::active(&guard, now_unix_i64()).expect("active watches")
    };
    assert_eq!(watches.len(), 2, "two armed refund watches");
    assert_ne!(
        watches[0].address, watches[1].address,
        "at two single-use refund addresses"
    );
    assert_ne!(
        watches[0].index, watches[1].index,
        "at two single-use indexes"
    );
    for watch in &watches {
        assert!(
            ids.contains(&watch.swap_id.as_str()),
            "each watch is keyed by an SDK identity, not the provider address"
        );
    }

    let queued = {
        let guard = w.inner.aux_db.lock().expect("aux db");
        crate::intent_store::list_queued(&guard).expect("queued intents")
    };
    assert_eq!(
        queued.len(),
        1,
        "exactly one deposit enqueued — the first's"
    );
    assert_eq!(
        queued[0].deposit_deadline,
        Some(
            i64::try_from(q1.expires_at + crate::constants::DEPOSIT_EXECUTE_MARGIN_SECS)
                .expect("deadline fits")
        ),
        "the enqueued deposit carries the FIRST approval's deadline tag"
    );
    w.close().await.expect("close");
}
