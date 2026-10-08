//! Shared `#[cfg(test)]` wallet harness — a `data_api::testing` `TestBuilder` over OUR
//! real [`WalletDb`] (the SAME store the live wallet drives), an in-memory block cache,
//! and funding/mining/reorg helpers.
//!
//! ## Why this exists (the FILE-BACKED mode)
//! The history/intent read path runs SQL on a SECOND `rusqlite::Connection` (the `aux_db`),
//! separate from the engine's `WalletDb` connection. Unit-testing the history queries
//! against a *hand-built* `v_transactions`-shaped fixture pins the query LOGIC but cannot
//! prove the queries behave correctly against librustzcash's REAL `v_transactions` VIEW
//! populated by a REAL scan (the view's column semantics — signed `account_balance_delta`,
//! NULL `mined_height`/`tx_index` on un-mined or `GetStatus`-path rows, the account scoping)
//! are upstream's, not ours. This harness closes that gap: it mines real compact notes,
//! SCANS them (the decrypt path that fills the view), and exposes the wallet DB as a temp
//! FILE so a second [`HistoryHarness::read_conn`] — the exact production aux pattern — can
//! page/look-up/reorg over the real rows. `truncate_to_height` (cache) and
//! `set_transaction_status` (the `GetStatus`/reorg path) drive the un-mine / re-mine edges
//! that a fixture can only fake.
//!
//! No SQLCipher key is applied here — these tests exercise scan + query LOGIC, not at-rest
//! encryption (SQLCipher is transparent once keyed; the keyed open path is covered by
//! `db.rs` and the history `queries_run_against_the_real_schema…` test). The harness runs
//! on the `data_api::testing` `LocalNetwork`, exactly as the `send` proving harness does.

#![cfg(test)]

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::time::Duration;

use rand_core::OsRng;
use rusqlite::Connection;
use tempfile::TempDir;
use zcash_client_backend::data_api::WalletWrite;
use zcash_client_backend::data_api::anchor_retention::AnchorRetentionInterval;
use zcash_client_backend::data_api::chain::BlockSource;
use zcash_client_backend::data_api::chain::error::Error as ChainError;
use zcash_client_backend::data_api::testing::{
    AddressType, CacheInsertionResult, DataStoreFactory, FakeCompactOutput, IronwoodFvk,
    TestBuilder, TestCache, TestState,
};
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_sqlite::WalletDb;
use zcash_client_sqlite::util::SystemClock;
use zcash_client_sqlite::wallet::init::init_wallet_db;
use zcash_keys::keys::transparent::gap_limits::GapLimits;
use zcash_primitives::block::BlockHash;
use zcash_protocol::TxId;
use zcash_protocol::consensus::BlockHeight;
use zcash_protocol::local_consensus::LocalNetwork;
use zcash_protocol::value::Zatoshis as ProtoZat;

use crate::constants::SQLITE_BUSY_TIMEOUT_MS;

/// The harness wallet DB — the SAME `WalletDb` shape the live wallet uses, on the fake
/// `LocalNetwork` the `data_api::testing` builder requires. (Deliberately the same shape as
/// `send::tests`'s aliases; the distinguishing axis between the two harnesses is the cache's
/// `InsertResult` and in-memory-vs-file storage, not these types — see the module doc.)
pub(crate) type HarnessDb = WalletDb<Connection, LocalNetwork, SystemClock, OsRng>;
pub(crate) type HarnessState = TestState<MemCache, HarnessDb, LocalNetwork>;

// ── In-memory block cache (a `TestCache` over a height→block map) ───────────────

#[derive(Default)]
pub(crate) struct MemCache {
    blocks: BTreeMap<u64, CompactBlock>,
}

/// The txids of the block just inserted (INTERNAL byte order, straight off each
/// `CompactTx.hash`). The `send` propose tests ignore these; the history reorg tests use
/// them to name the row to un-mine via `set_transaction_status` without a second query.
pub(crate) struct BlockTxids(Vec<TxId>);
impl CacheInsertionResult for BlockTxids {
    fn txids(&self) -> &[TxId] {
        &self.0
    }
}

