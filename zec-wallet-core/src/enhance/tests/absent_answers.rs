//! An absent answer is not evidence (`docs/plan/wl1-absent-answers.md` (d)): the TEST AUTHOR's
//! rows over the enhancement pass itself, written blind to the implementation against the
//! contract. Rows 1, 6, 7, 9, 10 and 11 run over the parent module's doubles, rows 2 and 3 over a
//! real engine database ([`EngineStore`]), and row 4's rows over an upgrade boundary are here
//! because they need no wallet. Rows 4, 5, 8 and 12, which go through a real `Wallet` and the
//! production store adapter, are `wallet/tests/absent_answers.rs`.
//!
//! **The join points.** Two signatures change in this contract: the selector takes the txid the
//! pass starts after, and the pass takes the cursor. Every call of either goes through the `join`
//! module below, and nothing else in this file names the cursor.

use std::collections::BTreeSet;
use std::sync::Mutex;

use rusqlite::params;
use zcash_client_backend::data_api::{WalletRead, WalletWrite};
use zcash_protocol::consensus::NetworkUpgrade;
use zcash_protocol::local_consensus::LocalNetwork;

use super::*;
use crate::memo::Memo;
use crate::state::TxStatus;
use crate::test_support::HistoryHarness;
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};

/// THE JOIN POINTS, over the built names. A cursor is born at a random txid, so a row that needs
/// to know where its first pass starts puts the cursor there itself.
mod join {
    use super::*;

    pub(super) struct Cursor(EnhancementCursor);

    impl Cursor {
        /// A cursor standing at the last txid there is, so the next pass wraps and starts at the
        /// smallest one pending.
        pub(super) fn at_the_end() -> Self {
            let cursor = EnhancementCursor::new();
            cursor.record(t(0xff));
            Self(cursor)
        }

        /// Where it stands.
        pub(super) fn get(&self) -> Option<TxId> {
            self.0.get()
        }
    }

    /// One pass at the production cap.
    pub(super) async fn pass<S, C, P>(
        store: &S,
        client: &mut C,
        params: &P,
        unparseable: &UnparseableTxids,
        cursor: &Cursor,
    ) -> Result<EnhancementOutcome, WalletError>
    where
        S: EnhancementStore + ?Sized,
        C: TransactionFetcher + ?Sized,
        P: Parameters,
    {
        run_enhancement_pass(
            store,
            client,
            params,
            MAX_ENHANCEMENTS_PER_PASS,
            &|| {},
            unparseable,
            &cursor.0,
        )
        .await
    }

    /// The selector, starting strictly after `after`.
    pub(super) fn select(
        requests: Vec<TransactionDataRequest>,
        max: usize,
        skip: &UnparseableTxids,
        after: Option<TxId>,
    ) -> Vec<(TxId, RequestKind)> {
        select_enhancement_targets(requests, max, skip, after)
    }
}

// ── Fixtures ─────────────────────────────────────────────────────────────────

/// A mainnet tip, and a mined height under it, at which a `tx_variant` transaction (branch
/// `Nu6_3`, stored expiry 0) is alive: below the NU7 activation, where its effective expiry is.
const TIP: u32 = 3_473_803;
const MINED_AT: u32 = 3_400_000;

const KINDS: [RequestKind; 2] = [RequestKind::Enhancement, RequestKind::Status];

/// The engine's request of `kind` about `txid`.
fn request(kind: RequestKind, txid: TxId) -> TransactionDataRequest {
    match kind {
        RequestKind::Enhancement => TransactionDataRequest::Enhancement(txid),
        RequestKind::Status => TransactionDataRequest::GetStatus(txid),
    }
}

/// An honest reply: the transaction's own bytes, mined at [`MINED_AT`].
fn served(data: &[u8]) -> Reply {
    Reply::Tx {
        data: data.to_vec(),
        height: u64::from(MINED_AT),
    }
}

/// A txid made of one byte, so its order is the byte's order in either byte order.
fn t(b: u8) -> TxId {
    TxId::from_bytes([b; 32])
}

fn ascending(mut txids: Vec<TxId>) -> Vec<TxId> {
    txids.sort();
    txids
}

/// Everything the pass wrote, of either kind, by txid.
fn written(store: &RecordingStore) -> BTreeSet<TxId> {
    let statuses = store.statuses.lock().expect("statuses");
    let stored = store.stored.lock().expect("stored");
    statuses
        .iter()
        .map(|(t, _)| *t)
        .chain(stored.iter().map(|(t, _)| *t))
        .collect()
}

/// Install the §5.4 capture layer for this test's thread; the guard must outlive the pass.
fn capture() -> (CapturedEvents, tracing::subscriber::DefaultGuard) {
    use tracing_subscriber::layer::SubscriberExt;
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(CaptureLayer::new(sink.clone())),
    );
    (sink, guard)
}

/// The same, for a SHORT stretch: the fields logged while `during` ran.
async fn logged_during<F: Future>(during: F) -> (F::Output, Vec<(String, String)>) {
    let (sink, guard) = capture();
    let out = during.await;
    drop(guard);
    let fields = sink.fields();
    (out, fields)
}

/// Run a pass until a fetch hangs and drop it there, which is what the cancel arm of a pass and
/// the deadline of a bounded run do to it.
async fn drop_the_pass_at_the_hang(
    store: &RecordingStore,
    fetcher: &mut ScriptedFetcher,
    cursor: &join::Cursor,
) {
    let hanging = std::sync::Arc::clone(&fetcher.hanging);
    let unparseable = UnparseableTxids::default();
    let mut pass = Box::pin(join::pass(
        store,
        fetcher,
        &MainNetwork,
        &unparseable,
        cursor,
    ));
    tokio::select! {
        biased;
        _ = &mut pass => panic!("the fixture: a pass over a hanging fetch does not end"),
        () = hanging.notified() => {}
    }
    drop(pass);
}

