//! **SCAN-2 (§4t) — the test half.** A consecutive batch derives its anchor from
//! the blocks the wallet already holds, and reconciles it against the endpoint
//! on a cadence.
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4t — the rows S2-1
//! (the fetch count), S2-3 (a reconcile that disagrees), S2-4 and S2-5 (the two
//! controls), the vector row S2-2 with its capture tool, and the S2-8
//! expectations written on each row. Written BLIND (IT-2a/2b, IT-6): the author
//! of this file has not read the implementer's half and may not edit
//! `sync.rs` outside `pub(crate) mod testing`, `wallet.rs`, `constants.rs` or
//! `tracing_guard.rs`.
//!
//! ## What is driven, and what is observed
//!
//! Every row drives the SHIPPED path — `Wallet::sync_once` over
//! [`FakeChain`], or the production pass body under a real `SyncController`
//! (`Wallet::controller_over`) where a durable stamp is the thing to observe.
//! Three additive seams were built in `sync::testing` for these rows and
//! nothing else: `FakeChain::tree_states`, the receipt of every height the loop
//! ASKED `tree_state` for (the sibling of `ranges`); `FakeChain::tree_state_override`,
//! an endpoint that serves a lying tree state at every height at or above a
//! chosen one; and `FakeChain::tree_state_lies`, the heights at which that
//! override actually FIRED. The third exists because the first cannot stand in
//! for it — a request is recorded whether or not a lie answered it, so S2-3's
//! non-vacuity clause aimed at the request receipt passed with the override arm
//! disabled (measured; see that row's clause 5).
//! Everything else is read through surfaces the wallet already has
//! (`scanned_tip`, `snapshot().last_synced`, the `wallet.sync` batch spans
//! under `tracing_guard`'s capture layer, the progress reporter `sync_once`
//! takes).
//!
//! ## The seam the contract NAMES, and what that does to this file at the base
//!
//! `sync::derive_chain_state(from: &ChainState, blocks: &[CompactBlock]) ->
//! Result<ChainState, WalletError>` is the implementer's to land, and it does
//! not exist at the contract commit. A row that named it directly would stop
//! this module COMPILING (`E0425`), and a module that does not compile takes
//! every other row down with it — none of S2-1, S2-3, S2-4, S2-5 could then be
//! measured at this branch's own head. So the call goes through [`the_seam`]:
//! ONE `use` line, and below it a fallback whose whole body is the absence.
//! The vector row is RED-BY-ABSENCE — red because the production half has not
//! landed, saying so in its panic — and the join re-points that one line at the
//! real function (see [`the_seam`] for the exact edit and the exact call).
//!
//! ## The lie S2-3 tells, and why it is a HASH and not a frontier
//!
//! §4t suggested a non-empty Sapling frontier. At the base every batch
//! fetches, so whatever the override serves at the reconcile height is ALSO
//! consumed by the base as the next batch's `from_state` — the reconcile
//! heights (`end − 1` of a batch) and the base's anchor heights (`from − 1` of
//! the next) are the same set. Upstream's `put_blocks` checks `from_state`'s
//! height and each pool's `tree_size()` against the first block's declared
//! sizes (`zcash_client_backend-0.24.0/src/data_api/ll/wallet.rs:326-346`,
//! read at source) and never reads its block hash, so: a frontier lie makes
//! the BASE fail that check (`NonSequentialBlocks` → `StoreCorrupt` through
//! `map_scan_err`), which is a red for a fixture reason at best; a hash lie is
//! tolerated by the base (the pass ends `Ok`, the row's honest red) and is
//! exactly what a reconcile that compares the derived `ChainState` — its hash,
//! its height, its three frontiers, `ChainState: PartialEq` — must catch. The
//! override therefore serves empty frontiers under a hash no block of the
//! chain carries. A reconcile that compared frontiers only would pass that
//! lie; §4t's own words ("the `ChainState` at the last block (its hash, its
//! frontiers)", P5) say it must not.
//!
//! ## The queue jump S2-4 drives
//!
//! On the fake chain upstream's queue is one contiguous `Historic` range (no
//! shard metadata ⇒ linear scanning, `scanning.rs::update_chain_tip`), so a
//! pass never serves a non-contiguous range on its own. The row builds the
//! geometry from OUTSIDE the wallet: a second SQLCipher-keyed connection onto
//! the same `wallet.db` (the `degraded_pool_proof.rs` observer pattern) splits
//! the queued range in two and raises the upper half to `ChainTip` priority
//! while the pass runs — from the progress reporter, after the first batch,
//! with no db lock held. `suggest_scan_ranges` orders by priority, so the pass
//! jumps UP to the raised range and then back DOWN to the rest: two
//! non-contiguous batches on the shipped loop, no production door. The seam is
//! the direct `scan_queue` write; the test author named it for RR-2 and
//! this round builds it.
//!
//! ## The vector (S2-2)
//!
//! A committed window of REAL mainnet chain data — `TreeState(h)`, the compact
//! blocks `h+1..=h+k`, `TreeState(h+k)` — captured once through the crate's own
//! `LightwalletdClient` by the `#[ignore]`d tool below and written as
//! `tests/fixtures/mainnet-anchor-window.bin` (the format is documented at
//! [`FIXTURE_MAGIC`]). Public chain data only (§5.4): no wallet, no key, no
//! address. The row folds the blocks in batch-sized steps and asserts the
//! derived `ChainState` equals the served one; the heights and the per-pool
//! commitment counts are pinned as constants on the row so the doc and the
//! bytes cannot drift apart.
#![cfg(test)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use prost::Message;
use tracing_subscriber::prelude::*;
use zcash_client_backend::data_api::chain::ChainState;
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_backend::proto::service::{ShieldedProtocol, TreeState};
use zcash_primitives::block::BlockHash;

use crate::config::{EndpointAuth, JitterPolicy, LightServerEndpoint, TorPolicy, WalletConfig};
use crate::constants::{REORG_MAX_BLOCKS, SYNC_BATCH_BLOCKS, SYNC_SUMMARY_REFRESH_BATCHES};
use crate::enhance::{FetchedTransaction, TransactionFetcher};
use crate::error::WalletError;
use crate::keychain::testvault::TestVault;
use crate::keychain::{KeychainPort, VaultTier};
use crate::money::{BlockHeight, Network};
use crate::net::grpc::{BlockStream, GrpcError, LightwalletdClient, SubtreeRootStream};
use crate::provision::{ChainOracle, ServerIdentity};
use crate::seal::WalletDbKey;
use crate::seed::{SeedPersistence, SeedSource};
use crate::state::{StallReason, SyncStatus};
use crate::sync::testing::{FakeChain, Fork, ForkAt, chain_tree_state};
use crate::sync::{self, CancelToken, ScanClient, SubtreeRootSource, TransparentUtxoSource};
use crate::tracing_guard::{
    CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
};
use crate::transparent::TransparentUtxoRecord;
use crate::wallet::Wallet;

// ── The seam §4t names, and the one line the join edits ──────────────────────

/// **The seam, and how it is re-pointed.** §4t names
/// `sync::derive_chain_state(from: &ChainState, blocks: &[CompactBlock]) ->
/// Result<ChainState, WalletError>`. The vector row calls it as
/// `the_seam::derive_chain_state(&state, chunk)` — `state: ChainState`,
/// `chunk: &[CompactBlock]` of length `SYNC_BATCH_BLOCKS` — and this module is
/// the ONE place that says where those two words resolve to:
///
/// ```text
/// base (was):   pub(super) use super::absent_at_base::derive_chain_state;
/// the join:     pub(super) use crate::sync::derive_chain_state;
/// ```
///
/// **RE-POINTED AT THE JOIN**, and the implementer's landed signature is
/// the one §4t named, unchanged — so this was the one line the join edited and
/// `mod absent_at_base` (whose every call panicked naming the missing function,
/// which is what made the vector row RED-BY-ABSENCE rather than red for a
/// fixture reason) was deleted with it.
mod the_seam {
    pub(super) use crate::sync::derive_chain_state;
}

// ── The wallet harness ───────────────────────────────────────────────────────

/// The anchor every testnet fixture wallet here is born on: account 0 imported
/// at `chain_tree_state(ANCHOR)`, so the first scanned block is `ANCHOR + 1`
/// and chains from `block_id(ANCHOR)` (the `wallet.rs` fixture's `FIX_BASE − 1`).
const ANCHOR: u64 = 279_999;

/// One batch, as a height delta.
const BATCH: u64 = SYNC_BATCH_BLOCKS as u64;

/// No-op progress reporter.
const NOOP: &(dyn Fn(sync::ScanProgress) + Sync) = &|_: sync::ScanProgress| {};

fn test_vault() -> Arc<dyn KeychainPort> {
    Arc::new(TestVault::new(VaultTier::Tee))
}

fn cfg(db_dir: &Path) -> WalletConfig {
    WalletConfig {
        db_dir: db_dir.to_path_buf(),
        network: Network::Test,
        // Never dialed: every row's endpoint is the fake.
        endpoint: LightServerEndpoint::new("https://zec.rocks:443").expect("endpoint"),
        endpoint_auth: None,
        tor: TorPolicy::Off,
        seed_persistence: SeedPersistence::SealedKeychain,
        birthday: None,
        broadcast_jitter: JitterPolicy::None,
        machine_memo_prefixes: Vec::new(),
        sync_servers: Vec::new(),
    }
}

fn raw_seed() -> SeedSource {
    SeedSource::raw_bytes(vec![0x37; 32]).expect("valid seed")
}

/// Import account 0 at birthday `anchor + 1` over the empty-frontier anchor the
/// fake chain links from.
async fn import_at(w: &Wallet, anchor: u64) {
    let birthday = crate::account::birthday_from_treestate(chain_tree_state(anchor))
        .expect("the empty-frontier anchor decodes");
    w.import_account(birthday).await.expect("import account");
}

/// A testnet wallet with account 0 born at `anchor + 1`.
async fn wallet_anchored_at(dir: &Path, vault: &Arc<dyn KeychainPort>, anchor: u64) -> Wallet {
    let w = Wallet::create_with_vault(cfg(dir), raw_seed(), Arc::clone(vault))
        .await
        .expect("create");
    import_at(&w, anchor).await;
    w
}

/// [`wallet_anchored_at`] that also captures the wallet's SQLCipher key, so a
/// row can open a SECOND keyed connection onto the same `wallet.db` while the
/// handle stays open (the `degraded_pool_proof.rs` observer pattern, copied:
/// the key is reachable only through `store::open`, which needs the
/// single-writer lock the live handle holds — hence close, capture, re-open).
async fn wallet_anchored_at_with_key(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    anchor: u64,
) -> (Wallet, WalletDbKey) {
    let w = Wallet::create_with_vault(cfg(dir), raw_seed(), Arc::clone(vault))
        .await
        .expect("create");
    w.close().await.expect("close");
    let key = {
        let lock = crate::lifecycle::WalletLock::acquire(dir).expect("acquire the wallet lock");
        let opened = crate::store::open(&lock, &**vault, Network::Test).expect("store open");
        let crate::store::OpenWallet {
            db_key, db, aux_db, ..
        } = opened;
        drop(db);
        drop(aux_db);
        db_key
    };
    let w = Wallet::open_with_vault(cfg(dir), Arc::clone(vault))
        .await
        .expect("re-open after capturing the db key");
    import_at(&w, anchor).await;
    (w, key)
}

/// A thread-local capture of every `zec_wallet_core` span and event until the
/// guard drops (the `tracing_guard` harness).
fn capture() -> (CapturedEvents, tracing::subscriber::DefaultGuard) {
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
    let guard = tracing::subscriber::set_default(subscriber);
    (sink, guard)
}