impl BlockSource for MemCache {
    type Error = Infallible;
    fn with_blocks<F, WalletErrT>(
        &self,
        from_height: Option<BlockHeight>,
        limit: Option<usize>,
        mut with_block: F,
    ) -> Result<(), ChainError<WalletErrT, Infallible>>
    where
        F: FnMut(CompactBlock) -> Result<(), ChainError<WalletErrT, Infallible>>,
    {
        let from = from_height.map(|h| u64::from(u32::from(h))).unwrap_or(0);
        let mut remaining = limit.unwrap_or(usize::MAX);
        for (_h, cb) in self.blocks.range(from..) {
            if remaining == 0 {
                break;
            }
            with_block(cb.clone())?;
            remaining -= 1;
        }
        Ok(())
    }
}

impl TestCache for MemCache {
    type BsError = Infallible;
    type BlockSource = Self;
    type InsertResult = BlockTxids;

    fn block_source(&self) -> &Self {
        self
    }
    fn insert(&mut self, cb: &CompactBlock) -> BlockTxids {
        self.blocks.insert(cb.height, cb.clone());
        BlockTxids(
            cb.vtx
                .iter()
                .map(|tx| {
                    let mut h = [0u8; 32];
                    h.copy_from_slice(&tx.txid);
                    TxId::from_bytes(h)
                })
                .collect(),
        )
    }
    fn truncate_to_height(&mut self, height: BlockHeight) {
        let h = u64::from(u32::from(height));
        self.blocks.retain(|k, _| *k <= h);
    }
}

// ── Our DataStoreFactory: a real `WalletDb` over a temp FILE ─────────────────────

/// Builds the harness `WalletDb` against a temp FILE (not in-memory) so a SECOND
/// `rusqlite::Connection` can read the SAME DB — the production aux pattern the history
/// queries depend on. (`send`'s propose harness uses its own in-memory factory; it needs no
/// second connection, so this stays file-only — YAGNI.)
pub(crate) struct FileBackedDsf {
    /// `pub(crate)` since `ironwood_spendability` builds its own `TestBuilder`
    /// (a different network, an initial chain state this file's harness does not want)
    /// over the SAME factory rather than duplicating it — and it needs the path for the
    /// second connection, exactly as [`HistoryHarness::read_conn`] does.
    pub(crate) path: PathBuf,
}

impl DataStoreFactory for FileBackedDsf {
    type Error = ();
    type AccountId = <HarnessDb as zcash_client_backend::data_api::WalletRead>::AccountId;
    type Account = <HarnessDb as zcash_client_backend::data_api::WalletRead>::Account;
    type DsError = <HarnessDb as zcash_client_backend::data_api::WalletRead>::Error;
    type DataStore = HarnessDb;

    fn new_data_store(
        &self,
        network: LocalNetwork,
        anchor_retention_interval: Option<AnchorRetentionInterval>,
        gap_limits: Option<GapLimits>,
    ) -> Result<Self::DataStore, ()> {
        // `for_path` opens the file AND loads the rarray vtab module `WalletDb` requires. No
        // SQLCipher key (see module doc — query/scan logic under test, not at-rest crypto).
        let mut db = WalletDb::for_path(&self.path, network, SystemClock, OsRng).map_err(|_| ())?;
        // 0.24.0's anchor-retention knob, applied as upstream's own `TestDbFactory`
        // does (None ⇒ the engine default, `ZIP_318`). Ignoring it would silently
        // hand a test the default when it asked for something else.
        if let Some(interval) = anchor_retention_interval {
            db = db.with_anchor_retention_interval(interval);
        }
        // Apply the builder's gap limits exactly as upstream `TestDbFactory` does (None ⇒
        // the engine default); transparent gap behavior is not under test in these helpers.
        if let Some(gap_limits) = gap_limits {
            db = db.with_gap_limits(gap_limits);
        }
        init_wallet_db(&mut db, None).map_err(|_| ())?;
        Ok(db)
    }
}

