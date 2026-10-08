//! S15-F1 phase B — "subtree-root ingestion fetches only what is new": the
//! test author's WALLET-LEVEL rows, written BLIND to the implementation.
//!
//! Contract: `docs/plan/s15-f1-subtree-roots-fetch-only-what-is-new.md`,
//! revision 2 — §3 (behaviour and names), §4 (the guarantee ledger), §7 (rows
//! T1–T24, T3b, T3c, and the rounding preamble). Every row below names the §7
//! row it is, the clause it pins, and the mutant it kills.
//!
//! A child of `wallet::tests` for its fixtures (`cfg`, `raw_seed`,
//! `test_vault`, `custom`, `wallet_with_chain_account`, `ScriptedPassClient`,
//! `FIX_TIP`), in its own file so no cited line of `wallet.rs` moves.
//!
//! ## What is driven, and what is observed
//!
//! **Driven** through `Wallet::update_subtree_roots`, the door every pass
//! takes, over this file's own [`Endpoint`] double. It honours `start_index`
//! exactly the way the two measured servers do (§1: a start at or past the
//! count is an empty OK stream), RECORDS every `(pool, start)` it is asked for,
//! and can be told to misbehave on a NON-ZERO start only — so a fallback to 0
//! always meets an honest server, and the row measures the fallback rather than
//! a second fault.
//!
//! **Observed** on four things, never on the implementer's private storage:
//! 1. the starts the endpoint was ASKED for — the wire-visible fact this change
//!    exists to move;
//! 2. upstream's own `{pool}_tree_shards` rows, through the aux connection;
//! 3. the two `cfg(test)` seams the contract names — `Wallet::root_ingest_memo`
//!    (a snapshot of the §3.1 `PoolMemo`) and `Wallet::root_ingest_fallbacks`;
//! 4. the §3.3 step 7 `outcome` codes, through the crate's `CaptureLayer`.
//!
//! ## Fixture rules (§7's preamble)
//!
//! Every multi-root fixture stores MORE than 128 roots, so `count − 1` and each
//! plan term can sit in a different 64-block. Off-by-one rows put the value on
//! a block edge (`≡ 0` or `≡ 63 (mod 64)`); the expected start of every row is
//! written as a literal beside the arithmetic that produces it.
//!
//! Heights are REAL-chain-consistent: Sapling and Orchard come from
//! `sync::testing::bind_consistent_heights` on MAINNET (the bind reads the
//! signed bundle, so an invented height is a refused endpoint, not a fixture);
//! Ironwood, which the bundle proves only a handful of, is extended ABOVE the
//! bundle's newest row (every bundled row then says "not yet complete" of the
//! extension, which is what an honest chain past the bundle looks like). The
//! endpoint's tip sits 1,000 blocks above everything served, and the scanned tip
//! is passed equal to it, so the reorg-window term (§3.2 term 2) is ABSENT
//! unless a row sets it on purpose.
//!
//! ## The C6 rows write `blocks` directly
//!
//! T8, T19 and T21 need a SCANNED completing block. Scanning a mainnet-height
//! block through `sync_once` needs a whole self-consistent chain
//! (`sync_bind_proof`'s `ScriptedChain`); what the bind and the plan read is one
//! `blocks` row's `hash` (`root_bind::read_scanned_hashes`). So those rows insert
//! that row through the aux connection — a fixture of what a scan leaves, with
//! every tree-size column NULL so the scanned-count oracle (d) abstains and
//! cannot be the refuser. The cost: these rows do not prove the scan writes
//! `blocks.hash` (the existing C6 rows in `sync_bind_proof` do).
//!
//! ## The adapter
//!
//! Every shape the contract left to the implementer is bound in ONE section
//! below, marked `// ADAPTER`. Nothing else in this file names an implementer
//! type, so the merge edits that section and nothing else.

use super::*;

use std::collections::BTreeMap;

use tracing_subscriber::prelude::*;
use zcash_client_backend::proto::service::{ShieldedProtocol, SubtreeRoot};

use crate::constants::REORG_MAX_BLOCKS;
use crate::net::grpc::testing::scripted_stream;
use crate::net::grpc::{GrpcError, SubtreeRootStream};
use crate::root_bind;
use crate::state::StallReason;
use crate::sync::testing::{ManualClock, bind_consistent_heights, wide_root_hash};
use crate::sync::{PoolFetch, SUBTREE_ROOT_POOLS};
use crate::tracing_guard::{
    CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
};

// ════════════════════════════════════════════════════════════════════════════
// ADAPTER: the one place the implementer's signatures are bound.
// ════════════════════════════════════════════════════════════════════════════

/// One root ingestion through the production door.
///
/// ASSUMES §3.2: `update_subtree_roots(client, tip, scanned_tip)`, with
/// `scanned_tip: Option<u32>` — the value `sync_once_body` already reads
/// (`self.scanned_tip().await?.map(|h| h.value())`) and the contract says is
/// "passed into `update_subtree_roots` beside `tip`". Return type unchanged.
async fn ingest(
    w: &Wallet,
    ep: &mut Endpoint,
    tip: u32,
    scanned_tip: Option<u32>,
) -> Result<[PoolFetch; SUBTREE_ROOT_POOLS.len()], WalletError> {
    w.update_subtree_roots(
        ep,
        zcash_protocol::consensus::BlockHeight::from_u32(tip),
        scanned_tip,
    )
    .await
}

/// The §3.1 memo for `pool`, copied into this file's own shape.
///
/// ASSUMES `Wallet::root_ingest_memo(pool)` (cfg(test)) returns the `PoolMemo`
/// by value — or anything that derefs to it — with the six §3.1 fields:
/// `verified: bool`, `chain_rewinds_seen: i64` (`rewind.observed`'s type),
/// `passes_since_full` an unsigned integer, `last_full_at: u64` (a reading of
/// `inner.clock.now_unix()`), `incremental_off: bool`, and
/// `c6_seen: BTreeMap<u64, (u32, [u8; 32])>` exactly as §3.1 spells it — and,
/// since the built-diff fold (F5), `consecutive_drops: u32`.
fn memo(w: &Wallet, pool: ShieldedProtocol) -> Memo {
    let m = w.root_ingest_memo(pool);
    Memo {
        verified: m.verified,
        chain_rewinds_seen: m.chain_rewinds_seen,
        passes_since_full: u64::from(m.passes_since_full),
        last_full_at: m.last_full_at,
        incremental_off: m.incremental_off,
        consecutive_drops: m.consecutive_drops,
        c6_seen: m
            .c6_seen
            .iter()
            .map(|(&index, &(height, hash))| (index, (height, hash)))
            .collect(),
    }
}

/// ASSUMES `Wallet::root_ingest_fallbacks()` (cfg(test)) is an unsigned count of
/// the §3.3 step 2–3 retries and fallbacks this `Inner` has taken.
fn fallbacks(w: &Wallet) -> u64 {
    w.root_ingest_fallbacks()
}

/// ASSUMES `root_bind::read_stored_run(aux, pool)` returns
/// `Result<u64, WalletError>` (it is a DB read; §1 writes the `Ok` type). If it
/// returns a bare `u64`, drop the `.expect`.
fn stored_run(w: &Wallet, pool: ShieldedProtocol) -> u64 {
    let aux = w.inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
    root_bind::read_stored_run(&aux, pool).expect("the stored run reads")
}

// ════════════════════════════════════════════════════════════════════════════
// End of ADAPTER.
// ════════════════════════════════════════════════════════════════════════════

/// The §3.1 memo in this file's shape (see [`memo`]).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Memo {
    verified: bool,
    chain_rewinds_seen: i64,
    passes_since_full: u64,
    last_full_at: u64,
    incremental_off: bool,
    /// The fold's F5 count (`SUBTREE_ROOTS_DROPS_OFF`), so a "memo untouched"
    /// assertion covers it too.
    consecutive_drops: u32,
    c6_seen: BTreeMap<u64, (u32, [u8; 32])>,
}

const SAPLING: ShieldedProtocol = ShieldedProtocol::Sapling;
const ORCHARD: ShieldedProtocol = ShieldedProtocol::Orchard;
const IRONWOOD: ShieldedProtocol = ShieldedProtocol::Ironwood;

/// Slots in `SUBTREE_ROOT_POOLS` order — pinned at compile time, so a reorder
/// there trips this file instead of silently reading another pool's receipt.
const SAP: usize = 0;
const ORC: usize = 1;
const IW: usize = 2;
const _: () = assert!(
    matches!(SUBTREE_ROOT_POOLS[SAP], ShieldedProtocol::Sapling)
        && matches!(SUBTREE_ROOT_POOLS[ORC], ShieldedProtocol::Orchard)
        && matches!(SUBTREE_ROOT_POOLS[IW], ShieldedProtocol::Ironwood),
    "s15_roots' slots must match SUBTREE_ROOT_POOLS"
);

/// The injected wall clock's starting instant for the clocked rows.
const T0: u64 = 1_757_000_000;

fn slot(pool: ShieldedProtocol) -> usize {
    SUBTREE_ROOT_POOLS
        .iter()
        .position(|p| *p == pool)
        .expect("an ingested pool")
}

/// Upstream's own table prefix (`zcash_client_sqlite`'s `*_TABLES_PREFIX`).
fn prefix(pool: ShieldedProtocol) -> &'static str {
    match pool {
        ShieldedProtocol::Sapling => "sapling",
        ShieldedProtocol::Orchard => "orchard",
        ShieldedProtocol::Ironwood => "ironwood",
    }
}

// ── Heights and scripts ─────────────────────────────────────────────────────

fn generator(pool: ShieldedProtocol) -> Vec<u32> {
    bind_consistent_heights(Network::Main, pool).expect("every mainnet window fills")
}

/// The first `n` honest mainnet Sapling completion heights. A prefix is what
/// the count bind calls harmless, so every prefix is accepted (the serve reads
/// `Withheld` — a REPORT — which no row here asserts against except T4/T2,
/// which serve the whole sequence).
fn sapling_prefix(n: usize) -> Vec<u32> {
    let all = generator(SAPLING);
    assert!(
        all.len() >= n,
        "the mainnet bundle proves {} Sapling subtrees; this fixture needs {n}",
        all.len()
    );
    all[..n].to_vec()
}

fn orchard_prefix(n: usize) -> Vec<u32> {
    let all = generator(ORCHARD);
    assert!(
        all.len() >= n,
        "the mainnet bundle proves too few Orchard subtrees"
    );
    all[..n].to_vec()
}

/// `n` Ironwood completion heights: the generator's (the bundle proves only a
/// few), then extended 2 blocks apart starting 10 above the bundle's newest
/// row. Every bundled row `(H, c)` has `c` at most the generator's length, so
/// the extension satisfies "index ≥ c ⇒ above H" for every row, and the
/// completion-gap floor (`MIN_COMPLETION_GAP_BLOCKS` = 2) holds.
fn ironwood_heights(n: usize) -> Vec<u32> {
    let mut h = generator(IRONWOOD);
    h.truncate(n);
    let above = root_bind::newest_bundled_height(Network::Main) + 10;
    let m = h.len();
    for i in m..n {
        h.push(above + 2 * u32::try_from(i - m).expect("a fixture-sized index"));
    }
    h
}

/// The endpoint's tip for a fixture: 1,000 above every served height and above
/// the bundle's newest row (so `Withheld` judges against every bundled row).
fn tip_above(seqs: &[&[u32]]) -> u32 {
    seqs.iter()
        .flat_map(|s| s.iter().copied())
        .max()
        .unwrap_or(0)
        .max(root_bind::newest_bundled_height(Network::Main))
        + 1_000
}

/// One wire root per height: `root_hash` distinct per `(pool, index)` and a
/// canonical field element in both fields (`wide_root_hash`), no completing
/// hash (C6 abstains) unless a row sets one.
fn roots_of(pool: ShieldedProtocol, heights: &[u32]) -> Vec<SubtreeRoot> {
    heights
        .iter()
        .zip(0u64..)
        .map(|(&h, i)| SubtreeRoot {
            root_hash: wide_root_hash(pool, i),
            completing_block_hash: Vec::new(),
            completing_block_height: u64::from(h),
        })
        .collect()
}

/// A 32-byte block id for `height` on chain `tag`. Byte 31 is the tag and
/// byte 0 is the height's low byte, so the id is never its own byte-reverse —
/// C6 accepts either order (`check_completing_hashes`), and a palindrome would
/// make a "lie" row pass for the wrong reason.
fn block_hash(height: u32, tag: u8) -> [u8; 32] {
    let mut v = [0u8; 32];
    v[..4].copy_from_slice(&height.to_le_bytes());
    v[31] = tag;
    v
}