/// The `wallet.sync` records of one pass, split into batches at each span
/// creation record (the one carrying `from`), in capture order — the shape
/// `wallet::tests::batches_of` reads (copied: proof doubles are private to
/// their files, the join hazard).
fn batches_of(records: &[Vec<(String, String)>]) -> Vec<Vec<(String, String)>> {
    let mut out: Vec<Vec<(String, String)>> = Vec::new();
    for r in records {
        if r.iter().any(|(n, _)| n == "from") {
            out.push(Vec::new());
        }
        if let Some(cur) = out.last_mut() {
            cur.extend(r.iter().cloned());
        }
    }
    out
}

fn field_of<'b>(batch: &'b [(String, String)], name: &str) -> Option<&'b str> {
    batch
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

fn outcome_of(batch: &[(String, String)]) -> &str {
    field_of(batch, "outcome").unwrap_or("<none>")
}

fn height_field(batch: &[(String, String)], name: &str) -> Option<u64> {
    field_of(batch, name).and_then(|v| v.parse().ok())
}

/// A short, printable rendering of the captured batches: `outcome@from..to`.
fn batch_shape(batches: &[Vec<(String, String)>]) -> Vec<String> {
    batches
        .iter()
        .map(|b| {
            format!(
                "{}@{}..{}",
                outcome_of(b),
                height_field(b, "from").map_or_else(|| "?".to_owned(), |f| f.to_string()),
                height_field(b, "to").map_or_else(|| "?".to_owned(), |t| t.to_string()),
            )
        })
        .collect()
}

/// The fake's `tree_state` receipt, copied out.
fn fetches(receipt: &Arc<Mutex<Vec<u64>>>) -> Vec<u64> {
    receipt.lock().expect("tree_states").clone()
}

/// The batch spans of one captured pass, §5.4-checked.
fn captured_batches(sink: &CapturedEvents) -> Vec<Vec<(String, String)>> {
    assert_5_4_clean(&sink.fields());
    batches_of(&sink.records_of("wallet.sync"))
}

// ── The controller double ────────────────────────────────────────────────────

/// A full `PassClient` over a [`FakeChain`]: the scan, the roots and the
/// provision handshake are the fake's own (it is an honest testnet endpoint
/// for those); the transparent poll and the enhancement drain abstain. The
/// shape `wallet::tests::ScriptedPassClient` has, without its root scripts —
/// the S2-3 row needs the shipped pass body under a real controller so the
/// stamp `emit_synced` writes is the thing observed, and nothing more.
struct ProofPassClient(FakeChain);

#[async_trait]
impl ScanClient for ProofPassClient {
    async fn block_range(
        &mut self,
        start: u64,
        end_inclusive: u64,
    ) -> Result<BlockStream, GrpcError> {
        self.0.block_range(start, end_inclusive).await
    }
    async fn tree_state(&mut self, height: u64) -> Result<TreeState, GrpcError> {
        self.0.tree_state(height).await
    }
    async fn latest_block_height(&mut self) -> Result<u64, GrpcError> {
        self.0.latest_block_height().await
    }
}

#[async_trait]
impl SubtreeRootSource for ProofPassClient {
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        self.0.subtree_roots(protocol, start_index).await
    }
}

#[async_trait]
impl ChainOracle for ProofPassClient {
    async fn tip_height(&mut self) -> Result<u64, GrpcError> {
        self.0.tip_height().await
    }
    async fn server_identity(&mut self) -> Result<ServerIdentity, GrpcError> {
        self.0.server_identity().await
    }
}

#[async_trait]
impl TransparentUtxoSource for ProofPassClient {
    async fn address_utxos(
        &mut self,
        _addresses: Vec<String>,
        _start_height: u64,
    ) -> Result<Vec<TransparentUtxoRecord>, GrpcError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl TransactionFetcher for ProofPassClient {
    async fn fetch_transaction(
        &mut self,
        _txid: zcash_protocol::TxId,
    ) -> Result<Option<FetchedTransaction>, GrpcError> {
        Ok(None)
    }
}

// ── S2-1 ─────────────────────────────────────────────────────────────────────

/// **§4t S2-1 — DEFECT row, RED at the base.** A consecutive batch does not
/// fetch the tree state.
///
/// One pass of `BATCHES` (= 5) consecutive batches over the honest fake chain,
/// through `sync_once`. Asserted, from the fake's `tree_states` receipt:
/// 1. the FIRST fetch is the first batch's anchor, `ANCHOR` (= `from − 1`) —
///    the clause that keeps the row non-vacuous: a receipt nobody writes to is
///    empty, and an empty receipt passes clause 2 for the wrong reason
///    (self-mutant measured at authoring time: the receipt's `push` removed →
///    this clause reds with "the first batch must fetch its anchor; got None");
/// 2. the pass made AT MOST `1 + BATCHES / TREE_STATE_RECONCILE_BATCHES` fetches:
///    the first batch's anchor plus one per cadence for the reconcile. The test
///    half wrote this as `1 + BATCHES / 2` because the cadence was a constant it
///    could not see (§4t "not decided" 1, the implementer's to choose); **the
///    join tightened it to the shipped constant**, read here rather than copied.
///    At 5 batches against a cadence of 20 the pass reconciles not at all, so
///    the ceiling is exactly ONE — the strongest form of this clause;
/// 3. every fetch is at a batch boundary (`ANCHOR + n·SYNC_BATCH_BLOCKS`, at or
///    below the tip) — a reconcile fetches at a batch's `end − 1`, an anchor at
///    a batch's `from − 1`, and nothing else is a fetch this loop makes.
///
/// At the base the loop calls `sync::fetch_chain_state` for every batch, so
/// clause 2 reds with `BATCHES` fetches. Killer mutants (S2-8): the fetch never
/// skipped (the base) → clause 2; "derive always" leaves this row green (its
/// killers are S2-4 and S2-5).
#[tokio::test]
async fn a_consecutive_batch_does_not_fetch_the_tree_state() {
    const BATCHES: u64 = 5;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, ANCHOR).await;
    // [ANCHOR + 1, tip + 1) is exactly BATCHES full batches.
    let tip = ANCHOR + BATCHES * BATCH;
    let mut chain = FakeChain::to_tip(tip);
    let receipt = Arc::clone(&chain.tree_states);
    let cancel = CancelToken::new();
    let pass = w
        .sync_once(&mut chain, &cancel, NOOP)
        .await
        .expect("an honest chain syncs");
    assert_eq!(
        pass.batches,
        u32::try_from(BATCHES).expect("a small count"),
        "the fixture's own precondition: {BATCHES} consecutive batches"
    );
    assert_eq!(pass.reorgs, 0, "an honest chain: no rewind");
    assert_eq!(
        w.scanned_tip().await.expect("scanned tip"),
        Some(BlockHeight::new(
            u32::try_from(tip).expect("a fixture height")
        )),
        "the pass reached the tip"
    );

    let fetched = fetches(&receipt);
    assert_eq!(
        fetched.first().copied(),
        Some(ANCHOR),
        "§4t S2-1: the first batch must fetch its anchor (the block below it), and the \
         receipt must say so; got {fetched:?}"
    );
    for h in &fetched {
        assert!(
            (h - ANCHOR).is_multiple_of(BATCH) && *h <= tip,
            "every fetch this loop makes is at a batch boundary — an anchor at `from − 1` \
             or a reconcile at `end − 1`; got {h} in {fetched:?}"
        );
    }
    // TIGHTENED AT THE JOIN, as clause 2's doc said it would be: the shipped
    // cadence is `TREE_STATE_RECONCILE_BATCHES`, read here rather than copied,
    // so a change to it moves this row with the code. At 5 batches against a
    // cadence of 20 the pass reconciles NOT AT ALL, and the ceiling is exactly
    // one fetch — the first batch's anchor and nothing else.
    let cadence = u64::from(crate::constants::TREE_STATE_RECONCILE_BATCHES);
    let ceiling = 1 + BATCHES / cadence;
    assert!(
        u64::try_from(fetched.len()).expect("a small count") <= ceiling,
        "§4t S2-1: a consecutive batch must not fetch the tree state — {BATCHES} consecutive \
         batches may make ONE fetch (the first batch's anchor) plus one per {cadence} batches \
         for the reconcile, so at most {ceiling} here; this pass fetched {} times, at {fetched:?}",
        fetched.len()
    );
    w.close().await.expect("close");
}

// ── S2-5 ─────────────────────────────────────────────────────────────────────

/// **§4t S2-5 — CONTROL, green at the base.** The first batch of every pass
/// fetches: the derived anchor never survives a pass.
///
/// Pass 1 to `T1` (three batches), then pass 2 against a chain two batches
/// higher. Asserted: pass 1's first fetch is `ANCHOR`; pass 2's first fetch is
/// `T1` — the block below ITS first batch — so a new pass starts from the
/// endpoint's word, never from something carried across `sync_once`'s return;
/// and the two receipts together hold at least two fetches.
///
/// Killer mutants (S2-8): an anchor persisted across passes (§4t "Out": one
/// fetch per pass is not the 22 %) → pass 2's receipt is empty; "derive always"
/// with the carried state seeded from the DB → the same.
#[tokio::test]
async fn the_first_batch_of_every_pass_fetches() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, ANCHOR).await;
    let cancel = CancelToken::new();

    let t1 = ANCHOR + 3 * BATCH;
    let mut first = FakeChain::to_tip(t1);
    let receipt1 = Arc::clone(&first.tree_states);
    let pass1 = w
        .sync_once(&mut first, &cancel, NOOP)
        .await
        .expect("pass 1");
    assert_eq!(
        pass1.batches, 3,
        "the fixture's own precondition: three batches"
    );
    let fetched1 = fetches(&receipt1);
    assert_eq!(
        fetched1.first().copied(),
        Some(ANCHOR),
        "pass 1's first batch fetches its anchor; got {fetched1:?}"
    );

    let mut second = FakeChain::to_tip(t1 + 2 * BATCH);
    let receipt2 = Arc::clone(&second.tree_states);
    let pass2 = w
        .sync_once(&mut second, &cancel, NOOP)
        .await
        .expect("pass 2");
    assert_eq!(
        pass2.batches, 2,
        "the fixture's own precondition: two more batches"
    );
    let fetched2 = fetches(&receipt2);
    assert_eq!(
        fetched2.first().copied(),
        Some(t1),
        "§4t S2-5: the first batch of a NEW pass fetches the tree state below it (the \
         derived anchor never survives a pass); pass 2's receipt: {fetched2:?}"
    );
    assert!(
        fetched1.len() + fetched2.len() >= 2,
        "two passes ⇒ at least two fetches; got {fetched1:?} then {fetched2:?}"
    );
    w.close().await.expect("close");
}

// ── S2-4 ─────────────────────────────────────────────────────────────────────

/// The tip pass 1 of the reorg geometry scans to: 301 blocks above the anchor —
/// four batches, a checkpoint at each end (`wallet::tests::REORG_PASS1_TIP`).
const REORG_PASS1_TIP: u64 = ANCHOR + 301;

/// One progress sample, with the fetch receipt's LENGTH at that instant — the
/// ordering witness: a fetch recorded at or past that index happened after the
/// sample. Deadlock-free by the loop's own structure: `sync_once` reports only
/// from the async task and only outside its db-lock sections.
#[derive(Debug, Clone, Copy)]
struct FetchSample {
    frontier: u64,
    rewound: bool,
    fetches_before: usize,
}