// ── The file-backed history harness ─────────────────────────────────────────────

/// A funded, file-backed wallet whose REAL `v_transactions` view can be paged over a second
/// aux connection. Mine notes with [`Self::mine_received`], bury them with
/// [`Self::mine_empty`], [`Self::scan`] to populate the view, then [`Self::read_conn`] to run
/// the production history SQL against real scanned rows. Field order matters: `state` (which
/// owns the engine connection to the temp file) drops BEFORE `_dir` (which deletes the file).
pub(crate) struct HistoryHarness {
    state: HarnessState,
    path: PathBuf,
    _dir: TempDir,
    /// The lowest cached block height (the scan start), set on the first mine.
    first: Option<BlockHeight>,
    /// How many blocks have been cached since construction (the scan span).
    cached: usize,
    /// How many blocks have already been scanned — [`Self::scan`] only scans the NEW tail, so
    /// re-scanning never re-derives an already-scanned block (which would, e.g., restore a
    /// `tx_index` the reorg test deliberately cleared via `set_transaction_status`).
    scanned: usize,
    /// The highest cached block height (the chain tip to publish after a scan).
    tip: Option<BlockHeight>,
}

impl HistoryHarness {
    /// A fresh wallet with one Sapling account at activation and no history yet.
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("wallet.db");
        let state = TestBuilder::new()
            .with_data_store_factory(FileBackedDsf { path: path.clone() })
            .with_block_cache(MemCache::default())
            .with_account_from_sapling_activation(BlockHash([0; 32]))
            .build();
        Self {
            state,
            path,
            _dir: dir,
            first: None,
            cached: 0,
            scanned: 0,
            tip: None,
        }
    }

    fn note_block(&mut self) {
        let h = self.tip.map(|t| t + 1).unwrap_or_else(|| {
            // first block lands one past the account's activation height
            self.state.sapling_activation_height()
        });
        self.first.get_or_insert(h);
        self.cached += 1;
        self.tip = Some(h);
    }

    /// Mine ONE sapling note of `value` paying this account's external receive address, in a
    /// new block at the next height. Returns the mined transaction's (display-order) txid.
    /// NOT yet scanned — call [`Self::scan`] to populate `v_transactions`.
    pub(crate) fn mine_received(&mut self, value: u64) -> TxId {
        let dfvk = self
            .state
            .test_account_sapling()
            .cloned()
            .expect("sapling dfvk");
        let (_h, res, _nf) = self.state.generate_next_block(
            &dfvk,
            AddressType::DefaultExternal,
            ProtoZat::const_from_u64(value),
        );
        self.note_block();
        // A single FakeCompactOutput ⇒ exactly one tx in the block.
        res.txids()[0]
    }

    /// Mine ONE **Ironwood** note of `value` paying this account's external receive address,
    /// in a new block at the next height. Returns the mined transaction's (display-order)
    /// txid. NOT yet scanned — call [`Self::scan`].
    ///
    /// T0-3 (§4z). The additive sibling of [`Self::mine_received`], and the only way to
    /// produce the shape the item is about: a batch whose ONLY receipt is Ironwood.
    /// `IronwoodFvk` is upstream's own test key (`data_api/testing.rs:2485`) — it wraps the
    /// account's Orchard FVK, but its outputs are version-3 notes placed in
    /// `tx.ironwood_actions` and encrypted under the Ironwood note-encryption domain, so a
    /// wallet scanning them takes the real Ironwood receive path rather than the Orchard one.
    pub(crate) fn mine_received_ironwood(&mut self, value: u64) -> TxId {
        let fvk = IronwoodFvk(
            self.state
                .test_account_orchard()
                .cloned()
                .expect("orchard fvk"),
        );
        let (_h, res, _nf) = self.state.generate_next_block(
            &fvk,
            AddressType::DefaultExternal,
            ProtoZat::const_from_u64(value),
        );
        self.note_block();
        res.txids()[0]
    }

    /// Mine ONE transaction carrying `values.len()` sapling notes (all paying this account's
    /// external receive address) into a SINGLE new block. Returns that transaction's
    /// (display-order) txid. This is the genuine "one tx, many EXTERNAL-receive notes" shape (a
    /// payer splitting the payment across output notes) — NOT a self-send-with-change, which would
    /// be a SPEND netting to ≈ −fee, not the gross `+sum` this produces. `v_transactions`
    /// aggregates it into ONE row whose `account_balance_delta` is the SUM of the notes, which is
    /// exactly the money-visibility property a multi-note receive must preserve. NOT a way to get
    /// multiple same-height ROWS: the testing harness puts every `FakeCompactOutput` into a single
    /// `CompactTx`, so N outputs ⇒ 1 tx ⇒ 1 row (two distinct txs at one height is unsupported
    /// upstream — see the GAP-1 note in the handoff). NOT yet scanned — call [`Self::scan`].
    pub(crate) fn mine_received_multi(&mut self, values: &[u64]) -> TxId {
        assert!(
            !values.is_empty(),
            "mine_received_multi needs at least one note"
        );
        let dfvk = self
            .state
            .test_account_sapling()
            .cloned()
            .expect("sapling dfvk");
        let outputs: Vec<_> = values
            .iter()
            .map(|v| {
                FakeCompactOutput::new(
                    &dfvk,
                    AddressType::DefaultExternal,
                    ProtoZat::const_from_u64(*v),
                )
            })
            .collect();
        let (_h, res, _nf) = self.state.generate_next_block_multi(&outputs);
        self.note_block();
        // All outputs land in ONE CompactTx ⇒ exactly one txid for the block. Self-check the
        // one-tx invariant the multi-note-aggregation tests rest on (catches any future upstream
        // change that splits outputs across txs).
        debug_assert_eq!(
            res.txids().len(),
            1,
            "a multi-output block must hold exactly one tx"
        );
        res.txids()[0]
    }

    /// Mine `n` empty blocks to bury prior notes deeper (raise their confirmation depth).
    pub(crate) fn mine_empty(&mut self, n: usize) {
        for _ in 0..n {
            self.state.generate_empty_block();
            self.note_block();
        }
    }

    /// Put ONE spendable transparent UTXO of `value` paying the account's external receive
    /// t-address at the CURRENT chain tip — the engine-write path a REAL transparent receive
    /// takes (`put_received_transparent_utxo`; transparent outputs never ride compact blocks,
    /// so [`Self::scan`] cannot produce them — the address-scoped UTXO fetch / enhancement
    /// does). Grafted from `send::tests::funded_transparent` (#317, the owed test).
    /// Call AFTER a [`Self::scan`] so a chain tip exists. Returns the UTXO's fake
    /// (INTERNAL-order) txid, under which `v_transactions` surfaces the receive row.
    pub(crate) fn fund_transparent(&mut self, value: u64) -> TxId {
        use zcash_client_backend::data_api::{Account, WalletRead};
        use zcash_client_backend::wallet::WalletTransparentOutput;
        use zcash_keys::keys::UnifiedAddressRequest;
        use zcash_transparent::bundle::{OutPoint, TxOut};

        let account = self.state.test_account().cloned().expect("account");
        let uaddr = self
            .state
            .wallet()
            .get_last_generated_address_matching(
                account.id(),
                UnifiedAddressRequest::AllAvailableKeys,
            )
            .expect("address read")
            .expect("a generated UA");
        let taddr = *uaddr
            .transparent()
            .expect("the account UA carries a transparent receiver (transparent-inputs ON)");
        let height = self
            .state
            .wallet()
            .chain_height()
            .expect("tip read")
            .expect("a tip (scan first)");
        let txid = [0x40u8; 32];
        let txout = TxOut::new(ProtoZat::const_from_u64(value), taddr.script().into());
        // `None` for 0.24.0's three attribution fields — the engine derives account
        // and key scope from its own `addresses` table (see `transparent.rs`).
        let utxo = WalletTransparentOutput::from_parts(
            OutPoint::new(txid, 0),
            txout,
            Some(height),
            None,
            None,
            None,
        )
        .expect("valid utxo");
        self.state
            .wallet_mut()
            .put_received_transparent_utxo(&utxo)
            .expect("put the received transparent UTXO");
        TxId::from_bytes(txid)
    }

    /// Propose + create + sign a REAL single-step transfer paying `recipient`, over the
    /// production `propose_core`/`create_signed_core` path — `create_proposed_transactions`
    /// STORES the created tx and its sent outputs into the wallet DB, which is exactly the
    /// engine-write path a wallet-created send (e.g. a de-shield to a t-addr) populates
    /// `v_tx_outputs` through (#317, the owed test). Needs spendable funds (mine +
    /// bury ≥ 10 blocks + scan first). Returns the created tx's (INTERNAL-order) txid.
    /// Runs the bundled prover — one real proof per call; use sparingly.
    pub(crate) fn create_send(
        &mut self,
        recipient: zcash_address::ZcashAddress,
        amount: u64,
    ) -> TxId {
        self.create_send_with_claims(recipient, amount).0
    }

    /// [`Self::create_send`], also returning the proposal's NOTE CLAIMS — what an
    /// intent row records at `mark_submitting`, derived by the production
    /// `send::claim_from_proposal` over the same proposal the engine signed — so a
    /// test can enrol a row that owns the send by ATTRIBUTION alone, with no txid
    /// recorded (the create-committed-but-unrecorded window; S8 `obligation` row 3).
    pub(crate) fn create_send_with_claims(
        &mut self,
        recipient: zcash_address::ZcashAddress,
        amount: u64,
    ) -> (TxId, Vec<crate::intent_store::NoteClaim>) {
        use zcash_client_backend::data_api::Account;

        let network = *self.state.network();
        let aid = self.state.test_account().expect("account").id();
        let request = zip321::TransactionRequest::new(vec![zip321::Payment::without_memo(
            recipient,
            ProtoZat::const_from_u64(amount),
        )])
        .expect("valid request");
        let proposal = crate::send::propose_core(self.state.wallet_mut(), &network, aid, request)
            .expect("propose");
        let claims = crate::send::claim_from_proposal(&proposal).expect("claims");
        let usk = self.state.test_account().expect("account").usk().clone();
        let prover = crate::prover::tx_prover();
        let txids = crate::send::create_signed_core(
            self.state.wallet_mut(),
            &network,
            usk,
            &proposal,
            prover,
            prover,
            crate::consensus::SigningPermit::assume_current_for_test(),
        )
        .expect("create+sign");
        assert_eq!(txids.len(), 1, "a single-step transfer creates one tx");
        // `create_signed_core` returns the engine's `zcash_protocol::TxId`s
        // (INTERNAL order) — the same convention the `mine_*` helpers speak.
        (txids[0], claims)
    }

    /// Scan the newly-cached blocks (the decrypt path that fills `v_transactions`) and publish
    /// the chain tip so confirmation depths are well-defined. Heights are contiguous from
    /// `first`, so the unscanned tail starts at `first + scanned`.
    pub(crate) fn scan(&mut self) {
        let first = self.first.expect("nothing mined to scan");
        if self.cached > self.scanned {
            let from = first + u32::try_from(self.scanned).expect("scanned block count fits u32");
            let n = self.cached - self.scanned;
            self.state.scan_cached_blocks(from, n);
            self.scanned = self.cached;
        }
        if let Some(tip) = self.tip {
            self.state
                .wallet_mut()
                .update_chain_tip(tip)
                .expect("update chain tip");
        }
    }

    /// Reorg the chain back to `height`, retaining the block cache so the orphaned range can
    /// be re-scanned. This is the REAL reorg primitive (`WalletWrite::truncate_to_height` on
    /// the SQLite store). librustzcash un-mines the affected txs (`mined_height → NULL`) but
    /// RETAINS the wallet's received notes (`truncate_to_height_internal` keeps received notes
    /// so spendability APIs can exclude un-mined ones) — so a reorged-out RECEIVE does NOT
    /// vanish from `v_transactions`; it re-surfaces as PENDING (with `expired_unmined` NULL —
    /// a received tx has no expiry_height), money intact, until [`Self::rescan_to_tip`] replays
    /// the cache and re-confirms it. The chain tip is rewound to `height`.
    pub(crate) fn reorg_to_height(&mut self, height: u32) {
        let bh = BlockHeight::from_u32(height);
        self.state.truncate_to_height_retaining_cache(bh);
        self.state
            .wallet_mut()
            .update_chain_tip(bh)
            .expect("rewind chain tip");
        let first = u32::from(self.first.expect("nothing mined"));
        // Heights ≤ `height` stay scanned; everything above is now unscanned in the wallet.
        // Checked: rewinding below the first mined block is a test bug, not a silent wrap.
        let kept = height
            .checked_add(1)
            .and_then(|h| h.checked_sub(first))
            .expect("reorg_to_height must not rewind past the first mined block");
        self.scanned = kept as usize;
        self.tip = Some(bh);
    }

    /// Re-scan the retained cache forward to `tip` after a [`Self::reorg_to_height`] — the
    /// orphaned txs re-derive from the cached blocks and re-surface in `v_transactions`.
    pub(crate) fn rescan_to_tip(&mut self, tip: u32) {
        self.tip = Some(BlockHeight::from_u32(tip));
        self.scan();
    }

    /// Drive `set_transaction_status(TxidNotRecognized)` for `txid` — the exact `GetStatus`
    /// "endpoint has no such tx" path the §3.3 enhancement loop takes on an empty reply. Used to
    /// prove a (possibly LYING) not-found reply for a tx we already SCANNED cannot un-detect the
    /// received note (money-visibility). `txid` is the upstream INTERNAL-order txid.
    pub(crate) fn mark_not_recognized(&mut self, txid: TxId) {
        use zcash_client_backend::data_api::{TransactionStatus, WalletWrite};
        self.state
            .wallet_mut()
            .set_transaction_status(txid, TransactionStatus::TxidNotRecognized)
            .expect("set TxidNotRecognized");
    }

    /// Drive `set_transaction_status(NotInMainChain)` for `txid` — the arm the §3.3
    /// enhancement pass writes for a `GetStatus` answer that names no mined height
    /// (`enhance::chain_status` maps a mempool / not-on-chain reply to exactly this; INC-019's
    /// `status_set=5` were five of these). Upstream writes `confirmed_unmined_at_height` under
    /// `mined_height IS NULL` and prunes the retrieval queue; it deletes no row. `txid` is
    /// the upstream INTERNAL-order txid.
    pub(crate) fn mark_not_in_main_chain(&mut self, txid: TxId) {
        use zcash_client_backend::data_api::{TransactionStatus, WalletWrite};
        self.state
            .wallet_mut()
            .set_transaction_status(txid, TransactionStatus::NotInMainChain)
            .expect("set NotInMainChain");
    }

    /// Drive `set_transaction_status(Mined(height))` for `txid` — the OTHER arm the §3.3
    /// enhancement pass can write (INC-019's guard, clause 5). Why the arm is accepted for a
    /// never-mined send, and what it does to the history surface, lives in ONE place:
    /// `enhance::chain_status`'s residual-risk paragraph. `txid` is INTERNAL order; no
    /// `blocks` row at `height` is required (upstream's `block` join is a second,
    /// conditional UPDATE).
    pub(crate) fn mark_mined_at(&mut self, txid: TxId, height: u32) {
        use zcash_client_backend::data_api::{TransactionStatus, WalletWrite};
        self.state
            .wallet_mut()
            .set_transaction_status(
                txid,
                TransactionStatus::Mined(BlockHeight::from_u32(height)),
            )
            .expect("set Mined");
    }

    /// The `expiry_height` upstream's builder stamped on a wallet-created tx (the row's own
    /// column — read, never derived from a constant, so the test that expires it mines to
    /// the height the row names). `txid` is INTERNAL order.
    pub(crate) fn expiry_height_of(&self, txid: TxId) -> u32 {
        self.read_conn()
            .query_row(
                "SELECT expiry_height FROM transactions WHERE txid = ?1",
                rusqlite::params![txid.as_ref()],
                |r| r.get::<_, u32>(0),
            )
            .expect("the created tx has a row with an expiry height")
    }

    /// The highest cached block height (after a `mine_*` this is the just-mined block, so a
    /// reorg/reconfirm test can name the mined height without re-querying the view).
    pub(crate) fn current_tip(&self) -> u32 {
        u32::from(self.tip.expect("nothing mined yet"))
    }

    /// The wallet's pending `transaction_data_requests()` — the REAL upstream enhancement backlog
    /// a compact-scanned receive produces. Lets a test prove the §3.3 enhancement loop's
    /// request-filtering (`select_enhancement_targets`) against genuine upstream output rather
    /// than hand-built requests.
    pub(crate) fn pending_requests(
        &self,
    ) -> Vec<zcash_client_backend::data_api::TransactionDataRequest> {
        use zcash_client_backend::data_api::WalletRead;
        self.state
            .wallet()
            .transaction_data_requests()
            .expect("transaction_data_requests")
    }

    /// The wallet DB file path (for opening additional connections in a concurrency test).
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// A SECOND `rusqlite::Connection` to the SAME wallet DB file — the production aux-db
    /// read pattern (off the engine lock). Mirrors `db.rs::open_existing_keyed_connection`
    /// EXCEPT the `PRAGMA key` (the harness DB is unencrypted — see the module doc). NEVER
    /// lift this keyless shape into production: the real opener applies `PRAGMA key` as its
    /// FIRST statement. Here it just loads the rarray module + sets `busy_timeout` so
    /// rollback-journal contention is absorbed (never a BUSY error).
    pub(crate) fn read_conn(&self) -> Connection {
        let conn = Connection::open(&self.path).expect("open aux read connection");
        rusqlite::vtab::array::load_module(&conn).expect("load rarray module");
        conn.busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
            .expect("busy_timeout");
        conn
    }

    /// The harness's engine store — the `WalletRead` a generic money-path door
    /// (`send::read_tx_expiry`, …) reads through, over the SAME file
    /// [`Self::read_conn`] can tamper with (R12 §4.5 test 6).
    pub(crate) fn wallet(&self) -> &HarnessDb {
        self.state.wallet()
    }
}