/// An [`EnhancementStore`] over a REAL engine database: the harness's, so the queue, the notes and
/// the memos are the engine's own. What it does not drive is the wallet's adapter
/// (`WalletDbEnhancementStore`): the wallet-level rows do.
struct EngineStore {
    harness: Mutex<HistoryHarness>,
    network: LocalNetwork,
    /// Every write the pass made, in order: the verb, and the txid it was about.
    writes: Mutex<Vec<EngineWrite>>,
}

type EngineWrite = (&'static str, TxId);

const NO_WRITES: [EngineWrite; 0] = [];

impl EngineStore {
    fn over(mut harness: HistoryHarness) -> Self {
        let network = *harness.state_mut().network();
        Self {
            harness: Mutex::new(harness),
            network,
            writes: Mutex::new(Vec::new()),
        }
    }

    fn pending(&self) -> Vec<TransactionDataRequest> {
        self.harness.lock().expect("harness").pending_requests()
    }

    fn writes(&self) -> Vec<EngineWrite> {
        self.writes.lock().expect("writes").clone()
    }

    /// A second connection to the same file, the one the history readers ride.
    fn conn(&self) -> rusqlite::Connection {
        self.harness.lock().expect("harness").read_conn()
    }

    /// The history row of `txid`, as the Activity list reads it.
    fn history_status(&self, txid: TxId) -> TxStatus {
        crate::history::transaction_by_txid(&self.conn(), &self.network, &txid.into())
            .expect("lookup")
            .expect("the transaction's row")
            .status
    }

    /// The text memos the wallet can read in `txid`.
    fn text_memos(&self, txid: TxId) -> Vec<String> {
        crate::history::transaction_memos(&self.conn(), &txid.into())
            .expect("read the memos")
            .memos
            .into_iter()
            .filter_map(|memo| match memo {
                Memo::Text(text) => Some(text),
                _ => None,
            })
            .collect()
    }
}

#[async_trait]
impl EnhancementStore for EngineStore {
    async fn pending_requests(&self) -> Result<Vec<TransactionDataRequest>, WalletError> {
        Ok(self.pending())
    }
    async fn chain_tip(&self) -> Result<Option<BlockHeight>, WalletError> {
        let harness = self.harness.lock().expect("harness");
        Ok(harness.wallet().chain_height().expect("the engine's tip"))
    }
    async fn set_status(&self, txid: TxId, status: CheckedStatus) -> Result<(), WalletError> {
        self.writes.lock().expect("writes").push(("status", txid));
        let mut harness = self.harness.lock().expect("harness");
        harness
            .state_mut()
            .wallet_mut()
            .set_transaction_status(txid, status.get())
            .expect("the engine takes the status");
        Ok(())
    }
    async fn mined_height_of(&self, txid: TxId) -> Result<Option<BlockHeight>, WalletError> {
        let harness = self.harness.lock().expect("harness");
        Ok(harness
            .wallet()
            .get_tx_height(txid)
            .expect("the engine's height"))
    }
    async fn wallet_birthday(&self) -> Result<Option<BlockHeight>, WalletError> {
        let harness = self.harness.lock().expect("harness");
        Ok(harness
            .wallet()
            .get_wallet_birthday()
            .expect("the engine's birthday"))
    }
    async fn store_decrypted(
        &self,
        tx: Transaction,
        mined_height: Option<BlockHeight>,
    ) -> Result<(), WalletError> {
        self.writes
            .lock()
            .expect("writes")
            .push(("data", tx.txid()));
        let mut harness = self.harness.lock().expect("harness");
        zcash_client_backend::data_api::wallet::decrypt_and_store_transaction(
            &self.network,
            harness.state_mut().wallet_mut(),
            &tx,
            mined_height,
        )
        .expect("the engine stores the transaction");
        Ok(())
    }
}

// ── C1: absence writes nothing ───────────────────────────────────────────────

/// **Row 1 (C1, C6).** An empty body, a gRPC status, a garbled or cut or foreign reply, a
/// transport failure and a timeout write nothing, for either kind of request, and the request
/// stays pending.
///
/// The mechanism that would answer instead is the empty body's arm, which wrote
/// `TxidNotRecognized` for either kind: here that shows as a status naming the absent txid, and
/// (the double loses the row as the engine does) as an `Enhancement` gone from the queue. Every
/// batch holds an honest request on each side of the absent one, so a pass that stopped early, or
/// that wrote nothing at all, fails on those; and the SAME absent request is asked again in the
/// next pass, where it is served and answered.
#[tokio::test]
async fn an_absent_answer_writes_nothing_for_either_request_kind() {
    let (one_bytes, one) = tx_variant(41);
    let (absent_bytes, absent) = tx_variant(42);
    let (two_bytes, two) = tx_variant(43);
    let (foreign_bytes, foreign) = tx_variant(44);
    let targets = [one, absent, two];
    assert!(
        !targets.contains(&foreign),
        "the fixture: the foreign transaction is none of the three"
    );
    let cut = absent_bytes[..absent_bytes.len() / 2].to_vec();
    let at_mined = |data: Vec<u8>| Reply::Tx {
        data,
        height: u64::from(MINED_AT),
    };
    let shapes: [(&str, Reply, bool); 8] = [
        ("an empty body", Reply::NotFound, true),
        ("gRPC NotFound", Reply::Status(GRPC_NOT_FOUND), false),
        ("gRPC Internal", Reply::Status(13), false),
        (
            "garbled bytes",
            at_mined(vec![0xde, 0xad, 0xbe, 0xef]),
            false,
        ),
        ("the transaction cut in half", at_mined(cut), false),
        (
            "another transaction's bytes",
            at_mined(foreign_bytes),
            false,
        ),
        ("a transport failure", Reply::Transport, false),
        ("a timeout", Reply::Timeout, false),
    ];

    for kind in KINDS {
        for (shape, reply, empty_body) in &shapes {
            let label = format!("{shape}, to {kind:?}");
            let store = RecordingStore::with_requests(
                targets.map(|txid| request(kind, txid)).to_vec(),
                Some(BlockHeight::from_u32(TIP)),
            );
            let mut fetcher = ScriptedFetcher::new(vec![
                (one, served(&one_bytes)),
                (absent, served(&absent_bytes)),
                (two, served(&two_bytes)),
            ])
            .scripting(absent, vec![reply.clone()]);
            let cursor = join::Cursor::at_the_end();
            let unparseable = UnparseableTxids::default();

            let out = join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
                .await
                .expect("pass");

            assert_eq!(
                ascending(fetcher.calls.clone()),
                ascending(targets.to_vec()),
                "{label}: every target of the batch is asked, the absent one among them"
            );
            assert_eq!(
                written(&store),
                BTreeSet::from([one, two]),
                "{label}: the two served targets are answered, and nothing is written about \
                 the absent one"
            );
            assert!(
                kind == RequestKind::Status || store.statuses.lock().expect("statuses").is_empty(),
                "{label}: no status is ever written for an Enhancement request"
            );
            assert!(
                store.still_pending(&request(kind, absent)),
                "{label}: the request stays pending"
            );
            assert!(
                !store.still_pending(&request(kind, one))
                    && !store.still_pending(&request(kind, two)),
                "{label}: control: an answered request does leave the queue"
            );
            assert_eq!(
                (out.requested, out.not_found, out.rejected),
                (3, usize::from(*empty_body), usize::from(!*empty_body)),
                "{label}: an empty body is `not_found`, every other absence is `rejected` (C6)"
            );
            assert_eq!(out.enhanced + out.status_set, 2, "{label}");

            // CONTROL: the same request, asked again, and answered by the bytes.
            let asked_before = fetcher.calls.len();
            let again = join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
                .await
                .expect("the next pass");
            assert_eq!(
                fetcher.calls[asked_before..],
                [absent],
                "{label}: the next pass asks the one request still pending"
            );
            assert_eq!(again.enhanced + again.status_set, 1, "{label}");
            assert!(
                written(&store).contains(&absent),
                "{label}: control: the served bytes are what writes"
            );
        }
    }
}

/// **Row 2 (C1), over the engine's own queue.** An empty body for a mined note, for a note a
/// rewind un-mined, and for a foreign parent (a txid the wallet has no row for) leaves all three
/// requests in `tx_retrieval_queue`, and the mined note still reads Confirmed.
///
/// The mechanism that would lose them is the engine's own arm for `TxidNotRecognized`, which
/// deletes an `Enhancement` row whatever the transaction's state. All three are asked twice, so
/// the row does not pass because nothing was asked.
#[tokio::test]
async fn an_empty_reply_keeps_a_real_enhancement_request() {
    let mut harness = HistoryHarness::new();
    let mined = harness.mine_received(456_000);
    let kept_height = harness.current_tip();
    let unmined = harness.mine_received(123_000);
    harness.mine_empty(4);
    harness.scan();
    harness.reorg_to_height(kept_height);
    // A parent of a transparent input of `mined`: queued by the engine under its own txid, with
    // no `transactions` row of its own (`queue_transparent_input_retrieval`).
    let foreign = t(0x5c);
    let planted = harness
        .read_conn()
        .execute(
            "INSERT INTO tx_retrieval_queue (txid, query_type, dependent_transaction_id) \
             SELECT ?1, 1, id_tx FROM transactions WHERE txid = ?2",
            params![foreign.as_ref(), mined.as_ref()],
        )
        .expect("plant the parent's request");
    assert_eq!(
        planted, 1,
        "the fixture: the parent hangs off the mined row"
    );

    let store = EngineStore::over(harness);
    let wanted = [mined, unmined, foreign];
    let pending_all = |store: &EngineStore, when: &str| {
        let pending = store.pending();
        for txid in wanted {
            assert!(
                pending.contains(&TransactionDataRequest::Enhancement(txid)),
                "{when}: the engine's queue holds the request for each of the three"
            );
        }
    };
    pending_all(&store, "premise");
    assert!(
        matches!(store.history_status(mined), TxStatus::Confirmed { .. }),
        "premise: the mined note is confirmed"
    );
    assert_eq!(
        store.history_status(unmined),
        TxStatus::Pending,
        "premise: the rewind un-mined the other note"
    );

    let mut fetcher = ScriptedFetcher::new(Vec::new());
    let cursor = join::Cursor::at_the_end();
    let unparseable = UnparseableTxids::default();
    for pass in 1..=2 {
        let asked_before = fetcher.calls.len();
        let out = join::pass(&store, &mut fetcher, &store.network, &unparseable, &cursor)
            .await
            .expect("pass");
        assert_eq!(
            ascending(fetcher.calls[asked_before..].to_vec()),
            ascending(wanted.to_vec()),
            "pass {pass}: all three are asked, and answered with an empty body"
        );
        assert_eq!(out.not_found, 3, "pass {pass}");
        assert_eq!(store.writes(), NO_WRITES, "pass {pass}: nothing is written");
        pending_all(&store, "after an empty body");
        assert!(
            matches!(store.history_status(mined), TxStatus::Confirmed { .. }),
            "pass {pass}: and the mined note still reads Confirmed"
        );
    }
}

/// **Row 3 (C1, C4), end to end over the engine.** One seed, two wallets: the first pays its own
/// address with a memo, the second mines and scans that transaction, so its compact block leaves
/// the memo out. Twelve passes of outage (gRPC NotFound, and on every third an empty body: the
/// two shapes an outage has been seen or claimed to take) leave the request pending, the memo
/// absent and nothing written; the thirteenth serves the bytes and the memo reads back.
///
/// The mechanism that would lose it is the same as row 2's, on the empty-body passes.
#[tokio::test]
async fn a_memo_survives_an_outage_and_lands_when_it_ends() {
    const MEMO: &str = "kept through an outage";
    const OUTAGE_PASSES: usize = 12;

    let mut payer = HistoryHarness::new();
    payer.mine_received(1_000_000);
    payer.mine_empty(12); // past the spendable depth
    payer.scan();
    let own_address = payer.own_sapling_address();
    let paid = payer.create_send_with_memo(own_address, 20_000, MEMO);
    let raw: Vec<u8> = payer
        .read_conn()
        .query_row(
            "SELECT raw FROM transactions WHERE txid = ?1",
            params![paid.as_ref()],
            |r| r.get(0),
        )
        .expect("the signed bytes");
    let tx = Transaction::read(&raw[..], BranchId::Nu5).expect("our own bytes parse");
    assert_eq!(
        tx.txid(),
        paid,
        "the fixture: the bytes are the transaction's"
    );

    let mut receiver = HistoryHarness::new();
    receiver.mine_transaction(&tx);
    let mined_at = receiver.current_tip();
    receiver.mine_empty(3);
    receiver.scan();
    let store = EngineStore::over(receiver);
    let request = TransactionDataRequest::Enhancement(paid);
    assert!(
        matches!(store.history_status(paid), TxStatus::Confirmed { .. }),
        "premise: the second wallet scanned the payment"
    );
    assert!(
        store.pending().contains(&request),
        "premise: and wants its bytes"
    );
    assert_eq!(
        store.text_memos(paid),
        Vec::<String>::new(),
        "premise: a compact block carries no memo"
    );

    let outage = (1..=OUTAGE_PASSES)
        .map(|pass| {
            if pass.is_multiple_of(3) {
                Reply::NotFound
            } else {
                Reply::Status(GRPC_NOT_FOUND)
            }
        })
        .collect();
    let mut fetcher = ScriptedFetcher::new(vec![(
        paid,
        Reply::Tx {
            data: raw,
            height: u64::from(mined_at),
        },
    )])
    .scripting(paid, outage);
    let cursor = join::Cursor::at_the_end();
    let unparseable = UnparseableTxids::default();

    for pass in 1..=OUTAGE_PASSES {
        join::pass(&store, &mut fetcher, &store.network, &unparseable, &cursor)
            .await
            .expect("an outage never fails a pass");
        assert_eq!(
            fetcher.calls,
            vec![paid; pass],
            "pass {pass}: the request is asked once in every pass of the outage"
        );
        assert_eq!(store.writes(), NO_WRITES, "pass {pass}: nothing is written");
        assert!(
            store.pending().contains(&request),
            "pass {pass}: the request is still pending"
        );
        assert_eq!(
            store.text_memos(paid),
            Vec::<String>::new(),
            "pass {pass}: and the memo is still absent"
        );
    }

    let out = join::pass(&store, &mut fetcher, &store.network, &unparseable, &cursor)
        .await
        .expect("the pass after the outage");
    assert_eq!(out.enhanced, 1, "the bytes are served and decrypted");
    assert_eq!(store.writes(), vec![("data", paid)]);
    assert_eq!(
        store.text_memos(paid),
        vec![MEMO.to_owned()],
        "the memo reads back"
    );
    assert!(
        !store.pending().contains(&request),
        "and the request is answered"
    );
}

// ── C3: an unmined answer past the expiry is not written ─────────────────────

/// **Row 4's pure rows (C3).** `chain_status` answers an unmined reply `NotInMainChain` below the
/// transaction's effective expiry and refuses it from that height on. Each expiry here comes from
/// the kernel over a real header: a stored expiry the branch outlives, a stored expiry the NU7
/// activation cuts short (the one the stored height cannot see), a stored 0 the activation ends,
/// and a stored 0 no activation ends.
#[test]
fn an_unmined_answer_is_a_status_only_below_the_effective_expiry() {
    let activation = u32::from(
        MainNetwork
            .activation_height(NetworkUpgrade::Nu7)
            .expect("mainnet's NU7 height"),
    );
    let of = |branch: u32, stored: u32| {
        crate::expiry::effective_expiry(&MainNetwork, &v6_header(branch), stored)
    };
    let own = activation - 100;
    let cut = activation + 25;
    let rows: [(&str, EffectiveExpiry, u32, bool); 15] = [
        (
            "ends by itself, one block before",
            of(NU6_3, own),
            own - 1,
            true,
        ),
        ("ends by itself, at the expiry", of(NU6_3, own), own, false),
        ("ends by itself, past it", of(NU6_3, own), own + 1, false),
        (
            "cut by the upgrade, the last block it can be answered at",
            of(NU6_3, cut),
            activation - 2,
            true,
        ),
        (
            "cut by the upgrade, at the block before the activation",
            of(NU6_3, cut),
            activation - 1,
            false,
        ),
        (
            "cut by the upgrade, the tip between the two heights",
            of(NU6_3, cut),
            activation + 10,
            false,
        ),
        (
            "cut by the upgrade, one block below the stored height",
            of(NU6_3, cut),
            cut - 1,
            false,
        ),
        (
            "cut by the upgrade, at the stored height",
            of(NU6_3, cut),
            cut,
            false,
        ),
        (
            "no stored expiry, ended by the upgrade, before",
            of(NU6_3, 0),
            activation - 2,
            true,
        ),
        (
            "no stored expiry, ended by the upgrade, at the block before it",
            of(NU6_3, 0),
            activation - 1,
            false,
        ),
        (
            "no stored expiry, ended by the upgrade, long after",
            of(NU6_3, 0),
            activation + 100_000,
            false,
        ),
        (
            "no stored expiry and no next activation",
            of(NU7, 0),
            activation + 100_000,
            true,
        ),
        (
            "no stored expiry and no next activation, at any tip",
            of(NU7, 0),
            u32::MAX,
            true,
        ),
        (
            "a stored expiry under the running branch, one block before",
            of(NU7, cut),
            cut - 1,
            true,
        ),
        (
            "a transaction no block can carry",
            EffectiveExpiry::Nowhere,
            1,
            false,
        ),
    ];
    for (label, expiry, tip, answered) in rows {
        let status = chain_status(
            None,
            &StatusEvidence {
                known_tip: BlockHeight::from_u32(tip),
                birthday: Some(BlockHeight::from_u32(1)),
                already_known: None,
                expiry,
            },
        );
        assert_eq!(
            status.map(CheckedStatus::get),
            answered.then_some(TransactionStatus::NotInMainChain),
            "{label} (tip {tip}, {expiry:?})"
        );
    }
}

/// **Row 4's upgrade row, through the pass.** The pass builds the evidence from the reply, and it
/// is there that a stored expiry could stand in for the effective one: a `Nu6_3` transaction whose
/// stored expiry is above the NU7 activation, a tip between the two, and both unmined sentinels.
/// No write, one refusal in `rejected`, nothing logged, the request still pending. The same
/// request one block before its effective expiry IS answered, and a HEIGHT refused at the very
/// same tip does log its warning, so the silence is the refusal's and not the capture's.
#[tokio::test]
async fn an_unmined_answer_between_an_upgrade_and_the_stored_expiry_writes_and_logs_nothing() {
    let activation = u32::from(
        MainNetwork
            .activation_height(NetworkUpgrade::Nu7)
            .expect("mainnet's NU7 height"),
    );
    let stored = activation + 25;
    let between = activation + 10;
    let (bytes, leg) = tx_variant_expiring(51, stored);
    for (sentinel, wire) in [("mempool", 0u64), ("off the main chain", u64::MAX)] {
        let store = RecordingStore::with_requests(
            vec![TransactionDataRequest::GetStatus(leg)],
            Some(BlockHeight::from_u32(activation - 2)),
        );
        let mut fetcher = ScriptedFetcher::new(vec![(
            leg,
            Reply::Tx {
                data: bytes.clone(),
                height: wire,
            },
        )]);
        let cursor = join::Cursor::at_the_end();
        let unparseable = UnparseableTxids::default();

        // CONTROL: below the effective expiry it is an answer.
        let out = join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
            .await
            .expect("pass");
        assert_eq!(
            *store.statuses.lock().expect("statuses"),
            vec![(leg, TransactionStatus::NotInMainChain)],
            "{sentinel}: one block before the effective expiry the answer is written"
        );
        assert_eq!((out.status_set, out.rejected), (1, 0), "{sentinel}");

        // The tip passes the effective expiry and stays below the stored one.
        store.set_tip(BlockHeight::from_u32(between));
        let (out, fields) = logged_during(join::pass(
            &store,
            &mut fetcher,
            &MainNetwork,
            &unparseable,
            &cursor,
        ))
        .await;
        let out = out.expect("pass");
        assert_eq!(fetcher.calls, vec![leg, leg], "{sentinel}: it was asked");
        assert_eq!(
            store.statuses.lock().expect("statuses").len(),
            1,
            "{sentinel}: past the effective expiry the same answer writes nothing more"
        );
        assert_eq!((out.status_set, out.rejected), (0, 1), "{sentinel}");
        assert!(
            fields.is_empty(),
            "{sentinel}: and it logs nothing. Captured: {fields:?}"
        );
        assert!(
            store.still_pending(&TransactionDataRequest::GetStatus(leg)),
            "{sentinel}: the question stays"
        );
    }

    // CONTROL for the silence: a HEIGHT the kernel refuses, at the same tip, warns.
    let store = RecordingStore::with_requests(
        vec![TransactionDataRequest::GetStatus(leg)],
        Some(BlockHeight::from_u32(between)),
    );
    let mut fetcher = ScriptedFetcher::new(vec![(
        leg,
        Reply::Tx {
            data: bytes,
            height: u64::from(activation),
        },
    )]);
    let (cursor, unparseable) = (join::Cursor::at_the_end(), UnparseableTxids::default());
    let (out, fields) = logged_during(join::pass(
        &store,
        &mut fetcher,
        &MainNetwork,
        &unparseable,
        &cursor,
    ))
    .await;
    assert_eq!(out.expect("pass").rejected, 1, "the height is refused");
    assert!(
        fields
            .iter()
            .any(|(_, v)| v.contains("contradicts what this wallet already knows")),
        "a refused HEIGHT keeps its warning. Captured: {fields:?}"
    );
}

// ── C4: rotation ─────────────────────────────────────────────────────────────

/// **Row 6 (C4).** Sixty requests nobody serves, of both kinds, and one fresh one the endpoint
/// does serve, at each end of the txid order, in its middle and past the cap: whenever it is
/// raised, the fresh one is asked within two passes, and every request pending is asked once in
/// every two. Each pass asks exactly the cap, no txid twice.
///
/// The mechanism that would starve it is a pass that always asks the first fifty.
#[tokio::test]
async fn a_request_that_nobody_serves_never_starves_another() {
    // A real transaction (the fresh request is SERVED) whose txid leaves room for sixty made-up
    // ones on either side of it.
    let lead = |t: &TxId| <[u8; 32]>::from(*t)[0];
    let (bytes, fresh) = (0..)
        .map(tx_variant)
        .find(|(_, t)| (100..=150).contains(&lead(t)))
        .expect("one in five txids leads with such a byte");

    for fresh_kind in KINDS {
        for below in [0u8, 30, 55, 60] {
            for passes_before in [0usize, 1, 3] {
                let label = format!(
                    "a fresh {fresh_kind:?} with {below} requests below it, raised after \
                     {passes_before} passes"
                );
                let unserved: Vec<TxId> = (0..60u8)
                    .map(|i| if i < below { t(i + 1) } else { t(255 - i) })
                    .collect();
                let requests = unserved
                    .iter()
                    .zip(KINDS.iter().cycle())
                    .map(|(txid, kind)| request(*kind, *txid))
                    .collect();
                let store =
                    RecordingStore::with_requests(requests, Some(BlockHeight::from_u32(TIP)));
                let mut fetcher = ScriptedFetcher::new(vec![(fresh, served(&bytes))])
                    .answering_the_rest(Reply::Status(GRPC_NOT_FOUND));
                let cursor = join::Cursor::at_the_end();
                let unparseable = UnparseableTxids::default();

                let mut asked: Vec<Vec<TxId>> = Vec::new();
                for pass in 0..passes_before + 4 {
                    if pass == passes_before {
                        store.add_request(request(fresh_kind, fresh));
                    }
                    let from = fetcher.calls.len();
                    join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
                        .await
                        .expect("pass");
                    asked.push(fetcher.calls[from..].to_vec());
                }

                for (pass, batch) in asked.iter().enumerate() {
                    assert_eq!(
                        batch.iter().collect::<BTreeSet<_>>().len(),
                        MAX_ENHANCEMENTS_PER_PASS,
                        "{label}: pass {pass} asks the cap's number of DIFFERENT txids"
                    );
                    assert_eq!(batch.len(), MAX_ENHANCEMENTS_PER_PASS, "{label}");
                }
                let after = &asked[passes_before..];
                assert!(
                    after[0].contains(&fresh) || after[1].contains(&fresh),
                    "{label}: it is asked within two passes of being raised"
                );
                assert_eq!(
                    fetcher.calls.iter().filter(|t| **t == fresh).count(),
                    1,
                    "{label}: and once served it is asked no more"
                );
                for (pass, two) in asked.windows(2).enumerate() {
                    for txid in &unserved {
                        assert!(
                            two[0].contains(txid) || two[1].contains(txid),
                            "{label}: every unserved request is asked in passes {pass} and {} \
                             taken together",
                            pass + 1
                        );
                    }
                }
            }
        }
    }
}

/// **Row 7 (C4), the selector.** The eligible requests are sorted by txid and taken from strictly
/// after the cursor, wrapping, up to the cap.
#[test]
fn the_cursor_starts_after_the_last_asked_txid_and_wraps() {
    let none = UnparseableTxids::default();
    // Listed out of order on purpose: the order is the selector's, not the backend's.
    let listed = |txids: &[u8]| -> Vec<TransactionDataRequest> {
        txids
            .iter()
            .map(|b| TransactionDataRequest::Enhancement(t(*b)))
            .collect()
    };
    let five = [4u8, 1, 5, 3, 2];
    // What it is, the txids listed, the cap, the cursor, and the txids selected.
    type Row<'a> = (&'a str, &'a [u8], usize, Option<u8>, &'a [u8]);
    let rows: [Row<'_>; 11] = [
        ("from the middle", &five, 10, Some(3), &[4, 5, 1, 2, 3]),
        (
            "at the last txid, it wraps",
            &five,
            10,
            Some(5),
            &[1, 2, 3, 4, 5],
        ),
        (
            "past the last txid, it wraps",
            &five,
            10,
            Some(9),
            &[1, 2, 3, 4, 5],
        ),
        (
            "before the first txid",
            &five,
            10,
            Some(0),
            &[1, 2, 3, 4, 5],
        ),
        (
            "no cursor is the whole order",
            &five,
            10,
            None,
            &[1, 2, 3, 4, 5],
        ),
        (
            "the row at the cursor is gone: its neighbour is not skipped",
            &[4, 1, 5, 2],
            10,
            Some(3),
            &[4, 5, 1, 2],
        ),
        (
            "the cursor stands between two rows",
            &[1, 2, 6, 8],
            10,
            Some(4),
            &[6, 8, 1, 2],
        ),
        ("the cap, across the wrap", &five, 3, Some(4), &[5, 1, 2]),
        ("the cap, short of the wrap", &five, 2, Some(1), &[2, 3]),
        (
            "one request, at the cursor, is still asked",
            &[7],
            10,
            Some(7),
            &[7],
        ),
        ("nothing to ask", &[], 10, Some(7), &[]),
    ];
    for (label, requests, max, after, want) in rows {
        let got: Vec<TxId> = join::select(listed(requests), max, &none, after.map(t))
            .into_iter()
            .map(|(txid, _)| txid)
            .collect();
        let want: Vec<TxId> = want.iter().map(|b| t(*b)).collect();
        assert_eq!(got, want, "{label}");
    }

    // Both kinds of one txid are one ask, and `Enhancement` subsumes `GetStatus`, wherever the
    // cursor stands and whichever is listed first, the cap included.
    for after in [None, Some(t(1)), Some(t(2)), Some(t(3))] {
        for requests in [
            vec![
                TransactionDataRequest::GetStatus(t(2)),
                TransactionDataRequest::Enhancement(t(2)),
            ],
            vec![
                TransactionDataRequest::Enhancement(t(2)),
                TransactionDataRequest::GetStatus(t(2)),
            ],
        ] {
            assert_eq!(
                join::select(requests, 1, &none, after),
                vec![(t(2), RequestKind::Enhancement)],
                "one ask for the txid, after {after:?}"
            );
        }
    }
}