/// A reporter that records every sample beside the receipt's length, and runs
/// `on_sample` on each (the queue-jump half's rewrite hook; `NOOP`-shaped for
/// the rewind half).
fn sampling_reporter<'a>(
    receipt: &'a Arc<Mutex<Vec<u64>>>,
    samples: &'a Mutex<Vec<FetchSample>>,
    on_sample: &'a (dyn Fn(&sync::ScanProgress) + Sync),
) -> impl Fn(sync::ScanProgress) + Sync + 'a {
    move |p: sync::ScanProgress| {
        let fetches_before = receipt.lock().expect("tree_states").len();
        samples
            .lock()
            .expect("samples mutex poisoned")
            .push(FetchSample {
                frontier: u64::from(p.frontier.value()),
                rewound: p.rewound,
                fetches_before,
            });
        on_sample(&p);
    }
}

/// **§4t S2-4, first half — CONTROL, green at the base.** After a `Fork`
/// rewind the next batch fetches the tree state again, for the height the
/// rewind LANDED on.
///
/// R-1's geometry (`BelowFirstRangeEnd(4)` revealed after the first range, tip
/// `REORG_PASS1_TIP + 200`): pass 2's second batch no longer chains from what
/// the wallet holds, `scan_batch` rewinds, and — since REQ-1 — re-queues and
/// re-scans across the fork inside the pass. The Rewind arm reports the
/// rewound position (`frontier = landed`, `rewound = true`) BEFORE the loop
/// asks for its next batch, so the receipt's length at that sample is the
/// index of the first fetch after the rewind. Asserted: the pass completes
/// with one rewind; the batch after the rewound one starts at `landed + 1`;
/// and the fetch at that index is `landed` — the anchor was re-fetched, not
/// carried across the rewind.
async fn a_rewind_fetches_the_tree_state_again() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_anchored_at(dir.path(), &vault, ANCHOR).await;
    let cancel = CancelToken::new();
    let mut original = FakeChain::to_tip(REORG_PASS1_TIP);
    let pass1 = w
        .sync_once(&mut original, &cancel, NOOP)
        .await
        .expect("pass 1");
    assert_eq!(
        pass1.batches, 4,
        "the fixture's own precondition (`reorg_passes`): four batches, a checkpoint at each end"
    );

    let mut forked = FakeChain::to_tip(REORG_PASS1_TIP + 200);
    forked.fork = Some(Fork {
        at: ForkAt::BelowFirstRangeEnd(4),
        after_ranges: 1,
    });
    let receipt = Arc::clone(&forked.tree_states);
    let samples: Mutex<Vec<FetchSample>> = Mutex::new(Vec::new());
    let quiet = |_: &sync::ScanProgress| {};
    let record = sampling_reporter(&receipt, &samples, &quiet);
    let (sink, guard) = capture();
    let pass = w.sync_once(&mut forked, &cancel, &record).await;
    drop(guard);
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let fetched = fetches(&receipt);
    let samples = samples.lock().expect("samples mutex poisoned").clone();

    let pass = match pass {
        Ok(p) => p,
        Err(e) => panic!(
            "the reorg pass completes on this single-fork geometry (R-1 holds at this base); \
             got Err({e:?}); batches {shape:?}"
        ),
    };
    assert_eq!(
        pass.reorgs, 1,
        "one revealed fork ⇒ one rewind; batches {shape:?}"
    );
    let rewind_at = batches
        .iter()
        .position(|b| outcome_of(b) == "rewind")
        .unwrap_or_else(|| panic!("the geometry must produce a rewound batch; batches {shape:?}"));
    let after = batches.get(rewind_at + 1).unwrap_or_else(|| {
        panic!("the re-queue gives the pass a batch after the rewind (R-1); batches {shape:?}")
    });
    let after_from = height_field(after, "from").expect("a batch span carries `from`");
    let rewound = samples
        .iter()
        .find(|s| s.rewound)
        .copied()
        .expect("the Rewind arm reports the rewound position with `rewound = true`");
    assert_eq!(
        after_from,
        rewound.frontier + 1,
        "the batch after the rewind starts one above the landed height; samples {samples:?}"
    );
    assert_eq!(
        fetched.get(rewound.fetches_before).copied(),
        Some(rewound.frontier),
        "§4t S2-4 (rewind): the first tree state fetched AFTER the rewind is the landed \
         height {} — the anchor is re-fetched, never carried across a rewind. Receipt \
         {fetched:?} (length {} at the rewound sample); batches {shape:?}",
        rewound.frontier,
        rewound.fetches_before
    );
    w.close().await.expect("close");
}

/// `scan_queue` priority codes (`zcash_client_sqlite-0.22.0/src/wallet/scanning.rs::priority_code`,
/// read at source): the two the queue-jump seam writes.
const PRIORITY_HISTORIC: i64 = 20;
const PRIORITY_CHAIN_TIP: i64 = 50;

/// **§4t S2-4, second half — CONTROL, green at the base.** A pass whose queue
/// serves a NON-CONTIGUOUS range fetches the tree state for it.
///
/// The geometry (module doc, "The queue jump"): pass 1 scans the wallet to
/// `S`; pass 2's endpoint is three batches higher. From the progress reporter
/// — the first sample past the first batch's end, no db lock held — a second
/// keyed connection rewrites the queued `Historic` range `[.., T + 1)` as its
/// lower two batches at `Historic` and its top batch at `ChainTip`. The loop's
/// next `next_batch` then serves the top batch (priority first), and after it
/// the lower one: a jump UP then a jump DOWN, each a batch whose `from` is not
/// the previous batch's `end`. Asserted: the pass completes at `T`; the batch
/// shape is exactly `[S+1.., S+1+2B.., S+1+B..]` (the seam held — anything
/// else is the seam failing, named, never a green); and for each jump the
/// receipt holds `from − 1` at or past the receipt's length at the previous
/// batch's last sample — the fetch was made for the jump, after the batch
/// before it.
///
/// Killer mutants (S2-8): "derive always" / the consecutive test inverted (a
/// non-contiguous range treated as consecutive) → the jump batch derives from
/// an anchor it does not follow — the receipt lacks `from − 1` here, and
/// upstream's `NonSequentialBlocks` check turns the pass `StoreCorrupt`.
async fn a_queue_jump_fetches_the_tree_state_again() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_anchored_at_with_key(dir.path(), &vault, ANCHOR).await;
    let cancel = CancelToken::new();
    let s = ANCHOR + 3 * BATCH;
    let mut first = FakeChain::to_tip(s);
    let pass1 = w
        .sync_once(&mut first, &cancel, NOOP)
        .await
        .expect("pass 1");
    assert_eq!(
        pass1.batches, 3,
        "the fixture's own precondition: three batches to S"
    );
    assert_eq!(
        w.scanned_tip().await.expect("scanned tip"),
        Some(BlockHeight::new(
            u32::try_from(s).expect("a fixture height")
        )),
        "the fixture's own precondition: the wallet holds S"
    );

    let t = s + 3 * BATCH;
    let split_at = s + 1 + 2 * BATCH; // the top batch, raised to ChainTip
    // Behind a `Mutex`: a `rusqlite::Connection` is `Send` but not `Sync`, and the
    // reporter that uses it must be.
    let observer = Mutex::new(
        crate::db::open_existing_keyed_connection(&dir.path().join("wallet.db"), &key)
            .expect("a second keyed connection onto the wallet db"),
    );
    let rewritten = Mutex::new(false);
    let rewrite = |p: &sync::ScanProgress| {
        let mut done = rewritten.lock().expect("rewritten flag");
        if *done || u64::from(p.frontier.value()) < s + 1 + BATCH {
            return;
        }
        *done = true;
        // The queued range ending at T + 1 with Historic priority — whatever its
        // start is at this instant (the first batch may or may not be marked
        // scanned yet) — split at `split_at`.
        let observer = observer.lock().expect("observer connection");
        let tx = observer.unchecked_transaction().expect("begin");
        let start: i64 = tx
            .query_row(
                "SELECT block_range_start FROM scan_queue WHERE block_range_end = ?1 AND priority = ?2",
                rusqlite::params![i64::try_from(t + 1).expect("a height"), PRIORITY_HISTORIC],
                |r| r.get(0),
            )
            .expect("the seam's precondition: ONE Historic range queued up to T + 1");
        tx.execute(
            "DELETE FROM scan_queue WHERE block_range_end = ?1",
            rusqlite::params![i64::try_from(t + 1).expect("a height")],
        )
        .expect("delete the queued range");
        tx.execute(
            "INSERT INTO scan_queue (block_range_start, block_range_end, priority) VALUES (?1, ?2, ?3)",
            rusqlite::params![start, i64::try_from(split_at).expect("a height"), PRIORITY_HISTORIC],
        )
        .expect("re-queue the lower part");
        tx.execute(
            "INSERT INTO scan_queue (block_range_start, block_range_end, priority) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                i64::try_from(split_at).expect("a height"),
                i64::try_from(t + 1).expect("a height"),
                PRIORITY_CHAIN_TIP
            ],
        )
        .expect("queue the top batch at ChainTip");
        tx.commit().expect("commit the split");
    };
    let mut chain = FakeChain::to_tip(t);
    let receipt = Arc::clone(&chain.tree_states);
    let samples: Mutex<Vec<FetchSample>> = Mutex::new(Vec::new());
    let record = sampling_reporter(&receipt, &samples, &rewrite);
    let (sink, guard) = capture();
    let pass = w.sync_once(&mut chain, &cancel, &record).await;
    drop(guard);
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let fetched = fetches(&receipt);
    let samples = samples.lock().expect("samples mutex poisoned").clone();
    assert!(
        *rewritten.lock().expect("rewritten flag"),
        "the seam's precondition: the queue was rewritten during the pass; batches {shape:?}"
    );

    let pass = match pass {
        Ok(p) => p,
        Err(e) => panic!(
            "a pass over a split queue completes (every range is the honest chain); got \
             Err({e:?}); batches {shape:?}; fetched {fetched:?}"
        ),
    };
    assert_eq!(pass.reorgs, 0, "no fork here; batches {shape:?}");
    assert_eq!(
        w.scanned_tip().await.expect("scanned tip"),
        Some(BlockHeight::new(
            u32::try_from(t).expect("a fixture height")
        )),
        "the pass scanned every queued range; batches {shape:?}"
    );
    let froms: Vec<u64> = batches
        .iter()
        .map(|b| height_field(b, "from").expect("a batch span carries `from`"))
        .collect();
    assert_eq!(
        froms,
        vec![s + 1, split_at, s + 1 + BATCH],
        "the seam held: the top batch (ChainTip) is served before the lower one — a jump up, \
         then a jump down; batches {shape:?}"
    );
    let mut jumps = 0;
    for i in 1..batches.len() {
        let prev_end = height_field(&batches[i - 1], "to").expect("a batch span carries `to`");
        let from = froms[i];
        if from == prev_end {
            continue;
        }
        jumps += 1;
        // `ScanProgress::sample` clamps the frontier to the tip, so the top
        // batch's committed frontier `T + 1` is reported as `T` — which the
        // pass-exit sample reports too, after every fetch; the FIRST sample at
        // that frontier is the previous batch reaching its end, and a fetch at
        // or past the receipt length it saw was made after it.
        let committed = prev_end.min(t);
        let after = samples
            .iter()
            .position(|smp| smp.frontier == committed)
            .map(|idx| samples[idx].fetches_before)
            .unwrap_or_else(|| {
                panic!(
                    "the previous batch reported its committed frontier {committed}; samples \
                     {samples:?}"
                )
            });
        assert!(
            fetched[after..].contains(&(from - 1)),
            "§4t S2-4 (queue jump): batch {i} starts at {from}, not at the previous batch's end \
             {prev_end}, so the pass must fetch the tree state at {} for it — after the previous \
             batch (receipt index ≥ {after}). Receipt {fetched:?}; batches {shape:?}",
            from - 1
        );
    }
    assert_eq!(
        jumps, 2,
        "the geometry's non-vacuity: two jumps; batches {shape:?}"
    );
    w.close().await.expect("close");
}