// ── R12 §4.5: a REAL held lock on a keyed wallet or cache file ───────────────
//
// The wallet DB runs WAL (`db::ensure_wal`), where a second connection's
// `BEGIN EXCLUSIVE` does NOT block readers (§4.5's warning), and where every
// connection that has READ the file keeps its SHARED lock on it for as long as
// it is open (`pager_unlock` releases nothing in WAL mode). So the fixture is:
//   1. close every connection to the file (or open the door's own connection
//      only AFTER the lock is taken, without reading — see
//      [`keyed_connection_that_has_not_read`]);
//   2. [`hold_the_wallet_file_locked`]: one connection in `locking_mode =
//      EXCLUSIVE` holding a write transaction — its EXCLUSIVE lock on the DB file
//      refuses every other connection's SHARED lock, readers included;
//   3. the door's connection then meets `SQLITE_BUSY` on its first read, after
//      [`R12_BUSY_WAIT`] — the production condition (WAL recovery, the TRUNCATE
//      checkpoint, a racing instance) as SQLite itself raises it.
// The cache DB is rollback-journal and holds no lock between statements, so a
// plain `BEGIN EXCLUSIVE` on a second connection is the real lock there.

/// How long a connection under test waits out a held lock before `SQLITE_BUSY`.
/// The production wait (`SQLITE_BUSY_TIMEOUT_MS`, 5 s) is the CONDITION's length,
/// not what is under test; a door's own connection is set to this by the test
/// that builds it. No production knob is touched.
pub(crate) const R12_BUSY_WAIT: Duration = Duration::from_millis(50);