// ── The endpoint double ─────────────────────────────────────────────────────

/// What the endpoint does with a request whose `start_index` is NOT zero. A
/// request from 0 is always served honestly — so every fallback meets an
/// honest server and the row measures the fallback, not a second fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OnNonzero {
    /// Serve the script from `start` (both measured servers, §1).
    Honest,
    /// Serve the script from 0 whatever `start` says (§3.3 step 3.1).
    IgnoresStart,
    /// Refuse the OPEN with a server status (§3.3 step 2, first bullet).
    RefusesOpen,
    /// Time out the OPEN (§3.3 step 2: classified as today's timeout).
    TimesOutOpen,
    /// Honour `start` but serve only this many roots (§3.3 step 3.3).
    ServesOnly(usize),
    /// Honour `start`, but the first served root's height is one higher than
    /// the one recorded there (§3.3 step 3.2).
    MovesOverlap,
    /// Honour `start`, but the first served root is a 31-byte `root_hash` —
    /// a decode fault (§3.3 step 2, third bullet).
    Malformed,
    /// Serve one root from `start`, then a mid-drain transport reset (§3.3
    /// step 2, second bullet).
    DropsMidDrain,
    /// Refuse the OPEN in the A9 shape: the server's InvalidArgument WITH
    /// trailers, which `classify_subtree_roots_status` maps to
    /// `ShieldedProtocolUnknown` — at `start > 0` a refusal of the START, never
    /// the pool-unknown outcome (the build's ruling 1).
    RefusesOpenProtocolUnknown,
    /// Serve one root from `start`, then a server InvalidArgument mid-drain (the
    /// first-pull door): with trailers it is the A9 shape the stream classifier
    /// reads as `ShieldedProtocolUnknown`, without them a plain `Status`.
    StatusMidDrain { trailers: bool },
    /// Fail the OPEN with a transport fault — a reset at the open, read like a
    /// mid-drain one (the build's ruling 2).
    TransportAtOpen,
    /// Serve one root from `start`, then STALL until the pump's per-message
    /// bound fires: a mid-drain `Timeout` (the final fold). Needs a paused
    /// tokio clock (`net::grpc::stall_testing`).
    TimesOutMidDrain,
}

/// A server-composed InvalidArgument as tonic hands it to the stream pump:
/// source-less, and — when `trailers` — carrying response metadata, the
/// discriminator `classify_subtree_roots_status` reads as A9's refusal.
fn invalid_argument(trailers: bool) -> tonic::Status {
    let mut metadata = tonic::metadata::MetadataMap::new();
    if trailers {
        metadata.insert(
            "grpc-server",
            tonic::metadata::MetadataValue::from_static("lightwalletd"),
        );
    }
    tonic::Status::with_metadata(
        tonic::Code::InvalidArgument,
        "unknown shielded protocol",
        metadata,
    )
}

/// A side effect run at every stream OPEN, after the request is recorded and
/// before anything is served — the "between the plan and the locked section"
/// window T23 needs.
type OpenHook = Box<dyn FnMut(ShieldedProtocol, u32) + Send>;

struct Endpoint {
    /// The pool's whole honest sequence, index `i` at position `i`.
    scripts: [Vec<SubtreeRoot>; 3],
    on_nonzero: [OnNonzero; 3],
    /// Every `(pool, start)` asked for since the last [`Endpoint::take_asked`].
    asked: Vec<(ShieldedProtocol, u32)>,
    on_open: Option<OpenHook>,
}

impl Endpoint {
    fn serving(sapling: &[u32], orchard: &[u32], ironwood: &[u32]) -> Self {
        Self {
            scripts: [
                roots_of(SAPLING, sapling),
                roots_of(ORCHARD, orchard),
                roots_of(IRONWOOD, ironwood),
            ],
            on_nonzero: [OnNonzero::Honest; 3],
            asked: Vec::new(),
            on_open: None,
        }
    }

    /// The starts asked for since the last call, per pool slot, then cleared.
    fn take_asked(&mut self) -> [Vec<u32>; 3] {
        let mut out: [Vec<u32>; 3] = Default::default();
        for (pool, start) in self.asked.drain(..) {
            out[slot(pool)].push(start);
        }
        out
    }

    fn items_from(&self, s: usize, from: usize) -> Vec<Result<Option<SubtreeRoot>, tonic::Status>> {
        self.scripts[s]
            .get(from..)
            .unwrap_or(&[])
            .iter()
            .cloned()
            .map(|r| Ok(Some(r)))
            .collect()
    }
}

#[async_trait::async_trait]
impl sync::SubtreeRootSource for Endpoint {
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        self.asked.push((protocol, start_index));
        if let Some(hook) = self.on_open.as_mut() {
            hook(protocol, start_index);
        }
        let s = slot(protocol);
        let from = usize::try_from(start_index).expect("a u32 fits a usize");
        if start_index == 0 {
            return Ok(scripted_stream(self.items_from(s, 0)));
        }
        let items = match self.on_nonzero[s] {
            OnNonzero::Honest => self.items_from(s, from),
            OnNonzero::IgnoresStart => self.items_from(s, 0),
            OnNonzero::RefusesOpen => {
                // A server-composed InvalidArgument, post-classification: a
                // STATUS, not the A9 protocol refusal (that one is about the
                // pool, and this pool is served from 0 on the retry).
                return Err(GrpcError::Status { code: 3 });
            }
            OnNonzero::TimesOutOpen => {
                return Err(GrpcError::Timeout {
                    stall: StallReason::EndpointUnreachable,
                });
            }
            OnNonzero::ServesOnly(n) => self.items_from(s, from).into_iter().take(n).collect(),
            OnNonzero::MovesOverlap => {
                let mut v = self.items_from(s, from);
                if let Some(Ok(Some(first))) = v.first_mut() {
                    first.completing_block_height += 1;
                }
                v
            }
            OnNonzero::Malformed => {
                let mut v = self.items_from(s, from);
                if let Some(Ok(Some(first))) = v.first_mut() {
                    first.root_hash.truncate(31);
                }
                v
            }
            OnNonzero::DropsMidDrain => {
                let mut v: Vec<_> = self.items_from(s, from).into_iter().take(1).collect();
                v.push(Err(tonic::Status::unavailable("circuit reset mid-drain")));
                v
            }
            OnNonzero::RefusesOpenProtocolUnknown => {
                return Err(GrpcError::ShieldedProtocolUnknown { code: 3 });
            }
            OnNonzero::StatusMidDrain { trailers } => {
                let mut v: Vec<_> = self.items_from(s, from).into_iter().take(1).collect();
                v.push(Err(invalid_argument(trailers)));
                v
            }
            OnNonzero::TransportAtOpen => {
                return Err(GrpcError::Transport {
                    stall: StallReason::EndpointUnreachable,
                });
            }
            OnNonzero::TimesOutMidDrain => {
                let first: Vec<_> = self.items_from(s, from).into_iter().take(1).collect();
                return Ok(crate::net::grpc::stall_testing::stalling_stream(first));
            }
        };
        Ok(scripted_stream(items))
    }
}

// ── Wallet, passes, observation ─────────────────────────────────────────────

/// A fresh MAINNET wallet (no account — the commitment trees are global, the
/// `update_subtree_roots_*` rows' shape) on the system clock.
async fn main_wallet(dir: &std::path::Path) -> (Wallet, Arc<dyn KeychainPort>) {
    let vault = test_vault();
    let w = Wallet::create_with_vault(
        cfg(dir, Network::Main, SeedPersistence::SealedKeychain),
        raw_seed(),
        Arc::clone(&vault),
    )
    .await
    .expect("create");
    (w, vault)
}

/// [`main_wallet`] REOPENED on `clock` (the P-G2 seam is the only way a clock
/// reaches `Inner`). The reopen is a new `Inner` — so a fresh memo — before
/// any root is ingested.
async fn main_wallet_on(dir: &std::path::Path, clock: &ManualClock) -> Wallet {
    let (w, vault) = main_wallet(dir).await;
    w.close().await.expect("close after create");
    Wallet::open_with_vault_and_seed_port(
        cfg(dir, Network::Main, SeedPersistence::SealedKeychain),
        vault,
        None,
        Some(clock.port()),
    )
    .await
    .expect("reopen on the injected clock")
}

/// What one pass did: its result and the starts the endpoint was asked for.
struct Pass {
    result: Result<[PoolFetch; SUBTREE_ROOT_POOLS.len()], WalletError>,
    asked: [Vec<u32>; 3],
}

impl Pass {
    fn ok(&self, what: &str) -> [PoolFetch; SUBTREE_ROOT_POOLS.len()] {
        match &self.result {
            Ok(o) => *o,
            Err(e) => panic!("{what}: expected Ok, got {e:?} (asked {:?})", self.asked),
        }
    }
}

/// One pass with the scanned tip at the endpoint's tip (the window term absent).
async fn pass(w: &Wallet, ep: &mut Endpoint, tip: u32) -> Pass {
    pass_scanned(w, ep, tip, Some(tip)).await
}

async fn pass_scanned(w: &Wallet, ep: &mut Endpoint, tip: u32, scanned_tip: Option<u32>) -> Pass {
    let result = ingest(w, ep, tip, scanned_tip).await;
    Pass {
        result,
        asked: ep.take_asked(),
    }
}

fn exec(w: &Wallet, sql: &str, params: impl rusqlite::Params) {
    let aux = w.inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
    aux.execute(sql, params)
        .unwrap_or_else(|e| panic!("{sql}: {e}"));
}

/// Upstream's own shard row: `(subtree_end_height, root_hash)`, `None` when
/// no row exists at `index`.
fn shard_row(
    w: &Wallet,
    pool: ShieldedProtocol,
    index: i64,
) -> Option<(Option<u32>, Option<Vec<u8>>)> {
    let aux = w.inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
    let sql = format!(
        "SELECT subtree_end_height, root_hash FROM {}_tree_shards WHERE shard_index = ?1",
        prefix(pool)
    );
    match aux.query_row(&sql, [index], |r| {
        Ok((r.get::<_, Option<u32>>(0)?, r.get::<_, Option<Vec<u8>>>(1)?))
    }) {
        Ok(row) => Some(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => panic!("reading {} shard {index}: {e}", prefix(pool)),
    }
}

/// A scanned block, as the scan leaves it for the two readers that matter
/// here (`blocks.hash` at a height). Every tree-size column NULL, so the
/// scanned-count oracle (d) abstains and cannot be what refuses a C6 row.
fn scan_block(w: &Wallet, height: u32, hash: [u8; 32]) {
    exec(
        w,
        "INSERT INTO blocks (height, hash, time, sapling_tree) VALUES (?1, ?2, 0, X'')",
        rusqlite::params![height, hash.to_vec()],
    );
}

fn rescan_block(w: &Wallet, height: u32, hash: [u8; 32]) {
    exec(
        w,
        "UPDATE blocks SET hash = ?2 WHERE height = ?1",
        rusqlite::params![height, hash.to_vec()],
    );
}

fn bracket_exists(w: &Wallet, pool: ShieldedProtocol, index: i64) -> bool {
    let aux = w.inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
    aux.query_row(
        "SELECT EXISTS(SELECT 1 FROM subtree_boundary_bound WHERE pool = ?1 AND shard_index = ?2)",
        rusqlite::params![prefix(pool), index],
        |r| r.get::<_, bool>(0),
    )
    .expect("the bracket table reads")
}

/// The rewind ledger's chain counter, through the production reader.
fn chain_rewinds(w: &Wallet) -> i64 {
    let aux = w.inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
    root_bind::read_rewind_watch(&aux, SAPLING)
        .expect("the ledger reads")
        .observed
}

/// A chain rewind as `sync::rewind_wallet_to` records it (the note half).
fn note_a_rewind(w: &Wallet) {
    let aux = w.inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
    root_bind::note_rewind(&aux).expect("note the rewind");
}

/// Mark `pool`'s heights as recorded at chain-rewind count `at` (the consume
/// half, or — with an older `at` — the R20 "written but still armed" state).
fn record_pool_at(w: &Wallet, pool: ShieldedProtocol, at: i64) {
    let aux = w.inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
    root_bind::note_roots_recorded(&aux, pool, at).expect("record the pool");
}

/// A thread-local capture of every `zec_wallet_core` field until the guard
/// drops (the `tracing_guard` harness, as `anchor_proof` installs it).
fn capture() -> (CapturedEvents, tracing::subscriber::DefaultGuard) {
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
    let guard = tracing::subscriber::set_default(subscriber);
    (sink, guard)
}

/// Did any captured record carry `outcome = code`? Read off the flattened
/// field list, so the warn may ride any event name. ASSUMES the §3.3 step 7
/// warns are emitted on the pass's async path (where the retry happens), not
/// inside a `run_blocking` section on another thread.
fn saw_outcome(sink: &CapturedEvents, code: &str) -> bool {
    sink.fields()
        .iter()
        .any(|(name, value)| name == "outcome" && value == code)
}

/// §5.4-clean, except the codebase's ONE exempt field: `transport_chain`, the
/// free-text transport error chain of `net/grpc.rs`'s debug diagnostic. It is
/// never allowlisted (so the device log withholds it), and
/// `extraction_policy::every_info_and_above_field_name_in_the_sdk_is_allowlisted`
/// pins that it is emitted at exactly one site, inside `#[cfg(debug_assertions)]`
/// — compiled out of every build that ships. Only a row whose fake hands a
/// tonic `Status` to the stream classifier (`DropsMidDrain`, `StatusMidDrain`)
/// meets it in this (debug) test build; every other field is graded. A row
/// whose fake answers with a `GrpcError` directly never reaches the classifier
/// and is graded whole (`assert_5_4_clean`).
fn assert_clean_but_the_debug_diagnostic(sink: &CapturedEvents) {
    let fields: Vec<(String, String)> = sink
        .fields()
        .into_iter()
        .filter(|(name, _)| name != "transport_chain")
        .collect();
    assert_5_4_clean(&fields);
}

/// The Ironwood bind refusal a row expects, by its stable code.
fn ironwood_refused_by(code: &'static str) -> PoolFetch {
    PoolFetch::HeightViolation { code }
}

fn c6_code() -> &'static str {
    root_bind::HeightBindRefusal::CompletingBlockHash {
        index: 0,
        height: 0,
    }
    .code()
}