/// **§4t S2-4 — CONTROL, green at the base.** Both halves, each its own
/// geometry and its own receipt: a `Fork` rewind fetches again for the landed
/// height, and a queue that serves a non-contiguous range fetches for it.
#[tokio::test]
async fn a_rewind_or_a_queue_jump_fetches_the_tree_state_again() {
    a_rewind_fetches_the_tree_state_again().await;
    a_queue_jump_fetches_the_tree_state_again().await;
}

// ── S2-3 ─────────────────────────────────────────────────────────────────────

/// The heights the stamp geometry runs at: above the signed bundle's newest
/// testnet row, so the pass grades the endpoint at-or-above and `emit_synced`'s
/// stamp gate is OPEN (`wallet::tests::h_above_the_row`, recomputed here).
fn h_above_the_row() -> u64 {
    u64::from(crate::root_bind::newest_bundled_height(Network::Test) + 10 * REORG_MAX_BLOCKS)
}

/// A tree state no block of the fake chain agrees with: empty frontiers (so a
/// build that still fetches per batch scans on unbothered — upstream never
/// reads an anchor's hash) under a hash the chain does not carry. Served at
/// every requested height at or above the override's, with `height` rewritten
/// by the fake.
fn a_tree_state_with_a_hash_no_block_carries() -> TreeState {
    let mut lie = chain_tree_state(0);
    lie.hash = hex::encode([0xA5u8; 32]);
    lie
}

/// **§4t S2-3 — DEFECT row, RED at the base.** A reconcile that disagrees with
/// the endpoint ends the pass as the endpoint's fault.
///
/// Through the shipped controller (`controller_over`, the stamp is the durable
/// claim): pass 1, honest, drives the wallet to `H` above the testnet row and
/// stamps `H` (the gate is open — asserted, so the stamp clause below cannot
/// pass vacuously). Pass 2's endpoint reports `H + (SYNC_SUMMARY_REFRESH_BATCHES
/// + 1) × SYNC_BATCH_BLOCKS` and, from the first batch's `end − 1` on, answers
/// `GetTreeState` with the lying state (module doc) — at EVERY height a build
/// could reconcile at, whatever its cadence, and the pass is long enough that
/// any cadence the contract allows (≤ `SYNC_SUMMARY_REFRESH_BATCHES`) reconciles
/// at least once. Asserted:
/// 1. the pass is `Err(Sync { EndpointMisbehaving })` — never `StoreCorrupt`,
///    never a panic, never `Ok`;
/// 2. the surface says so: `Stalled { EndpointMisbehaving }`;
/// 3. nothing stamped: `last_synced` after the pass equals the stamp pass 1
///    wrote;
/// 4. the batch that caught it carries `outcome = "error"` and is the pass's
///    last;
/// 5. the fixture's own non-vacuity: the lie was actually SERVED —
///    `FakeChain::tree_state_lies` is non-empty and every entry is at or above
///    the override's height. Read off what the fake served, never off what the
///    loop asked for: aimed at the request receipt this clause passed with the
///    override arm disabled (self-mutant B, measured), which is to say it
///    discriminated nothing.
///
/// At the base there is no reconcile: every batch fetches, the lie is taken as
/// each batch's anchor (its empty frontiers pass upstream's sequential check,
/// its hash is never read), the pass ends `Ok` at the reported tip and the
/// controller stamps it — clause 1 reds on the `Ok` arm. Killer mutants
/// (S2-8): the reconcile comparison removed → the same `Ok`; the mismatch
/// mapped to `StoreCorrupt` → the `Err(other)` arm; the stamp written on the
/// error path → clause 3.
#[tokio::test]
async fn a_reconcile_that_disagrees_with_the_endpoint_ends_the_pass_as_the_endpoints_fault() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_anchored_at(dir.path(), &vault, h - 250).await;

    let ctl = w.controller_over(ProofPassClient(FakeChain::to_tip(h)));
    let first = ctl.once().await.expect("pass 1");
    assert!(
        first.batches > 0,
        "precondition: a REAL scan drives the wallet to H"
    );
    assert_eq!(
        w.scanned_tip().await.expect("scanned"),
        Some(BlockHeight::new(
            u32::try_from(h).expect("a fixture height")
        )),
        "precondition: after pass 1 the wallet HOLDS H"
    );
    let stamp_before = w.snapshot().await.expect("snap").last_synced;
    assert_eq!(
        stamp_before.map(|s| s.height.value()),
        Some(u32::try_from(h).expect("a fixture height")),
        "precondition: the server grades at or above the row, so the stamp gate is OPEN and \
         pass 1 stamped H — which is what makes 'nothing stamped' below a claim, not a default"
    );
    drop(ctl);

    let override_from = h + BATCH; // pass 2's first batch's `end − 1`
    let tip = h + u64::from(SYNC_SUMMARY_REFRESH_BATCHES + 1) * BATCH;
    let mut chain = FakeChain::to_tip(tip);
    chain.tree_state_override = Some((override_from, a_tree_state_with_a_hash_no_block_carries()));
    let receipt = Arc::clone(&chain.tree_states);
    let lies = Arc::clone(&chain.tree_state_lies);
    let (sink, guard) = capture();
    let ctl2 = w.controller_over(ProofPassClient(chain));
    let rx = ctl2.subscribe();
    let pass = ctl2.once().await;
    let status = rx.borrow().clone();
    drop(guard);
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let fetched = fetches(&receipt);
    let lied_at = fetches(&lies);
    let stamp_after = w.snapshot().await.expect("snap").last_synced;
    let scanned_after = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    w.close().await.expect("close");

    // The fixture's own non-vacuity, read off what the fake SERVED. The request
    // receipt cannot carry this clause: it fills up at every height whether or
    // not a lie answered, so aimed at `fetched` the assertion passed with the
    // override arm disabled (measured) — it discriminated nothing.
    assert!(
        !lied_at.is_empty(),
        "the fixture's own non-vacuity: the endpoint must actually have SERVED the disagreeing \
         tree state during this pass — the override at {override_from} never fired. The pass \
         asked for {} tree states, at {fetched:?}",
        fetched.len()
    );
    assert!(
        lied_at.iter().all(|&l| l >= override_from),
        "the override lies only at or above {override_from}; it fired at {lied_at:?}"
    );
    match pass {
        Err(WalletError::Sync {
            stall: StallReason::EndpointMisbehaving,
        }) => {
            assert!(
                matches!(
                    status,
                    SyncStatus::Stalled {
                        reason: StallReason::EndpointMisbehaving
                    }
                ),
                "the surface names the endpoint: got {status:?}"
            );
            assert_eq!(
                stamp_after, stamp_before,
                "nothing stamped: a pass the endpoint's own tree state contradicts makes no \
                 durable claim (the wallet holds {scanned_after:?}); batches {shape:?}"
            );
            let last = batches
                .last()
                .unwrap_or_else(|| panic!("at least one batch ran; receipt {fetched:?}"));
            assert_eq!(
                outcome_of(last),
                "error",
                "the batch whose reconcile disagreed carries `outcome = \"error\"` and is the \
                 pass's last; batches {shape:?}"
            );
        }
        Ok(p) => panic!(
            "§4t S2-3: the endpoint's GetTreeState disagrees with the blocks it serves from \
             {override_from} on, so the pass must end Err(Sync {{ EndpointMisbehaving }}); got \
             Ok({p:?}) — the pass ran to the reported tip on the lie, status {status:?}, the \
             stamp {stamp_before:?} → {stamp_after:?}, the wallet holds {scanned_after:?}; \
             receipt {fetched:?}",
        ),
        Err(other) => panic!(
            "§4t S2-3: a disagreeing endpoint is the ENDPOINT's fault — never StoreCorrupt, \
             never another class; got {other:?} (status {status:?}, batches {shape:?})"
        ),
    }
}

/// **§4t-run review row 2 — the undo.** A reconcile that disagrees truncates the
/// derived streak's work back to the last anchor this pass FETCHED, before it
/// fails.
///
/// The same fixture as S2-3 above — pass 1 honest to `H`, pass 2 lying from
/// `H + BATCH` on — asked a different question: not what the pass RETURNS, but
/// what the wallet HOLDS afterwards. Up to `TREE_STATE_RECONCILE_BATCHES − 1`
/// batches are scanned on an anchor derived from the endpoint's own blocks and
/// `put_blocks` commits each derived frontier into the persisted shard tree; those
/// writes used to survive the `Err`. Altered or extra commitments shift every later
/// note position, so the witnesses are wrong and the user's spends fail to prove —
/// and the next pass fetches a true anchor and scans on from the poisoned height
/// with nothing detecting, repairing or reporting it.
///
/// Asserted:
/// 1. **the non-vacuity, first**: pass 2 ADVANCED at least
///    `TREE_STATE_RECONCILE_BATCHES − 1` batches before the mismatch — so there
///    was un-attested work to undo, and a build that failed on batch 1 could not
///    pass this row by doing nothing;
/// 2. the wallet holds exactly `H` again — the height pass 1 left it at, which is
///    the last anchor pass 2 fetched (its first batch's, below the override). At
///    the base it holds `H + (TREE_STATE_RECONCILE_BATCHES − 1) × BATCH`;
/// 3. the pass still ends `Err(Sync { EndpointMisbehaving })` — the undo is
///    additional to the verdict, not a replacement for it.
///
/// Killer mutants: the `rewind_to_attested_anchor` call deleted → clause 2; the
/// truncate target taken from the batch's own `from` instead of the carried
/// `last_fetched_anchor` → clause 2 (it would land one batch below the mismatch,
/// not at `H`); `last_fetched_anchor` set on the DERIVE arm as well → clause 2.
#[tokio::test]
async fn a_reconcile_mismatch_truncates_back_to_the_last_fetched_anchor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_anchored_at(dir.path(), &vault, h - 250).await;

    let ctl = w.controller_over(ProofPassClient(FakeChain::to_tip(h)));
    let first = ctl.once().await.expect("pass 1");
    assert!(
        first.batches > 0,
        "precondition: a REAL scan drives the wallet to H"
    );
    let held_before = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    assert_eq!(
        held_before,
        Some(u32::try_from(h).expect("a fixture height")),
        "precondition: after pass 1 the wallet HOLDS H"
    );
    drop(ctl);

    let override_from = h + BATCH; // pass 2's first batch's `end − 1`
    let tip = h + u64::from(SYNC_SUMMARY_REFRESH_BATCHES + 1) * BATCH;
    let mut chain = FakeChain::to_tip(tip);
    chain.tree_state_override = Some((override_from, a_tree_state_with_a_hash_no_block_carries()));
    let lies = Arc::clone(&chain.tree_state_lies);
    let (sink, guard) = capture();
    let ctl2 = w.controller_over(ProofPassClient(chain));
    let pass = ctl2.once().await;
    drop(guard);
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let lied_at = fetches(&lies);
    let held_after = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    w.close().await.expect("close");

    assert!(
        !lied_at.is_empty(),
        "the fixture's own non-vacuity: the endpoint must actually have SERVED the disagreeing \
         tree state during this pass; batches {shape:?}"
    );
    // Clause 1 — there WAS un-attested work. Read off the batch spans, not off a
    // count the loop reports: an `Err` pass returns no `SyncPass`.
    let cadence = u64::from(crate::constants::TREE_STATE_RECONCILE_BATCHES);
    let advanced = batches
        .iter()
        .filter(|b| outcome_of(b) == "advance")
        .count() as u64;
    assert!(
        advanced >= cadence - 1,
        "the row is about UNDOING a derived streak, so the pass must first have scanned one: \
         {advanced} batches advanced, expected at least {} (the cadence less the reconciling \
         batch); batches {shape:?}",
        cadence - 1
    );
    // Clause 2 — the undo itself.
    assert_eq!(
        held_after, held_before,
        "the wallet must hold exactly what it held before the pass: every batch above H rode an \
         anchor only the endpoint's own blocks attested, and the reconcile refused the anchor \
         that was supposed to attest them. {advanced} batches advanced on the derived streak; \
         batches {shape:?}"
    );
    // Clause 3 — the verdict is unchanged.
    assert!(
        matches!(
            pass,
            Err(WalletError::Sync {
                stall: StallReason::EndpointMisbehaving
            })
        ),
        "the undo is additional to the endpoint's-fault verdict, never a replacement: got \
         {pass:?}"
    );
}