/// The busy timeout of the aux stores' CONTENTION tests (N threads, a barrier, one
/// `IMMEDIATE` write each), whose claim is the OUTCOME: every write lands, none
/// collides, none deadlocks. How long the slowest writer waits is not under test,
/// and on a loaded CI machine it exceeded the production 5 s (`SQLITE_BUSY_TIMEOUT_MS`).
/// Two minutes keeps the claim deterministic and still fails a real deadlock
/// rather than hanging. In milliseconds, like the production constant it stands in
/// for. Production's own wait stays under test where it is the claim: the opener's
/// contention test in `db.rs` (through `with_aux_busy_retry`, as production writes)
/// and the retry helper's tests.
pub(crate) const CONTENTION_BUSY_WAIT_MS: u64 = 120_000;

/// A keyed connection to an EXISTING wallet file that has not read a byte of it
/// yet (SQLCipher keys lazily; nothing here touches a page), with the rarray
/// module the engine needs and [`R12_BUSY_WAIT`] as its busy timeout — so it
/// holds no lock until a door's first statement runs.
pub(crate) fn keyed_connection_that_has_not_read(
    path: &std::path::Path,
    key: &crate::seal::WalletDbKey,
) -> Connection {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open the wallet file without CREATE");
    conn.execute_batch(key.pragma_key_statement().as_str())
        .expect("PRAGMA key");
    rusqlite::vtab::array::load_module(&conn).expect("load rarray module");
    conn.busy_timeout(R12_BUSY_WAIT).expect("busy_timeout");
    conn
}