/// **Row 9 (C4).** The cursor is the last txid ASKED, written before its fetch is awaited: a pass
/// dropped while a fetch hangs, and a pass a store fault ended, both leave the next pass to start
/// after the txid they were asking, which is asked again only once its turn comes round.
///
/// Three wrong cursors are told apart here: one set to the last txid SELECTED starts the second
/// pass at the first request again, one written after the loop does the same, and one written
/// after the fetch starts at the hung txid.
#[tokio::test]
async fn a_dropped_pass_resumes_after_the_txid_it_was_asking() {
    // A fetch that hangs.
    let five = [t(1), t(2), t(3), t(4), t(5)];
    let store = RecordingStore::with_backlog(&five, Some(BlockHeight::from_u32(TIP)));
    let mut fetcher = ScriptedFetcher::new(Vec::new())
        .scripting(t(3), vec![Reply::Hang])
        .answering_the_rest(Reply::Status(GRPC_NOT_FOUND));
    let cursor = join::Cursor::at_the_end();
    drop_the_pass_at_the_hang(&store, &mut fetcher, &cursor).await;
    assert_eq!(
        fetcher.calls,
        [t(1), t(2), t(3)],
        "the fixture: the pass was dropped while it asked the third"
    );
    let unparseable = UnparseableTxids::default();
    join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
        .await
        .expect("the next pass");
    assert_eq!(
        fetcher.calls[3..],
        [t(4), t(5), t(1), t(2), t(3)],
        "the next pass starts after the hung txid, and comes to it last"
    );

    // A store that faults, on the write of either kind.
    for kind in KINDS {
        let mut served_txs: Vec<(Vec<u8>, TxId)> = (61..66).map(tx_variant).collect();
        served_txs.sort_by_key(|(_, txid)| *txid);
        let txids: Vec<TxId> = served_txs.iter().map(|(_, txid)| *txid).collect();
        let faulting = txids[2];
        let store = RecordingStore::with_requests(
            txids.iter().map(|txid| request(kind, *txid)).collect(),
            Some(BlockHeight::from_u32(TIP)),
        )
        .faulting_on(faulting);
        let mut fetcher = ScriptedFetcher::new(
            served_txs
                .iter()
                .map(|(bytes, txid)| (*txid, served(bytes)))
                .collect(),
        );
        let cursor = join::Cursor::at_the_end();

        let first = join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor).await;
        assert!(
            matches!(first, Err(WalletError::StoreCorrupt)),
            "{kind:?}: the fixture: the fault ends the pass, as it always did"
        );
        assert_eq!(fetcher.calls, txids[..3], "{kind:?}: at the third request");

        let second = join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor).await;
        assert!(
            matches!(second, Err(WalletError::StoreCorrupt)),
            "{kind:?}: the row still faults when its turn comes"
        );
        assert_eq!(
            fetcher.calls[3..],
            [txids[3], txids[4], faulting],
            "{kind:?}: the next pass starts after the faulting txid, so the two behind it are \
             asked and answered, and it comes round last"
        );
        assert!(
            written(&store).contains(&txids[3]) && written(&store).contains(&txids[4]),
            "{kind:?}: one faulting row holds no other request back"
        );
    }
}