/// **The review's shared HIGH — both angles, independently.** The undo
/// works on the geometry where it MOST matters: a pass whose first anchor is
/// below every block the wallet holds.
///
/// The row above drives pass 2 of a wallet pass 1 already scanned, so its
/// truncate target IS a `blocks` row and upstream's
/// `select_truncation_height` always finds it. That is the ONE geometry where
/// the truncate cannot refuse — and it is not the geometry that matters.
/// `blocks` rows exist only where a block was SCANNED, so on a fresh or
/// restored wallet the first anchor of a pass sits below every row there is,
/// upstream answers `RequestedRewindInvalid` rather than truncating, and
/// before the fix:
///
/// 1. the derived-anchor frontiers `put_blocks` had committed SURVIVED the
///    `Err` — the undo did not happen, in exactly the deep-restore window an
///    attacker would work in; and
/// 2. `RequestedRewindInvalid` had no arm in `ClassifyStoreFault`, so it took
///    the `_ => StoreCorrupt` default and the user was sent to restore from
///    their seed over a condition the SERVER caused.
///
/// Here the wallet is anchored and NOT pre-scanned, so the pass's very first
/// batch fetches at the birthday and nothing sits at or below it. Asserted:
/// the pass is `EndpointMisbehaving` (never `StoreCorrupt`, never `Ok`), and
/// the wallet holds nothing above the anchor afterwards.
#[tokio::test]
async fn a_reconcile_mismatch_undoes_the_streak_on_a_wallet_with_nothing_below_the_anchor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_anchored_at(dir.path(), &vault, h).await;
    assert_eq!(
        w.scanned_tip().await.expect("scanned"),
        None,
        "precondition: NOTHING is scanned, so no `blocks` row sits at or below the anchor the \
         pass's first batch will fetch — the cell the row above cannot reach"
    );

    // The chain runs far enough for the cadence to reconcile, and lies from the
    // first batch's end on — so the reconciling batch meets the disagreement.
    let override_from = h + BATCH;
    let tip = h + u64::from(SYNC_SUMMARY_REFRESH_BATCHES + 1) * BATCH;
    let mut chain = FakeChain::to_tip(tip);
    chain.tree_state_override = Some((override_from, a_tree_state_with_a_hash_no_block_carries()));
    let lies = Arc::clone(&chain.tree_state_lies);
    let (sink, guard) = capture();
    let ctl = w.controller_over(ProofPassClient(chain));
    let pass = ctl.once().await;
    drop(guard);
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let lied_at = fetches(&lies);
    let held_after = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    w.close().await.expect("close");

    assert!(
        !lied_at.is_empty(),
        "the fixture's own non-vacuity: the endpoint must actually have SERVED the disagreeing \
         tree state; batches {shape:?}"
    );
    let advanced = batches
        .iter()
        .filter(|b| outcome_of(b) == "advance")
        .count();
    assert!(
        advanced > 1,
        "the row is about UNDOING a derived streak, so the pass must first have scanned one: \
         {advanced} batches advanced; batches {shape:?}"
    );
    assert!(
        matches!(
            pass,
            Err(WalletError::Sync {
                stall: StallReason::EndpointMisbehaving
            })
        ),
        "the endpoint's tree state contradicted its own blocks, so the verdict is the \
         ENDPOINT's. At the base the truncate was refused by upstream and the refusal read as \
         StoreCorrupt — the red restore-from-your-seed remedy, for a condition the server \
         caused. Got {pass:?}; batches {shape:?}"
    );
    // The undo's reach in THIS geometry, stated exactly rather than wished for.
    // Upstream refuses a truncate to a height with no `blocks` row at or below
    // it, and the fetched anchor at `h` is exactly that here — so the undo lands
    // on the FALLBACK, the first derived batch's own anchor, which is a `blocks`
    // row by construction. That removes every batch that rode a derived anchor
    // and leaves the ONE scanned on the independently fetched one. It is the
    // smallest residue this API can express; removing it too needs a rescan, not
    // a rewind, and that is recorded as owed rather than pretended away.
    assert!(
        held_after.is_some_and(|held| u64::from(held) <= h + BATCH),
        "the wallet must hold at most the ONE batch scanned on the FETCHED anchor — every \
         batch above that rode an anchor only the endpoint's own blocks attested. It holds \
         {held_after:?} against an anchor of {h} and a batch size of {BATCH}; {advanced} \
         batches advanced; batches {shape:?}"
    );
    assert!(
        held_after.is_some_and(|held| u64::from(held) > h),
        "…and the undo must not have gone BELOW the fetched anchor either — there is nothing \
         down there to remove and a deeper truncate would only throw away attested work. It \
         holds {held_after:?}"
    );
}

// ── Option (c): the verdict split by field ───────────────────────────────────
//
// The maintainer took option (c) on 2026-09-11: a frontier/height mismatch stays
// the endpoint's fault and still undoes; a hash-ONLY mismatch that is a verified
// byte REVERSAL keeps the work and the pass completes.
//
// The maintainer's refinement ALSO said to stop deriving for that endpoint's
// session, and the crypto audit showed that half to be a security
// regression rather than a safeguard: with deriving off every batch takes the
// fetch-and-TRUST path, which compares nothing, so one reversed hash — free, and
// indistinguishable from an honest quirky server — switched the ommer-level
// reconcile off for the whole session. The benign arm ALONE ends the live-lock,
// so deriving now continues and c-4 is the row that holds it to that.

/// **Option (c), the benign arm.** A server whose `GetTreeState` block hash is in
/// the opposite byte-order convention from its `GetBlockRange` hash COMPLETES the
/// pass and keeps the scanned work.
///
/// The fixture is not hypothetical: `FakeChain::tree_state_byte_order_flip` serves
/// the honest state with the raw block id hexed WITHOUT the reversal
/// `chain_tree_state` applies — which is exactly the shape this repository's own
/// fixtures had in THREE places across three sessions, and it survived every time
/// because nothing downstream reads the field.
///
/// Asserted, and each clause is about a different failure this could have had:
/// 1. the pass is `Ok` — before (c) the whole `ChainState` comparison made this
///    `EndpointMisbehaving`, and with SCAN-2's undo it made no net progress;
/// 2. the wallet HOLDS the new tip — the scanned work stands, nothing was undone;
/// 3. the warn carries `outcome = "anchor_hash_byte_order"`, its OWN code, so a
///    field report separates one server's hex convention from two RPCs describing
///    two different chains, and it fires exactly ONCE even though the condition
///    recurs every cadence;
/// 4. **deriving CONTINUES** — the receipt keeps the every-cadence shape rather
///    than collapsing to one fetch per batch. This is the clause the audit
///    inverted: it used to assert the opposite;
/// 5. the fixture's own non-vacuity: the flip actually FIRED (`tree_state_lies`).
#[tokio::test]
async fn a_hash_only_byte_order_mismatch_keeps_the_work_and_keeps_deriving() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_anchored_at(dir.path(), &vault, h - 250).await;

    let ctl = w.controller_over(ProofPassClient(FakeChain::to_tip(h)));
    let first = ctl.once().await.expect("pass 1");
    assert!(
        first.batches > 0,
        "precondition: a REAL scan drives the wallet to H"
    );
    assert_eq!(
        w.scanned_tip()
            .await
            .expect("scanned")
            .map(BlockHeight::value),
        Some(u32::try_from(h).expect("a fixture height")),
        "precondition: after pass 1 the wallet HOLDS H"
    );
    drop(ctl);

    let flip_from = h + BATCH; // pass 2's first batch's `end − 1`
    let tip = h + u64::from(SYNC_SUMMARY_REFRESH_BATCHES + 1) * BATCH;
    let mut chain = FakeChain::to_tip(tip);
    chain.tree_state_byte_order_flip = Some(flip_from);
    let receipt = Arc::clone(&chain.tree_states);
    let lies = Arc::clone(&chain.tree_state_lies);
    let (sink, guard) = capture();
    let ctl2 = w.controller_over(ProofPassClient(chain));
    let pass = ctl2.once().await;
    drop(guard);
    let reconciles = sink.records_of("wallet.anchor_reconcile");
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let fetched = fetches(&receipt);
    let flipped_at = fetches(&lies);
    let held_after = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    w.close().await.expect("close");

    // Clause 5 first: anything below is about the flip, so prove it happened.
    assert!(
        !flipped_at.is_empty(),
        "the fixture's own non-vacuity: the endpoint must actually have SERVED the other-convention \
         hash during this pass — the flip at {flip_from} never fired. The pass asked for {} tree \
         states, at {fetched:?}; batches {shape:?}",
        fetched.len()
    );
    // Clause 1.
    let pass = pass.unwrap_or_else(|e| {
        panic!(
            "§3a (c): a hash that differs only by byte ORDER is benign — nothing in the scan path \
             reads the field — so the pass must COMPLETE. Got {e:?}; reconcile warns {:?}; batches \
             {shape:?}",
            reconciles
                .iter()
                .map(|r| r
                    .iter()
                    .map(|(n, v)| format!("{n}={v}"))
                    .collect::<Vec<_>>()
                    .join(" "))
                .collect::<Vec<_>>()
        )
    });
    // Clause 2 — the work STANDS. The opposite of the undo rows above.
    assert_eq!(
        held_after,
        Some(u32::try_from(tip).expect("a fixture height")),
        "the scanned work must stand: the endpoint told the truth about which block it was, so \
         there is nothing to undo. The pass reports {} batches; batches {shape:?}",
        pass.batches
    );
    // Clause 3 — its OWN code, and the endpoint is NOT blamed.
    let codes: Vec<&str> = reconciles.iter().map(|r| outcome_of(r)).collect();
    assert!(
        codes.contains(&"anchor_hash_byte_order"),
        "the benign arm must warn under its own code so a field report can separate a hex \
         convention from two different chains; got {codes:?}"
    );
    assert!(
        !codes.contains(&"anchor_mismatch"),
        "and it must NOT also report the endpoint's-fault code for the same condition; got \
         {codes:?}"
    );
    assert_eq!(
        codes
            .iter()
            .filter(|&&c| c == "anchor_hash_byte_order")
            .count(),
        1,
        "and it warns ONCE per session, not once per cadence — the condition recurs by \
         construction, so a per-reconcile warn would bury the log for a server that is \
         behaving. Got {codes:?}"
    );
    // Clause 4 — DERIVING CONTINUES. This clause used to assert the opposite (that
    // every later batch fetched, the latched pre-SCAN-2 anchor path) and the
    // crypto audit showed that behaviour to be the regression: the fetch-and-trust
    // path compares nothing, so the ommer-level reconcile was switched off for the
    // session by one free, honest-looking reversed hash.
    //
    // Stated as the SHAPE of the fake's own receipt, not as a count: with deriving
    // on, a pass this long fetches about once per cadence, so the tail must still
    // contain `cadence × BATCH` gaps. Collapse to one fetch per batch — every gap
    // equal to BATCH — is exactly the regression, and it reds here.
    let cadence = u64::from(crate::constants::TREE_STATE_RECONCILE_BATCHES);
    let advanced = batches
        .iter()
        .filter(|b| outcome_of(b) == "advance")
        .count() as u64;
    assert!(
        advanced > 2 * cadence,
        "precondition for clause 4: the pass must span more than TWO cadences, or 'it kept \
         deriving after the flip' is not a claim — one cadence could be the streak that \
         caught the flip. {advanced} batches advanced, cadence {cadence}; batches {shape:?}"
    );
    let after_flip: Vec<u64> = fetched
        .iter()
        .copied()
        .filter(|&f| f >= flip_from)
        .collect();
    let gaps: Vec<u64> = after_flip.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        gaps.iter().any(|&g| g == cadence * BATCH),
        "deriving must CONTINUE after a benign hash-order mismatch, so the receipt's tail \
         still holds at least one {}-block gap (one fetch per cadence). Got gaps {gaps:?} \
         over {after_flip:?} — all-{BATCH} gaps would mean every batch fetched, which is the \
         fetch-and-TRUST path the audit found: it compares nothing, so an endpoint that \
         serves ONE reversed hash turns the reconcile off for the session. ({advanced} \
         batches advanced.) Receipt {fetched:?}",
        cadence * BATCH
    );
}