fn gap_code() -> &'static str {
    root_bind::HeightBindRefusal::CompletionGap {
        index: 0,
        previous: 0,
        height: 0,
    }
    .code()
}

// ════════════════════════════════════════════════════════════════════════════
// The named rows (§7). Names are the contract's; a rename is a finding.
// ════════════════════════════════════════════════════════════════════════════

/// **T1** — §3.2 term 1 and the floor: an `Inner` that has verified a pool
/// asks next time only from `floor64(count − 1)`.
///
/// Two fixtures, per §7's preamble: `count = 193` puts `count − 1 = 192` on
/// `≡ 0`, where a `count − 2` mutant reads 191 → 128; `count = 192` puts it on
/// `≡ 63`, where a `start = count` mutant reads 192 → 192. Also asserts zero
/// fallbacks (the incremental path took none) and that `read_stored_run` reads
/// the whole run (`count` is what the plan is built from).
///
/// Mutants: the plan always `Full` (asked `[0]`); `start = count` (row 2);
/// `start = count − 2` (row 1).
#[tokio::test]
async fn a_second_pass_asks_only_from_the_last_stored_root() {
    for (count, expected) in [(193usize, 192u32), (192, 128)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (w, _vault) = main_wallet(dir.path()).await;
        let heights = sapling_prefix(count);
        let tip = tip_above(&[&heights]);
        let mut ep = Endpoint::serving(&heights, &[], &[]);

        let first = pass(&w, &mut ep, tip).await;
        first.ok("the first pass");
        assert_eq!(
            first.asked[SAP],
            vec![0],
            "count {count}: an unverified pool is fetched from 0"
        );
        assert_eq!(
            stored_run(&w, SAPLING),
            count as u64,
            "the whole serve is the stored run"
        );
        let before = fallbacks(&w);

        let second = pass(&w, &mut ep, tip).await;
        second.ok("the second pass");
        assert_eq!(
            second.asked[SAP],
            vec![expected],
            "count {count}: floor64(count − 1) = {expected}, asked exactly once"
        );
        assert_eq!(
            fallbacks(&w),
            before,
            "count {count}: the incremental path takes no fallback"
        );
        w.close().await.expect("close");
    }
}

/// **T2** — §3.3 step 3.1: a server that ignores `start_index` and serves from 0
/// is RE-CLASSIFIED as a full fetch — one stream, no second request; the
/// absolute `Served` count is the served length (not `start + len`); the memo
/// records a full verification (`verified`, `passes_since_full = 0`,
/// `last_full_at = now`) and `incremental_off`; outcome `roots_start_ignored`.
///
/// Serves the WHOLE mainnet Sapling sequence so the outcome is `Served { m }`
/// exactly (a prefix would read `Withheld`, which says nothing about the count).
///
/// Mutants: the from-0 serve re-fetched (asked `[s, 0]`); refused (`Err`);
/// over-counted (`Served { s + m }`); `last_full_at`/`passes_since_full` not
/// reset; `incremental_off` not set (the next pass asks `[s]` again).
#[tokio::test]
async fn an_endpoint_that_ignores_start_index_is_taken_as_full() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clock = ManualClock::at(T0);
    let w = main_wallet_on(dir.path(), &clock).await;
    let heights = generator(SAPLING);
    let m = heights.len();
    assert!(
        m > 128,
        "the fixture needs more than two 64-blocks; mainnet proves {m}"
    );
    let start = u32::try_from((m - 1) / 64 * 64).expect("small");
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);

    pass(&w, &mut ep, tip).await.ok("full");
    let incremental = pass(&w, &mut ep, tip).await;
    incremental.ok("incremental");
    assert_eq!(
        incremental.asked[SAP],
        vec![start],
        "the control: incremental before the fault"
    );
    assert_eq!(memo(&w, SAPLING).passes_since_full, 1);

    clock.advance(10);
    ep.on_nonzero[SAP] = OnNonzero::IgnoresStart;
    let (sink, _guard) = capture();
    let ignored = pass(&w, &mut ep, tip).await;
    let outcomes = ignored.ok("an ignored start is a full serve, not a fault");
    assert_eq!(
        ignored.asked[SAP],
        vec![start],
        "ONE stream — the from-0 serve is not re-fetched"
    );
    assert_eq!(
        outcomes[SAP],
        PoolFetch::Served { roots: m },
        "the absolute count is the served length, never start + len"
    );
    let after = memo(&w, SAPLING);
    assert!(
        after.verified,
        "a re-classified serve is a full, bound fetch"
    );
    assert_eq!(after.passes_since_full, 0, "and resets the pass count");
    assert_eq!(
        after.last_full_at,
        T0 + 10,
        "and the full-verification clock"
    );
    assert!(
        after.incremental_off,
        "and this server is not asked incrementally again"
    );
    assert!(
        saw_outcome(&sink, "roots_start_ignored"),
        "the re-classification is said"
    );
    assert_5_4_clean(&sink.fields());

    let next = pass(&w, &mut ep, tip).await;
    next.ok("after the ignore");
    assert_eq!(
        next.asked[SAP],
        vec![0],
        "incremental_off holds for the rest of this Inner"
    );
    w.close().await.expect("close");
}

/// **T2, second clause** — "a pool near the cap that counts from 0": with
/// `count = 40,000` the plan asks from 39,936, and a server that ignores it
/// serves 40,000 roots. `39,936 + 40,000 ≥ 65,536`, so a flood cap held to the
/// REQUESTED start refuses an honest full serve; the re-classification
/// (`start := 0` for the flood cap, §3.3 step 3.1) accepts it.
///
/// Ironwood, extended above the bundle (see the module doc). Heavy by design:
/// two 40,000-root puts.
///
/// Mutant: the absolute cap computed against the requested start (`Err`).
#[tokio::test]
async fn an_endpoint_that_ignores_start_index_is_taken_as_full_near_the_cap() {
    const COUNT: usize = 40_000;
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = ironwood_heights(COUNT);
    let tip = tip_above(&[&heights]);
    let start = u32::try_from((COUNT - 1) / 64 * 64).expect("small");
    assert!(
        u64::from(start) + COUNT as u64 >= u64::from(crate::constants::MAX_SUBTREE_ROOTS_PER_POOL),
        "the fixture must put start + served past the cap, or the row proves nothing"
    );
    let mut ep = Endpoint::serving(&[], &[], &heights);
    pass(&w, &mut ep, tip)
        .await
        .ok("the full 40,000-root serve");

    ep.on_nonzero[IW] = OnNonzero::IgnoresStart;
    let ignored = pass(&w, &mut ep, tip).await;
    let outcomes = ignored.ok("an ignored start counts from 0 for the flood cap");
    assert_eq!(ignored.asked[IW], vec![start]);
    assert_eq!(outcomes[IW], PoolFetch::Served { roots: COUNT });
    assert!(memo(&w, IRONWOOD).incremental_off);
    w.close().await.expect("close");
}

/// **T3** — §3.3 step 3.3: an incremental serve that leaves the stored run
/// short (`start + served < count`, the empty serve included) falls back to a
/// full fetch in the same pass, says `roots_served_short`, and turns
/// `incremental_off` on.
///
/// Rows: 3 roots from 192 of a 200-root run (195 < 200); and 0 roots.
///
/// Mutant: the `start + len < count` fallback removed (asked `[192]` and the
/// pass accepts a short incremental serve).
#[tokio::test]
async fn a_short_incremental_serve_falls_back_to_full() {
    for served in [3usize, 0] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (w, _vault) = main_wallet(dir.path()).await;
        let heights = sapling_prefix(200);
        let tip = tip_above(&[&heights]);
        let mut ep = Endpoint::serving(&heights, &[], &[]);
        pass(&w, &mut ep, tip).await.ok("full");
        let before = fallbacks(&w);

        ep.on_nonzero[SAP] = OnNonzero::ServesOnly(served);
        let (sink, guard) = capture();
        let short = pass(&w, &mut ep, tip).await;
        drop(guard);
        short.ok("a short serve is repaired in the same pass");
        assert_eq!(
            short.asked[SAP],
            vec![192, 0],
            "served {served}: one incremental try, one full"
        );
        assert!(
            saw_outcome(&sink, "roots_served_short"),
            "served {served}: said"
        );
        assert_5_4_clean(&sink.fields());
        assert!(
            memo(&w, SAPLING).incremental_off,
            "served {served}: remembered"
        );
        assert!(
            fallbacks(&w) > before,
            "served {served}: counted as a fallback"
        );

        let next = pass(&w, &mut ep, tip).await;
        next.ok("next");
        assert_eq!(
            next.asked[SAP],
            vec![0],
            "served {served}: one stream per pass from now on"
        );
        w.close().await.expect("close");
    }
}

/// **T3b** — §3.3 step 3.2: the first served root's height is not the one
/// recorded at `start` (`overlap_height`) — fall back from 0, say
/// `roots_overlap_moved`, `incremental_off`. The recorded height at 192 is
/// untouched (nothing from the moved serve was written).
///
/// Mutant: the overlap-height fallback removed (the moved height reaches the
/// bind as a recorded-height violation — fatal for Sapling — or, with the bind
/// relaxed, is written).
#[tokio::test]
async fn an_overlap_whose_height_moved_falls_back_to_full() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");

    ep.on_nonzero[SAP] = OnNonzero::MovesOverlap;
    let (sink, guard) = capture();
    let moved = pass(&w, &mut ep, tip).await;
    drop(guard);
    moved.ok("a moved overlap is repaired in the same pass");
    assert_eq!(moved.asked[SAP], vec![192, 0]);
    assert!(saw_outcome(&sink, "roots_overlap_moved"));
    assert_5_4_clean(&sink.fields());
    assert!(memo(&w, SAPLING).incremental_off);
    assert_eq!(
        shard_row(&w, SAPLING, 192).and_then(|(h, _)| h),
        Some(heights[192]),
        "the recorded overlap height is the honest one"
    );
    let next = pass(&w, &mut ep, tip).await;
    next.ok("next");
    assert_eq!(next.asked[SAP], vec![0]);
    w.close().await.expect("close");
}

/// **T3c** — §3.3 step 2, third bullet: a DECODE fault in a `start > 0` stream
/// re-fetches the pool from 0 in the same pass, says `roots_start_failed`, and
/// turns `incremental_off` on — so a server that cannot serve incrementally
/// costs one stream per pass, not two.
///
/// Mutants: the decode fault retried every pass (the next pass asks
/// `[192, 0]`); not retried at all (`Err`).
#[tokio::test]
async fn a_malformed_incremental_stream_falls_back_and_turns_incremental_off() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");

    ep.on_nonzero[SAP] = OnNonzero::Malformed;
    let (sink, guard) = capture();
    let failed = pass(&w, &mut ep, tip).await;
    drop(guard);
    failed.ok("a malformed incremental stream is repaired from 0");
    assert_eq!(failed.asked[SAP], vec![192, 0]);
    assert!(saw_outcome(&sink, "roots_start_failed"));
    assert_5_4_clean(&sink.fields());
    assert!(memo(&w, SAPLING).incremental_off);

    let next = pass(&w, &mut ep, tip).await;
    next.ok("next");
    assert_eq!(next.asked[SAP], vec![0], "never two streams per pass");
    w.close().await.expect("close");
}