/// An unlisted row for C4's claim about the passes that do NOT run to their end: no request can
/// hold another back in them either. An endpoint that hangs on every ask, under a host that drops
/// every pass at its deadline, is asked about each of the five requests in five passes.
#[tokio::test]
async fn an_endpoint_that_hangs_on_every_ask_is_asked_about_every_request_in_turn() {
    let five = [t(1), t(2), t(3), t(4), t(5)];
    let store = RecordingStore::with_backlog(&five, Some(BlockHeight::from_u32(TIP)));
    let mut fetcher = ScriptedFetcher::new(Vec::new()).answering_the_rest(Reply::Hang);
    let cursor = join::Cursor::at_the_end();
    for _ in 0..five.len() {
        drop_the_pass_at_the_hang(&store, &mut fetcher, &cursor).await;
    }
    assert_eq!(
        fetcher.calls, five,
        "each dropped pass asks the next request"
    );
}

// ── C5: the unparseable set is for enhancements only ─────────────────────────

/// **Row 10 (C5).** A reply that names an unknown branch, to a `Status` request, is a refused
/// reply like any other: no entry in [`UnparseableTxids`], no `unknown_branch` count, no
/// stale-build warning, and the request is asked again in the next pass, where an honest answer
/// confirms it. To an `Enhancement` request it is what it was: counted, warned, and asked no more
/// in this process.
///
/// The forged header is the same twelve bytes in both halves, so the second half is also the
/// control that the fixture is one the reader does recognise.
#[tokio::test]
async fn a_status_request_is_never_hidden_by_a_forged_branch() {
    assert!(
        BranchId::try_from(UNKNOWN_BRANCH).is_err(),
        "the fixture must name a branch this build genuinely lacks"
    );
    let forged = Reply::Tx {
        data: v6_header(UNKNOWN_BRANCH),
        height: u64::from(MINED_AT),
    };
    let (bytes, leg) = tx_variant(71);
    let warned = |fields: &[(String, String)]| fields.iter().any(|(n, _)| n == "endpoint_branch");

    // A Status request.
    let store = RecordingStore::with_requests(
        vec![TransactionDataRequest::GetStatus(leg)],
        Some(BlockHeight::from_u32(TIP)),
    );
    let mut fetcher =
        ScriptedFetcher::new(vec![(leg, served(&bytes))]).scripting(leg, vec![forged.clone()]);
    let cursor = join::Cursor::at_the_end();
    let unparseable = UnparseableTxids::default();
    let (out, fields) = logged_during(join::pass(
        &store,
        &mut fetcher,
        &MainNetwork,
        &unparseable,
        &cursor,
    ))
    .await;
    let out = out.expect("pass");
    assert_eq!(fetcher.calls, [leg], "it was asked");
    assert_eq!(
        (out.rejected, out.unknown_branch),
        (1, 0),
        "a refused reply like any other"
    );
    assert_eq!(unparseable.len(), 0, "no entry that would hide it");
    assert!(
        !warned(&fields),
        "no stale-build warning: this build made or parsed the transaction. Captured: {fields:?}"
    );
    assert!(
        written(&store).is_empty() && store.still_pending(&TransactionDataRequest::GetStatus(leg)),
        "nothing is written and the question stays"
    );

    let out = join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
        .await
        .expect("the next pass");
    assert_eq!(fetcher.calls, [leg, leg], "it is asked again");
    assert_eq!(out.status_set, 1);
    assert_eq!(
        *store.statuses.lock().expect("statuses"),
        vec![(
            leg,
            TransactionStatus::Mined(BlockHeight::from_u32(MINED_AT))
        )],
        "and an honest answer confirms it"
    );

    // An Enhancement request: as today.
    let store = RecordingStore::with_backlog(&[leg], Some(BlockHeight::from_u32(TIP)));
    let mut fetcher = ScriptedFetcher::new(vec![(leg, forged)]);
    let unparseable = UnparseableTxids::default();
    let (out, fields) = logged_during(join::pass(
        &store,
        &mut fetcher,
        &MainNetwork,
        &unparseable,
        &cursor,
    ))
    .await;
    let out = out.expect("pass");
    assert_eq!((out.rejected, out.unknown_branch), (1, 1));
    assert_eq!(unparseable.len(), 1);
    assert!(
        warned(&fields),
        "the operator's warning. Captured: {fields:?}"
    );
    let out = join::pass(&store, &mut fetcher, &MainNetwork, &unparseable, &cursor)
        .await
        .expect("the next pass");
    assert_eq!(
        (out.requested, fetcher.calls.len()),
        (0, 1),
        "and it is asked once in this process"
    );
    assert!(store.still_pending(&TransactionDataRequest::Enhancement(leg)));
}