/// **Option (c), clause 4's teeth — the row the crypto audit's MEDIUM 1
/// bought.** After a benign hash-order mismatch, the every-cadence reconcile is
/// STILL LIVE: a frontier lie later in the same session is still caught.
///
/// The finding this row exists for: with "stop deriving" built literally, every
/// batch after the benign arm took the `None` anchor path, which fetches the
/// served state and USES it with no comparison at all. So an endpoint could serve
/// ONE reversed hash — free, and by construction indistinguishable from an honest
/// quirky server — and thereby switch the ommer-level reconcile off for the whole
/// session, after which anchors with a correct `tree_size` and WRONG ommers pass
/// (the surviving checks, `initial_block_sequential` and the fold's
/// `chain_metadata` self-check, are size-level only). Those ommers become the
/// left-hand witness data for every note appended after: correct balance,
/// unprovable spends, no repair short of a full rescan.
///
/// c-1 asserts the receipt's shape, which is evidence about the mechanism. This
/// row asserts the PROPERTY: both fixture knobs armed at once, the byte-order flip
/// from the pass's first reconcile and the frontier lie from higher up, and the
/// pass must still end as the endpoint's fault. Under the regression it would
/// return `Ok` and commit the lie.
#[tokio::test]
async fn a_benign_hash_order_mismatch_does_not_disarm_the_frontier_check() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_anchored_at(dir.path(), &vault, h - 250).await;

    let ctl = w.controller_over(ProofPassClient(FakeChain::to_tip(h)));
    ctl.once().await.expect("pass 1");
    let held_before = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    assert_eq!(
        held_before,
        Some(u32::try_from(h).expect("a fixture height")),
        "precondition: after pass 1 the wallet HOLDS H"
    );
    drop(ctl);

    let window = decode_window(
        &std::fs::read(FIXTURE_PATH)
            .unwrap_or_else(|e| panic!("the committed window {FIXTURE_PATH} reads: {e}")),
    );
    let foreign_frontier = window.start.sapling_tree.clone();
    assert!(
        !foreign_frontier.is_empty(),
        "precondition: the committed window's Sapling frontier is non-empty"
    );

    let cadence = u64::from(crate::constants::TREE_STATE_RECONCILE_BATCHES);
    // The flip from the FIRST reconcile the pass can reach, so the benign arm is
    // taken early; the frontier lie only from well above it, so it can only be
    // caught by a reconcile that happened AFTER the benign arm ran.
    let flip_from = h + BATCH;
    let lie_from = h + (cadence + 2) * BATCH;
    let tip = h + u64::from(SYNC_SUMMARY_REFRESH_BATCHES + 1) * BATCH;
    assert!(
        tip > lie_from + cadence * BATCH,
        "precondition: the pass must run at least one more full cadence above {lie_from}, or \
         the frontier lie is never reconciled against and the row proves nothing"
    );
    let mut chain = FakeChain::to_tip(tip);
    chain.tree_state_byte_order_flip = Some(flip_from);
    chain.tree_state_frontier_lie = Some((lie_from, foreign_frontier));
    let lies = Arc::clone(&chain.tree_state_lies);
    let (sink, guard) = capture();
    let ctl2 = w.controller_over(ProofPassClient(chain));
    let pass = ctl2.once().await;
    drop(guard);
    let reconciles = sink.records_of("wallet.anchor_reconcile");
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let lied_at = fetches(&lies);
    let held_after = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    w.close().await.expect("close");

    let codes: Vec<&str> = reconciles.iter().map(|r| outcome_of(r)).collect();
    // Non-vacuity, in BOTH directions: the benign arm has to have run (or this is
    // just c-2 again), and the frontier lie has to have been served.
    assert!(
        codes.contains(&"anchor_hash_byte_order"),
        "non-vacuity: the byte-order flip must actually have been reconciled against and taken \
         the BENIGN arm before the frontier lie — otherwise this row is c-2 with extra steps. \
         Got {codes:?}; batches {shape:?}"
    );
    assert!(
        lied_at.iter().any(|&l| l >= lie_from),
        "non-vacuity: the frontier lie at or above {lie_from} must actually have been SERVED; \
         the fake served disagreements at {lied_at:?}"
    );
    // THE PROPERTY.
    assert!(
        matches!(
            pass,
            Err(WalletError::Sync {
                stall: StallReason::EndpointMisbehaving
            })
        ),
        "a benign hash-order mismatch must NOT disarm the reconcile: the frontier lie above it \
         is still the endpoint's fault. Got {pass:?} — an `Ok` here means the session stopped \
         comparing anchors after the benign arm, which is the fetch-and-TRUST regression. \
         Codes {codes:?}; batches {shape:?}"
    );
    assert!(
        codes.contains(&"anchor_mismatch"),
        "and it is caught BY the reconcile, under the endpoint's-fault code — not by some \
         other guard further down. Got {codes:?}"
    );
    // And the undo runs — but NOT back to H, and the difference is a property of
    // (c) worth pinning rather than an inconvenience. The benign arm stamps its
    // served anchor as a FETCHED one (it was independently attested: the states
    // agreed on the height and all three frontiers), so the undo target is that
    // anchor rather than the pass's first one, and strictly LESS work is discarded
    // than c-2 discards. My first draft of this clause asserted `held_after ==
    // held_before` — c-2's geometry — and it red at `Some(4304840)` vs
    // `Some(4302840)`, which is the mechanism telling the truth.
    let held_after_h = held_after
        .map(u64::from)
        .expect("the wallet holds a height");
    assert!(
        held_after_h > u64::from(held_before.expect("pass 1 stamped")),
        "the undo must land on the anchor the BENIGN arm attested, above H — the benign arm \
         stamped a fetched anchor, so there is attested work between H and the lie that must \
         NOT be thrown away. It holds {held_after:?}, pass 1 held {held_before:?}; batches \
         {shape:?}"
    );
    assert!(
        held_after_h < lie_from,
        "…and it must still be BELOW the first lying height {lie_from}: every batch above the \
         last attested anchor rode an anchor only the endpoint's own blocks attested, and the \
         reconcile refused the state that was supposed to attest them. It holds {held_after:?}"
    );
}

/// **Option (c), the arm that must NOT have changed.** A mismatch in a FRONTIER
/// still undoes the derived streak and still returns `EndpointMisbehaving`.
///
/// Every pre-(c) row lies on the block HASH, so without this row nothing measures
/// the frontier path at all — and the frontier is the field that actually matters:
/// it decides where each note sits, and a shifted position means a wrong witness,
/// a spend that fails to prove, and a balance that still reads correct.
/// `tree_state_frontier_lie` therefore keeps the hash HONEST and replaces the
/// Sapling frontier with a real one from the committed mainnet window, so the
/// served state differs from the derived one in a frontier and in nothing else.
#[tokio::test]
async fn a_frontier_mismatch_still_undoes_the_streak_and_still_blames_the_endpoint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_anchored_at(dir.path(), &vault, h - 250).await;

    let ctl = w.controller_over(ProofPassClient(FakeChain::to_tip(h)));
    ctl.once().await.expect("pass 1");
    let held_before = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    assert_eq!(
        held_before,
        Some(u32::try_from(h).expect("a fixture height")),
        "precondition: after pass 1 the wallet HOLDS H"
    );
    drop(ctl);

    // A REAL non-empty Sapling frontier, from the committed window — the fake's own
    // chain has empty frontiers everywhere, so this differs, and it differs as a
    // frontier a real server could actually serve rather than as arbitrary bytes.
    let window = decode_window(&std::fs::read(FIXTURE_PATH).unwrap_or_else(|e| {
        panic!("the committed window {FIXTURE_PATH} reads (the capture tool writes it): {e}")
    }));
    let foreign_frontier = window.start.sapling_tree.clone();
    assert!(
        !foreign_frontier.is_empty(),
        "precondition: the committed window's Sapling frontier is non-empty, so it differs from \
         the fake chain's empty one"
    );

    let lie_from = h + BATCH;
    let tip = h + u64::from(SYNC_SUMMARY_REFRESH_BATCHES + 1) * BATCH;
    let mut chain = FakeChain::to_tip(tip);
    chain.tree_state_frontier_lie = Some((lie_from, foreign_frontier));
    let lies = Arc::clone(&chain.tree_state_lies);
    let (sink, guard) = capture();
    let ctl2 = w.controller_over(ProofPassClient(chain));
    let pass = ctl2.once().await;
    drop(guard);
    let reconciles = sink.records_of("wallet.anchor_reconcile");
    let batches = captured_batches(&sink);
    let shape = batch_shape(&batches);
    let lied_at = fetches(&lies);
    let held_after = w
        .scanned_tip()
        .await
        .expect("scanned")
        .map(BlockHeight::value);
    w.close().await.expect("close");

    assert!(
        !lied_at.is_empty(),
        "the fixture's own non-vacuity: the endpoint must actually have SERVED the foreign \
         frontier during this pass; batches {shape:?}"
    );
    assert!(
        matches!(
            pass,
            Err(WalletError::Sync {
                stall: StallReason::EndpointMisbehaving
            })
        ),
        "splitting the verdict by FIELD must not soften the frontier: a served frontier the \
         wallet's own derivation contradicts is still the endpoint's fault. Got {pass:?}; \
         reconcile warns {:?}; batches {shape:?}",
        reconciles.iter().map(|r| outcome_of(r)).collect::<Vec<_>>()
    );
    assert_eq!(
        held_after, held_before,
        "and the undo still happens: every batch above H rode an anchor only the endpoint's own \
         blocks attested; batches {shape:?}"
    );
    let codes: Vec<&str> = reconciles.iter().map(|r| outcome_of(r)).collect();
    assert!(
        codes.contains(&"anchor_mismatch"),
        "under the endpoint's-fault code, never the benign one; got {codes:?}"
    );
    assert!(
        !codes.contains(&"anchor_hash_byte_order"),
        "the benign code must not fire for a frontier disagreement; got {codes:?}"
    );
    let field = reconciles
        .iter()
        .find(|r| outcome_of(r) == "anchor_mismatch")
        .and_then(|r| field_of(r, "field").map(str::to_owned));
    assert_eq!(
        field.as_deref(),
        Some("sapling"),
        "and the field code names the frontier that differs, not the hash — the discriminator \
         §4t-run review row 3 asked for"
    );
}