/// The same, wrapped as the engine's [`crate::db::WalletConn`] on `network`.
pub(crate) fn wallet_conn_that_has_not_read(
    path: &std::path::Path,
    key: &crate::seal::WalletDbKey,
    network: crate::money::Network,
) -> crate::db::WalletConn {
    WalletDb::from_connection(
        keyed_connection_that_has_not_read(path, key),
        network.consensus(),
        SystemClock,
        OsRng,
    )
}

/// Take a lock on the keyed wallet file that refuses EVERY other connection,
/// readers included, and hold it for as long as the returned connection lives
/// (drop it, or `ROLLBACK`, to release). No other connection may hold the file
/// open with a read behind it when this is called — in WAL each such connection
/// keeps a SHARED lock and the EXCLUSIVE one could never be granted; the call
/// fails loudly then, naming that.
///
/// The fixture proves itself before it returns: a fresh keyed reader is refused
/// with `SQLITE_BUSY` (the raw code, read straight off `rusqlite`), so a test that
/// then sees anything else at a door is measuring the door, not the fixture.
pub(crate) fn hold_the_wallet_file_locked(
    path: &std::path::Path,
    key: &crate::seal::WalletDbKey,
) -> Connection {
    let blocker =
        crate::db::open_existing_keyed_connection(path, key).expect("the blocking connection");
    let mode: String = blocker
        .query_row("PRAGMA locking_mode = EXCLUSIVE", [], |r| r.get(0))
        .expect("set locking_mode");
    assert!(
        mode.eq_ignore_ascii_case("exclusive"),
        "fixture: locking_mode echoed {mode:?}"
    );
    blocker.execute_batch("BEGIN EXCLUSIVE;").expect(
        "fixture: the EXCLUSIVE lock was refused — another connection still holds this file \
         open with a read behind it (in WAL it keeps its SHARED lock until it closes)",
    );
    let probe = keyed_connection_that_has_not_read(path, key);
    match probe.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    }) {
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::DatabaseBusy => {}
        other => panic!(
            "fixture: a held lock must refuse a fresh reader with SQLITE_BUSY, got {other:?}"
        ),
    }
    blocker
}