/// **T3c, second clause** — §3.3 step 2's first sentence: "the bind's and
/// validation's refusals (step 4) are reported exactly as today, never
/// retried". An Ironwood incremental serve carrying a NEW root one block after
/// the last (the completion-gap floor) is refused with its code — Ironwood's
/// instrument, the other two pools' refusal is a code-less `Err` — and the
/// endpoint is asked exactly once.
///
/// Mutant: a bind refusal swallowed by the retry (asked `[192, 0]`, and the
/// from-0 serve — which carries the same lie — is what gets reported, or the
/// refusal is turned into `incremental_off`).
#[tokio::test]
async fn a_bind_refusal_at_a_nonzero_start_is_reported_not_retried() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = ironwood_heights(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&[], &[], &heights);
    pass(&w, &mut ep, tip).await.ok("full");
    let before = fallbacks(&w);

    let mut lying = heights.clone();
    lying.push(heights[199] + 1);
    ep.scripts[IW] = roots_of(IRONWOOD, &lying);
    let refused = pass(&w, &mut ep, tip).await;
    let outcomes = refused.ok("an Ironwood refusal is a recorded outcome");
    assert_eq!(refused.asked[IW], vec![192], "never retried");
    assert_eq!(
        outcomes[IW],
        ironwood_refused_by(gap_code()),
        "reported as today"
    );
    assert_eq!(fallbacks(&w), before, "a refusal is not a fallback");
    assert!(
        !memo(&w, IRONWOOD).incremental_off,
        "and says nothing about start_index support"
    );
    assert_eq!(
        shard_row(&w, IRONWOOD, 200),
        None,
        "and nothing of it is written"
    );
    w.close().await.expect("close");
}

/// **T4** — §3.3 step 6: `PoolFetch::Served` carries the ABSOLUTE count
/// (`start + served.len()`), which `withheld` compares against the bundle's
/// proof. A full mainnet Sapling sequence served incrementally must read
/// `Served { m }`, not `Withheld`.
///
/// Mutant: a relative count (`Served { m − start }` < proven ⇒ `Withheld`).
#[tokio::test]
async fn an_incremental_serve_that_completes_the_proof_is_not_withheld() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = generator(SAPLING);
    let m = heights.len();
    let start = u32::try_from((m - 1) / 64 * 64).expect("small");
    assert!(start > 0, "the row needs an incremental start");
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    let full = pass(&w, &mut ep, tip).await;
    assert_eq!(
        full.ok("full")[SAP],
        PoolFetch::Served { roots: m },
        "the control"
    );

    let inc = pass(&w, &mut ep, tip).await;
    let outcomes = inc.ok("incremental");
    assert_eq!(
        inc.asked[SAP],
        vec![start],
        "non-vacuity: the serve WAS incremental"
    );
    assert_eq!(outcomes[SAP], PoolFetch::Served { roots: m });
    w.close().await.expect("close");
}

/// **T5** — §3.3 step 4.4 / ledger row 13: `put_*_subtree_roots(start, served)`
/// lands each root at ITS OWN index. Read back from upstream's table: every
/// index 0..206 carries its own `root_hash` and height after a 200-root full
/// write and a growing incremental serve from 192.
///
/// Mutant: `put_*(start ± 1)` — a shifted put writes root `i` at `i ± 1`, where
/// the cap already holds a different root (`Conflict` ⇒ `Err`), or (above the
/// old run) at the wrong index (the readback).
#[tokio::test]
async fn an_incremental_write_lands_each_root_at_its_own_index() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let grown = sapling_prefix(206);
    let tip = tip_above(&[&grown]);
    let mut ep = Endpoint::serving(&grown[..200], &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");

    ep.scripts[SAP] = roots_of(SAPLING, &grown);
    let before = fallbacks(&w);
    let inc = pass(&w, &mut ep, tip).await;
    inc.ok("a growing incremental serve");
    assert_eq!(inc.asked[SAP], vec![192]);
    assert_eq!(fallbacks(&w), before);
    for (i, &h) in grown.iter().enumerate() {
        let index = i64::try_from(i).expect("small");
        assert_eq!(
            shard_row(&w, SAPLING, index),
            Some((Some(h), Some(wide_root_hash(SAPLING, index as u64)))),
            "index {i} holds its own height and its own root"
        );
    }
    assert_eq!(
        shard_row(&w, SAPLING, 206),
        None,
        "and nothing past the serve"
    );
    w.close().await.expect("close");
}

/// **T6a** — §3.2 `Rewound`, input 1: `rewind.rewound_since_record()` for the
/// pool forces a full fetch — PER POOL. The state is R20's "written but still
/// armed" (a consume that failed after a put): the chain counter equals what
/// the memo saw, only Sapling's record is behind it. Ironwood, beside it, stays
/// incremental.
///
/// Mutant: the pool's rewind input ignored (Sapling asks `[192]`; with a rewind
/// outstanding at `start > 0` the bind returns `Internal`, §3.3 step 4.3).
#[tokio::test]
async fn a_pool_rewind_forces_a_full_refetch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let sap = sapling_prefix(200);
    let iw = ironwood_heights(200);
    let tip = tip_above(&[&sap, &iw]);
    let mut ep = Endpoint::serving(&sap, &[], &iw);
    // One rewind before anything is recorded: pass 1 is relaxed (and consumes
    // it), pass 2 is the first BOUND full fetch, pass 3 the incremental control.
    note_a_rewind(&w);
    pass(&w, &mut ep, tip).await.ok("relaxed full");
    pass(&w, &mut ep, tip).await.ok("bound full");
    let control = pass(&w, &mut ep, tip).await;
    control.ok("control");
    assert_eq!(control.asked[SAP], vec![192]);
    assert_eq!(control.asked[IW], vec![192]);
    assert_eq!(memo(&w, SAPLING).chain_rewinds_seen, chain_rewinds(&w));

    record_pool_at(&w, SAPLING, chain_rewinds(&w) - 1);
    let rewound = pass(&w, &mut ep, tip).await;
    rewound.ok("a pool rewind is a full, relaxed fetch");
    assert_eq!(
        rewound.asked[SAP],
        vec![0],
        "the rewound pool is fetched from 0"
    );
    assert_eq!(rewound.asked[IW], vec![192], "the other pool is not");
    w.close().await.expect("close");
}

/// **T6b** — §3.2 `Rewound`, input 2: `rewind.observed != memo.chain_rewinds_seen`
/// forces a full fetch even when every pool's record has caught up (no bind is
/// relaxed). Then the memo takes the new count and the next pass is incremental.
///
/// Mutant: the chain-counter input ignored (asked `[192]` on both pools).
#[tokio::test]
async fn a_chain_rewind_forces_a_full_refetch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let sap = sapling_prefix(200);
    let iw = ironwood_heights(200);
    let tip = tip_above(&[&sap, &iw]);
    let mut ep = Endpoint::serving(&sap, &[], &iw);
    pass(&w, &mut ep, tip).await.ok("full");
    let control = pass(&w, &mut ep, tip).await;
    control.ok("control");
    assert_eq!(control.asked[SAP], vec![192]);

    note_a_rewind(&w);
    let now = chain_rewinds(&w);
    for pool in SUBTREE_ROOT_POOLS {
        record_pool_at(&w, *pool, now);
    }
    let after = pass(&w, &mut ep, tip).await;
    after.ok("a bound full fetch after a chain rewind");
    assert_eq!(after.asked[SAP], vec![0]);
    assert_eq!(after.asked[IW], vec![0]);
    assert_eq!(
        memo(&w, SAPLING).chain_rewinds_seen,
        now,
        "the memo takes the new count"
    );

    let again = pass(&w, &mut ep, tip).await;
    again.ok("again");
    assert_eq!(again.asked[SAP], vec![192], "and incremental resumes");
    w.close().await.expect("close");
}

/// **T7** — §3.2 term 2: every root above `window_floor = min(tip,
/// scanned_tip) − REORG_MAX_BLOCKS` is re-served — a reorg ABOVE the scanned
/// tip, where no rewind fires (ledger row 6).
///
/// Rows (count 200, so term 1 alone is 192):
/// * `recorded[127] == window_floor` exactly ⇒ first index above it is 128 ⇒
///   start 128 (a `>=` mutant reads 127 ⇒ 64);
/// * first index above the floor is 191 ⇒ start 128 (an index shifted up reads
///   192 ⇒ 192).
///
/// The endpoint's tip stays far above, so the window exists only through
/// `scanned_tip` — the "anchored on `tip` alone" mutant reads 192 in both rows.
///
/// Mutants: the window term dropped; `>` vs `>=`; the window anchored on `tip`.
#[tokio::test]
async fn a_root_inside_the_reorg_window_is_re_served() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    let control = pass(&w, &mut ep, tip).await;
    control.ok("control");
    assert_eq!(
        control.asked[SAP],
        vec![192],
        "no window with the scan at the tip"
    );

    for (first_in_window, expected) in [(128usize, 128u32), (191, 128)] {
        let scanned_tip = heights[first_in_window - 1] + REORG_MAX_BLOCKS;
        assert!(
            scanned_tip < tip,
            "the window must come from the scanned tip"
        );
        let p = pass_scanned(&w, &mut ep, tip, Some(scanned_tip)).await;
        p.ok("a window pass");
        assert_eq!(
            p.asked[SAP],
            vec![expected],
            "first index above the floor {first_in_window} ⇒ start {expected}"
        );
    }
    w.close().await.expect("close");
}

/// **T8** — §3.2 term 4 and §3.1 `c6_seen`: a completing block scanned AFTER
/// its root was written is hash-checked on the next pass, at its own index —
/// Ironwood, whose C6 refusal comes back as `HeightViolation` with its code.
///
/// The sequence (Ironwood, 200 roots; index 70 sits in block 64):
/// 1. full, nothing scanned (C6 abstains, nothing in `c6_seen`);
/// 2. incremental `[192]` — the control;
/// 3. block `recorded[70]` scanned as X, the endpoint's hash for 70 is a lie ⇒
///    asked `[64]`, refused `completing_block_hash`;
/// 4. the endpoint now serves X ⇒ asked `[64]` again (the refused pass wrote
///    nothing, so the memo is unchanged), accepted, `c6_seen[70] = (h, X)`;
/// 5. asked `[192]` — the index is seen;
/// 6. the block re-scanned as X2 and the endpoint serves NO hash ⇒ asked `[64]`
///    (the seen entry no longer matches the scanned hash), accepted — an
///    abstain on a missing hash;
/// 7. asked `[192]` — the abstain was recorded.
///
/// Mutants: term 4 dropped (3: `[192]` and `Served`); `c6_seen` not filled on a
/// compare (5: `[64]`); a missing-hash abstain not recorded (7: `[64]`); the
/// entry not keyed by the scanned hash (6: `[192]`).
#[tokio::test]
async fn a_newly_scanned_completing_block_is_hash_checked_on_the_next_pass() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let iw = ironwood_heights(200);
    let tip = tip_above(&[&iw]);
    let h70 = iw[70];
    let x = block_hash(h70, 0xA1);
    let lie = block_hash(h70, 0xB2);
    let x2 = block_hash(h70, 0xC3);
    let mut ep = Endpoint::serving(&[], &[], &iw);
    ep.scripts[IW][70].completing_block_hash = lie.to_vec();

    pass(&w, &mut ep, tip).await.ok("1: full, nothing scanned");
    let p2 = pass(&w, &mut ep, tip).await;
    p2.ok("2");
    assert_eq!(p2.asked[IW], vec![192], "2: the control");

    scan_block(&w, h70, x);
    let p3 = pass(&w, &mut ep, tip).await;
    let o3 = p3.ok("3: an Ironwood refusal is an outcome");
    assert_eq!(
        p3.asked[IW],
        vec![64],
        "3: the newly scanned index is re-served"
    );
    assert_eq!(
        o3[IW],
        ironwood_refused_by(c6_code()),
        "3: and C6 refuses the lie"
    );

    ep.scripts[IW][70].completing_block_hash = x.to_vec();
    let p4 = pass(&w, &mut ep, tip).await;
    p4.ok("4");
    assert_eq!(
        p4.asked[IW],
        vec![64],
        "4: the refused pass left the memo as it was"
    );
    assert_eq!(
        memo(&w, IRONWOOD).c6_seen.get(&70),
        Some(&(h70, x)),
        "4: C6 compared at 70, keyed by the scanned hash"
    );
    let p5 = pass(&w, &mut ep, tip).await;
    p5.ok("5");
    assert_eq!(p5.asked[IW], vec![192], "5: a seen index is not re-served");

    rescan_block(&w, h70, x2);
    ep.scripts[IW][70].completing_block_hash = Vec::new();
    let p6 = pass(&w, &mut ep, tip).await;
    p6.ok("6: a missing hash abstains");
    assert_eq!(
        p6.asked[IW],
        vec![64],
        "6: the scanned hash changed, so 70 is re-served"
    );
    let p7 = pass(&w, &mut ep, tip).await;
    p7.ok("7");
    assert_eq!(
        p7.asked[IW],
        vec![192],
        "7: a missing-hash abstain is recorded as seen"
    );
    w.close().await.expect("close");
}