/// **Option (c)'s third row — what stops it becoming "ignore the hash".** The
/// excuse is the verified byte REVERSAL and nothing wider.
///
/// A unit row on the discriminator itself rather than a fourth full pass: the
/// question is which pairs of states the excuse covers, and that is answered
/// exactly by enumerating them. Real mainnet bytes for the base state (the
/// committed window), so the frontiers are a server's own rather than empty.
#[test]
fn only_a_verified_byte_reversal_of_the_hash_is_excused() {
    let window = decode_window(
        &std::fs::read(FIXTURE_PATH)
            .unwrap_or_else(|e| panic!("the committed window {FIXTURE_PATH} reads: {e}")),
    );
    let served = window
        .start
        .to_chain_state()
        .expect("TreeState(WINDOW_START) decodes");

    let with_hash = |bytes: [u8; 32]| {
        ChainState::new(
            served.block_height(),
            BlockHash(bytes),
            served.final_sapling_tree().clone(),
            served.final_orchard_tree().clone(),
            served.final_ironwood_tree().clone(),
        )
    };
    let mut reversed_bytes = served.block_hash().0;
    reversed_bytes.reverse();

    // (a) THE ONE CASE EXCUSED: the derived hash is the exact reversal of the
    //     served one, everything else equal.
    assert!(
        sync::is_hash_only_byte_reversal(&with_hash(reversed_bytes), &served),
        "the pure reversal is the byte-order convention and is the one benign case"
    );
    // (b) Equal hashes are not a mismatch at all — and this is also the guard that
    //     keeps a palindromic hash (its own reversal) out of the benign arm.
    assert!(
        !sync::is_hash_only_byte_reversal(&served, &served),
        "an identical state is no mismatch, so it is not a benign mismatch either"
    );
    // (c) A hash that merely DIFFERS is two different blocks at one height — the
    //     endpoint's two RPCs describing two different chains. Not excused.
    let mut arbitrary = served.block_hash().0;
    arbitrary[0] ^= 0x01;
    assert!(
        !sync::is_hash_only_byte_reversal(&with_hash(arbitrary), &served),
        "a hash that is neither equal nor the reversal names a DIFFERENT block; excusing it \
         would turn splitting the verdict by field into ignoring the hash"
    );
    // (d) The reversal is NOT a licence for the rest of the state: the reversed
    //     hash plus a frontier difference is still the endpoint's fault. This is
    //     the case a field-NAME check would have got wrong —
    //     `chain_state_mismatch_field` returns "hash" for it, first match wins.
    let reversed_and_a_foreign_frontier = ChainState::new(
        served.block_height(),
        BlockHash(reversed_bytes),
        window
            .end
            .to_chain_state()
            .expect("TreeState(WINDOW_START + WINDOW_BLOCKS) decodes")
            .final_sapling_tree()
            .clone(),
        served.final_orchard_tree().clone(),
        served.final_ironwood_tree().clone(),
    );
    assert_ne!(
        window
            .end
            .to_chain_state()
            .expect("decodes")
            .final_sapling_tree(),
        served.final_sapling_tree(),
        "precondition: the window's two ends have different Sapling frontiers, so (d) really \
         does carry a frontier difference"
    );
    assert!(
        !sync::is_hash_only_byte_reversal(&reversed_and_a_foreign_frontier, &served),
        "the reversal excuses the HASH and nothing else: a frontier difference alongside it is \
         still the endpoint's fault. A check written over the field NAME would have excused \
         this one — `chain_state_mismatch_field` answers \"hash\" here, because it compares \
         height then HASH before either frontier and the first difference wins"
    );
    // (e) A height difference alongside the reversal, for the same reason.
    let reversed_at_another_height = ChainState::new(
        served.block_height() + 1,
        BlockHash(reversed_bytes),
        served.final_sapling_tree().clone(),
        served.final_orchard_tree().clone(),
        served.final_ironwood_tree().clone(),
    );
    assert!(
        !sync::is_hash_only_byte_reversal(&reversed_at_another_height, &served),
        "nor does it excuse a height disagreement"
    );
}

// ── S2-2: the vector, its fixture and its capture tool ───────────────────────

/// The fixture's on-disk format, one file: this magic, then `u64` LE `h` (the
/// window's start height, the height of the first tree state), then `u32` LE
/// `k` (the block count), then — each `prost` length-delimited — `TreeState(h)`,
/// the `k` compact blocks `h+1..=h+k` in height order, and `TreeState(h+k)`.
/// Nothing else: the reader asserts the file ends there.
const FIXTURE_MAGIC: &[u8; 9] = b"ZWANCHOR1";

/// Where the committed fixture lives (the capture tool writes it, the row reads it).
const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/mainnet-anchor-window.bin"
);

/// The committed window: `TreeState(WINDOW_START)`, the blocks
/// `WINDOW_START + 1 ..= WINDOW_START + WINDOW_BLOCKS`, and the tree state at
/// `WINDOW_START + WINDOW_BLOCKS`. Mainnet, post-NU6.3 (Ironwood activation is
/// above [`IRONWOOD_ACTIVATION_FLOOR`]). Pinned here so the row's doc and the
/// fixture's bytes cannot drift: the row asserts the file's header against these.
const WINDOW_START: u64 = 3_477_766;
/// `k`: two batches, so at least one derived step feeds another.
const WINDOW_BLOCKS: u32 = 2 * SYNC_BATCH_BLOCKS;
/// Note commitments in the window, per pool — Sapling outputs, Orchard
/// actions, Ironwood actions — as the capture tool counted and printed them;
/// each > 0 (§4t S2-2: all three pools).
const WINDOW_COMMITMENTS: [u64; 3] = [74, 233, 1123];

/// The window's cap: the smallest window the contract allows, chosen to stay
/// well inside the repository's size discipline.
const FIXTURE_MAX_BYTES: usize = 300 * 1024;

/// The floor §4t names for "post-NU6.3 mainnet": Ironwood activation is above
/// this height. Both the capture tool (which never searches below it) and the
/// row (which asserts the committed window is above it) read this one constant.
const IRONWOOD_ACTIVATION_FLOOR: u64 = 3_451_205;

struct AnchorWindow {
    start: TreeState,
    blocks: Vec<CompactBlock>,
    end: TreeState,
}

fn encode_window(w: &AnchorWindow) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(FIXTURE_MAGIC);
    buf.extend_from_slice(&w.start.height.to_le_bytes());
    buf.extend_from_slice(
        &u32::try_from(w.blocks.len())
            .expect("a small count")
            .to_le_bytes(),
    );
    w.start
        .encode_length_delimited(&mut buf)
        .expect("a Vec never runs out of room");
    for b in &w.blocks {
        b.encode_length_delimited(&mut buf)
            .expect("a Vec never runs out of room");
    }
    w.end
        .encode_length_delimited(&mut buf)
        .expect("a Vec never runs out of room");
    buf
}

fn decode_window(bytes: &[u8]) -> AnchorWindow {
    let (magic, rest) = bytes.split_at(FIXTURE_MAGIC.len());
    assert_eq!(magic, FIXTURE_MAGIC, "the fixture's magic");
    let (h, rest) = rest.split_at(8);
    let h = u64::from_le_bytes(h.try_into().expect("8 bytes"));
    let (k, rest) = rest.split_at(4);
    let k = u32::from_le_bytes(k.try_into().expect("4 bytes"));
    let mut cur: &[u8] = rest;
    let start: TreeState = Message::decode_length_delimited(&mut cur).expect("TreeState(h)");
    let blocks: Vec<CompactBlock> = (0..k)
        .map(|_| Message::decode_length_delimited(&mut cur).expect("a compact block"))
        .collect();
    let end: TreeState = Message::decode_length_delimited(&mut cur).expect("TreeState(h+k)");
    assert!(
        cur.is_empty(),
        "the fixture holds nothing past its last tree state"
    );
    assert_eq!(
        start.height, h,
        "the header's h is the first tree state's height"
    );
    assert_eq!(
        end.height,
        h + u64::from(k),
        "the last tree state is at h + k"
    );
    AnchorWindow { start, blocks, end }
}

/// Note commitments per pool over `blocks`: Sapling outputs, Orchard actions,
/// Ironwood actions — the fields the scanner appends to each pool's tree.
fn pool_commitments(blocks: &[CompactBlock]) -> [u64; 3] {
    blocks.iter().fold([0u64; 3], |acc, b| {
        b.vtx.iter().fold(acc, |[s, o, i], tx| {
            [
                s + tx.outputs.len() as u64,
                o + tx.actions.len() as u64,
                i + tx.ironwood_actions.len() as u64,
            ]
        })
    })
}

/// The window's blocks are the contiguous heights `start + 1 ..= end` and each
/// chains from the one below (the shape `derive_chain_state` folds).
fn assert_window_is_contiguous(w: &AnchorWindow) {
    assert_eq!(
        w.blocks.first().map(|b| b.height),
        Some(w.start.height + 1),
        "the first block is one above the start tree state"
    );
    for pair in w.blocks.windows(2) {
        assert_eq!(pair[1].height, pair[0].height + 1, "contiguous heights");
        assert_eq!(
            pair[1].prev_hash, pair[0].hash,
            "each block chains from the one below"
        );
    }
    assert_eq!(
        w.blocks.last().map(|b| b.height),
        Some(w.end.height),
        "the last block is the end tree state's"
    );
}

/// A lightwalletd block hash as `TreeState.hash` renders it: zcashd's hex is
/// byte-reversed relative to the compact block's raw `hash` field
/// (`TreeState::to_chain_state` reverses it back).
fn display_hash(raw: &[u8]) -> String {
    let mut bytes = raw.to_vec();
    bytes.reverse();
    hex::encode(bytes)
}

/// The chain the fixture is captured from — `TreeState.network` as lightwalletd
/// fills it (measured on the committed bytes: `0a 04 6d 61 69 6e`). §4t S2-2
/// says MAINNET, post-NU6.3; a fixture recaptured against testnet would
/// otherwise be indistinguishable from a good one to the row below.
const WINDOW_NETWORK: &str = "main";

/// Everything that must be true of the window BEFORE a derivation is asked
/// about — the fixture's own soundness, checked identically by the capture tool
/// (which refuses to write a window that fails) and by the row (which refuses
/// to blame a derivation for a window that was never sound). Returns the
/// per-pool commitment counts.
///
/// Without this the vector row would red the same way on a truncated, a
/// hand-made or a testnet fixture as it does on a wrong derivation, and the
/// red would be a fixture reason wearing the row's name.
fn assert_window_is_sound(w: &AnchorWindow) -> [u64; 3] {
    assert_eq!(
        (w.start.network.as_str(), w.end.network.as_str()),
        (WINDOW_NETWORK, WINDOW_NETWORK),
        "§4t S2-2: the window is MAINNET chain data"
    );
    assert_window_is_contiguous(w);
    let first = w.blocks.first().expect("a non-empty window");
    let last = w.blocks.last().expect("a non-empty window");
    assert_eq!(
        w.start.hash,
        display_hash(&first.prev_hash),
        "TreeState(h) names the block the window chains from"
    );
    assert_eq!(
        w.end.hash,
        display_hash(&last.hash),
        "TreeState(h+k) names the window's last block"
    );
    let s = w.start.to_chain_state().expect("TreeState(h) decodes");
    let e = w.end.to_chain_state().expect("TreeState(h+k) decodes");
    let counts = pool_commitments(&w.blocks);
    let sizes = [
        (
            "sapling",
            s.final_sapling_tree().tree_size(),
            e.final_sapling_tree().tree_size(),
        ),
        (
            "orchard",
            s.final_orchard_tree().tree_size(),
            e.final_orchard_tree().tree_size(),
        ),
        (
            "ironwood",
            s.final_ironwood_tree().tree_size(),
            e.final_ironwood_tree().tree_size(),
        ),
    ];
    for (i, (pool, from, to)) in sizes.iter().enumerate() {
        assert_eq!(
            from + counts[i],
            *to,
            "{pool}: the served tree states and the window's blocks agree on the size \
             ({from} + {} commitments should be {to})",
            counts[i]
        );
    }
    assert!(
        counts.iter().all(|&c| c > 0),
        "§4t S2-2: all three pools carry commitments in the window: {counts:?}"
    );
    counts
}