/// **Row 11 (C5), the selector.** With both rows of one txid pending and the txid in the set, the
/// `Enhancement` row is skipped BEFORE the merge, so the `Status` row survives as itself, in
/// either listing order. (The engine does not reach this state at 0.23.0; the row pins the
/// selector.) Controls: an empty set merges them as it always did, and a skipped `Enhancement`
/// alone is skipped.
#[test]
fn an_enhancement_skip_does_not_hide_a_status_row_for_the_same_txid() {
    let skipping = UnparseableTxids::default();
    skipping.insert(t(2));
    let none = UnparseableTxids::default();
    let enhancement = TransactionDataRequest::Enhancement(t(2));
    let status = TransactionDataRequest::GetStatus(t(2));

    for (label, both) in [
        (
            "enhancement listed first",
            vec![enhancement.clone(), status.clone()],
        ),
        (
            "status listed first",
            vec![status.clone(), enhancement.clone()],
        ),
    ] {
        assert_eq!(
            join::select(both.clone(), 10, &skipping, None),
            vec![(t(2), RequestKind::Status)],
            "{label}: the status question is asked, as itself"
        );
        assert_eq!(
            join::select(both, 10, &none, None),
            vec![(t(2), RequestKind::Enhancement)],
            "{label}: control: with nothing skipped the Enhancement subsumes it"
        );
    }
    assert_eq!(
        join::select(vec![status], 10, &skipping, None),
        vec![(t(2), RequestKind::Status)],
        "a Status row alone is never skipped"
    );
    assert_eq!(
        join::select(
            vec![enhancement, TransactionDataRequest::Enhancement(t(3))],
            10,
            &skipping,
            None
        ),
        vec![(t(3), RequestKind::Enhancement)],
        "control: an Enhancement in the set is skipped, and its neighbour is not"
    );
    // The skip comes before the cap: it costs the batch no place.
    assert_eq!(
        join::select(
            vec![
                TransactionDataRequest::Enhancement(t(2)),
                TransactionDataRequest::Enhancement(t(3)),
                TransactionDataRequest::GetStatus(t(4)),
            ],
            2,
            &skipping,
            None
        ),
        vec![
            (t(3), RequestKind::Enhancement),
            (t(4), RequestKind::Status)
        ],
        "the skipped row takes no place under the cap"
    );
}

// The rows that name the cursor itself. Gated on its own line as well: the
// source scans under tests/ skip a file only by its own declaration's gate.
#[cfg(test)]
mod named;