/// **T9** — §3.2 term 3 and §3.3 step 4.5: a boundary bracket the scan minted
/// at index 128 anchors the start below it (`lowest bracket − 1 = 127` ⇒ 64),
/// and the honest re-serve — inside the bracket — withdraws it
/// (`clear_boundary_bounds_in` over the written range).
///
/// Index 128 is on `≡ 0`: a mutant that drops the `− 1` reads 128 ⇒ 128.
///
/// Mutants: term 3 dropped (asked `[192]`, the bracket outside the written range
/// is never withdrawn); the `− 1` dropped (asked `[128]`).
#[tokio::test]
async fn a_bracket_below_the_overlap_is_re_served_and_withdrawn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");

    exec(
        &w,
        "INSERT INTO subtree_boundary_bound (pool, shard_index, floor_height, ceiling_height)
         VALUES ('sapling', 128, ?1, ?2)",
        rusqlite::params![heights[128] - 1, heights[128]],
    );
    assert!(bracket_exists(&w, SAPLING, 128), "fixture");
    let p = pass(&w, &mut ep, tip).await;
    p.ok("the honest height is inside the bracket");
    assert_eq!(p.asked[SAP], vec![64], "lowest bracket − 1 = 127 ⇒ 64");
    assert!(
        !bracket_exists(&w, SAPLING, 128),
        "the write that covered it withdrew it"
    );
    let next = pass(&w, &mut ep, tip).await;
    next.ok("next");
    assert_eq!(next.asked[SAP], vec![192], "and with it gone, term 1 alone");
    w.close().await.expect("close");
}

/// **T10a** — §3.1 / ledger row 10: a server switch builds a new `Inner`, and a
/// new `Inner` starts unverified: the first pass after the switch is full.
///
/// Mutant: the memo carried across the switch (`Carried`) — asked `[192]`.
#[tokio::test]
async fn a_server_switch_starts_full() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    let control = pass(&w, &mut ep, tip).await;
    control.ok("control");
    assert_eq!(control.asked[SAP], vec![192]);

    let mut oracle = provision::testing::FakeOracle::honest(Network::Main, 3_482_911);
    let w = match w
        .switch_sync_server_inner(custom("https://other.example:443"), Some(&mut oracle))
        .await
    {
        Ok(w) => w,
        Err(refused) => panic!("the switch must succeed: {:?}", refused.error),
    };
    assert!(
        !memo(&w, SAPLING).verified,
        "a new Inner has verified nothing"
    );
    let after = pass(&w, &mut ep, tip).await;
    after.ok("after the switch");
    assert_eq!(
        after.asked[SAP],
        vec![0],
        "the new server is verified from 0"
    );
    w.close().await.expect("close");
}

/// **T10b** — §3.1 / risk 5: the memo is in memory only; a reopen is a new
/// `Inner` and its first pass is full.
///
/// Mutant: the memo persisted (asked `[192]` after the reopen).
#[tokio::test]
async fn a_reopen_starts_full() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    let control = pass(&w, &mut ep, tip).await;
    control.ok("control");
    assert_eq!(control.asked[SAP], vec![192]);
    w.close().await.expect("close");

    let w = Wallet::open_with_vault(
        cfg(dir.path(), Network::Main, SeedPersistence::SealedKeychain),
        vault,
    )
    .await
    .expect("reopen");
    let after = pass(&w, &mut ep, tip).await;
    after.ok("after the reopen");
    assert_eq!(after.asked[SAP], vec![0]);
    w.close().await.expect("close");
}

/// **T11a** — §3.2 `PassesDue` at its boundary (gate 7):
/// `passes_since_full >= SUBTREE_ROOTS_FULL_VERIFY_PASSES` (180). With 179
/// incremental passes counted the plan is still incremental; at 180 it is full,
/// and the count restarts. The clock does not move (`TimeDue` cannot fire).
///
/// Count 130, so each incremental pass re-serves two roots from 128.
///
/// Mutants: `>` for `>=` (pass 181 asks `[128]`); `>= N − 1` (pass 180 asks
/// `[0]`); never (no `[0]` at all); the counter not incremented.
#[tokio::test]
async fn the_full_verification_runs_every_n_passes() {
    use crate::constants::SUBTREE_ROOTS_FULL_VERIFY_PASSES;
    let n = SUBTREE_ROOTS_FULL_VERIFY_PASSES as u64;
    let dir = tempfile::tempdir().expect("tempdir");
    let clock = ManualClock::at(T0);
    let w = main_wallet_on(dir.path(), &clock).await;
    let heights = sapling_prefix(130);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    for k in 0..n {
        assert_eq!(
            memo(&w, SAPLING).passes_since_full,
            k,
            "before incremental pass {}",
            k + 1
        );
        let p = pass(&w, &mut ep, tip).await;
        p.ok("incremental");
        assert_eq!(
            p.asked[SAP],
            vec![128],
            "passes_since_full {k} < {n}: incremental"
        );
    }
    assert_eq!(memo(&w, SAPLING).passes_since_full, n);
    let due = pass(&w, &mut ep, tip).await;
    due.ok("the due full pass");
    assert_eq!(due.asked[SAP], vec![0], "passes_since_full {n}: full");
    assert_eq!(
        memo(&w, SAPLING).passes_since_full,
        0,
        "and the count restarts"
    );
    let next = pass(&w, &mut ep, tip).await;
    next.ok("next");
    assert_eq!(next.asked[SAP], vec![128]);
    w.close().await.expect("close");
}

/// **T11b** — §3.2 `TimeDue` at its boundary (gate 7):
/// `clock − last_full_at >= SUBTREE_ROOTS_FULL_VERIFY_SECS` (3,600 s), on
/// `inner.clock`. At `T − 1` incremental, at `T` full, and `last_full_at` moves
/// to the new instant.
///
/// Mutants: `>` for `>=`; `T − 1`; never; the system clock read instead of
/// `inner.clock` (the injected clock never reaches it — the row stays
/// incremental at `T`).
#[tokio::test]
async fn the_full_verification_runs_every_t_seconds() {
    use crate::constants::SUBTREE_ROOTS_FULL_VERIFY_SECS;
    let t: u64 = SUBTREE_ROOTS_FULL_VERIFY_SECS;
    let dir = tempfile::tempdir().expect("tempdir");
    let clock = ManualClock::at(T0);
    let w = main_wallet_on(dir.path(), &clock).await;
    let heights = sapling_prefix(130);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    assert_eq!(
        memo(&w, SAPLING).last_full_at,
        T0,
        "the full fetch is stamped on inner.clock"
    );

    clock.set(T0 + t - 1);
    let early = pass(&w, &mut ep, tip).await;
    early.ok("T − 1");
    assert_eq!(
        early.asked[SAP],
        vec![128],
        "{} s after: incremental",
        t - 1
    );

    clock.set(T0 + t);
    let due = pass(&w, &mut ep, tip).await;
    due.ok("T");
    assert_eq!(due.asked[SAP], vec![0], "{t} s after: full");
    assert_eq!(memo(&w, SAPLING).last_full_at, T0 + t);
    w.close().await.expect("close");
}

/// A [`ScriptedPassClient`] whose every root stream opens after `pause` — so a
/// pass's roots phase is measurably non-zero (the phase-A row's fake has the
/// same knob on `FakeChain`, which is not a `PassClient`). Everything else is
/// delegated unchanged.
struct SlowRoots {
    inner: ScriptedPassClient,
    pause: std::time::Duration,
}

#[async_trait::async_trait]
impl sync::ScanClient for SlowRoots {
    async fn block_range(
        &mut self,
        start: u64,
        end_inclusive: u64,
    ) -> Result<crate::net::grpc::BlockStream, GrpcError> {
        sync::ScanClient::block_range(&mut self.inner, start, end_inclusive).await
    }
    async fn tree_state(
        &mut self,
        height: u64,
    ) -> Result<zcash_client_backend::proto::service::TreeState, GrpcError> {
        sync::ScanClient::tree_state(&mut self.inner, height).await
    }
    async fn latest_block_height(&mut self) -> Result<u64, GrpcError> {
        sync::ScanClient::latest_block_height(&mut self.inner).await
    }
}

#[async_trait::async_trait]
impl sync::SubtreeRootSource for SlowRoots {
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        tokio::time::sleep(self.pause).await;
        sync::SubtreeRootSource::subtree_roots(&mut self.inner, protocol, start_index).await
    }
}

#[async_trait::async_trait]
impl ChainOracle for SlowRoots {
    async fn tip_height(&mut self) -> Result<u64, GrpcError> {
        ChainOracle::tip_height(&mut self.inner).await
    }
    async fn server_identity(&mut self) -> Result<provision::ServerIdentity, GrpcError> {
        ChainOracle::server_identity(&mut self.inner).await
    }
}

#[async_trait::async_trait]
impl sync::TransparentUtxoSource for SlowRoots {
    async fn address_utxos(
        &mut self,
        addresses: Vec<String>,
        start_height: u64,
    ) -> Result<Vec<crate::transparent::TransparentUtxoRecord>, GrpcError> {
        sync::TransparentUtxoSource::address_utxos(&mut self.inner, addresses, start_height).await
    }
}

#[async_trait::async_trait]
impl crate::enhance::TransactionFetcher for SlowRoots {
    async fn fetch_transaction(
        &mut self,
        txid: zcash_protocol::TxId,
    ) -> Result<Option<crate::enhance::FetchedTransaction>, GrpcError> {
        crate::enhance::TransactionFetcher::fetch_transaction(&mut self.inner, txid).await
    }
}

/// **T17** — §3.3 step 8 (phase A's owed LOW-3): a pass cancelled AFTER its
/// tip and roots phases keeps their timings. The download gate holds the pass
/// in its first batch (both phases done), `stop()` cancels it there — the
/// `select!` drops the pass future, the cancelled arm builds the `SyncPass` —
/// and `tip_ms` / `roots_ms` still carry the 30 ms pauses the fake inserts.
/// Deterministic order (the gate, no wall-clock wait); the pauses are LOWER
/// bounds, which is what the assertion reads.
///
/// Mutant: the cancelled arm's `..SyncPass::default()` (both read 0).
#[tokio::test]
async fn a_cancelled_pass_keeps_its_phase_timings() {
    const PAUSE_MS: u64 = 30;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_with_chain_account(dir.path(), &vault).await;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel::<(u64, u64)>();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    let mut inner = ScriptedPassClient::honest(FIX_TIP);
    inner.chain.tip_delay = Some(std::time::Duration::from_millis(PAUSE_MS));
    inner.chain.download_gate = Some((started_tx, release_rx));
    let ctl = w.controller_over(SlowRoots {
        inner,
        pause: std::time::Duration::from_millis(PAUSE_MS),
    });
    let stopper = async {
        started_rx
            .await
            .expect("the pass reached its first download, past the tip and the roots");
        ctl.stop().await;
        drop(release_tx);
    };
    let (pass, ()) = tokio::join!(ctl.once(), stopper);
    let pass = pass.expect("a cancelled pass is Ok");
    assert!(pass.cancelled, "the pass was cancelled mid-download");
    let pools = u64::try_from(SUBTREE_ROOT_POOLS.len()).expect("small");
    assert!(
        pass.tip_ms >= PAUSE_MS,
        "tip_ms survives the cancel: {} ms",
        pass.tip_ms
    );
    assert!(
        pass.roots_ms >= PAUSE_MS * pools,
        "roots_ms survives the cancel: {} ms",
        pass.roots_ms
    );
    drop(ctl);
    w.close().await.expect("close");
}

