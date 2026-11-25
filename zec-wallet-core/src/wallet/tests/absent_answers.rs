//! An absent answer is not evidence (`docs/plan/wl1-absent-answers.md` (d)): the TEST AUTHOR's
//! rows that go through a REAL `Wallet`, its production store adapter
//! (`WalletDbEnhancementStore`) and its engine database, written blind to the implementation
//! against the contract: rows 4, 5 and 12 (row 8 joins them with the names it needs). The rows
//! over the pass itself are `enhance/tests/absent_answers.rs`. A child of `wallet::tests` for its
//! fixtures (`wallet_with_account_at`, `test_vault`, `cfg`), appended at the END of `wallet.rs` so
//! no cited line above moves.
//!
//! **How the engine's state is set up.** The wallet is `wallet_with_account_at` (a real create and
//! account import, on testnet), the tip is the engine's own `update_chain_tip` on the wallet's
//! engine connection, and the rows are planted in the engine's `transactions` and
//! `tx_retrieval_queue` tables through `Inner::aux_db`, the second connection the wallet's own
//! readers ride (the `delivery_obligation` shape). Nothing is scanned: no row here reads a block.
//!
//! **What stands for the TEX second leg.** A wallet-created transaction the wallet holds the bytes
//! of, with a stored expiry, un-mined, and a `Status` request (`query_type` 0) the engine raises
//! because scanning cannot see it. Its bytes are a real v5 transaction, so the reply parses and
//! its txid matches; it has no bundles, which no reader here looks at.

use std::sync::Arc;

use rusqlite::params;
use zcash_client_backend::data_api::{TransactionDataRequest, WalletRead, WalletWrite};
use zcash_primitives::transaction::{Authorized, TransactionData, TxVersion};
use zcash_protocol::TxId as EngineTxId;
use zcash_protocol::consensus::{BlockHeight as EngineHeight, BranchId};

use super::*;
use crate::constants::MAX_ENHANCEMENTS_PER_PASS;
use crate::enhance::{EnhancementOutcome, FetchedTransaction, TransactionFetcher};
use crate::net::grpc::GrpcError;
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};

/// Testnet, NU6-era: account 0's birthday is the block after. The branch `Nu6` runs on testnet
/// from 2,976,000 to 3,536,500, so a leg signed here ends at its own stored expiry.
const ANCHOR: u64 = 2_999_700;
/// The height the leg was signed at, and the last height it can be mined at.
const SIGNED_AT: u32 = 3_000_000;
const EXPIRY: u32 = 3_000_040;

/// The engine's codes for a queued request (`TxQueryType`).
const STATUS: i64 = 0;
const ENHANCEMENT: i64 = 1;

/// gRPC `NotFound`, the answer every measured server gives for a txid it does not know.
const GRPC_NOT_FOUND: i32 = 5;

// ── The wallet and its rows ──────────────────────────────────────────────────

async fn wallet_at(dir: &std::path::Path, vault: &Arc<dyn KeychainPort>, tip: u32) -> Wallet {
    let w = wallet_with_account_at(dir, vault, ANCHOR).await;
    move_tip(&w, tip);
    w
}

/// The endpoint announces `tip`, and the engine records it.
fn move_tip(w: &Wallet, tip: u32) {
    let mut guard = w.inner.db.lock().expect("wallet db");
    guard
        .db
        .update_chain_tip(EngineHeight::from_u32(tip))
        .expect("record the tip");
    assert_eq!(
        guard.db.chain_height().expect("read the tip"),
        Some(EngineHeight::from_u32(tip)),
        "the fixture: the engine's tip is the one announced"
    );
}

/// A leg the wallet signed: its bytes and its txid.
struct Leg {
    raw: Vec<u8>,
    txid: EngineTxId,
}

/// Plant a TEX-like leg and its `Status` request. `seed` tells legs apart.
fn plant_leg(w: &Wallet, seed: u32) -> Leg {
    let data: TransactionData<Authorized> = TransactionData::from_parts(
        TxVersion::V5,
        BranchId::Nu6,
        seed,
        EngineHeight::from_u32(EXPIRY),
        None,
        None,
        None,
        None,
    );
    let tx = data.freeze().expect("freeze a v5 transaction");
    let mut raw = Vec::new();
    tx.write(&mut raw).expect("serialize");
    let leg = Leg {
        raw,
        txid: tx.txid(),
    };
    let aux = w.inner.aux_db.lock().expect("aux db");
    aux.execute(
        "INSERT INTO transactions (txid, raw, expiry_height, min_observed_height, created) \
         VALUES (?1, ?2, ?3, ?4, '1970-01-01 00:00:00.0000000+00:00')",
        params![leg.txid.as_ref(), leg.raw, EXPIRY, SIGNED_AT],
    )
    .expect("plant the leg");
    plant_request(&aux, leg.txid, STATUS);
    leg
}