/// **The capture tool (§4t S2-2).** `#[ignore]`d: it talks to a live mainnet
/// lightwalletd and writes the committed fixture. Run ONCE, from `sdk/`:
///
/// ```sh
/// ZEC_WALLET_LWD=https://zec.rocks:443 cargo test -p zec-wallet-core --lib -- \
///   'anchor_proof::capture' --ignored --nocapture
/// ```
///
/// The endpoint door is `live_scan_probe.rs`'s, reused: `ZEC_WALLET_LWD` (the
/// public default), and the optional `ZEC_WALLET_LWD_KEY_HEADER` /
/// `ZEC_WALLET_LWD_KEY` pair, both or neither. Walks candidate windows of
/// `WINDOW_BLOCKS` blocks downward from a day below the tip through the
/// crate's own `LightwalletdClient` (`ScanClient::block_range`, `tree_state`)
/// and takes the first whose three pools all carry commitments and whose
/// encoding is under [`FIXTURE_MAX_BYTES`]; then fetches the two tree states,
/// checks the window against them — the block hashes name the tree states'
/// blocks, and each pool's start size plus the window's commitments is its end
/// size (the fold's own arithmetic, so a window the row could not pass for a
/// fixture reason is refused here) — prints the heights, the counts and the
/// size, and writes the fixture. §5.4: heights, counts and hashes of public
/// blocks only.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network: captures the S2-2 mainnet fixture; run once, on purpose"]
async fn capture_the_mainnet_anchor_window() {
    /// The search walks DOWN from `tip − CAPTURE_SEARCH_BELOW_TIP` one window
    /// at a time (a day below the tip is far past Zcash's 100-block pruning
    /// depth, so the window is final), and never below
    /// [`IRONWOOD_ACTIVATION_FLOOR`].
    const CAPTURE_SEARCH_BELOW_TIP: u64 = 1_000;
    const CAPTURE_SEARCH_ATTEMPTS: u64 = 150;

    let url = std::env::var("ZEC_WALLET_LWD").unwrap_or_else(|_| "https://zec.rocks:443".into());
    let auth = match (
        std::env::var("ZEC_WALLET_LWD_KEY_HEADER"),
        std::env::var("ZEC_WALLET_LWD_KEY"),
    ) {
        (Ok(h), Ok(v)) => Some(EndpointAuth::new(h, v).expect("valid auth pair")),
        (Err(_), Err(_)) => None,
        _ => panic!("ZEC_WALLET_LWD_KEY_HEADER and ZEC_WALLET_LWD_KEY: both or neither"),
    };
    let endpoint = LightServerEndpoint::new(url.clone()).expect("valid endpoint");
    let fell_back = Arc::new(crate::net::tor_posture::TorPosture::new());
    let mut client = LightwalletdClient::connect(
        &endpoint,
        &TorPolicy::Off,
        None,
        &fell_back,
        &Default::default(),
        auth.as_ref(),
    )
    .expect("connect builds lazily");
    let tip = client
        .latest_block_height()
        .await
        .expect("latest_block_height");
    let k = u64::from(WINDOW_BLOCKS);
    println!(
        "capture: {url} tip={tip} window={k} blocks, searching down from {}",
        tip - CAPTURE_SEARCH_BELOW_TIP
    );

    let mut chosen: Option<(u64, Vec<CompactBlock>)> = None;
    for attempt in 0..CAPTURE_SEARCH_ATTEMPTS {
        let h = tip - CAPTURE_SEARCH_BELOW_TIP - k - attempt * k;
        assert!(
            h > IRONWOOD_ACTIVATION_FLOOR,
            "the search ran below the Ironwood activation floor at {h}"
        );
        let mut stream = client.block_range(h + 1, h + k).await.expect("block_range");
        let mut blocks = Vec::new();
        while let Some(b) = stream.next_block().await.expect("a streamed block") {
            blocks.push(b);
        }
        assert_eq!(
            blocks.len() as u64,
            k,
            "the endpoint served the whole window at {h}"
        );
        let counts = pool_commitments(&blocks);
        let bytes: usize = blocks.iter().map(Message::encoded_len).sum();
        println!(
            "  window {}..={}: sapling={} orchard={} ironwood={} blocks_bytes={bytes}",
            h + 1,
            h + k,
            counts[0],
            counts[1],
            counts[2]
        );
        if counts.iter().all(|&c| c > 0) && bytes <= FIXTURE_MAX_BYTES {
            chosen = Some((h, blocks));
            break;
        }
    }
    let (h, blocks) =
        chosen.expect("a window with commitments in all three pools under the size cap");
    let start = client.tree_state(h).await.expect("TreeState(h)");
    let end = client.tree_state(h + k).await.expect("TreeState(h+k)");
    let window = AnchorWindow { start, blocks, end };
    let first = window.blocks.first().expect("k > 0");
    let last = window.blocks.last().expect("k > 0");
    println!(
        "  network={} TreeState({h}).hash={} vs display(prev_hash of {})={}",
        window.start.network,
        window.start.hash,
        first.height,
        display_hash(&first.prev_hash)
    );
    println!(
        "  TreeState({}).hash={} vs display(hash of {})={}",
        h + k,
        window.end.hash,
        last.height,
        display_hash(&last.hash)
    );
    // The same soundness the ROW checks, so the tool never writes a fixture the
    // row would red on for a fixture reason.
    let counts = assert_window_is_sound(&window);
    let s = window.start.to_chain_state().expect("TreeState(h) decodes");
    let e = window.end.to_chain_state().expect("TreeState(h+k) decodes");
    for (i, (pool, from, to)) in [
        (
            "sapling",
            s.final_sapling_tree().tree_size(),
            e.final_sapling_tree().tree_size(),
        ),
        (
            "orchard",
            s.final_orchard_tree().tree_size(),
            e.final_orchard_tree().tree_size(),
        ),
        (
            "ironwood",
            s.final_ironwood_tree().tree_size(),
            e.final_ironwood_tree().tree_size(),
        ),
    ]
    .iter()
    .enumerate()
    {
        println!(
            "  {pool}: tree size {from} + {} commitments = {to}",
            counts[i]
        );
    }

    let bytes = encode_window(&window);
    // The cap is on the FILE, not just its blocks: the two tree states are a
    // couple of KB each and the search above never weighed them.
    assert!(
        bytes.len() <= FIXTURE_MAX_BYTES,
        "the encoded fixture is {} bytes, over the {FIXTURE_MAX_BYTES}-byte cap",
        bytes.len()
    );
    let dir = Path::new(FIXTURE_PATH).parent().expect("a parent dir");
    std::fs::create_dir_all(dir).expect("tests/fixtures");
    std::fs::write(FIXTURE_PATH, &bytes).expect("write the fixture");
    println!(
        "  wrote {FIXTURE_PATH}: WINDOW_START={h} WINDOW_BLOCKS={k} WINDOW_COMMITMENTS=[{}, {}, {}] size={} bytes",
        counts[0],
        counts[1],
        counts[2],
        bytes.len()
    );
}

/// **§4t S2-2 — the VECTOR (crypto-change review step 3); RED-BY-ABSENCE at the
/// base.** A derived anchor equals the served tree state on a captured mainnet
/// window.
///
/// Loads the committed fixture (heights and counts pinned above and asserted
/// against the bytes), folds the blocks onto `TreeState(WINDOW_START)` with
/// `sync::derive_chain_state` in `SYNC_BATCH_BLOCKS`-sized steps — two steps,
/// so the second consumes the first's result — and asserts the result equals
/// `TreeState(WINDOW_START + WINDOW_BLOCKS).to_chain_state()`: height, hash
/// and each of the three frontiers named one by one (so a red says WHICH
/// disagreed), then the whole `ChainState` (`PartialEq`).
///
/// The exact call this row makes, for the join: `sync::derive_chain_state(&state,
/// chunk)` with `state: ChainState` and `chunk: &[CompactBlock]` of length
/// `SYNC_BATCH_BLOCKS`, reached through the one-line [`the_seam`]. At the base
/// that function does not exist, so the row reds on the first fold step with a
/// panic that names it. Killer mutants (S2-8): the Ironwood fold
/// dropped → the ironwood frontier clause; a pool's constructor swapped
/// (Sapling's `from_cmu` for Orchard's `from_cmx`) → that pool's clause; the
/// hash taken from the wrong block → the hash clause.
#[test]
fn a_derived_anchor_equals_the_served_tree_state_on_a_captured_mainnet_window() {
    let bytes = std::fs::read(FIXTURE_PATH).unwrap_or_else(|e| {
        panic!("the committed fixture {FIXTURE_PATH} reads (the capture tool writes it): {e}")
    });
    assert!(
        bytes.len() <= FIXTURE_MAX_BYTES,
        "the committed fixture is {} bytes, over the {FIXTURE_MAX_BYTES}-byte cap §4t asks the \
         window to stay inside",
        bytes.len()
    );
    let window = decode_window(&bytes);
    assert_eq!(window.start.height, WINDOW_START, "the pinned window start");
    assert_eq!(
        window.blocks.len(),
        WINDOW_BLOCKS as usize,
        "the pinned window length"
    );
    // Two facts about the pinned window that are true at COMPILE time, so a
    // future edit to either constant cannot get past `cargo build`.
    const {
        assert!(
            WINDOW_BLOCKS >= 2 * SYNC_BATCH_BLOCKS,
            "k ≥ 2 × SYNC_BATCH_BLOCKS so one derived step feeds another"
        );
    }
    const {
        assert!(
            WINDOW_START > IRONWOOD_ACTIVATION_FLOOR,
            "§4t S2-2: post-NU6.3 mainnet — the window is above the Ironwood activation floor"
        );
    }
    // The fixture's own soundness FIRST (mainnet, contiguous, both tree states
    // naming the window's ends, each pool's sizes reconciling with the blocks),
    // so anything that reds below is the derivation and not the bytes.
    let counts = assert_window_is_sound(&window);
    assert_eq!(
        counts, WINDOW_COMMITMENTS,
        "the pinned per-pool commitment counts"
    );

    let mut state = window
        .start
        .to_chain_state()
        .expect("TreeState(WINDOW_START) decodes");
    let mut steps = 0;
    for chunk in window.blocks.chunks(SYNC_BATCH_BLOCKS as usize) {
        state = the_seam::derive_chain_state(&state, chunk).unwrap_or_else(|e| {
            panic!(
                "derive_chain_state over {}..={} from the state at {}: {e:?}",
                chunk[0].height,
                chunk[chunk.len() - 1].height,
                state.block_height()
            )
        });
        steps += 1;
    }
    assert_eq!(steps, 2, "two batch-sized steps over the window");
    let served = window
        .end
        .to_chain_state()
        .expect("TreeState(WINDOW_START + WINDOW_BLOCKS) decodes");
    assert_eq!(state.block_height(), served.block_height(), "the height");
    assert_eq!(state.block_hash(), served.block_hash(), "the block hash");
    assert_eq!(
        state.final_sapling_tree(),
        served.final_sapling_tree(),
        "the Sapling frontier"
    );
    assert_eq!(
        state.final_orchard_tree(),
        served.final_orchard_tree(),
        "the Orchard frontier"
    );
    assert_eq!(
        state.final_ironwood_tree(),
        served.final_ironwood_tree(),
        "the Ironwood frontier"
    );
    assert_eq!(state, served, "the whole ChainState");
}