/// **T19** — §3.3 step 5's first rule and ledger rows 4–5: a RELAXED bind
/// (a rewind outstanding) leaves the pool unverified with `c6_seen` cleared, so
/// the next pass is full AND strict — and C6 catches there what the relaxed pass
/// had to let through.
///
/// Ironwood, 200 roots, index 70's completing block scanned as X:
/// 1. full, the endpoint serves X at 70 ⇒ `c6_seen[70]`;
/// 2. incremental `[192]` (the control: 70 is seen);
/// 3. a rewind is noted; the endpoint now serves a LIE at 70 ⇒ full (`Rewound`),
///    relaxed — C6 abstains under the gate — accepted; the memo goes
///    unverified with `c6_seen` empty;
/// 4. full again (`Unverified`) and bound ⇒ C6 refuses the lie.
///
/// Mutant: a relaxed pass treated as a full one (`verified`, `c6_seen` filled):
/// pass 4 plans incremental, asks `[192]`, never re-serves 70, reads `Served`.
#[tokio::test]
async fn a_relaxed_pass_is_followed_by_a_full_bound_pass() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let iw = ironwood_heights(200);
    let tip = tip_above(&[&iw]);
    let h70 = iw[70];
    let x = block_hash(h70, 0xA1);
    scan_block(&w, h70, x);
    let mut ep = Endpoint::serving(&[], &[], &iw);
    ep.scripts[IW][70].completing_block_hash = x.to_vec();

    pass(&w, &mut ep, tip)
        .await
        .ok("1: full, C6 compares at 70");
    let p2 = pass(&w, &mut ep, tip).await;
    p2.ok("2");
    assert_eq!(p2.asked[IW], vec![192], "2: the control");

    note_a_rewind(&w);
    ep.scripts[IW][70].completing_block_hash = block_hash(h70, 0xB2).to_vec();
    let p3 = pass(&w, &mut ep, tip).await;
    let o3 = p3.ok("3: the relaxed pass");
    assert_eq!(
        p3.asked[IW],
        vec![0],
        "3: an outstanding rewind forces full"
    );
    assert_eq!(
        o3[IW],
        PoolFetch::Served { roots: 200 },
        "3: C6 abstains under the gate"
    );
    let relaxed = memo(&w, IRONWOOD);
    assert!(!relaxed.verified, "3: a relaxed pass verifies nothing");
    assert!(relaxed.c6_seen.is_empty(), "3: and clears what C6 had seen");

    let p4 = pass(&w, &mut ep, tip).await;
    let o4 = p4.ok("4: an Ironwood refusal is an outcome");
    assert_eq!(p4.asked[IW], vec![0], "4: full");
    assert_eq!(o4[IW], ironwood_refused_by(c6_code()), "4: and strict");
    w.close().await.expect("close");
}

/// **T20a** — §3.3 step 2, first bullet: a server STATUS refusing the open of a
/// `start > 0` stream is retried from 0 in the same pass (`roots_start_refused`)
/// and remembered (`incremental_off`), for that pool only. Rows: Sapling (a
/// required pool) and Ironwood (the optional one — its refusal must not be read
/// as the A9 "unsupported" outcome).
///
/// Mutants: the retry removed (Sapling: the pass fails; Ironwood:
/// `Unsupported`); the refusal not remembered (the next pass asks `[192, 0]`).
#[tokio::test]
async fn an_endpoint_that_refuses_a_nonzero_start_falls_back_to_full() {
    for (refusing, other) in [(SAP, IW), (IW, SAP)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (w, _vault) = main_wallet(dir.path()).await;
        let sap = sapling_prefix(200);
        let iw = ironwood_heights(200);
        let tip = tip_above(&[&sap, &iw]);
        let mut ep = Endpoint::serving(&sap, &[], &iw);
        pass(&w, &mut ep, tip).await.ok("full");
        let before = fallbacks(&w);

        ep.on_nonzero[refusing] = OnNonzero::RefusesOpen;
        let (sink, guard) = capture();
        let p = pass(&w, &mut ep, tip).await;
        drop(guard);
        let outcomes = p.ok("a refused start is retried from 0");
        assert_eq!(
            p.asked[refusing],
            vec![192, 0],
            "slot {refusing}: retried once, same pass"
        );
        assert_eq!(p.asked[other], vec![192], "slot {other}: untouched");
        // The from-0 retry was SERVED and written: not `Unsupported` (the A9
        // reading of a refusal — wrong here, the pool is served from 0) and not
        // a refusal. Sapling's 200-root prefix reads `Withheld` (a report), the
        // extended Ironwood run `Served`.
        assert!(
            matches!(
                outcomes[refusing],
                PoolFetch::Served { .. } | PoolFetch::Withheld { .. }
            ),
            "slot {refusing}: the retry served the pool, got {:?}",
            outcomes[refusing]
        );
        assert_eq!(stored_run(&w, SUBTREE_ROOT_POOLS[refusing]), 200);
        assert!(
            saw_outcome(&sink, "roots_start_refused"),
            "slot {refusing}: said"
        );
        assert_5_4_clean(&sink.fields());
        assert!(memo(&w, SUBTREE_ROOT_POOLS[refusing]).incremental_off);
        assert!(!memo(&w, SUBTREE_ROOT_POOLS[other]).incremental_off);
        assert!(fallbacks(&w) > before);

        let next = pass(&w, &mut ep, tip).await;
        next.ok("next");
        assert_eq!(
            next.asked[refusing],
            vec![0],
            "slot {refusing}: incremental_off holds"
        );
        assert_eq!(next.asked[other], vec![192]);
        w.close().await.expect("close");
    }
}

/// **T20b** — §3.3 step 2, second bullet: a MID-DRAIN transport fault (over
/// Tor, possibly one reset) is retried from 0 in the same pass
/// (`roots_start_dropped`) and does NOT turn incremental off: the drop costs
/// `[192, 0]` once, and once it heals the pass is `[192]`. (Two drops IN A ROW
/// do turn it off — the built-diff fold's F5,
/// `two_consecutive_dropped_incremental_streams_turn_incremental_off`.)
///
/// Mutants: the retry removed (`Err`); a reset turning incremental off (the
/// next pass asks `[0]`).
#[tokio::test]
async fn a_dropped_incremental_stream_retries_once_and_stays_incremental() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");

    ep.on_nonzero[SAP] = OnNonzero::DropsMidDrain;
    let (sink, guard) = capture();
    let dropped = pass(&w, &mut ep, tip).await;
    drop(guard);
    dropped.ok("a dropped stream is retried from 0");
    assert_eq!(dropped.asked[SAP], vec![192, 0]);
    assert!(saw_outcome(&sink, "roots_start_dropped"));
    assert_clean_but_the_debug_diagnostic(&sink);
    assert!(
        !memo(&w, SAPLING).incremental_off,
        "a reset is not a refusal"
    );

    ep.on_nonzero[SAP] = OnNonzero::Honest;
    let healed = pass(&w, &mut ep, tip).await;
    healed.ok("healed");
    assert_eq!(healed.asked[SAP], vec![192], "stays incremental");
    w.close().await.expect("close");
}

/// **§3.3 step 2, the timeout clause** (not a §7 row; the clause is pinned
/// here because nothing else exercises it): a TIMEOUT opening a `start > 0`
/// stream is "classified as today's timeout (no retry)" — the pass fails typed
/// and the endpoint is asked once. Since the built-diff fold (F4) the timeout
/// also turns incremental off, so the next pass is full (the wedge fix).
///
/// Mutants: a timeout retried from 0 (asked `[192, 0]`); a timeout leaving
/// incremental on.
#[tokio::test]
async fn an_incremental_timeout_is_not_retried() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");

    ep.on_nonzero[SAP] = OnNonzero::TimesOutOpen;
    let timed_out = pass(&w, &mut ep, tip).await;
    assert!(
        matches!(timed_out.result, Err(WalletError::Sync { .. })),
        "a timeout is today's typed Sync fault, got {:?}",
        timed_out.result
    );
    assert_eq!(timed_out.asked[SAP], vec![192], "and is not retried");
    assert!(memo(&w, SAPLING).incremental_off);

    ep.on_nonzero[SAP] = OnNonzero::Honest;
    let next = pass(&w, &mut ep, tip).await;
    next.ok("next");
    assert_eq!(next.asked[SAP], vec![0]);
    w.close().await.expect("close");
}

/// **T21** — §3.3 step 5: "an `Err` pass … leaves the memo untouched". Sapling's
/// newly scanned index 70 is re-served and C6 compares (the C6 set is `{70}`);
/// then ORCHARD's put fails (a mutated `root_hash` at a recorded index — the cap
/// `Conflict`) and the pass is `Err` — after Sapling's put already landed. The
/// memo must not take Sapling's C6 set from that pass: the next pass re-serves
/// 70 again.
///
/// Mutant: the memo updated on `Err` (or per pool as each put lands): pass 4
/// asks `[192]`.
#[tokio::test]
async fn a_failed_write_leaves_the_memo_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let sap = sapling_prefix(200);
    let orc = orchard_prefix(3);
    let tip = tip_above(&[&sap, &orc]);
    let mut ep = Endpoint::serving(&sap, &orc, &[]);
    pass(&w, &mut ep, tip).await.ok("1: full");
    let p2 = pass(&w, &mut ep, tip).await;
    p2.ok("2");
    assert_eq!(p2.asked[SAP], vec![192], "2: the control");

    let h70 = sap[70];
    let x = block_hash(h70, 0xA1);
    scan_block(&w, h70, x);
    ep.scripts[SAP][70].completing_block_hash = x.to_vec();
    let honest_orchard_0 = ep.scripts[ORC][0].root_hash.clone();
    ep.scripts[ORC][0].root_hash = wide_root_hash(ORCHARD, 100_000);
    let p3 = pass(&w, &mut ep, tip).await;
    assert!(
        matches!(p3.result, Err(WalletError::Sync { .. })),
        "3: Orchard's put is a cap Conflict, got {:?}",
        p3.result
    );
    assert_eq!(
        p3.asked[SAP],
        vec![64],
        "3: Sapling re-served its scanned index"
    );
    assert!(
        !memo(&w, SAPLING).c6_seen.contains_key(&70),
        "3: an Err pass leaves the memo untouched"
    );

    ep.scripts[ORC][0].root_hash = honest_orchard_0;
    let p4 = pass(&w, &mut ep, tip).await;
    p4.ok("4: Orchard honest again");
    assert_eq!(
        p4.asked[SAP],
        vec![64],
        "4: 70 is still unseen, so re-served"
    );
    assert_eq!(memo(&w, SAPLING).c6_seen.get(&70), Some(&(h70, x)));
    let p5 = pass(&w, &mut ep, tip).await;
    p5.ok("5");
    assert_eq!(p5.asked[SAP], vec![192]);
    w.close().await.expect("close");
}