fn plant_request(aux: &rusqlite::Connection, txid: EngineTxId, query_type: i64) {
    let planted = aux
        .execute(
            "INSERT INTO tx_retrieval_queue (txid, query_type) VALUES (?1, ?2)",
            params![txid.as_ref(), query_type],
        )
        .expect("plant the request");
    assert_eq!(planted, 1);
}

/// Plant `n` (at most 63) `Enhancement` requests for foreign parents (txids with no `transactions`
/// row), and return their txids in ascending order, the order the engine lists them in. They stand
/// four bytes apart, so a cursor born at a random txid starts anywhere among sixty of them.
fn plant_parents(w: &Wallet, n: u8) -> Vec<EngineTxId> {
    let txids: Vec<EngineTxId> = (1..=n)
        .map(|i| EngineTxId::from_bytes([i * 4; 32]))
        .collect();
    let aux = w.inner.aux_db.lock().expect("aux db");
    for txid in &txids {
        plant_request(&aux, *txid, ENHANCEMENT);
    }
    txids
}

/// What the engine holds about a leg: is its `Status` row in the queue, is the question one it
/// still asks, the height it is mined at, and the height it was last confirmed un-mined at.
#[derive(Debug, PartialEq, Eq)]
struct Held {
    row: bool,
    asked: bool,
    mined: Option<u32>,
    confirmed_unmined_at: Option<u32>,
}