/// **T22** — §1 `read_stored_run` / ledger row 16: a NULL `root_hash` at a
/// recorded index ENDS the stored run, so it is re-served and today's put
/// restores it.
///
/// ## The probe (static, upstream `zcash_client_sqlite-0.22.0`, read 2026-10-03)
///
/// How a REAL scan leaves `root_hash` NULL under a non-NULL `subtree_end_height`:
/// * `commitment_tree.rs` `put_shard` (~545–589) computes
///   `subtree_root_hash = subtree.root().annotation().and_then(..)` and writes it
///   through `INSERT … ON CONFLICT (shard_index) DO UPDATE SET root_hash =
///   :root_hash, shard_data = :shard_data` — `subtree_end_height` is NOT in the
///   `SET` list, so a row `put_shard_roots` filled keeps its height while
///   `root_hash` is overwritten with whatever the annotation is.
/// * `shardtree-0.7.1 tree.rs` ~49: `annotation()` is `None` for a `Leaf` or
///   `Nil` root, and a `Parent`'s annotation is itself an `Option`.
/// * `shardtree-0.7.1 prunable.rs` `insert_subtree` (~903–916): when the stored
///   shard is a `Leaf` (exactly what `put_shard_roots` stores as `shard_data`:
///   `PrunableTree::leaf((root, EPHEMERAL))`, commitment_tree.rs ~1305) and the
///   scan inserts a subtree at the shard address whose root is computable and
///   not a leaf, the code takes the "replace the existing root UNANNOTATED"
///   arm — a `Parent` whose annotation is `None`.
/// * So a scan that fills a whole shard whose root was first served by
///   `put_shard_roots` writes `root_hash = NULL` beside the served height.
///   (`get_shard` reannotates from `root_hash` on READ, ~466–470, which is why
///   nothing upstream notices.)
///
/// This is a reading, not a run: an end-to-end probe needs a 2^16-commitment
/// shard scanned through `scan_cached_blocks`, which no fixture in this crate
/// carries. The fixture below writes the state the reading predicts directly,
/// the way `root_bind::tests::a_suffix_withdrawal_leaves_the_recorded_heights_a_dense_prefix`
/// does (`UPDATE … SET root_hash = NULL`). Recorded in the report as owed:
/// the measured probe in `docs/plan/probes/`.
///
/// The NULL sits at index 128 (`≡ 0`): the run is 128, `count − 1 = 127` ⇒ 64.
/// A run that counted the NULL row (or counted heights alone) reads 129..200 ⇒
/// 128 or 192.
///
/// Mutant: `count` from heights alone (asked `[192]`; 128's root never restored).
#[tokio::test]
async fn a_null_root_hash_ends_the_stored_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");

    exec(
        &w,
        "UPDATE sapling_tree_shards SET root_hash = NULL WHERE shard_index = 128",
        [],
    );
    assert_eq!(
        shard_row(&w, SAPLING, 128),
        Some((Some(heights[128]), None)),
        "the fixture: a recorded height with no root"
    );
    assert_eq!(
        stored_run(&w, SAPLING),
        128,
        "the run ends at the NULL root"
    );
    let p = pass(&w, &mut ep, tip).await;
    p.ok("the re-serve");
    assert_eq!(p.asked[SAP], vec![64]);
    assert_eq!(
        shard_row(&w, SAPLING, 128),
        Some((Some(heights[128]), Some(wide_root_hash(SAPLING, 128)))),
        "today's put restores the root"
    );
    assert_eq!(stored_run(&w, SAPLING), 200);
    w.close().await.expect("close");
}

/// **T23** — §3.3 step 4.1 / ledger row 15: the locked section RE-READS the
/// plan's snapshot (`count`, `recorded[..start]`, the rewind counter) and any
/// difference writes nothing and returns `Sync { Internal }`; the next pass
/// re-plans.
///
/// The change lands in the fetch window (the endpoint's open hook — after the
/// plan, before the locked section). Two rows, each shaped so ONLY the
/// re-check can catch it:
/// * the CHAIN counter moves and every pool's record moves with it — no rewind
///   is outstanding, so `check_pool_from`'s own rewind refusal cannot fire;
/// * the stored run SHRINKS (a NULL root at 150, below the start) — the
///   recorded heights are unchanged, so the bind accepts the serve.
/// The serve grows the run to 201, so "writes nothing" is a row that must not
/// exist.
///
/// Mutant: the snapshot re-check removed (both rows `Ok`, index 200 written).
#[tokio::test]
async fn a_changed_plan_snapshot_writes_nothing() {
    #[derive(Clone, Copy, Debug)]
    enum Change {
        ChainCounter,
        RunShrinks,
    }
    for change in [Change::ChainCounter, Change::RunShrinks] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (w, _vault) = main_wallet(dir.path()).await;
        let grown = sapling_prefix(201);
        let tip = tip_above(&[&grown]);
        let mut ep = Endpoint::serving(&grown[..200], &[], &[]);
        pass(&w, &mut ep, tip).await.ok("full");

        ep.scripts[SAP] = roots_of(SAPLING, &grown);
        let inner = Arc::clone(&w.inner);
        let mut fired = false;
        ep.on_open = Some(Box::new(move |pool: ShieldedProtocol, start: u32| {
            if fired || pool != ShieldedProtocol::Sapling || start == 0 {
                return;
            }
            fired = true;
            let aux = inner.aux_db.lock().expect("wallet aux-db mutex poisoned");
            match change {
                Change::ChainCounter => {
                    root_bind::note_rewind(&aux).expect("note");
                    let now = root_bind::read_rewind_watch(&aux, ShieldedProtocol::Sapling)
                        .expect("read")
                        .observed;
                    for p in SUBTREE_ROOT_POOLS {
                        root_bind::note_roots_recorded(&aux, *p, now).expect("record");
                    }
                }
                Change::RunShrinks => {
                    aux.execute(
                        "UPDATE sapling_tree_shards SET root_hash = NULL WHERE shard_index = 150",
                        [],
                    )
                    .expect("null a root below the start");
                }
            }
        }));
        let changed = pass(&w, &mut ep, tip).await;
        assert_eq!(
            changed.asked[SAP],
            vec![192],
            "{change:?}: the plan was incremental"
        );
        assert!(
            matches!(
                changed.result,
                Err(WalletError::Sync {
                    stall: StallReason::Internal
                })
            ),
            "{change:?}: a changed snapshot is Sync {{ Internal }}, got {:?}",
            changed.result
        );
        assert_eq!(
            shard_row(&w, SAPLING, 200),
            None,
            "{change:?}: nothing was written"
        );

        ep.on_open = None;
        let next = pass(&w, &mut ep, tip).await;
        next.ok("the next pass re-plans");
        let expected = match change {
            Change::ChainCounter => 0,
            Change::RunShrinks => 128,
        };
        assert_eq!(
            next.asked[SAP],
            vec![expected],
            "{change:?}: re-planned from the new state"
        );
        assert_eq!(
            shard_row(&w, SAPLING, 200).and_then(|(h, _)| h),
            Some(grown[200]),
            "{change:?}: and the growth lands"
        );
        drop(ep);
        w.close().await.expect("close");
    }
}

/// **T24** — §3.2 `TimeDue`, first half: "a clock set backwards is untrusted".
/// One second before `last_full_at` forces a full fetch, and the subtraction
/// does not underflow (a debug build panics on `clock − last_full_at`).
/// Afterwards `last_full_at` is the new (earlier) instant and the next pass at
/// that instant is incremental.
///
/// Mutants: the backwards-clock rule dropped (asked `[192]`); an unsaturated
/// subtraction (panic).
#[tokio::test]
async fn a_clock_set_backwards_forces_a_full_fetch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clock = ManualClock::at(T0);
    let w = main_wallet_on(dir.path(), &clock).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    let control = pass(&w, &mut ep, tip).await;
    control.ok("control");
    assert_eq!(control.asked[SAP], vec![192]);

    clock.set(T0 - 1);
    let back = pass(&w, &mut ep, tip).await;
    back.ok("a backwards clock");
    assert_eq!(
        back.asked[SAP],
        vec![0],
        "a clock before last_full_at is untrusted"
    );
    assert_eq!(memo(&w, SAPLING).last_full_at, T0 - 1);
    let next = pass(&w, &mut ep, tip).await;
    next.ok("next");
    assert_eq!(next.asked[SAP], vec![192]);
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// The built-diff fold (S15-F1 plan §11): rows the diff review asked for.
// ════════════════════════════════════════════════════════════════════════════

/// **F1 (security MEDIUM-1)** — §3.3 step 2 at the refusal doors T20a/T20b do
/// not reach: the A9-shaped `ShieldedProtocolUnknown` at the OPEN of a
/// `start > 0` stream, a server InvalidArgument MID-DRAIN (with trailers — the
/// shape the stream classifier reads as A9's refusal — and without), and a
/// TRANSPORT fault at the open. Each on a Sapling row (a required pool: must not
/// be fatal) and an Ironwood row (the optional pool: must not become
/// `Unsupported`). Every one is retried from 0 in the same pass (`[192, 0]`) and
/// served; a status turns incremental off (`roots_start_refused`), a transport
/// fault does not (`roots_start_dropped`).
///
/// Mutants: a non-zero-start `ShieldedProtocolUnknown` (open or mid-drain)
/// classified as today's A9 refusal — Ironwood `Unsupported` with no retry
/// (asked `[192]`), Sapling fatal; a mid-drain status made fatal; a transport
/// fault at the open made fatal or read as a refusal (`incremental_off` set).
#[tokio::test]
async fn every_refusal_door_at_a_nonzero_start_is_retried_from_zero() {
    let doors = [
        (
            OnNonzero::RefusesOpenProtocolUnknown,
            "roots_start_refused",
            true,
        ),
        (
            OnNonzero::StatusMidDrain { trailers: true },
            "roots_start_refused",
            true,
        ),
        (
            OnNonzero::StatusMidDrain { trailers: false },
            "roots_start_refused",
            true,
        ),
        (OnNonzero::TransportAtOpen, "roots_start_dropped", false),
    ];
    for (door, code, off) in doors {
        for (faulty, other) in [(SAP, IW), (IW, SAP)] {
            let dir = tempfile::tempdir().expect("tempdir");
            let (w, _vault) = main_wallet(dir.path()).await;
            let sap = sapling_prefix(200);
            let iw = ironwood_heights(200);
            let tip = tip_above(&[&sap, &iw]);
            let mut ep = Endpoint::serving(&sap, &[], &iw);
            pass(&w, &mut ep, tip).await.ok("full");
            let before = fallbacks(&w);

            ep.on_nonzero[faulty] = door;
            let (sink, guard) = capture();
            let p = pass(&w, &mut ep, tip).await;
            drop(guard);
            let outcomes = p.ok(&format!("{door:?} slot {faulty}: not fatal"));
            assert_eq!(
                p.asked[faulty],
                vec![192, 0],
                "{door:?} slot {faulty}: retried once from 0, same pass"
            );
            assert_eq!(
                p.asked[other],
                vec![192],
                "{door:?}: slot {other} untouched"
            );
            assert!(
                matches!(
                    outcomes[faulty],
                    PoolFetch::Served { .. } | PoolFetch::Withheld { .. }
                ),
                "{door:?} slot {faulty}: the retry served the pool — never Unsupported, \
                 never a refusal — got {:?}",
                outcomes[faulty]
            );
            assert_eq!(stored_run(&w, SUBTREE_ROOT_POOLS[faulty]), 200);
            assert!(
                saw_outcome(&sink, code),
                "{door:?} slot {faulty}: said {code}"
            );
            // Only a tonic Status through the classifier meets the debug-only
            // `transport_chain` diagnostic; the doors the fake answers with a
            // GrpcError directly are graded on every field.
            if matches!(door, OnNonzero::StatusMidDrain { .. }) {
                assert_clean_but_the_debug_diagnostic(&sink);
            } else {
                assert_5_4_clean(&sink.fields());
            }
            assert_eq!(
                memo(&w, SUBTREE_ROOT_POOLS[faulty]).incremental_off,
                off,
                "{door:?} slot {faulty}: incremental_off"
            );
            assert!(!memo(&w, SUBTREE_ROOT_POOLS[other]).incremental_off);
            assert_eq!(fallbacks(&w), before + 1, "{door:?}: one second stream");
            w.close().await.expect("close");
        }
    }
}

/// **F3 (crypto MEDIUM)** — ledger row 13: the overlap's ROOT guard is the cap
/// `Conflict`, with no SDK-side compare. After a full 200-root pass, an
/// incremental pass (asks `[192]`) whose script carries a DIFFERENT `root_hash`
/// at an already-recorded index — at the SAME height — must fail typed, leave
/// that shard row as it was, write nothing at the new index 200 the serve also
/// carried, and leave the memo untouched. Two rows: the mutation at 195 (inside
/// the overlap) and at 192 (the overlap index itself, `== start`, where the
/// height check passes by construction). The refusal is exactly the typed
/// `Sync { EndpointMisbehaving }` a cap `Conflict` maps to. First, the honest
/// CONTROL: the same 201-root script unmutated is served incrementally
/// (`[192]`), accepted, and writes index 200 — so the refusals below are the
/// mutation's, not the grown serve's.
///
/// Mutants: a positional write that overwrites the recorded leaf (`put` at an
/// index the cap does not hold); the memo updated on the failed pass; an
/// incremental put that skips the overlap it was served.
#[tokio::test]
async fn an_incremental_overlap_with_a_different_root_is_a_conflict() {
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let (w, _vault) = main_wallet(dir.path()).await;
        let grown = sapling_prefix(201);
        let tip = tip_above(&[&grown]);
        let mut ep = Endpoint::serving(&grown[..200], &[], &[]);
        pass(&w, &mut ep, tip).await.ok("control: full");
        ep.scripts[SAP] = roots_of(SAPLING, &grown);
        let p = pass(&w, &mut ep, tip).await;
        assert_eq!(p.asked[SAP], vec![192], "control: incremental");
        p.ok("control: the honest grown serve is accepted");
        assert!(
            shard_row(&w, SAPLING, 200).is_some(),
            "control: the honest grown serve writes index 200"
        );
        w.close().await.expect("close");
    }
    for mutated in [195usize, 192] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (w, _vault) = main_wallet(dir.path()).await;
        let grown = sapling_prefix(201);
        let tip = tip_above(&[&grown]);
        let mut ep = Endpoint::serving(&grown[..200], &[], &[]);
        pass(&w, &mut ep, tip).await.ok("full");
        let memo_before = memo(&w, SAPLING);
        let index = i64::try_from(mutated).expect("a fixture index");
        let row_before = shard_row(&w, SAPLING, index);
        assert_eq!(
            row_before,
            Some((
                Some(grown[mutated]),
                Some(wide_root_hash(SAPLING, mutated as u64))
            )),
            "index {mutated}: the honest row is recorded"
        );

        ep.scripts[SAP] = roots_of(SAPLING, &grown);
        ep.scripts[SAP][mutated].root_hash = wide_root_hash(SAPLING, 100_000);
        let p = pass(&w, &mut ep, tip).await;
        assert_eq!(p.asked[SAP], vec![192], "index {mutated}: incremental");
        assert!(
            matches!(
                p.result,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "index {mutated}: a different root at a recorded index is a typed \
             Conflict, got {:?}",
            p.result
        );
        assert_eq!(
            shard_row(&w, SAPLING, index),
            row_before,
            "index {mutated}: the recorded root stands"
        );
        assert_eq!(
            shard_row(&w, SAPLING, 200),
            None,
            "index {mutated}: nothing of the refused serve is written"
        );
        assert_eq!(
            memo(&w, SAPLING),
            memo_before,
            "index {mutated}: a failed pass leaves the memo untouched"
        );
        w.close().await.expect("close");
    }
}