fn held(w: &Wallet, leg: &Leg) -> Held {
    let (row, mined, confirmed_unmined_at) = {
        let aux = w.inner.aux_db.lock().expect("aux db");
        let rows: i64 = aux
            .query_row(
                "SELECT COUNT(*) FROM tx_retrieval_queue WHERE txid = ?1 AND query_type = ?2",
                params![leg.txid.as_ref(), STATUS],
                |r| r.get(0),
            )
            .expect("read the queue");
        let (mined, confirmed): (Option<u32>, Option<u32>) = aux
            .query_row(
                "SELECT mined_height, confirmed_unmined_at_height FROM transactions \
                 WHERE txid = ?1",
                params![leg.txid.as_ref()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("read the leg");
        (rows == 1, mined, confirmed)
    };
    let guard = w.inner.db.lock().expect("wallet db");
    let asked = guard
        .db
        .transaction_data_requests()
        .expect("read the requests")
        .contains(&TransactionDataRequest::GetStatus(leg.txid));
    Held {
        row,
        asked,
        mined,
        confirmed_unmined_at,
    }
}

// ── The endpoint ─────────────────────────────────────────────────────────────

/// A counting [`TransactionFetcher`]: records every ask in order, answers the txids it was given
/// an answer for, and gRPC `NotFound` for the rest.
#[derive(Default)]
struct CountingFetcher {
    calls: Vec<EngineTxId>,
    /// The bytes and the height CLAIM served for a txid.
    answers: Vec<(EngineTxId, Vec<u8>, u64)>,
    /// The 0-based call that never answers: it wakes `hanging`, then pends for good.
    hang_on_call: Option<usize>,
    hanging: Arc<tokio::sync::Notify>,
}

impl CountingFetcher {
    /// Answer the asks about `leg` with its own bytes, claimed at `wire`, from here on.
    fn serve(&mut self, leg: &Leg, wire: u64) {
        self.serve_bytes(leg, leg.raw.clone(), wire);
    }

    fn serve_bytes(&mut self, leg: &Leg, data: Vec<u8>, wire: u64) {
        self.answers.retain(|(txid, _, _)| *txid != leg.txid);
        self.answers.push((leg.txid, data, wire));
    }
}

#[async_trait::async_trait]
impl TransactionFetcher for CountingFetcher {
    async fn fetch_transaction(
        &mut self,
        txid: EngineTxId,
    ) -> Result<Option<FetchedTransaction>, GrpcError> {
        let call = self.calls.len();
        self.calls.push(txid);
        if self.hang_on_call == Some(call) {
            self.hanging.notify_one();
            return std::future::pending().await;
        }
        match self.answers.iter().find(|(t, _, _)| *t == txid) {
            Some((_, data, height)) => Ok(Some(FetchedTransaction {
                data: data.clone(),
                height: *height,
            })),
            None => Err(GrpcError::Status {
                code: GRPC_NOT_FOUND,
            }),
        }
    }
}

/// One production pass over the wallet.
async fn pass(w: &Wallet, fetcher: &mut CountingFetcher) -> EnhancementOutcome {
    w.enhance_transactions(fetcher, &|| {})
        .await
        .expect("the pass runs")
}

/// One production pass, with every wallet-targeted field it logged. The store's own work runs on
/// the blocking pool, off this thread's subscriber: what is captured is the loop's.
async fn pass_logging(
    w: &Wallet,
    fetcher: &mut CountingFetcher,
) -> (EnhancementOutcome, Vec<(String, String)>) {
    use tracing_subscriber::layer::SubscriberExt;
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(CaptureLayer::new(sink.clone())),
    );
    let out = pass(w, fetcher).await;
    drop(guard);
    (out, sink.fields())
}

/// `calls` walks `cycle` round and round, from wherever it started, missing nothing and asking
/// nothing twice in a round.
#[track_caller]
fn assert_walks(calls: &[EngineTxId], cycle: &[EngineTxId], label: &str) {
    let start = cycle
        .iter()
        .position(|txid| Some(txid) == calls.first())
        .expect("the first txid asked is one of the planted ones");
    let walked: Vec<EngineTxId> = cycle
        .iter()
        .cycle()
        .skip(start)
        .take(calls.len())
        .copied()
        .collect();
    assert_eq!(calls, walked, "{label}");
}

// ── C3: an unmined answer past the expiry is not written ─────────────────────

/// **Row 4 (C3).** A validated reply that the transaction is not mined, as the mempool sentinel
/// and as the off-chain one, one block before the leg's expiry, at it and past it.
///
/// Before the expiry it is an answer: `NotInMainChain` is written and the question stays. From the
/// expiry on it writes nothing, logs nothing and counts one refusal, and the `Status` row is still
/// in `tx_retrieval_queue` and still asked.
///
/// The mechanism that would end the question is the engine's: at a tip at or past the stored
/// expiry its arm for this status deletes the `Status` row, for good. Every refused reply here was
/// asked for (the fetcher counts it) and is the same bytes the first step answers with, so the row
/// does not pass because the pass stopped before the check. Two legs are planted after the first
/// step, so a write past the expiry shows on a row that was never written at all.
#[tokio::test]
async fn an_unmined_answer_past_the_expiry_keeps_the_question() {
    const SENTINELS: [(&str, u64); 2] = [("mempool", 0), ("off the main chain", u64::MAX)];

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_at(dir.path(), &vault, EXPIRY - 1).await;
    let mut fetcher = CountingFetcher::default();
    let plant = |fetcher: &mut CountingFetcher, seed: u32| -> Vec<(&'static str, Leg)> {
        SENTINELS
            .iter()
            .zip(seed..)
            .map(|((sentinel, wire), seed)| {
                let leg = plant_leg(&w, seed);
                fetcher.serve(&leg, *wire);
                (*sentinel, leg)
            })
            .collect()
    };

    // One block before the expiry: an answer.
    let early = plant(&mut fetcher, 1);
    let (out, _) = pass_logging(&w, &mut fetcher).await;
    assert_eq!(
        (out.requested, out.status_set, out.rejected),
        (2, 2, 0),
        "below the expiry an unmined answer is an answer"
    );
    for (sentinel, leg) in &early {
        assert_eq!(
            held(&w, leg),
            Held {
                row: true,
                asked: true,
                mined: None,
                confirmed_unmined_at: Some(EXPIRY - 1),
            },
            "{sentinel}: one block before the expiry it is written, and the question stays"
        );
    }

    let late = plant(&mut fetcher, 11);
    for (tip, when) in [(EXPIRY, "at the expiry"), (EXPIRY + 1, "past the expiry")] {
        move_tip(&w, tip);
        let asked_before = fetcher.calls.len();
        let (out, fields) = pass_logging(&w, &mut fetcher).await;
        assert_eq!(
            fetcher.calls.len() - asked_before,
            4,
            "{when}: all four questions are asked"
        );
        assert_eq!(
            (out.requested, out.status_set, out.rejected),
            (4, 0, 4),
            "{when}: none is answered, and each refusal is counted"
        );
        assert!(
            fields.is_empty(),
            "{when}: a refused unmined answer logs nothing. Captured: {fields:?}"
        );
        for (legs, confirmed_unmined_at) in [(&early, Some(EXPIRY - 1)), (&late, None)] {
            for (sentinel, leg) in legs {
                assert_eq!(
                    held(&w, leg),
                    Held {
                        row: true,
                        asked: true,
                        mined: None,
                        confirmed_unmined_at,
                    },
                    "{when}, {sentinel}: nothing is written and the question stays"
                );
            }
        }
    }

    // CONTROL for the silence: a HEIGHT the kernel refuses, through the same pass, warns.
    let (_, leg) = &late[0];
    fetcher.serve(leg, u64::from(EXPIRY) + 1);
    let (out, fields) = pass_logging(&w, &mut fetcher).await;
    assert_eq!((out.status_set, out.rejected), (0, 4));
    assert!(
        fields
            .iter()
            .any(|(_, v)| v.contains("contradicts what this wallet already knows")),
        "a refused HEIGHT keeps its warning. Captured: {fields:?}"
    );
    w.close().await.expect("close");
}

/// **Row 5 (C3).** The money claim of row 4: a leg that WAS mined, inside its expiry, still
/// confirms after a forged unmined answer past it. The forged reply is answered by nothing; the
/// next pass asks again, the honest height is written, and the `Status` row goes dormant (kept in
/// the queue, asked no more).
///
/// At the base the forged reply deletes the row, and the second pass asks nothing: the leg reads
/// un-mined for good, which is what `Stranded` is decided on.
#[tokio::test]
async fn a_leg_mined_after_a_forged_unmined_answer_still_confirms() {
    const MINED_AT: u32 = EXPIRY - 3;

    for (sentinel, wire) in [("mempool", 0u64), ("off the main chain", u64::MAX)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let w = wallet_at(dir.path(), &vault, EXPIRY + 5).await;
        let leg = plant_leg(&w, 21);
        let mut fetcher = CountingFetcher::default();

        fetcher.serve(&leg, wire);
        pass(&w, &mut fetcher).await;
        assert_eq!(fetcher.calls, [leg.txid], "{sentinel}: it was asked");
        assert_eq!(
            held(&w, &leg),
            Held {
                row: true,
                asked: true,
                mined: None,
                confirmed_unmined_at: None,
            },
            "{sentinel}: the forged answer is no answer"
        );

        fetcher.serve(&leg, u64::from(MINED_AT));
        let out = pass(&w, &mut fetcher).await;
        assert_eq!(fetcher.calls, [leg.txid; 2], "{sentinel}: asked again");
        assert_eq!(out.status_set, 1, "{sentinel}");
        assert_eq!(
            held(&w, &leg),
            Held {
                row: true,
                asked: false,
                mined: Some(MINED_AT),
                confirmed_unmined_at: None,
            },
            "{sentinel}: the honest height is written, and the row goes dormant"
        );

        let out = pass(&w, &mut fetcher).await;
        assert_eq!(
            (out, fetcher.calls.len()),
            (EnhancementOutcome::default(), 2),
            "{sentinel}: dormant: nothing is asked any more"
        );
        w.close().await.expect("close");
    }
}

/// An unlisted row for C5's money claim, through the real wallet: a forged branch id in answer to
/// a leg's `Status` request does not hide the question. The next pass asks again and the honest
/// height is written.
///
/// At the base the txid joins the process-lifetime set, and the leg is asked about no more until
/// the app is relaunched.
#[tokio::test]
async fn a_forged_branch_does_not_stop_a_leg_from_confirming() {
    use zcash_protocol::constants::{V6_TX_VERSION, V6_VERSION_GROUP_ID};
    const UNKNOWN_BRANCH: u32 = 0x0bad_0bad;
    const MINED_AT: u32 = EXPIRY - 3;
    assert!(
        BranchId::try_from(UNKNOWN_BRANCH).is_err(),
        "the fixture must name a branch this build genuinely lacks"
    );
    let mut forged = Vec::new();
    for word in [
        0x8000_0000 | V6_TX_VERSION,
        V6_VERSION_GROUP_ID,
        UNKNOWN_BRANCH,
        0,
        0,
    ] {
        forged.extend_from_slice(&word.to_le_bytes());
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_at(dir.path(), &vault, EXPIRY + 5).await;
    let leg = plant_leg(&w, 31);
    let mut fetcher = CountingFetcher::default();

    fetcher.serve_bytes(&leg, forged, u64::from(MINED_AT));
    let out = pass(&w, &mut fetcher).await;
    assert_eq!(
        (out.requested, out.rejected, out.unknown_branch),
        (1, 1, 0),
        "a refused reply like any other"
    );
    assert_eq!(
        held(&w, &leg),
        Held {
            row: true,
            asked: true,
            mined: None,
            confirmed_unmined_at: None,
        }
    );

    fetcher.serve(&leg, u64::from(MINED_AT));
    let out = pass(&w, &mut fetcher).await;
    assert_eq!(fetcher.calls, [leg.txid; 2], "the question is asked again");
    assert_eq!(out.status_set, 1);
    assert_eq!(
        held(&w, &leg),
        Held {
            row: true,
            asked: false,
            mined: Some(MINED_AT),
            confirmed_unmined_at: None,
        },
        "and the honest height is written"
    );
    w.close().await.expect("close");
}

// ── C4: the cursor, over the wallet handle ───────────────────────────────────

/// **Row 12 (C4).** The cursor belongs to the wallet handle: over a queue of sixty requests nobody
/// serves, each pass starts where the one before it stopped, so the passes walk the sixty round
/// and round. Closed and opened again, the sixty are all still there and the place in them is
/// not: a handle starts at a point of its own.
///
/// A fresh cursor made for each pass starts each at a random place, and no cursor at all starts
/// each at the first request: either breaks the walk at the second pass. A cursor that outlived
/// the handle would go on with the walk after every reopen. A fresh one goes on with it by chance
/// about once in 64 reopens, and four times in a row less than once in three million runs.
#[tokio::test]
async fn the_wallet_handle_keeps_the_cursor_across_passes_and_drops_it_on_close() {
    const REOPENS: usize = 4;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_at(dir.path(), &vault, SIGNED_AT).await;
    let sixty = plant_parents(&w, 60);
    let mut fetcher = CountingFetcher::default();
    for n in 1..=4 {
        let out = pass(&w, &mut fetcher).await;
        assert_eq!(
            (out.requested, out.rejected),
            (MAX_ENHANCEMENTS_PER_PASS, MAX_ENHANCEMENTS_PER_PASS),
            "pass {n} asks the cap, and nobody serves any of it"
        );
        assert_walks(
            &fetcher.calls,
            &sixty,
            "each pass starts after the last txid the one before it asked",
        );
    }

    let mut w = w;
    let mut went_on = 0;
    for n in 1..=REOPENS {
        let last = *fetcher.calls.last().expect("something was asked");
        let at = sixty
            .iter()
            .position(|txid| *txid == last)
            .expect("the last txid asked is one of the sixty");
        w.close().await.expect("close");
        w = Wallet::open_with_vault(
            cfg(dir.path(), Network::Test, SeedPersistence::SealedKeychain),
            Arc::clone(&vault),
        )
        .await
        .expect("reopen");
        let asked_before = fetcher.calls.len();
        let out = pass(&w, &mut fetcher).await;
        assert_eq!(
            out.requested, MAX_ENHANCEMENTS_PER_PASS,
            "control, reopen {n}: the queue itself outlives the handle"
        );
        assert_walks(
            &fetcher.calls[asked_before..],
            &sixty,
            "a reopened handle's pass is a walk of its own",
        );
        if fetcher.calls[asked_before] == sixty[(at + 1) % sixty.len()] {
            went_on += 1;
        }
    }
    assert!(
        went_on < REOPENS,
        "every reopened handle went on with the closed one's walk: the cursor outlived its handle"
    );
    w.close().await.expect("close");
}

/// An unlisted row, row 9 through the real wallet: a pass dropped while a fetch hangs leaves the
/// handle's cursor at the hung txid, so the next pass starts after it and comes to it last.
#[tokio::test]
async fn a_pass_dropped_over_a_real_wallet_resumes_after_the_txid_it_was_asking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_at(dir.path(), &vault, SIGNED_AT).await;
    let five = plant_parents(&w, 5);
    let mut fetcher = CountingFetcher {
        hang_on_call: Some(2),
        ..CountingFetcher::default()
    };
    let hanging = Arc::clone(&fetcher.hanging);
    tokio::select! {
        biased;
        _ = w.enhance_transactions(&mut fetcher, &|| {}) => {
            panic!("the fixture: a pass over a hanging fetch does not end")
        }
        () = hanging.notified() => {}
    }
    assert_eq!(fetcher.calls.len(), 3, "dropped while it asked the third");
    pass(&w, &mut fetcher).await;
    assert_eq!(fetcher.calls.len(), 8, "the next pass asks all five");
    assert_walks(
        &fetcher.calls,
        &five,
        "the next pass starts after the hung txid",
    );
    w.close().await.expect("close");
}

// Row 8, which names the cursor itself. Gated on its own line as well: the
// source scans under tests/ skip a file only by its own declaration's gate.
#[cfg(test)]
mod named;