/// **F4 (security LOW-2, the wedge)** — a TIMEOUT opening a `start > 0` stream
/// still fails the pass as today's timeout (typed `Sync`, asked once, no second
/// stream) but turns the pool's incremental fetch OFF and says so
/// (`roots_start_timed_out`), so the next pass is full — an endpoint that
/// stalls only on a non-zero start cannot fail every pass of the session.
///
/// Mutants: the timeout retried (asked `[192, 0]`, a fallback counted);
/// incremental left on (the next pass asks `[192]` and times out again); the
/// warn dropped.
#[tokio::test]
async fn an_incremental_timeout_turns_incremental_off() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    let before = fallbacks(&w);

    ep.on_nonzero[SAP] = OnNonzero::TimesOutOpen;
    let (sink, guard) = capture();
    let timed_out = pass(&w, &mut ep, tip).await;
    drop(guard);
    assert!(
        matches!(timed_out.result, Err(WalletError::Sync { .. })),
        "a timeout is today's typed Sync fault, got {:?}",
        timed_out.result
    );
    assert_eq!(timed_out.asked[SAP], vec![192], "asked once, not retried");
    assert_eq!(fallbacks(&w), before, "no second stream");
    assert!(saw_outcome(&sink, "roots_start_timed_out"));
    assert_5_4_clean(&sink.fields());
    assert!(memo(&w, SAPLING).incremental_off, "the next pass is full");

    // The endpoint still stalls on any non-zero start: the session goes on.
    let next = pass(&w, &mut ep, tip).await;
    next.ok("a full pass, never the stalling start");
    assert_eq!(next.asked[SAP], vec![0]);
    let after = pass(&w, &mut ep, tip).await;
    after.ok("and for the rest of the session");
    assert_eq!(after.asked[SAP], vec![0]);
    w.close().await.expect("close");
}

/// **F5 (security LOW-1)** — `SUBTREE_ROOTS_DROPS_OFF` = 2: two
/// `roots_start_dropped` IN A ROW for one pool turn its incremental fetch off,
/// so a server that resets every non-zero start costs `[192, 0]` twice, then
/// `[0]`; one at the open and one mid-drain count alike. A pass that WRITES the
/// pool from a `start > 0` serve between two drops resets the count (row 2).
/// The final fold's semantics (ADR-0570): a drop counts only when the same
/// pass's retry from 0 was SERVED — one whose retry also fails is not counted
/// (row 3); and a whole `start > 0` serve the bind refuses writes nothing and
/// resets nothing (row 4).
///
/// Mutants: the count never kept (pass 2 leaves it on); the reset removed (row 2
/// turns off after drop–heal–drop); the threshold at 1 (pass 1 turns it off) or
/// at 3 (pass 2 leaves it on); the count kept across pools (Ironwood's honest
/// serve is untouched by Sapling's drops — asserted); a drop counted before its
/// retry is known to be served (row 3 counts 1 after the failed pass); the count
/// reset by a serve drained whole but never written (row 4 stays on).
#[tokio::test]
async fn two_consecutive_dropped_incremental_streams_turn_incremental_off() {
    // Row 1: two drops in a row — the open door, then the mid-drain door.
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let sap = sapling_prefix(200);
    let iw = ironwood_heights(200);
    let tip = tip_above(&[&sap, &iw]);
    let mut ep = Endpoint::serving(&sap, &[], &iw);
    pass(&w, &mut ep, tip).await.ok("full");

    ep.on_nonzero[SAP] = OnNonzero::TransportAtOpen;
    let first = pass(&w, &mut ep, tip).await;
    first.ok("drop 1");
    assert_eq!(first.asked[SAP], vec![192, 0]);
    assert!(!memo(&w, SAPLING).incremental_off, "one drop is a reset");

    ep.on_nonzero[SAP] = OnNonzero::DropsMidDrain;
    let second = pass(&w, &mut ep, tip).await;
    second.ok("drop 2");
    assert_eq!(second.asked[SAP], vec![192, 0]);
    assert!(
        memo(&w, SAPLING).incremental_off,
        "two in a row turn incremental off"
    );
    assert!(!memo(&w, IRONWOOD).incremental_off, "per pool");

    let third = pass(&w, &mut ep, tip).await;
    third.ok("full from now on");
    assert_eq!(third.asked[SAP], vec![0], "never a third dropped stream");
    assert_eq!(third.asked[IW], vec![192]);
    w.close().await.expect("close");

    // Row 2: a whole incremental serve between two drops resets the count.
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let mut ep = Endpoint::serving(&sap, &[], &iw);
    pass(&w, &mut ep, tip).await.ok("full");
    for round in 1..=2 {
        ep.on_nonzero[SAP] = OnNonzero::DropsMidDrain;
        let dropped = pass(&w, &mut ep, tip).await;
        dropped.ok("a drop");
        assert_eq!(dropped.asked[SAP], vec![192, 0], "round {round}");
        assert!(
            !memo(&w, SAPLING).incremental_off,
            "round {round}: a heal in between resets the count"
        );
        ep.on_nonzero[SAP] = OnNonzero::Honest;
        let healed = pass(&w, &mut ep, tip).await;
        healed.ok("healed");
        assert_eq!(healed.asked[SAP], vec![192], "round {round}: incremental");
    }
    w.close().await.expect("close");

    // Row 3: a drop whose retry from 0 ALSO fails is not counted — that says the
    // path is down, not that the server resets non-zero starts. The retry is
    // failed by a 31-byte root at index 0, which only a from-0 stream carries.
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let mut ep = Endpoint::serving(&sap, &[], &iw);
    pass(&w, &mut ep, tip).await.ok("full");
    ep.on_nonzero[SAP] = OnNonzero::TransportAtOpen;
    let honest_0 = ep.scripts[SAP][0].root_hash.clone();
    ep.scripts[SAP][0].root_hash.truncate(31);
    let failed = pass(&w, &mut ep, tip).await;
    assert!(
        matches!(failed.result, Err(WalletError::Sync { .. })),
        "row 3: the retry's decode fault fails the pass, got {:?}",
        failed.result
    );
    assert_eq!(failed.asked[SAP], vec![192, 0], "row 3: dropped, retried");
    assert_eq!(
        memo(&w, SAPLING).consecutive_drops,
        0,
        "row 3: a drop whose retry also failed is not counted"
    );
    ep.scripts[SAP][0].root_hash = honest_0;
    let counted = pass(&w, &mut ep, tip).await;
    counted.ok("row 3: a drop whose retry is served");
    assert_eq!(counted.asked[SAP], vec![192, 0]);
    assert!(
        !memo(&w, SAPLING).incremental_off,
        "row 3: the uncounted drop did not make this the second"
    );
    let second = pass(&w, &mut ep, tip).await;
    second.ok("row 3: the second counted drop");
    assert!(
        memo(&w, SAPLING).incremental_off,
        "row 3, the control: two counted drops in a row turn it off"
    );
    w.close().await.expect("close");

    // Row 4: a whole `start > 0` serve the BIND refuses (Ironwood: a recorded
    // outcome, nothing written) does not reset the count — drop, refused serve,
    // drop is two in a row.
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let mut ep = Endpoint::serving(&sap, &[], &iw);
    pass(&w, &mut ep, tip).await.ok("full");
    ep.on_nonzero[IW] = OnNonzero::TransportAtOpen;
    let first = pass(&w, &mut ep, tip).await;
    first.ok("row 4: drop 1");
    assert_eq!(first.asked[IW], vec![192, 0]);
    assert_eq!(memo(&w, IRONWOOD).consecutive_drops, 1, "row 4: counted");
    ep.on_nonzero[IW] = OnNonzero::Honest;
    let mut lying = iw.clone();
    lying.push(iw[199] + 1);
    ep.scripts[IW] = roots_of(IRONWOOD, &lying);
    let refused = pass(&w, &mut ep, tip).await;
    let outcomes = refused.ok("row 4: an Ironwood refusal is a recorded outcome");
    assert_eq!(
        refused.asked[IW],
        vec![192],
        "row 4: drained whole from 192"
    );
    assert_eq!(outcomes[IW], ironwood_refused_by(gap_code()));
    assert_eq!(
        memo(&w, IRONWOOD).consecutive_drops,
        1,
        "row 4: a refused serve writes nothing and resets nothing"
    );
    ep.scripts[IW] = roots_of(IRONWOOD, &iw);
    ep.on_nonzero[IW] = OnNonzero::TransportAtOpen;
    let second = pass(&w, &mut ep, tip).await;
    second.ok("row 4: drop 2");
    assert_eq!(second.asked[IW], vec![192, 0]);
    assert!(
        memo(&w, IRONWOOD).incremental_off,
        "row 4: drop, refused serve, drop — two in a row turn it off"
    );
    w.close().await.expect("close");
}

/// **The final fold (security LOW + crypto LOW): a MID-DRAIN timeout at a
/// non-zero start** — F4's door at its second place. The stream opens at 192,
/// serves the overlap root, then stalls until the pump's per-message bound
/// (`GRPC_STREAMING_TIMEOUT_SECS`, on the paused tokio clock) fires. Exactly
/// as at the open: the pass fails typed, the endpoint is asked once (`[192]`,
/// no fallback stream), incremental is turned off and said
/// (`roots_start_timed_out`), and the next pass is full (`[0]`). The pump's
/// timeout arm never reaches the status classifier, so every field is graded.
///
/// Mutant: the mid-drain timeout arm read as a drop (`StartFault::Dropped` —
/// retried from 0: asked `[192, 0]`, the pass `Ok`, incremental left on).
#[tokio::test(start_paused = true)]
async fn an_incremental_timeout_mid_drain_turns_incremental_off() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _vault) = main_wallet(dir.path()).await;
    let heights = sapling_prefix(200);
    let tip = tip_above(&[&heights]);
    let mut ep = Endpoint::serving(&heights, &[], &[]);
    pass(&w, &mut ep, tip).await.ok("full");
    let before = fallbacks(&w);

    ep.on_nonzero[SAP] = OnNonzero::TimesOutMidDrain;
    let (sink, guard) = capture();
    let timed_out = pass(&w, &mut ep, tip).await;
    drop(guard);
    assert!(
        matches!(timed_out.result, Err(WalletError::Sync { .. })),
        "a mid-drain timeout is today's typed Sync fault, got {:?}",
        timed_out.result
    );
    assert_eq!(timed_out.asked[SAP], vec![192], "asked once, not retried");
    assert_eq!(fallbacks(&w), before, "no second stream");
    assert!(saw_outcome(&sink, "roots_start_timed_out"));
    assert_5_4_clean(&sink.fields());
    assert!(memo(&w, SAPLING).incremental_off, "the next pass is full");

    let next = pass(&w, &mut ep, tip).await;
    next.ok("a full pass, never the stalling start");
    assert_eq!(next.asked[SAP], vec![0]);
    w.close().await.expect("close");
}
