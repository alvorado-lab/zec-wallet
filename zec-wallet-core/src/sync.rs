//! §3.2g — the lightwalletd sync engine (W3-inc-2c-iv-d).
//!
//! **inc-2c-iv-d-1 (this slice): subtree-root ingestion — the SPEND-witness trust
//! anchor.** Stream the note-commitment **subtree roots** for every pool in
//! [`SUBTREE_ROOT_POOLS`] — Sapling, Orchard **and Ironwood** (T0-1; the third was
//! missing since NU6.3 and its absence is what made incoming Ironwood money show as
//! Pending and never clear) — from the endpoint and write them into the wallet's
//! shardtrees (`WalletCommitmentTrees::put_{sapling,orchard,ironwood}_subtree_roots`).
//! The shardtree needs these COMPLETED subtree roots to build anchors/witnesses
//! for SPENDING
//! (inc-2d) and to underpin `suggest_scan_ranges` prioritization. Fund DETECTION
//! (trial-decryption) works WITHOUT them, but the reference
//! `zcash_client_backend::sync::run` does this FIRST (`sync.rs:78`), so we mirror
//! it (the alignment audit, S25) — omitting it would diverge from the reference
//! silently and break spend later.
//!
//! **Hostile input (§4.6).** Each `SubtreeRoot` is untrusted: the `root_hash` is
//! decoded through the leaf type's CANONICAL `from_bytes` (a non-32-byte or
//! non-canonical field element → typed [`SubtreeRootInvalid`], NEVER a panic), and
//! the completing height is range-checked to `u32` (consensus heights are u32 — a
//! larger value is rejected, never truncated). A malformed root makes the endpoint
//! unusable for sync (typed `Sync { EndpointUnreachable }` — the retry/fallback
//! signal iv-d-3 owns), never silent. NO key material, NO decryption: this is the
//! PUBLIC commitment tree, not note plaintext. A server lying with well-formed but
//! WRONG roots is not caught here (the put succeeds on FIRST ingestion) — for a
//! subtree the wallet SCANS (at/above the account birthday) it surfaces at scan
//! time as a shardtree `Conflict` on a COMPLETED shard (the crypto audit, §3.2f),
//! iv-d-2's concern; only a cross-sync root MUTATION conflicts at put time
//! (attributed honestly, [`put_subtree_roots`]). ⚠️ **A wrong PRE-BIRTHDAY root is
//! NOT covered by the scan-conflict defense** (the wallet never scans below its
//! birthday, so that root is never re-derived to disagree) — its only check is the
//! network's anchor verification when a witness built over it is broadcast (a wrong
//! sibling ⇒ a wrong anchor ⇒ the tx is REJECTED, never an accepted wrong-value
//! spend). Bounded to recoverable DENIAL-OF-SPEND (no fund loss, no theft); the
//! tightened recovery state + a birthday-frontier cross-check are owed to iv-d-2 /
//! inc-2d (the money-lens 3-lens, S30).
//!
//! The transport pump (per-message idle timeout, SPENT-ON-ERROR latch,
//! cancel-safety) is the shared [`MessageStream`](crate::net::grpc) from net/grpc
//! (the iv-a generalization, §3.2g); this module is the CONVERSION + DB-write layer
//! over it. The `LightwalletdClient` is reached through the [`SubtreeRootSource`]
//! seam so the orchestration is unit-testable with NO live gRPC server (mirroring
//! provision's `ChainOracle`).
//
// Staged: the d-2 scan loop + d-3 controller grow in this module; the
// orchestration is driven by the async `Wallet` handle (wallet.rs).

use async_trait::async_trait;
use orchard::tree::MerkleHashOrchard;
use prost::Message;
use sapling_crypto::Node as SaplingNode;
use shardtree::error::ShardTreeError;
use zcash_client_backend::data_api::chain::{
    ChainState, CommitmentTreeRoot, error::Error as ChainScanError, scan_cached_blocks,
};
use zcash_client_backend::data_api::{WalletCommitmentTrees, WalletRead, WalletWrite};
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_backend::proto::service::{ShieldedProtocol, SubtreeRoot, TreeState};
use zcash_client_sqlite::error::SqliteClientError;
use zcash_client_sqlite::wallet::commitment_tree;
use zcash_primitives::block::BlockHash;
use zcash_protocol::TxId;
use zcash_protocol::consensus::{BlockHeight, NetworkUpgrade, Parameters};

use crate::block_cache::BlockCache;
use crate::constants::{
    DOWNLOAD_BATCH_MAX_BYTES, MAX_SCAN_REORGS_PER_PASS, MAX_SUBTREE_ROOTS_PER_POOL,
    MAX_TOTAL_REORGS_PER_PASS, PROGRESS_REPORT_BLOCKS, REORG_MAX_BLOCKS, REWIND_DISTANCE_BLOCKS,
    SUBTREE_ROOTS_DROPS_OFF, SUBTREE_ROOTS_FULL_VERIFY_PASSES, SUBTREE_ROOTS_FULL_VERIFY_SECS,
    SUBTREE_ROOTS_START_GRANULARITY, SYNC_BATCH_BLOCKS,
};
use crate::db::{WalletConn, WalletConnFor};
use crate::enhance::{FetchedTransaction, TransactionFetcher};
use crate::error::WalletError;
use crate::money::Network;
use crate::net::grpc::{
    BlockStream, GrpcError, LightwalletdClient, SubtreeRootStream, transport_err,
};
use crate::root_bind;
use crate::state::StallReason;
use crate::transparent::TransparentUtxoRecord;

/// What subtree-root ingestion needs from the (untrusted) endpoint — the streaming
/// sibling of provision's `ChainOracle`. A testability seam: the crate ships only
/// the lightwalletd CLIENT codegen (no server), so the ingestion is driven through
/// a fake source (`testing::FakeRootSource`) with no live network.
#[async_trait]
pub(crate) trait SubtreeRootSource {
    /// Open a stream of every completed subtree root for `protocol` from shard
    /// index `start_index` on. `0` is the reference contract (fetch from 0, write
    /// at 0; upstream `sync.rs:223`) and every full fetch; a non-zero start is
    /// S15-F1's incremental fetch (ADR-0569), whose plan, checks and fallbacks are
    /// [`fetch_subtree_roots_planned`]'s. A double must HONOUR the start — serve
    /// the roots from that index — or it claims a capability production's
    /// endpoints are measured to have (`docs/plan/probes/README.md`).
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError>;
}

#[async_trait]
impl SubtreeRootSource for LightwalletdClient {
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        // `start_index` is sent as asked: 0 for a full fetch, the plan's start for
        // an incremental one (S15-F1). Both measured servers honour it, and a start
        // at or past the count is an EMPTY stream with OK status; a server that
        // refuses or ignores it is caught by the fetch's own checks and retried
        // from 0 in the same pass (`fetch_subtree_roots_planned`).
        // `max_entries == 0` is the proto's documented "all entries" sentinel
        // (lightwalletd `service.proto`: "Maximum number of entries to return, or 0
        // for all entries"). We do NOT pass a non-zero geometric ceiling: lightwalletd
        // forwards `max_entries` as zcashd's `z_getsubtreesbyindex` `limit`, and zcashd
        // rejects a `limit` of exactly `1 << 16` (= the old `MAX_SUBTREE_ROOTS_PER_POOL`)
        // with `RPC_INVALID_PARAMETER` ("Invalid params") — the subtree index space for
        // a depth-32 tree is `0..2^16`, so `2^16` is one past the inclusive maximum.
        // (Verified on-device against ECC lightwalletd v0.4.19 / zec.rocks: `limit`
        // 0..=65535 all return roots; 65536 fails. This was the on-device "can't reach
        // the server" sync stall — a SERVER InvalidArgument the engine mislabelled as
        // EndpointUnreachable.) The anti-flood defense is unchanged and remains the
        // CLIENT-SIDE count cap: `collect_roots` stops at `MAX_SUBTREE_ROOTS_PER_POOL`
        // regardless of what the server streams (a malicious server ignores any hint
        // anyway — the client cap was always "the real defense"; since S15-F1 it
        // counts the ABSOLUTE index, `start + served`).
        self.get_subtree_roots(protocol, start_index, 0).await
    }
}

/// EVERY shielded pool whose subtree roots this build ingests, in the order the
/// RPCs are issued. **The set is written down exactly once, here**, and
/// [`fetch_subtree_roots`] iterates it — which is the half of T0-1 that is not
/// "add Ironwood".
///
/// **Why a set and not a third branch.** Ironwood was added to the wire protocol,
/// to `PoolType`, to `ValuePool`, to `TreeState`, to the shard tables and to
/// upstream's own reference loop, and none of that added it *here*, because "which
/// pools do we fetch roots for" was not a thing this module had — it was three
/// hand-unrolled statements that nobody had a reason to revisit. NU6.3 is not the
/// last network upgrade, so the fix has to be the thing that fails when the next
/// one lands:
///
/// 1. `fetch_subtree_roots`'s per-pool `match` is EXHAUSTIVE over
///    `ShieldedProtocol` with **no wildcard arm** — upstream adding a fourth
///    variant is a COMPILE ERROR there, so the pool cannot arrive unnoticed.
/// 2. `put_subtree_roots` destructures [`SubtreeRoots`] BY NAME, so a fourth
///    collected pool that nobody wrote is a compile error too.
/// 3. `testing::FakeRootSource` matches exhaustively for the same reason, so the
///    double can never claim a capability production does not have.
///
/// **The cost, and the residual gap, stated rather than hidden.** This does NOT
/// make a new pool work by itself: adding one still needs a leaf parser, a
/// `put_*_subtree_roots` call and an activation height. And membership of THIS
/// array is not itself compiler-forced — Rust cannot enumerate an enum's variants,
/// so a future author who adds `Nu7` to the `match` arms but forgets this array
/// would write an arm that is never reached. The array and the match therefore name
/// each other explicitly. What the structure buys is real but bounded: the
/// compiler names three sites on the day the variant appears, instead of the pool
/// being quietly absent for two network upgrades and surfacing as unspendable
/// money.
pub(crate) const SUBTREE_ROOT_POOLS: &[ShieldedProtocol] = &[
    ShieldedProtocol::Sapling,
    ShieldedProtocol::Orchard,
    ShieldedProtocol::Ironwood,
];

/// **The wire values, asserted at COMPILE time.** `GetSubtreeRootsArg.shielded_protocol`
/// takes `ShieldedProtocol`, where Ironwood is **2**
/// (`zcash_client_backend-0.24.0/src/proto/service.rs:334-338`). Two OTHER upstream
/// enums name the same pool with a different number — `PoolType::Ironwood` and
/// `proposal::ValuePool::Ironwood` are both **4** — and sending 4 lands on
/// lightwalletd's unrecognized-protocol path, which is a hard error whose code and
/// message differ by server version. The names are identical, so nothing but the
/// number distinguishes a correct call from a silently wrong one; here is the
/// number.
///
/// The order is asserted too, because `SubtreeRoots::from_pools` (test-only) fills its
/// per-pool outcome array positionally.
const _: () = assert!(
    (SUBTREE_ROOT_POOLS[0] as i32) == 0
        && (SUBTREE_ROOT_POOLS[1] as i32) == 1
        && (SUBTREE_ROOT_POOLS[2] as i32) == 2,
    "GetSubtreeRoots wire values: Sapling=0, Orchard=1, Ironwood=2 — NOT PoolType/ValuePool's 4"
);

/// What the endpoint actually DID for one pool on one fetch — kept separate from
/// the root count so "served nothing" and "refused the protocol" can never be read
/// as the same answer (A9/A10). Recorded for every pool in [`SUBTREE_ROOT_POOLS`],
/// so no pool's outcome can be dropped by being forgotten.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PoolFetch {
    /// The endpoint streamed completed subtree roots and closed cleanly, and after
    /// this pass the pool's tree holds `roots` of them from index 0 — the ABSOLUTE
    /// count `start + served` (S15-F1): an incremental serve of 3 roots from index
    /// 1,125 is `Served { roots: 1_128 }`, which is what [`withheld`] compares to
    /// the bundle's proof and what `PoolService` exports.
    ///
    /// `roots == 0` is a LEGITIMATE state and is deliberately not an error: it is
    /// every pool's state before its first 2^16 notes, and the state NU7's pool
    /// will be in the day it activates (A7 ii). It is also the cheapest attack on
    /// this whole item (A10) — an endpoint that accepts protocol 2, opens the
    /// stream and closes it with zero roots leaves `subtree_end_height` NULL, which
    /// is exactly the stuck state, with no error and no stall. An empty OK stream
    /// and a genuinely empty pool are the same bytes, so this seam cannot tell them
    /// apart and does not pretend to; what it does is make the zero DISTINCT and
    /// visible — a separate value here plus a `warn!` — instead of indistinguishable
    /// from a healthy fetch.
    Served { roots: usize },
    /// The endpoint refused the protocol outright (`GrpcError::ShieldedProtocolUnknown`
    /// — `Unknown (2)` on lightwalletd v0.5.3, `InvalidArgument (3)` on v0.5.4).
    /// NEVER to be rendered as "the pool is empty": nothing was learned about the
    /// pool, and the roots we already hold for it are untouched.
    Unsupported,
    /// **T0-1a's fourth outcome.** The endpoint gave a completion height for this
    /// pool that cannot be true, and [`crate::root_bind`] refused it.
    ///
    /// **TWO producers since BIND-1 (§4x), and they differ in what has already
    /// happened by the time this is raised.** At INGEST ([`apply_height_bind`])
    /// nothing from this pool is written and the heights already recorded for it
    /// are untouched. At SCAN time ([`reconcile_scan_boundaries`]) the height was
    /// written on an earlier pass and the wallet's own scanned counts have since
    /// refuted it, so the row has ALREADY been corrected — to the height the scan
    /// counted, or to NULL — and the stabilization latch it licensed undone,
    /// before this outcome is raised. What the two share, and all the surface
    /// needs: **no completion height this endpoint chose is standing for this
    /// pool.** `code` says which oracle caught it.
    ///
    /// A FOURTH state, not a shade of the other three. "The endpoint lied about
    /// heights", "the pool is empty" and "the server does not know this pool" are
    /// three different sentences and the conflation of any two of them is the exact
    /// defect (A10) this whole item chain exists to stop. `code` names which oracle
    /// caught it ([`crate::root_bind::HeightBindRefusal::code`]) and is §5.4-safe: a
    /// stable string, no height, no note, no account, no endpoint identity.
    ///
    /// Only reachable for **Ironwood**. The two required pools abort the pass
    /// instead — see [`apply_height_bind`] for the disposition and what it costs.
    HeightViolation { code: &'static str },
    /// **T0-1b's fifth outcome (adjudication (b)), widened by T0-1d.** The endpoint
    /// served FEWER roots — none, or a strict prefix — than the SIGNED BUNDLE
    /// proves complete for this pool at or below the endpoint's own reported tip
    /// (`proven`). A `Served { roots }` that survives that check is a serve the
    /// wallet has no proof against and stays `Served`; this one is a server
    /// serving less than the network has — the A10 stuck state with a witness,
    /// for every note above the last served subtree. Computed in
    /// [`apply_height_bind`] ([`withheld`]), where the bundled rows are already
    /// in hand, and reachable for EVERY pool. This REPORTS, it does not refuse: a
    /// zero serve writes nothing and leaves the pool's rewind relaxation armed; a
    /// short serve writes its prefix (the count bind accepts a prefix) and
    /// consumes the relaxation like any accepted write. The surface carries
    /// `proven` only — the served count is on the log line
    /// (`report_under_served`); the trade is stated at [`withheld`].
    Withheld { proven: u64 },
}

/// The collected subtree roots for every pool in [`SUBTREE_ROOT_POOLS`] (the output
/// of [`fetch_subtree_roots`], the input to [`put_subtree_roots`]). `Send` so it
/// crosses into the `spawn_blocking` DB write.
///
/// ⚠️ `orchard` and `ironwood` have the **identical Rust type** —
/// `Vec<CommitmentTreeRoot<MerkleHashOrchard>>`, because Ironwood shares Orchard's
/// tree shape and leaf hash upstream (`zcash_client_sqlite-0.22.0/src/lib.rs:3257`
/// vs `:3288`). `SubtreeRoots { orchard: x, ironwood: x }` therefore COMPILES, and
/// a cross-wire between the two trees is invisible to the type system. Whatever
/// asserts on this struct has to read each pool's roots BACK from its own tree.
pub(crate) struct SubtreeRoots {
    pub(crate) sapling: Vec<CommitmentTreeRoot<SaplingNode>>,
    pub(crate) orchard: Vec<CommitmentTreeRoot<MerkleHashOrchard>>,
    pub(crate) ironwood: Vec<CommitmentTreeRoot<MerkleHashOrchard>>,
    /// The wire `SubtreeRoot.completing_block_hash` for each root, one inner
    /// vector per pool in [`SUBTREE_ROOT_POOLS`] order, positionally aligned with
    /// the three root vectors above.
    ///
    /// Carried rather than dropped at parse time (it used to be parsed and
    /// discarded — nothing in this SDK read it and `zcash_client_sqlite` never
    /// reads it either) because it is the ONE field that ties an endpoint to a
    /// block this wallet may already have scanned: `root_bind`'s C6 cross-check.
    /// An empty entry means the endpoint served no hash for that root, and the
    /// cross-check abstains for it.
    completing_hashes: [Vec<Vec<u8>>; SUBTREE_ROOT_POOLS.len()],
    /// One slot per pool, positionally aligned with [`SUBTREE_ROOT_POOLS`].
    outcomes: [PoolFetch; SUBTREE_ROOT_POOLS.len()],
    /// The shard index each pool's roots START at, one slot per pool in
    /// [`SUBTREE_ROOT_POOLS`] order (S15-F1, ADR-0569): `0` for a full fetch —
    /// including a retry, a fallback and a serve re-classified as full — and the
    /// plan's start for an incremental one. [`put_subtree_roots`] writes each pool
    /// at its start, the bind checks the virtual sequence below it
    /// ([`root_bind::check_pool_from`]), and [`Self::written_range`] reads it.
    starts: [u32; SUBTREE_ROOT_POOLS.len()],
}

impl SubtreeRoots {
    /// An empty set with every pool recorded as having served nothing — the shape a
    /// caller gets before any RPC, and the base case the fetch loop fills in.
    fn empty() -> Self {
        Self {
            sapling: Vec::new(),
            orchard: Vec::new(),
            ironwood: Vec::new(),
            completing_hashes: [const { Vec::new() }; SUBTREE_ROOT_POOLS.len()],
            outcomes: [PoolFetch::Served { roots: 0 }; SUBTREE_ROOT_POOLS.len()],
            starts: [0; SUBTREE_ROOT_POOLS.len()],
        }
    }

    /// How many roots this pass carries for `pool` (the served length, not the
    /// absolute count). EXHAUSTIVE over the pool enum for the reason every other
    /// per-pool match in this module is: a fourth pool must name its own vector or
    /// fail to compile, rather than silently reporting another pool's length.
    fn served_len(&self, pool: ShieldedProtocol) -> usize {
        match pool {
            ShieldedProtocol::Sapling => self.sapling.len(),
            ShieldedProtocol::Orchard => self.orchard.len(),
            ShieldedProtocol::Ironwood => self.ironwood.len(),
        }
    }

    /// The shard index `pool`'s roots start at (0 for a full fetch).
    pub(crate) fn start_of(&self, pool: ShieldedProtocol) -> u32 {
        SUBTREE_ROOT_POOLS
            .iter()
            .position(|p| *p == pool)
            .map_or(0, |slot| self.starts[slot])
    }

    /// The shard indices [`put_subtree_roots`] will write for `pool` —
    /// `[start, start + served)`, empty when nothing is to be written. The roots are
    /// written positionally from the pool's start, so this is exactly the set of
    /// indices the write covers.
    ///
    /// BIND-1-R's consumer is `root_bind::clear_boundary_bounds_in`: an accepted
    /// write supersedes the scan's refutation at the indices it actually rewrote and
    /// says nothing about the others, so neither a short serve nor an incremental one
    /// may clear a bracket it did not reach.
    pub(crate) fn written_range(&self, pool: ShieldedProtocol) -> std::ops::Range<u64> {
        let start = u64::from(self.start_of(pool));
        start..start + self.served_len(pool) as u64
    }

    /// Assemble a value from already-collected roots, recording every pool as
    /// having SERVED what it holds. For a caller that built the roots itself rather
    /// than streaming them — the fetch path records outcomes as it goes, because
    /// "served nothing" and "refused the protocol" are not recoverable from the
    /// vectors afterwards.
    ///
    /// Leaves `completing_hashes` EMPTY, so `root_bind`'s C6 cross-check abstains
    /// for every root in the result. That is the honest shape for a caller that did
    /// not stream the roots: it never saw the wire field, so it has nothing to
    /// attest. The height binds are unaffected — they read the DB and the bundle,
    /// not the wire.
    #[cfg(test)]
    pub(crate) fn from_pools(
        sapling: Vec<CommitmentTreeRoot<SaplingNode>>,
        orchard: Vec<CommitmentTreeRoot<MerkleHashOrchard>>,
        ironwood: Vec<CommitmentTreeRoot<MerkleHashOrchard>>,
    ) -> Self {
        let mut out = Self::empty();
        out.outcomes = [
            PoolFetch::Served {
                roots: sapling.len(),
            },
            PoolFetch::Served {
                roots: orchard.len(),
            },
            PoolFetch::Served {
                roots: ironwood.len(),
            },
        ];
        out.sapling = sapling;
        out.orchard = orchard;
        out.ironwood = ironwood;
        out
    }

    /// What the endpoint did for `pool` on the fetch that produced this value.
    /// `None` only for a protocol this build does not ingest at all.
    #[cfg(test)]
    pub(crate) fn outcome(&self, pool: ShieldedProtocol) -> Option<PoolFetch> {
        SUBTREE_ROOT_POOLS
            .iter()
            .position(|p| *p == pool)
            .map(|slot| self.outcomes[slot])
    }
}

// ── S15-F1 phase B: fetch only the roots the wallet does not hold (ADR-0569) ──
//
// The design and its guarantee ledger: `docs/plan/s15-f1-subtree-roots-fetch-only-
// what-is-new.md` §3–§4. In one sentence: every pool is planned from what this
// wallet already stores and what this SESSION has already verified (the memo,
// below); a pool that is not provably settled fetches from 0 as before, and a
// settled one fetches from the last stored root (or earlier, rounded down), with
// every write still bound against the whole virtual sequence under one lock.

/// Why a pool's plan is a full fetch from index 0 — the first of these that holds,
/// in this order ([`plan_pool_fetch`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FullReason {
    /// This session has not yet completed a full, BOUND (non-relaxed) fetch of the
    /// pool: a fresh `Inner` (open, server switch, rescan) or the pass after a
    /// relaxed bind.
    Unverified,
    /// A rewind is outstanding for the pool, or the chain's rewind counter moved
    /// since the memo last saw it.
    Rewound,
    /// The wallet stores no complete root for the pool (`read_stored_run == 0`).
    NothingStored,
    /// This server refused, ignored or mis-served a non-zero start earlier in the
    /// session.
    IncrementalOff,
    /// [`SUBTREE_ROOTS_FULL_VERIFY_PASSES`] incremental passes since the last full.
    PassesDue,
    /// [`SUBTREE_ROOTS_FULL_VERIFY_SECS`] since the last full on `Inner.clock`, or a
    /// clock that reads earlier than it (untrusted).
    TimeDue,
}

/// One pool's fetch plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FetchPlan {
    /// Fetch every root from index 0.
    Full(FullReason),
    /// Fetch from shard index `start` (a multiple of
    /// [`SUBTREE_ROOTS_START_GRANULARITY`], below the stored run's length), and
    /// expect the first served root's completion height to be `overlap_height` —
    /// the height this wallet recorded at `start`. `start == 0` is a full fetch for
    /// the memo's bookkeeping.
    From { start: u32, overlap_height: u32 },
}

/// What one session has learned about one pool's roots — in memory only, one per
/// slot of [`SUBTREE_ROOT_POOLS`] (`Inner.root_ingest`). Built fresh with every
/// `Inner`, never carried and never persisted: a session re-measures its endpoint.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PoolMemo {
    /// This session completed a full, BOUND (non-relaxed) fetch of the pool.
    pub(crate) verified: bool,
    /// `RewindWatch::observed` at the memo's last full fetch.
    pub(crate) chain_rewinds_seen: i64,
    /// Incremental passes since the last full fetch.
    pub(crate) passes_since_full: u32,
    /// `Inner.clock`'s `now_unix()` at the last full fetch.
    pub(crate) last_full_at: u64,
    /// This server refused, ignored, mis-served or timed out a non-zero start
    /// once, or dropped it twice in a row; the pool fetches from 0 for the rest of
    /// the session.
    pub(crate) incremental_off: bool,
    /// `roots_start_dropped`s in a row for this pool, counting only a drop whose
    /// retry from 0 was SERVED in the same pass (evidence the server resets
    /// non-zero starts, not that the path is down) — reset only when the locked
    /// section WROTE the pool from a `start > 0` serve. At
    /// [`SUBTREE_ROOTS_DROPS_OFF`] the pool turns incremental off: a server that
    /// resets every non-zero start would otherwise cost two streams on every pass
    /// of the session (the built-diff security review, LOW-1).
    pub(crate) consecutive_drops: u32,
    /// The C6 indices a bound pass covered, keyed to the scanned block hash it saw
    /// ([`root_bind::C6Seen`]).
    pub(crate) c6_seen: root_bind::C6Seen,
}

impl PoolMemo {
    /// Apply one pass's fetch signals for this pool — on the `Ok` AND the `Err`
    /// path (the one exception to "an `Err` pass leaves the memo untouched"): a
    /// turn-off is remembered; a drop whose retry from 0 was served
    /// (`IngestSignals::dropped`) counts toward [`SUBTREE_ROOTS_DROPS_OFF`] in a
    /// row; `written_incremental` — the locked section WROTE the pool from a
    /// `start > 0` serve, so `false` on every `Err` pass — resets the count. A
    /// `start > 0` stream drained whole but refused by the bind or validation, or
    /// never written because the pass failed, resets nothing.
    pub(crate) fn note_fetch(
        &mut self,
        incremental_off: bool,
        dropped: bool,
        written_incremental: bool,
    ) {
        if written_incremental {
            self.consecutive_drops = 0;
        }
        if dropped {
            self.consecutive_drops = self.consecutive_drops.saturating_add(1);
            if self.consecutive_drops >= SUBTREE_ROOTS_DROPS_OFF {
                self.incremental_off = true;
            }
        }
        if incremental_off {
            self.incremental_off = true;
        }
    }
}

/// `Inner.root_ingest`'s value: one [`PoolMemo`] per slot of
/// [`SUBTREE_ROOT_POOLS`]. Its mutex is taken only inside two synchronous helpers —
/// the plan, before the locked section, and the memo update after it — never while
/// `db` or `aux_db` is held, never across an `.await`.
#[derive(Debug, Default)]
pub(crate) struct RootIngestMemo {
    pub(crate) pools: [PoolMemo; SUBTREE_ROOT_POOLS.len()],
    /// Every SECOND STREAM this session's root ingestion opened (each retry and
    /// fallback re-fetch from 0; not an ignored start re-classified as full) —
    /// the counter a row meant to exercise the incremental path asserts zero on.
    #[cfg(test)]
    pub(crate) fallbacks: u64,
}

/// What the plan reads for one pool, under `db` then `aux_db`, before the network
/// await ([`PoolSnapshot::read`]). It reads NO stored root-hash VALUE.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PoolSnapshot {
    /// [`root_bind::read_stored_run`]: the leading run of complete roots.
    pub(crate) count: u64,
    /// [`root_bind::read_recorded_heights`]; every index below `count` is `Some`.
    pub(crate) recorded: Vec<Option<u32>>,
    /// [`root_bind::read_boundary_bounds`].
    pub(crate) bounds: std::collections::HashMap<usize, root_bind::BoundaryBounds>,
    /// [`root_bind::read_rewind_watch`].
    pub(crate) rewind: root_bind::RewindWatch,
    /// `blocks.hash` for the recorded completing heights below `count` this
    /// wallet has scanned ([`root_bind::read_scanned_hashes`]).
    pub(crate) scanned_hashes: std::collections::HashMap<u32, Vec<u8>>,
}

impl PoolSnapshot {
    /// Read one pool's snapshot through the aux connection (the caller holds `db`
    /// first — the documented order).
    pub(crate) fn read(
        aux: &rusqlite::Connection,
        pool: ShieldedProtocol,
    ) -> Result<Self, WalletError> {
        let count = root_bind::read_stored_run(aux, pool)?;
        let recorded = root_bind::read_recorded_heights(aux, pool)?;
        let stored: Vec<u32> = recorded
            .iter()
            .take(usize::try_from(count).map_err(|_| internal_fault())?)
            .flatten()
            .copied()
            .collect();
        Ok(Self {
            count,
            scanned_hashes: root_bind::read_scanned_hashes(aux, &stored)?,
            recorded,
            bounds: root_bind::read_boundary_bounds(aux, pool)?,
            rewind: root_bind::read_rewind_watch(aux, pool)?,
        })
    }
}

/// `Sync { Internal }` — a fault on this device, never the endpoint's.
fn internal_fault() -> WalletError {
    WalletError::Sync {
        stall: StallReason::Internal,
    }
}

/// **Plan one pool's fetch** (pure; S15-F1 §3.2). `tip` is this pass's endpoint
/// tip, `scanned_tip` the wallet's own scanned height read before it, `now`
/// `Inner.clock`'s reading.
///
/// A full fetch from 0, with the first [`FullReason`] that holds; otherwise
/// `raw = min` of the terms that exist:
///
/// 1. `count − 1` — re-serve the last stored root;
/// 2. the first `i < count` whose recorded height is above `window_floor =
///    min(tip, scanned_tip.unwrap_or(tip)) − REORG_MAX_BLOCKS` — the lower height,
///    so no endpoint number can shrink the window;
/// 3. `lowest bracket − 1` — anchor below a record the scan has already shown to
///    be wrong. A bracket at index 0 saturates to 0, so the plan is
///    `From { start: 0 }` rather than `Full`: the fetch is from 0 all the same, and
///    the memo books it as a FULL fetch (`start == 0`) — the plan's "Full if that
///    is below 0" read as "a start of 0" (the build's ruling 3);
/// 4. the first `i < count` whose completing block is scanned and whose
///    `c6_seen` entry is missing or names another height or block hash.
///
/// `start = floor(raw / 64) × 64` — only ever DOWN, a larger re-serve
/// ([`SUBTREE_ROOTS_START_GRANULARITY`]), and `overlap_height = recorded[start]`.
/// A `None` at an index below `count` is `Sync { Internal }`: the stored run
/// promised a height there.
pub(crate) fn plan_pool_fetch(
    memo: &PoolMemo,
    snapshot: &PoolSnapshot,
    tip: u32,
    scanned_tip: Option<u32>,
    now: u64,
) -> Result<FetchPlan, WalletError> {
    let rewind = snapshot.rewind;
    let reason = if !memo.verified {
        Some(FullReason::Unverified)
    } else if rewind.rewound_since_record() || rewind.observed != memo.chain_rewinds_seen {
        Some(FullReason::Rewound)
    } else if snapshot.count == 0 {
        Some(FullReason::NothingStored)
    } else if memo.incremental_off {
        Some(FullReason::IncrementalOff)
    } else if memo.passes_since_full >= SUBTREE_ROOTS_FULL_VERIFY_PASSES {
        Some(FullReason::PassesDue)
    } else if now < memo.last_full_at
        || now.saturating_sub(memo.last_full_at) >= SUBTREE_ROOTS_FULL_VERIFY_SECS
    {
        Some(FullReason::TimeDue)
    } else {
        None
    };
    if let Some(reason) = reason {
        return Ok(FetchPlan::Full(reason));
    }

    let count = usize::try_from(snapshot.count).map_err(|_| internal_fault())?;
    let stored: Vec<u32> = snapshot
        .recorded
        .get(..count)
        .ok_or_else(internal_fault)?
        .iter()
        .map(|h| h.ok_or_else(internal_fault))
        .collect::<Result<_, _>>()?;

    // Term 1 always exists (`count >= 1` here).
    let mut raw = count - 1;
    // Term 2: the reorg window.
    let window_floor = tip
        .min(scanned_tip.unwrap_or(tip))
        .saturating_sub(REORG_MAX_BLOCKS);
    if let Some(i) = stored.iter().position(|&h| h > window_floor) {
        raw = raw.min(i);
    }
    // Term 3: below the lowest bracket. A bracket at 0 saturates to a start of
    // 0 — `From { start: 0 }`, which the memo books as a full fetch.
    if let Some(&lowest) = snapshot.bounds.keys().min() {
        raw = raw.min(lowest.saturating_sub(1));
    }
    // Term 4: a scanned completing block this session has not hash-checked at its
    // current scanned hash.
    let unchecked = stored.iter().enumerate().position(|(i, &height)| {
        snapshot.scanned_hashes.get(&height).is_some_and(|ours| {
            !memo
                .c6_seen
                .get(&(i as u64))
                .is_some_and(|(seen_height, seen_hash)| {
                    *seen_height == height && seen_hash.as_slice() == ours.as_slice()
                })
        })
    });
    if let Some(i) = unchecked {
        raw = raw.min(i);
    }

    let granularity = SUBTREE_ROOTS_START_GRANULARITY as usize;
    let start = raw / granularity * granularity;
    Ok(FetchPlan::From {
        start: u32::try_from(start).map_err(|_| internal_fault())?,
        overlap_height: stored[start],
    })
}

/// What the fetch needs from one pool's plan: the plan itself, the stored run's
/// length (the short-serve check), and the height recorded at index 0 (the
/// ignored-start check). `first_recorded` is `None` exactly when `count == 0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlannedPool {
    pub(crate) plan: FetchPlan,
    pub(crate) count: u64,
    pub(crate) first_recorded: Option<u32>,
}

impl PlannedPool {
    /// A full fetch — what [`fetch_subtree_roots`] asks for every pool. The reason
    /// never reaches the fetch; `Unverified` is the one a caller with no memo has.
    pub(crate) const fn full() -> Self {
        Self {
            plan: FetchPlan::Full(FullReason::Unverified),
            count: 0,
            first_recorded: None,
        }
    }

    /// Pair a plan with the snapshot it was made from.
    pub(crate) fn from_snapshot(plan: FetchPlan, snapshot: &PoolSnapshot) -> Self {
        Self {
            plan,
            count: snapshot.count,
            first_recorded: snapshot.recorded.first().copied().flatten(),
        }
    }
}

/// What the fetch carries OUT besides the roots, on the `Ok` AND the `Err` path:
/// which pools this server turned incremental off for (§3.3 steps 2–3), which
/// pools' `start > 0` stream DROPPED with its retry from 0 served (the memo's
/// consecutive-drop count — the reset is the locked section's write, not a
/// fetch signal), and how many SECOND STREAMS it opened (every retry and
/// fallback re-fetch from 0; an ignored start re-classified as full opens none
/// and is not counted).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IngestSignals {
    pub(crate) incremental_off: [bool; SUBTREE_ROOT_POOLS.len()],
    /// A `roots_start_dropped` that COUNTS: the pool's `start > 0` stream hit a
    /// transport fault, and its re-fetch from 0 in the same pass was served. A
    /// drop whose retry also fails says the path is down, not that the server
    /// resets non-zero starts, and is not set here.
    pub(crate) dropped: [bool; SUBTREE_ROOT_POOLS.len()],
    pub(crate) fallbacks: u32,
}

/// What the memo update needs from one pool's locked section (S15-F1 §3.3 step 5).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PoolIngest {
    /// The pool's put wrote at least one root this pass.
    pub(crate) written: bool,
    /// The index the written roots start at (0 = a full fetch).
    pub(crate) start: u32,
    /// The bind ran relaxed (a rewind was outstanding).
    pub(crate) relaxed: bool,
    /// The chain rewind count the bind read.
    pub(crate) observed: i64,
    /// The bind's C6 set ([`root_bind::check_pool_from`]).
    pub(crate) c6: root_bind::C6Seen,
}

/// [`fetch_subtree_roots_planned`]'s result.
pub(crate) struct PlannedFetch {
    pub(crate) roots: Result<SubtreeRoots, WalletError>,
    pub(crate) signals: IngestSignals,
}

/// A `SubtreeRoot` from the (untrusted) endpoint failed to decode (§4.6 hostile
/// input): a `root_hash` not exactly 32 bytes or not a canonical field element, or
/// a completing height beyond `u32`. Crate-internal — mapped to a typed `Sync`
/// error (the endpoint is unusable), NEVER a panic, never silent.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SubtreeRootInvalid;

/// The wire `root_hash` as a 32-byte array — any other length is a hostile shape
/// (validate before the field decode, §4.6).
fn root_hash_bytes(root: &SubtreeRoot) -> Result<[u8; 32], SubtreeRootInvalid> {
    <[u8; 32]>::try_from(root.root_hash.as_slice()).map_err(|_| SubtreeRootInvalid)
}

/// The completing-block height as a consensus `BlockHeight` — a value beyond `u32`
/// is a garbage/hostile response (reject, never truncate, unlike the reference's
/// `as u32`).
fn completing_height(root: &SubtreeRoot) -> Result<BlockHeight, SubtreeRootInvalid> {
    u32::try_from(root.completing_block_height)
        .map(BlockHeight::from_u32)
        .map_err(|_| SubtreeRootInvalid)
}

/// Decode one Sapling subtree root (the wire `root_hash` is a canonical jubjub
/// field element; `from_bytes` returns `CtOption` — `None` on a non-canonical
/// encoding). Variable-time `Option::from` is correct: a PUBLIC root's validity is
/// not secret. Byte-identical decode to the reference's `sapling::Node::read`.
fn parse_sapling_root(
    root: &SubtreeRoot,
) -> Result<CommitmentTreeRoot<SaplingNode>, SubtreeRootInvalid> {
    let node: Option<SaplingNode> = SaplingNode::from_bytes(root_hash_bytes(root)?).into();
    Ok(CommitmentTreeRoot::from_parts(
        completing_height(root)?,
        node.ok_or(SubtreeRootInvalid)?,
    ))
}

/// Decode one Orchard subtree root (canonical pallas field element; same
/// hostile-input contract as [`parse_sapling_root`]).
fn parse_orchard_root(
    root: &SubtreeRoot,
) -> Result<CommitmentTreeRoot<MerkleHashOrchard>, SubtreeRootInvalid> {
    let node: Option<MerkleHashOrchard> =
        MerkleHashOrchard::from_bytes(&root_hash_bytes(root)?).into();
    Ok(CommitmentTreeRoot::from_parts(
        completing_height(root)?,
        node.ok_or(SubtreeRootInvalid)?,
    ))
}

/// Decode one Ironwood subtree root. Ironwood shares Orchard's tree shape and leaf
/// hash upstream, so this is [`parse_orchard_root`] under its own name — spelled
/// out rather than aliased so the fetch loop's Ironwood arm names Ironwood, and so
/// the day the two shapes diverge there is a function to change instead of a call
/// site to notice.
fn parse_ironwood_root(
    root: &SubtreeRoot,
) -> Result<CommitmentTreeRoot<MerkleHashOrchard>, SubtreeRootInvalid> {
    parse_orchard_root(root)
}

/// Why a pool's root fetch stopped. Separated from [`WalletError`] because exactly
/// ONE of these is survivable and only for a pool the caller decides is optional
/// (A9) — everything else is a fault of the same weight it has always had.
#[derive(Debug)]
pub(crate) enum RootFetchFault {
    /// The endpoint does not know this shielded protocol. Nothing is known about
    /// the pool; it is NOT empty.
    ProtocolUnknown,
    /// Already classified into the SDK taxonomy (transport, timeout, hostile root,
    /// flood cap, a height sequence that cannot be true).
    Fatal(WalletError),
}

impl From<RootFetchFault> for WalletError {
    /// The fail-closed reading, for a pool whose roots are REQUIRED: an endpoint
    /// that cannot serve a pool we must have is an endpoint we cannot sync from —
    /// clean transport, unusable data.
    fn from(f: RootFetchFault) -> Self {
        match f {
            RootFetchFault::ProtocolUnknown => endpoint_unusable(),
            RootFetchFault::Fatal(e) => e,
        }
    }
}

/// The endpoint-controlled `completing_block_height` sequence, checked BEFORE any
/// of it can reach the DB (**A11**).
///
/// **What this number does once it is written.** `put_shard_roots` lands it via
/// `ON CONFLICT … DO UPDATE SET subtree_end_height = :subtree_end_height`
/// (`zcash_client_sqlite-0.22.0/src/wallet/commitment_tree.rs:1298-1301`) — an
/// UNCONDITIONAL overwrite with no check on the height, re-writable on every sync
/// by an endpoint whose `root_hash`es are otherwise fine. That one number then
/// drives `mark_stabilized_notes`' `subtree_end_height <= :pruning_floor`, the join
/// window of the per-pool unscanned-ranges view, AND `min_shard_tip`. An endpoint
/// that sets it just above the activation can mark notes `witness_stabilized` on
/// shards the wallet has never scanned: a spendable balance with no constructible
/// witness.
///
/// Three properties, all of which any honest server satisfies with enormous margin
/// (measured against `zec.rocks` mainnet + testnet, 2026-09-08):
///
/// * **strictly increasing** — subtree *i* completes before subtree *i+1*, and two
///   subtrees cannot complete in the same block: that needs one block carrying a
///   whole further subtree's worth of note commitments (2^16 = 65,536), which the
///   consensus block size limit puts orders of magnitude out of reach;
/// * **first root at or above the pool's activation** — no subtree of a pool can
///   complete before the pool exists. Sapling's first mainnet subtree completes at
///   558,822 against an activation of 419,200; Ironwood's at 3,451,206 against
///   3,428,143;
/// * **every root at or below the endpoint's OWN reported tip** — a server may not
///   claim a subtree completed in a block it has not seen. Checked per root rather
///   than only on the last, which is strictly stronger and fails on the first bad
///   one.
///
/// A violation is a typed `Sync` fault for the WHOLE pool — never a partial accept,
/// because `put_shard_roots` writes positionally from `start_index` and a truncated
/// prefix would be silently wrong rather than merely short.
///
/// **Over HEIGHTS, from shard index 0 (S15-F1).** A full fetch passes the served
/// heights. An incremental one passes the virtual sequence `recorded[..start] ++
/// served` ([`root_bind::virtual_heights`]), so the activation floor still applies
/// to index 0, monotonicity holds across the seam between the stored prefix and the
/// serve, and every prefix height is held to THIS pass's tip as a full re-serve
/// would be — never the suffix alone.
///
/// `floor` is `None` only when this build's params have no activation height for
/// the pool on this network (the regtest shape). Both networks this SDK ships know
/// all three activations, so `None` is unreachable in production; it is not treated
/// as a licence to skip the other two checks.
fn validate_root_sequence(
    heights: &[u32],
    floor: Option<BlockHeight>,
    ceiling: BlockHeight,
) -> Result<(), WalletError> {
    let mut prev: Option<BlockHeight> = None;
    for &height in heights {
        let h = BlockHeight::from_u32(height);
        match prev {
            None => {
                if floor.is_some_and(|f| h < f) {
                    return Err(endpoint_unusable());
                }
            }
            Some(p) if h <= p => return Err(endpoint_unusable()),
            Some(_) => {}
        }
        if h > ceiling {
            return Err(endpoint_unusable());
        }
        prev = Some(h);
    }
    Ok(())
}

/// Drain a [`SubtreeRootStream`], converting each root with `parse`. A transport
/// fault surfaces typed `Sync { stall }` (the stream's SPENT-ON-ERROR latch
/// already makes any re-poll loud); a malformed root makes the endpoint unusable
/// (`Sync { EndpointUnreachable }`). DRY: the two pool collectors differ only in
/// the leaf type + parser.
///
/// HOSTILE-COUNT CAP (§4.6, security fold): the per-message idle timeout has no
/// TOTAL bound (a slow-but-alive link must survive — §3.2d), so a flooding endpoint
/// could stream well-formed roots forever and OOM the sync task. The accumulator is
/// hard-capped at [`MAX_SUBTREE_ROOTS_PER_POOL`] (the consensus-fixed maximum number
/// of completable subtrees) — beyond it the endpoint is provably lying → typed
/// `Sync`, never unbounded growth, never a silent truncate.
///
/// The mid-drain REFUSAL door (A9): lightwalletd may reject an unrecognized
/// `shielded_protocol` at stream open OR on the first pull, depending on whether it
/// has already flushed headers — the probe recorded the status CODE, not the door.
/// `SubtreeRoot`'s stream classifier catches both, and this pump reports the
/// refusal as [`RootFetchFault::ProtocolUnknown`] rather than folding it into a
/// `WalletError`, so the caller — and only the caller — decides whether this pool
/// is one it can do without.
/// **T0-1a:** the drain also keeps each root's `completing_block_hash`, which used
/// to be parsed and discarded. It is returned as a PARALLEL vector rather than
/// folded into `CommitmentTreeRoot` because that type is upstream's and carries
/// only what upstream writes; the pairing is positional and the two vectors are
/// always the same length by construction (one push each per accepted root).
async fn collect_roots_with_hashes<H>(
    stream: &mut SubtreeRootStream,
    parse: impl Fn(&SubtreeRoot) -> Result<CommitmentTreeRoot<H>, SubtreeRootInvalid>,
) -> Result<(Vec<CommitmentTreeRoot<H>>, Vec<Vec<u8>>), RootFetchFault> {
    match drain_roots(stream, parse, 0, None).await {
        Ok(Drained::Served { roots, hashes, .. }) => Ok((roots, hashes)),
        // Unreachable without a judge; kept honest rather than `unreachable!()`.
        Ok(Drained::OverlapMoved) => Err(RootFetchFault::Fatal(internal_fault())),
        Err(fault) => Err(fault.into_fetch_fault()),
    }
}

/// Why a drain stopped, before the caller decides what it costs: a full fetch
/// folds it into [`RootFetchFault`] exactly as it always did
/// ([`DrainFault::into_fetch_fault`]); an incremental one reads it to choose
/// between the same-pass retry from 0 and today's fatal answer (§3.3 step 2).
#[derive(Debug)]
enum DrainFault {
    /// The stream itself failed: open-to-close transport, a timeout, or a status.
    Grpc(GrpcError),
    /// A root that will not decode.
    Decode,
    /// More roots than the index space holds — the flood cap.
    Flood,
}

impl DrainFault {
    fn into_fetch_fault(self) -> RootFetchFault {
        match self {
            Self::Grpc(GrpcError::ShieldedProtocolUnknown { .. }) => {
                RootFetchFault::ProtocolUnknown
            }
            Self::Grpc(e) => RootFetchFault::Fatal(transport_err(e)),
            // a flood and a root that will not decode are the same class, never a
            // dead link: clean transport, unusable data (T0-1b)
            Self::Decode | Self::Flood => RootFetchFault::Fatal(endpoint_unusable()),
        }
    }
}

/// The first-root checks of an incremental serve (§3.3 step 3, in order).
#[derive(Clone, Copy, Debug)]
struct FirstRootJudge {
    /// The height this wallet recorded at index 0.
    first_recorded: Option<u32>,
    /// The height this wallet recorded at the plan's `start`.
    overlap_height: u32,
}

/// What a drain produced.
enum Drained<H> {
    /// The stream closed cleanly. `ignored`: the first root showed the server
    /// ignored `start_index` and served from index 0.
    Served {
        roots: Vec<CommitmentTreeRoot<H>>,
        hashes: Vec<Vec<u8>>,
        ignored: bool,
    },
    /// The first root's height is not the one recorded at `start`: stop draining.
    OverlapMoved,
}

/// The drain under [`collect_roots_with_hashes`] and the incremental fetch. With a
/// `judge` (a `start > 0` serve), the FIRST root decides as it arrives, before the
/// rest is drained: at `recorded[0]`'s height the server ignored `start_index` and
/// is serving from 0, so the serve is re-classified as a full fetch — `start := 0`
/// for the flood cap and everything after; otherwise a height other than
/// `overlap_height` stops the drain ([`Drained::OverlapMoved`]).
///
/// The flood cap is ABSOLUTE and at today's point (before the push): `start +
/// roots.len()` against [`MAX_SUBTREE_ROOTS_PER_POOL`], so an incremental serve
/// cannot buy itself a second index space by asking from a high start.
async fn drain_roots<H>(
    stream: &mut SubtreeRootStream,
    parse: impl Fn(&SubtreeRoot) -> Result<CommitmentTreeRoot<H>, SubtreeRootInvalid>,
    start: u32,
    judge: Option<FirstRootJudge>,
) -> Result<Drained<H>, DrainFault> {
    let mut start = start;
    let mut ignored = false;
    let mut roots = Vec::new();
    let mut hashes: Vec<Vec<u8>> = Vec::new();
    loop {
        let next = stream.next_root().await.map_err(DrainFault::Grpc)?;
        let Some(root) = next else { break };
        if u64::from(start) + roots.len() as u64 >= u64::from(MAX_SUBTREE_ROOTS_PER_POOL) {
            // more roots than the tree can ever hold ⇒ the endpoint is flooding us:
            // clean transport, unusable data — the content-tier class (T0-1b).
            return Err(DrainFault::Flood);
        }
        // a root that will not decode is the same class, never a dead link
        let parsed = parse(&root).map_err(|_| DrainFault::Decode)?;
        if roots.is_empty()
            && let Some(judge) = judge
        {
            let height = u32::from(parsed.subtree_end_height());
            if judge.first_recorded == Some(height) {
                start = 0;
                ignored = true;
            } else if height != judge.overlap_height {
                return Ok(Drained::OverlapMoved);
            }
        }
        roots.push(parsed);
        // THE BYTE DIMENSION IS CAPPED HERE, AT THE BOUNDARY (security
        // review, HIGH). The count cap above bounds how MANY entries this drain
        // retains; it says nothing about how BIG one is — each field is bounded
        // only by the 8 MiB per-message wire cap, so retaining it verbatim hands
        // a hostile endpoint count × message-size of resident allocation across
        // three pools before any validation runs. That is the same two-dimension
        // reasoning `DOWNLOAD_BATCH_MAX_BYTES` records ("the count bound alone
        // does NOT cover the byte dimension"), and an earlier version of this
        // line cited the count cap as if it did. A wrong-length hash can never
        // match anything — `root_bind::check_completing_hashes` filters on
        // exactly 32 bytes and ABSTAINS otherwise — so replacing it with an
        // empty entry here is behaviourally identical and drops the allocation.
        // Named test: `an_oversized_completing_block_hash_is_dropped_at_the_boundary`.
        hashes.push(if root.completing_block_hash.len() == 32 {
            root.completing_block_hash
        } else {
            Vec::new()
        });
    }
    Ok(Drained::Served {
        roots,
        hashes,
        ignored,
    })
}

/// The height-only drain: [`collect_roots_with_hashes`] with the wire block hashes
/// dropped. For the two callers that only ever wanted the roots.
#[cfg(test)]
async fn collect_roots<H>(
    stream: &mut SubtreeRootStream,
    parse: impl Fn(&SubtreeRoot) -> Result<CommitmentTreeRoot<H>, SubtreeRootInvalid>,
) -> Result<Vec<CommitmentTreeRoot<H>>, RootFetchFault> {
    collect_roots_with_hashes(stream, parse)
        .await
        .map(|(roots, _)| roots)
}

#[cfg(test)]
async fn collect_sapling_roots(
    stream: &mut SubtreeRootStream,
) -> Result<Vec<CommitmentTreeRoot<SaplingNode>>, WalletError> {
    collect_roots(stream, parse_sapling_root)
        .await
        .map_err(WalletError::from)
}

#[cfg(test)]
async fn collect_orchard_roots(
    stream: &mut SubtreeRootStream,
) -> Result<Vec<CommitmentTreeRoot<MerkleHashOrchard>>, WalletError> {
    collect_roots(stream, parse_orchard_root)
        .await
        .map_err(WalletError::from)
}

/// The pool's own activation height on `network` — the floor
/// [`validate_root_sequence`] holds the FIRST root to. Read from the compiled
/// consensus params, never written down here: a literal would be a second source of
/// truth for a consensus constant, which is the exact shape the Ironwood outage
/// came in (`consensus.rs`'s `KNOWN_UPGRADES` comment).
fn pool_activation(network: Network, upgrade: NetworkUpgrade) -> Option<BlockHeight> {
    network.consensus().activation_height(upgrade)
}

/// Open one pool's root stream FROM INDEX 0. A protocol REFUSAL at the open door
/// comes back as [`RootFetchFault::ProtocolUnknown`]; every other fault is
/// classified exactly as it always was.
async fn open_pool_stream<C: SubtreeRootSource + ?Sized>(
    client: &mut C,
    pool: ShieldedProtocol,
) -> Result<SubtreeRootStream, RootFetchFault> {
    match client.subtree_roots(pool, 0).await {
        Ok(s) => Ok(s),
        Err(GrpcError::ShieldedProtocolUnknown { .. }) => Err(RootFetchFault::ProtocolUnknown),
        Err(e) => Err(RootFetchFault::Fatal(transport_err(e))),
    }
}

/// One pool's served roots and where they start.
struct PoolServe<H> {
    start: u32,
    roots: Vec<CommitmentTreeRoot<H>>,
    hashes: Vec<Vec<u8>>,
}

impl<H> PoolServe<H> {
    /// The ABSOLUTE count the tree will hold from index 0 once this is written.
    fn absolute(&self) -> usize {
        self.start as usize + self.roots.len()
    }
}

/// The full fetch from index 0 — today's fetch, classified exactly as before.
async fn fetch_pool_full<C: SubtreeRootSource + ?Sized, H>(
    client: &mut C,
    pool: ShieldedProtocol,
    parse: fn(&SubtreeRoot) -> Result<CommitmentTreeRoot<H>, SubtreeRootInvalid>,
) -> Result<PoolServe<H>, RootFetchFault> {
    let mut stream = open_pool_stream(client, pool).await?;
    let (roots, hashes) = collect_roots_with_hashes(&mut stream, parse).await?;
    Ok(PoolServe {
        start: 0,
        roots,
        hashes,
    })
}

/// Say that an incremental fetch fell back, was retried or was re-classified
/// (§3.3 step 7): a `wallet.sync` warn whose `outcome` names the check. §5.4: a
/// pool name and a static code; never the start, a height or a count.
fn report_start_fallback(pool: ShieldedProtocol, outcome: &'static str) {
    tracing::warn!(
        target: "zec_wallet_core",
        pool = pool.as_str_name(),
        outcome,
        "wallet.sync"
    );
}

/// **One pool's fetch, per its plan** (S15-F1 §3.3 steps 1–3).
///
/// A full plan (or `From { start: 0 }`) is [`fetch_pool_full`], unchanged. A
/// `start > 0` plan opens the stream at `start` and:
///
/// * **step 2 — fetch and decode faults retry from 0 in the same pass**, and only
///   the `start = 0` result is classified (as today). A TIMEOUT, at the open or
///   mid-drain, fails the pass as today's timeout with no retry, and turns
///   incremental off (`roots_start_timed_out`, [`start_timed_out`]); the flood cap
///   is fatal with no retry. A server STATUS, at the open or mid-drain (an
///   InvalidArgument included — it is never read as "protocol unknown" at a
///   non-zero start), is `roots_start_refused` and turns incremental off; a
///   transport fault at either point (a reset — over Tor, possibly one) is
///   `roots_start_dropped` and does NOT, unless it is the pool's second in a row
///   this session whose retry from 0 was served (`PoolMemo::consecutive_drops`);
///   a decode fault is
///   `roots_start_failed` and turns it off.
/// * **step 3 — the first served root decides**: at `recorded[0]`'s height the
///   server ignored `start_index` — the serve is re-classified as a full fetch from
///   0 and judged exactly as one, with no second stream (`roots_start_ignored`);
///   any other height but `overlap_height` falls back from 0
///   (`roots_overlap_moved`); and after the drain, `start + served < count` (an
///   empty serve included) falls back from 0 (`roots_served_short`). All three
///   turn incremental off.
///
/// The bind's and validation's refusals are not here: they run on the returned
/// serve in the locked section and are reported exactly as today, never retried.
async fn fetch_pool<C: SubtreeRootSource + ?Sized, H>(
    client: &mut C,
    pool: ShieldedProtocol,
    slot: usize,
    planned: &PlannedPool,
    parse: fn(&SubtreeRoot) -> Result<CommitmentTreeRoot<H>, SubtreeRootInvalid>,
    signals: &mut IngestSignals,
) -> Result<PoolServe<H>, RootFetchFault> {
    let (start, overlap_height) = match planned.plan {
        FetchPlan::From {
            start,
            overlap_height,
        } if start > 0 => (start, overlap_height),
        FetchPlan::From { .. } | FetchPlan::Full(_) => {
            return fetch_pool_full(client, pool, parse).await;
        }
    };
    let fault = match client.subtree_roots(pool, start).await {
        Err(e @ GrpcError::Timeout { .. }) => {
            return Err(start_timed_out(pool, slot, signals, e));
        }
        // A TRANSPORT-class answer at the open is this crate's "could not reach /
        // contact the endpoint" (`GrpcError::Transport`) — a reset, not a server
        // refusing the start — so it is read like a mid-drain reset: retried once,
        // incremental stays on (until a second one in a row — the memo's count).
        // Only a server-composed status is a refusal.
        Err(GrpcError::Transport { .. }) => StartFault::Dropped,
        Err(GrpcError::Status { .. } | GrpcError::ShieldedProtocolUnknown { .. }) => {
            StartFault::Refused
        }
        Ok(mut stream) => {
            let judge = FirstRootJudge {
                first_recorded: planned.first_recorded,
                overlap_height,
            };
            match drain_roots(&mut stream, parse, start, Some(judge)).await {
                Ok(Drained::Served {
                    roots,
                    hashes,
                    ignored: true,
                }) => {
                    // Re-classified, not re-fetched: no second stream, so it is
                    // not counted in `fallbacks` (the orchestrator's ruling 7).
                    report_start_fallback(pool, "roots_start_ignored");
                    signals.incremental_off[slot] = true;
                    return Ok(PoolServe {
                        start: 0,
                        roots,
                        hashes,
                    });
                }
                Ok(Drained::Served {
                    roots,
                    hashes,
                    ignored: false,
                }) => {
                    if u64::from(start) + roots.len() as u64 >= planned.count {
                        return Ok(PoolServe {
                            start,
                            roots,
                            hashes,
                        });
                    }
                    StartFault::ServedShort
                }
                Ok(Drained::OverlapMoved) => StartFault::OverlapMoved,
                Err(DrainFault::Flood) => return Err(RootFetchFault::Fatal(endpoint_unusable())),
                Err(DrainFault::Grpc(e @ GrpcError::Timeout { .. })) => {
                    return Err(start_timed_out(pool, slot, signals, e));
                }
                Err(DrainFault::Grpc(GrpcError::Transport { .. })) => StartFault::Dropped,
                // A server STATUS mid-drain (the first-pull door, A9's second
                // place a refusal lands) is a refusal of the start like one at the
                // open: NOT classified here — at `start > 0` neither an
                // InvalidArgument nor any other status may make Ironwood
                // Unsupported or a required pool fatal; only the retry's `start =
                // 0` answer is classified (the orchestrator's ruling 1).
                Err(DrainFault::Grpc(
                    GrpcError::Status { .. } | GrpcError::ShieldedProtocolUnknown { .. },
                )) => StartFault::Refused,
                Err(DrainFault::Decode) => StartFault::Failed,
            }
        }
    };
    report_start_fallback(pool, fault.code());
    signals.fallbacks += 1;
    match fault {
        // A drop is counted below, once its retry is known to have been served.
        StartFault::Dropped => {}
        StartFault::Refused
        | StartFault::Failed
        | StartFault::OverlapMoved
        | StartFault::ServedShort => signals.incremental_off[slot] = true,
    }
    let retried = fetch_pool_full(client, pool, parse).await;
    // A drop counts toward `SUBTREE_ROOTS_DROPS_OFF` only when the retry from 0
    // was SERVED: that is evidence the server resets non-zero starts. A retry that
    // fails too says the path is down, which is not held against incremental.
    if fault == StartFault::Dropped && retried.is_ok() {
        signals.dropped[slot] = true;
    }
    retried
}

/// Why a `start > 0` stream is re-fetched from 0 in the same pass (§3.3 steps
/// 2–3). Every one but [`Self::Dropped`] turns the pool's incremental fetch off
/// for the session; a drop does only on the second in a row (the memo counts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartFault {
    /// A server status at the open or mid-drain.
    Refused,
    /// A transport fault at the open or mid-drain.
    Dropped,
    /// A root that would not decode.
    Failed,
    /// The first root's height is not the one recorded at `start`.
    OverlapMoved,
    /// The serve ended below the stored run.
    ServedShort,
}

impl StartFault {
    /// The `outcome` code its warn carries (§3.3 step 7).
    const fn code(self) -> &'static str {
        match self {
            Self::Refused => "roots_start_refused",
            Self::Dropped => "roots_start_dropped",
            Self::Failed => "roots_start_failed",
            Self::OverlapMoved => "roots_overlap_moved",
            Self::ServedShort => "roots_served_short",
        }
    }
}

/// A TIMEOUT on a `start > 0` stream, at the open or mid-drain (§3.3 step 2): the
/// pass fails exactly as today's timeout does — no retry, no second stream — but
/// the pool's incremental fetch is turned off for the rest of the session, so the
/// next pass is a full fetch from 0 (`roots_start_timed_out`). Without that, an
/// endpoint that answers from 0 and stalls only on a non-zero start would fail
/// every pass of the session: a wedge (the built-diff security review, LOW-2).
fn start_timed_out(
    pool: ShieldedProtocol,
    slot: usize,
    signals: &mut IngestSignals,
    e: GrpcError,
) -> RootFetchFault {
    report_start_fallback(pool, "roots_start_timed_out");
    signals.incremental_off[slot] = true;
    RootFetchFault::Fatal(transport_err(e))
}

/// Fetch + decode the subtree roots for EVERY pool in [`SUBTREE_ROOT_POOLS`]
/// (async, network). Every pool is collected BEFORE any DB write, so a fault
/// returns `Err` with NOTHING written (the money-safe property: a bad endpoint
/// never half-writes the witness tree; the next sync re-fetches what it needs).
/// Every pool from index 0 — the full fetch; [`fetch_subtree_roots_planned`] is
/// the same fetch per pool plan (S15-F1).
///
/// `tip` is the endpoint's OWN reported chain tip, read by the caller on the same
/// pass; it is the ceiling in [`validate_root_sequence`] (A11). It is a required
/// argument and not an `Option` on purpose — an optional bound is an off-switch,
/// and a fixture that supplies the value which disables a guard is how a guard ends
/// up vacuous. The caller reads the tip BEFORE the roots, so a chain that advanced
/// between the two reads could in principle put a root above it; that needs a
/// subtree (2^16 notes) to complete inside the gap between two RPCs, and the
/// failure it would produce is one refused pass that the next sync repairs.
///
/// ## A4 vs A9 — the contradiction the contract left open, and how it is resolved
///
/// **A9 wins, narrowly.** A4's "all pools collected before any DB write" is kept in
/// full for every pool the wallet cannot do without. What is carved out is exactly
/// one condition for exactly one pool: an endpoint that says it does not KNOW the
/// Ironwood protocol is survivable, and the other two pools are still written.
///
/// The alternative was letting A4 stand unqualified, and its user-visible meaning
/// is what decided it: against a lightwalletd too old to have heard of Ironwood,
/// collect-all-before-write throws away the Sapling and Orchard roots too, so the
/// whole wallet stops syncing on a server that worked yesterday — balance frozen,
/// history frozen, and (through `classify_status`'s `Unknown` arm, until this
/// change) retried forever as if the link were flaky.
///
/// **What the carve-out buys:** a wallet on an Ironwood-ignorant endpoint keeps
/// syncing, keeps its Sapling and Orchard money spendable, and records
/// [`PoolFetch::Unsupported`] for Ironwood — which is *no information about the
/// pool*, never "the pool is empty".
///
/// **What it costs, stated plainly (IT-1b):** the property lost is
/// all-or-nothing-ness. Before, a `SubtreeRoots` value that existed meant every
/// pool had been heard from; now it can mean two pools were heard from and one
/// refused, and any reader that treats a written tree as a complete tree is wrong
/// in a way the type no longer prevents. Ironwood notes on such an endpoint stay
/// unspendable — the pre-T0-1 state — and the only thing standing between that and
/// silence is [`PoolFetch`] being carried and surfaced. The carve-out is also
/// bounded by classification, not by intent: it fires on a server-origin
/// `Unknown`/`InvalidArgument` from this one RPC, and `InvalidArgument` from
/// `GetSubtreeRoots` has historically had a second cause (zcashd rejecting a
/// `max_entries` of 65536 — see [`LightwalletdClient::subtree_roots`]). So a future
/// argument fault could be recorded as "unsupported" rather than as a hard error.
///
/// **What makes the trade acceptable, and where that property lives (T0-1b).**
/// The refusal is NOT silent in a shipped build — but the `warn!` in
/// [`report_pool_outcome`] is not what makes it so. That log is disabled in every
/// release artifact (the SDK installs a tracing subscriber only under
/// `#[cfg(all(debug_assertions, target_os = "android"))]` —
/// `zec_wallet/rust/src/api/meta.rs`, "RELEASE: installs NO logger"), and the
/// post-build review measured the previous sentence here — *"it cannot be silent"*
/// — as false for exactly that reason. The surface is the status channel:
/// `SyncPass.root_outcomes` carries every pool's [`PoolFetch`] out of the pass,
/// `sync_controller::emit_synced` reads it on every clean pass and renders it
/// through ONE exhaustive, wildcard-free match into
/// [`crate::state::PoolService`], and a refused or lied-about pool publishes
/// [`crate::state::SyncStatus::UpToDateDegraded`] instead of `UpToDate` — on the
/// same `watch` channel `Wallet::watch_sync_status` streams to hosts and
/// `Wallet::snapshot` reads, with no subscriber involved. A zero-root serve is
/// CONDITIONAL: [`apply_height_bind`] turns it into [`PoolFetch::Withheld`] —
/// degraded, "switch servers" — when the signed bundle proves the pool has
/// completed subtrees at or below the endpoint's own tip, and leaves it a
/// `Served { roots: 0 }` (rendered `UpToDate`, count carried) when it has no
/// such proof: the honest pre-first-subtree state, on which raising a variant
/// would be the false alarm rather than the honesty. Any change that breaks that
/// chain reopens `docs/plan/production-readiness-phase-1.md` §4a owed row 2.
///
/// Everything else about Ironwood stays fatal: a transport fault, a timeout, a
/// malformed root, a flooding stream and a height sequence that cannot be true all
/// abort the whole fetch with nothing written, exactly as they do for Sapling.
#[cfg_attr(not(test), allow(dead_code))] // production fetches per plan
pub(crate) async fn fetch_subtree_roots<C: SubtreeRootSource + ?Sized>(
    client: &mut C,
    network: Network,
    tip: BlockHeight,
) -> Result<SubtreeRoots, WalletError> {
    const FULL: [PlannedPool; SUBTREE_ROOT_POOLS.len()] = [PlannedPool::full(); 3];
    fetch_subtree_roots_planned(client, network, tip, &FULL)
        .await
        .roots
}

/// [`fetch_subtree_roots`] per pool plan (S15-F1, ADR-0569): each pool is fetched
/// from its plan's start ([`fetch_pool`] — the retries, fallbacks and
/// re-classification of §3.3 steps 2–3), and what the fetch learned about this
/// server's incremental serving rides out in [`PlannedFetch::signals`] on the
/// `Err` path too. A served sequence is validated here only when it starts at 0;
/// one that starts above 0 is validated over the virtual sequence in the locked
/// section ([`apply_height_bind`]), where the stored prefix is read.
pub(crate) async fn fetch_subtree_roots_planned<C: SubtreeRootSource + ?Sized>(
    client: &mut C,
    network: Network,
    tip: BlockHeight,
    plans: &[PlannedPool; SUBTREE_ROOT_POOLS.len()],
) -> PlannedFetch {
    let mut signals = IngestSignals::default();
    let roots = fetch_planned_pools(client, network, tip, plans, &mut signals).await;
    PlannedFetch { roots, signals }
}

async fn fetch_planned_pools<C: SubtreeRootSource + ?Sized>(
    client: &mut C,
    network: Network,
    tip: BlockHeight,
    plans: &[PlannedPool; SUBTREE_ROOT_POOLS.len()],
    signals: &mut IngestSignals,
) -> Result<SubtreeRoots, WalletError> {
    let mut out = SubtreeRoots::empty();

    for (slot, &pool) in SUBTREE_ROOT_POOLS.iter().enumerate() {
        let planned = &plans[slot];
        // ONE exhaustive, wildcard-free match over the pool set. Each arm names its
        // own parser, its own destination, its own activation upgrade and its own
        // fault policy — and upstream adding a fourth `ShieldedProtocol` variant
        // fails THIS match to compile, which is the thing that did not happen when
        // Ironwood landed.
        let outcome = match pool {
            ShieldedProtocol::Sapling => {
                let served = fetch_pool(client, pool, slot, planned, parse_sapling_root, signals)
                    .await
                    .map_err(WalletError::from)?;
                if served.start == 0 {
                    validate_root_sequence(
                        &root_heights(&served.roots),
                        pool_activation(network, NetworkUpgrade::Sapling),
                        tip,
                    )?;
                }
                let roots = served.absolute();
                out.starts[slot] = served.start;
                out.sapling = served.roots;
                out.completing_hashes[slot] = served.hashes;
                PoolFetch::Served { roots }
            }
            ShieldedProtocol::Orchard => {
                let served = fetch_pool(client, pool, slot, planned, parse_orchard_root, signals)
                    .await
                    .map_err(WalletError::from)?;
                if served.start == 0 {
                    validate_root_sequence(
                        &root_heights(&served.roots),
                        pool_activation(network, NetworkUpgrade::Nu5),
                        tip,
                    )?;
                }
                let roots = served.absolute();
                out.starts[slot] = served.start;
                out.orchard = served.roots;
                out.completing_hashes[slot] = served.hashes;
                PoolFetch::Served { roots }
            }
            // The one OPTIONAL pool (the A4-vs-A9 ruling above): a REFUSAL is a
            // recorded degraded state, everything else is as fatal as it is for the
            // other two. An incremental start the server refuses is retried from 0
            // first (`fetch_pool`), so only the from-0 answer can read Unsupported.
            ShieldedProtocol::Ironwood => {
                match fetch_pool(client, pool, slot, planned, parse_ironwood_root, signals).await {
                    Err(RootFetchFault::ProtocolUnknown) => PoolFetch::Unsupported,
                    Err(RootFetchFault::Fatal(e)) => return Err(e),
                    Ok(served) => {
                        if served.start == 0 {
                            validate_root_sequence(
                                &root_heights(&served.roots),
                                pool_activation(network, NetworkUpgrade::Nu6_3),
                                tip,
                            )?;
                        }
                        let roots = served.absolute();
                        out.starts[slot] = served.start;
                        out.ironwood = served.roots;
                        out.completing_hashes[slot] = served.hashes;
                        PoolFetch::Served { roots }
                    }
                }
            }
        };
        out.outcomes[slot] = outcome;
        report_pool_outcome(pool, outcome);
    }

    Ok(out)
}

/// The completion heights of a served root sequence, in order.
fn root_heights<H>(roots: &[CommitmentTreeRoot<H>]) -> Vec<u32> {
    roots
        .iter()
        .map(|r| u32::from(r.subtree_end_height()))
        .collect()
}

/// **T0-1a — hold every pool's served heights to what the wallet knows, and decide
/// what a violation costs.** Runs SYNCHRONOUSLY under the db lock, on the aux
/// connection, in the same critical section as [`put_subtree_roots`] — see
/// `Wallet::update_subtree_roots`. Returns the outcomes to record, having zeroed
/// any pool it refused so the write that follows cannot carry it.
///
/// ## The disposition of a height violation (T0-1a question 5)
///
/// **It was never settled and this is the decision: Sapling and Orchard are FATAL
/// to the pass; Ironwood degrades to a recorded, refused pool.** Today
/// `validate_root_sequence(..)?` makes all three fatal, so this changes exactly one
/// pool's answer, and the argument is the one the A4-vs-A9 ruling already made for
/// `ProtocolUnknown`, applied to a second condition.
///
/// **Why Ironwood degrades.** The decisive question is what the choice buys against
/// a HOSTILE endpoint and what it costs against an HONEST-BUT-WRONG one. Against a
/// hostile endpoint it buys nothing: that endpoint can already keep Ironwood
/// unspendable forever by refusing the protocol ([`PoolFetch::Unsupported`]) or by
/// serving zero roots (`Served { roots: 0 }`), both of which this wallet survives
/// by design. Making a height violation fatal adds no new defence against it —
/// it only hands it a way to stop the whole wallet syncing. Against an
/// honest-but-buggy server, or against a bind of ours that turns out to be
/// mis-specified on some corner of the fleet, fatal means Sapling and Orchard money
/// stops moving too, on a server that worked yesterday. That is the "remedy worse
/// than the bug" shape, and the blast radius of a brand-new money-path guard
/// meeting a fleet it has never run against belongs bounded to the pool it guards.
///
/// **What degrading COSTS, stated plainly (IT-1b).** Ironwood money stays
/// unspendable while the wallet keeps reporting progress — which is INC-020's own
/// user-visible failure mode. That cost is only acceptable because the outcome is
/// CARRIED ([`PoolFetch::HeightViolation`]) and because rendering it is contracted
/// as T0-1b. If T0-1b does not land, this trade is being made on a property the
/// build does not have — the same sentence the review had to correct about the
/// A9 carve-out, written here before it can be false rather than after.
///
/// **Why the other two stay fatal.** Their roots are REQUIRED: without an honest
/// Sapling or Orchard root sequence the wallet cannot build a witness for money it
/// already holds, and the only alternative to stopping is writing endpoint-chosen
/// heights into the tree that decides spendability. The cost is real and is the one
/// the contract names — one lying endpoint stops the wallet syncing until the user
/// switches servers, and this SDK does not rotate endpoints by itself.
///
/// **How each disposition reaches the surface (T0-1b).** The Ironwood violation
/// reaches a caller as a `PoolFetch` and renders as
/// `SyncStatus::UpToDateDegraded` with `PoolService::HeightViolation` for that
/// pool. The Sapling/Orchard violation is fatal and returns
/// [`endpoint_unusable`] — `Sync { StallReason::EndpointMisbehaving }`, the
/// content-tier class every clean-but-wrong answer shares since the adjudication
/// re-aimed the helper itself — so it renders as a stall a dead link cannot
/// produce (D9) and a local fault cannot produce (D10): "this server's answers
/// are wrong; switch servers". Before T0-1b that helper said `EndpointUnreachable`,
/// the masquerade `StallReason::Internal` was minted to end, one door over.
///
/// **The withheld read (adjudication (b); short serves since T0-1d).** For EVERY
/// pool, a serve of FEWER roots than the bundled treestates at or below the
/// endpoint's tip prove complete — zero, or a strict prefix — is recorded as
/// [`PoolFetch::Withheld`] ([`withheld`] carries the argument and the trade).
/// This is a REPORT, not a refusal: a required pool served zero is accepted
/// today (the rewind ledger's own proofs rely on it) and stays accepted, and a
/// short prefix is written exactly as before, so nothing changes in what is
/// written — only in what the surface says. Making it fatal for a required pool
/// is a separate decision with the "remedy worse than the bug" shape.
///
/// ## Ordering, and the window it defines (C16)
///
/// Every pool is bound BEFORE any pool is written, so a refusal on a later pool
/// leaves the wallet DB exactly as it was — the same all-or-nothing window
/// [`fetch_subtree_roots`] already establishes for the network half, extended over
/// the bind. Within that window the Ironwood degrade is a local edit to the value
/// about to be written, never a partial write.
///
/// ## The rewind ledger's other half (T0-1a-R; consumed per pool since T0-1a-R2)
///
/// [`HeightBind::consume_rewind`] carries out, per pool, the rewind count this bind
/// READ — and only for a pool whose roots this pass is actually about to write. The
/// caller records it through [`put_subtree_roots`]'s per-pool hook, immediately after
/// THAT pool's put lands and before the next pool's begins (§4g 0b), which is what
/// makes the post-rewind relaxation last exactly one accepted write instead of forever
/// (R4) — and what stops a later pool's failed put from holding an earlier,
/// already-written pool's relaxation open. A pool that served nothing, or whose roots
/// were refused, is `None`: nothing was corrected, so nothing is consumed. That
/// matters — an endpoint that answers `Unsupported` for Ironwood on the pass after a
/// reorg must not thereby spend the relaxation the honest re-serve needs.
pub(crate) fn apply_height_bind(
    aux: &rusqlite::Connection,
    network: Network,
    roots: &mut SubtreeRoots,
    tip: BlockHeight,
) -> Result<HeightBind, WalletError> {
    // The three slots, named against `SUBTREE_ROOT_POOLS` rather than assumed. The
    // array's order is already compile-time asserted at the top of this module; a
    // reorder there would trip these instead of silently binding a pool's roots
    // against another pool's evidence.
    const SAPLING_SLOT: usize = 0;
    const ORCHARD_SLOT: usize = 1;
    const IRONWOOD_SLOT: usize = 2;
    const _: () = assert!(
        matches!(SUBTREE_ROOT_POOLS[SAPLING_SLOT], ShieldedProtocol::Sapling)
            && matches!(SUBTREE_ROOT_POOLS[ORCHARD_SLOT], ShieldedProtocol::Orchard)
            && matches!(
                SUBTREE_ROOT_POOLS[IRONWOOD_SLOT],
                ShieldedProtocol::Ironwood
            ),
        "apply_height_bind's slots must match SUBTREE_ROOT_POOLS"
    );

    let mut outcomes = roots.outcomes;
    let mut consume_rewind = [None; SUBTREE_ROOT_POOLS.len()];
    // Defaults to the GUARDING reading: a pool this function returns before binding
    // (the required-pool early return) is reported as rewound, so nothing forgets a
    // bracket on its behalf.
    let mut rewound = [true; SUBTREE_ROOT_POOLS.len()];
    let mut observed = [0i64; SUBTREE_ROOT_POOLS.len()];
    let mut c6: [root_bind::C6Seen; SUBTREE_ROOT_POOLS.len()] = Default::default();
    let starts = roots.starts;

    // The two REQUIRED pools. Both are fatal, so a violation on either returns
    // before the Ironwood arm can edit anything and before anything is written.
    // Written out per pool rather than looped because each names its own root
    // vector, exactly as the fetch loop's arms do — the two Orchard-shaped pools
    // have the IDENTICAL Rust type and a loop over `(slot, pool)` would let a
    // cross-wire between them compile.
    let required = [
        (
            SAPLING_SLOT,
            ShieldedProtocol::Sapling,
            roots.sapling.len(),
            bind_pool(
                aux,
                network,
                ShieldedProtocol::Sapling,
                PoolServed {
                    start: starts[SAPLING_SLOT],
                    roots: &roots.sapling,
                    hashes: &roots.completing_hashes[SAPLING_SLOT],
                },
                pool_activation(network, NetworkUpgrade::Sapling),
                tip,
            )?,
        ),
        (
            ORCHARD_SLOT,
            ShieldedProtocol::Orchard,
            roots.orchard.len(),
            bind_pool(
                aux,
                network,
                ShieldedProtocol::Orchard,
                PoolServed {
                    start: starts[ORCHARD_SLOT],
                    roots: &roots.orchard,
                    hashes: &roots.completing_hashes[ORCHARD_SLOT],
                },
                pool_activation(network, NetworkUpgrade::Nu5),
                tip,
            )?,
        ),
    ];
    for (slot, pool, served, bound) in required {
        rewound[slot] = bound.rewound;
        observed[slot] = bound.observed;
        let seen = match bound.verdict {
            Ok(seen) => seen,
            Err(refusal) => {
                report_pool_outcome(
                    pool,
                    PoolFetch::HeightViolation {
                        code: refusal.code(),
                    },
                );
                // T0-1b D9: the server answered and the answer cannot be true —
                // `endpoint_unusable()` is the content-tier class ("switch servers"),
                // no longer the dead-link masquerade it used to render as.
                return Err(endpoint_unusable());
            }
        };
        c6[slot] = seen;
        if let Some(outcome) = withheld(outcomes[slot], bound.proven) {
            outcomes[slot] = outcome;
            roots.outcomes[slot] = outcome;
            // The ABSOLUTE count (S15-F1): what the tree holds from index 0.
            report_under_served(pool, starts[slot] as usize + served, bound.proven);
        }
        consume_rewind[slot] = (served > 0).then_some(bound.observed);
    }

    // Ironwood: refused for this pool only. The roots are DROPPED rather than
    // truncated — `put_shard_roots` writes positionally from the pool's start, so a
    // partial prefix would be silently wrong rather than merely short (C4).
    let ironwood_served = roots.ironwood.len();
    let bound = bind_pool(
        aux,
        network,
        ShieldedProtocol::Ironwood,
        PoolServed {
            start: starts[IRONWOOD_SLOT],
            roots: &roots.ironwood,
            hashes: &roots.completing_hashes[IRONWOOD_SLOT],
        },
        pool_activation(network, NetworkUpgrade::Nu6_3),
        tip,
    )?;
    rewound[IRONWOOD_SLOT] = bound.rewound;
    observed[IRONWOOD_SLOT] = bound.observed;
    match bound.verdict {
        Err(refusal) => {
            roots.ironwood.clear();
            roots.completing_hashes[IRONWOOD_SLOT].clear();
            let outcome = PoolFetch::HeightViolation {
                code: refusal.code(),
            };
            outcomes[IRONWOOD_SLOT] = outcome;
            roots.outcomes[IRONWOOD_SLOT] = outcome;
            report_pool_outcome(ShieldedProtocol::Ironwood, outcome);
        }
        Ok(seen) => {
            c6[IRONWOOD_SLOT] = seen;
            if let Some(outcome) = withheld(outcomes[IRONWOOD_SLOT], bound.proven) {
                outcomes[IRONWOOD_SLOT] = outcome;
                roots.outcomes[IRONWOOD_SLOT] = outcome;
                report_under_served(
                    ShieldedProtocol::Ironwood,
                    starts[IRONWOOD_SLOT] as usize + ironwood_served,
                    bound.proven,
                );
            }
            consume_rewind[IRONWOOD_SLOT] = (ironwood_served > 0).then_some(bound.observed);
        }
    }

    Ok(HeightBind {
        outcomes,
        consume_rewind,
        rewound,
        observed,
        c6,
    })
}

/// The withheld read (T0-1b adjudication (b); widened to SHORT serves by T0-1d):
/// a pool the fetch recorded as `Served { roots }` — the endpoint OPENED the
/// stream and closed it cleanly — with FEWER roots than the bundle proves
/// complete at or below the endpoint's tip is [`PoolFetch::Withheld`]; every
/// other outcome is left as the fetch recorded it. Keyed on the OUTCOME, not on
/// the count alone: a refused pool (`Unsupported`) also has nothing served, and
/// it must stay "the server does not know this pool" — collapsing it into
/// "withheld" is the A10 conflation with a new name (the proof's rows 2/3 caught
/// exactly that in the first cut).
///
/// **One comparison, `served < proven`, and it covers zero and short alike.**
/// Until T0-1d this read fired on `roots == 0` only, and its doc said "not
/// short" because *every reproducible mainnet fixture in this repository is
/// short* — the honest control served ONE Sapling root where the bundle proves
/// 1,128 complete. The T0-1b adjudicator ruled (§4i (i)) that the fixtures were
/// the floor, not the rule: a strict prefix above a proven boundary read
/// `UpToDate`, and the roots it leaves out are the OMMERS every later note's
/// witness needs — a wallet whose notes sit above `n` is in INC-020's stuck
/// state with a healthy badge. The fixtures were regenerated instead
/// (`testing::bind_consistent_roots`, derived from `bundled_counts` alone), so
/// the rule can say what the chain says. What the widening buys: a server that
/// serves SOME roots and withholds the rest is graded like one that serves none,
/// because to the notes above `n` they are the same server. What it costs, and
/// where it is charged: the surface carries `proven` and not the gap
/// ([`crate::state::PoolService::Withheld`] — the count carried is the proof;
/// `served` is on the log line, `report_under_served`), and the name "withheld"
/// now reads figuratively for a short serve — the server withheld `proven − n`
/// roots, not all of them. A sibling variant would have said "short" on the
/// surface at E12's price (a bridge variant, five UI sites, sixteen locales) for
/// a fact whose next step is identical: switch servers.
///
/// The tip reading is [`proven_complete_at_or_below`]'s, unchanged: a short
/// serve is judged against the bundled rows the endpoint's OWN tip covers, so an
/// honest server behind the bundle is never graded short for rows its chain has
/// not reached (P9 — no new tip-dependence; a tip below the newest row is
/// already `SyncStatus::EndpointBehind`). A serve of `proven` or more roots is
/// healthy at ANY count: the bundle can prove no more than it carries, and above
/// the newest row a server is trusted for what the chain has since done — the
/// residual [`tip_standing`] states.
///
/// Reported, not refused — for both cases. The short prefix IS written (a prefix
/// is what the count bind accepts and what `put_shard_roots` writes from index
/// 0), and its rewind relaxation IS consumed (`consume_rewind` keys on
/// `served > 0`, F3/F4's own control), so a short serve moves the record while
/// a zero serve leaves it. Only the surface differs from before.
fn withheld(fetched: PoolFetch, proven: u64) -> Option<PoolFetch> {
    matches!(fetched, PoolFetch::Served { roots } if (roots as u64) < proven)
        .then_some(PoolFetch::Withheld { proven })
}

/// Say out loud that a pool was under-served (principle 10): the served count
/// AND the bundle's proof on one line, because the surface value carries only
/// the proof ([`withheld`]'s stated cost) and this line is where `served` lives.
/// `warn!` for the reason the refused and lied-about arms are: this pool's money
/// above `served` is not spendable from this server and the wallet is otherwise
/// about to look synced. §5.4: `count` (the served root count — the healthy
/// arm's own field) and `limit` (the bundle's proof, a public constant of the
/// signed binary — the name [`tip_standing`] uses for the same kind of number);
/// no height, no note, no endpoint identity.
fn report_under_served(pool: ShieldedProtocol, served: usize, proven: u64) {
    tracing::warn!(
        target: "zec_wallet_core",
        pool = pool.as_str_name(),
        count = served,
        limit = proven,
        "endpoint served fewer subtree roots for this pool than the bundled treestates \
         prove complete at or below its own tip — the missing roots are the ommers every \
         later note's witness needs, so notes in them cannot be witnessed from this server. \
         NOT the same as the pool being empty."
    );
}

/// How many subtrees the SIGNED BUNDLE proves complete for a pool at or below
/// `tip` — the largest `complete` over the bundled rows the endpoint's own tip
/// covers, `0` when it covers none.
///
/// **Tip-bounded on purpose, and this is what the bound buys and costs now that
/// the tip is graded (T0-1c, §4k-R decision 4 — Q-E2 answered for real).** A
/// REPORT may lean on the tip; the bind may not (`root_bind::gather` filters
/// nothing, and `root_bind::check_pool` never reads this number). What the bound
/// buys: a server that is BEHIND the bundle — an honest validator mid-sync (P4),
/// or a testnet whose bundled rows sit above its tip — serves zero roots for a
/// pool whose first bundled proof lies above its tip, and that zero is "no proof
/// yet AT THAT TIP": `Served { roots: 0 }`, never `Withheld`, because a server is
/// not withholding what its chain has not reached. What the bound costs, and
/// where it is charged: rows above the tip are switched off, so an endpoint that
/// UNDER-reports its tip below a row can switch that row off — and since T0-1c
/// every tip below the bundle's newest row is graded in [`fetch_tip`] and the
/// pass publishes `SyncStatus::EndpointBehind` for it, so the badge the
/// under-reporter hides is replaced by the sentence that says why the pool
/// cannot be graded from that tip (INC-023's own geometry, E2). At or above the
/// newest row every bundled row is admitted
/// (`tests::at_or_above_the_newest_row_the_filter_admits_every_bundled_row_and_one_below_excludes_it`),
/// so the residual under-reporter — to anywhere at or above the row — cannot
/// switch a row off; it can only hold the wallet at a stale, self-consistent
/// height, which [`tip_standing`] states. An earlier version of this paragraph
/// called the bound a "belt" a refusing floor made total; under REPORT AND
/// CONTINUE the bound is load-bearing on every behind pass, and this is its
/// statement.
fn proven_complete_at_or_below(bundled: &[(u32, u64)], tip: u32) -> u64 {
    bundled
        .iter()
        .filter(|(height, _)| *height <= tip)
        .map(|(_, complete)| *complete)
        .max()
        .unwrap_or(0)
}

/// What [`apply_height_bind`] hands back: the outcomes to record, and the rewind
/// ledger's per-pool consumption for the write that is about to happen.
pub(crate) struct HeightBind {
    pub(crate) outcomes: [PoolFetch; SUBTREE_ROOT_POOLS.len()],
    /// `Some(rewinds observed while binding)` for each pool whose roots this pass
    /// will WRITE — the value `root_bind::note_roots_recorded` stores once the write
    /// lands. `None` ⇒ nothing was written for that pool (an empty serve, or a
    /// refusal), so its post-rewind relaxation stays armed for the pass that does
    /// correct it.
    pub(crate) consume_rewind: [Option<i64>; SUBTREE_ROOT_POOLS.len()],
    /// **BIND-1-R: was this pool's bind RELAXED by an unconsumed rewind?** Carried
    /// out because a write that landed inside that window may not withdraw a
    /// boundary bracket — see `Wallet::update_subtree_roots`' hook. Read from the
    /// same `RewindWatch` the bind itself read, so the two cannot disagree about
    /// whether a rewind was outstanding.
    pub(crate) rewound: [bool; SUBTREE_ROOT_POOLS.len()],
    /// The chain rewind count each pool's bind read (`RewindWatch::observed`) —
    /// what the session memo records as `chain_rewinds_seen` after a full fetch
    /// (S15-F1). `0` for a pool the bind never reached.
    pub(crate) observed: [i64; SUBTREE_ROOT_POOLS.len()],
    /// Each accepted pool's C6 set ([`root_bind::check_pool_from`]); empty for a
    /// refused pool, one the bind never reached, and a relaxed bind.
    pub(crate) c6: [root_bind::C6Seen; SUBTREE_ROOT_POOLS.len()],
}

impl HeightBind {
    /// The ledger value to record for `pool` once ITS roots have landed — `None` when
    /// nothing is to be written for it. Looked up against [`SUBTREE_ROOT_POOLS`] so
    /// the slot a pool's consumption lives in is the slot its outcome and its bind
    /// used; a pool that array did not name would consume nothing, which is the
    /// guarding default.
    pub(crate) fn consume_for(&self, pool: ShieldedProtocol) -> Option<i64> {
        SUBTREE_ROOT_POOLS
            .iter()
            .position(|p| *p == pool)
            .and_then(|slot| self.consume_rewind[slot])
    }

    /// Whether `pool`'s bind ran with an unconsumed rewind outstanding. **Defaults
    /// to `true` for a pool [`SUBTREE_ROOT_POOLS`] does not name** — the guarding
    /// reading, because the only thing this gates is the FORGETTING of a durable
    /// refusal, and forgetting is the irreversible direction.
    pub(crate) fn rewound_for(&self, pool: ShieldedProtocol) -> bool {
        SUBTREE_ROOT_POOLS
            .iter()
            .position(|p| *p == pool)
            .is_none_or(|slot| self.rewound[slot])
    }
}

/// One pool's bind, plus the two numbers the caller needs from the evidence it
/// gathered: the rewind count the bind READ (`observed` — carried out so the write
/// that follows records the value the bind actually saw, not a fresh one) and the
/// bundle's proof of completed subtrees at or below the tip (`proven` — the
/// withheld read's discriminator, taken from the SAME `PoolEvidence` so the bind
/// and the report cannot disagree about what the bundle says).
struct PoolBound {
    observed: i64,
    /// Whether a rewind was outstanding for this pool when the bind read the ledger
    /// (BIND-1-R — the bracket's forgetting rule needs it; see [`HeightBind`]).
    rewound: bool,
    proven: u64,
    verdict: Result<root_bind::C6Seen, root_bind::HeightBindRefusal>,
}

/// One pool's serve as the bind sees it: where it starts, the roots and their
/// positionally aligned completing hashes.
struct PoolServed<'a, H> {
    start: u32,
    roots: &'a [CommitmentTreeRoot<H>],
    hashes: &'a [Vec<u8>],
}

/// Gather one pool's evidence, validate the virtual sequence and run the bind
/// (S15-F1 §3.3 step 4.2–4.3). The outer `Result` is an OUR-SIDE fault (a DB read
/// that failed — including the fail-closed missing-column case — or a stored
/// prefix the plan promised and the table no longer holds), or the A11 sequence
/// refusal, which is fatal for every pool exactly as it was at the fetch; the inner
/// one — [`PoolBound::verdict`] — is the endpoint's.
///
/// The sequence check runs over `recorded[..start] ++ served`
/// ([`root_bind::virtual_heights`]): for a full fetch that is the served sequence
/// the fetch already validated, and for an incremental one it is the only place
/// the stored prefix is in hand.
fn bind_pool<H>(
    aux: &rusqlite::Connection,
    network: Network,
    pool: ShieldedProtocol,
    served: PoolServed<'_, H>,
    floor: Option<BlockHeight>,
    tip: BlockHeight,
) -> Result<PoolBound, WalletError> {
    let evidence = root_bind::gather(aux, network, pool, served.roots)?;
    let start = u64::from(served.start);
    validate_root_sequence(
        &root_bind::virtual_heights(start, served.roots, &evidence.recorded)?,
        floor,
        tip,
    )?;
    let verdict = root_bind::check_pool_from(start, served.roots, served.hashes, &evidence)?;
    Ok(PoolBound {
        observed: evidence.rewind.observed,
        rewound: evidence.rewind.rewound_since_record(),
        proven: proven_complete_at_or_below(&evidence.bundled, u32::from(tip)),
        verdict,
    })
}

/// Say out loud what the endpoint did for this pool (principle 10 — no silent
/// failures). The two conditions this item exists to stop being invisible are
/// **zero roots above the activation** (A10) and **a refused protocol** (A9); both
/// are `warn!`, the healthy case is `debug!`. §5.4-safe: a pool name and a count
/// carry no wallet data.
fn report_pool_outcome(pool: ShieldedProtocol, outcome: PoolFetch) {
    match outcome {
        PoolFetch::Unsupported => tracing::warn!(
            target: "zec_wallet_core",
            pool = pool.as_str_name(),
            "endpoint does not serve subtree roots for this pool — its notes cannot be \
             witnessed from this server. NOT the same as the pool being empty."
        ),
        PoolFetch::Served { roots: 0 } => tracing::warn!(
            target: "zec_wallet_core",
            pool = pool.as_str_name(),
            "endpoint served ZERO subtree roots for this pool — correct before the pool's \
             first completed subtree, and indistinguishable on the wire from an endpoint \
             that simply serves none"
        ),
        // T0-1a's fourth outcome. `warn!` for the same reason the other two are:
        // this pool's money is not spendable from this server and the wallet is
        // otherwise about to look healthy.
        //
        // §5.4: the value is a stable, payload-free code
        // ([`root_bind::HeightBindRefusal::code`]) and it rides the ALREADY
        // ALLOWLISTED `outcome` name — the crate's established field for a coarse
        // outcome code — rather than minting a new one. The numbers the refusal
        // carries (which index, which height, which oracle row) stay OUT of the
        // log: a completion height narrows a wallet's scan range.
        // The sentence used to end "nothing was written for this pool", which was
        // true while the bind ran only at INGEST. Since BIND-1 (§4x) this outcome
        // has a second producer — `reconcile_scan_boundaries`, which fires on a
        // height already in the table and WITHDRAWS it — so the claim is stated as
        // the one thing true of both: no height the endpoint chose survives.
        PoolFetch::HeightViolation { code } => tracing::warn!(
            target: "zec_wallet_core",
            pool = pool.as_str_name(),
            outcome = code,
            "endpoint gave subtree completion heights for this pool that cannot be true — \
             none of them stands. NOT the same as the pool being empty, and NOT the same \
             as the server not knowing this pool."
        ),
        // T0-1b's fifth outcome. The bind reports it through `report_under_served`,
        // which holds the served count beside the proof; this arm exists so every
        // `PoolFetch` has a sentence here and reaches no production caller today.
        // `limit` = the bundle's proof, a public constant from the signed binary
        // (the name `tip_standing` uses for the same kind of number); no height,
        // no note, no endpoint identity.
        PoolFetch::Withheld { proven } => tracing::warn!(
            target: "zec_wallet_core",
            pool = pool.as_str_name(),
            limit = proven,
            "endpoint served fewer subtree roots for this pool than the bundled treestates \
             prove complete at or below its own tip — notes in the missing subtrees cannot \
             be witnessed from this server. NOT the same as the pool being empty."
        ),
        // `count`, not `roots`: `count` is already on the §5.4 field allowlist for
        // exactly this shape (a bounded integer that names no note and no account).
        PoolFetch::Served { roots } => tracing::debug!(
            target: "zec_wallet_core",
            pool = pool.as_str_name(),
            count = roots,
            "subtree roots collected"
        ),
    }
}

/// Map a shard-tree failure to a typed [`WalletError`], attributing the LAYER by
/// the evidence each arm has (no silent failure, honest degradation). Concrete over
/// the shard store's own error type since T0-1d, because one arm reads the SQL
/// fault inside it.
///
/// **TWO callers since B1-5 (§4x), and that is the point of the row.** The
/// served-root write ([`put_subtree_roots`], the ingest door) and the SCAN door —
/// `ClassifyStoreFault for SqliteClientError`'s three commitment-tree variants,
/// which is where `scan_cached_blocks`' own shard write lands. They used to run two
/// bodies that agreed on everything except `Insert`, where the scan door said
/// `StoreCorrupt` and this one says the endpoint's: ONE class of fault — a served
/// root the tree refuses — with two contradictory remedies chosen by which door
/// happened to catch it, and the destructive remedy on the door an endpoint can
/// reach at will (§4w (b)). One body, one grep, one remedy.
///
/// **`Insert` — the tree refused a served root: `Sync { EndpointMisbehaving }`, and
/// it is our RECORD, from a server. NARROWED by BIND-1-R** from all seven
/// `InsertionError` variants to the two `merge_checked` raises, with the arm itself
/// carrying the variant-by-variant reason for the rest. The variant a serve reaches is
/// `InsertionError::Conflict(addr)`: the cap already holds a root at that subtree
/// address (every root ever put lands there with `Retention::Reference`, which is
/// non-prunable) and the served one differs. **At the SCAN door the same variant
/// arrives with the disagreement measured against LEAVES rather than against an
/// earlier serve** — `merge_checked` refuses to reconcile the root the endpoint
/// served for a shard with the commitments this wallet just counted into it
/// (`shardtree-0.7.1/src/prunable.rs:946`) — which is if anything a stronger
/// statement about the endpoint, never a weaker one about our store. The
/// conflict has TWO candidate liars
/// — the server whose root is recorded and the server serving now — and the
/// conflict alone cannot tell them apart (§4l P6): the recorded root is not this
/// wallet's knowledge, it is what an EARLIER server said, kept as a record. So
/// this arm never says `StoreCorrupt` (the DB is intact; "restore from your
/// recovery phrase" would be a destructive remedy for a fault that is not on the
/// device — the seed is not involved) and never `Internal`. What it says is the
/// content-tier class, whose remedy reads in two steps: try another server; and
/// if EVERY server is refused at the same address, the record is the poison — a
/// first-contact server wrote a root this wallet now holds every honest server
/// to — and the exit is the rescan rebuild: `Wallet::rescan_from` →
/// `store::reset_data_db_keep_seed`, whose fresh data DB has no shard rows and
/// no cap, so the next honest serve is accepted at index 0
/// (`wallet::tests::a_first_contact_root_conflict_is_cleared_by_the_rescan_rebuild`;
/// the exit is written into [`crate::state::StallReason::EndpointMisbehaving`]
/// and the reference UI's copy). The same two-server shape exists one dimension
/// over — a recorded completion HEIGHT an honest server disagrees with
/// (`root_bind::check_recorded_heights`, whose exit is the same rescan) — which
/// is why the exit rides the class's copy rather than a variant of its own
/// (§4l decision 1: property bought, one class with one grep and one remedy
/// that is honest for both record-conflict dimensions; property lost, the
/// surface cannot say "this server is certainly wrong" for the members where
/// the wallet does know — a bundled-frontier violation, a flooding stream —
/// so the copy hedges for all of them).
///
/// **`Storage` carrying a UNIQUE-constraint failure — the endpoint's, by this
/// evidence (INC-021).** The shard write is `INSERT … ON CONFLICT (shard_index)
/// DO UPDATE` into `{prefix}_tree_shards`, whose only UNIQUE constraint besides
/// the index is `root_unique (root_hash)`
/// (`zcash_client_sqlite-0.22.0/src/wallet/db.rs:1033`, `:1100`, `:1167`); the
/// cap write is `ON CONFLICT (cap_id) DO UPDATE`; the discontinuity check only
/// reads. So a `SQLITE_CONSTRAINT_UNIQUE` (extended code 2067) inside this
/// transaction can come from exactly one place, and the value it refused is the
/// served `root_hash`: the serve named ONE root at TWO subtree indices, which no
/// chain produces (two distinct 2^16-leaf subtrees with one root is a collision
/// of the tree hash). Could the second copy be OURS — a recorded row the serve
/// did not carry? No: the cap's `batch_insert` runs first and refuses a served
/// root that differs from the recorded one at any covered index (`Insert`,
/// above), so by the time the shard rows are written every recorded index the
/// serve covers already agrees with it — the repeated hash is in the serve.
/// Measured, not argued: two same-hash Sapling roots through
/// `put_sapling_subtree_roots` come back as `Storage(Query(SqliteFailure {
/// ConstraintViolation, 2067 }, "UNIQUE constraint failed:
/// sapling_tree_shards.root_hash"))` — NOT as `ShardTreeError::Query`, which the
/// incident row first named — and `into_store_fault` folded that to
/// `StoreCorrupt`, the seed-restore scare on an intact wallet. Every other
/// constraint code stays local: `PRIMARYKEY`/`NOTNULL` cannot be reached by a
/// served value here, so if one ever is, the schema moved under the pin and
/// that is ours to look at.
///
/// **`Storage`, otherwise — [`ClassifyStoreFault`] (#371):** out-of-disk / busy /
/// IO on this write is `DiskFull`/`StoreBusy`/`Io`, not the seed-restore scare —
/// the same honesty as the scan door, since `put_subtree_roots` is itself a
/// scan-path write; the tree-logic variants (`CheckpointConflict`,
/// `SubtreeDiscontinuity`) and everything else fold to `StoreCorrupt`.
///
/// **`Query` — local, and unreachable from this write at the pin.** The pinned
/// `put_shard_roots` (`commitment_tree.rs:1210`) issues no tree query: `get_cap`,
/// `batch_insert`, `put_cap`, a discontinuity SELECT and the shard INSERTs, each
/// mapped to `Storage` or `Insert`. Every `QueryError` variant describes the
/// wallet's OWN tree (an address it does not contain, a pruned checkpoint, `Nil`
/// nodes), so the reading stays `StoreCorrupt`; a future upstream that queries
/// here is a reviewed event under the `=0.22.0` pin.
fn map_shardtree_err(e: ShardTreeError<commitment_tree::Error>) -> WalletError {
    match e {
        // A served root the tree rejects: the endpoint answered wrongly — the
        // content-tier class (T0-1b), never a dead link and never our corruption.
        // It is our record, from a server; the exit is in the doc above.
        //
        // NARROWED BY BIND-1-R (R9, §4x-run owed row 4). `Insert(_)` was all seven
        // `InsertionError` variants, and the doc above already said which one a serve
        // reaches. The line is `merge_checked`: these two are the only members raised
        // by the walk that compares the node WE hold against the node the serve — or
        // the served frontier — implies, so they are statements about the other side.
        ShardTreeError::Insert(
            shardtree::error::InsertionError::Conflict(_)
            | shardtree::error::InsertionError::InputMalformed(_),
        ) => endpoint_unusable(),
        // OURS, and the remedy is the local one. Variant by variant, on both doors
        // (read at `shardtree-0.7.1`, which `sdk/Cargo.toml` pins `=0.7.1`):
        //
        // * `OutOfRange(position, range)` — `LocatedTree::insert_frontier_nodes`
        //   (`prunable.rs:1177-1183`) when the frontier's leaf position is outside the
        //   shard's range, and `batch_insert` (`:1180`) when the start position is
        //   outside the cap's. `ShardTree::insert_frontier_nodes` derives the shard
        //   address FROM that same position (`lib.rs:438-439`) and `put_shard_roots`
        //   is passed a `start_index` this wallet chose — 0, or since S15-F1 the
        //   plan's start, which is below the stored run's length and so inside the
        //   cap (`commitment_tree.rs:1210`; the truncate door passes 0) — so neither
        //   number is one an endpoint chooses.
        // * `NotContained(addr)` — `insert_subtree` (`prunable.rs:1038`) when the
        //   subtree's root address is not inside the shard's. The subtree is built by
        //   `from_frontier` AT the shard's own level, from the same position, so it is
        //   contained by construction.
        // * `TreeFull` — raised only by `append` (`prunable.rs:1066`) and
        //   `ShardTree::append` (`lib.rs:367`). Neither door calls either: both enter
        //   through `insert_frontier` (§4x-run's settled reading, points 1-3).
        // * `CheckpointOutOfOrder` and `MarkedRetentionInvalid` — unreachable on both
        //   doors by that same reading (`ShardTree::append` only, and
        //   `Marking::None` at every truncate-door call site).
        //
        // So every member of this arm describes a tree or an address that only this
        // SDK and upstream compute. If one ever arrives it is a fault in what we hold,
        // and the honest answer is the local one — which is also the FAIL-CLOSED
        // direction: a local fault read as the endpoint's would tell a user to switch
        // servers forever while the thing that is wrong travels with them.
        ShardTreeError::Insert(_) => WalletError::StoreCorrupt,
        ShardTreeError::Query(_) => WalletError::StoreCorrupt,
        // INC-021: one served root hash at two indices, refused by `root_unique`.
        ShardTreeError::Storage(commitment_tree::Error::Query(ref sql))
            if is_unique_constraint_violation(sql) =>
        {
            endpoint_unusable()
        }
        ShardTreeError::Storage(s) => s.into_store_fault(),
    }
}

/// `SQLITE_CONSTRAINT_UNIQUE` (extended code 2067), and nothing else: not the
/// primary-key or not-null constraint codes, not the bare `SQLITE_CONSTRAINT`
/// (19). [`map_shardtree_err`] says why that one code is the endpoint's on the
/// served-root write and the others are not.
fn is_unique_constraint_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(f, _)
            if f.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    )
}

/// Write EVERY collected pool's subtree roots to the wallet shardtrees (BLOCKING
/// SQLCipher work — the caller runs this inside `spawn_blocking` under a SHORT
/// state-lock critical section, S20). Each pool's roots are written at the shard
/// index they were fetched from (the upstream `put_*_subtree_roots(start_index, ..)`
/// contract): 0 for a full fetch, the plan's start for an incremental one (S15-F1,
/// ADR-0569), whose stored prefix below the start is left as it is. A root
/// re-written at an index the cap already holds must equal the recorded one or the
/// put is a `Conflict` (REFERENCE leaves are never pruned), which is the overlap's
/// root guard.
///
/// # Precondition — this function validates NOTHING about the heights it writes
///
/// It is the UNBOUND door. Every check on endpoint-served completion heights — the
/// counting bind, the bundled floor, the completing-hash cross-check, the refuted-
/// height brackets (`root_bind`) — runs one layer up, in [`apply_height_bind`],
/// which `Wallet::update_subtree_roots` calls BEFORE its one production call here.
/// `put_shard_roots`' `ON CONFLICT DO UPDATE` overwrites `subtree_end_height`
/// unconditionally, and a strictly-increasing sequence of attacker-chosen heights
/// inside `[activation, tip]` collapses the shard join windows and latches
/// `witness_stabilized` (INC-020 clause (g), the A11 bypass). So: **every non-test
/// caller routes through `Wallet::update_subtree_roots`.** Every in-tree caller
/// that reaches this function directly — `ironwood_spendability`'s modelled S3/S2
/// rows with four hardcoded mainnet heights, and its harness `write_the_roots` with
/// generator-derived ones — is a `#[cfg(test)]` fixture of the predicate BELOW the
/// bind, not a pattern to copy.
///
/// **`put_ironwood_subtree_roots` is the write half of the whole item.** Until it
/// is called, `ironwood_tree_shards.subtree_end_height` stays NULL forever — the
/// shardtree's own scan write-back (`put_shard`) omits that column from both the
/// insert and the `DO UPDATE SET`, so no amount of scanning can ever fill it, and
/// only `put_shard_roots` (reached ONLY from these `put_*_subtree_roots` calls)
/// writes it. A NULL there keeps every Ironwood note out of `mark_stabilized_notes`
/// and drops Ironwood out of `min_shard_tip`. Measured, level X:
/// `docs/plan/probes/ironwood-unspendability-repro.py` — and guarded in-tree by
/// `ironwood_spendability`'s modelled rows (the probe's S3 and S2 cells on the real
/// mainnet schema with the crate's own SQL, both driven through THIS function; the
/// module header lists every row and what each does not prove). Delete the Ironwood
/// arm below and both go red, with the two harness root rows beside them.
///
/// NOT cross-pool atomic: the three pools are three upstream calls (three
/// transactions), mirroring the reference `sync::run`. The trees are independent
/// and the write is idempotent at its start, so a fault on a later pool's put heals
/// on the next sync's re-put (an unwritten pool's stored run is unchanged, so its
/// next plan re-serves the same indices) — never a mis-built witness. A well-formed-but-WRONG
/// server root is cached on FIRST ingestion (the put succeeds) and caught at SCAN
/// time as a shardtree `Conflict` on a completed shard (iv-d-2); only a cross-sync
/// root MUTATION conflicts at put time, attributed honestly by [`map_shardtree_err`].
///
/// **`recorded` runs once per pool, immediately after THAT pool's put returned `Ok`
/// and before the next pool's begins** (T0-1a-R2, §4g 0b). `Wallet::update_subtree_roots`
/// supplies the rewind ledger's per-pool consume (`root_bind::note_roots_recorded`);
/// the unit rows that drive the write alone pass `|_| Ok(())`. It never runs for a
/// pool whose put failed, nor for any pool after it. The pre-R2 shape consumed after
/// all three puts and only if all three succeeded, so an endpoint that let Sapling's
/// downward step be written and then made Orchard's put fail (a mutated `root_hash`
/// at a recorded index is a shardtree `Conflict`, mapped by [`map_shardtree_err`])
/// kept Sapling's relaxation armed — one rewind bought unbounded downward steps on
/// every pool but the last. A hook rather than a second connection parameter so the
/// aux lock is taken per pool and never held across a linear shard write (the S30
/// lock-hold caveat on the caller), and so the ORDER — put A, consume A, put B,
/// consume B, put C, consume C — reads in one place. The put/consume window itself
/// is stated on `note_roots_recorded`.
///
/// **Generic over the consensus parameters ONLY**, through the named alias
/// [`WalletConnFor`]`<P>`: the live wallet runs it over [`WalletConn`] (`P =
/// Network`) and the funded `data_api::testing` harness runs it over
/// `LocalNetwork`, and `ironwood_spendability` must drive THIS function rather
/// than upstream's `put_ironwood_subtree_roots` underneath it — the whole finding
/// `INC-020` records is that our wrapper had no caller. The MOTIVE is the one
/// [`crate::send::propose_core`] has; the MECHANISM is narrower on purpose:
/// `propose_core` is generic over the whole store (`DbT: InputSource + …`) because
/// it needs several traits of it, while this function needs only upstream's
/// `WalletCommitmentTrees` impl on the concrete handle, so the connection, clock
/// and RNG stay what they are everywhere else (§4ac row 9). The error type is
/// unchanged: the shard store's `commitment_tree::Error` does not depend on the
/// parameters, so [`map_shardtree_err`] still reads one type.
pub(crate) fn put_subtree_roots<P: Parameters>(
    db: &mut WalletConnFor<P>,
    roots: &SubtreeRoots,
    mut recorded: impl FnMut(ShieldedProtocol) -> Result<(), WalletError>,
) -> Result<(), WalletError> {
    // Destructured BY NAME on purpose: a pool collected into `SubtreeRoots` that
    // nobody writes is the exact defect this item repairs, so the compiler is made
    // to name it here rather than leaving it to be noticed. (`outcomes` is the
    // record of what the endpoint did, not something to write.)
    let SubtreeRoots {
        sapling,
        orchard,
        ironwood,
        // Read by `apply_height_bind` before this point, never written to the tree:
        // `completing_block_hash` is not a column upstream keeps.
        completing_hashes: _,
        outcomes: _,
        // Each pool is written AT ITS OWN START (S15-F1): 0 for a full fetch, the
        // plan's start for an incremental one, whose prefix stays as stored.
        starts,
    } = roots;
    // The three slots, named against `SUBTREE_ROOT_POOLS` (compile-time asserted at
    // the top of this module), so a pool cannot be written at another's start.
    let [sapling_start, orchard_start, ironwood_start] = starts.map(u64::from);
    db.put_sapling_subtree_roots(sapling_start, sapling)
        .map_err(map_shardtree_err)?;
    recorded(ShieldedProtocol::Sapling)?;
    db.put_orchard_subtree_roots(orchard_start, orchard)
        .map_err(map_shardtree_err)?;
    recorded(ShieldedProtocol::Orchard)?;
    db.put_ironwood_subtree_roots(ironwood_start, ironwood)
        .map_err(map_shardtree_err)?;
    recorded(ShieldedProtocol::Ironwood)?;
    Ok(())
}

// ── The per-batch scan primitive (§3.2g inc-2c-iv-d-2b-i) ────────────────────
//
// OUR orchestration over the consumed-WHOLE `zcash_client_backend` scan
// primitives, factored as the "fetch (async, no lock) + apply (sync, under the db
// lock)" pair the iv-d-1 subtree-root path established. The iv-d-2b-ii driver loop
// (`suggest_scan_ranges` → batch → reorg re-suggest) composes these; here is the
// single-batch unit where the money-critical correctness lives — the panic GUARD,
// the COMPLETENESS check, and the reorg classification.

/// What the scan loop needs from the (untrusted) endpoint per batch — the
/// per-range siblings of [`SubtreeRootSource`]: the block-download stream and the
/// anchoring tree state. A testability seam (the crate ships no gRPC server): the
/// loop is driven through a fake (`testing::FakeScanClient`) with no live network.
#[async_trait]
pub(crate) trait ScanClient {
    /// Open a stream of CompactBlocks for the INCLUSIVE range `[start, end_inclusive]`
    /// (the gRPC wire contract; the iv-a `BlockStream`).
    async fn block_range(
        &mut self,
        start: u64,
        end_inclusive: u64,
    ) -> Result<BlockStream, GrpcError>;
    /// Fetch the commitment-tree state at `height` (the per-batch scan anchor — the
    /// block BELOW the batch start; the reference's `download_chain_state`).
    async fn tree_state(&mut self, height: u64) -> Result<TreeState, GrpcError>;
    /// The best-chain tip HEIGHT (the loop records it via `update_chain_tip` so
    /// `suggest_scan_ranges` tracks the moving tip; the reference's
    /// `update_chain_tip`). RAW height from an UNTRUSTED endpoint — a lying-HIGH
    /// tip over-suggests (the download then short-reads → typed `Sync`, recoverable),
    /// a lying-LOW tip under-suggests (recent funds wait for an honest tip — the M2
    /// forward-owed concern, §3.2f); never a fund loss, never a panic.
    async fn latest_block_height(&mut self) -> Result<u64, GrpcError>;
}

#[async_trait]
impl ScanClient for LightwalletdClient {
    async fn block_range(
        &mut self,
        start: u64,
        end_inclusive: u64,
    ) -> Result<BlockStream, GrpcError> {
        self.get_block_range(start, end_inclusive).await
    }
    async fn tree_state(&mut self, height: u64) -> Result<TreeState, GrpcError> {
        self.get_tree_state(height).await
    }
    async fn latest_block_height(&mut self) -> Result<u64, GrpcError> {
        Ok(self.get_latest_block().await?.height)
    }
}

/// The §3.3a Recv-2b transparent-UTXO detection seam (an own port, ISP — distinct from the
/// shielded-scan [`ScanClient`]): poll the endpoint for the wallet's transparent receive
/// address. Segregated so the many `ScanClient`/`SubtreeRootSource` test fakes need not know
/// about transparent detection — only the production [`LightwalletdClient`] and the focused
/// `Wallet::refresh_transparent_utxos` test fake implement it. A testability seam (no live
/// gRPC): the fake returns canned [`TransparentUtxoRecord`]s with no network.
#[async_trait]
pub(crate) trait TransparentUtxoSource {
    /// Current unspent transparent outputs paying any of `addresses`, mined at or above
    /// `start_height`. RAW records from an UNTRUSTED endpoint — validated at the §4.6 boundary
    /// by [`crate::transparent::validate`] before anything is put.
    async fn address_utxos(
        &mut self,
        addresses: Vec<String>,
        start_height: u64,
    ) -> Result<Vec<TransparentUtxoRecord>, GrpcError>;
}

#[async_trait]
impl TransparentUtxoSource for LightwalletdClient {
    async fn address_utxos(
        &mut self,
        addresses: Vec<String>,
        start_height: u64,
    ) -> Result<Vec<TransparentUtxoRecord>, GrpcError> {
        self.get_address_utxos(addresses, start_height).await
    }
}

// The §3.3 tx-enhancement fetch port [`TransactionFetcher`] lives in the `enhance` domain module
// (alongside `FetchedTransaction` and the §4.6 boundary it feeds); this is just the production
// gRPC adapter for it, kept here with the other `LightwalletdClient` impls.
#[async_trait]
impl TransactionFetcher for LightwalletdClient {
    async fn fetch_transaction(
        &mut self,
        txid: TxId,
    ) -> Result<Option<FetchedTransaction>, GrpcError> {
        self.get_transaction(txid).await
    }
}

/// The outcome of scanning one batch ([`scan_batch`]).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ScanOutcome {
    /// The batch scanned cleanly; the driver advances to the next suggested range.
    /// `received_notes` is the engine `ScanSummary`'s received-note count (sapling +
    /// orchard) for the batch — the ADR-0536 CHEAP GATE for the incoming-funds
    /// detect: the (bounded, aux-read) truth diff runs ONLY when it is non-zero, so
    /// the overwhelmingly-common empty batch of a deep restore costs zero extra
    /// queries. It OVER-approximates arrivals (change/self-send notes count too) —
    /// the diff is the filter, this is only the gate.
    ///
    /// `boundary` is BIND-1's scan-time verdict for this batch (§4x): what the
    /// wallet's own per-block counts had to say about the completion heights it
    /// already holds. Carried OUT of the scan rather than only logged, for the
    /// reason [`SyncPass::root_outcomes`] is — and, since §4w (f5), for a second
    /// one: this runs inside `run_blocking`, so a `tracing` EVENT emitted here
    /// resolves a tokio worker's dispatcher and is invisible to a thread-local
    /// capture. "The pool reports it" has to be a RETURNED value or it is not a
    /// report at all.
    Scanned {
        received_notes: usize,
        boundary: BoundaryReport,
    },
    /// A reorg (continuity error): the server's block history diverged from our
    /// scanned chain. The wallet DB has ALREADY been rewound to `rewind_height`
    /// (under the same db lock, inside [`scan_batch`]) and, when `requeued`, the
    /// rewound span `rewind_height + 1 ..= tip` has ALREADY been re-queued (§4q
    /// P-R1, INC-025); the driver must `cache.truncate(rewind_height)` to match
    /// and re-`suggest_scan_ranges`, which then finds that span.
    ///
    /// `requeued` is `false` when the arm SKIPPED the re-queue because the recorded
    /// tip is at or below `rewind_height` (§4q-R P-RR1 — the endpoint is behind
    /// our own scan, or a range a cancelled pass left queued above the tip forked):
    /// upstream's `update_chain_tip` early-returns on `new_tip < max_scanned`, the
    /// queue stays trimmed, and the pass ends as the under-claim it is. The loop
    /// reads it only to choose the cause word at its up-to-date exit; whether the
    /// pass may end is the exit's own DB read (P-RR2).
    ///
    /// `rewound_blocks` is the DB's scanned frontier BEFORE the truncate minus
    /// `rewind_height` — the blocks this rewind actually un-scanned (§4q-R P-RR4).
    /// The loop sums it into `SyncPass::rewound_blocks`, the `depth`
    /// `wallet.reorg_rewind` logs. A count of blocks, never a height.
    ///
    /// `rewind_height` is the height upstream's `truncate_to_height` LANDED on —
    /// the highest checkpointed height at or below the [`rewind_target`] — and
    /// never the target itself (§4q P-R2): the two differ whenever no checkpoint
    /// sits at the target, and the cache truncated to the target would then hold
    /// blocks the DB no longer does. Every reader follows this one value
    /// (`classify_batch` → `BatchAction::Rewind` → the cache truncate and the
    /// post-rewind progress sample). A reorg is recoverable denial-of-progress
    /// (re-scan the rewound span) — §1.7, NEVER fund loss / theft (a rewind only
    /// un-scans; balances re-derive on re-scan).
    Reorg {
        rewind_height: u64,
        requeued: bool,
        rewound_blocks: u64,
    },
}

/// **BIND-1 (§4x): what the scan-time boundary reconcile found, per pool, on one
/// batch** — `Some` for a pool whose recorded completion heights the wallet's own
/// scanned counts refuted, `None` for one it had nothing to say about.
///
/// Positional in [`SUBTREE_ROOT_POOLS`] order, like every other per-pool array in
/// this module, so the slot a report lands in is the slot the pass's
/// [`SyncPass::root_outcomes`] renders.
///
/// **Only the Ironwood slot can be `Some` on a returned value today**, and that is
/// a disposition and not a property of this type: a refuted Sapling or Orchard row
/// is pass-fatal ([`reconcile_scan_boundaries`] says why), so the batch returns
/// `Err` and no report survives to be carried. The array stays three wide because
/// the disposition is the thing under review — a type that could only express the
/// Ironwood outcome would have to be rebuilt to change it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BoundaryReport {
    /// `pub(crate)` so a row driving [`scan_batch`] directly can assert on the
    /// slot it names, rather than only through the pass the loop assembles.
    pub(crate) outcomes: [Option<PoolFetch>; SUBTREE_ROOT_POOLS.len()],
}

impl BoundaryReport {
    /// Fold this batch's verdict into the pass's per-pool outcomes, overwriting
    /// only the pools it actually judged.
    ///
    /// The scan-time verdict OVERRIDES the ingest's for a pool it refuted, and the
    /// direction is the honest one: the ingest said "this serve is admissible on
    /// what I knew then", the scan says "the row it left cannot be true", and the
    /// second statement is made with strictly more evidence. A pool this report is
    /// silent about keeps whatever the ingest recorded for it — silence here is
    /// never "served".
    pub(crate) fn apply_to(&self, outcomes: &mut [PoolFetch; SUBTREE_ROOT_POOLS.len()]) {
        for (slot, found) in self.outcomes.iter().enumerate() {
            if let Some(outcome) = found {
                outcomes[slot] = *outcome;
            }
        }
    }
}

/// **BIND-1's seam (§4x): hold every pool's RECORDED completion heights to what
/// this batch actually counted**, correcting the rows the scan refutes and
/// deciding what each refusal costs.
///
/// Runs from [`scan_batch`]'s `Ok` arm, after upstream's transaction has committed,
/// under the db lock, on the aux connection — the choice §4x's "does NOT decide" 3
/// left open. **What that seam buys:** the exact post-commit state, including the
/// `witness_stabilized` latch `mark_stabilized_notes` may have just set inside that
/// same transaction, in one lock acquisition, before any other reader can see it.
/// **What it costs:** the reconcile runs on every batch of every pass rather than
/// once per pass, and a fault in it fails the batch. The per-pool cheap exit
/// (nothing recorded ⇒ one small read and out, `root_bind::reconcile_scanned_boundaries`)
/// is what keeps that affordable on the mid-heal wallet this item exists for.
///
/// # The disposition, and it is the ingest's (§4x "does NOT decide" 1)
///
/// **Sapling and Orchard are FATAL to the pass; Ironwood degrades to a reported,
/// refuted pool** — the same split [`apply_height_bind`] makes at ingest, for the
/// same reasons, and the floor B1-4 states. The alternative (degrade all three,
/// since the batch is already committed and its progress is durable) was the real
/// option and it is refused: a required pool whose recorded heights the wallet's
/// own scan contradicts is a server whose answers cannot build the witnesses this
/// wallet needs to SPEND, and continuing to scan on it would mean writing more
/// blocks against a tree we have just proved wrong at a recorded index. **What
/// fatal buys:** one class and one grep for "this server's roots are wrong",
/// whichever door catches it, and a pass that stops on the pool that matters.
/// **What it costs:** a pass that fails AFTER a batch committed — the scan progress
/// stands (it is durable and correct; the refutation is about a height row, not
/// about the blocks), so the cost is one stalled pass per occurrence and the
/// "switch servers" remedy, not lost work. **The exit is real and is the whole
/// point:** the refuted row has already been corrected or NULLed by the time this
/// returns, so the next pass's ingest either accepts the honest height on equality
/// or refuses the lie with the scanned-count oracle now reaching it.
fn reconcile_scan_boundaries(
    aux: &rusqlite::Connection,
    network: Network,
    from_state: &ChainState,
    scanned: std::ops::Range<BlockHeight>,
) -> Result<BoundaryReport, WalletError> {
    let anchor_height = u32::from(from_state.block_height());
    let lo = u32::from(scanned.start);
    // `scanned_range` is half-open; the reconcile's block read takes it inclusive. An
    // EMPTY range (nothing scanned) saturates to `hi < lo`, which reads no blocks
    // and leaves the anchor as the only observation.
    let hi = u32::from(scanned.end).saturating_sub(1);
    let mut report = BoundaryReport::default();
    let mut required_refused = false;
    // R6: the loop runs Sapling → Orchard → **Ironwood last**, and before this repair
    // every call was `?`-propagated — so a transient `StoreBusy` on Sapling's aux read
    // skipped Orchard AND Ironwood for a batch that had ALREADY COMMITTED, and by this
    // module's own "examined by the batch that contained it and not again" that range
    // never returned. The fault is now CARRIED, not propagated: every pool is
    // reconciled, and the first fault is returned once they all have been. It is not
    // swallowed — the batch still fails on it — but it no longer silences the pool
    // this item exists for.
    let mut first_fault: Option<WalletError> = None;
    for (slot, &pool) in SUBTREE_ROOT_POOLS.iter().enumerate() {
        // EXHAUSTIVE and wildcard-free, like every other per-pool match in this
        // module: a fourth pool must name its own frontier here or fail to compile,
        // rather than silently reconciling against another pool's tree.
        let anchor_size = match pool {
            ShieldedProtocol::Sapling => from_state.final_sapling_tree().tree_size(),
            ShieldedProtocol::Orchard => from_state.final_orchard_tree().tree_size(),
            ShieldedProtocol::Ironwood => from_state.final_ironwood_tree().tree_size(),
        };
        // The corrections are applied inside this call for EVERY pool, including
        // the two whose refusal is fatal: the repair is what stops the false latch,
        // and it must not be conditional on what the pass then decides to do about
        // the server.
        //
        // **NO BUSY RETRY HERE, and removing it is the repair** (the join's
        // sixth finding). `db::with_aux_busy_retry` re-runs its whole closure, and
        // this closure is reads → COMMIT → one more read (`read_rewind_watch`, for
        // the blame gate). A busy on that LAST read re-ran the body, which then found
        // the correction already applied, derived nothing, and returned `Ok(None)` —
        // no badge, no pass-fatal disposition. A transient BUSY silently converting a
        // DETECTED endpoint lie into an undetected one is exactly the MEDIUM the
        // retry was added to close, reintroduced by the closing. The helper's own
        // soundness contract (`db.rs:537-543`) asks for ONE `IMMEDIATE` transaction
        // per closure, and reads-then-commit-then-read is not that.
        //
        // What actually closes P6 is the `first_fault` carry below: every pool is
        // reconciled even when an earlier one faults, so a transient on Sapling no
        // longer skips the pool this item exists for. A busy that reaches here fails
        // the batch honestly, and the batch is retried by the next pass.
        let refusal = match root_bind::reconcile_scanned_boundaries(
            aux,
            network,
            pool,
            anchor_height,
            anchor_size,
            (lo, hi),
        ) {
            Ok(Some(refusal)) => refusal,
            Ok(None) => continue,
            Err(e) => {
                // Said out loud (principle 10): a pool the reconcile could not run
                // for is a pool whose recorded heights went unexamined for a range
                // that will not come back. §5.4: a pool name and a stable code.
                tracing::warn!(
                    target: "zec_wallet_core",
                    pool = pool.as_str_name(),
                    outcome = "scanned_boundary_fault",
                    "the scan-time boundary reconcile could not run for this pool on \
                     this batch — its recorded completion heights went unexamined for \
                     a range this wallet has already committed and will not re-read"
                );
                first_fault.get_or_insert(e);
                continue;
            }
        };
        let outcome = PoolFetch::HeightViolation {
            code: refusal.code(),
        };
        report.outcomes[slot] = Some(outcome);
        // Said out loud as well as returned (principle 10) — and the RETURNED value
        // is the one anything may assert on (§4w (f5)).
        report_pool_outcome(pool, outcome);
        if !matches!(pool, ShieldedProtocol::Ironwood) {
            required_refused = true;
        }
    }
    // The endpoint's fault outranks ours: a required pool whose recorded heights this
    // wallet's own scan contradicts is a server to switch away from, and saying
    // "store fault" for it would send the user to the wrong remedy.
    if required_refused {
        return Err(endpoint_unusable());
    }
    if let Some(fault) = first_fault {
        return Err(fault);
    }
    Ok(report)
}

/// Whether `download_range` delivered the full span or returned early on
/// cooperative cancellation (§3.2g iv-d-3-b). `Cancelled` is NOT an error: the
/// in-flight download buffer was never written to the cache, so the only thing
/// lost is re-downloadable work — durable scan progress lives in the wallet DB,
/// never in the disposable cache. The next pass re-downloads the range cleanly.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DownloadOutcome {
    /// The whole span is in the cache. `outputs` is the batch's PUBLIC
    /// shielded-output count — Sapling outputs + Orchard actions + Ironwood
    /// actions, summed over every compact tx the stream delivered
    /// ([`shielded_outputs`]; SCAN-1, §4o S1) — the denominator a per-output
    /// scan cost is measured against. It is chain data over `[start, end)`, the
    /// same class as `blocks` (a number every observer of the chain already
    /// has), and says nothing about which of those outputs, if any, are this
    /// wallet's.
    Completed {
        outputs: u64,
    },
    Cancelled,
}

/// The AUTHORITATIVE scan-progress + spendability for the `Scanning` UX (§3.2g
/// iv-d-3-b-iii), read from the audited `WalletRead::get_wallet_summary()` after each
/// committed batch ([`crate::account::progress_snapshot`]). Distinct from the
/// [`ScanProgress::frontier`] (a per-range DOWNLOAD position — liveness/watchdog
/// only): `percent` is the MONOTONIC scanned/total-notes fraction (§2.5 — never range
/// position) and `spendable_ready` the Spend-before-Sync signal (§1.7). `Copy` so it
/// rides the `Copy` [`ScanCtx`]; `Default` = `{0.0, false}` (the pre-first-batch carry
/// value). NOT `Eq` — it carries an `f32` (a display fraction, never a money decision).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct ProgressSnapshot {
    /// Scan fraction in `[0, 1]` from `Progress::scan()` (a zero denominator — no
    /// shielded notes in the birthday→tip window — means there is nothing to scan ⇒
    /// `1.0`; never a divide-by-zero). Monotonic absent a reorg; a reorg rewind lowers
    /// it to the honest post-rewind truth (`crate::account::scan_percent`). UX-only
    /// (§2.5).
    pub(crate) percent: f32,
    /// Any spendable shielded value RIGHT NOW under the ZIP-315 default confirmations
    /// policy (§1.7 funds-usable-before-100%). Honest "you can spend" signal.
    pub(crate) spendable_ready: bool,
}

/// One sync-progress sample emitted DURING a pass — the d-3 stuck-sync watchdog's
/// re-arm tick + the per-range position the live `Scanning` status renders, plus the
/// authoritative [`ProgressSnapshot`] the engine forwards as the monotonic UI percent.
/// Pure SDK data (heights are §5.4-loggable); NO controller / librustzcash coupling,
/// so the d-3-b engine maps it onto the controller's `SyncProgress` without `sync.rs`
/// depending on the controller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScanProgress {
    /// The height currently being downloaded/scanned within the ACTIVE suggested
    /// range. **NOT a globally monotonic percent denominator, and NOT the rendered
    /// blocks-left position.** Spend-before-Sync scans the highest-priority range FIRST
    /// (the Verify range, near the tip), so across a multi-range pass this frontier is
    /// NOT strictly ascending — it is a per-range position + the watchdog liveness tick
    /// ONLY. BOTH UI numbers derive from `summary.percent` (the monotonic wallet-summary
    /// fraction): the live `Scanning` percent directly, and the `from` blocks-left height
    /// via `wallet::scanned_equiv` — NEVER from a naive `frontier / tip` (which would
    /// regress when the scan plan jumps between priority ranges — the §2.5 "never range
    /// position" warning, the "1 %, 289 blocks left" footgun). The frontier reaches the
    /// rendered `from` only as the pre-provision fallback (no scan floor yet). Clamped to `tip`.
    pub(crate) frontier: crate::money::BlockHeight,
    /// The recorded chain tip the pass is scanning toward.
    pub(crate) tip: crate::money::BlockHeight,
    /// The AUTHORITATIVE monotonic percent + spendability (§3.2g iv-d-3-b-iii). On a
    /// post-batch report it is the freshly-read wallet summary; on an intra-batch
    /// download tick it is the prior committed batch's summary CARRIED FORWARD (no
    /// notes are scanned mid-download, so the fraction does not move until the batch
    /// commits — `{0.0, false}` before the first batch). The engine forwards this
    /// verbatim into `SyncStatus::Scanning`.
    pub(crate) summary: ProgressSnapshot,
    /// The pass has rewound at least once (§2.5 `Scanning.rewound`, #317).
    /// [`sample`](Self::sample) constructs it `false`; `Wallet::sync_once`'s
    /// report wrapper stamps the live value — samples never carry it raw.
    pub(crate) rewound: bool,
}

impl ScanProgress {
    /// Build a sample from RAW `u64` heights + the carried/just-read [`ProgressSnapshot`].
    /// Both heights fit `u32` (consensus heights: `tip` is `u32`-range-checked in
    /// [`fetch_tip`]; `frontier` rides a `suggest_scan_ranges`-derived `u32` range), so
    /// the cast is exact — the saturate is defensive only. `frontier` is clamped to
    /// `tip` (a server streaming past the tip is rejected upstream, but the clamp keeps
    /// the rendered position sane regardless).
    pub(crate) fn sample(frontier: u64, tip: u64, summary: ProgressSnapshot) -> ScanProgress {
        let frontier = frontier.min(tip);
        ScanProgress {
            frontier: crate::money::BlockHeight::new(u32::try_from(frontier).unwrap_or(u32::MAX)),
            tip: crate::money::BlockHeight::new(u32::try_from(tip).unwrap_or(u32::MAX)),
            summary,
            rewound: false,
        }
    }
}

/// The cooperative-cancel + progress context for one batch download (§3.2g
/// iv-d-3-b), bundled so the money-critical per-batch call site stays readable
/// (and the d-3-b engine wires one value, not three positional args): the cancel
/// flag polled per block, the live progress sink, and the recorded tip each sample
/// carries. Tiny + `Copy` (a u64 + two borrows) — built once per pass, passed by
/// value per batch.
#[derive(Clone, Copy)]
pub(crate) struct ScanCtx<'a> {
    /// Recorded chain tip — carried in every [`ScanProgress`] sample this pass.
    pub(crate) tip: u64,
    /// Polled per block during a download; a flip returns [`DownloadOutcome::Cancelled`].
    pub(crate) cancel: &'a CancelToken,
    /// Live progress sink. During a download: a [`ScanProgress`] every
    /// [`PROGRESS_REPORT_BLOCKS`] blocks. After a committed batch: once more, called
    /// by `sync_once` directly (the honest post-scan position — guarantees ≥ 1 sample
    /// even for a batch smaller than the cadence).
    pub(crate) report: &'a (dyn Fn(ScanProgress) + Sync),
    /// The authoritative [`ProgressSnapshot`] to stamp this batch's download ticks
    /// with — the PRIOR committed batch's wallet summary, carried forward (no notes
    /// are scanned mid-download, so the monotonic fraction does not move until the
    /// batch commits; `{0.0, false}` for the first batch). `sync_once` refreshes it
    /// from `get_wallet_summary()` after each committed batch (§3.2g iv-d-3-b-iii).
    pub(crate) summary: ProgressSnapshot,
}

/// The endpoint returned a clean-but-WRONG response — usable transport, unusable
/// DATA (a short/gapped/overlong block span, a tree state that won't decode or
/// anchors the wrong height, a malformed block or root, a flooding stream, a
/// required pool the server does not know, a completion-height sequence the
/// wallet's own evidence refutes). Retryable / fallback signal, NOT local
/// corruption ("repair your wallet"). Transport-LAYER faults already carry their
/// precise stall via `transport_err`; this is the content-tier sibling and it is
/// ONE class with ONE stall, [`StallReason::EndpointMisbehaving`] — "this server
/// is wrong; switch servers". Until T0-1b it returned `EndpointUnreachable`, so
/// every one of its sites rendered as a dead link and sent the user to check a
/// connection that worked: the masquerade `StallReason::Internal` was minted to
/// end, one tier over. The adjudication re-aimed the helper rather than one site,
/// so no site can fall back into it. Never `TorUnavailable`, never `Internal`.
///
/// `pub(crate)` since T0-1c: `provision::resolve_birthday`'s tip conversion is the
/// one content-tier site outside this module, and it used to spell its stall out
/// by hand — a direct construction that a grep for this helper cannot see (the
/// undercount, three sites in one sweep). Every content-tier site goes
/// through here, so the class has one definition and one grep.
pub(crate) fn endpoint_unusable() -> WalletError {
    WalletError::Sync {
        stall: StallReason::EndpointMisbehaving,
    }
}

/// Classify a wallet-DB fault surfaced during a scan into the SDK store taxonomy,
/// honest about disk-full / transient-busy / IO vs true corruption (**#371**).
///
/// The engine's [`scan_cached_blocks`] writes OUR wallet DB; a fault there arrives as
/// `ChainScanError::Wallet(W)` where the production `W` is [`SqliteClientError`]. The
/// pre-#371 code folded EVERY such fault to `StoreCorrupt` — so a mid-scan `SQLITE_FULL`
/// (out of disk) rendered the RED "restore from your recovery phrase" remedy, while the
/// cache side already showed the honest `DiskFull`: one condition, two contradictory
/// stories (the review MED). This trait lets [`map_scan_err`] PROBE the concrete
/// error — mirroring [`crate::send::GapLimitProbe`], which peers into the same
/// `SqliteClientError` for the TEX gap-limit ceiling — WITHOUT pinning the mapper to a
/// concrete backend (the generic-over-`W` shape is kept for the synthetic-error tests).
///
/// `pub(crate)`, not private: it sits in the `where`-bound of `pub(crate)`
/// [`map_scan_err`], so the `private_bounds` lint requires at-least-equal visibility. An
/// internal mapper detail — never part of the public API.
pub(crate) trait ClassifyStoreFault {
    /// Consume the fault and return its SDK taxonomy. Total: the conservative default is
    /// the fail-closed `StoreCorrupt` (unchanged from pre-#371 for anything unrecognized).
    fn into_store_fault(self) -> WalletError;
}

// A shard-tree fault carried by any of the three `SqliteClientError`
// commitment-tree variants used to be classified HERE, by a private
// `classify_shard_fault` that differed from [`map_shardtree_err`] in exactly one
// arm — and that arm is B1-5 (§4x). It is gone; both doors now run the one body.
// [`map_shardtree_err`] carries the argument.

impl ClassifyStoreFault for SqliteClientError {
    fn into_store_fault(self) -> WalletError {
        match self {
            // The direct SQLite door: route through the shared SSOT
            // ([`crate::db::classify_sqlite_error`]) so this seam and `map_aux_err` can
            // never drift (FULL→DiskFull, BUSY→StoreBusy, IOERR→Io, else→StoreCorrupt).
            SqliteClientError::DbError(e) => crate::db::classify_sqlite_error(&e),
            // NOT a filesystem error: a DECODE of our own stored bytes (R12 §4.1) — see
            // [`engine_io_fault`]. `StorageFull` → `DiskFull`, every other kind → corrupt.
            SqliteClientError::Io(e) => engine_io_fault(e),
            // THE commitment-tree (shard) write door. This is a first-class part of the
            // scan write path and among the LARGEST per-batch writes (shard BLOBs), so a
            // mid-scan ENOSPC is MOST LIKELY to fault HERE — and its `Storage` arm carries
            // the underlying `rusqlite`/io error verbatim. Unwrap it through the SAME
            // classifier so a full disk is `DiskFull` here too, not the `StoreCorrupt`
            // seed-restore scare (the reliability review's HIGH — without this the fix's
            // headline guarantee failed on the likeliest real path). A `Query` shard
            // error is a tree LOGIC fault (our own inconsistency) and keeps the
            // fail-closed `StoreCorrupt`.
            //
            // **`Insert` is the ENDPOINT's, and since B1-5 (§4x) it says so HERE too.**
            // A served root the tree rejects at SCAN time is a `shardtree` `Conflict`
            // (`shardtree-0.7.1/src/prunable.rs:946`, an `InsertionError` → a
            // `ShardTreeError::Insert`) raised when the leaves this wallet just counted
            // refute the root hash the endpoint served for that shard — the iv-d-2
            // detection this SDK's own spec names as what catches a well-formed-but-WRONG
            // root. It arrived here as `PutBlocksCommitmentTree` and fell to the
            // fail-closed `StoreCorrupt`: an endpoint lie, rendered to the user as "your
            // wallet is corrupt, restore from your recovery phrase", on an intact wallet,
            // from a condition one hostile server can produce at will (§4w (b)). The
            // INGEST door already called the identical class the endpoint's
            // ([`map_shardtree_err`]'s `Insert` arm, T0-1d), so this was ONE class with
            // TWO answers depending on which door caught it. There is now one body and
            // one grep: both doors run [`map_shardtree_err`].
            //
            // THREE variants carry that error, not one. `zcash_client_sqlite` 0.22.0
            // split the single `CommitmentTree` into three: the original, plus
            // `PutBlocksCommitmentTree` (the per-block-range shard write, which is the
            // scan path's own door and where an ENOSPC is likeliest) and
            // `TruncateCommitmentTree` (the reorg rewind). Under 0.21.0 both of those
            // faults arrived as `CommitmentTree` and classified correctly; matching
            // only the original after the bump would have left #371's headline
            // guarantee DEAD on the path it was written for — a full disk mid-scan
            // rendering the red "restore from your recovery phrase" again — while the
            // guard test kept passing, because it hand-constructs the one variant
            // production no longer produces. Found by crypto audit on this diff.
            //
            // Factored into ONE helper so a fourth variant cannot be added to two arms
            // and forgotten in the third — and since B1-5 that helper is the one the
            // ingest door already used, so the two cannot drift again.
            SqliteClientError::CommitmentTree(e)
            | SqliteClientError::PutBlocksCommitmentTree { error: e, .. }
            | SqliteClientError::TruncateCommitmentTree { error: e, .. } => map_shardtree_err(e),
            // ENDPOINT DATA, not our corruption (T0-1 B3). `put_blocks` raises this
            // when the anchoring `from_state` does not agree with the first block of
            // the batch — and since 0.24.0 that agreement is checked for THREE pools:
            // `final_ironwood_tree().tree_size() + block.ironwood().commitments().len()
            // == block.ironwood().final_tree_size()`
            // (`zcash_client_backend-0.24.0/src/data_api/ll/wallet.rs:337-339`). The
            // `from_state` comes from the endpoint's `GetTreeState` on a FETCHED
            // batch (`fetch_chain_state` — the first of a pass, one after a rewind or
            // a queue jump, and every `TREE_STATE_RECONCILE_BATCHES`th consecutive
            // one; SCAN-2, §4t) and is otherwise derived from the endpoint's own
            // blocks (`derive_chain_state`), so an endpoint that omits
            // `ironwood_tree` on ONE batch treestate reaches this on that batch —
            // and before this arm it fell through to the catch-all and told a user
            // whose wallet is intact to restore from their recovery phrase.
            //
            // The block-sequence half of the same variant is unreachable from here:
            // `download_range` already proves the span is contiguous and in order
            // before anything is cached.
            //
            // `endpoint_unusable` is the honest door and the one this codebase
            // already uses for "clean transport, unusable DATA" — the same
            // classification a malformed block gets from `map_scan_err`. The wallet
            // stays open and readable, sync stalls and says so, and the next step is
            // retry-or-switch-endpoint rather than a destructive recovery.
            SqliteClientError::NonSequentialBlocks => endpoint_unusable(),
            // Every other `SqliteClientError` variant (decoding, gap-limit — the last
            // handled upstream via `GapLimitProbe`, tree Query/Insert logic faults):
            // unchanged fail-closed `StoreCorrupt`.
            _ => WalletError::StoreCorrupt,
        }
    }
}

// The synthetic-backend arm ([`map_scan_err`]'s unit tests pass `W = WalletError`): a
// `WalletError` reaching the `Wallet` arm is already classified — return it unchanged.
impl ClassifyStoreFault for WalletError {
    fn into_store_fault(self) -> WalletError {
        self
    }
}

// The shard-STORAGE fault type, so [`map_shardtree_err`]'s `Storage` arm classifies a
// disk-full/IO condition on the pre-scan subtree-root write ([`put_subtree_roots`]) as
// honestly as the scan door does (#371). `Query`/`Serialization` carry the underlying
// `rusqlite`/io error; the tree-logic variants (`CheckpointConflict`/`SubtreeDiscontinuity`)
// are our own inconsistency → the fail-closed `StoreCorrupt` default.
impl ClassifyStoreFault for commitment_tree::Error {
    fn into_store_fault(self) -> WalletError {
        match self {
            commitment_tree::Error::Query(e) => crate::db::classify_sqlite_error(&e),
            commitment_tree::Error::Serialization(e) => engine_io_fault(e),
            _ => WalletError::StoreCorrupt,
        }
    }
}

/// The engine's `io::Error` (`SqliteClientError::Io`, `commitment_tree::Error::
/// Serialization`) is a DECODE fault, not a filesystem one (R12 §4.1, security review
/// BLOCKER): at the `zcash_client_sqlite` 0.22.0 pin no value of these two types comes
/// from a file syscall — that crate touches files only in `chain.rs` (`FsBlockDb`, its own
/// error type) and `testing.rs`, and every source decodes a buffer SQLite already returned
/// (`parse_tx` over `&[u8]`, `read_shard`/`H::read` over a `Cursor`, `write_shard` into a
/// `Vec`). A real short read fails inside SQLite (`SQLITE_IOERR_SHORT_READ`) and goes
/// through [`crate::db::classify_sqlite_error`]. So `StorageFull` stays `DiskFull` and
/// EVERY other kind is `StoreCorrupt` — not a list of decode kinds: `zcash_primitives`
/// decodes with `InvalidInput` too. Routing them to `from_io`'s `Io` said "retrying"
/// forever over a damaged row. **Re-check this claim whenever the `zcash_client_sqlite`
/// pin moves.** [`WalletError::from_io`] is unchanged: it is the door for REAL file I/O.
fn engine_io_fault(e: std::io::Error) -> WalletError {
    if e.kind() == std::io::ErrorKind::StorageFull {
        WalletError::DiskFull
    } else {
        WalletError::StoreCorrupt
    }
}

/// Map a NON-continuity scan failure to a typed [`WalletError`], honest about the
/// LAYER (no silent failure): `Wallet` is OUR DB — classified via [`ClassifyStoreFault`]
/// (`DiskFull`/`StoreBusy`/`Io` where honest, else fail-closed `StoreCorrupt` — #371;
/// and, since B1-5, `EndpointMisbehaving` for the one fault inside it that is NOT
/// ours: a served root the shard tree refuses against the leaves this scan counted);
/// `BlockSource` is the cache's own typed error, passed through (it already distinguishes
/// `DiskFull` from corruption); a non-continuity `Scan` is a malformed block from
/// the endpoint (`endpoint_unusable` — re-download/fallback). Continuity errors
/// never reach here — [`scan_batch`] classifies them as `Reorg` first.
fn map_scan_err<W: ClassifyStoreFault>(e: ChainScanError<W, WalletError>) -> WalletError {
    match e {
        ChainScanError::Wallet(w) => w.into_store_fault(),
        ChainScanError::BlockSource(cache_err) => cache_err,
        ChainScanError::Scan(_) => endpoint_unusable(),
        // 0.24.0 marked this enum `#[non_exhaustive]`. It still has exactly the three
        // variants above, so this arm is UNREACHABLE at the pinned version — it exists
        // so a future variant cannot silently take one of the three classifications
        // above.
        //
        // `Internal`, not `StoreCorrupt` and not `EndpointUnreachable`.
        //
        // NOT because it avoids the restore-from-seed remedy — an earlier version of
        // this comment claimed that and it is false: `StallReason::Internal`'s own
        // doc (`state.rs`) names "repair / restore-from-seed" as its honest next
        // step. The real reasons are two. First, a STALL is not a hard error: sync
        // stops and says so, the wallet stays open and readable, and the user is not
        // handed a modal telling them their funds need recovering — which is what
        // `StoreCorrupt` does on a condition we have not actually diagnosed. Second,
        // `EndpointUnreachable` would be an active lie, sending the user to change
        // servers for a fault that follows them. `Internal` says the true thing: sync
        // stopped, the cause is on this side, and it will not heal by retrying.
        _ => WalletError::Sync {
            stall: StallReason::Internal,
        },
    }
}

/// The height to rewind to on a reorg: `REWIND_DISTANCE_BLOCKS` below the block the
/// continuity error fired at (the reference `sync.rs:409` heuristic — rewind at
/// least one block past the error, deeper for safety). SATURATES at 0 near genesis
/// (a continuity error below the rewind distance rewinds to the start, never
/// underflows). Pure — boundary-tested without block fixtures.
fn rewind_target(at_height: BlockHeight) -> BlockHeight {
    at_height.saturating_sub(REWIND_DISTANCE_BLOCKS)
}

/// Fetch the per-batch anchor: the tree state at `height` (the block BELOW the
/// batch start) decoded to the `ChainState` `scan_cached_blocks` needs as
/// `from_state`. Async — holds NO db lock. A transport fault is typed `Sync` (stall
/// preserved); a tree state that fails to decode is a misbehaving server
/// (`endpoint_unusable`), NEVER a panic.
///
/// SCAN-2 (§4t): no longer every batch. `Wallet::sync_once` calls this on a
/// FETCHED batch — the first of a pass, one whose range does not start at the
/// previous batch's end, one after a rewind — and, every
/// `TREE_STATE_RECONCILE_BATCHES` consecutive batches, as the RECONCILE of the
/// anchor [`derive_chain_state`] produced from the previous batch's own blocks
/// (the two compared whole; a disagreement is the endpoint's). The shape is
/// untouched: one `GetTreeState`, one `to_chain_state`.
pub(crate) async fn fetch_chain_state<C: ScanClient + ?Sized>(
    client: &mut C,
    height: u64,
) -> Result<ChainState, WalletError> {
    let tree_state = client.tree_state(height).await.map_err(transport_err)?;
    tree_state.to_chain_state().map_err(|_| endpoint_unusable())
}

/// Download the HALF-OPEN block range `[start, end)` into the cache, in order (the
/// iv-a `BlockStream` → iv-b cache). Async — holds NO db lock.
///
/// COMPLETENESS (§3.2g, the money property): the stream must deliver EXACTLY the
/// contiguous heights `start..end`. A clean-but-SHORT `Ok(None)` (a truncated span),
/// a gap, a duplicate, an out-of-order block, or an overlong span is a typed `Sync`
/// — NEVER silently accepted. Without this a short stream would leave a hole the
/// scanner records as scanned = silently missed funds (`BlockStream` contract #1: a
/// clean end does NOT assert the full span).
///
/// SIZE-CAP BEFORE ALLOCATING (§4.6, principle 7): the count is bounded by the
/// range width, but each `CompactBlock` is only wire-capped at 8 MiB, so the
/// accumulated BYTES are hard-capped at [`DOWNLOAD_BATCH_MAX_BYTES`] — a server
/// padding block bodies to exhaust memory is a typed `Sync`, never an OOM (the byte
/// dimension the iv-d-1 count cap did not cover). The caller bounds the range width
/// to `≤ SYNC_BATCH_BLOCKS` and pre-checks cache headroom (iv-d-2b-ii).
///
/// COOPERATIVE CANCELLATION + PROGRESS (§3.2g iv-d-3-b): `cancel` is polled per
/// BLOCK so a `stop()` / watchdog cancel is honored PROMPTLY mid-download (not only
/// between batches) — returning [`DownloadOutcome::Cancelled`] with NOTHING written
/// to the cache (the in-flight buffer is dropped). `report` emits a
/// [`ScanProgress`] sample every [`PROGRESS_REPORT_BLOCKS`] blocks, re-arming the
/// d-3 stuck-sync watchdog so a slow-but-advancing download is never falsely
/// restarted, and driving the live `Scanning` position. The cancel flag, sink, and
/// the recorded tip each sample carries are all bundled in `ctx`.
#[cfg(test)]
pub(crate) async fn download_range<C: ScanClient + ?Sized>(
    client: &mut C,
    cache: &BlockCache,
    start: u64,
    end: u64,
    ctx: ScanCtx<'_>,
) -> Result<DownloadOutcome, WalletError> {
    download_range_capped(client, cache, start, end, DOWNLOAD_BATCH_MAX_BYTES, ctx).await
}

/// Is this (untrusted) `CompactBlock` safe to hand to the consumed-WHOLE
/// `scan_cached_blocks` (§4.6, principle 7 — every byte from the network is
/// hostile)? The scanner reads several fixed-width fields through upstream
/// accessors that **PANIC** (not error) on a wrong length, and a panic across the
/// FFI bridge is a CRASH — a malformed block from a buggy/malicious lightwalletd
/// would otherwise be a remote DoS. A block decodes as protobuf with ANY field
/// length, so the shape is validated HERE, at the download boundary, before it
/// ever reaches the cache or the scanner; a bad block makes the endpoint unusable
/// (typed `Sync`), never a panic.
///
/// The exact panic surface — pinned to `zcash_client_backend` 0.23.0 (proto.rs +
/// scanning/compact.rs); re-audit on bump. A field is a panic site when the scanner
/// reads it through an accessor whose RESULT it then `unwrap`/`expect`s, OR through
/// an accessor that itself `copy_from_slice`/`unwrap`s internally — a `Result`-
/// returning accessor is NOT safe if its caller unwraps it (the review's BLOCKER):
/// - `CompactBlock::height()` — `u64→u32` `unwrap` (proto.rs:94).
/// - `CompactBlock::hash()`/`prev_hash()` — `BlockHash::from_slice` `unwrap` when the
///   header is absent/unparseable AND the raw field isn't 32 bytes (proto.rs:65-85).
/// - `CompactTx::index` — `TxIndex::try_from(u64).expect()` ⇒ panics for `index ≥
///   2^16` (compact.rs:244; `TxIndex` is a `u16`).
/// - `CompactTx::txid()` — `copy_from_slice` into `[u8;32]` (proto.rs:113).
/// - `CompactSaplingSpend::nf` — `find_spent` does `spend.nf().expect()`
///   UNCONDITIONALLY for every spend, in the SECOND scan pass only (compact.rs:250),
///   so a bad-length sapling spend nullifier is NOT caught by the first pass; it
///   panics. (`sapling::Nullifier::from_slice` fails ONLY on length, so a 32-byte
///   check is exact.)
/// - `CompactSaplingOutput::cmu()` — `copy_from_slice` BEFORE its fallible
///   `from_bytes` (proto.rs:144).
///
/// ORCHARD action fields (`nullifier`/`cmx`/`ephemeral_key`/`ciphertext`) are
/// caught FALLIBLY in the FIRST pass: `add_block` parses each action via
/// `CompactAction::try_from(action)?` (compact.rs:160), so a malformed orchard action
/// rejects the block as a typed `Scan` error BEFORE the second pass's panicking
/// `find_spent` runs — they are NOT a live panic. We length-check them anyway
/// (cheap DEFENSE-IN-DEPTH: the "pass-1 catches orchard" ordering is a subtle
/// upstream property a 0.23.x bump could change; a 32-byte check never rejects a
/// legitimate action). The `fuzz_compact_block` target asserts GUARD SOUNDNESS over
/// arbitrary bytes (accepted ⇒ no panicking accessor crashes).
fn block_is_scannable(block: &CompactBlock) -> bool {
    // height must fit u32 (the scanner's `height()` unwraps).
    if u32::try_from(block.height).is_err() {
        return false;
    }
    // A present, parseable header makes hash()/prev_hash() read the parsed header
    // (safe); otherwise both raw 32-byte fields must be exactly 32 bytes.
    if block.header().is_none() && (block.hash.len() != 32 || block.prev_hash.len() != 32) {
        return false;
    }
    for tx in &block.vtx {
        // tx.index → TxIndex::try_from(u64).expect() (a u16); ≥ 2^16 PANICS.
        if u16::try_from(tx.index).is_err() {
            return false;
        }
        if tx.txid.len() != 32 {
            return false;
        }
        // sapling spend nullifiers — find_spent's `spend.nf().expect()` (2nd pass,
        // not caught earlier); from_slice fails only on length, so 32 bytes is exact.
        for spend in &tx.spends {
            if spend.nf.len() != 32 {
                return false;
            }
        }
        for out in &tx.outputs {
            if out.cmu.len() != 32 {
                return false;
            }
        }
        // orchard action nf/cmx — defense-in-depth (caught fallibly in pass 1 today).
        for action in &tx.actions {
            if action.nullifier.len() != 32 || action.cmx.len() != 32 {
                return false;
            }
        }
    }
    true
}

/// `download_range` with the byte ceiling injectable so the gate-7 boundary test
/// need not stream the 128 MiB production cap (mirrors `BlockCache::open_with_bound`).
#[cfg(test)]
async fn download_range_capped<C: ScanClient + ?Sized>(
    client: &mut C,
    cache: &BlockCache,
    start: u64,
    end: u64,
    max_bytes: u64,
    ctx: ScanCtx<'_>,
) -> Result<DownloadOutcome, WalletError> {
    download_range_folding(client, cache, start, end, max_bytes, ctx, None)
        .await
        .map(|(outcome, _)| outcome)
}

/// The anchor DERIVED for the batch after the one just downloaded (SCAN-2, §4t):
/// [`derive_chain_state`] over the downloaded blocks, and how long that fold
/// took. `Wallet::sync_once` carries `state` into its next iteration and uses it
/// — skipping the `GetTreeState` RPC — exactly when the next batch starts at
/// this one's `end` and this one advanced; `fold_ms` is moved out of `dl_ms`
/// and into `anchor_ms` there, so the download field never absorbs anchor work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DerivedAnchor {
    /// The `ChainState` at the batch's last block: its height, its own hash,
    /// the three frontiers with the batch's commitments appended.
    pub(crate) state: ChainState,
    /// The fold's own wall-clock, integer milliseconds ([`elapsed_ms`]).
    pub(crate) fold_ms: u64,
}

/// `download_range` that ALSO derives the next batch's anchor from the blocks
/// it downloaded (SCAN-2, §4t). `from` is THIS batch's anchor (the `ChainState`
/// at `start − 1`); the returned [`DerivedAnchor`] is the state at `end − 1`,
/// `Some` exactly when the download `Completed`. The fold runs at the download
/// boundary over the in-memory blocks — AFTER the span is proven complete and
/// BEFORE the cache insert (IT-1b 2: the blocks are in hand there and already
/// walked by `block_is_scannable`; a cache read before the delete would decode
/// them a third time). Its `Err` — a block whose `chain_metadata` sizes
/// disagree with its own commitments (P6), a malformed commitment — is the
/// endpoint's (`endpoint_unusable`) and nothing reaches the cache. The fold is
/// timed on its own so the loop can attribute it honestly.
pub(crate) async fn download_and_derive<C: ScanClient + ?Sized>(
    client: &mut C,
    cache: &BlockCache,
    start: u64,
    end: u64,
    ctx: ScanCtx<'_>,
    from: &ChainState,
) -> Result<(DownloadOutcome, Option<DerivedAnchor>), WalletError> {
    download_range_folding(
        client,
        cache,
        start,
        end,
        DOWNLOAD_BATCH_MAX_BYTES,
        ctx,
        Some(from),
    )
    .await
}

/// Derive the anchor for the batch AFTER `blocks` from the anchor BEFORE them
/// (SCAN-2, §4t — the seam the contract names). Returns the `ChainState` at the
/// last block: its height, its own `hash`, and each pool's frontier — `from`'s
/// frontier with every note commitment of `blocks` appended in block / tx /
/// output order, per pool, through the SAME leaf constructors the compact
/// scanner uses (`sapling::Node::from_cmu`; `MerkleHashOrchard::from_cmx` for
/// Orchard AND Ironwood — the Ironwood tree is Orchard-shaped and hashes the
/// same way; `zcash_client_backend-0.24.0/src/scanning/compact.rs:433`, `:471`,
/// `:509`) and `incrementalmerkletree`'s `Frontier::append`. No hash is written
/// here (principle 2): `append` combines ommers with each pool's own `Hashable`,
/// the code the shard tree runs anyway. P1: this is the frontier `put_blocks`
/// leaves in the shard tree after the batch; P2: upstream reads a `from_state`
/// for its height and its three frontiers only (`data_api/chain.rs:635`,
/// `data_api/ll/wallet.rs:326-339`, `:665-681`, `:712-730` — the hash is never
/// read), so a derived state equal to the served one is indistinguishable to it.
///
/// PURE and total — a slice in, a `ChainState` or a typed error out; no IO, no
/// panic. `Err` is the endpoint's class ([`endpoint_unusable`]) when:
/// - a block is not `from.block_height() + 1 + i` (the fold needs the
///   contiguous span; the download proved it, this is the seam's own check), or
///   its height does not fit a `u32`;
/// - a block's `hash` is not 32 bytes and it has no parseable header
///   (`CompactBlock::hash` would panic — the `block_is_scannable` shape);
/// - a commitment is not 32 bytes or not a canonical field element (the
///   scanner rejects the same output as `EncodingInvalid`);
/// - a pool's frontier is full (`append` → `false`: 2^32 leaves, unreachable on
///   any real chain — typed rather than assumed);
/// - P6: a block CARRIES `chain_metadata` and, after its commitments, a pool's
///   `tree_size()` is not that metadata's final size — the scanner's own
///   end-of-block check (`check_end_of_compact_block_consistency`), run here on
///   EVERY block at the download boundary so a block that would fail it never
///   reaches the cache. A block WITHOUT metadata is folded and not checked,
///   exactly as the scanner treats it (its sizes then come from the prior
///   block); refusing it here would stall a chain the scanner accepts.
///
/// NOT checked, deliberately: the hash chain (`prev_hash`). A batch that forks
/// from what the wallet holds is the scanner's continuity error and
/// `scan_batch`'s REORG; refusing it here would turn a rewind into a stall.
/// `blocks` empty → `from` itself (the identity fold).
///
/// **And NOT checked here either: `from` itself.** P6 holds every block of the
/// batch to the anchor it was folded from; it says nothing about whether that
/// anchor is true, because a consistently-carried lie satisfies it at every
/// block. Since BIND-1-R the anchor is compared, once per batch and per pool,
/// against `blocks.{prefix}_commitment_tree_size` at its own height — this
/// wallet's own earlier count of the same block — in
/// `root_bind::reconcile_scanned_boundaries`, which ABSTAINS for the batch when
/// the two disagree. That is a different question at a different seam and it is
/// named here so a reader of this doc does not take P6 for more than it is.
pub(crate) fn derive_chain_state(
    from: &ChainState,
    blocks: &[CompactBlock],
) -> Result<ChainState, WalletError> {
    let mut height = from.block_height();
    let mut hash = from.block_hash();
    let mut sapling = from.final_sapling_tree().clone();
    let mut orchard = from.final_orchard_tree().clone();
    let mut ironwood = from.final_ironwood_tree().clone();
    for block in blocks {
        if block.height != u64::from(height).saturating_add(1) {
            return Err(endpoint_unusable());
        }
        let block_height =
            BlockHeight::from_u32(u32::try_from(block.height).map_err(|_| endpoint_unusable())?);
        if block.header().is_none() && block.hash.len() != 32 {
            return Err(endpoint_unusable());
        }
        for tx in &block.vtx {
            for output in &tx.outputs {
                // `cmu()` copies 32 bytes before it parses (proto.rs:155); the
                // length check keeps this seam total.
                if output.cmu.len() != 32 {
                    return Err(endpoint_unusable());
                }
                let cmu = output.cmu().map_err(|_| endpoint_unusable())?;
                if !sapling.append(SaplingNode::from_cmu(&cmu)) {
                    return Err(endpoint_unusable());
                }
            }
            for action in &tx.actions {
                let cmx = action.cmx().map_err(|_| endpoint_unusable())?;
                if !orchard.append(MerkleHashOrchard::from_cmx(&cmx)) {
                    return Err(endpoint_unusable());
                }
            }
            for action in &tx.ironwood_actions {
                let cmx = action.cmx().map_err(|_| endpoint_unusable())?;
                if !ironwood.append(MerkleHashOrchard::from_cmx(&cmx)) {
                    return Err(endpoint_unusable());
                }
            }
        }
        if let Some(meta) = &block.chain_metadata
            && (sapling.tree_size() != u64::from(meta.sapling_commitment_tree_size)
                || orchard.tree_size() != u64::from(meta.orchard_commitment_tree_size)
                || ironwood.tree_size() != u64::from(meta.ironwood_commitment_tree_size))
        {
            return Err(endpoint_unusable());
        }
        height = block_height;
        hash = block.hash();
    }
    Ok(ChainState::new(height, hash, sapling, orchard, ironwood))
}

/// The one download body behind `download_range`, `download_range_capped`
/// and [`download_and_derive`]: the stream, the per-block guards, the byte cap,
/// the completeness check, the fold when `fold_from` is given, the cache insert.
async fn download_range_folding<C: ScanClient + ?Sized>(
    client: &mut C,
    cache: &BlockCache,
    start: u64,
    end: u64,
    max_bytes: u64,
    ctx: ScanCtx<'_>,
    fold_from: Option<&ChainState>,
) -> Result<(DownloadOutcome, Option<DerivedAnchor>), WalletError> {
    // Empty / degenerate range — nothing to download (a defensive no-op; the driver
    // passes a non-empty `[start, end)`). Also makes the `end - 1` below
    // underflow-proof in release, where a `debug_assert!` would be compiled out.
    if end <= start {
        return Ok((DownloadOutcome::Completed { outputs: 0 }, None));
    }
    // INCLUSIVE wire range [start, end-1] (the gRPC contract; the reference's `end - 1`).
    let mut stream = client
        .block_range(start, end - 1)
        .await
        .map_err(transport_err)?;
    let mut blocks = Vec::new();
    let mut next = start;
    let mut total_bytes: u64 = 0;
    // SCAN-1 (§4o S1): the batch's shielded-output count, tallied as each block
    // passes the boundary (one walk over `vtx`, beside the byte tally).
    let mut outputs: u64 = 0;
    while let Some(block) = stream.next_block().await.map_err(transport_err)? {
        // Cooperative cancellation, honored per BLOCK during the download (§3.2g
        // iv-d-3-b obligation 1 — a `stop()` / watchdog cancel is prompt mid-batch,
        // not parked until the batch boundary). The in-flight `blocks` buffer is
        // dropped UNWRITTEN — nothing reached the cache and the wallet-DB scan
        // frontier is untouched (the scan runs only after a FULL download), so the
        // only loss is re-downloadable work (obligation 3: never a torn cache or a
        // half-scanned batch). The next pass re-downloads this range cleanly.
        if ctx.cancel.is_cancelled() {
            return Ok((DownloadOutcome::Cancelled, None));
        }
        // Each block must be exactly the next expected height and never past end-1:
        // rejects gap / duplicate / out-of-order / overlong before it reaches the cache.
        if next >= end || block.height != next {
            return Err(endpoint_unusable());
        }
        // Hostile-shape reject BEFORE the cache: a malformed block would PANIC the
        // consumed-whole scanner (an FFI crash / remote DoS) — see `block_is_scannable`.
        if !block_is_scannable(&block) {
            return Err(endpoint_unusable());
        }
        outputs = outputs.saturating_add(shielded_outputs(&block));
        // Byte-cap the buffer against a server padding block bodies (DoS/OOM, §4.6).
        total_bytes = total_bytes.saturating_add(block.encoded_len() as u64);
        if total_bytes > max_bytes {
            return Err(endpoint_unusable());
        }
        next += 1;
        blocks.push(block);
        // Intra-batch progress (§3.2g iv-d-3-b obligation 2): every
        // PROGRESS_REPORT_BLOCKS blocks, re-arm the d-3 watchdog + feed the live
        // Scanning status. The frontier is the DOWNLOAD position (a per-range
        // liveness tick); the authoritative monotonic percent rides `ctx.summary`
        // (the prior committed batch's wallet summary, carried — no notes scanned
        // mid-download). `watch` coalescing bounds UI churn.
        if (next - start).is_multiple_of(u64::from(PROGRESS_REPORT_BLOCKS)) {
            (ctx.report)(ScanProgress::sample(next, ctx.tip, ctx.summary));
        }
    }
    // A clean end before the full span = a truncated download (the §3.2g property).
    if next != end {
        return Err(endpoint_unusable());
    }
    // SCAN-2 (§4t): the NEXT batch's anchor, folded from the blocks in hand —
    // after the span is proven complete (a fold over a short span would be a
    // state at the wrong height) and before the insert (a block the fold refuses
    // never reaches the cache, the same door `block_is_scannable` uses). Timed on
    // its own: the loop moves this out of `dl_ms` and into `anchor_ms`.
    let derived = match fold_from {
        Some(from) => {
            let t_fold = std::time::Instant::now();
            let state = derive_chain_state(from, &blocks)?;
            Some(DerivedAnchor {
                state,
                fold_ms: elapsed_ms(t_fold),
            })
        }
        None => None,
    };
    cache.insert(blocks).await?;
    Ok((DownloadOutcome::Completed { outputs }, derived))
}

/// One compact block's public shielded-output count (SCAN-1, §4o S1): Sapling
/// outputs + Orchard actions + Ironwood actions over its compact txs — every
/// field the compact scanner will trial-decrypt, INCLUDING `ironwood_actions`
/// (compact tag 9, the field [`block_is_scannable`] does not read — §4o owed row
/// 1 is about that field's length check, not this count). Chain data: a number
/// every observer of the block has; it says nothing about whose outputs they
/// are. Saturating, like the byte tally beside it.
fn shielded_outputs(block: &CompactBlock) -> u64 {
    block.vtx.iter().fold(0u64, |n, tx| {
        n.saturating_add(tx.outputs.len() as u64)
            .saturating_add(tx.actions.len() as u64)
            .saturating_add(tx.ironwood_actions.len() as u64)
    })
}

/// Milliseconds since `since`, as the integer the SCAN-1 span fields carry (§4o
/// S1). Saturates rather than wraps or panics: a duration past `u64::MAX`
/// milliseconds is not one this loop will see, and if it were, a pinned-high
/// field is honest where a wrapped one is a lie.
pub(crate) fn elapsed_ms(since: std::time::Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Truncate the wallet to `target`, or as close BELOW it as upstream permits.
///
/// **The security + crypto review's shared HIGH, closed here.** Upstream's
/// `truncate_to_height` picks `MAX(height) FROM blocks WHERE height <= target`
/// under the per-pool checkpoint tolerances (`zcash_client_sqlite-0.22.0
/// wallet.rs:4091-4122`) and, when NO row qualifies, returns
/// `RequestedRewindInvalid { safe_rewind_height, .. }` rather than truncating.
/// That is not a rare cell: `blocks` rows exist only where a block was SCANNED,
/// so the first anchor of any pass opening a region the wallet has not reached
/// sits below every row it holds. A restore's `Historic` range is exactly that
/// shape — it begins at the birthday while the only scanned rows are the
/// `ChainTip` window near the tip.
///
/// Two things went wrong there before this function existed, and both were
/// measured:
///
/// 1. **the undo did not happen** — the derived-anchor frontiers the reconcile
///    exists to remove survived the `Err`, in the deep-restore window where an
///    attacker would work; and
/// 2. **the endpoint escaped attribution** — `RequestedRewindInvalid` has no arm
///    in [`ClassifyStoreFault`], so it took the `_ => StoreCorrupt` default and
///    the user was sent to repair their wallet over a condition the SERVER
///    caused.
///
/// **`safe_rewind_height` is NOT the answer, and that was measured.** Upstream's
/// payload is `min_shared_checkpoint_height` (`wallet.rs:4015-4040`) — the lowest
/// CHECKPOINT — while `select_truncation_height` needs a `blocks` row. On the
/// fresh-restore cell the account's birthday import writes a checkpoint at the
/// anchor and no block, so upstream answers
/// `RequestedRewindInvalid { safe_rewind_height: Some(H), requested_height: H }`
/// — the same height it just refused. Retrying there is a no-op loop; the probe
/// that found this printed both values equal.
///
/// So the caller supplies a `fallback`: the anchor of the FIRST DERIVED batch of
/// the streak. That height is a `blocks` row **by construction** — the batch
/// below it scanned up to it — so upstream can always truncate there, and it
/// removes every batch that rode a derived anchor. What it leaves is the ONE
/// batch scanned on the independently FETCHED anchor, which is the smallest
/// residue this API can express and is inside the ≤ `TREE_STATE_RECONCILE_BATCHES`
/// bound §4t already accepted. Stated plainly, because it is a real residual: if
/// that batch's blocks were the tampered ones, its notes survive the undo, and
/// only a rescan removes them.
///
/// If both heights are refused, the caller is told the ENDPOINT is unusable —
/// never that the store is corrupt. The wallet is stalled and says so; nothing
/// is stamped.
///
/// **What this does NOT bound, stated rather than implied:** how far below
/// `target` the truncate may land. On a disjoint scanned set (a restore scans
/// the `ChainTip` window near the tip before the `Historic` range from the
/// birthday) the nearest qualifying row can sit far below, and everything above
/// it goes — including an already-scanned tip region. That is work, not money
/// (the next pass's `record_chain_tip` re-queues it and the balance recovers as
/// it re-scans), it is pre-existing in the reorg arm for the same reason, and
/// the depth is reported by the caller's `rewound_blocks`. Bounding it is an
/// owed decision, not a silent one.
fn truncate_at_or_below(
    db: &mut WalletConn,
    target: BlockHeight,
    fallback: Option<BlockHeight>,
) -> Result<BlockHeight, WalletError> {
    match db.truncate_to_height(target) {
        Ok(landed) => Ok(landed),
        Err(SqliteClientError::RequestedRewindInvalid { .. }) => {
            // §5.4: a static code naming the condition; no height, no path.
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = "rewind_target_unreachable",
                "wallet.reorg_rewind"
            );
            let Some(fallback) = fallback.filter(|f| *f >= target) else {
                // Nothing the wallet holds sits at or below the target, and the
                // caller named no reachable base. The caller must not read this
                // as corruption, and must not read it as success either.
                return Err(endpoint_unusable());
            };
            match db.truncate_to_height(fallback) {
                Ok(landed) => Ok(landed),
                // Still refused: the wallet holds nothing truncatable in the
                // streak's range at all. Honest and endpoint-attributed.
                Err(SqliteClientError::RequestedRewindInvalid { .. }) => Err(endpoint_unusable()),
                Err(e) => Err(e.into_store_fault()),
            }
        }
        Err(e) => Err(e.into_store_fault()),
    }
}

/// Where a rewind LANDED and what it cost, as [`rewind_wallet_to`] returns them.
pub(crate) struct RewindLanding {
    /// The height the truncate landed on — the highest CHECKPOINTED height at or
    /// below the target (§4q P-R2). The caller truncates the block cache to it.
    pub(crate) landed: BlockHeight,
    /// The blocks this rewind un-scanned (§4q-R P-RR4) — the DB's pre-rewind
    /// frontier minus `landed`, never a count multiplied by a constant.
    pub(crate) rewound_blocks: u64,
}

/// Rewind the wallet DB to `target`, recording the event in the A11 bind's ledger
/// first and withdrawing the record if the truncate fails.
///
/// The ONE body that runs `truncate_to_height` in this SDK. Two callers, and they
/// rewind for different reasons: [`scan_batch`]'s continuity arm (a chain reorg —
/// the endpoint's blocks contradict a stored one) and `sync_once`'s anchor
/// reconcile (§4t-run review row 2 — the endpoint's own tree state contradicts the
/// anchor its own blocks derived, so every batch scanned on the derived streak is
/// un-attested and must not stay committed). Both leave the wallet holding blocks
/// only up to the returned `landed`; the caller matches the block cache to it and
/// decides whether to re-queue.
///
/// **`aux` is the second SQLCipher connection to the SAME file, and it is REQUIRED**
/// (T0-1a-R): the rewind is the event that makes the A11 height bind's three memory
/// oracles stale, so it is recorded here.
pub(crate) fn rewind_wallet_to(
    db: &mut WalletConn,
    aux: &rusqlite::Connection,
    target: BlockHeight,
    fallback: Option<BlockHeight>,
) -> Result<RewindLanding, WalletError> {
    // §4q-R P-RR4: the DB's scanned frontier BEFORE the truncate — read here,
    // not derived from the batch (`from_height − 1` is the batch's anchor,
    // which sits below `block_max_scanned` whenever a `Verify` range is
    // scanned under the frontier). The `Rewind` arm sums
    // `pre_rewind_scanned − landed` into `SyncPass::rewound_blocks`, the
    // number `wallet.reorg_rewind` logs as `depth`: the blocks this rewind
    // actually un-scanned, never a constant product. Read BEFORE the ledger
    // note below, so a fault here leaves no armed-without-rewind cell; it is
    // the pre-scan frontier too — the scan that just failed committed
    // nothing (`scan_cached_blocks` is one transaction). `None` (no block
    // scanned) cannot reach a continuity error — upstream mismatches the
    // first block against a STORED one — and reads as zero blocks rewound.
    let pre_rewind_scanned = db
        .block_max_scanned()
        .map_err(ClassifyStoreFault::into_store_fault)?
        .map(|meta| u64::from(meta.block_height()));
    // TODO(iv-d-2b-ii/d-3): the first tracing site lands in d-3 (§3.2g) — log
    // the reorg `at_height`/`rewind` + the erased truncate error there (§5.4
    // heights decision pending) so an in-the-wild reorg is diagnosable.
    // T0-1a-R / T0-1a-R2 (§4g 0a): the ledger write and the truncate are on two
    // connections and are two transactions (`WalletDb` exposes no accessor for
    // its own), so they are ORDERED and COMPENSATED so that every state a kill
    // or a fault can leave is either true or the safer of the two false ones:
    //   · note, THEN truncate — a kill after the truncate leaves a TRUE ledger;
    //     a kill between them leaves "armed, no rewind": one relaxed pass on
    //     the next sync, consumed by that pass's write, after which this scan
    //     re-detects the same continuity error and rewinds for real. The other
    //     order would leave rewound-but-unrecorded, whose only exit is a rescan
    //     (pass-fatal for Sapling/Orchard until then).
    //   · truncate `Err` ⇒ UN-NOTE. Upstream's truncate is one transaction
    //     (`transactionally`), so `Err` means nothing was truncated. Before the
    //     compensation existed the ledger stayed armed — and was re-armed every
    //     pass — on a wallet that never rewound, endpoint-inducible through
    //     `RequestedRewindInvalid` (a continuity error within
    //     `REWIND_DISTANCE_BLOCKS` of the scan floor). §4f owed row 0a.
    //   · a fault in the un-note itself is logged and the truncate's own
    //     classified fault is returned (the original error wins over the
    //     compensation's); the ledger is then one ahead of the truth, the same
    //     bounded state as the kill-between-them cell.
    // The full matrix is the `root_bind` module doc's §4g table. The note comes
    // first for one more reason: if the ledger cannot be written, the rewind
    // does not happen and the pass fails recoverably.
    root_bind::note_rewind(aux)?;
    // The rewrite grows the WAL to log the deletion, so a tight disk can
    // `SQLITE_FULL` here (reorg + ENOSPC): classify it honestly (#371), never the
    // seed-restore scare on what is a "free space and retry" condition.
    // `RequestedRewindInvalid` is handled by [`truncate_at_or_below`] and never
    // reaches the classifier's `StoreCorrupt` default — see its doc.
    //
    // §4q P-R2: the truncate RETURNS the height it landed on — the highest
    // checkpointed height at or below `target` (`select_truncation_height`;
    // checkpoints sit at batch ends, so the two differ whenever no batch ended
    // at the target). That is the height the wallet now holds blocks up to, it
    // is what every reader downstream follows, and the target is discarded.
    let landed = match truncate_at_or_below(db, target, fallback) {
        Ok(landed) => landed,
        Err(e) => {
            let fault = e;
            if let Err(unnote) = root_bind::unnote_rewind(aux) {
                // §5.4: a static code on the allowlisted `outcome` field; no height,
                // no path. Not swallowed: this is the one state this arm can leave
                // that only a spent relaxed pass corrects, so it must be visible.
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = unnote.code(),
                    "a rewind was recorded, the truncate failed, and the record could \
                     not be withdrawn — the height bind will relax ONE pass it should \
                     not; the next accepted root write consumes it"
                );
            }
            return Err(fault);
        }
    };
    // §4q-R P-RR4: the blocks this rewind un-scanned. Saturating: the
    // frontier is never below the landed height (a `blocks` row at `landed`
    // is what `select_truncation_height` chose), so a zero from the
    // saturation would itself be that invariant breaking — never a panic.
    let landed_height = u64::from(landed);
    let rewound_blocks = pre_rewind_scanned
        .unwrap_or(landed_height)
        .saturating_sub(landed_height);
    Ok(RewindLanding {
        landed,
        rewound_blocks,
    })
}

/// The FIELD on which two [`ChainState`]s disagree, as a static code for the
/// `wallet.anchor_reconcile` warn (§4t-run review row 3).
///
/// The comparison itself stays WHOLE — `ChainState: PartialEq` is deep over height,
/// hash and all three frontiers, and dropping the hash would drop the only check on
/// a field upstream never reads. What the review asked for is the ability to tell a
/// CONVENTION bug from a tamper from a field report: a server whose `GetTreeState`
/// hash byte-order differs from its `GetBlockRange` hash is `"hash"` on every
/// mismatch forever, while a tamper moves a frontier. §5.4: a static code on the
/// allowlisted `outcome`-class field — never a height, a hash or a tree size.
pub(crate) fn chain_state_mismatch_field(
    derived: &ChainState,
    served: &ChainState,
) -> &'static str {
    if derived.block_height() != served.block_height() {
        "height"
    } else if derived.block_hash() != served.block_hash() {
        "hash"
    } else if derived.final_sapling_tree() != served.final_sapling_tree() {
        "sapling"
    } else if derived.final_orchard_tree() != served.final_orchard_tree() {
        "orchard"
    } else if derived.final_ironwood_tree() != served.final_ironwood_tree() {
        "ironwood"
    } else {
        // Unreachable while `ChainState: PartialEq` is the deep derive over exactly
        // these five fields — and a code, not a panic, if upstream ever adds a sixth.
        "unknown"
    }
}

/// Is this reconcile mismatch the BENIGN one — the hash and ONLY the hash
/// differing, by exactly the byte-order convention? (§3a option (c), maintainer
/// 2026-09-11.)
///
/// **Why the field gets a verdict of its own.** The block hash in the SERVED
/// state comes from `GetTreeState`, and `TreeState::to_chain_state` reverses the
/// hex UNCONDITIONALLY — *"Zcashd hex strings for block hashes are
/// byte-reversed"*, `zcash_client_backend-0.24.0/src/proto.rs:456-457`. The
/// DERIVED one comes from `CompactBlock::hash()`, which does not reverse.
/// Nothing pins that a server's two RPCs agree on that convention, because
/// nothing downstream consults the field: the production scan path reads
/// `from_state` for `block_height()` and the three frontiers only. So a server
/// can be entirely honest and still fail a WHOLE `ChainState` comparison
/// forever, and before this split that cost the user a scary badge plus, after
/// SCAN-2's undo, no net progress at all.
///
/// **What is NOT excused.** A hash that merely DIFFERS is two different blocks at
/// one height — the endpoint's two RPCs describing two different chains — which
/// is a real signal and stays the endpoint's fault. Only the verified reversal
/// is benign, which costs 32 byte comparisons.
///
/// **Why this is written as a RECONSTRUCTION rather than a five-field check.**
/// Rebuilding `served` with its hash reversed and comparing under the deep
/// `PartialEq` makes the excuse fail CLOSED in both directions a hand-rolled
/// conjunction over the five fields we know about would not. A sixth member added
/// as a `ChainState::new` parameter is a COMPILE error here; one added OUTSIDE
/// `new` keeps compiling, but then the reconstruction carries that member's
/// DEFAULT while `derived` carries the served value, so `*derived == candidate`
/// goes false and the excuse is withheld. Either way the new field is never
/// silently excused — which is the guarantee, stated no larger than it is (the
/// crypto audit caught the first draft claiming the compile error covered
/// both cases).
pub(crate) fn is_hash_only_byte_reversal(derived: &ChainState, served: &ChainState) -> bool {
    // Equal hashes are not a hash mismatch at all (and a palindromic hash is its
    // own reversal, so this guard is what keeps that case out of the benign arm).
    if derived.block_hash() == served.block_hash() {
        return false;
    }
    let mut reversed = served.block_hash().0;
    reversed.reverse();
    let candidate = ChainState::new(
        served.block_height(),
        BlockHash(reversed),
        served.final_sapling_tree().clone(),
        served.final_orchard_tree().clone(),
        served.final_ironwood_tree().clone(),
    );
    *derived == candidate
}

/// Scan ONE already-cached batch `[from_height, from_height + limit)` against
/// `from_state` (the chain state at `from_height − 1`). SYNC blocking work — the
/// caller runs it inside `spawn_blocking` under the db lock (the iv-d-2a split keeps
/// this off the UI-read fields; one batch is `≤ SYNC_BATCH_BLOCKS`, so the hold is
/// bounded). Returns [`ScanOutcome`].
///
/// **`ScanOutcome::Scanned::received_notes` counts THREE pools, and Ironwood is
/// counted separately because upstream does not count it** (T0-3, §4z).
/// `ScanSummary` carries `received_sapling_note_count` and
/// `received_orchard_note_count` and nothing else — its accumulation loop
/// (`zcash_client_backend-0.24.0 data_api/chain.rs:679-687`) never reads
/// `wtx.ironwood_outputs()` — so an Ironwood-only batch reported zero, ADR-0536's
/// incoming-funds gate stayed shut, and no arrival event ever fired. The
/// Ironwood share is read from the wallet DB as a delta across the scan
/// ([`crate::history::ironwood_received_high_water`], which says why it is
/// cheap). Like the other two counts it OVER-approximates — change and
/// self-sends count too; the Advance arm's `detect_incoming` truth diff is the
/// filter, and this number only decides whether that diff runs at all.
///
/// GUARDS the upstream `assert_eq!(from_height == from_state.block_height + 1)`
/// (chain.rs:600, ALWAYS compiled) BEFORE the call — a mismatch (the server
/// anchored its tree state at the wrong height) is a typed `Sync`, NEVER the
/// FFI-crashing panic. On a continuity error (reorg) it rewinds the wallet DB to
/// `at_height − REWIND_DISTANCE_BLOCKS` (the reference `sync.rs:409` heuristic)
/// under the SAME lock, RE-QUEUES the rewound span up to `recorded_tip` under that
/// same lock when `recorded_tip` is above the height the truncate landed on (§4q
/// P-R1 — [`record_chain_tip`], the call the pass head makes, made now: upstream's
/// truncate only TRIMS the scan queue, and a pass that read the trimmed queue
/// ended "up to date" at the pre-rewind tip and stamped it, INC-025; §4q-R P-RR1
/// — with the recorded tip at or below the landed height the re-queue is skipped,
/// the outcome says so, and the pass ends as an under-claim), and returns
/// `Reorg { rewind_height, requeued, rewound_blocks }`: the height the truncate
/// LANDED on (§4q P-R2) so the caller truncates the cache to match, whether the
/// re-queue was made, and the blocks the rewind un-scanned (§4q-R P-RR4).
///
/// `recorded_tip` is the pass-head tip `sync_once_body` recorded — threaded in by
/// name, never re-read from the endpoint inside the lock. The re-queue can fail
/// after the truncate committed; that is a store fault classified like the
/// truncate's own ([`ClassifyStoreFault`]) and the rewind note STANDS, because
/// the wallet did rewind (the arm says why). On that `Err` the block cache is NOT
/// truncated — the named exception of §4q-R P-RR6; the arm says why it is safe.
///
/// **`aux` is the second SQLCipher connection to the SAME file, and it is REQUIRED**
/// (T0-1a-R). Three things need it now, and the doc used to name only the first:
///
/// 1. **The rewind arm.** A rewind is the event that makes the A11 height bind's
///    memory oracles stale, so this is where the wallet records that it happened
///    ([`root_bind::note_rewind`]) — and withdraws the record when the truncate
///    returns `Err` ([`root_bind::unnote_rewind`], T0-1a-R2), so the ledger says what
///    HAPPENED and not what was attempted; a `scan_batch` that could rewind without
///    recording it would strand an honest endpoint behind a stale shard row with no
///    exit but a rescan.
/// 2. **The `Ok` arm, on EVERY batch** (BIND-1, §4x): [`reconcile_scan_boundaries`]
///    reads the per-block tree sizes the batch just committed and holds the RECORDED
///    completion heights to them. This is the ordinary path, not the exceptional one
///    — the sentence that stood here described the rewind arm as if it were the only
///    reason the connection is threaded, which has not been true since BIND-1.
/// 3. **The boundary-bracket ledger** (BIND-1-R): what that reconcile REFUTES is
///    written here and read back at the next pass's ingest bind
///    ([`root_bind::BOUNDARY_BOUND_TABLE`]).
///
/// Passed rather than made optional on purpose — an `Option<&Connection>` here is the
/// fail-open seam `docs/REVIEW.md` §3 exists to catch: every fake would pass `None`
/// and the ledger would only ever be written in production.
//
// Eight parameters since §4q threaded `recorded_tip` in by name. Bundling them into a
// struct purely to satisfy the lint would hide which value the reorg arm re-queues to
// (the `send::prepare_queued` precedent).
#[allow(clippy::too_many_arguments)]
pub(crate) fn scan_batch(
    network: Network,
    cache: &BlockCache,
    db: &mut WalletConn,
    aux: &rusqlite::Connection,
    from_height: u64,
    from_state: &ChainState,
    limit: usize,
    recorded_tip: BlockHeight,
) -> Result<ScanOutcome, WalletError> {
    // GUARD the upstream assert: from_height must be exactly one above the anchor.
    if from_height != u64::from(from_state.block_height()) + 1 {
        return Err(endpoint_unusable());
    }
    let from_bh =
        BlockHeight::from_u32(u32::try_from(from_height).map_err(|_| endpoint_unusable())?);
    let consensus = network.consensus();
    // T0-3 (§4z): the Ironwood receipt count, which upstream's `ScanSummary`
    // does not carry. Read BEFORE the scan so the delta below is this batch's
    // alone; two O(1) rowid seeks (`history::ironwood_received_high_water` says
    // why that is what keeps ADR-0536's gate cheap). A fault here is a store
    // fault like any other read's and fails the batch recoverably — it is not
    // swallowed, because a silently-zero Ironwood count is the defect this
    // whole item exists to close.
    let ironwood_before = crate::history::ironwood_received_high_water(aux)?;
    match scan_cached_blocks(&consensus, cache, db, from_bh, from_state, limit) {
        Ok(summary) => {
            // Sapling + Orchard from the summary, Ironwood from the wallet DB.
            // `saturating_sub` on a value that only grows across a SUCCEEDED
            // scan: the reorg arm below computes no delta, so a rewind's
            // deletions can never be read as a negative receipt count.
            let ironwood =
                crate::history::ironwood_received_high_water(aux)?.saturating_sub(ironwood_before);
            // BIND-1 (§4x), the seam §4w (d) names: the batch is COMMITTED, so the
            // per-block commitment tree sizes it just wrote are readable and the
            // `witness_stabilized` latch it may have set inside that same
            // transaction is already in place. This is where the wallet's own count
            // becomes an oracle over the completion heights it was told — the one
            // that reaches above the newest bundled row. A refusal on a required
            // pool fails the batch from inside here (`Err`), with the row already
            // corrected; Ironwood's rides out in the outcome.
            let boundary =
                reconcile_scan_boundaries(aux, network, from_state, summary.scanned_range())?;
            Ok(ScanOutcome::Scanned {
                received_notes: summary.received_sapling_note_count()
                    + summary.received_orchard_note_count()
                    + usize::try_from(ironwood).unwrap_or(usize::MAX),
                boundary,
            })
        }
        Err(ChainScanError::Scan(e)) if e.is_continuity_error() => {
            let rewind = rewind_target(e.at_height());
            let RewindLanding {
                landed,
                rewound_blocks,
            } = rewind_wallet_to(db, aux, rewind, None)?;
            let landed_height = u64::from(landed);
            // §4q P-R1 (INC-025): RE-QUEUE the rewound span before this lock is released.
            // Upstream's truncate only TRIMS the scan queue to `landed`
            // (`truncate_to_height_internal` → `trim_scan_queue_to`); the `Historic`
            // re-queue belongs to `rewind_to_chain_state`, which this arm does not call.
            // Without this write the driver's next `next_batch` read found an EMPTY
            // queue, the pass returned `Ok` "up to date" carrying the pre-rewind tip,
            // and the controller published `UpToDate { tip }` and STAMPED it while the
            // wallet held blocks only up to `landed` — until the next pass's
            // `record_chain_tip` made exactly this call. So it is made here, now — the
            // ONE function ([`record_chain_tip`], §4q-R row 7: one body, never a copy)
            // with the same pass-head tip.
            //
            // §4q-R P-RR1 (the REQ-1 fold review, row 1) — the PRECONDITION: upstream
            // reads `max_scanned` from `blocks` (just lowered to `landed`) and
            // early-returns on `new_tip < max_scanned`, so the call queues
            // `landed + 1 .. tip + 1` (`Verify`/`ChainTip`) exactly when
            // `recorded_tip > landed`. That is the ordinary fork: the endpoint's tip is
            // above the height the wallet fell back to, the re-scan across the fork is
            // THIS pass's, and the pass ends "up to date" only once it has scanned to
            // the tip it recorded. When `recorded_tip <= landed` the wallet's own held
            // height is at or above the endpoint's claimed tip — the endpoint is behind
            // our scan (a peer switch, a hostile server), or a range a cancelled pass
            // left queued above the tip forked — and the call would do nothing: it is
            // SKIPPED, the outcome says so (`requeued: false`), the queue stays trimmed,
            // and the loop's exit reads the DB at or above the tip and ends the pass
            // `Ok` as the under-claim it is (P-RR2). Before P-RR1 that cell reached the
            // belt and stalled a wallet that had lost no blocks.
            //
            // §4q-R P-RR5 — the INC-024 invariant, stated where the call is made:
            // `landed` is a `blocks` row and every `blocks` row is ≥ the wallet
            // birthday, so a tip below the birthday makes `landed > tip` and upstream's
            // early return fires — the INC-024 assert is unreachable from here for that
            // reason and no other. (`birthday_beyond_tip` abstains once the wallet has
            // scanned past the tip, and this call runs after the frontier was lowered;
            // the invariant above, not the guard, is what keeps it below the assert —
            // and the `<=` test above never lets the call run in that cell anyway.)
            //
            // A fault here is a store fault like the truncate's own (`ClassifyStoreFault`
            // — a `SQLITE_FULL` on the queue rewrite is the same `DiskFull` it would be
            // one write earlier) and the pass fails recoverably: no stamp, and the next
            // pass's `record_chain_tip` re-queues. The rewind note is NOT withdrawn: the
            // truncate COMMITTED (its own `transactionally`), so the wallet DID rewind
            // and the ledger at `N+1` is TRUE — the `root_bind` matrix's "kill AFTER the
            // truncate" cell. Un-noting here would manufacture rewound-but-unrecorded,
            // the state the note-before-truncate ordering above exists to avoid.
            //
            // §4q-R P-RR6 — the block cache on that `Err`, the NAMED exception: the DB
            // is truncated to `landed` and the cache is not (its truncate is the
            // `Rewind` arm's, async, and this `Err` returns before that arm). Safe
            // because the cache is disposable and the scanner is never fed a row a
            // batch did not just download: every batch downloads its whole range
            // before scanning it (`BlockCache::insert` is `ON CONFLICT(height) DO
            // UPDATE`, so a stale row inside the range is overwritten) and
            // `with_blocks` delivers only that range, so a stale row above the DB's
            // frontier is never scanned; the pass fails with no stamp, and the next
            // pass's downloads replace the rows it needs. The `Rewind` arm's doc names
            // the same exception from its side.
            let requeued = if recorded_tip <= landed {
                false
            } else {
                record_chain_tip(db, recorded_tip)?;
                true
            };
            Ok(ScanOutcome::Reorg {
                rewind_height: landed_height,
                requeued,
                rewound_blocks,
            })
        }
        Err(e) => Err(map_scan_err(e)),
    }
}

// ── The driver loop seam (§3.2g inc-2c-iv-d-2b-ii) ───────────────────────────
//
// `Wallet::sync_once` (wallet.rs) composes the 2b-i per-batch primitives into OUR
// scan loop, mirroring the reference `sync::run` shape (`update_subtree_roots →
// while running { update_chain_tip → suggest → batch → download → scan →
// delete/truncate → re-suggest }`) flattened to a single re-suggesting loop. The
// tip is READ before the roots (T0-1 A11 needs it as the ceiling on every
// endpoint-supplied `completing_block_height`); the WRITE order above is unchanged.
// The highest-priority range — the Verify range, when present — is always the first
// suggested range, so taking the first non-empty range each iteration honors
// Spend-before-Sync (§1.7) AND folds reorg-verification in without a separate
// phase. These are the pieces the loop sequences; the loop itself lives on the
// async `Wallet` handle because it owns the iv-d-2a db lock (taken FRESH per batch,
// RELEASED across the lock-free async fetch/download between batches).

/// Cooperative cancellation for the scan loop (§7 — the app backgrounds mid-sync).
/// The controller (d-3) flips this; the scan loop honors it at TWO granularities
/// (§3.2g iv-d-3-b obligation 1): [`crate::wallet::Wallet::sync_once`] checks it
/// between batches, and `download_range` polls it per BLOCK so a stop is prompt
/// even mid-download (not parked until the batch boundary — the mobile-backgrounding
/// case). Scan progress is durable in the wallet DB (the cache is disposable), so a
/// cancel loses at most the in-flight, re-downloadable batch — never a torn scan
/// (the scan commits atomically under the db lock or not at all). The only span
/// that cannot be interrupted is ONE `scan_cached_blocks` call — CPU-bound, no
/// network wait, bounded by [`SYNC_BATCH_BLOCKS`] — so worst-case cancel latency is
/// one batch SCAN, never a batch's whole download. `Clone` shares the flag (the
/// controller keeps one, the loop borrows one).
#[derive(Clone, Default)]
pub(crate) struct CancelToken {
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Wakes any task awaiting [`CancelToken::cancelled`] the instant `cancel`
    /// fires — so an engine can interrupt a long wait PROMPTLY instead of only
    /// polling `is_cancelled` between batches. The polling `sync_once` loop is
    /// unchanged (it still checks `is_cancelled` between batches); the d-3
    /// controller's stuck-sync watchdog + a deterministic test engine use the
    /// async wait.
    notify: std::sync::Arc<tokio::sync::Notify>,
}

impl CancelToken {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    /// Request cancellation (idempotent). Honored before the NEXT batch by the
    /// polling `sync_once` loop, and IMMEDIATELY by any `cancelled().await`.
    pub(crate) fn cancel(&self) {
        self.flag.store(true, std::sync::atomic::Ordering::Relaxed);
        self.notify.notify_waiters();
    }
    pub(crate) fn is_cancelled(&self) -> bool {
        self.flag.load(std::sync::atomic::Ordering::Relaxed)
    }
    /// Resolve once cancelled. Idempotent + safe for many concurrent waiters. The
    /// arm-then-recheck closes the lost-wakeup race: `notify_waiters` only wakes
    /// ALREADY-registered waiters, so we register the waiter (`enable()`) BEFORE
    /// the final flag re-check — a `cancel` racing in between is caught by the
    /// re-check, and one arriving after is caught by the wakeup.
    pub(crate) async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

/// The result of one `sync_once` pass — the observable progress (d-3's status
/// surface + the deterministic tests). Pure SDK data, no librustzcash leak.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct SyncPass {
    /// Batches scanned this pass (each ≤ [`SYNC_BATCH_BLOCKS`]). 0 ⇒ already up to
    /// date with the recorded tip.
    pub(crate) batches: u32,
    /// Reorgs (continuity-error rewinds) handled this pass.
    pub(crate) reorgs: u32,
    /// Blocks the pass's rewinds actually un-scanned: Σ over its rewinds of the
    /// DB's `block_max_scanned` before the truncate minus the height it landed on
    /// (§4q-R P-RR4, the REQ-1 fold review's row 5). `sync_controller::emit_synced`
    /// logs it as `wallet.reorg_rewind`'s `depth` — the measured number, where the
    /// product `reorgs × REWIND_DISTANCE_BLOCKS` once stood while that fold measured
    /// one rewind landing 92 below a 10-block target. A count of blocks (§5.4),
    /// never a height; zero on a default-constructed pass and on one with no rewind.
    pub(crate) rewound_blocks: u64,
    /// `true` ⇒ the pass returned early on cooperative cancellation (progress is
    /// durable; the next pass resumes). NOT an error.
    pub(crate) cancelled: bool,
    /// SCAN-1 (§4o S3): the pass's phase sums — each the sum of the same-named
    /// field over this pass's `wallet.sync` batch spans (`Wallet::sync_once`),
    /// integer milliseconds. `sync_controller::emit_synced` logs them beside
    /// `batches`/`reorgs`, so ONE line per pass says where the pass went. A
    /// default-constructed pass (the fakes) carries zeros — "not measured",
    /// which no reader turns into "instant". Durations, never heights: nothing
    /// here narrows the wallet beyond what the span's `from`/`to` carry.
    pub(crate) anchor_ms: u64,
    /// The `GetBlockRange` stream + cache insert, summed over the batches.
    pub(crate) dl_ms: u64,
    /// The locked `scan_cached_blocks` (decrypt + shardtree + the one upstream
    /// commit), summed over the batches.
    pub(crate) scan_ms: u64,
    /// `progress_snapshot` on the batches that refreshed the authoritative
    /// summary (every `SYNC_SUMMARY_REFRESH_BATCHES`th, the first, a reorg's
    /// forced one). The tail refresh at up-to-date is NOT a batch and is not
    /// in here — it is in `wall_ms`.
    pub(crate) snap_ms: u64,
    /// S15-F1 phase A: the pass's `fetch_tip` (one `GetLatestBlock` round trip
    /// plus its checks). Zero when the phase did not run to completion (a
    /// cancelled or default-constructed pass) or on an undelayed fake.
    pub(crate) tip_ms: u64,
    /// S15-F1 phase A: the pass's whole `update_subtree_roots` — every pool's
    /// root stream, the bind and the locked write. Zero as for `tip_ms`.
    pub(crate) roots_ms: u64,
    /// Wall-clock of the WHOLE pass, entry to return of `sync_once` — the tip
    /// fetch, the root ingestion, the between-batch `next_batch` reads, the
    /// tail summary refresh and every batch. `tip_ms + roots_ms + Σ batch wall
    /// ≤ wall_ms`; the rest is the time no phase sees (§4o Q-S2).
    pub(crate) wall_ms: u64,
    /// Shielded outputs downloaded this pass — the batch spans' `chain_outputs`
    /// summed (chain data; [`DownloadOutcome::Completed`]). `chain_` so the bare
    /// word is never confused with a transaction's outputs (`tracing_guard`).
    pub(crate) chain_outputs: u64,
    /// The chain tip this pass recorded (from `fetch_tip`), once it got past the
    /// tip fetch — the engine caches it for the controller's terminal
    /// `UpToDate { tip }` (so a no-op pass that scans nothing still reports the tip).
    /// `None` only on a default-constructed pass (the fake-engine tests) or one that
    /// errored before the tip fetch. On a pass [`birthday_beyond_tip`] returned
    /// early (T0-1c-R3) it is the tip the pass REPORTED and did not record — the
    /// height `EndpointBehind { tip, .. }` names as the server's; nothing durable
    /// carries it.
    pub(crate) tip: Option<crate::money::BlockHeight>,
    /// What the endpoint did for each pool's subtree roots on this pass, in
    /// [`SUBTREE_ROOT_POOLS`] order. Carried OUT of the pass rather than only
    /// logged, so the status surface can tell "this endpoint serves us no roots
    /// for that pool" ([`PoolFetch::Unsupported`]), "this endpoint lied about that
    /// pool" ([`PoolFetch::HeightViolation`]) and "this endpoint served zero
    /// roots" ([`PoolFetch::Served`] with `roots == 0`) apart from a healthy
    /// ingest — states that are otherwise indistinguishable after the fact, and
    /// whose conflation is exactly what let Ironwood money sit Pending and never
    /// clear. READ by `sync_controller::emit_synced` on every clean pass (T0-1b):
    /// that is the one renderer, and it is where a refused pool becomes
    /// `SyncStatus::UpToDateDegraded`. `None` on a pass that errored before (or
    /// never reached) the root ingestion, and on a default-constructed pass (the
    /// fake engines) — which the renderer reads as "no claim made", never as
    /// "every pool served".
    pub(crate) root_outcomes: Option<[PoolFetch; SUBTREE_ROOT_POOLS.len()]>,
    /// Where the endpoint's reported tip stands against the newest row the
    /// signed bundle carries for the wallet's network (T0-1c, [`tip_standing`]).
    /// The second carrier beside `root_outcomes`, for the same reason that one
    /// exists (§4k-R decision 1): no port method, so no fake and no future engine
    /// can default its way to "at or above". `Some` once [`fetch_tip`] ran —
    /// `Wallet::sync_once` fills it in the same struct literal as `root_outcomes`
    /// — and READ by `sync_controller::emit_synced`, where `BehindBundle` becomes
    /// `SyncStatus::EndpointBehind`. `None` on a pass that errored before the tip
    /// fetch and on a default-constructed pass (the fake engines): "no claim
    /// made", never "at or above".
    pub(crate) tip_standing: Option<TipStanding>,
}

/// The pass's pre-batch phase timings, SHARED between the pass body and its owner
/// (S15-F1, phase A's owed LOW-3). The engine's `run_pass` races the whole pass
/// against cancellation and, on cancel, DROPS the pass future and builds a
/// `SyncPass` of its own — so a timing held only in the body's locals died with it,
/// and a pass cancelled mid-scan reported `tip_ms = roots_ms = 0` after spending
/// most of its time in exactly those phases. The body writes each field as its
/// phase ends; the cancelled arm reads them. Zero means the phase did not finish.
#[derive(Debug, Default)]
pub(crate) struct PassPhases {
    pub(crate) tip_ms: std::sync::atomic::AtomicU64,
    pub(crate) roots_ms: std::sync::atomic::AtomicU64,
}

impl PassPhases {
    /// Record the tip phase's duration.
    pub(crate) fn set_tip_ms(&self, ms: u64) {
        self.tip_ms.store(ms, std::sync::atomic::Ordering::Relaxed);
    }
    /// Record the roots phase's duration.
    pub(crate) fn set_roots_ms(&self, ms: u64) {
        self.roots_ms
            .store(ms, std::sync::atomic::Ordering::Relaxed);
    }
    /// `(tip_ms, roots_ms)` as recorded so far.
    pub(crate) fn read(&self) -> (u64, u64) {
        (
            self.tip_ms.load(std::sync::atomic::Ordering::Relaxed),
            self.roots_ms.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

/// One batch's phase timings + chain count, as `Wallet::sync_once` measured them
/// (SCAN-1). The pass sums are accumulated from this in ONE place,
/// [`SyncPass::absorb`], so the accumulator is a pure, unit-tested step rather
/// than five `+=` lines inside the loop (review, item 3: the in-process
/// S1 identity is vacuous for the network phases and the chain count, which the
/// fake chain cannot make non-zero — the pure test is where those are guarded).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct BatchPhases {
    pub(crate) anchor_ms: u64,
    pub(crate) dl_ms: u64,
    pub(crate) scan_ms: u64,
    /// `Some` on a batch that attempted an authoritative refresh (the sum, if
    /// it attempted twice — the refresh-then-reorg shape); `None` otherwise.
    pub(crate) snap_ms: Option<u64>,
    pub(crate) chain_outputs: u64,
}

impl SyncPass {
    /// Add one batch's phases to the pass sums (saturating). `wall_ms` is the
    /// pass's own, never a batch's, and is untouched here.
    pub(crate) fn absorb(&mut self, batch: &BatchPhases) {
        self.anchor_ms = self.anchor_ms.saturating_add(batch.anchor_ms);
        self.dl_ms = self.dl_ms.saturating_add(batch.dl_ms);
        self.scan_ms = self.scan_ms.saturating_add(batch.scan_ms);
        self.snap_ms = self.snap_ms.saturating_add(batch.snap_ms.unwrap_or(0));
        self.chain_outputs = self.chain_outputs.saturating_add(batch.chain_outputs);
    }
}

/// Where an endpoint's reported tip stands against the newest row the signed
/// bundle carries for the wallet's network — the T0-1c grade, computed once per
/// pass by [`tip_standing`] inside [`fetch_tip`] and carried out in
/// [`SyncPass::tip_standing`].
///
/// The variant names predate T0-1c-R2 and are kept (the contract names them as
/// live identifiers): the "Bundle" in them is the grade's FLOOR, not its whole
/// reference. Since T0-1c-R2 the reference is the LARGER of the bundle's newest
/// row and the wallet's own scanned height less the reorg margin — see
/// [`tip_standing`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TipStanding {
    /// At or above the reference. Nothing here disputes the tip; above the
    /// reference it is a number the endpoint alone asserts (the residual,
    /// [`tip_standing`]).
    AtOrAboveBundle,
    /// Below the reference: the endpoint is behind data this wallet already
    /// holds — the signed bundle's newest row, or its own scanned blocks —
    /// whatever else it says. `newest_known` is the reference the grade fired
    /// against, carried so the surface can say by at least how much.
    BehindBundle { newest_known: u32 },
}

/// What [`fetch_tip`] hands back: the range-checked tip, and its standing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FetchedTip {
    pub(crate) tip: BlockHeight,
    pub(crate) standing: TipStanding,
}

/// Fetch + range-check the chain tip from the (untrusted) endpoint (async — holds
/// NO db lock). A height beyond `u32` is a garbage/hostile response →
/// `endpoint_unusable`, never the `BlockHeight::from_u32` panic.
///
/// **T0-1c — the tip is also GRADED here, and this is the one place** (§4k
/// decision 1): against the bundle's newest row for `network` AND the wallet's
/// own scanned height (`scanned_tip`, T0-1c-R2 — `None` for a wallet that has
/// scanned nothing), by [`tip_standing`], before [`record_chain_tip`], before
/// the roots, before any RPC that depends on the number — so every caller holds
/// the standing beside the tip. A tip below the reference is RETURNED, not
/// refused (§4k-R: REPORT AND CONTINUE); the standing rides
/// [`SyncPass::tip_standing`] to the surface.
pub(crate) async fn fetch_tip<C: ScanClient + ?Sized>(
    client: &mut C,
    network: Network,
    scanned_tip: Option<u32>,
) -> Result<FetchedTip, WalletError> {
    let height = client.latest_block_height().await.map_err(transport_err)?;
    let h = u32::try_from(height).map_err(|_| endpoint_unusable())?;
    Ok(FetchedTip {
        tip: BlockHeight::from_u32(h),
        standing: tip_standing(network, h, scanned_tip),
    })
}

/// The grade itself, pure so it can be driven directly. The REFERENCE is the
/// larger of two heights the wallet holds without trusting any endpoint
/// ([`tip_reference`]): the bundle's newest row for `network`
/// ([`root_bind::newest_bundled_height`] — a checkpoint the chain provably
/// reached before this binary shipped), and the wallet's OWN scanned height
/// less [`REORG_MAX_BLOCKS`] (`scanned_tip` — blocks this wallet validated by
/// scanning; `None` before the first scan, when the row alone is the
/// reference). `AtOrAboveBundle` at or above the reference, `BehindBundle`
/// strictly below it — no slack against the row, because any slack is exactly
/// the width an under-reporter gets for free (the one-below plant), and no
/// firing at equality, because the row is the newest height the bundle can
/// vouch for and a tip AT it is consistent with every proof the bundle
/// carries (the at-newest plant; the earlier reason here — "on release day
/// every honest endpoint's tip is the row" — was false by P3: the row is the
/// bundle's tail, ~17k blocks below the chain at release, and that reason
/// would have justified slack).
///
/// **T0-1c-R2 (§4n Q-G1, M1) — the second reference, and its margin.** An
/// endpoint pinned at any tip below the wallet's own scanned height used to
/// pass the row's floor: upstream `update_chain_tip` returns early on
/// `new_tip < max_scanned` (a clean 0-batch pass), the pass published a plain
/// `UpToDate` at the pinned height and the durable stamp was rewritten DOWN.
/// The wallet's own height is the stronger oracle and it was already read every
/// pass (`Wallet::evaluate_consensus`); no endpoint can move it. The margin is
/// [`REORG_MAX_BLOCKS`], the ONE reorg budget this crate has (the anchor clamp
/// in `evaluate_consensus`, the storm guard [`reorg_storm`] via
/// [`MAX_SCAN_REORGS_PER_PASS`]), and NOT one rewind step
/// ([`REWIND_DISTANCE_BLOCKS`]): a rewind never raises an endpoint's tip — it
/// LOWERS `scanned_tip`, which only relaxes this grade — so the case the margin
/// exists for is the other one: the wallet's own scanned tip sitting on a fork
/// the network then orphaned (or a server switched to one a few blocks behind
/// the last), where an HONEST, current server reports a tip below what this
/// wallet scanned by up to the reorg depth the wallet is built to survive.
/// That depth is `REORG_MAX_BLOCKS` (up to `MAX_SCAN_REORGS_PER_PASS` rewinds of
/// `REWIND_DISTANCE_BLOCKS` each); a one-step margin would badge an honest
/// server "behind" for every survivable reorg deeper than ten blocks, until the
/// chain grew past the wallet's stale height. Property bought: the badge fires
/// only on evidence stronger than the reorg budget — a dip of at most
/// `REORG_MAX_BLOCKS` below our own height reads as a reorg, never as a behind
/// server, so no reorg the wallet survives can produce a false claim on glass.
/// Property lost: an under-reporter above the row keeps `REORG_MAX_BLOCKS` of
/// slack below our own height (≈ two hours of blocks) — the residual, narrowed
/// from binary age to the reorg budget — and a dip inside the margin is
/// accepted as current, so the stamp CAN be rewritten down by up to that much
/// on an honest pass. A second cost, named because it is real: the reference
/// trusts the wallet's own scan, which trusts prev-hash continuity and not
/// proof-of-work, so a server that served a fabricated extension yesterday
/// makes the honest server today read "behind" until the chain passes the
/// fabricated height — the same trust the wallet already places in that scan
/// (it holds that height as "up to date"); the grade adds a mislabel of the
/// next server, not a new trust.
///
/// **Disposition (§4k-R): REPORT AND CONTINUE** — the orchestrator's product
/// decision, recorded for the maintainer to overturn; REFUSE was built
/// (on a private branch) and held back. The pass scans to the endpoint's height,
/// records that tip, and `sync_controller::emit_synced` publishes
/// `SyncStatus::EndpointBehind { tip, newest_known, pools }` — and, since
/// T0-1c-R2 (M2), writes NO durable stamp for a behind pass (`emit_synced`
/// gates `record_synced` on `AtOrAboveBundle`; the reasoning is there).
/// Property bought: the balance is as fresh as this server allows and the
/// surface tells the truth about how fresh; an honest-but-behind validator (P4
/// — narrowed: since lightwalletd `4267cc97`, 2022-10-20, `GetLatestBlock`
/// returns the validator's `getblockchaininfo.blocks`, so what trips this is a
/// validator still syncing, stuck or forked, or a pre-2022/forked lightwalletd)
/// is ridden out by the retry loop with the truth on glass meanwhile, never a
/// total outage (the R2-5 reasoning; principle 6). Property lost: the
/// wallet WRITES a behind tip (`record_chain_tip`) and asks a behind server for
/// roots — both money-safe: the bind never leans on the tip
/// (`root_bind::check_pool` reads `gather`'s unfiltered evidence, so a stale
/// tip cannot move a refusal), only the `proven` REPORT is tip-bounded
/// ([`proven_complete_at_or_below`] says what that buys), and a later pass on a
/// current server rescans the gap (E6); plus E12 in full — a variant, the
/// bridge, five UI sites, sixteen locales — and a send built against a behind
/// tip carries an expiry the real chain may already be past (it expires, funds
/// return; the variant's copy says so).
///
/// **Three endpoint heights per pass, ONE graded here — and the other two are
/// BOUNDED, not trusted** (the ruling corrected P1; §4k-R decision 5, amended
/// by T0-1c-R2 M3/M4 after the wrap found each of them writing durable state).
/// This one — `GetLatestBlock` — bounds the A11 ceiling, the `proven` report,
/// the recorded tip and the scan, and is the one a stale value can misdescribe
/// on glass, so it is the one graded and the one that reaches the surface.
/// `Wallet::evaluate_consensus`'s identity height (`ServerIdentity.block_height`,
/// saturated to `u32::MAX`, every pass) is JUDGED AT, not trusted, for the
/// VERDICT — taken at `max(scanned, claimed)`, so a low value cannot pull the
/// judgement below blocks already scanned — and its OTHER consumer, the grace
/// anchor `capable_tip` that `consensus_stamp::record` persists, is clamped into
/// `[scanned, scanned + REORG_MAX_BLOCKS]` at that site (M4), so a low value
/// cannot lower a persisted signing input either. `provision::resolve_birthday`'s
/// tip (first provisioning only) is read through THIS grade with no scanned
/// height (an unprovisioned wallet has none), and a below-row value is clamped
/// UP to the row before the birthday resolves (M3): the birthday it writes is
/// durable, and a lying-low tip used to floor it toward activation — a ~3 M-block
/// scan that survived switching servers. The clamp alone cannot continue the
/// pass — a clamped birthday is above a behind tip by construction, and
/// upstream cannot take a birthday above `tip + 1` (the R2 ruling measured the
/// clamp panicking the pass on exactly the remnant it was written for) — so it
/// is paired with [`birthday_beyond_tip`] in `Wallet::sync_once` (T0-1c-R3),
/// where such a pass becomes `EndpointBehind` with nothing scanned instead of
/// an inverted range handed to `update_chain_tip`. Neither height reaches the
/// surface; the sentence at each site says what it does.
///
/// **The cells** (the behind cell on a PROVISIONING pass is the fifth row):
///
/// | endpoint | standing | the pass |
/// |---|---|---|
/// | current and honest (tip ≥ the reference) | `AtOrAboveBundle` | unchanged: `UpToDate`, or `UpToDateDegraded` per the pools; the durable stamp is written |
/// | behind and honest (tip < the reference; P4) | `BehindBundle` | scans to its height, records it; `EndpointBehind { tip, newest_known, pools }`; NO durable stamp; clears by itself on the pass the server reports a tip at or above the reference (E6) |
/// | INC-023's under-reporter (tip 3,451,205, Ironwood served zero) | `BehindBundle` | the pools ARE asked; `proven` reads 0 for the rows above its tip, so the zero stays `Served { 0 }` — and the surface says the server is behind: the badge it hid is replaced by the sentence that says why it cannot be graded from that tip (E2) |
/// | under-reporting to anywhere at or above the row but below `scanned − REORG_MAX_BLOCKS` | `BehindBundle` | T0-1c-R2: the wallet's own height is the reference; a clean 0-batch pass (upstream's early return), reported behind, no stamp (G1) |
/// | under-reporting to within `REORG_MAX_BLOCKS` of our own height, at or above the row | `AtOrAboveBundle` | **the residual**, below |
/// | a behind server on the FIRST (provisioning) pass | `BehindBundle` | `resolve_birthday` clamps the tip up to the row and provisions (M3) at the birthday a current server at the row gives — the anchor ceiling, never activation; the pass then reaches this grade, and because that birthday is above the server's tip by construction, [`birthday_beyond_tip`] returns the pass BEFORE `record_chain_tip` (T0-1c-R3): `EndpointBehind { tip, newest_known, pools }`, zero batches, no tip recorded, no stamp, the loop alive — NOT "scans to its height like the second row" (that sentence stood here on the R2 join and was false: the pass panicked in upstream `from_parts`). The behind cell is unreachable on that pass ONLY when a configured birthday exceeds `max(tip, row)`: provisioning stops with `BirthdayInFuture`, rendered as its own `StallReason::BirthdayInFuture` (§4m #13, §4k-run owed 2) — never "check your connection" |
/// | any server whose tip is below the account's birthday − 1, with nothing scanned (INC-024: a fresh create or a configured birthday — both resolve to at most the anchor row + 1 — against a validator stuck below that anchor; the clamped remnant above) | `BehindBundle` below the reference, `AtOrAboveBundle` at or above it | [`birthday_beyond_tip`] fires before `record_chain_tip` (upstream would assert on `birthday..tip + 1`): below the reference, `EndpointBehind` with zero batches and the pools report beside it; at or above it, `Stalled { BirthdayInFuture }` — unreachable from any shipped resolver today (a resolved birthday is below the row on both networks), pinned through the seam with a synthetic anchor. Both leave the loop alive; the pass on which a server reports a tip at or past the birthday scans as normal |
///
/// **The residual, stated so the next reader need not rediscover it.** An
/// endpoint that under-reports its tip to anywhere at or above the reference —
/// at or above the bundle's row, and within `REORG_MAX_BLOCKS` below the
/// wallet's own scanned height — is not caught here: the wallet syncs to that
/// height and publishes `UpToDate` at it. Before T0-1c-R2 that window was the
/// distance between the chain and the row this binary shipped with (~15 days
/// at release, growing with binary age); now it is the reorg margin, ~100
/// blocks, and it does not grow. Above the row every bundled proof is in force
/// ([`proven_complete_at_or_below`] admits every row), so what such a server
/// cannot do is hide a withheld pool; what it can still do is hold the wallet
/// at most `REORG_MAX_BLOCKS` behind its own last scan. Closing THAT needs a
/// device-clock bound on the bundle's `(height, time)` rows — its own item.
///
/// §5.4: `claimed` (the endpoint's tip — the field `wallet.consensus` already
/// uses for the same number), `newest_known` (the reference: a public constant
/// of the signed binary, or the wallet's own scanned height less a constant —
/// a height `wallet.sync`'s `to` and `UpToDate { tip }` already carry; its own
/// allowlist entry, T0-1c-R2 crypto #12) and `outcome` (which reference fired,
/// a code); no host, no note, no account.
pub(crate) fn tip_standing(network: Network, tip: u32, scanned_tip: Option<u32>) -> TipStanding {
    let newest_bundled = root_bind::newest_bundled_height(network);
    let newest_known = tip_reference(newest_bundled, scanned_tip);
    if tip < newest_known {
        // Which oracle fired: the row (the endpoint is behind the chain this
        // BUILD knows) or our own scan (behind the chain this WALLET knows).
        let outcome = if tip < newest_bundled {
            "behind_bundle"
        } else {
            "behind_scanned"
        };
        tracing::warn!(
            target: "zec_wallet_core",
            claimed = tip,
            newest_known,
            outcome,
            "endpoint reports a chain tip below a height this wallet already holds — \
             the server is behind the chain; syncing to its height and reporting it"
        );
        return TipStanding::BehindBundle { newest_known };
    }
    TipStanding::AtOrAboveBundle
}

/// The grade's reference, pure: `max(newest_bundled, scanned − REORG_MAX_BLOCKS)`,
/// where a wallet that has scanned nothing contributes nothing (the row alone).
/// The subtraction saturates, so a wallet scanned to less than the margin still
/// grades against the row. Factored out so the margin can be driven at its
/// boundary without a client (`tests::the_reference_is_the_larger_of_the_row_and_our_own_height_less_the_reorg_margin`).
pub(crate) fn tip_reference(newest_bundled: u32, scanned_tip: Option<u32>) -> u32 {
    let own = scanned_tip.map_or(0, |s| s.saturating_sub(REORG_MAX_BLOCKS));
    newest_bundled.max(own)
}

/// **T0-1c-R3 (§4n-R Q-R5, INC-024) — would `record_chain_tip(tip)` hand upstream
/// an inverted range?** Pure, so the boundary can be driven without a client;
/// the seam is `Wallet::sync_once`, which calls this between the roots and the
/// tip WRITE and returns the pass instead of writing when it answers `true`.
///
/// **The mechanism it guards.** Upstream `update_chain_tip`
/// (`zcash_client_sqlite-0.22.0/src/wallet/scanning.rs:640-668`) builds
/// `ScanRange::from_parts(birthday..tip + 1, _)` for a wallet that has scanned
/// nothing (`Historic`) and for one whose last shard ends below its birthday
/// (`ChainTip`), and `from_parts`
/// (`zcash_client_backend-0.24.0/src/data_api/scanning.rs:49-52`) ASSERTS
/// `end >= start`. A birthday above `tip + 1` is therefore a panic, not an
/// error; `run_blocking` re-raises it, the controller's loop task dies and the
/// last status stays on glass forever (INC-024 — every fresh wallet against a
/// validator stuck below its birthday, on every pass; a configured birthday
/// above a behind server's tip once the account exists; the T0-1c-R2 clamped
/// remnant). The threshold is `tip + 1`, not `tip`: a birthday of exactly
/// `tip + 1` is the empty range `(tip + 1)..(tip + 1)`, which upstream accepts
/// (nothing to scan yet — a fresh wallet against a server exactly at its
/// anchor reads `UpToDate { tip }` at the base, and still does).
///
/// **The scanned clause.** Upstream returns EARLY on `tip < max_scanned`
/// (`scanning.rs:618-621`) — no range is built, no assert is reached — so a
/// wallet that has itself validated blocks past the endpoint's tip is never
/// guarded here: a tip inside the reorg margin below our own height is a reorg
/// ([`tip_standing`]'s margin, G5), one beyond it is `BehindBundle` already, and
/// a wallet that scanned at or past its birthday holds the strongest evidence
/// that the birthday is not in the future. Bought: the guard fires on exactly
/// the inverted-range precondition and never turns a survivable dip into a
/// stall. Lost: one more input, read anyway (`Wallet::sync_once` already holds
/// `scanned_tip` for the grade).
///
/// **The three decisions the contract left open (§4n-R IT-1b), recorded:**
///
/// 1. *M3's shape behind the guard — the CLAMP is kept, not REFUSE.* A
///    provisioning tip below the row is still clamped up to it in
///    `provision::resolve_birthday`, so the account-less remnant provisions at
///    the same birthday a current server at the row gives (the anchor ceiling —
///    never activation, never a server-chosen number) and this guard turns the
///    pass into `EndpointBehind` with nothing scanned. Bought: no stall on a
///    remnant, an address at once, and one posture with the shipped fresh
///    install (FR-24 writes that same birthday with no server at all). Lost:
///    the remnant carries a birthday above the behind server's chain until a
///    current server serves it — the state every FR-24 wallet is already in
///    against that server, so nothing new — and the clamp is load-bearing on
///    THIS guard: alone it crashed the pass (the R2 ruling), so a reader of
///    either site must read the other. REFUSE (a typed stall, nothing imported)
///    would have bought "no birthday the server cannot serve" at the price of
///    provisioning waiting for a current endpoint while the fresh install does
///    not.
/// 2. *One guard, not two.* One predicate; the standing [`tip_standing`]
///    already computed for this tip decides the sentence: below the reference
///    the server is provably behind and the pass publishes
///    `SyncStatus::EndpointBehind { tip, newest_known, pools }` (zero batches,
///    no tip recorded); at or above it the birthday is above a height no oracle
///    of this wallet disputes, and the pass stalls
///    `StallReason::BirthdayInFuture` — the wallet cannot say whether the
///    server is behind or the birthday is ahead, and the copy names both
///    remedies. Bought: one comparison, one site, one mutant reds both cells,
///    no second oracle and no second warn. Lost: the below-the-reference
///    sentence cannot ALSO say "your birthday is above this server" — the
///    surface carries the server fact, the birthday fact is on the log line as
///    a code, and "switch servers" is the right remedy for both.
/// 3. *The birthday is read from upstream* (`WalletRead::get_wallet_birthday`,
///    the MIN over the accounts table — `Wallet::upstream_birthday`), NOT from
///    the primary account's row (`account::scan_window_start`, the surface's
///    `birthday_height`). Bought: the guard compares the number the assert
///    reads, so it cannot drift from it (a second account or an upstream change
///    moves both). Lost: a second reader of "the birthday" beside the surface's;
///    they agree on this single-account wallet (ZIP-32 index pinned to 0), and
///    if they ever disagree the guard follows upstream, the surface the row.
///
/// **Reachability (G11).** The below-the-reference cell is the shipped path:
/// every resolver writes at most the anchor row + 1
/// (`checkpoints::bundled_treestate` caps a request at
/// `newest_known_activation + 1`; 3,427,311 mainnet / 4,125,171 testnet
/// today), which is BELOW the bundle's newest row on both networks, so a fresh
/// create or a configured birthday against any server below that anchor lands
/// here — not, as §4n-R's premise sentence had it, "the tail + 1" (no resolver
/// can write that; the R2 plant imports it through a synthetic anchor). The
/// same fact makes the at-or-above cell unreachable from any shipped resolver
/// today (a resolved birthday is always ≤ the row < the reference ≤ such a
/// tip); the arm exists because the contract names the cell and it costs one
/// match arm, and it is pinned through the seam with a synthetic anchor.
///
/// §5.4: the caller logs `claimed` (the tip) and an `outcome` code only; the
/// birthday height is deliberately not logged — no allowlisted field carries
/// it, and borrowing one is §4m #12's hole.
pub(crate) fn birthday_beyond_tip(
    birthday: Option<u32>,
    tip: u32,
    scanned_tip: Option<u32>,
) -> bool {
    let Some(birthday) = birthday else {
        return false; // no account yet — upstream ignores the whole range
    };
    let scanned_past_tip = scanned_tip.is_some_and(|scanned| tip < scanned);
    birthday > tip.saturating_add(1) && !scanned_past_tip
}

/// Record the endpoint's chain tip so `suggest_scan_ranges` tracks it (SYNC, under
/// the db lock — the reference's `update_chain_tip`). This is the FIRST unconditional
/// DB WRITE of every sync pass (it rewrites the scan queue to the new tip), so an
/// out-of-disk is MOST likely to fault here — BEFORE `scan_batch` — hence it is
/// classified through [`ClassifyStoreFault`] (#371): a `SQLITE_FULL` is the honest
/// `DiskFull`, not the `StoreCorrupt` seed-restore scare (the reliability review's HIGH
/// — the fixed scan door is never reached on the common full-disk ordering otherwise).
///
/// NOT unconditional since T0-1c-R3: the caller first asks [`birthday_beyond_tip`]
/// whether upstream can take this tip at all — a birthday above `tip + 1` makes
/// `update_chain_tip` assert (an inverted `ScanRange`), a panic and not an error —
/// and returns the pass without this write when it cannot (INC-024).
///
/// Made a SECOND time, with the same pass-head tip, by [`scan_batch`]'s reorg arm
/// right after a successful `truncate_to_height` (§4q P-R1, INC-025): upstream's
/// truncate trims the queue and re-queues nothing, and this is the call that
/// re-queues the rewound span — so it is made under the scan lock rather than
/// left to the next pass. The arm calls THIS function (§4q-R row 7: one body, not
/// a copy), and only when the recorded tip is above the landed height (§4q-R
/// P-RR1 — at or below it upstream would early-return, and the arm says so in the
/// outcome instead of making the call).
pub(crate) fn record_chain_tip(db: &mut WalletConn, tip: BlockHeight) -> Result<(), WalletError> {
    db.update_chain_tip(tip)
        .map_err(ClassifyStoreFault::into_store_fault)
}

/// The next batch to scan: the HALF-OPEN `[from, end)` of the highest-priority
/// suggested range (Spend-before-Sync §1.7 — `suggest_scan_ranges` returns ranges
/// in priority order, so the Verify range, when present, is first), CLAMPED to at
/// most [`SYNC_BATCH_BLOCKS`] blocks from the low end (forward scan, the reference's
/// `split_at(start + batch_size)`; the lock-budget + resume-granularity bound,
/// §6.3). Skips any (defensive) empty leading range so the returned batch is always
/// NON-EMPTY (`end > from`) — a guarantee the loop relies on for forward progress
/// (an empty batch would download nothing, scan nothing, and re-suggest the same
/// range forever). `Ok(None)` ⇒ nothing left to scan. SYNC, under the db lock; a fault
/// reading OUR scan queue is classified through [`ClassifyStoreFault`] (#371): a
/// device-locked `SQLITE_IOERR` (iOS Data Protection) is the honest `Io` and a WAL-race
/// `SQLITE_BUSY` the transient `StoreBusy`, not the `StoreCorrupt` scare (a read won't
/// `SQLITE_FULL`, but the IOERR/BUSY honesty is the point).
pub(crate) fn next_batch(db: &mut WalletConn) -> Result<Option<(u64, u64)>, WalletError> {
    let ranges = db
        .suggest_scan_ranges()
        .map_err(ClassifyStoreFault::into_store_fault)?;
    let Some(range) = ranges.iter().find(|r| !r.is_empty()) else {
        return Ok(None);
    };
    let from = u64::from(u32::from(range.block_range().start));
    let end_full = u64::from(u32::from(range.block_range().end));
    let end = end_full.min(from + u64::from(SYNC_BATCH_BLOCKS));
    Ok(Some((from, end)))
}

/// A reorg STORM guard: `true` once reorgs SINCE THE LAST FORWARD PROGRESS STRICTLY
/// exceed [`MAX_SCAN_REORGS_PER_PASS`] (rewinding has outpaced real progress by more
/// than `REORG_MAX_BLOCKS` — a forking/hostile endpoint, not an ordinary reorg). The
/// loop then stops with `Stalled { ChainReorg }` rather than rewind-and-rescan forever
/// (money-safety: every loop terminates, even under an adversarial scan/reorg
/// oscillation — the counter is reset only by a new high-water frontier, not by any
/// `Scanned`). Pure — boundary-tested.
pub(crate) fn reorg_storm(reorgs_since_progress: u32) -> bool {
    reorgs_since_progress > MAX_SCAN_REORGS_PER_PASS
}

/// The ABSOLUTE per-pass rewind cap (§4q-R P-RR3, the REQ-1 fold review's row 2):
/// `true` once the rewinds THIS pass has performed — counting the one being
/// classified — STRICTLY exceed [`MAX_TOTAL_REORGS_PER_PASS`], regardless of the
/// high-water and of [`reorg_storm`]'s since-progress epoch. That epoch counter
/// is a TERMINATION bound, not a work bound: any batch end above the prior
/// high-water resets it, and the tip that limits those resets is the endpoint's
/// own claim, so a fork → fabricated advance → fork server could hold ONE pass
/// alive for `(tip − high_water) × (MAX_SCAN_REORGS_PER_PASS + 1)` rewinds, each
/// a DB truncate + queue rewrite + ledger note + re-download + re-scan. This cap
/// is the number the endpoint cannot pick. Pure — boundary-tested.
pub(crate) fn reorg_cap(total_reorgs: u32) -> bool {
    total_reorgs > MAX_TOTAL_REORGS_PER_PASS
}

/// What the driver loop does with one batch's [`ScanOutcome`] (the IO it then
/// performs lives in `Wallet::sync_once`).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BatchAction {
    /// Scanned cleanly — evict `[from, end)` from the cache and continue.
    Advance,
    /// Reorg within both budgets — truncate the cache to `rewind_height` (the
    /// height the DB truncate LANDED on, §4q P-R2), then re-suggest. When
    /// `requeued` the rewound span is already queued under the scan lock (P-R1)
    /// and the re-suggest finds it; when not (§4q-R P-RR1 — the recorded tip at
    /// or below the landed height) the queue stays trimmed and the exit's DB read
    /// ends the pass as the under-claim (P-RR2). `rewound_blocks` is the depth the
    /// loop sums into `SyncPass::rewound_blocks` (P-RR4). All three pass through
    /// from [`ScanOutcome::Reorg`] unchanged.
    Rewind {
        rewind_height: u64,
        requeued: bool,
        rewound_blocks: u64,
    },
    /// Reorg STORM ([`reorg_storm`]) or the absolute cap ([`reorg_cap`]) — stop
    /// the pass with `Stalled { ChainReorg }`.
    StallReorg,
}

/// Fold a batch [`ScanOutcome`] into the loop's [`BatchAction`], updating the
/// storm-bound state IN PLACE: `high_water` (the best scanned frontier this pass)
/// and `reorgs_since_progress` (reorgs since the frontier last advanced). This is
/// the money-critical reorg ACCOUNTING — the termination guarantee (§3.2g, the
/// security fold) — factored PURE so it is exhaustively unit-testable. Until a
/// REAL continuity-error reorg could not be produced from hand-built blocks (six
/// probed scenarios — bad linkage / divergent re-scan / mis-anchored extend — all
/// surfaced as `Scanned` or `StoreCorrupt`); what those probes missed is that
/// upstream checks the first block's `prev_hash` against the wallet's STORED block
/// (`scanning/compact.rs`, `PrevHashMismatch`), so a fork the fake REVEALS after
/// the wallet scanned the original blocks does trip it — `testing::Fork`, driven by
/// `wallet::tests::a_rewound_throttled_batch_records_exactly_one_snap_ms`. Two
/// upstream facts that fixture measured: `truncate_to_height` lands on the highest
/// checkpointed height at or below the target (a batch end; none at or below ⇒
/// `RequestedRewindInvalid`) — the landed height is what `Reorg { rewind_height }`
/// carries (§4q P-R2) — and it trims the scan queue above that height without
/// re-queuing, which is why `scan_batch`'s reorg arm re-queues the rewound span
/// itself before the lock is released (§4q P-R1, INC-025): the re-scan across the
/// fork is THIS pass's, and this loop keeps running after a `Rewind`.
///
/// **The storm bound is therefore load-bearing inside ONE pass (§4q P-R4), and it
/// holds three ways.** The counter resets ONLY on a NEW high-water (`end >
/// high_water`), never on a `Scanned` that merely re-covers rewound territory —
/// that is what bounds the adversarial scan/reorg oscillation. (1) A server that
/// forks WITHOUT new progress trips [`reorg_storm`] after
/// [`MAX_SCAN_REORGS_PER_PASS`] rewinds (`REORG_MAX_BLOCKS /
/// REWIND_DISTANCE_BLOCKS`, 10) — `StallReorg`, and the pass stops. (2) A server
/// that alternates a fork with a one-block advance is bounded by the RECORDED tip:
/// a reset needs a batch end strictly above the prior high-water, the tip is fixed
/// for the pass, so the resets number at most the blocks between the first
/// high-water and the tip, each buying at most `MAX_SCAN_REORGS_PER_PASS` more
/// rewinds — a TERMINATION bound and not a work bound, because that tip is the
/// endpoint's own claim (§4q-R row 2). (3) So `total_reorgs` — the rewinds this
/// pass has already performed, `pass.reorgs`, which the loop owns — is held to
/// the absolute [`reorg_cap`] on every `Reorg` regardless of the high-water
/// (§4q-R P-RR3): the fork-advance-fork server gets [`MAX_TOTAL_REORGS_PER_PASS`]
/// rewinds in one pass and the next is `StallReorg`. Every pass terminates under
/// every shape, and the work one pass can be made to do is bounded by a constant.
pub(crate) fn classify_batch(
    outcome: ScanOutcome,
    end: u64,
    high_water: &mut u64,
    reorgs_since_progress: &mut u32,
    total_reorgs: u32,
) -> BatchAction {
    match outcome {
        ScanOutcome::Scanned { .. } => {
            if end > *high_water {
                *high_water = end;
                *reorgs_since_progress = 0;
            }
            BatchAction::Advance
        }
        ScanOutcome::Reorg {
            rewind_height,
            requeued,
            rewound_blocks,
        } => {
            *reorgs_since_progress += 1;
            // The cap counts THIS rewind (the DB already performed it inside
            // `scan_batch`), so the `MAX_TOTAL_REORGS_PER_PASS + 1`-th one stalls —
            // the same convention as `reorg_storm`'s counter.
            if reorg_storm(*reorgs_since_progress) || reorg_cap(total_reorgs.saturating_add(1)) {
                BatchAction::StallReorg
            } else {
                BatchAction::Rewind {
                    rewind_height,
                    requeued,
                    rewound_blocks,
                }
            }
        }
    }
}

/// Fuzz-only hook (testing-patterns §3): drive the two hostile-input subtree-root
/// parsers on arbitrary `(root_hash, height)`. `#[doc(hidden)]` and NOT a supported
/// API — it exists solely so the `cargo-fuzz` harness (a separate crate that sees
/// only `pub` items, [`fuzz_subtree_root`](../fuzz/fuzz_targets/fuzz_subtree_root.rs))
/// can reach these crate-internal parsers. The contract under fuzz is **never
/// panic** on any bytes/length/height (re-exported as `zec_wallet_core::
/// __fuzz_parse_subtree_root`). Returns whether BOTH pools accepted (a per-pool
/// accept/reject divergence is then observable to the fuzzer).
#[cfg(fuzzing)]
#[doc(hidden)]
pub fn __fuzz_parse_subtree_root(root_hash: Vec<u8>, height: u64) -> bool {
    let root = SubtreeRoot {
        root_hash,
        completing_block_hash: Vec::new(),
        completing_block_height: height,
    };
    parse_sapling_root(&root).is_ok() && parse_orchard_root(&root).is_ok()
}

/// Fuzz-only hook (testing-patterns §3): the GUARD-SOUNDNESS invariant for the
/// hostile-block crash defense. For ANY bytes that decode to a `CompactBlock`, IF
/// [`block_is_scannable`] accepts it, EVERY accessor the scanner unwraps must NOT
/// panic — so the `download_range` shape-check provably turns the FFI-crash surface
/// (§4.6) into a typed reject for EVERY input, not just the cases the unit tests
/// enumerate. The hook drives the FULL live panic surface, exactly mirroring
/// `scanning/compact.rs`: `height`/`hash`/`prev_hash` (block), `TxIndex::try_from
/// (tx.index)` + `txid` (tx), `spend.nf().expect()` (the find_spent SAPLING site —
/// NOT mirrored before the review's BLOCKER fix), and `cmu` (output). Orchard
/// `find_spent` is NOT mirrored — it is caught fallibly in pass 1, so it is not a
/// live panic (running `.expect()` on it would be a false positive). A gap in the
/// guard surfaces as a libFuzzer crash. `#[doc(hidden)]`, NOT a supported API
/// (re-exported as `zec_wallet_core::__fuzz_block_is_scannable`). Returns whether the
/// block was accepted (so the fuzzer sees the accept/reject split).
#[cfg(fuzzing)]
#[doc(hidden)]
pub fn __fuzz_block_is_scannable(block_bytes: Vec<u8>) -> bool {
    let Ok(block) = CompactBlock::decode(block_bytes.as_slice()) else {
        return false;
    };
    if !block_is_scannable(&block) {
        return false;
    }
    // Accepted ⇒ the panicking accessors the scanner uses must all be safe. Running
    // them on any accepted block is the assertion (a crash = a guard gap). `u16` is
    // exactly what `TxIndex::try_from` reduces to; `spend.nf().expect()` mirrors
    // `find_spent`'s SAPLING unwrap.
    let _ = block.height();
    let _ = block.hash();
    let _ = block.prev_hash();
    for tx in &block.vtx {
        let _ = u16::try_from(tx.index).expect("guard ⇒ tx.index fits u16 (TxIndex)");
        let _ = tx.txid();
        for spend in &tx.spends {
            let _ = spend
                .nf()
                .expect("guard ⇒ sapling spend nf decodes (find_spent)");
        }
        for out in &tx.outputs {
            let _ = out.cmu();
        }
    }
    true
}

/// Test-only scripted [`SubtreeRootSource`] — no live gRPC server (mirrors
/// provision's `testing::FakeOracle`). Shared by sync's own tests and the
/// `wallet::update_subtree_roots` end-to-end test.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::net::grpc::testing::scripted_stream;

    /// The test double for [`crate::ports::WallClock`] (GRACE-1, §4p P-G2): a
    /// wall clock a proof sets by hand. One shared `Arc<AtomicU64>`, so the
    /// handle a test keeps and the one it hands the wallet
    /// (`Wallet::open_with_vault_and_seed_port(cfg, vault, None,
    /// Some(Arc::new(clock.clone())))`) read the same instant — advance or
    /// rewind it between passes and the next read of the consensus stamp sees
    /// the change through the shipped seam, with no `#[cfg(test)]` door in the
    /// wallet itself. `Clone` is the sharing: every clone is the same clock.
    #[derive(Clone, Debug)]
    pub(crate) struct ManualClock(std::sync::Arc<std::sync::atomic::AtomicU64>);

    impl ManualClock {
        /// A clock reading `unix_secs` now.
        pub(crate) fn at(unix_secs: u64) -> Self {
            Self(std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
                unix_secs,
            )))
        }
        /// Set the clock to an absolute instant (forward OR backward — the
        /// G-2b / G-3 geometries).
        pub(crate) fn set(&self, unix_secs: u64) {
            self.0.store(unix_secs, std::sync::atomic::Ordering::SeqCst);
        }
        /// Move the clock forward by `secs` (saturating).
        pub(crate) fn advance(&self, secs: u64) {
            let now = self.now();
            self.set(now.saturating_add(secs));
        }
        /// Move the clock BACK by `secs` (saturating at the epoch) — a device
        /// whose time was corrected downward, or set back deliberately.
        pub(crate) fn rewind(&self, secs: u64) {
            let now = self.now();
            self.set(now.saturating_sub(secs));
        }
        /// What the clock reads now.
        pub(crate) fn now(&self) -> u64 {
            self.0.load(std::sync::atomic::Ordering::SeqCst)
        }
        /// This clock as the port the wallet takes.
        pub(crate) fn port(&self) -> std::sync::Arc<dyn crate::ports::WallClock> {
            std::sync::Arc::new(self.clone())
        }
    }

    impl crate::ports::WallClock for ManualClock {
        fn now_unix(&self) -> u64 {
            self.now()
        }
    }

    /// A scripted endpoint. Each pool yields its scripted stream items in order;
    /// `*_open_err` injects a fault when OPENING that pool's stream (before any
    /// item) — a flaky link that drops between two RPCs, or, with
    /// [`GrpcError::ShieldedProtocolUnknown`], a server too old to know the pool.
    ///
    /// **The third stream landed in the same change as the production fetch/put**,
    /// which is what the `unreachable!()` that used to sit in `subtree_roots` was
    /// holding the line for: a fake that served Ironwood while production skipped it
    /// would have made the pair look symmetric and every test green.
    ///
    /// ⚠️ The Orchard and Ironwood streams are the SAME Rust type all the way down —
    /// `SubtreeRoot` on the wire, `CommitmentTreeRoot<MerkleHashOrchard>` after the
    /// parse — so serving the same root bytes to both is not a type error and a
    /// cross-wire between the two trees would be invisible. Serve them DISTINCT
    /// values (see [`canonical_root`]'s `tag`), and read each back from its own
    /// tree; distinct values alone prove nothing if nobody looks.
    pub(crate) struct FakeRootSource {
        pub(crate) sapling: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        pub(crate) orchard: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        pub(crate) ironwood: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        pub(crate) sapling_open_err: Option<GrpcError>,
        pub(crate) orchard_open_err: Option<GrpcError>,
        pub(crate) ironwood_open_err: Option<GrpcError>,
    }

    impl FakeRootSource {
        /// An honest endpoint serving the given roots for each pool.
        pub(crate) fn honest(
            sapling: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
            orchard: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
            ironwood: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        ) -> Self {
            Self {
                sapling,
                orchard,
                ironwood,
                sapling_open_err: None,
                orchard_open_err: None,
                ironwood_open_err: None,
            }
        }
    }

    #[async_trait]
    impl SubtreeRootSource for FakeRootSource {
        async fn subtree_roots(
            &mut self,
            protocol: ShieldedProtocol,
            start_index: u32,
        ) -> Result<SubtreeRootStream, GrpcError> {
            // Exhaustive with NO wildcard arm, for the same reason
            // `fetch_subtree_roots` is: a fourth upstream pool must break this
            // double's build rather than let it quietly answer for a pool it has no
            // script for.
            match protocol {
                ShieldedProtocol::Sapling => match self.sapling_open_err.take() {
                    Some(e) => Err(e),
                    None => Ok(scripted_stream(from_index(
                        std::mem::take(&mut self.sapling),
                        start_index,
                    ))),
                },
                ShieldedProtocol::Orchard => match self.orchard_open_err.take() {
                    Some(e) => Err(e),
                    None => Ok(scripted_stream(from_index(
                        std::mem::take(&mut self.orchard),
                        start_index,
                    ))),
                },
                ShieldedProtocol::Ironwood => match self.ironwood_open_err.take() {
                    Some(e) => Err(e),
                    None => Ok(scripted_stream(from_index(
                        std::mem::take(&mut self.ironwood),
                        start_index,
                    ))),
                },
            }
        }
    }

    /// Honour `start_index` over a scripted pool stream (S15-F1): the script is the
    /// pool's roots FROM INDEX 0, and a stream opened at `start` serves the roots
    /// from index `start` on — the first `start` ROOTS are skipped, while every
    /// non-root item (a fault, a clean end) keeps its place relative to the roots
    /// that are served. `start == 0` is the script unchanged.
    pub(crate) fn from_index(
        script: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        start: u32,
    ) -> Vec<Result<Option<SubtreeRoot>, tonic::Status>> {
        let mut skip = start as usize;
        script
            .into_iter()
            .filter(|item| {
                if skip > 0 && matches!(item, Ok(Some(_))) {
                    skip -= 1;
                    false
                } else {
                    true
                }
            })
            .collect()
    }

    /// A canonical-field-element root_hash: a small little-endian integer is always
    /// in range for both jubjub::Base and pallas::Base (moduli ≈ 2^255).
    pub(crate) fn canonical_root(height: u64, tag: u8) -> SubtreeRoot {
        let mut root_hash = vec![0u8; 32];
        root_hash[0] = tag;
        SubtreeRoot {
            root_hash,
            completing_block_hash: Vec::new(),
            completing_block_height: height,
        }
    }

    // ── T0-1d: the bind-consistent sequence generator ─────────────────────────
    //
    // The short-serve rule (`withheld`, `served < proven`) grades every honest
    // fixture that serves fewer roots than the bundle proves as degraded — and
    // every reproducible mainnet fixture in this repository did (one Sapling root
    // against 1,128). The adjudicator ruled (§4i (i)) that a bind-consistent
    // sequence is derivable from `bundled_counts` alone, and this is that
    // derivation: for subtree index `i` the honest completing height lies in
    // `(max{H : complete(H) <= i}, min{H : complete(H) > i}]` — strictly above the
    // newest bundled row that had not yet completed subtree `i`, at or below the
    // oldest row that had — exactly the two halves of `root_bind::check_against_count`.
    // Consecutive completions are placed `MIN_COMPLETION_GAP_BLOCKS` apart (the
    // bind's own floor) and as LOW as their window allows, which is optimal: the
    // windows are monotone in `i`, so an earlier placement never costs a later one.
    // A window that cannot hold its completion is REPORTED, never padded — that
    // would be a finding about the bundle's spacing, not about the item (P8).
    //
    // The proof files carry their own generator from the same spec (the join
    // hazard: proof doubles are private to their files); the two are unified at
    // the fold, and this one is the implementer's (§4l decision 3).

    /// A bundled window that cannot hold the completion it must — the generator's
    /// receipt when the bundle's rows are closer together than the bind's own gap
    /// floor allows (never observed on either shipped bundle;
    /// `tests::the_generator_fills_every_window_on_both_networks_and_the_bind_accepts_it`
    /// measures it).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct UnfillableWindow {
        pub(crate) network: Network,
        pub(crate) pool: ShieldedProtocol,
        /// The subtree index whose window could not be filled.
        pub(crate) index: u64,
        /// The lowest height the index could take: above the newest row proving at
        /// most `index` subtrees, at or above the pool's activation, and at least
        /// the gap floor above the previous completion.
        pub(crate) lowest: u32,
        /// The oldest bundled row proving MORE than `index` subtrees — the height
        /// the completion must sit at or below.
        pub(crate) ceiling: u32,
    }

    /// The network upgrade whose activation floors a pool's first completion —
    /// the same three pairings `fetch_subtree_roots` spells per arm. Exhaustive so
    /// a fourth pool must be paired here before it can be generated for.
    fn pool_upgrade(pool: ShieldedProtocol) -> NetworkUpgrade {
        match pool {
            ShieldedProtocol::Sapling => NetworkUpgrade::Sapling,
            ShieldedProtocol::Orchard => NetworkUpgrade::Nu5,
            ShieldedProtocol::Ironwood => NetworkUpgrade::Nu6_3,
        }
    }

    /// One completing height per subtree the bundle proves complete for `pool`
    /// on `network` — `m` heights for the bundle's largest `complete`, each inside
    /// its window, strictly increasing, `MIN_COMPLETION_GAP_BLOCKS` apart, at or
    /// above the pool's activation. Every bundled row is honoured (the bind reads
    /// them unfiltered), so the sequence is accepted by `root_bind::check_pool`
    /// on a fresh wallet at any tip at or above the newest row, and its prefixes
    /// are accepted too (a prefix is what the count bind calls harmless).
    pub(crate) fn bind_consistent_heights(
        network: Network,
        pool: ShieldedProtocol,
    ) -> Result<Vec<u32>, UnfillableWindow> {
        let rows = root_bind::bundled_counts(network, pool);
        let proven = rows.iter().map(|&(_, c)| c).max().unwrap_or(0);
        let activation = pool_activation(network, pool_upgrade(pool)).map_or(0, u32::from);
        let mut heights: Vec<u32> = Vec::with_capacity(usize::try_from(proven).unwrap_or(0));
        for index in 0..proven {
            // Above every row that had NOT yet completed subtree `index`
            // (`check_against_count`'s "at or above `complete` ⇒ strictly above").
            let floor = rows
                .iter()
                .filter(|&&(_, c)| c <= index)
                .map(|&(h, _)| h)
                .max();
            // At or below the oldest row that HAD (its "below `complete` ⇒ at or
            // below" half). Exists for every `index < proven` by construction.
            let ceiling = rows
                .iter()
                .filter(|&&(_, c)| c > index)
                .map(|&(h, _)| h)
                .min()
                .expect("index < proven ⇒ some row proves more than index subtrees");
            let after_previous = heights.last().map_or(0, |&p| {
                p.saturating_add(root_bind::MIN_COMPLETION_GAP_BLOCKS)
            });
            let lowest = floor
                .map_or(activation, |f| f.saturating_add(1))
                .max(activation)
                .max(after_previous);
            if lowest > ceiling {
                return Err(UnfillableWindow {
                    network,
                    pool,
                    index,
                    lowest,
                    ceiling,
                });
            }
            heights.push(lowest);
        }
        Ok(heights)
    }

    /// A 32-byte `root_hash` distinct per `(pool, index)` that is a canonical
    /// field element for BOTH jubjub::Base and pallas::Base: the little-endian
    /// integer `(index + 1) + tag · 2^64`, where `tag` is the pool's wire value
    /// plus one — always below 2^66, so always in range of either modulus
    /// (≈ 2^255). Bytes 0..8 carry the index, byte 8 the pool: no overflow at
    /// 1,128 Sapling roots (or at 65,536, the per-pool cap), no collision across
    /// pools (the two Orchard-shaped trees MUST differ, or the cross-wire the
    /// `SubtreeRoots` doc warns of is invisible), and no collision with
    /// [`canonical_root`]'s single-byte tags (those are the integers 0..=255).
    pub(crate) fn wide_root_hash(pool: ShieldedProtocol, index: u64) -> Vec<u8> {
        let mut root_hash = vec![0u8; 32];
        root_hash[..8].copy_from_slice(&(index + 1).to_le_bytes());
        // `ShieldedProtocol as i32` is the wire value (0/1/2 — compile-time
        // asserted at `SUBTREE_ROOT_POOLS`); +1 keeps byte 8 non-zero for every pool.
        root_hash[8] = u8::try_from(pool as i32 + 1).expect("three pools");
        root_hash
    }

    /// The FULL honest serve for `pool` on `network`: one wire `SubtreeRoot` per
    /// height from [`bind_consistent_heights`], hashed by [`wide_root_hash`], no
    /// `completing_block_hash` (the C6 cross-check abstains). Serve a prefix of it
    /// to build a short serve; serve all of it at a tip at or above the bundle's
    /// newest row to build the healthy control.
    pub(crate) fn bind_consistent_roots(
        network: Network,
        pool: ShieldedProtocol,
    ) -> Result<Vec<SubtreeRoot>, UnfillableWindow> {
        Ok(bind_consistent_heights(network, pool)?
            .into_iter()
            .zip(0u64..)
            .map(|(height, index)| SubtreeRoot {
                root_hash: wide_root_hash(pool, index),
                completing_block_hash: Vec::new(),
                completing_block_height: u64::from(height),
            })
            .collect())
    }

    use zcash_client_backend::proto::compact_formats::{ChainMetadata, CompactBlock};

    /// A deterministic 32-byte block id for `height` (the wire `hash`/`prev_hash`
    /// field shape: exactly 32 bytes — what `block_is_scannable` requires and what a
    /// real lightwalletd serves). Height-encoded so blocks chain (`prev_hash` of `h`
    /// = id of `h-1`).
    pub(crate) fn block_id(height: u64) -> Vec<u8> {
        let mut v = vec![0u8; 32];
        v[..8].copy_from_slice(&height.to_le_bytes());
        v
    }

    /// **B1-10 — what one block DECLARES its three pools' commitment trees hold.**
    ///
    /// The scripted chain's per-block `ChainMetadata`, made a knob instead of the
    /// three hard zeros it was. The zeros are still the default and every existing
    /// caller keeps them byte for byte ([`compact_block`] is
    /// `compact_block_with_sizes(h, DeclaredTreeSizes::ZERO)`), which matters because
    /// the zeros are load-bearing for the rows that want the SCANNED count oracle to
    /// refuse (`sync_bind_proof`'s `OVER_SCAN_BASE` window).
    ///
    /// **What declaring a non-zero size costs, since four rows have been blocked on
    /// it since an earlier revision** (`sync_bind_proof`'s `CAP_BASE` note, RULING.md "the four
    /// blocked rows"). A declared size is checked by THREE readers, and a fixture
    /// that satisfies fewer than three is refused before it reaches the behaviour
    /// under test:
    ///
    /// 1. **Ours** — `derive_chain_state` (the SCAN-2 derived-frontier comparison)
    ///    folds the anchor's frontier over the block's own outputs/actions and
    ///    refuses `endpoint_unusable()` on a mismatch. So a declared size must equal
    ///    *anchor frontier size + the commitments this block actually carries* — you
    ///    cannot declare growth the block's transactions do not contain.
    /// 2. **Upstream's scanner** — `ScanError::TreeSizeMismatch`
    ///    (`zcash_client_backend-0.24.0 scanning/compact.rs:757-785`), the same
    ///    equation against the batch's own running position.
    /// 3. **Upstream's writer** — `put_blocks_rows`
    ///    (`zcash_client_backend-0.24.0 data_api/ll/wallet.rs:326-345`) refuses
    ///    `NonSequentialBlocks` unless `from_state`'s frontier size plus the FIRST
    ///    block's commitments equals that block's declared size. **This is the half
    ///    the adjudicator hit**: an empty frontier pins the anchor at zero, so
    ///    the block may only declare zero.
    ///
    /// The knob is therefore TWO halves and both are here: this type for the blocks,
    /// and [`frontier_hex`] for the anchor/birthday `TreeState` that has to agree
    /// with them.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct DeclaredTreeSizes {
        pub(crate) sapling: u32,
        pub(crate) orchard: u32,
        pub(crate) ironwood: u32,
    }

    impl DeclaredTreeSizes {
        /// What every block of this fake declared before B1-10 and what every
        /// existing caller still gets.
        pub(crate) const ZERO: Self = Self {
            sapling: 0,
            orchard: 0,
            ironwood: 0,
        };

        /// `leaves` declared for Sapling, the other two left at zero.
        pub(crate) const fn sapling(leaves: u32) -> Self {
            Self {
                sapling: leaves,
                ..Self::ZERO
            }
        }

        /// `leaves` declared for Ironwood, the other two left at zero.
        pub(crate) const fn ironwood(leaves: u32) -> Self {
            Self {
                ironwood: leaves,
                ..Self::ZERO
            }
        }

        /// This declaration as the wire `ChainMetadata` a block carries.
        pub(crate) fn metadata(self) -> ChainMetadata {
            ChainMetadata {
                sapling_commitment_tree_size: self.sapling,
                orchard_commitment_tree_size: self.orchard,
                ironwood_commitment_tree_size: self.ironwood,
            }
        }
    }

    /// **B1-10's ANCHOR half: the hex `TreeState.*_tree` field of a frontier holding
    /// exactly `leaves` note commitments.**
    ///
    /// One function for all three pools, because the wire form is the legacy
    /// `CommitmentTree` serialization (`zcash_primitives-0.30.1 merkle_tree.rs:184`:
    /// `Optional<Node> left | Optional<Node> right | Vector<Optional<Node>> parents`)
    /// and every node it carries here is the 32-byte all-zero encoding — the integer
    /// zero, which is a canonical field element for `jubjub::Base` AND for
    /// `pallas::Base`, so the same bytes parse as a Sapling node and as the
    /// Orchard-shaped node the Orchard and Ironwood trees use.
    ///
    /// **The nodes are filler and the SIZE is the whole point.** Nothing downstream
    /// of a test fixture recomputes a root from this frontier and compares it to a
    /// chain: `CommitmentTree::size()` is read off which slots are OCCUPIED
    /// (`incrementalmerkletree-0.8.2 frontier.rs:513`), never from the node values,
    /// and the three readers in [`DeclaredTreeSizes`]' doc compare sizes and nothing
    /// else. Appending real leaves instead would mean 65,536 Pedersen hashes per
    /// fixture for the one property being bought.
    ///
    /// The occupancy is that `size()` fold read backwards: `left`/`right` carry the
    /// low 1 or 2, and `parents[i]` carries `2^(i+1)`. So an ODD `leaves` is
    /// `left` alone plus the bits of `leaves - 1`, and an EVEN one is `left` +
    /// `right` plus the bits of `leaves - 2` — and in both cases `leaves - base` is
    /// even, which is exactly the invariant that makes the parents' powers of two
    /// reach it.
    ///
    /// `leaves == 0` is the EMPTY string, which is what the shipped bundle carries
    /// for a pool below its activation and what `TreeState::sapling_tree()` reads as
    /// `CommitmentTree::empty()`.
    ///
    /// **These bytes are hand-built and therefore self-checked at every use**: the
    /// callers run them back through the SAME upstream reader production uses
    /// (`TreeState::to_chain_state`, via `account::birthday_from_treestate`) and
    /// assert the decoded frontier's `tree_size()` — see
    /// `sync_bind_proof::the_declared_tree_size_knob_is_accepted_by_both_checkers`.
    /// A wrong encoding fails there loudly; it cannot pass as a smaller tree.
    pub(crate) fn frontier_hex(leaves: u64) -> String {
        if leaves == 0 {
            return String::new();
        }
        /// The 32-byte all-zero node encoding, canonical in both fields.
        const FILLER: [u8; 32] = [0u8; 32];
        fn optional_node(out: &mut Vec<u8>, present: bool) {
            if present {
                out.push(1);
                out.extend_from_slice(&FILLER);
            } else {
                out.push(0);
            }
        }

        let (right, remaining) = if leaves % 2 == 1 {
            (false, leaves - 1)
        } else {
            (true, leaves - 2)
        };
        // `remaining` is even, so its highest set bit is at index >= 1 and
        // `parents[i]` (worth `2^(i+1)`) reaches every set bit of it.
        let parents_len = if remaining == 0 {
            0
        } else {
            usize::try_from(63 - remaining.leading_zeros()).expect("a small tree depth")
        };
        assert!(
            parents_len < 32,
            "a CommitmentTree of depth 32 cannot hold {leaves} leaves"
        );

        let mut out: Vec<u8> = Vec::new();
        optional_node(&mut out, true); // left is always occupied for leaves >= 1
        optional_node(&mut out, right);
        // `Vector`'s CompactSize length: one byte while the length is below 253,
        // which a depth-32 tree's 31-entry maximum always is.
        out.push(u8::try_from(parents_len).expect("parents_len < 32"));
        for i in 0..parents_len {
            optional_node(&mut out, remaining & (1u64 << (i + 1)) != 0);
        }
        hex::encode(out)
    }

    /// A WELL-FORMED (shape-valid, chained) empty CompactBlock at `height` — 32-byte
    /// `hash`/`prev_hash`, no transactions, empty commitment-tree metadata (tree size
    /// unchanged). Passes `block_is_scannable` AND, chained from the right anchor,
    /// the REAL `scan_cached_blocks` (which records it as scanned — probe-verified).
    /// Note CONTENT (a detectable note) needs heavy upstream `test-dependencies`
    /// fixtures and is the GA-blocking e2e's job; the completeness/ordering/guard +
    /// driver-loop tests need only valid SHAPE + chaining, which this provides.
    ///
    /// The three zeros are [`DeclaredTreeSizes::ZERO`] now rather than three literals;
    /// what declaring anything else costs is written on that type (B1-10).
    pub(crate) fn compact_block(height: u64) -> CompactBlock {
        compact_block_with_sizes(height, DeclaredTreeSizes::ZERO)
    }

    /// [`compact_block`] whose `ChainMetadata` declares `sizes` (B1-10). ADDITIVE:
    /// `compact_block` IS this at [`DeclaredTreeSizes::ZERO`], so no existing caller
    /// changes by a byte.
    pub(crate) fn compact_block_with_sizes(height: u64, sizes: DeclaredTreeSizes) -> CompactBlock {
        CompactBlock {
            height,
            hash: block_id(height),
            prev_hash: block_id(height.wrapping_sub(1)),
            chain_metadata: Some(sizes.metadata()),
            ..Default::default()
        }
    }

    /// One real Ironwood receipt, and the height that carries it (T0-3, §4z).
    ///
    /// The fake's chain is otherwise empty, which is exactly why an Ironwood-only
    /// batch is the shape §4z is about: no Sapling output and no Orchard action
    /// anywhere, so `ScanSummary`'s two counts stay zero and the ADR-0536 gate can
    /// only open on the Ironwood count.
    #[derive(Clone)]
    pub(crate) struct IronwoodReceipt {
        /// The height whose block carries the transaction.
        pub(crate) at: u64,
        /// The transaction, built by the caller from upstream's `IronwoodFvk`
        /// against the WALLET'S OWN account key — a note this wallet can actually
        /// decrypt, not a shape.
        pub(crate) tx: zcash_client_backend::proto::compact_formats::CompactTx,
        /// How many Ironwood commitments the transaction adds. The chain has no
        /// others, so this is the cumulative tree size from `at` upward — and the
        /// SCAN-2 fold checks every block's `chain_metadata` against the frontier
        /// it derives, so a fixture that did not carry this would be refused as
        /// the endpoint's fault before it ever reached the scanner.
        pub(crate) commitments: u32,
    }

    /// Apply an [`IronwoodReceipt`] to one block: the transaction at its own
    /// height, and the cumulative `ironwood_commitment_tree_size` at that height
    /// and every height above it.
    fn with_ironwood_receipt(
        mut block: CompactBlock,
        receipt: Option<&IronwoodReceipt>,
    ) -> CompactBlock {
        let Some(r) = receipt else {
            return block;
        };
        if block.height < r.at {
            return block;
        }
        if block.height == r.at {
            block.vtx = vec![r.tx.clone()];
        }
        if let Some(meta) = block.chain_metadata.as_mut() {
            meta.ironwood_commitment_tree_size = r.commitments;
        }
        block
    }

    /// Hex a 32-byte block id the way a lightwalletd serves it in a `TreeState`:
    /// DISPLAY order, the byte-reverse of the internal order a `CompactBlock.hash`
    /// carries, because `TreeState::to_chain_state` reverses what it decodes
    /// (`zcash_client_backend-0.24.0/src/proto.rs:456-457`).
    ///
    /// The convention's home for the fixtures that go through it, so one of them
    /// cannot serve the other order by accident. That has happened four times in
    /// this file's history: `chain_tree_state` itself (fixed at the SCAN-2 fold), a
    /// second copy in `degraded_pool_proof` (fixed), the three FORK paths
    /// below (found by the crypto audit) and `sync_bind_proof`'s `tree_state_on`
    /// (found by the security review, in the same sweep that added this
    /// helper). Reach for this rather than a bare `hex::encode` — but note the claim
    /// is "the callers listed here route through one predicate", NOT "no bare
    /// `hex::encode` of a block id exists": a `grep` is what checks that, and a
    /// helper cannot enforce it.
    pub(crate) fn display_order_hex(id: Vec<u8>) -> String {
        hex::encode(id.into_iter().rev().collect::<Vec<u8>>())
    }

    /// A `TreeState` whose block `hash` is `block_id(height)` and whose pools are
    /// empty (empty frontier @ `height`) — the per-batch scan ANCHOR consistent with
    /// the [`compact_block`] chain (`from_state @ h-1`'s block hash == block `h`'s
    /// `prev_hash`). Importing the account birthday at `chain_tree_state(B)` anchors
    /// the FIRST scanned block (`B+1`) to `block_id(B)`, so the whole chain links.
    pub(crate) fn chain_tree_state(height: u64) -> TreeState {
        TreeState {
            network: "test".to_owned(),
            height,
            // DISPLAY order, which is the reverse of the internal order a
            // `CompactBlock.hash` carries — `TreeState::to_chain_state` reverses
            // what it decodes (`zcash_client_backend-0.24.0/src/proto.rs:456-457`),
            // so a fixture that hex-encoded the raw id served a hash that was the
            // byte-reverse of the same block's own. Nothing read it until SCAN-2:
            // the anchor reconcile compares the whole `ChainState`, so over this
            // fake a pass longer than the cadence stalled `EndpointMisbehaving` —
            // it red two shipped summary-throttle rows that scan 81 consecutive
            // batches. The SCAN-2 implementer measured the reversal and named this
            // line as the join's to fix; the join records it here.
            hash: display_order_hex(block_id(height)),
            time: 0,
            sapling_tree: String::new(),
            orchard_tree: String::new(),
            // Empty is CORRECT for these fixtures: they model a synthetic testnet
            // chain far below the NU6.3 activation, where the Ironwood pool does not
            // exist. Do NOT reuse this helper at a post-activation height — an empty
            // frontier there decodes without error and anchors against a pool the
            // chain has, which is the silent failure `bundled_treestate` and
            // `treestate_bundle_carries_an_ironwood_frontier_after_activation` exist
            // to prevent.
            ironwood_tree: String::new(),
        }
    }

    /// A generative scripted endpoint for the DRIVER-LOOP tests: serves the
    /// [`compact_block`] chain for ANY requested range, the [`chain_tree_state`]
    /// anchor at any height, the configured tip, and EMPTY subtree roots (the loop's
    /// spend-anchor step is a no-op for detection). Unlike [`FakeScanClient`] (a
    /// fixed scripted Vec for the unit download tests) this drives the real
    /// `scan_cached_blocks` over a self-consistent chain — NO gRPC server, NO network.
    /// A ONE-SHOT async rendezvous (fired on the FIRST `block_range`): `started`
    /// carries the requested `(start,end)`, then the fake `await`s `release`.
    pub(crate) type DownloadGate = (
        tokio::sync::oneshot::Sender<(u64, u64)>,
        tokio::sync::oneshot::Receiver<()>,
    );

    pub(crate) struct FakeChain {
        tip: u64,
        /// Records each `block_range(start, end_inclusive)` the loop requests.
        pub(crate) ranges: std::sync::Arc<std::sync::Mutex<Vec<(u64, u64)>>>,
        /// The [`DownloadGate`] fired on the FIRST `block_range` (the lock-free
        /// download phase between two per-batch db-lock holds): lets a test
        /// deterministically interleave — over `tokio::join!` on one runtime — a
        /// concurrent db-lock acquisition (the per-batch lock-release proof) or a
        /// `cancel` flip (the between-batch cancellation proof) while NO db lock is
        /// held. Consumed on first use; later batches run ungated.
        pub(crate) download_gate: Option<DownloadGate>,
        /// Inject a transport fault when `block_range` is OPENED for this start
        /// height (the flaky-network 3-lens: a link drop mid-multi-batch sync).
        /// Cleared on use, so a healed retry serves the range normally.
        pub(crate) block_range_err_at: Option<u64>,
        /// Inject a transport fault on `latest_block_height` (the endpoint drops at
        /// the chain-tip fetch — the very start of a pass).
        pub(crate) tip_err: Option<GrpcError>,
        /// A fork the fake REVEALS after [`Fork::after_ranges`] `block_range`
        /// calls (SCAN-1, review item 5 — the loop-level reorg fixture).
        pub(crate) fork: Option<Fork>,
        /// A SEQUENCE of forks, revealed one per advance the wallet makes
        /// (§4q-R RR-3 — the fork-advance-fork server). ADDITIVE beside
        /// [`Self::fork`] and read only when that one is `None`: every existing
        /// caller keeps the single-fork behaviour byte for byte. See
        /// [`ForkStorm`].
        pub(crate) storm: Option<ForkStorm>,
        // Both rounds widened this fixture blind of each other — REW-1 needs a
        // fork at every pass head, SCAN-2 needs a receipt of the tree-state calls and
        // a lying state to reconcile against. The fields are independent knobs on one
        // fake; the fold takes both sides rather than choosing.
        /// A fork at EVERY pass head (§4u RW-4 — the server that rewinds a
        /// controller's successive passes, each once). ADDITIVE beside
        /// [`Self::fork`] and [`Self::storm`] and read only when both are
        /// `None`. See [`PassForks`].
        pub(crate) pass_forks: Option<PassForks>,
        /// The heights [`Self::pass_forks`] has forked at, ascending — one per
        /// pass head served while the mode had forks left. Behind a handle so a
        /// row can read the fixture's own receipt after the client was moved
        /// into an engine (the way `ranges` is read).
        pub(crate) pass_fork_heights: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
        /// The (virtual-clock) instant of every `latest_block_height` call —
        /// the pass-head tip fetch, one per pass — so a row that runs a
        /// controller LOOP over this chain can read the inter-pass spacing the
        /// loop chose, the way the controller's own `FakeEngine::call_times`
        /// does. Recorded in every mode; nothing reads it but a row.
        pub(crate) pass_heads: std::sync::Arc<std::sync::Mutex<Vec<tokio::time::Instant>>>,
        /// SCAN-2 (§4t S2-1/S2-4/S2-5): every height the loop asked
        /// `tree_state` for, in call order — the receipt that says how many
        /// `GetTreeState` round trips a pass made and for which anchors. The
        /// sibling of [`Self::ranges`], kept as its own list (a fetch is not a
        /// range). Shared behind an `Arc` for the same reason `ranges` is: the
        /// pass borrows the fake mutably, so a reporter or a controller-driven
        /// row reads the receipt through a clone taken before the pass.
        pub(crate) tree_states: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
        /// SCAN-2 (§4t S2-3): a tree state served INSTEAD of [`chain_tree_state`]
        /// for every requested height AT OR ABOVE `.0`, its `height` rewritten to
        /// the requested one — an endpoint whose `GetTreeState` disagrees with the
        /// blocks it serves, from that height on. "At or above" rather than "at":
        /// the height a build reconciles at is that build's cadence, so a row
        /// that must catch ANY cadence lies at every height after its first
        /// fetch. `None` (every existing caller) ⇒ the honest chain. Read AFTER
        /// the receipt above is written, so a lie is counted like any fetch.
        pub(crate) tree_state_override: Option<(u64, TreeState)>,
        /// SCAN-2 (§4t S2-3): every height at which [`Self::tree_state_override`]
        /// actually FIRED — what the fake SERVED, not what the loop asked for.
        /// [`Self::tree_states`] cannot answer that question: it records a
        /// request whether or not a lie answered it, so a row reading it to
        /// prove "the lie was served" passes just as well when the override is
        /// dead (MEASURED with the arm disabled, the S2-3 row's
        /// non-vacuity clause still passed and the row reddened on its `Ok`
        /// arm exactly as it does honestly — the clause discriminated nothing).
        /// This list does answer it: empty whenever the override is `None`, or
        /// set but never reached.
        pub(crate) tree_state_lies: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
        /// SCAN-2 option (c): from this height on, serve the HONEST tree state with
        /// its block hash hexed in the OTHER byte-order convention — the raw block
        /// id, without the reversal [`chain_tree_state`] applies to cancel
        /// `TreeState::to_chain_state`'s unconditional one. Every other field is the
        /// honest one, so the served `ChainState` differs from the derived one in the
        /// block hash and in nothing else, and differs by exactly a byte reversal.
        ///
        /// A knob rather than a fixed [`Self::tree_state_override`] state because the
        /// hash has to track the REQUESTED height: the reconcile height is a build's
        /// cadence, so a fixture that must work at any cadence cannot hard-code one
        /// hash. Read AFTER the override (so a row can set both and the override
        /// wins) and recorded in [`Self::tree_state_lies`] like any served
        /// disagreement.
        ///
        /// This is not a hypothetical server: it is the shape this repository's own
        /// fixtures had, in two separate places in one session, and the reason
        /// nothing downstream reading the field is what made it survive.
        pub(crate) tree_state_byte_order_flip: Option<u64>,
        /// SCAN-2 option (c), the twin of the flip above: from this height on, serve
        /// the honest tree state with its Sapling frontier replaced by this hex — the
        /// block HASH left honest, so the served `ChainState` differs from the derived
        /// one in a FRONTIER and not in the hash.
        ///
        /// It exists because (c) gives the hash a verdict of its own, and the only
        /// thing that stops "split by field" from becoming "ignore the hash" is a row
        /// proving the OTHER fields still refuse. The pre-(c) rows all lie on the
        /// hash, so without this knob nothing measures the frontier path at all.
        /// `None` (every existing caller) ⇒ the honest frontier.
        pub(crate) tree_state_frontier_lie: Option<(u64, String)>,
        /// T0-3 (§4z): ONE real Ironwood receipt this chain carries, and the
        /// height that carries it. ADDITIVE beside every mode above — it rewrites
        /// the block the other modes produced rather than replacing it, so a
        /// `None` (every existing caller) leaves the chain byte for byte. See
        /// [`IronwoodReceipt`].
        pub(crate) ironwood_receipt: Option<IronwoodReceipt>,
        /// S15-F1 phase A: a wall-clock pause before each pool's root stream is
        /// served — a slow link's root phase, so a row can prove `roots_ms` is
        /// measured (a fake's root phase is otherwise ~0 ms, which proves
        /// nothing). `None` (every existing caller) serves at once.
        pub(crate) roots_delay: Option<std::time::Duration>,
        /// S15-F1 phase A: the same pause before the tip is answered, so a row
        /// can prove `tip_ms` is measured. `None` answers at once.
        pub(crate) tip_delay: Option<std::time::Duration>,
    }

    /// A chain fork the [`FakeChain`] serves once revealed: from the
    /// `after_ranges`-th `block_range` call on, every block at height ≥ `from`
    /// carries [`forked_id`] and its successors chain from it, while block `from`
    /// itself still chains from the ORIGINAL block below it (`block_id(from-1)`),
    /// and `tree_state` at a forked height carries the forked hash. A wallet that
    /// scanned the original blocks past `from` then meets a block whose
    /// `prev_hash` is not the hash it holds — upstream's `PrevHashMismatch`
    /// (`scanning/compact.rs`, checked against the wallet's stored block), the
    /// continuity error `scan_batch` classifies as a reorg — rewinds
    /// `REWIND_DISTANCE_BLOCKS`, and re-scans clean once the rewind has landed
    /// below `from` (the fork's first block chains from what the wallet holds).
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct Fork {
        pub(crate) at: ForkAt,
        pub(crate) after_ranges: usize,
    }

    /// Where a [`Fork`] starts.
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum ForkAt {
        /// An absolute height.
        Height(u64),
        /// This many blocks below the END (inclusive) of the FIRST range the fake
        /// served — so the fork starts inside the first batch the loop scanned,
        /// whatever geometry upstream's scan queue chose for it (the 10-block
        /// `Verify` lookahead above a freshly-scanned tip, or a full 100-block
        /// batch), and the first block of the NEXT batch is the one that no
        /// longer chains from what the wallet holds.
        BelowFirstRangeEnd(u64),
    }

    /// The forked chain's id for `height` — [`block_id`] with the last byte set.
    pub(crate) fn forked_id(height: u64) -> Vec<u8> {
        let mut v = block_id(height);
        v[31] = 0xF0;
        v
    }

    /// [`compact_block`] on the fork described in [`Fork`]: unchanged below
    /// `fork_from`; at `fork_from` the forked hash over the original parent; above
    /// it the forked chain.
    pub(crate) fn forked_block(height: u64, fork_from: u64) -> CompactBlock {
        let mut b = compact_block(height);
        if height >= fork_from {
            b.hash = forked_id(height);
            if height > fork_from {
                b.prev_hash = forked_id(height - 1);
            }
        }
        b
    }

    // ── The REPEATED fork (§4q-R RR-3; §4q-run owed row 1, Q-R2's named seam) ──
    //
    // [`Fork`] reveals ONE fork and only one: `revealed_fork` filters a single
    // `Option<Fork>` and `forked_block` tags every height at or above it with the
    // SAME [`forked_id`], so once the wallet has rewound below that height the
    // fork links from what it holds and no second continuity error is producible.
    // The test author named the seam from `FakeChain::block_range`'s mapping
    // closure ("`FakeChain.fork` becomes a sequence, `revealed_fork` returns the
    // currently-revealed one, and `forked_id`/`forked_block` take a generation")
    // and did not build it; this is that seam, built for the row that needs a
    // server which can keep ONE pass rewinding: a fork, an advance, a fork again.
    //
    // WHY A GENERATION AND NOT A `Vec<Fork>` OF THE OLD SHAPE. The ids must be
    // MONOTONE in the fork list: adding a fork at `F` may change the id of a
    // height at or above `F` and MUST NOT change the id of any height below it.
    // Re-serving `compact_block` (the original id) below a new fork would put the
    // fake's own blocks at odds with the ones the wallet already holds under an
    // OLDER generation — every re-scan after a rewind would then trip a
    // continuity error the fixture never intended, cascade downward, and trip the
    // per-epoch storm bound instead of the absolute cap this row is about. So a
    // height's id is decided by the HIGHEST revealed fork at or below it
    // ([`storm_id`]) and nothing else.

    /// A server that forks, lets the wallet advance, and forks again — the
    /// geometry §4q-R P-RR3's absolute per-pass rewind cap exists for (the fold
    /// review's row 2: `reorgs_since_progress` resets on any new high water, so
    /// an endpoint that pays one advance per fork is bounded only by the tip it
    /// itself chose).
    ///
    /// The next generation is revealed on the first `block_range` whose START is
    /// at least [`Self::advance_blocks`] above the current generation's fork
    /// height, and it forks at `start - 1`: the first block of THAT range no
    /// longer chains from the block the wallet holds below it, so the range is a
    /// continuity error, the pass rewinds, re-queues, re-scans across the fork —
    /// and the next range that climbs far enough forks again. The advance between
    /// two forks is what makes this geometry different from a plain reorg storm:
    /// each cycle pushes a NEW high-water frontier, which RESETS
    /// `reorgs_since_progress`, so [`MAX_SCAN_REORGS_PER_PASS`] never trips and
    /// only an absolute per-pass bound can end the pass.
    ///
    /// [`Self::max_generations`] is a HARD stop: after that many forks the chain
    /// goes honest and the wallet reaches the tip, so a build with no absolute cap
    /// FAILS the row instead of spinning the fixture forever.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct ForkStorm {
        /// How far above the current fork a requested range must start before the
        /// next fork is revealed.
        pub(crate) advance_blocks: u64,
        /// The most forks this endpoint will ever serve (the fixture's bound).
        pub(crate) max_generations: usize,
    }

    /// The id of `height` on the branch created by the `generation`-th revealed
    /// fork (1-based): [`forked_id`] with the generation in byte 30. Distinct
    /// from [`block_id`] (byte 31 clear), from [`forked_id`] (byte 30 clear —
    /// the single-fork fixture is generation 0 and is untouched by this seam),
    /// and from every other generation.
    pub(crate) fn generation_id(height: u64, generation: usize) -> Vec<u8> {
        let mut v = forked_id(height);
        v[30] = u8::try_from(generation).expect("a fixture serves few generations");
        v
    }

    /// The id of `height` on the chain a [`ForkStorm`] has served up to `forks`:
    /// the generation of the HIGHEST fork at or below `height`, or the original
    /// [`block_id`] when no fork has reached that far down. Monotone by
    /// construction — revealing a fork at `F` leaves every height below `F`
    /// exactly as it was, which is what lets a wallet re-scan across an older
    /// fork after a rewind without meeting an error the fixture did not mean.
    pub(crate) fn storm_id(height: u64, forks: &[u64]) -> Vec<u8> {
        match forks.iter().rposition(|&f| height >= f) {
            Some(i) => generation_id(height, i + 1),
            None => block_id(height),
        }
    }

    /// [`compact_block`] on the chain a [`ForkStorm`] has served up to `forks`:
    /// both the block's own id and its parent's are [`storm_id`], so the served
    /// stream is internally consistent at every fork boundary and mismatches ONLY
    /// against a block the wallet already holds from an older generation.
    pub(crate) fn storm_block(height: u64, forks: &[u64]) -> CompactBlock {
        let mut b = compact_block(height);
        b.hash = storm_id(height, forks);
        b.prev_hash = storm_id(height.saturating_sub(1), forks);
        b
    }

    // ── A fork at EVERY pass head (§4u RW-4; the REQ-1-R fold review's row 1) ──
    //
    // [`ForkStorm`] keys its generations to RANGE STARTS over a tip that never
    // moves, so a storm is spent inside the pass that meets it: the wallet
    // climbs to the tip (or the cap stalls it), the next pass over the same
    // chain asks for nothing new, and no LATER pass can be made to rewind. The
    // §4u rows need the other server — one that rewinds a controller's
    // successive passes, each once, so that what happens BETWEEN passes (the
    // backoff ladder, the streak) is what the row measures. That is a fork keyed
    // to the pass head — `latest_block_height`, the one tip fetch a pass makes
    // (`sync::fetch_tip`) — with a tip that moves, so every pass has a range to
    // ask for and its first block is the one that no longer chains.
    //
    // Read only when neither `fork` nor `storm` is set, like `storm` beside
    // `fork`: the three modes never mix, and every existing caller is untouched.

    /// A server that forks once per pass. Every pass-head tip fetch moves the
    /// reported tip up by [`Self::tip_step`] and reveals a new branch from the
    /// tip it reported LAST time — the height a wallet that completed the
    /// previous pass holds — so the first block above what the wallet holds
    /// carries a parent hash the wallet does not have: one continuity error,
    /// one rewind, a clean re-scan across the fork to the new tip, and the next
    /// pass head forks again. After [`Self::max_passes`] forks the chain goes
    /// honest — the tip stops and no further branch appears — so a build that
    /// never counts the streak ends the row up to date instead of spinning it.
    ///
    /// Ids come from [`storm_id`] over the fork heights served so far, the
    /// generation list a [`ForkStorm`] uses, so the branches are monotone for
    /// the same reason: revealing a fork at `F` changes no id below `F`, and a
    /// wallet re-scanning across an older fork after its rewind meets no error
    /// the fixture did not mean. The identity a pass reads BEFORE its tip fetch
    /// (`evaluate_consensus`, the `ChainOracle` read) sees the tip as it stood
    /// at the previous pass head — the shape of two RPCs racing a moving chain on
    /// a real server, and on this fixture the same branch either way.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct PassForks {
        /// How far the reported tip moves at every pass head that forks.
        pub(crate) tip_step: u64,
        /// How many pass heads fork before the chain goes honest (the fixture's
        /// bound).
        pub(crate) max_passes: usize,
    }

    impl FakeChain {
        pub(crate) fn to_tip(tip: u64) -> Self {
            Self {
                tip,
                ranges: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
                download_gate: None,
                block_range_err_at: None,
                tip_err: None,
                fork: None,
                storm: None,
                pass_forks: None,
                pass_fork_heights: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
                pass_heads: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
                tree_states: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
                tree_state_override: None,
                tree_state_lies: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
                tree_state_byte_order_flip: None,
                tree_state_frontier_lie: None,
                ironwood_receipt: None,
                roots_delay: None,
                tip_delay: None,
            }
        }

        /// The fork's first height, once revealed by the number of `block_range`
        /// calls made so far; `None` while it is not.
        fn revealed_fork(&self) -> Option<u64> {
            let ranges = self.ranges.lock().expect("ranges");
            let fork = self.fork.filter(|f| ranges.len() > f.after_ranges)?;
            Some(match fork.at {
                ForkAt::Height(h) => h,
                ForkAt::BelowFirstRangeEnd(below) => ranges.first()?.1.saturating_sub(below),
            })
        }

        /// Every fork a [`ForkStorm`] has revealed so far, ascending — replayed
        /// from the ranges served, so it is a pure function of what the loop has
        /// asked for and gives the same answer however often it is called (the
        /// current range is already recorded when this runs, so the range that
        /// EARNS a fork is the one served on it). Empty when no storm is
        /// configured, and empty while [`FakeChain::fork`] is set: the two modes
        /// never mix.
        pub(crate) fn storm_forks(&self) -> Vec<u64> {
            let Some(storm) = self.storm.filter(|_| self.fork.is_none()) else {
                return Vec::new();
            };
            let ranges = self.ranges.lock().expect("ranges");
            let mut forks: Vec<u64> = Vec::new();
            for &(start, _) in ranges.iter() {
                if forks.len() >= storm.max_generations {
                    break;
                }
                let due = forks
                    .last()
                    .is_none_or(|&f| start >= f.saturating_add(storm.advance_blocks));
                if due {
                    forks.push(start.saturating_sub(1));
                }
            }
            forks
        }

        /// Every fork the per-pass mode ([`PassForks`]) has revealed so far,
        /// ascending — the generation list [`storm_id`] reads. Empty unless a
        /// `PassForks` is configured and neither [`Self::fork`] nor
        /// [`Self::storm`] is: the modes never mix.
        pub(crate) fn pass_forks_revealed(&self) -> Vec<u64> {
            if self.pass_forks.is_none() || self.fork.is_some() || self.storm.is_some() {
                return Vec::new();
            }
            self.pass_fork_heights.lock().expect("pass forks").clone()
        }

        /// One pass head under [`PassForks`]: fork at the tip reported last
        /// time, then move the tip. A no-op outside the mode and once
        /// `max_passes` forks have been served (the chain has gone honest).
        fn serve_pass_head(&mut self) {
            let Some(mode) = self
                .pass_forks
                .filter(|_| self.fork.is_none() && self.storm.is_none())
            else {
                return;
            };
            let mut forks = self.pass_fork_heights.lock().expect("pass forks");
            if forks.len() >= mode.max_passes {
                return;
            }
            forks.push(self.tip);
            self.tip = self.tip.saturating_add(mode.tip_step);
        }
    }

    #[async_trait]
    impl ScanClient for FakeChain {
        async fn block_range(
            &mut self,
            start: u64,
            end_inclusive: u64,
        ) -> Result<BlockStream, GrpcError> {
            self.ranges
                .lock()
                .expect("ranges")
                .push((start, end_inclusive));
            if self.block_range_err_at == Some(start) {
                self.block_range_err_at = None; // heal for the retry
                return Err(GrpcError::Transport {
                    stall: StallReason::EndpointUnreachable,
                });
            }
            if let Some((started, release)) = self.download_gate.take() {
                started
                    .send((start, end_inclusive))
                    .expect("download gate: the test's started-receiver is alive");
                release
                    .await
                    .expect("download gate: the test releases the gate");
            }
            let fork_from = self.revealed_fork();
            // §4q-R RR-3: the repeated-fork mode rides the SAME mapping closure,
            // as the seam was named from it. `storm_forks` is empty unless a
            // `ForkStorm` is configured (and `fork` is not), so the single-fork
            // and honest paths below are unchanged.
            let storm_forks = self.storm_forks();
            // §4u RW-4: the per-pass mode rides the same closure and the same
            // ids (`storm_block` over its own fork list); empty unless a
            // `PassForks` is configured alone, so the three paths above it are
            // unchanged.
            let pass_forks = self.pass_forks_revealed();
            // T0-3 (§4z): a real receipt carried by ONE block of the otherwise
            // empty chain. Applied last so it rides whatever the fork modes
            // above produced for that height; a `None` leaves every block byte
            // for byte as it was.
            let receipt = self.ironwood_receipt.clone();
            let blocks: Vec<_> = (start..=end_inclusive)
                .map(|h| {
                    let block = match fork_from {
                        Some(from) => forked_block(h, from),
                        None if !storm_forks.is_empty() => storm_block(h, &storm_forks),
                        None if !pass_forks.is_empty() => storm_block(h, &pass_forks),
                        None => compact_block(h),
                    };
                    Ok(Some(with_ironwood_receipt(block, receipt.as_ref())))
                })
                .collect();
            Ok(scripted_stream(blocks))
        }
        async fn tree_state(&mut self, height: u64) -> Result<TreeState, GrpcError> {
            // SCAN-2: the receipt first, so an overridden answer is counted too.
            self.tree_states.lock().expect("tree_states").push(height);
            if let Some((from, lie)) = &self.tree_state_override
                && height >= *from
            {
                // What was SERVED, recorded where a row can read it.
                self.tree_state_lies
                    .lock()
                    .expect("tree_state_lies")
                    .push(height);
                let mut lie = lie.clone();
                lie.height = height;
                return Ok(lie);
            }
            let mut ts = chain_tree_state(height);
            // SCAN-2 option (c): the honest state in the OTHER hash byte-order
            // convention. Applied to `chain_tree_state`'s own hash (not to a fork's),
            // so it composes with nothing and stays exactly "one field, reversed".
            if let Some(from) = self.tree_state_byte_order_flip
                && height >= from
            {
                self.tree_state_lies
                    .lock()
                    .expect("tree_state_lies")
                    .push(height);
                // DELIBERATELY NOT `display_order_hex` — this line's whole job is to
                // MODEL the wrong convention, so routing it through the predicate would
                // make the flip a silent no-op. The helper's doc invites a grep sweep for
                // bare `hex::encode` block-id sites (four have been real bugs); this is
                // the one that must survive it. The code review asked for this line.
                ts.hash = hex::encode(block_id(height));
            }
            // SCAN-2 option (c): the frontier twin — the honest hash, a foreign
            // Sapling frontier. Recorded as a served disagreement like the flip.
            if let Some((from, frontier)) = &self.tree_state_frontier_lie
                && height >= *from
            {
                self.tree_state_lies
                    .lock()
                    .expect("tree_state_lies")
                    .push(height);
                ts.sapling_tree = frontier.clone();
            }
            // The three fork paths hex their ids in DISPLAY order, the same way
            // `chain_tree_state` does — `to_chain_state` reverses unconditionally, so
            // the reversal here is what makes the served hash equal the forked block's
            // OWN hash rather than its byte-reverse. Found by the crypto audit,
            // which measured that no existing fork row reaches the 20-batch reconcile
            // cadence (`reorg_passes` is 4 batches, `ForkStorm.advance_blocks` 100,
            // `PassForks.tip_step` one batch) — so nothing red today, and that is
            // exactly the problem: the next reorg row that scans past the cadence would
            // have found a forked server serving the byte-order quirk, taken option
            // (c)'s BENIGN arm instead of `anchor_mismatch`, and read clean off
            // `tree_state_lies` (these paths record no lie). The
            // fixture-that-disabled-the-check shape, pre-planted. Cheaper to fix than
            // to carry.
            if let Some(from) = self.revealed_fork()
                && height >= from
            {
                ts.hash = display_order_hex(forked_id(height));
            }
            let storm_forks = self.storm_forks();
            if !storm_forks.is_empty() {
                ts.hash = display_order_hex(storm_id(height, &storm_forks));
            }
            let pass_forks = self.pass_forks_revealed();
            if !pass_forks.is_empty() {
                ts.hash = display_order_hex(storm_id(height, &pass_forks));
            }
            Ok(ts)
        }
        async fn latest_block_height(&mut self) -> Result<u64, GrpcError> {
            self.pass_heads
                .lock()
                .expect("pass heads")
                .push(tokio::time::Instant::now());
            if let Some(pause) = self.tip_delay {
                tokio::time::sleep(pause).await;
            }
            match self.tip_err.take() {
                Some(e) => Err(e),
                None => {
                    // §4u RW-4: under `PassForks` a pass head forks and moves
                    // the tip; a no-op in every other mode.
                    self.serve_pass_head();
                    Ok(self.tip)
                }
            }
        }
    }

    #[async_trait]
    impl SubtreeRootSource for FakeChain {
        // Serves no roots for any pool, so every start is honoured trivially.
        async fn subtree_roots(
            &mut self,
            _protocol: ShieldedProtocol,
            _start_index: u32,
        ) -> Result<SubtreeRootStream, GrpcError> {
            if let Some(pause) = self.roots_delay {
                tokio::time::sleep(pause).await;
            }
            Ok(scripted_stream::<SubtreeRoot>([]))
        }
    }

    // FakeChain is a full SCAN + PROVISION client (the LightdSyncEngine drives both):
    // an honest TESTNET endpoint (the driver-loop tests are Test-network), so
    // `provision_account` resolves the birthday + imports account 0 over it with NO
    // gRPC server.
    use zcash_protocol::consensus::{BlockHeight as ConsensusHeight, BranchId};

    #[async_trait]
    impl crate::provision::ChainOracle for FakeChain {
        async fn tip_height(&mut self) -> Result<u64, GrpcError> {
            Ok(self.tip)
        }
        async fn server_identity(&mut self) -> Result<crate::provision::ServerIdentity, GrpcError> {
            Ok(crate::provision::ServerIdentity {
                chain_name: crate::checkpoints::chain_name(Network::Test).to_owned(),
                sapling_activation_height: u64::from(crate::checkpoints::activation_height(
                    Network::Test,
                )),
                // An honest TESTNET endpoint: the branch our compiled params
                // compute at this fake's tip, so a testnet-driven pass reaches
                // a `Current` verdict instead of tripping the staleness gate on
                // an unrelated test.
                consensus_branch_id: Some(u32::from(BranchId::for_height(
                    &Network::Test.consensus(),
                    ConsensusHeight::from_u32(u32::try_from(self.tip).unwrap_or(u32::MAX)),
                ))),
                block_height: self.tip,
            })
        }
    }

    /// A TreeState at `height` with empty pools (the genesis-anchor shape — decodes
    /// to an empty-frontier `ChainState` at `height`, the same shape
    /// `account::birthday_from_treestate` uses for an activation birthday).
    pub(crate) fn tree_state_at(height: u64) -> TreeState {
        TreeState {
            network: "test".to_owned(),
            height,
            hash: "00".repeat(32),
            time: 0,
            sapling_tree: String::new(),
            orchard_tree: String::new(),
            // Empty is correct below the NU6.3 activation — see `chain_tree_state`.
            ironwood_tree: String::new(),
        }
    }

    /// A scripted [`ScanClient`] — no live gRPC server (the sibling of
    /// [`FakeRootSource`]). `block_open_err`/`tree_state_err` inject a transport
    /// fault when OPENING the block stream / fetching the tree state; the block
    /// items are a scripted stream (mid-stream `Err` faults included).
    pub(crate) struct FakeScanClient {
        pub(crate) blocks: Vec<Result<Option<CompactBlock>, tonic::Status>>,
        pub(crate) block_open_err: Option<GrpcError>,
        pub(crate) tree_state: Option<TreeState>,
        pub(crate) tree_state_err: Option<GrpcError>,
        /// The `(start, end_inclusive)` the last `block_range` call requested — lets
        /// a test assert the half-open→inclusive wire conversion (`end - 1`).
        pub(crate) last_block_range: Option<(u64, u64)>,
    }

    impl FakeScanClient {
        fn new() -> Self {
            Self {
                blocks: Vec::new(),
                block_open_err: None,
                tree_state: None,
                tree_state_err: None,
                last_block_range: None,
            }
        }
        /// Serves the given block stream (the download tests).
        pub(crate) fn with_blocks(
            blocks: Vec<Result<Option<CompactBlock>, tonic::Status>>,
        ) -> Self {
            Self {
                blocks,
                ..Self::new()
            }
        }
        /// Serves the given tree state (the anchor-fetch tests).
        pub(crate) fn with_tree_state(tree_state: TreeState) -> Self {
            Self {
                tree_state: Some(tree_state),
                ..Self::new()
            }
        }
    }

    #[async_trait]
    impl ScanClient for FakeScanClient {
        async fn block_range(
            &mut self,
            start: u64,
            end_inclusive: u64,
        ) -> Result<BlockStream, GrpcError> {
            self.last_block_range = Some((start, end_inclusive));
            match self.block_open_err.take() {
                Some(e) => Err(e),
                None => Ok(scripted_stream(std::mem::take(&mut self.blocks))),
            }
        }
        async fn tree_state(&mut self, _height: u64) -> Result<TreeState, GrpcError> {
            match self.tree_state_err.take() {
                Some(e) => Err(e),
                None => Ok(self
                    .tree_state
                    .clone()
                    .expect("FakeScanClient: tree_state not set")),
            }
        }
        async fn latest_block_height(&mut self) -> Result<u64, GrpcError> {
            // Not exercised by the download/anchor unit tests; the chain-tip path is
            // tested via `fetch_tip` over `FakeChain`. A fixed value keeps the trait
            // satisfied without a misleading magic number on the download fake.
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{FakeRootSource, canonical_root};
    use super::*;

    // testnet Sapling activation — a fresh keyed DB needs no account for the
    // shardtree write (the commitment tree is global, not per-account).
    use crate::money::Network;
    use crate::seal::WalletDbKey;
    use tempfile::tempdir;

    fn fresh_conn(dir: &std::path::Path) -> (WalletConn, WalletDbKey) {
        let path = dir.join("wallet.db");
        let key = WalletDbKey::generate();
        crate::db::provision_db(&path, &key, Network::Test, None).expect("provision");
        let conn = crate::db::open_db_migrated(&path, &key, Network::Test, None).expect("open");
        (conn, key)
    }

    /// The SECOND keyed connection onto the same wallet file — the production
    /// `store::OpenWallet::aux_db` shape, which `scan_batch` needs so its reorg arm
    /// can record the rewind in the A11 bind's ledger (T0-1a-R). A real connection
    /// rather than a stub on purpose: a fake here would let a `scan_batch` that never
    /// writes the ledger pass every test in this module.
    fn fresh_aux(dir: &std::path::Path, key: &WalletDbKey) -> rusqlite::Connection {
        crate::db::open_existing_keyed_connection(&dir.join("wallet.db"), key)
            .expect("a second keyed connection onto the wallet db")
    }

    #[test]
    fn parse_subtree_root_decodes_canonical_and_rejects_hostile_shapes() {
        // §8: a 32-byte canonical root decodes to the right height + hash for BOTH
        // pools; a wrong length, a non-canonical field element, and a height beyond
        // u32 are each a typed reject (never a panic / never a silent truncate).
        let h = 1_950_000u64;
        let ok = canonical_root(h, 0x07);

        let s = parse_sapling_root(&ok).expect("canonical sapling root decodes");
        assert_eq!(u64::from(s.subtree_end_height()), h);
        assert_eq!(
            s.root_hash().to_bytes()[0],
            0x07,
            "round-trips the leaf bytes"
        );
        let o = parse_orchard_root(&ok).expect("canonical orchard root decodes");
        assert_eq!(u64::from(o.subtree_end_height()), h);

        // (CommitmentTreeRoot is not PartialEq — assert the reject via matches!)
        // wrong length (31 and 33 bytes)
        for len in [31usize, 33] {
            let mut bad = canonical_root(h, 1);
            bad.root_hash = vec![0u8; len];
            assert!(
                matches!(parse_sapling_root(&bad), Err(SubtreeRootInvalid)),
                "sapling len {len}"
            );
            assert!(
                matches!(parse_orchard_root(&bad), Err(SubtreeRootInvalid)),
                "orchard len {len}"
            );
        }

        // non-canonical field element (all-0xFF > both field moduli)
        let mut nc = canonical_root(h, 0);
        nc.root_hash = vec![0xFFu8; 32];
        assert!(matches!(parse_sapling_root(&nc), Err(SubtreeRootInvalid)));
        assert!(matches!(parse_orchard_root(&nc), Err(SubtreeRootInvalid)));

        // completing height beyond u32 — rejected, never truncated
        let mut hi = canonical_root(0, 1);
        hi.completing_block_height = u64::from(u32::MAX) + 1;
        assert!(matches!(parse_sapling_root(&hi), Err(SubtreeRootInvalid)));
        assert!(matches!(parse_orchard_root(&hi), Err(SubtreeRootInvalid)));
    }

    #[tokio::test]
    async fn collect_roots_drains_in_order_and_empty_is_ok() {
        // §8: a stream of roots collects in order; an empty stream (the quiet
        // tip-follow case) is an empty Vec, not an error.
        let mut s = crate::net::grpc::testing::scripted_stream([
            Ok(Some(canonical_root(1_900_000, 1))),
            Ok(Some(canonical_root(1_950_000, 2))),
        ]);
        let roots = collect_sapling_roots(&mut s).await.expect("collects");
        assert_eq!(roots.len(), 2);
        assert_eq!(u64::from(roots[0].subtree_end_height()), 1_900_000);
        assert_eq!(u64::from(roots[1].subtree_end_height()), 1_950_000);

        let mut empty = crate::net::grpc::testing::scripted_stream::<SubtreeRoot>([]);
        assert!(
            collect_orchard_roots(&mut empty)
                .await
                .expect("ok")
                .is_empty(),
            "an empty subtree-root stream is an empty Vec, not an error"
        );
    }

    #[tokio::test]
    async fn collect_roots_malformed_root_is_typed_sync_not_panic() {
        // §8 (hostile input): a non-canonical root mid-stream surfaces a typed
        // `Sync { EndpointMisbehaving }` (the endpoint answered, wrongly — the
        // content-tier class, T0-1b), never a panic and never a silently-dropped root.
        let mut bad = canonical_root(1_900_000, 1);
        bad.root_hash = vec![0xFFu8; 32];
        let mut s = crate::net::grpc::testing::scripted_stream([
            Ok(Some(canonical_root(1_850_000, 1))),
            Ok(Some(bad)),
        ]);
        assert!(
            matches!(
                collect_sapling_roots(&mut s).await,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "a malformed root is a typed Sync error"
        );
    }

    #[tokio::test]
    async fn collect_roots_transport_fault_preserves_stall() {
        // §8 (unstable-network): a transport fault mid-stream surfaces a typed
        // `Sync { stall }` carrying the stream's policy stall (here
        // `EndpointUnreachable`) — never a silent clean end. (The fail-closed
        // `TorUnavailable` identity surviving the tonic erasure is pinned on the
        // stream-OPEN path — `fetch_subtree_roots_orchard_open_failure_writes_nothing`
        // — and on the unary path; the classify+map path is the same regardless of
        // WHEN the fault lands.)
        let mut s = crate::net::grpc::testing::scripted_stream([
            Ok(Some(canonical_root(1_850_000, 1))),
            Err(tonic::Status::unavailable("transport gone")),
        ]);
        assert!(matches!(
            collect_sapling_roots(&mut s).await,
            Err(WalletError::Sync {
                stall: StallReason::EndpointUnreachable
            })
        ));
    }

    #[tokio::test]
    async fn fetch_subtree_roots_orchard_open_failure_writes_nothing() {
        // §8 (money / unstable-network): Sapling streams fine but the Orchard RPC
        // fails to open — `fetch_subtree_roots` returns `Err` BEFORE any DB write,
        // so nothing is half-written (the next sync re-fetches from 0). The typed
        // stall is preserved.
        // T0-1: widened to the three-pool `FakeRootSource` shape (assumed seam
        // S-2, documented in the T0-1 block below). Behaviour unchanged.
        let mut fake =
            three_pool_fake(vec![Ok(Some(canonical_root(1_900_000, 1)))], vec![], vec![]);
        fake.orchard_open_err = Some(GrpcError::Timeout {
            stall: StallReason::TorUnavailable,
        });
        assert!(
            matches!(
                fetch_subtree_roots(&mut fake, Network::Test, BlockHeight::from_u32(2_000_000))
                    .await,
                Err(WalletError::Sync {
                    stall: StallReason::TorUnavailable
                })
            ),
            "an Orchard-pool fault aborts the whole fetch with the stall preserved"
        );
    }

    #[tokio::test]
    async fn put_subtree_roots_writes_both_pools_and_is_idempotent() {
        // §8: the collected roots write through a REAL keyed WalletDb, and a re-put
        // from index 0 (what every subsequent sync does) is idempotent — Ok, not a
        // conflict — so re-sync is safe (the unstable-network retry property).
        //
        // T0-1 A6, PARTIALLY. The contract asks for this guard to be extended to
        // three pools "PLUS this spendability assertion — the idempotence half
        // alone would pass on a wiring that never fixes anything". The three-pool
        // half is here.
        //
        // ⚠️ THE SPENDABILITY HALF IS NOT IN THIS TREE AND IS OWED. The author's
        // draft of this comment sent the reader to `mod ironwood_spendability
        // below`; ADJUDICATOR — that module does not exist anywhere in the
        // repository (`grep -rn ironwood_spendability sdk/` matches only this
        // comment), so the sentence pointed at coverage nobody wrote, on the one
        // assertion the contract calls "the end-to-end assertion, and it is the
        // one that matters". The author's own report says NOT COVERED; the tree
        // said the opposite, and a session reads the tree.
        //
        // WHY it is owed rather than missing by oversight, so nobody re-derives
        // it: A6's fixture needs an Ironwood note above a COMPLETED subtree with a
        // gap below it, and `zcash_client_backend-0.24.0`'s `InitialChainState`
        // has `prior_sapling_roots` and `prior_orchard_roots` and NO
        // `prior_ironwood_roots` (`testing.rs:1711-1719`); `TestBuilder::build()`
        // writes only those two (`:2052`, `:2069`). Building it needs
        // `incrementalmerkletree` as a dev-dependency — a supply-chain decision
        // this crate has not taken — plus a hand-generated consistent
        // (prior_roots, frontier) pair. See the T0-1 ruling, §5.1 and OWED #3.
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());

        // ADJUDICATOR (fixture repair, no assertion changed): the test author
        // served the Ironwood root at 1,900,000, the height the two pre-existing
        // pools use. Testnet NU6.3 activates at 4,134,000, so an Ironwood subtree
        // completing at 1,900,000 is a claim the chain cannot support and A11's
        // activation floor refuses it — the same class of correction the author
        // itself applied in `wallet.rs` (where it left the stream EMPTY for this
        // exact reason) and the implementer applied to the 279,000 Sapling
        // fixtures. Height moved to a post-activation one; the `tip` ceiling moves
        // with it so the ceiling clause is not what is being exercised here.
        let ironwood_h = nu63_activation(Network::Test) + 10_000;
        let mut fake = honest_three(
            vec![Ok(Some(canonical_root(1_900_000, 1)))],
            vec![Ok(Some(canonical_root(1_900_000, 2)))],
            vec![Ok(Some(canonical_root(ironwood_h, 3)))],
        );
        let roots = fetch_subtree_roots(
            &mut fake,
            Network::Test,
            BlockHeight::from_u32(u32::try_from(ironwood_h + 100_000).expect("tip fits u32")),
        )
        .await
        .expect("fetch");
        assert_eq!(roots.sapling.len(), 1);
        assert_eq!(roots.orchard.len(), 1);
        assert_eq!(roots.ironwood.len(), 1, "the third pool is collected too");

        put_subtree_roots(&mut conn, &roots, |_| Ok(())).expect("first write");
        put_subtree_roots(&mut conn, &roots, |_| Ok(())).expect("idempotent re-put from index 0");

        // an empty set is a valid no-op write (the quiet tip-follow case)
        let empty = SubtreeRoots::from_pools(Vec::new(), Vec::new(), Vec::new());
        put_subtree_roots(&mut conn, &empty, |_| Ok(())).expect("empty write is a no-op");
    }

    /// security review (HIGH): the drain's count cap bounds how many
    /// `completing_block_hash` entries are retained, not how big one is — each is
    /// bounded only by the per-message wire cap, so an endpoint padding the field
    /// would sit resident at count × message-size before any validation ran. The
    /// 32-byte filter therefore lives at the BOUNDARY. Nothing behavioural
    /// changes — `root_bind::check_completing_hashes` abstains on any wrong-length
    /// entry either way — the ALLOCATION is what this pins.
    #[tokio::test]
    async fn an_oversized_completing_block_hash_is_dropped_at_the_boundary() {
        let mut oversized = canonical_root(1_900_000, 1);
        oversized.completing_block_hash = vec![0xAB; 8 * 1024];
        let mut runt = canonical_root(1_900_500, 2);
        runt.completing_block_hash = vec![0xCD; 8]; // wrong in the other direction
        let mut wellformed = canonical_root(1_901_000, 3);
        wellformed.completing_block_hash = testing::block_id(1_901_000);

        let mut fake = honest_three(
            vec![Ok(Some(oversized)), Ok(Some(runt)), Ok(Some(wellformed))],
            Vec::new(),
            Vec::new(),
        );
        let roots = fetch_subtree_roots(&mut fake, Network::Test, BlockHeight::from_u32(2_000_000))
            .await
            .expect("an odd-length hash abstains; it must not refuse the fetch");
        assert_eq!(roots.sapling.len(), 3);
        let hashes = &roots.completing_hashes[0]; // SAPLING_SLOT
        assert_eq!(
            hashes[0].len(),
            0,
            "the 8 KiB entry is dropped at the boundary, not retained"
        );
        assert_eq!(hashes[1].len(), 0, "so is the 8-byte runt");
        assert_eq!(
            hashes[2].len(),
            32,
            "and the well-formed sibling is retained verbatim"
        );
    }

    #[tokio::test]
    async fn fetch_subtree_roots_orchard_mid_drain_fault_returns_typed_sync() {
        // §8 (unstable-network 3-lens): Sapling opens + drains FULLY (no fault),
        // Orchard opens fine then faults MID-DRAIN (the link drops BETWEEN Orchard
        // items) — distinct from the open-failure arm. `fetch_subtree_roots` must
        // bubble a typed Sync out BEFORE any write (the second-half code path).
        let mut fake = three_pool_fake(
            vec![Ok(Some(canonical_root(1_900_000, 1)))],
            vec![
                Ok(Some(canonical_root(1_900_000, 2))),
                Err(tonic::Status::unavailable("link dropped mid-orchard-drain")),
            ],
            vec![],
        );
        assert!(
            matches!(
                fetch_subtree_roots(&mut fake, Network::Test, BlockHeight::from_u32(2_000_000))
                    .await,
                Err(WalletError::Sync { .. })
            ),
            "a mid-Orchard-drain fault aborts the whole fetch before any write"
        );
    }

    #[test]
    fn sapling_committed_orchard_pending_then_full_reput_converges() {
        // §8 (money 3-lens): a crash AFTER the Sapling put commits but BEFORE Orchard
        // (the per-pool non-atomic write) heals — the next sync re-puts BOTH from
        // index 0 idempotently. Models the half-state by putting Sapling-only, then
        // the full two-pool set.
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());

        // ADJUDICATOR (shape repair): `SubtreeRoots` gained a private `outcomes`
        // field, so the author's struct literal cannot be written; `from_pools` is
        // the implementer's constructor for exactly this case. Same three vectors,
        // in the same order, including the author's added Ironwood half-write
        // coverage. No height touched — this test drives `put_subtree_roots`
        // directly, which validates nothing.
        let half = SubtreeRoots::from_pools(
            vec![parse_sapling_root(&canonical_root(279_000, 1)).expect("ok")],
            Vec::new(), // Orchard never committed (the crash window)
            Vec::new(), // nor Ironwood, which is written after it
        );
        put_subtree_roots(&mut conn, &half, |_| Ok(())).expect("sapling-only half-write");

        // the next sync re-fetches ALL THREE from 0 — Sapling re-put is a no-op,
        // Orchard and Ironwood complete; converges with no conflict.
        let full = SubtreeRoots::from_pools(
            vec![parse_sapling_root(&canonical_root(279_000, 1)).expect("ok")],
            vec![parse_orchard_root(&canonical_root(279_000, 2)).expect("ok")],
            vec![parse_orchard_root(&canonical_root(279_000, 3)).expect("ok")],
        );
        put_subtree_roots(&mut conn, &full, |_| Ok(()))
            .expect("full re-put after the half-write converges");
    }

    #[test]
    fn mutated_root_reput_is_insert_conflict_not_silent_overwrite() {
        // §8 (money 3-lens): the "only a MUTATION conflicts at put, never a silent
        // overwrite" safety claim the audit depends on. Put a root at index 0, then a
        // DIFFERENT canonical root at the SAME index ⇒ a shardtree `Insert` conflict,
        // surfaced as the typed `Sync { EndpointMisbehaving }` (a lying/buggy endpoint
        // mutating a completed-subtree root — never a silent cap overwrite). The
        // idempotence test above already covers the SAME-value re-put (a no-op).
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());

        let first = SubtreeRoots::from_pools(
            vec![parse_sapling_root(&canonical_root(279_000, 1)).expect("ok")],
            Vec::new(),
            Vec::new(),
        );
        put_subtree_roots(&mut conn, &first, |_| Ok(())).expect("first root at index 0");

        // a DIFFERENT root_hash (tag 2 ≠ tag 1) at the same index 0 + same height
        let mutated = SubtreeRoots::from_pools(
            vec![parse_sapling_root(&canonical_root(279_000, 2)).expect("ok")],
            Vec::new(),
            Vec::new(),
        );
        assert!(
            matches!(
                put_subtree_roots(&mut conn, &mutated, |_| Ok(())),
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "a mutated root at an existing index is a typed Sync conflict, never a silent overwrite"
        );
    }

    // ═══ T0-1 · the pool wiring (A1–A5, A7–A11) ══════════════════════════════
    //
    // ASSUMED SEAMS — the contract (`production-readiness-phase-1.md` §4) names
    // the files and functions but does not fix these signatures. Every one is
    // flagged here and in the test author's report; an adjudicator ruling on a
    // mismatch is ruling on the ASSUMPTION, not on the assertion above it.
    //
    //   S-1  `SubtreeRoots` gains `ironwood: Vec<CommitmentTreeRoot<MerkleHashOrchard>>`.
    //        (NAMED by the contract, A3: "`SubtreeRoots.orchard` and the new
    //        `.ironwood` have the identical Rust type".) Not an assumption.
    //   S-2  `testing::FakeRootSource` gains `ironwood: Vec<Result<Option<SubtreeRoot>,
    //        tonic::Status>>` and `ironwood_open_err: Option<GrpcError>`, mirroring
    //        the two existing pools, and `FakeRootSource::honest` takes a THIRD
    //        vector. The contract requires "a third stream" (A3/A5) but fixes
    //        neither field names nor the constructor arity. ALL construction of
    //        the fake in this block goes through `three_pool_fake` /
    //        `honest_three` below, so a shape mismatch is ONE edit, not thirty.
    //   S-3  `fetch_subtree_roots` keeps its one-argument `(&mut C)` signature.
    //        A11's third clause ("last ≤ the endpoint's own reported tip") names
    //        an input this seam does not have; see the report's findings.
    //
    // The recorder below is MINE, not the implementer's: `SubtreeRootSource` is a
    // crate-internal trait, so a test can implement it without touching the
    // double the implementer owns (IT-6).

    /// A [`SubtreeRootSource`] that RECORDS the wire protocol number of every
    /// request and serves an empty stream for each. The only way to assert what
    /// `fetch_subtree_roots` actually asked the endpoint for.
    struct RecordingSource {
        asked: std::sync::Arc<std::sync::Mutex<Vec<i32>>>,
    }

    #[async_trait]
    impl SubtreeRootSource for RecordingSource {
        // Serves an empty stream for every pool, so every start is honoured.
        async fn subtree_roots(
            &mut self,
            protocol: ShieldedProtocol,
            _start_index: u32,
        ) -> Result<SubtreeRootStream, GrpcError> {
            // `protocol as i32` is byte-for-byte what `LightwalletdClient::
            // get_subtree_roots` puts in `GetSubtreeRootsArg.shielded_protocol`
            // (`net/grpc.rs:293`), so recording it here records the WIRE value,
            // not the variant name.
            self.asked.lock().expect("asked").push(protocol as i32);
            Ok(crate::net::grpc::testing::scripted_stream::<SubtreeRoot>([]))
        }
    }

    /// S-2. The one place the assumed `FakeRootSource` shape is written down.
    fn three_pool_fake(
        sapling: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        orchard: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        ironwood: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
    ) -> FakeRootSource {
        FakeRootSource {
            sapling,
            orchard,
            ironwood,
            sapling_open_err: None,
            orchard_open_err: None,
            ironwood_open_err: None,
        }
    }

    /// S-2, the honest-endpoint sugar.
    fn honest_three(
        sapling: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        orchard: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
        ironwood: Vec<Result<Option<SubtreeRoot>, tonic::Status>>,
    ) -> FakeRootSource {
        FakeRootSource::honest(sapling, orchard, ironwood)
    }

    /// A `Result<SubtreeRoots, WalletError>` reduced to a printable summary.
    /// `SubtreeRoots` is deliberately not required to be `Debug`: demanding a
    /// derive on the implementer's struct so a test can format it would be the
    /// test shaping the implementation.
    fn describe_ref(r: &Result<SubtreeRoots, WalletError>) -> String {
        match r {
            Ok(roots) => format!(
                "Ok(sapling={}, orchard={}, ironwood={})",
                roots.sapling.len(),
                roots.orchard.len(),
                roots.ironwood.len()
            ),
            Err(e) => format!("Err({e:?})"),
        }
    }

    fn describe(r: Result<SubtreeRoots, WalletError>) -> String {
        describe_ref(&r)
    }

    /// The NU6.3 activation height for `net`, read from the compiled params (never
    /// a literal — `checkpoints::tests::ironwood_activation` does the same).
    fn nu63_activation(net: Network) -> u64 {
        use zcash_protocol::consensus::{NetworkUpgrade, Parameters};
        u64::from(u32::from(
            net.consensus()
                .activation_height(NetworkUpgrade::Nu6_3)
                .expect("the pinned params know Nu6_3"),
        ))
    }

    /// A second SQLCipher connection onto the same wallet file — the production
    /// aux-read pattern — so the tests below can observe the SHARD TABLES
    /// directly instead of inferring DB state from a return value.
    fn observe(dir: &std::path::Path, key: &WalletDbKey) -> rusqlite::Connection {
        crate::db::open_existing_keyed_connection(&dir.join("wallet.db"), key)
            .expect("a second keyed connection onto the wallet db")
    }

    /// `MAX(subtree_end_height)` over `<pool>_tree_shards` — the column
    /// `put_shard` provably cannot write (`commitment_tree.rs:572-580`) and only
    /// `put_shard_roots` fills (`:1298-1301`). `None` is the stuck state §3a
    /// measured: `mark_stabilized_notes` opens with `shard.subtree_end_height IS
    /// NOT NULL`, and `tip_shard_end_height` reads exactly this.
    fn max_subtree_end_height(conn: &rusqlite::Connection, pool: &str) -> Option<i64> {
        conn.query_row(
            &format!("SELECT MAX(subtree_end_height) FROM {pool}_tree_shards"),
            [],
            |r| r.get::<_, Option<i64>>(0),
        )
        .expect("the shard table exists in a migrated wallet db")
    }

    fn count_rows(conn: &rusqlite::Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap_or_else(|e| panic!("counting {table}: {e}"))
    }

    /// **S-3, REPAIRED BY THE ADJUDICATOR.** The test author wrote this block
    /// against a one-argument `fetch_subtree_roots(&mut C)` and flagged that shape
    /// as assumption S-3. The implementation takes `(client, network, tip)`: A11's
    /// third clause ("every root ≤ the endpoint's OWN reported tip") names an input
    /// that does not exist at the fetch site otherwise, so the widening is forced
    /// by the contract, not chosen. The author routed every fake through two
    /// helpers precisely so a shape mismatch would be one edit; this is the third.
    ///
    /// **The `tip` is deliberately far ABOVE every fixture height in this block.**
    /// IT-10: for every negative case, the mechanism that refuses it must be the
    /// mechanism the case is named after. A tip anywhere near the fixtures would
    /// let A11's *ceiling* clause refuse cases named for the *ordering* and
    /// *activation* clauses, and the tests would pass while the clause they name
    /// was broken.
    ///
    /// **What this costs, recorded rather than hidden:** because the ceiling is
    /// never binding here, NO test in the joined tree exercises A11's third clause.
    /// The author reported that as F-2 and it is real. See the ruling.
    async fn fetch_three_pools<C: SubtreeRootSource + ?Sized>(
        client: &mut C,
    ) -> Result<SubtreeRoots, WalletError> {
        let far_above = nu63_activation(Network::Test) + 100_000;
        fetch_subtree_roots(
            client,
            Network::Test,
            BlockHeight::from_u32(u32::try_from(far_above).expect("the test tip fits u32")),
        )
        .await
    }

    #[tokio::test]
    async fn fetch_subtree_roots_asks_for_ironwood_on_wire_value_two() {
        // T0-1 A1. The pool must be requested as `ShieldedProtocol::Ironwood`,
        // whose WIRE value is 2 (`zcash_client_backend-0.24.0/src/proto/
        // service.rs:334-338`) — NOT the 4 that `PoolType::Ironwood`
        // (`service.rs:302`) and `proposal::ValuePool::Ironwood`
        // (`proposal.rs:206`) use for the SAME pool. Sending 4 lands on the
        // unrecognized-protocol path, which is a hard error whose text differs by
        // server version, so the failure would look like a flaky endpoint.
        //
        // The assertion is on the recorded `protocol as i32`, which is literally
        // the integer `get_subtree_roots` writes into
        // `GetSubtreeRootsArg.shielded_protocol` — an assertion that only checked
        // "the Ironwood variant" would be blind to the whole defect, which is why
        // the contract says so in as many words.
        let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut rec = RecordingSource {
            asked: asked.clone(),
        };
        fetch_three_pools(&mut rec)
            .await
            .expect("an endpoint serving three empty streams is not an error");

        let asked = asked.lock().expect("asked").clone();
        assert_eq!(
            asked,
            vec![0i32, 1, 2],
            "the three pools must be requested as the WIRE values Sapling=0, \
             Orchard=1, Ironwood=2. A 4 anywhere here is `PoolType`/`ValuePool` \
             leaking into a `ShieldedProtocol` argument; a missing 2 is the pool \
             never being fetched at all (INC-020)."
        );
        // Pin the two enums apart, so the day somebody "simplifies" one into the
        // other the arithmetic above is not the only thing standing in the way.
        assert_eq!(ShieldedProtocol::Ironwood as i32, 2);
        assert_eq!(
            zcash_client_backend::proto::service::PoolType::Ironwood as i32,
            4,
            "`PoolType::Ironwood` is 4 — a different enum with the same name for \
             the same pool. This is the trap A1 exists for."
        );
    }

    #[tokio::test]
    async fn fake_root_source_serves_ironwood_instead_of_firing_its_tripwire() {
        // T0-1 A5. `sync.rs:1243` is a deliberate `unreachable!()` in
        // `testing::FakeRootSource` whose own text says it must be replaced "in
        // the same change as the production fetch/put, never before it". This is
        // the assertion that it WAS: a direct Ironwood request through the double
        // returns a stream instead of panicking.
        //
        // Kept as its own named test rather than left implicit in the tests that
        // happen to drive three pools: if the fake were wired but production
        // still asked for two, every other test here would pass and nothing would
        // say which half was missing.
        let mut fake = honest_three(vec![], vec![], vec![Ok(Some(canonical_root(1_900_000, 3)))]);
        let mut stream = fake
            .subtree_roots(ShieldedProtocol::Ironwood, 0)
            .await
            .expect("the double serves an Ironwood stream");
        let first = stream
            .next_root()
            .await
            .expect("the scripted stream yields")
            .expect("one scripted root");
        assert_eq!(
            first.completing_block_height, 1_900_000,
            "the Ironwood stream serves the Ironwood script, not another pool's"
        );
    }

    #[tokio::test]
    async fn put_subtree_roots_writes_each_pools_root_into_its_own_tree() {
        // T0-1 A2 + A3, and A3 is the one with teeth.
        //
        // `SubtreeRoots.orchard` and `.ironwood` have the IDENTICAL Rust type
        // `CommitmentTreeRoot<MerkleHashOrchard>` (`zcash_client_sqlite` lib.rs
        // :3257 vs :3288 — same `H`, same `ORCHARD_SHARD_HEIGHT`), so
        // `SubtreeRoots { orchard: x, ironwood: x }` COMPILES and
        // `put_ironwood_subtree_roots(0, &roots.orchard)` compiles too. Serving
        // distinct values and asserting they differ does not catch either: both
        // trees would hold Orchard's root and the two served values would still
        // be different from each other.
        //
        // So: READ BACK, per pool, and assert each tree holds ITS OWN pool's
        // served root. Naming the observation, as the contract asks.
        let dir = tempdir().expect("tempdir");
        let (mut conn, key) = fresh_conn(dir.path());

        // Three DISTINCT roots (tags 1/2/3 ⇒ distinct leaf bytes), same height so
        // height cannot be what distinguishes them.
        let h = nu63_activation(Network::Test) + 10_000;
        let mut fake = honest_three(
            vec![Ok(Some(canonical_root(h, 1)))],
            vec![Ok(Some(canonical_root(h, 2)))],
            vec![Ok(Some(canonical_root(h, 3)))],
        );
        let roots = fetch_three_pools(&mut fake).await.expect("fetch");
        assert_eq!(roots.sapling.len(), 1);
        assert_eq!(roots.orchard.len(), 1);
        assert_eq!(
            roots.ironwood.len(),
            1,
            "A1/A2: the Ironwood stream was fetched and collected"
        );

        put_subtree_roots(&mut conn, &roots, |_| Ok(())).expect("three-pool write");

        let orchard_back = conn
            .get_orchard_subtree_root(0)
            .expect("read back orchard root 0")
            .expect("orchard root 0 was written");
        let ironwood_back = conn
            .get_ironwood_subtree_root(0)
            .expect("read back ironwood root 0")
            .expect("A2: ironwood root 0 was written at start_index 0");

        assert_eq!(
            orchard_back.to_bytes()[0],
            2,
            "the ORCHARD tree must hold the root the ORCHARD stream served (tag 2)"
        );
        assert_eq!(
            ironwood_back.to_bytes()[0],
            3,
            "THE CROSS-WIRE ASSERTION. The IRONWOOD tree must hold the root the \
             IRONWOOD stream served (tag 3). Reading tag 2 here means Orchard's \
             roots were written into Ironwood's tree — which compiles, because \
             the two `SubtreeRoots` fields have the identical Rust type, and which \
             `put_subtree_roots(..) == Ok` cannot see."
        );
        assert_ne!(
            orchard_back.to_bytes(),
            ironwood_back.to_bytes(),
            "the two trees must not have ended up with the same root"
        );

        // A2, the start_index half: the row is at shard index 0, exactly as
        // `sync.rs:277-279` already writes the other two.
        let obs = observe(dir.path(), &key);
        let idx: i64 = obs
            .query_row(
                "SELECT MIN(shard_index) FROM ironwood_tree_shards",
                [],
                |r| r.get(0),
            )
            .expect("an ironwood shard row exists");
        assert_eq!(idx, 0, "A2: the ironwood put starts at index 0");

        // Idempotence, extended to three pools (the existing named guard's half).
        put_subtree_roots(&mut conn, &roots, |_| Ok(()))
            .expect("idempotent three-pool re-put from index 0");
    }

    #[tokio::test]
    async fn ironwood_roots_fill_the_column_only_put_shard_roots_can_write() {
        // T0-1 A7(i) + A8, and the load-bearing premise of the whole item
        // measured on OUR code rather than on the Python reproduction.
        //
        // A7(i): after a write that carried at least one completed shard's root,
        // `subtree_end_height` is non-NULL for Ironwood, so
        // `tip_shard_end_height(Ironwood)` (`scanning.rs:582`) stops returning
        // NULL and the pool re-enters the `min_shard_tip` minimum
        // (`scanning.rs:637-640`) — the second half of the same defect, which §3a
        // found in nobody's plan and which nothing would notice breaking.
        //
        // A8: the heal costs ZERO scan-queue rows and ZERO blocks. §3a measured
        // that on the SQL; this measures it on ours, which is what makes "existing
        // wallets self-heal on their next sync — no migration, no rescan" a
        // statement about this codebase.
        let dir = tempdir().expect("tempdir");
        let (mut conn, key) = fresh_conn(dir.path());
        let obs = observe(dir.path(), &key);

        assert_eq!(
            max_subtree_end_height(&obs, "ironwood"),
            None,
            "the stuck state, before: nothing has ever written ironwood \
             subtree_end_height, because `put_shard` omits the column from BOTH \
             its column list and its DO UPDATE SET"
        );
        let queue_before = count_rows(&obs, "scan_queue");
        let blocks_before = count_rows(&obs, "blocks");

        let h = nu63_activation(Network::Test) + 10_000;
        let mut fake = honest_three(
            vec![Ok(Some(canonical_root(h, 1)))],
            vec![Ok(Some(canonical_root(h, 2)))],
            vec![Ok(Some(canonical_root(h, 3)))],
        );
        let roots = fetch_three_pools(&mut fake).await.expect("fetch");
        put_subtree_roots(&mut conn, &roots, |_| Ok(())).expect("write");

        assert_eq!(
            max_subtree_end_height(&obs, "ironwood"),
            Some(i64::try_from(h).expect("height fits i64")),
            "A7(i): the served completing height must land in \
             ironwood_tree_shards.subtree_end_height — the column the spendability \
             gate and the scan-queue tip-shard range both read"
        );
        assert_eq!(
            count_rows(&obs, "scan_queue"),
            queue_before,
            "A8: healing must touch ZERO scan-queue rows"
        );
        assert_eq!(
            count_rows(&obs, "blocks"),
            blocks_before,
            "A8: healing must rescan ZERO blocks"
        );
    }

    #[tokio::test]
    async fn a_pool_that_served_no_roots_is_left_exactly_as_it_was() {
        // T0-1 A7(ii). The OTHER live state, and the contract splits A7 in two
        // because as one sentence it is false in this one: `put_shard_roots`
        // returns `Ok(())` immediately on an empty slice
        // (`commitment_tree.rs:1220`), so a non-NULL `subtree_end_height` is
        // UNSATISFIABLE for a pool that served nothing. That is the state of any
        // pool before its first 65,536 notes and the state NU7's pool will be in.
        //
        // It must be UNCHANGED and honestly reported, NOT asserted away — a fake
        // that always serves one root makes the single-sentence version of A7
        // pass and teaches nothing.
        let dir = tempdir().expect("tempdir");
        let (mut conn, key) = fresh_conn(dir.path());
        let obs = observe(dir.path(), &key);

        // ADJUDICATOR (shape repair): struct literal → `SubtreeRoots::from_pools`,
        // because the implementation gave `SubtreeRoots` a private `outcomes`
        // field. Same three vectors, same order, nothing asserted changed.
        let h = nu63_activation(Network::Test) + 10_000;
        let empty_ironwood = SubtreeRoots::from_pools(
            vec![parse_sapling_root(&canonical_root(h, 1)).expect("ok")],
            vec![parse_orchard_root(&canonical_root(h, 2)).expect("ok")],
            Vec::new(),
        );
        put_subtree_roots(&mut conn, &empty_ironwood, |_| Ok(()))
            .expect("a pool with nothing to write is a valid no-op, not an error");

        assert_eq!(
            max_subtree_end_height(&obs, "ironwood"),
            None,
            "A7(ii): a zero-root pool leaves subtree_end_height NULL. That is \
             upstream's behaviour and it is correct; a guard demanding non-NULL \
             here would be demanding something `put_shard_roots` cannot do."
        );
        assert_eq!(
            count_rows(&obs, "ironwood_tree_shards"),
            0,
            "A7(ii): and it writes no shard row at all"
        );
        assert!(
            max_subtree_end_height(&obs, "orchard").is_some(),
            "anti-vacuity: the OTHER pools in the same call did write, so 'nothing \
             happened' is not why the ironwood assertions above hold"
        );
    }

    #[tokio::test]
    async fn an_ironwood_fault_writes_nothing_at_all() {
        // T0-1 A4 — THE COLLECT-ALL-BEFORE-WRITE ARM.
        //
        // A4 and A9 contradict each other and the contract does not resolve it:
        // the implementer does, and must say which won. This test certifies the
        // A4 arm — all three pools collected BEFORE any DB write, so a fault on
        // ANY pool returns `Err` with nothing written. Its SIBLING,
        // `an_ironwood_refusal_leaves_the_other_two_pools_working`, certifies the
        // A9 arm. Exactly one of the two is the live guard; the other names what
        // was given up, which is the point of writing both.
        //
        // WHAT THIS ARM COSTS, stated because A4 requires it be stated: against a
        // lightwalletd too old to know Ironwood, the hard refusal from the third
        // fetch throws away the Sapling and Orchard roots too — the whole wallet
        // stops syncing on a server that worked yesterday. If that is the chosen
        // arm, the user-visible paragraph must say so plainly.
        let dir = tempdir().expect("tempdir");
        // ADJUDICATOR: `conn` renamed to `_conn` — the fetch aborts before any put,
        // so this test never writes through the handle and reads the shard tables
        // through `obs`. Left as `mut conn` it is `unused_variables` +
        // `unused_mut`, and the tree is built with `clippy -D warnings`. The
        // binding is KEPT (not deleted) so the migrated wallet db stays alive for
        // `observe` below. No assertion touched.
        let (_conn, key) = fresh_conn(dir.path());
        let obs = observe(dir.path(), &key);

        let h = nu63_activation(Network::Test) + 10_000;
        let mut fake = three_pool_fake(
            vec![Ok(Some(canonical_root(h, 1)))],
            vec![Ok(Some(canonical_root(h, 2)))],
            vec![],
        );
        fake.ironwood_open_err = Some(GrpcError::Timeout {
            stall: StallReason::TorUnavailable,
        });

        let r = fetch_three_pools(&mut fake).await;
        let described = describe_ref(&r);
        assert!(
            matches!(
                r,
                Err(WalletError::Sync {
                    stall: StallReason::TorUnavailable
                })
            ),
            "an Ironwood-pool fault aborts the whole fetch with the precise stall \
             preserved (the fail-closed Tor identity must survive); got {described}"
        );

        // and NOTHING was written — the money-safe property the existing doc
        // comment claims, now for three pools.
        for pool in ["sapling", "orchard", "ironwood"] {
            assert_eq!(
                count_rows(&obs, &format!("{pool}_tree_shards")),
                0,
                "A4: a fault on ANY pool must leave the witness tree untouched — \
                 {pool} was half-written"
            );
        }
    }

    #[tokio::test]
    async fn an_ironwood_refusal_leaves_the_other_two_pools_working() {
        // T0-1 A9 — THE NON-FATAL ARM, the sibling of the test above. See its
        // header for why both exist.
        //
        // A refusing endpoint is one that ANSWERED: an unsupported protocol is a
        // hard error, never an empty OK stream (measured, §3). Under this arm the
        // two pools it does serve must still be written, and "Ironwood roots
        // unavailable" must be recorded as an honest degraded state — NEVER as
        // "the pool is empty", which is what a silent `Ok(vec![])` would say.
        let dir = tempdir().expect("tempdir");
        let (mut conn, key) = fresh_conn(dir.path());
        let obs = observe(dir.path(), &key);

        let h = nu63_activation(Network::Test) + 10_000;
        let mut fake = three_pool_fake(
            vec![Ok(Some(canonical_root(h, 1)))],
            vec![Ok(Some(canonical_root(h, 2)))],
            vec![],
        );
        // v0.5.4's refusal, post-classification. ADJUDICATOR: the author wrote
        // `GrpcError::Status { code: 3 }` here, with the comment "carries no
        // transport source, so `classify_status` returns `Status { code: 3 }`".
        // That was TRUE at HEAD and is FALSE against this implementation: the
        // `GetSubtreeRoots` path now runs `classify_subtree_roots_status`, which
        // maps a source-less `InvalidArgument` on THIS RPC to the new
        // `GrpcError::ShieldedProtocolUnknown { code: 3 }` — that variant IS the
        // A9 fix. The fixture is moved to the value production now produces so
        // that it still models the thing the test names (a v0.5.4 refusal). NO
        // assertion changed; the injected value did, because the implementation
        // falsified the comment's own provenance claim.
        fake.ironwood_open_err = Some(GrpcError::ShieldedProtocolUnknown { code: 3 });

        let roots = fetch_three_pools(&mut fake)
            .await
            .expect("A9 arm: an Ironwood refusal does not stop the whole sync");
        assert_eq!(
            roots.sapling.len(),
            1,
            "Sapling's roots survive the refusal"
        );
        assert_eq!(
            roots.orchard.len(),
            1,
            "Orchard's roots survive the refusal"
        );
        put_subtree_roots(&mut conn, &roots, |_| Ok(())).expect("the two served pools are written");
        assert!(
            max_subtree_end_height(&obs, "orchard").is_some(),
            "A9 arm: the wallet keeps syncing the pools the server does serve"
        );
        // THE DISTINCTION THE WHOLE A4-vs-A9 RULING RESTS ON, and until the
        // post-build review nothing in this tree asserted it: a REFUSAL and an
        // EMPTY POOL are different answers. Both reviews found the same hole
        // independently — `PoolFetch` had zero assertions anywhere, so collapsing
        // `Unsupported` into `Served { roots: 0 }`, or dropping the `outcomes`
        // write entirely, compiled and passed every named test in silence
        // (sync.rs carried a file-wide `#![allow(dead_code)]` until P3-7, so not even a
        // warning). Its sibling assertion is in
        // `a_pre_activation_endpoint_may_honestly_serve_no_ironwood_roots`.
        assert_eq!(
            roots.outcome(ShieldedProtocol::Ironwood),
            Some(PoolFetch::Unsupported),
            "a refused pool is recorded as Unsupported — NOT as an empty one. \
             'No roots from this endpoint' and 'this pool has none yet' are the \
             two states A10 was ruled CONTRACT_WRONG for conflating, and the \
             honest-degradation claim is false if they are the same value"
        );
    }

    #[tokio::test]
    async fn an_ironwood_refusal_is_handled_the_same_on_both_server_versions() {
        // T0-1 A9, and this is the half a detector cannot fake.
        //
        // The refusal's STATUS CODE AND MESSAGE TEXT DIFFER BY SERVER VERSION —
        // measured on the wire (§3, finding 2): `Unknown (2) "unrecognized
        // shielded protocol"` on ECC lightwalletd v0.5.3, `InvalidArgument (3)
        // "GetSubtreeRoots: unrecognized shielded protocol: 99"` on v0.5.4. And
        // it is ALREADY MIS-FILED: `net/grpc.rs::classify_status:607` reads
        // `Code::Unavailable | Code::Unknown => GrpcError::Transport { stall }`,
        // so v0.5.3's refusal is indistinguishable from a flaky link and is
        // retried forever, while only v0.5.4's reaches `GrpcError::Status`.
        //
        // An assertion written against `InvalidArgument` alone PASSES while
        // v0.5.3 spins. So the assertion is not "it is classified as X" — which
        // would pick the implementer's mechanism for them — but the property
        // neither arm may violate: THE TWO SERVERS MUST PRODUCE THE SAME OUTCOME.
        // Whether that is achieved by carving `Unknown` out protocol-awarely or
        // by descoping to v0.5.4, this is what says the answer is not
        // version-dependent.
        let outcome = |e: GrpcError| async move {
            let h = nu63_activation(Network::Test) + 10_000;
            let mut fake = three_pool_fake(
                vec![Ok(Some(canonical_root(h, 1)))],
                vec![Ok(Some(canonical_root(h, 2)))],
                vec![],
            );
            fake.ironwood_open_err = Some(e);
            describe(fetch_three_pools(&mut fake).await)
        };

        // ADJUDICATOR: both fixtures moved, for the reason recorded on the sibling
        // test above. The author injected the values `classify_status` produced at
        // HEAD — `Transport { .. }` for v0.5.3's source-less `Unknown (2)` and
        // `Status { code: 3 }` for v0.5.4's `InvalidArgument (3)` — and asserted
        // the two must not diverge. Against this implementation neither value is
        // what a refusal produces any more: `classify_subtree_roots_status` maps
        // BOTH codes, on this RPC only, to `ShieldedProtocolUnknown { code }`. Left
        // at the old values the assertion would still have held, but for the wrong
        // reason (two ordinary faults, both fatal, trivially equal) and could never
        // have seen the version asymmetry break. The codes are kept distinct — 2
        // and 3, exactly what the two servers send — so a carve-out written for one
        // code only fails here.
        //
        // HONEST LIMIT, and it survives this repair: the fake injects
        // POST-classification values, so the mapping itself
        // (`tonic::Status` → `ShieldedProtocolUnknown`) is below this seam and is
        // NOT exercised by any test in this tree. That is the author's F-3, and
        // the ruling records it as owed back to the author.
        //
        // v0.5.3: `Unknown (2)`, no transport source.
        let v053 = outcome(GrpcError::ShieldedProtocolUnknown { code: 2 }).await;
        // v0.5.4: `InvalidArgument (3)`.
        let v054 = outcome(GrpcError::ShieldedProtocolUnknown { code: 3 }).await;

        assert_eq!(
            v053, v054,
            "the SAME endpoint refusal of the SAME request is handled differently \
             depending on which lightwalletd version answered: v0.5.3 gave \
             `{v053}` and v0.5.4 gave `{v054}`. One of those two servers is being \
             retried forever for a condition that will never heal."
        );
    }

    // ── T0-1 A10 · `a_zero_root_ironwood_stream_is_surfaced_not_silently_accepted`
    //    DELETED BY THE ADJUDICATOR. Verdict on A10: **CONTRACT_WRONG.**
    //
    // The test asserted `Err` for a zero-root Ironwood stream served alongside
    // Sapling/Orchard roots 10,000 blocks ABOVE the NU6.3 activation. It read A10's
    // "a surfaced condition, not `Ok(())`" as "a typed `Err`", and flagged that
    // reading as assumption S-4 for exactly this ruling. The assumption is a fair
    // reading of the row. The ROW is what is wrong.
    //
    // A10 rests on one sentence: *"An empty OK stream and a genuinely empty pool
    // are the same bytes; above the activation they are not the same thing."*
    // **That sentence is false, and this repository holds the measurement that
    // falsifies it.** `docs/plan/probes/ironwood-subtree-roots-probe.output.txt`
    // records the FIRST Ironwood `completing_block_height` as **3,451,206** on
    // mainnet against an activation of 3,428,143, and **4,307,326** on testnet
    // against an activation of 4,134,000. A pool's first subtree needs 2^16 = 65,536
    // note commitments, so for **23,063 mainnet blocks** (~3 weeks) and **173,326
    // testnet blocks** after NU6.3 activated, *every honest endpoint on earth*
    // served zero Ironwood roots at heights far above the activation. Height cannot
    // separate "this server serves us none" from "this pool has not filled its
    // first subtree yet", because for the whole window they are the same claim.
    //
    // A10 therefore collides with A7(ii) on A7(ii)'s own terms — it names the
    // legitimate zero-root state as *"the state of any pool before its first 65,536
    // notes, and the state NU7's pool will be in"*, which is a state that outlives
    // the activation. Any detector satisfying A10-as-written refuses a healthy
    // wallet for weeks after every future network upgrade: the "my remedy was worse
    // than the bug" shape the B block was re-decided to avoid, on the money path.
    // The author saw half of this and planted
    // `a_pre_activation_endpoint_may_honestly_serve_no_ironwood_roots` below,
    // predicting an unconditional `Err`; that guard survives and is the right one.
    //
    // WHAT SURVIVES, and it is the honest residue of A10: the zero is made a
    // DISTINCT value (`PoolFetch::Served { roots: 0 }`, never conflated with
    // `Unsupported` or with a healthy fetch), it gets a `warn!`, and it is carried
    // out of the pass in `SyncPass.root_outcomes`. What is NOT satisfied is
    // "surfaced": nothing renders it to a user. That is recorded as owed, not as
    // done. See the ruling for the owed row.
    #[tokio::test]
    async fn a_pre_activation_endpoint_may_honestly_serve_no_ironwood_roots() {
        // UNLISTED CASE — not an assertion row, and it is the negative half A10
        // needs to not become a bug of its own.
        //
        // PREDICTION: the cheapest way to make A10 green is "empty ironwood
        // stream ⇒ Err", unconditionally. That refuses the ONE case A7(ii) says
        // is legitimate and NU7's pool will be in on day one — a wallet syncing a
        // chain below the NU6.3 activation, where zero Ironwood roots is the
        // truthful answer. Shipping that turns a silent stuck state into a
        // wallet that cannot sync at all, which is strictly worse: the same
        // "my remedy was worse than the bug" shape the B block was re-decided
        // to avoid.
        //
        // So A10 must discriminate BY HEIGHT, not by emptiness. This is the case
        // that says so.
        let below = nu63_activation(Network::Test) - 100_000;
        let mut honest = honest_three(
            vec![Ok(Some(canonical_root(below, 1)))],
            vec![Ok(Some(canonical_root(below, 2)))],
            vec![],
        );
        let r = fetch_three_pools(&mut honest).await;
        let described = describe_ref(&r);
        assert!(
            r.is_ok(),
            "an endpoint whose newest served root is {below} — BELOW the NU6.3 \
             activation — is telling the truth when it serves no Ironwood roots: \
             the pool does not exist there yet. Refusing it makes the wallet \
             unable to sync a chain that is perfectly healthy; got {described}"
        );
        let served = r.expect("just asserted");
        assert!(
            served.ironwood.is_empty(),
            "and the honest empty answer stays empty"
        );
        // The other half of the distinction — see the sibling assertion in
        // `an_ironwood_refusal_leaves_the_other_two_pools_working`. An honest
        // empty stream is `Served { roots: 0 }`, never `Unsupported`: the server
        // answered, and the answer was "none yet".
        assert_eq!(
            served.outcome(ShieldedProtocol::Ironwood),
            Some(PoolFetch::Served { roots: 0 }),
            "an endpoint that ANSWERS with zero roots is Served{{roots:0}}, not \
             Unsupported — conflating the two is what A10 was ruled \
             CONTRACT_WRONG for, and it is the state every pool is in before its \
             first 65,536 notes"
        );
    }

    #[test]
    fn our_store_really_overrides_the_ironwood_tree_accessor() {
        // T0-1 A6b, PARTIAL — the half that is constructible in this tree. See the
        // test author's report, finding F-1, for why the whole of A6b is not.
        //
        // A6b rests on `with_ironwood_tree_mut` being a REAL accessor on our
        // store: the trait ships a default impl returning `Ok(None)`, which
        // `create_proposed_transactions` turns into `Error::ProposalNotSupported`
        // (`zcash_client_backend-0.24.0/src/data_api/wallet.rs:1899`). The
        // contract checked that `zcash_client_sqlite` overrides it at `lib.rs:3309`
        // and `:3433` and noted that "a test double that does not override it
        // would be a silent fake".
        //
        // That is a claim about OUR `WalletConn`, which is a type alias over a
        // generic `WalletDb`, and nothing in this tree asserted it. If the
        // override were ever lost — a feature flip, a wrapper type, a different
        // `WalletDb` instantiation — every Ironwood spend would fail with
        // "proposal not supported" and the roots would be written correctly the
        // whole time. The distinguishing observation is Some-vs-None: the default
        // returns `Ok(None)` WITHOUT running the callback, the override runs it
        // and wraps the result in `Some`.
        let dir = tempdir().expect("tempdir");
        let (mut conn, _key) = fresh_conn(dir.path());

        let ran = std::cell::Cell::new(false);
        let got: Option<()> = conn
            .with_ironwood_tree_mut::<_, (), ShardTreeError<commitment_tree::Error>>(|_tree| {
                ran.set(true);
                Ok(())
            })
            .expect("reading our own ironwood tree is not a fault");

        assert!(
            got.is_some(),
            "`WalletConn::with_ironwood_tree_mut` returned `None` — that is the \
             TRAIT DEFAULT, which `create_proposed_transactions` maps to \
             `Error::ProposalNotSupported`. Every Ironwood spend would fail with \
             'proposal not supported' no matter how correctly the subtree roots \
             were fetched and written."
        );
        assert!(
            ran.get(),
            "the callback never ran, so the accessor is not reaching a real tree"
        );

        // The sibling accessors A6b's witness path uses, on the same store, so a
        // pool-specific gap shows here rather than inside a proposal error.
        conn.get_ironwood_subtree_root(0)
            .expect("the ironwood subtree-root read path exists on our store");
    }

    #[tokio::test]
    async fn a_hostile_completing_height_is_refused_before_it_is_written() {
        // T0-1 A11 — the endpoint-controlled number that can mint a spendable
        // balance with no constructible witness.
        //
        // `collect_roots` validates SHAPE only — 32 bytes and `u32` range
        // (`sync.rs:133-144`) — and the value lands via `ON CONFLICT … DO UPDATE
        // SET subtree_end_height = :subtree_end_height`
        // (`commitment_tree.rs:1298-1301`): an UNCONDITIONAL overwrite with no
        // conflict check on the height, re-writable on EVERY sync by an endpoint
        // whose `root_hash`es are otherwise correct. That one number then drives
        // `mark_stabilized_notes`' `subtree_end_height <= :pruning_floor`, the
        // join window of the unscanned-ranges view, AND `min_shard_tip`. Set it
        // just above the activation and notes look `witness_stabilized` on shards
        // the wallet has never scanned.
        //
        // TWO of the contract's three clauses are asserted here. The third —
        // "last ≤ the endpoint's own reported tip" — names an input neither
        // `SubtreeRootSource` nor `fetch_subtree_roots` has; see the report.
        //
        // SEAM-AGNOSTIC BY CONSTRUCTION: the assertion is on the fetch+put PAIR,
        // because the activation bound needs the network (which `WalletConn`
        // carries and `fetch_subtree_roots` does not) while the ordering bound
        // needs only the stream. Either placement passes; neither is prescribed.
        let activation = nu63_activation(Network::Test);

        // (a) NOT strictly increasing across the stream.
        let non_monotonic = vec![
            Ok(Some(canonical_root(activation + 20_000, 3))),
            Ok(Some(canonical_root(activation + 10_000, 4))),
        ];
        // (b) first root BELOW the NU6.3 activation — an Ironwood subtree cannot
        //     have completed before the pool existed.
        let below_activation = vec![Ok(Some(canonical_root(activation - 1, 3)))];

        for (name, script) in [
            ("non-monotonic completing heights", non_monotonic),
            (
                "first completing height below the NU6.3 activation",
                below_activation,
            ),
        ] {
            let dir = tempdir().expect("tempdir");
            let (mut conn, key) = fresh_conn(dir.path());
            let obs = observe(dir.path(), &key);

            let good = activation + 30_000;
            let mut fake = honest_three(
                vec![Ok(Some(canonical_root(good, 1)))],
                vec![Ok(Some(canonical_root(good, 2)))],
                script,
            );
            let outcome = match fetch_three_pools(&mut fake).await {
                Err(e) => Err(e),
                Ok(roots) => put_subtree_roots(&mut conn, &roots, |_| Ok(())),
            };
            assert!(
                matches!(outcome, Err(WalletError::Sync { .. })),
                "{name}: an endpoint-supplied completing height that cannot be true \
                 must be a typed `Sync` fault. Accepting it lets the endpoint choose \
                 the height that decides which notes count as stabilized — a \
                 spendable balance with no constructible witness. Got {outcome:?}"
            );
            assert_eq!(
                max_subtree_end_height(&obs, "ironwood"),
                None,
                "{name}: and NEVER a partial accept — nothing may reach \
                 ironwood_tree_shards.subtree_end_height"
            );
        }
    }

    #[tokio::test]
    async fn collect_roots_caps_a_flooding_endpoint() {
        // §8 (DoS, security fold): a server streaming valid roots FOREVER is stopped
        // at MAX_SUBTREE_ROOTS_PER_POOL with a typed Sync error — never unbounded Vec
        // growth, never a hang (the count dimension of size-cap-before-alloc,
        // principle 7). The endless source has no clean end and no error, so ONLY the
        // count ceiling can terminate `collect_roots`.
        let mut s = crate::net::grpc::testing::endless_stream(canonical_root(1_000_000, 1));
        assert!(
            matches!(
                collect_sapling_roots(&mut s).await,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "a flooding endpoint is capped, not allowed to OOM the sync task"
        );
    }

    proptest::proptest! {
        #[test]
        fn parse_root_never_panics_on_arbitrary_bytes(
            hash in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..80usize),
            height in proptest::prelude::any::<u64>(),
        ) {
            // §8 (testing-patterns #2 — a parser of attacker bytes must never panic):
            // any length / any height / any byte content decodes to Ok or a typed
            // Err(SubtreeRootInvalid), NEVER a panic. A non-32-byte hash or an
            // out-of-u32 height is ALWAYS rejected (the deterministic guards); a
            // 32-byte hash may or may not be a canonical field element (both valid).
            let root = SubtreeRoot {
                root_hash: hash.clone(),
                completing_block_hash: Vec::new(),
                completing_block_height: height,
            };
            let s = parse_sapling_root(&root);
            let o = parse_orchard_root(&root);
            if hash.len() != 32 || height > u64::from(u32::MAX) {
                proptest::prop_assert!(s.is_err(), "non-32-byte/out-of-range sapling reject");
                proptest::prop_assert!(o.is_err(), "non-32-byte/out-of-range orchard reject");
            }
        }
    }

    // ── The per-batch scan primitive (iv-d-2b-i) ─────────────────────────────

    fn fresh_cache(dir: &std::path::Path) -> BlockCache {
        // The cache is a SEPARATE keyed DB (`block-cache.db`) under the same dir as
        // `fresh_conn`'s wallet.db; its key is independent (data plane only).
        let key = WalletDbKey::generate();
        BlockCache::open(dir, &key).expect("block cache opens")
    }

    /// Test wrapper: `download_range` with a never-cancelled token + a no-op
    /// reporter. The completeness / shape / wire-conversion tests below don't
    /// exercise cancel or progress (those have dedicated tests); `tip = end` makes
    /// the percent inert. Keeps those tests focused on their original concern.
    async fn dl<C: ScanClient + ?Sized>(
        client: &mut C,
        cache: &BlockCache,
        start: u64,
        end: u64,
    ) -> Result<DownloadOutcome, WalletError> {
        let cancel = CancelToken::new();
        let ctx = ScanCtx {
            tip: end,
            cancel: &cancel,
            report: &|_| {},
            summary: ProgressSnapshot::default(),
        };
        download_range(client, cache, start, end, ctx).await
    }

    /// [`dl`] for the injectable-byte-cap path (the gate-7 DoS boundary test).
    async fn dl_capped<C: ScanClient + ?Sized>(
        client: &mut C,
        cache: &BlockCache,
        start: u64,
        end: u64,
        max_bytes: u64,
    ) -> Result<DownloadOutcome, WalletError> {
        let cancel = CancelToken::new();
        let ctx = ScanCtx {
            tip: end,
            cancel: &cancel,
            report: &|_| {},
            summary: ProgressSnapshot::default(),
        };
        download_range_capped(client, cache, start, end, max_bytes, ctx).await
    }

    #[tokio::test]
    async fn download_range_delivers_the_full_span_in_order() {
        // §8: a clean in-order stream of the full half-open span lands every block in
        // the cache (the iv-a BlockStream → iv-b cache happy path).
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![
            Ok(Some(compact_block(100))),
            Ok(Some(compact_block(101))),
            Ok(Some(compact_block(102))),
        ]);
        dl(&mut client, &cache, 100, 103)
            .await
            .expect("the full span downloads");
        let got = cache.read(100, 103).await.expect("read back");
        assert_eq!(got.len(), 3, "all three blocks are cached");
        assert_eq!(
            client.last_block_range,
            Some((100, 102)),
            "the half-open [100,103) is requested as the INCLUSIVE wire range [100,102]"
        );
    }

    #[tokio::test]
    async fn download_range_short_stream_is_typed_sync_never_silent() {
        // §8 (THE money property, §3.2g stream-completeness): a clean-but-SHORT
        // stream (the server ends before the requested span) is a typed `Sync`,
        // NEVER silently accepted — else the unscanned tail becomes a gap the
        // scanner records as scanned = missed funds.
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![
            Ok(Some(compact_block(100))),
            Ok(Some(compact_block(101))),
        ]);
        let r = dl(&mut client, &cache, 100, 103).await;
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "a short clean stream is typed Sync, never a silent partial; got {r:?}"
        );
    }

    #[tokio::test]
    async fn download_range_gap_in_the_span_is_typed_sync() {
        // §8: a gap (100 then 102, skipping 101) is rejected at download BEFORE the
        // cache — stronger than the cache's own gap-stop (defense in depth).
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![
            Ok(Some(compact_block(100))),
            Ok(Some(compact_block(102))),
        ]);
        let r = dl(&mut client, &cache, 100, 103).await;
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "an out-of-order / gapped block is typed Sync; got {r:?}"
        );
    }

    #[tokio::test]
    async fn download_range_transport_fault_preserves_stall() {
        // §8 (unstable-network): a mid-stream transport fault surfaces typed `Sync`
        // (the SPENT-ON-ERROR latch makes any re-poll loud) — never `Ok(None)`.
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![
            Ok(Some(compact_block(100))),
            Err(tonic::Status::unavailable("link dropped mid-stream")),
        ]);
        let r = dl(&mut client, &cache, 100, 103).await;
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "a mid-stream transport fault is typed Sync; got {r:?}"
        );
    }

    #[tokio::test]
    async fn download_range_overlong_span_is_typed_sync() {
        // §8: a server that delivers the full span THEN an extra block (overlong) is
        // rejected by the `next >= end` arm — never an over-read past end-1.
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![
            Ok(Some(compact_block(100))),
            Ok(Some(compact_block(101))),
            Ok(Some(compact_block(102))), // one past the requested [100, 102)
        ]);
        let r = dl(&mut client, &cache, 100, 102).await;
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "an overlong span (a block past end-1) is typed Sync; got {r:?}"
        );
    }

    #[tokio::test]
    async fn download_range_caps_a_block_flooding_endpoint() {
        // §8 (gate-7/DoS, §4.6): a server padding the batch past the byte ceiling is
        // stopped with a typed `Sync`, never an unbounded buffer / OOM. Driven with a
        // TINY injected cap (the real DOWNLOAD_BATCH_MAX_BYTES = 128 MiB is impractical
        // to stream — the BlockCache::open_with_bound pattern).
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        // ~40 minimal blocks; each encodes to a few bytes, summing past a 16-byte cap.
        let blocks: Vec<_> = (100..140).map(|h| Ok(Some(compact_block(h)))).collect();
        let mut client = FakeScanClient::with_blocks(blocks);
        let r = dl_capped(&mut client, &cache, 100, 140, 16).await;
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "a flooding endpoint is capped with a typed Sync; got {r:?}"
        );
    }

    #[tokio::test]
    async fn fetch_chain_state_decodes_the_anchor_and_a_fault_is_typed() {
        // §8: a valid tree state decodes to a `ChainState` at the requested height;
        // a transport fault is typed `Sync` with the precise stall preserved (Tor
        // vs endpoint survives the conversion).
        use super::testing::{FakeScanClient, tree_state_at};
        let mut client = FakeScanClient::with_tree_state(tree_state_at(800_000));
        let cs = fetch_chain_state(&mut client, 800_000)
            .await
            .expect("the anchor decodes");
        assert_eq!(
            u64::from(cs.block_height()),
            800_000,
            "anchored at the height"
        );

        let mut faulty = FakeScanClient::with_tree_state(tree_state_at(800_000));
        faulty.tree_state_err = Some(GrpcError::Timeout {
            stall: StallReason::TorUnavailable,
        });
        let r = fetch_chain_state(&mut faulty, 800_000).await;
        assert!(
            matches!(
                r,
                Err(WalletError::Sync {
                    stall: StallReason::TorUnavailable
                })
            ),
            "the precise stall survives the fetch; got {r:?}"
        );
    }

    #[test]
    fn scan_batch_from_state_misalignment_is_typed_never_panics() {
        // §8 (THE robustness GUARD, §3.2g): the upstream `scan_cached_blocks`
        // `assert_eq!(from_height == from_state.block_height + 1)` (chain.rs:600) is
        // an FFI-crashing PANIC. A server that anchors its tree state at the wrong
        // height (from_state @ H, scanning from H+5) must surface a typed `Sync`,
        // NEVER reach the assert. (If the guard were missing, this test would PANIC
        // the runner instead of returning Err.)
        use super::testing::tree_state_at;
        let dir = tempdir().expect("tempdir");
        let (mut conn, k) = fresh_conn(dir.path());
        let aux = fresh_aux(dir.path(), &k);
        let cache = fresh_cache(dir.path());
        let from_state = tree_state_at(1_000_000)
            .to_chain_state()
            .expect("anchor decodes");
        let r = scan_batch(
            Network::Test,
            &cache,
            &mut conn,
            &aux,
            1_000_005,
            &from_state,
            10,
            BlockHeight::from_u32(1_000_015),
        );
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "a from_state/from_height misalignment is typed Sync, never a panic; got {r:?}"
        );
    }

    #[test]
    fn scan_batch_clean_empty_scan_advances() {
        // §8 (happy-path wiring): an aligned batch over an empty cache finds no
        // blocks and advances cleanly (`Scanned`). Positive note-detection + a real
        // reorg need heavy upstream `test-dependencies` block fixtures (a
        // supply-chain decision) and are an e2e obligation, NOT a unit test.
        use super::testing::tree_state_at;
        let dir = tempdir().expect("tempdir");
        let (mut conn, k) = fresh_conn(dir.path());
        let aux = fresh_aux(dir.path(), &k);
        let cache = fresh_cache(dir.path());
        let from_state = tree_state_at(500_000)
            .to_chain_state()
            .expect("anchor decodes");
        let out = scan_batch(
            Network::Test,
            &cache,
            &mut conn,
            &aux,
            500_001,
            &from_state,
            10,
            BlockHeight::from_u32(500_011),
        )
        .expect("a clean empty scan");
        assert_eq!(
            out,
            ScanOutcome::Scanned {
                received_notes: 0,
                // BIND-1 (§4x): a wallet with no recorded completion heights has
                // nothing for the scan-time reconcile to contradict, so the batch
                // carries an EMPTY report — the abstention, asserted rather than
                // ignored with a `..`.
                boundary: BoundaryReport::default(),
            },
            "an empty aligned batch advances (and an EMPTY batch reports zero received notes \
             — the ADR-0536 cheap gate stays closed)"
        );
    }

    // ── T0-1 · B3 + B4: `NonSequentialBlocks` is classified honestly ──────────

    /// A shape-valid, correctly-chained block at `height` whose IRONWOOD
    /// commitment-tree size is a LIE: it claims `ironwood_final_tree_size` notes
    /// while carrying zero Ironwood actions. That is exactly what an endpoint
    /// serving a batch treestate with `ironwood_tree` omitted produces — the
    /// wallet's anchor says the Ironwood tree is size 0, the block says it ended
    /// at N, and upstream's per-batch check
    /// (`zcash_client_backend-0.24.0/src/data_api/ll/wallet.rs:337-339`) refuses
    /// the batch.
    fn ironwood_desynced_block(height: u64, ironwood_final_tree_size: u32) -> CompactBlock {
        use super::testing::compact_block;
        let mut b = compact_block(height);
        b.chain_metadata
            .as_mut()
            .expect("the fixture block carries chain metadata")
            .ironwood_commitment_tree_size = ironwood_final_tree_size;
        b
    }

    /// Drive a REAL scan of one `ironwood_desynced_block` against a correctly
    /// chained anchor and return whatever `scan_batch` classified it as.
    ///
    /// B4 forbids the shortcut this deliberately avoids: the error is produced by
    /// upstream's own code on a real batch, never hand-constructed. That exact
    /// shortcut already produced a guard that kept passing while production
    /// stopped emitting the variant it built (`sync.rs:588-593` records it), and
    /// `SqliteClientError` is `#[non_exhaustive]`, so a hand-built
    /// `NonSequentialBlocks` would also stop tracking any future field it gains.
    async fn scan_one_ironwood_desynced_block() -> Result<ScanOutcome, WalletError> {
        use super::testing::{FakeScanClient, chain_tree_state};
        let dir = tempdir().expect("tempdir");
        let (mut conn, k) = fresh_conn(dir.path());
        let aux = fresh_aux(dir.path(), &k);
        let cache = fresh_cache(dir.path());

        let h = 1_000_000u64;
        let mut client = FakeScanClient::with_blocks(vec![Ok(Some(ironwood_desynced_block(h, 7)))]);
        dl(&mut client, &cache, h, h + 1)
            .await
            .expect("the fixture block is shape-valid and downloads");

        let from_state = chain_tree_state(h - 1)
            .to_chain_state()
            .expect("the anchor decodes");
        scan_batch(
            Network::Test,
            &cache,
            &mut conn,
            &aux,
            h,
            &from_state,
            1,
            BlockHeight::from_u32(1_000_001),
        )
    }

    #[tokio::test]
    async fn ironwood_treestate_desync_is_not_reported_as_a_destroyed_wallet() {
        // T0-1 B3 + B4 (`production-readiness-phase-1.md` §4). An endpoint that
        // omits `ironwood_tree` on ONE batch treestate makes upstream refuse the
        // batch with `NonSequentialBlocks`, a `SqliteClientError` variant that
        // reaches `into_store_fault`'s catch-all `_ => WalletError::StoreCorrupt`
        // (`sync.rs:599`) — the RED restore-from-your-recovery-phrase remedy, on
        // a wallet whose data is intact. Same mis-attribution class as T0-7 and
        // `#371`, at the door the error actually exits.
        //
        // WHAT THIS ASSERTS, and why it is a negative rather than a match on one
        // variant. The contract's requirement is about the REMEDY the error
        // names, not about a particular enum shape the implementer has not been
        // handed. Two outcomes claim the wallet is destroyed and this tree says
        // so in its own docs:
        //   - `WalletError::StoreCorrupt` — the red seed-restore remedy
        //     (`error.rs:207`, and the whole subject of #371).
        //   - `WalletError::Sync { stall: StallReason::Internal }` — whose doc
        //     names "repair / restore-from-seed" as its honest next step
        //     (`state.rs:111-118`).
        // Anything else is a truthful answer to a wrong-anchor condition; picking
        // WHICH is the implementer's call, and the adjudicator's to rule on.
        //
        // HONEST LIMIT, stated rather than papered over: this asserts the class
        // of remedy through the variant, not the rendered sentence. There is no
        // seam in this tree that carries "name re-import as the remedy" as data,
        // so the third clause of B3 is not machine-checkable here; it is reported
        // as a finding instead of being faked with a string match.
        let r = scan_one_ironwood_desynced_block().await;

        // Anti-vacuity FIRST: if the fixture stopped producing a refusal at all,
        // every assertion below would hold for the wrong reason. A batch whose
        // ironwood tree size disagrees with the anchor must NOT scan cleanly.
        assert!(
            r.is_err(),
            "the fixture no longer produces a refusal — upstream accepted a batch whose \
             ironwood final tree size (7) cannot follow an empty-ironwood anchor. Every \
             assertion in this test is vacuous until that is fixed; got {r:?}"
        );

        let err = r.expect_err("just asserted");
        assert!(
            !matches!(err, WalletError::StoreCorrupt),
            "a wrong/absent Ironwood anchor from the ENDPOINT told a user with an intact \
             wallet to restore from their recovery phrase. The data is fine; the treestate \
             is not. Got {err:?}"
        );
        assert!(
            !matches!(
                err,
                WalletError::Sync {
                    stall: StallReason::Internal
                }
            ),
            "`StallReason::Internal`'s own doc names repair / restore-from-seed as the \
             honest next step, so routing an ENDPOINT-caused desync there just moves the \
             same false claim one variant over. Got {err:?}"
        );
    }

    #[tokio::test]
    async fn ironwood_treestate_desync_is_not_a_reorg() {
        // UNLISTED CASE — not an assertion row. PREDICTION: the cheapest way to
        // make B3 green is to widen `scan_batch`'s continuity arm, or to map the
        // whole `Wallet` error class to `EndpointUnreachable`, so the desync
        // becomes "a reorg" or "switch servers" and the wallet quietly rewinds
        // and re-scans the same batch forever. Both satisfy B3's letter (neither
        // is `StoreCorrupt`) and both are dishonest: a rewind cannot fix a
        // treestate the endpoint keeps omitting, so the wallet loops.
        //
        // A wrong anchor is not a chain reorganisation and must never be
        // classified as one.
        let r = scan_one_ironwood_desynced_block().await;
        assert!(
            !matches!(r, Ok(ScanOutcome::Reorg { .. })),
            "an Ironwood treestate desync was classified as a REORG — the wallet will \
             rewind and re-scan the same batch against the same wrong anchor, forever, \
             with no error shown; got {r:?}"
        );
        assert!(
            !matches!(r, Ok(ScanOutcome::Scanned { .. })),
            "an Ironwood treestate desync was ACCEPTED as a clean scan; got {r:?}"
        );
    }

    // ── SCAN-2 (§4t): the derived anchor — the implementer's own pins ─────────

    /// A small little-endian integer as 32 bytes — canonical for jubjub::Base and
    /// pallas::Base alike (both moduli ≈ 2^255), so it is a valid `cmu` AND a
    /// valid `cmx`.
    fn le32(n: u64) -> [u8; 32] {
        let mut v = [0u8; 32];
        v[..8].copy_from_slice(&n.to_le_bytes());
        v
    }

    /// The Orchard-shaped leaf for [`le32`]`(n)`.
    fn orchard_leaf(n: u64) -> MerkleHashOrchard {
        Option::from(MerkleHashOrchard::from_bytes(&le32(n))).expect("canonical")
    }

    /// A shape-valid compact tx carrying exactly the given 32-byte commitments:
    /// one Sapling output per `cmus` entry, one Orchard action per `cmxs`, one
    /// Ironwood action per `ironwood_cmxs` — the fields the fold reads.
    fn tx_with_commitments(
        cmus: &[[u8; 32]],
        cmxs: &[[u8; 32]],
        ironwood_cmxs: &[[u8; 32]],
    ) -> zcash_client_backend::proto::compact_formats::CompactTx {
        use zcash_client_backend::proto::compact_formats::{
            CompactOrchardAction, CompactSaplingOutput, CompactTx,
        };
        let action = |c: &[u8; 32]| CompactOrchardAction {
            cmx: c.to_vec(),
            ..Default::default()
        };
        CompactTx {
            txid: vec![0u8; 32],
            outputs: cmus
                .iter()
                .map(|c| CompactSaplingOutput {
                    cmu: c.to_vec(),
                    ..Default::default()
                })
                .collect(),
            actions: cmxs.iter().map(action).collect(),
            ironwood_actions: ironwood_cmxs.iter().map(action).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn a_derived_anchor_carries_the_frontier_and_the_hash_across_empty_blocks() {
        // §4t pin (a): over the fake chain's empty blocks the fold moves the
        // height and the hash forward and leaves every frontier as it was; the
        // identity fold is `from`; a gap and a wrong first height are refused as
        // the endpoint's.
        use super::testing::{chain_tree_state, compact_block};
        let h = 1_000_000u64;
        let from = chain_tree_state(h)
            .to_chain_state()
            .expect("the anchor decodes");
        let blocks: Vec<CompactBlock> = (h + 1..=h + 3).map(compact_block).collect();
        let derived =
            derive_chain_state(&from, &blocks).expect("three consecutive empty blocks fold");
        assert_eq!(
            u64::from(derived.block_height()),
            h + 3,
            "the state is at the LAST block"
        );
        assert_eq!(
            derived.block_hash(),
            blocks[2].hash(),
            "the hash is the last block's own `hash` — P5: endpoint-attested exactly as the \
             block is"
        );
        let served = chain_tree_state(h + 3)
            .to_chain_state()
            .expect("the served state decodes");
        assert_eq!(derived.final_sapling_tree(), served.final_sapling_tree());
        assert_eq!(derived.final_orchard_tree(), served.final_orchard_tree());
        assert_eq!(derived.final_ironwood_tree(), served.final_ironwood_tree());
        // THE JOIN ITEM THIS ROW WAS WRITTEN AGAINST, NOW CLOSED — and the assertion
        // is inverted rather than deleted, because the agreement is the property the
        // reconcile rests on. The reconcile compares the WHOLE `ChainState` (§4t),
        // and `TreeState::to_chain_state` REVERSES the hex it is given (zcashd's
        // display order, `zcash_client_backend-0.24.0/src/proto.rs:456-457`) while
        // `testing::compact_block(h).hash` is the raw `block_id(h)`. The fixture
        // used to hex-encode that id unreversed, so the served hash was the
        // byte-reverse of the block's own and a pass longer than
        // `TREE_STATE_RECONCILE_BATCHES` stalled `EndpointMisbehaving` over the
        // fake. It red two shipped rows that scan 81 consecutive batches
        // (`sync_summary_is_throttled_not_read_every_batch` and
        // `…_refreshes_on_first_batch_and_at_tip`) — neither half ran them, and both
        // fold reviews found it independently. `chain_tree_state` now encodes the
        // display order, so the two sides agree and the fake can reconcile.
        assert_eq!(
            served.block_hash(),
            derived.block_hash(),
            "the fake's served tree-state hash and the block's own must land on ONE \
             internal order, or no pass longer than the reconcile cadence can agree \
             over `FakeChain` — `chain_tree_state` encodes the DISPLAY order because \
             `to_chain_state` reverses what it decodes"
        );
        assert_eq!(
            derive_chain_state(&from, &[]).expect("the empty fold"),
            from,
            "no blocks: the anchor is unchanged"
        );
        let refused = |blocks: &[CompactBlock]| {
            matches!(
                derive_chain_state(&from, blocks),
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            )
        };
        assert!(
            refused(&[compact_block(h + 1), compact_block(h + 3)]),
            "a gap inside the span is the endpoint's"
        );
        assert!(
            refused(&[compact_block(h + 2)]),
            "a first block that is not `from + 1` is the endpoint's"
        );
    }

    /// **§4t-run review row 3.** The reconcile's mismatch warn names WHICH of the
    /// five compared members disagreed, in the order the comparison reads them:
    /// height, hash, then the three pools. The comparison itself stays WHOLE — this
    /// only labels it, so a field report can tell a server whose `GetTreeState` hash
    /// convention differs from its `GetBlockRange` one (the same field on every
    /// mismatch, forever) from a tamper (a frontier moved).
    ///
    /// Every pool arm holds the height AND the hash equal to the base's, so the arm
    /// under test is the only one that CAN fire — a label that merely repeated the
    /// first difference found would pass a weaker row than this one.
    #[test]
    fn the_reconcile_mismatch_names_the_field_that_disagreed() {
        use super::testing::{chain_tree_state, compact_block};
        use zcash_client_backend::proto::compact_formats::ChainMetadata;
        let h = 1_000_000u64;
        let base = chain_tree_state(h)
            .to_chain_state()
            .expect("the anchor decodes");

        // Equal states reach the fall-through, which is the "upstream grew a sixth
        // member" code and not a field — the reconcile never calls it on an
        // agreement, and it is a code rather than a panic if it ever does.
        assert_eq!(chain_state_mismatch_field(&base, &base), "unknown");

        // HEIGHT outranks the hash that necessarily differs with it.
        let higher = chain_tree_state(h + 1)
            .to_chain_state()
            .expect("the anchor decodes");
        assert_ne!(
            base.block_hash(),
            higher.block_hash(),
            "the fixture's two hashes DO differ, so `height` below is a priority and not a \
             default"
        );
        assert_eq!(chain_state_mismatch_field(&base, &higher), "height");

        // A hash no block carries, at the same height.
        let mut lie = chain_tree_state(h);
        lie.hash = hex::encode([0xA5u8; 32]);
        let lie = lie.to_chain_state().expect("the lie decodes");
        assert_eq!(base.block_height(), lie.block_height());
        assert_eq!(chain_state_mismatch_field(&base, &lie), "hash");

        // One commitment in each pool, so each frontier has a non-empty
        // counterpart to swap in.
        let mut block = compact_block(h + 1);
        block.vtx = vec![tx_with_commitments(&[le32(1)], &[le32(2)], &[le32(3)])];
        block.chain_metadata = Some(ChainMetadata {
            sapling_commitment_tree_size: 1,
            orchard_commitment_tree_size: 1,
            ironwood_commitment_tree_size: 1,
        });
        let filled =
            derive_chain_state(&base, std::slice::from_ref(&block)).expect("the block folds");
        for (pool, state) in [
            (
                "sapling",
                ChainState::new(
                    base.block_height(),
                    base.block_hash(),
                    filled.final_sapling_tree().clone(),
                    base.final_orchard_tree().clone(),
                    base.final_ironwood_tree().clone(),
                ),
            ),
            (
                "orchard",
                ChainState::new(
                    base.block_height(),
                    base.block_hash(),
                    base.final_sapling_tree().clone(),
                    filled.final_orchard_tree().clone(),
                    base.final_ironwood_tree().clone(),
                ),
            ),
            (
                "ironwood",
                ChainState::new(
                    base.block_height(),
                    base.block_hash(),
                    base.final_sapling_tree().clone(),
                    base.final_orchard_tree().clone(),
                    filled.final_ironwood_tree().clone(),
                ),
            ),
        ] {
            assert_eq!(
                base.block_height(),
                state.block_height(),
                "{pool}: the height is held equal"
            );
            assert_eq!(
                base.block_hash(),
                state.block_hash(),
                "{pool}: the hash is held equal"
            );
            assert_ne!(base, state, "{pool}: the states must actually differ");
            assert_eq!(chain_state_mismatch_field(&base, &state), pool);
        }
    }

    #[test]
    fn a_derived_anchor_appends_every_pool_in_output_order_and_checks_each_block_size() {
        // §4t pin (a), the non-empty half: one block with one Sapling output
        // (cmu 1), two Orchard actions (cmx 2, 3) and one Ironwood action (cmx 4)
        // lands in THREE trees (the cross-wire hazard the `SubtreeRoots` doc
        // names — Orchard and Ironwood share a type) in output order, with
        // `Frontier::append`'s own ommer, and its `chain_metadata` {1, 2, 1}
        // passes the per-block size check (P6). Then the refusals: a 31-byte cmu
        // (the scanner's `cmu()` would panic — the fold errs), a non-canonical
        // cmx, a metadata size the commitments do not reach; and a block WITHOUT
        // metadata folds unchecked, as the scanner treats it.
        use super::testing::{chain_tree_state, compact_block};
        use zcash_client_backend::proto::compact_formats::ChainMetadata;
        let h = 1_000_000u64;
        let from = chain_tree_state(h)
            .to_chain_state()
            .expect("the anchor decodes");
        let mut block = compact_block(h + 1);
        block.vtx = vec![tx_with_commitments(
            &[le32(1)],
            &[le32(2), le32(3)],
            &[le32(4)],
        )];
        block.chain_metadata = Some(ChainMetadata {
            sapling_commitment_tree_size: 1,
            orchard_commitment_tree_size: 2,
            ironwood_commitment_tree_size: 1,
        });
        let derived =
            derive_chain_state(&from, std::slice::from_ref(&block)).expect("the block folds");
        assert_eq!(derived.final_sapling_tree().tree_size(), 1);
        assert_eq!(derived.final_orchard_tree().tree_size(), 2);
        assert_eq!(derived.final_ironwood_tree().tree_size(), 1);
        let sapling = derived.final_sapling_tree().value().expect("one leaf");
        assert_eq!(
            *sapling.leaf(),
            Option::<SaplingNode>::from(SaplingNode::from_bytes(le32(1))).expect("canonical"),
            "the Sapling leaf is the output's cmu through `Node::from_cmu`"
        );
        let orchard = derived.final_orchard_tree().value().expect("two leaves");
        assert_eq!(
            u64::from(orchard.position()),
            1,
            "the Orchard frontier sits at the second leaf"
        );
        assert_eq!(
            *orchard.leaf(),
            orchard_leaf(3),
            "…whose leaf is the LAST action's cmx (output order)"
        );
        assert_eq!(
            orchard.ommers(),
            &[orchard_leaf(2)][..],
            "…with the first action's cmx as its one ommer — `Frontier::append`'s, not ours"
        );
        let ironwood = derived.final_ironwood_tree().value().expect("one leaf");
        assert_eq!(u64::from(ironwood.position()), 0);
        assert_eq!(
            *ironwood.leaf(),
            orchard_leaf(4),
            "the Ironwood action went to the IRONWOOD tree, not Orchard's"
        );
        let refused = |b: CompactBlock| {
            matches!(
                derive_chain_state(&from, &[b]),
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            )
        };
        let mut short = block.clone();
        short.vtx[0].outputs[0].cmu.truncate(31);
        assert!(refused(short), "a 31-byte cmu is refused, never a panic");
        let mut noncanonical = block.clone();
        noncanonical.vtx[0].actions[0].cmx = vec![0xFF; 32];
        assert!(
            refused(noncanonical),
            "a non-canonical cmx is refused (the scanner's EncodingInvalid class)"
        );
        let mut lying = block.clone();
        lying
            .chain_metadata
            .as_mut()
            .expect("the fixture block carries chain metadata")
            .orchard_commitment_tree_size = 3;
        assert!(
            refused(lying),
            "P6: a block whose metadata claims a size its commitments do not reach is refused"
        );
        let mut unchecked = block.clone();
        unchecked.chain_metadata = None;
        assert!(
            derive_chain_state(&from, &[unchecked]).is_ok(),
            "a block without metadata folds unchecked, as the scanner treats it"
        );
    }

    #[tokio::test]
    async fn the_download_refuses_a_block_whose_metadata_disagrees_before_the_cache() {
        // §4t pin (b): `ironwood_desynced_block` (the T0-1 B3 fixture — a block
        // claiming 7 Ironwood commitments while carrying none) through the SHIPPED
        // `download_and_derive`: the fold's per-block size check refuses it as
        // the endpoint's (`EndpointMisbehaving` — never `StoreCorrupt`, never a
        // panic) and nothing reaches the cache, where the base let it through to
        // the scanner's own refusal one phase later
        // (`ironwood_treestate_desync_is_not_reported_as_a_destroyed_wallet`). The
        // honest control: the same block with metadata that tells the truth folds
        // through the same seam, carries the next anchor, and lands in the cache.
        use super::testing::{FakeScanClient, chain_tree_state};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let h = 1_000_000u64;
        let from = chain_tree_state(h - 1)
            .to_chain_state()
            .expect("the anchor decodes");
        assert!(
            matches!(
                derive_chain_state(&from, &[ironwood_desynced_block(h, 7)]),
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "the pure seam refuses the desynced block"
        );
        let cancel = CancelToken::new();
        let ctx = ScanCtx {
            tip: h + 1,
            cancel: &cancel,
            report: &|_| {},
            summary: ProgressSnapshot::default(),
        };
        let mut client = FakeScanClient::with_blocks(vec![Ok(Some(ironwood_desynced_block(h, 7)))]);
        let r = download_and_derive(&mut client, &cache, h, h + 1, ctx, &from).await;
        assert!(
            matches!(
                r,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "the shipped download refuses it as the endpoint's; got {r:?}"
        );
        assert!(
            cache.read(h, h + 1).await.expect("read").is_empty(),
            "a refused block never reaches the cache"
        );
        let mut honest = FakeScanClient::with_blocks(vec![Ok(Some(ironwood_desynced_block(h, 0)))]);
        let (outcome, derived) = download_and_derive(&mut honest, &cache, h, h + 1, ctx, &from)
            .await
            .expect("an honest block downloads");
        assert_eq!(outcome, DownloadOutcome::Completed { outputs: 0 });
        let derived = derived.expect("a completed download carries the next anchor");
        assert_eq!(u64::from(derived.state.block_height()), h);
        assert_eq!(
            cache.read(h, h + 1).await.expect("read").len(),
            1,
            "the honest block is cached"
        );
    }

    #[test]
    fn rewind_target_saturates_at_genesis() {
        // §8 (gate-7, the reorg money-arithmetic the reorg arm depends on): the rewind
        // height is REWIND_DISTANCE_BLOCKS below the error, SATURATING at 0 near
        // genesis (never an underflow). The reorg arm itself (a real continuity error
        // + truncate) is an e2e obligation; this pins the pure arithmetic.
        let h = |n: u32| BlockHeight::from_u32(n);
        assert_eq!(rewind_target(h(0)), h(0), "below the distance → genesis");
        assert_eq!(rewind_target(h(5)), h(0), "below the distance → genesis");
        assert_eq!(rewind_target(h(10)), h(0), "exactly the distance → genesis");
        assert_eq!(
            rewind_target(h(1_000_000)),
            h(999_990),
            "well above → at − {REWIND_DISTANCE_BLOCKS}"
        );
    }

    #[test]
    fn map_scan_err_passes_through_the_cache_error_and_classifies_our_db() {
        // §8 (honest error attribution): a `BlockSource` (cache) fault keeps its
        // precise typed error (DiskFull stays DiskFull — the disposable cache is
        // recoverable, not "your wallet is corrupt"); a non-continuity `Scan` is the
        // endpoint's bad block (`endpoint_unusable`, retry/fallback).
        let passthrough =
            map_scan_err::<WalletError>(ChainScanError::BlockSource(WalletError::DiskFull));
        assert!(
            matches!(passthrough, WalletError::DiskFull),
            "a cache DiskFull passes through, not collapsed to StoreCorrupt; got {passthrough:?}"
        );
    }

    /// Build the production `Wallet`-arm error (`SqliteClientError`) carrying a specific
    /// SQLite result code — the "vector" for the #371 engine-seam classification. `ffi`
    /// flattens extended codes to the primary `ErrorCode`, exactly what the mapper reads.
    fn wallet_scan_err(
        code: std::os::raw::c_int,
    ) -> ChainScanError<SqliteClientError, WalletError> {
        let sqlite = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
        ChainScanError::Wallet(SqliteClientError::DbError(sqlite))
    }

    /// Build the SAME SQLite fault but arriving through the COMMITMENT-TREE (shard) write
    /// door — `SqliteClientError::CommitmentTree(ShardTreeError::Storage(Query(..)))` — the
    /// path the reliability review proved was the LIKELIEST real mid-scan ENOSPC site and
    /// the one the first cut left folding to `StoreCorrupt`.
    fn shard_scan_err(code: std::os::raw::c_int) -> ChainScanError<SqliteClientError, WalletError> {
        ChainScanError::Wallet(SqliteClientError::CommitmentTree(shard_storage_err(code)))
    }

    /// The same `Storage(Query(..))` shard fault, bare — so each of the THREE
    /// `SqliteClientError` commitment-tree variants can be built around it.
    fn shard_storage_err(code: std::os::raw::c_int) -> ShardTreeError<commitment_tree::Error> {
        let sqlite = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
        ShardTreeError::Storage(commitment_tree::Error::Query(sqlite))
    }

    /// The 0.22.0 per-block-range shard door — the variant the SCAN path now actually
    /// produces (`PutBlocksError::ShardTreeForBlockRange` → `error.rs:594`).
    fn put_blocks_shard_err(
        code: std::os::raw::c_int,
    ) -> ChainScanError<SqliteClientError, WalletError> {
        ChainScanError::Wallet(SqliteClientError::PutBlocksCommitmentTree {
            pool: zcash_protocol::ShieldedPool::Orchard,
            block_range: BlockHeight::from_u32(100)..BlockHeight::from_u32(200),
            error: shard_storage_err(code),
        })
    }

    /// The 0.22.0 truncate (reorg rewind) shard door.
    fn truncate_shard_err(
        code: std::os::raw::c_int,
    ) -> ChainScanError<SqliteClientError, WalletError> {
        ChainScanError::Wallet(SqliteClientError::TruncateCommitmentTree {
            pool: zcash_protocol::ShieldedPool::Ironwood,
            height: BlockHeight::from_u32(150),
            error: shard_storage_err(code),
        })
    }

    #[test]
    fn map_scan_err_classifies_the_wallet_db_fault_371() {
        // #371 — the review MED: a fault WRITING our wallet DB mid-scan must not
        // blanket-fold to `StoreCorrupt` (the RED "restore your recovery phrase" scare)
        // when the true cause is out-of-disk / transient-busy / a locked-device IO fault.
        // This discharges the e2e obligation the old test noted (it needed a real
        // `SqliteClientError` to exercise the arm — now constructed directly).

        // SQLITE_FULL: out of disk → the honest `DiskFull`, matching the cache side.
        assert!(
            matches!(
                map_scan_err(wallet_scan_err(rusqlite::ffi::SQLITE_FULL)),
                WalletError::DiskFull
            ),
            "a mid-scan SQLITE_FULL is DiskFull, never StoreCorrupt"
        );
        // SQLITE_BUSY (and _SNAPSHOT, which flattens to DatabaseBusy): a lost WAL writer
        // race → the transient, retryable `StoreBusy`; the store is intact.
        assert!(
            matches!(
                map_scan_err(wallet_scan_err(rusqlite::ffi::SQLITE_BUSY)),
                WalletError::StoreBusy
            ),
            "a mid-scan SQLITE_BUSY is the transient StoreBusy, never StoreCorrupt"
        );
        // SQLITE_IOERR (iOS Data Protection device-locked / ENOSPC-masked): the honest
        // generic `Io` — no seed-restore scare, no possibly-wrong "free space."
        assert!(
            matches!(
                map_scan_err(wallet_scan_err(rusqlite::ffi::SQLITE_IOERR)),
                WalletError::Io(_)
            ),
            "a mid-scan SQLITE_IOERR is the generic Io, never StoreCorrupt"
        );
        // A genuine corruption code stays the fail-closed `StoreCorrupt` default.
        assert!(
            matches!(
                map_scan_err(wallet_scan_err(rusqlite::ffi::SQLITE_CORRUPT)),
                WalletError::StoreCorrupt
            ),
            "SQLITE_CORRUPT remains StoreCorrupt — the fail-closed default is unchanged"
        );
        // A non-SQLite ENOSPC surfaced at the std IO layer routes the ONE `from_io`
        // door → `DiskFull` (parity with the SQLITE_FULL path).
        let io_full = ChainScanError::Wallet(SqliteClientError::Io(std::io::Error::from(
            std::io::ErrorKind::StorageFull,
        )));
        assert!(
            matches!(map_scan_err(io_full), WalletError::DiskFull),
            "a std-layer StorageFull routes through from_io → DiskFull"
        );
    }

    /// Every `std::io::ErrorKind` stable at the MSRV — the R12 §4.5 test 6 unit row
    /// runs each one through both classifier arms.
    const EVERY_IO_ERROR_KIND: &[std::io::ErrorKind] = {
        use std::io::ErrorKind::*;
        &[
            NotFound,
            PermissionDenied,
            ConnectionRefused,
            ConnectionReset,
            HostUnreachable,
            NetworkUnreachable,
            ConnectionAborted,
            NotConnected,
            AddrInUse,
            AddrNotAvailable,
            NetworkDown,
            BrokenPipe,
            AlreadyExists,
            WouldBlock,
            NotADirectory,
            IsADirectory,
            DirectoryNotEmpty,
            ReadOnlyFilesystem,
            StaleNetworkFileHandle,
            InvalidInput,
            InvalidData,
            TimedOut,
            WriteZero,
            StorageFull,
            NotSeekable,
            QuotaExceeded,
            FileTooLarge,
            ResourceBusy,
            ExecutableFileBusy,
            Deadlock,
            CrossesDevices,
            TooManyLinks,
            InvalidFilename,
            ArgumentListTooLong,
            Interrupted,
            Unsupported,
            UnexpectedEof,
            OutOfMemory,
            Other,
        ]
    };

    /// **R12 §4.1 / §4.5 test 6 — decode is corruption (B1), the every-kind row.**
    /// The engine raises `SqliteClientError::Io` when it fails to DECODE our own
    /// stored bytes (`parse_tx`), and `commitment_tree::Error::Serialization` when a
    /// shard does not read back; at the pin neither comes from a file syscall. So
    /// both arms map `StorageFull` → `DiskFull` and EVERY other kind → `StoreCorrupt`
    /// — not `Io`, which since R10 renders "retrying" forever over a damaged row.
    /// A garbage row almost always fails on `InvalidData`/`UnexpectedEof`;
    /// `zcash_primitives` also decodes with `InvalidInput`, and this row is the only
    /// one that reaches every kind (narrowing the rule to a list reds here and
    /// nowhere else). RED before the sweep: `from_io` sends all but one kind to `Io`.
    #[test]
    fn every_io_error_kind_from_the_engine_is_corruption_except_a_full_disk() {
        let mut wrong = Vec::new();
        for kind in EVERY_IO_ERROR_KIND.iter().copied() {
            let engine = SqliteClientError::Io(std::io::Error::from(kind)).into_store_fault();
            let shard = commitment_tree::Error::Serialization(std::io::Error::from(kind))
                .into_store_fault();
            for (arm, got) in [("SqliteClientError::Io", engine), ("Serialization", shard)] {
                let right = if kind == std::io::ErrorKind::StorageFull {
                    matches!(got, WalletError::DiskFull)
                } else {
                    matches!(got, WalletError::StoreCorrupt)
                };
                if !right {
                    wrong.push(format!("{arm}({kind:?}) → {got:?}"));
                }
            }
        }
        // A message-carrying error of an `Other` kind (the shape `io::Error::other`
        // builds) reads the same as the bare kind.
        let other = SqliteClientError::Io(std::io::Error::other("short buffer")).into_store_fault();
        if !matches!(other, WalletError::StoreCorrupt) {
            wrong.push(format!("SqliteClientError::Io(other(..)) → {other:?}"));
        }
        assert!(
            wrong.is_empty(),
            "R12 §4.1: the engine's Io and the shard Serialization arm decode OUR bytes — \
             StorageFull is DiskFull and every other kind is StoreCorrupt, never the \
             'retrying' Io:\n{}",
            wrong.join("\n")
        );
    }

    #[test]
    fn map_scan_err_classifies_the_commitment_tree_shard_door_371() {
        // The reliability review's HIGH: a mid-scan ENOSPC is MOST likely to fault on the
        // large shard-BLOB write, which arrives as
        // `CommitmentTree(ShardTreeError::Storage(Query(SqliteFailure{FULL})))` — NOT the
        // `DbError` door. The first cut folded it to `StoreCorrupt` (the seed-restore scare
        // this fix exists to kill). It must classify identically to the direct door.
        assert!(
            matches!(
                map_scan_err(shard_scan_err(rusqlite::ffi::SQLITE_FULL)),
                WalletError::DiskFull
            ),
            "a shard-write SQLITE_FULL is DiskFull too — the likeliest real path is honest"
        );
        assert!(
            matches!(
                map_scan_err(shard_scan_err(rusqlite::ffi::SQLITE_BUSY)),
                WalletError::StoreBusy
            ),
            "a shard-write SQLITE_BUSY is the transient StoreBusy"
        );
        assert!(
            matches!(
                map_scan_err(shard_scan_err(rusqlite::ffi::SQLITE_IOERR)),
                WalletError::Io(_)
            ),
            "a shard-write SQLITE_IOERR is the generic Io"
        );
        assert!(
            matches!(
                map_scan_err(shard_scan_err(rusqlite::ffi::SQLITE_CORRUPT)),
                WalletError::StoreCorrupt
            ),
            "a shard-write SQLITE_CORRUPT stays the fail-closed StoreCorrupt"
        );
        // A shard SERIALIZATION io fault (StorageFull) routes the same `from_io` door.
        let ser_full = ChainScanError::<SqliteClientError, WalletError>::Wallet(
            SqliteClientError::CommitmentTree(ShardTreeError::Storage(
                commitment_tree::Error::Serialization(std::io::Error::from(
                    std::io::ErrorKind::StorageFull,
                )),
            )),
        );
        assert!(
            matches!(map_scan_err(ser_full), WalletError::DiskFull),
            "a shard Serialization StorageFull → DiskFull via from_io"
        );

        // ALL THREE commitment-tree variants, not just the one this test used to
        // construct. `zcash_client_sqlite` 0.22.0 split the door: the scan path's own
        // per-block-range write now arrives as `PutBlocksCommitmentTree` and the reorg
        // rewind as `TruncateCommitmentTree`. Before this was added, the classifier
        // matched only `CommitmentTree`, this test hand-built only `CommitmentTree`,
        // and #371's guarantee was DEAD on the path it exists for while the test
        // stayed green — the inert-guard shape, caught by review rather than by CI.
        // Every branch of every variant is exercised, so the helper cannot regress in
        // one arm and pass in another.
        for (name, build) in [
            (
                "PutBlocksCommitmentTree (the scan write path)",
                put_blocks_shard_err
                    as fn(std::os::raw::c_int) -> ChainScanError<SqliteClientError, WalletError>,
            ),
            (
                "TruncateCommitmentTree (the reorg rewind)",
                truncate_shard_err,
            ),
        ] {
            assert!(
                matches!(
                    map_scan_err(build(rusqlite::ffi::SQLITE_FULL)),
                    WalletError::DiskFull
                ),
                "{name}: SQLITE_FULL must be DiskFull, never the seed-restore scare"
            );
            assert!(
                matches!(
                    map_scan_err(build(rusqlite::ffi::SQLITE_BUSY)),
                    WalletError::StoreBusy
                ),
                "{name}: SQLITE_BUSY is the transient StoreBusy"
            );
            assert!(
                matches!(
                    map_scan_err(build(rusqlite::ffi::SQLITE_IOERR)),
                    WalletError::Io(_)
                ),
                "{name}: SQLITE_IOERR is the generic Io"
            );
            assert!(
                matches!(
                    map_scan_err(build(rusqlite::ffi::SQLITE_CORRUPT)),
                    WalletError::StoreCorrupt
                ),
                "{name}: SQLITE_CORRUPT stays the fail-closed StoreCorrupt"
            );
        }
        // A tree LOGIC fault (Query/Insert, NOT Storage) stays StoreCorrupt — local
        // inconsistency, correctly NOT reclassified as a disk/io condition.
        let logic = ChainScanError::<SqliteClientError, WalletError>::Wallet(
            SqliteClientError::CommitmentTree(ShardTreeError::Query(
                shardtree::error::QueryError::CheckpointPruned,
            )),
        );
        assert!(
            matches!(map_scan_err(logic), WalletError::StoreCorrupt),
            "a shard tree-logic (Query) fault stays StoreCorrupt, never a false DiskFull"
        );
    }

    #[test]
    fn map_shardtree_err_classifies_the_storage_door_371() {
        // The pre-scan subtree-root write (`put_subtree_roots`) is a scan-path store write:
        // an out-of-disk on it must be `DiskFull`, not the `StoreCorrupt` scare (#371). A
        // tree-LOGIC `Query` stays `StoreCorrupt`.
        //
        // **REPAIRED BY BIND-1-R (§4x-R R9), and the repair is the row's own vacuity.**
        // The third assertion hand-constructed `InsertionError::MarkedRetentionInvalid`
        // and called it "a server-root Insert conflict". It is not one, and no serve
        // can produce it: it fires only for `Retention::Marked` or
        // `Checkpoint { marking: Marking::Marked }` (`shardtree-0.7.1 lib.rs:415-421`),
        // and all three truncate-door `insert_frontier` call sites pass
        // `Marking::None` (`zcash_client_sqlite-0.22.0 wallet.rs:4668`, `:4685`,
        // `:4701`) while the scan door reaches the tree through `insert_frontier` too.
        // So the row pinned the endpoint reading of a variant production never raises
        // — the same shape as the three-variant arm's own "the guard test kept
        // passing, because it hand-constructs the one variant production no longer
        // produces". It now asserts what that variant actually MEANS, which since R9
        // is the local classification.
        //
        // **The endpoint half is not dropped; it moves to the row that proves it with
        // a REAL error**: `mutated_root_reput_is_insert_conflict_not_silent_overwrite`
        // drives a genuine `InsertionError::Conflict` through `put_subtree_roots` and
        // asserts `Sync { EndpointMisbehaving }`. It cannot be asserted here because
        // `Conflict` carries an `incrementalmerkletree::Address` and this crate has no
        // direct dependency on that crate — §4b owed row 5, a supply-chain decision
        // the maintainer has not taken — so a hand-constructed one is not available at
        // all, which is part of why the un-constructible variant was reached for.
        let full = ShardTreeError::Storage(commitment_tree::Error::Query(
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL),
                None,
            ),
        ));
        assert!(
            matches!(map_shardtree_err(full), WalletError::DiskFull),
            "a shard-root-write SQLITE_FULL is DiskFull, never StoreCorrupt"
        );
        let logic = ShardTreeError::<commitment_tree::Error>::Query(
            shardtree::error::QueryError::CheckpointPruned,
        );
        assert!(
            matches!(map_shardtree_err(logic), WalletError::StoreCorrupt),
            "a tree-logic Query stays StoreCorrupt"
        );
        for (local, why) in [
            (
                shardtree::error::InsertionError::MarkedRetentionInvalid,
                "a retention fault: it needs a Marked retention, and no call site on \
                 either door passes one",
            ),
            (
                shardtree::error::InsertionError::TreeFull,
                "a capacity fault: raised only by `append`, which neither door calls",
            ),
            (
                shardtree::error::InsertionError::CheckpointOutOfOrder,
                "an ordering fault: raised only by `ShardTree::append`, unreachable \
                 through `insert_frontier`",
            ),
        ] {
            assert!(
                matches!(
                    map_shardtree_err(ShardTreeError::<commitment_tree::Error>::Insert(local)),
                    WalletError::StoreCorrupt
                ),
                "{why} — so it is OURS, and telling a user to switch servers forever \
                 for a fault that travels with them is the fail-OPEN direction"
            );
        }
    }

    /// §4l Q-F2 at the mapper (INC-021): the ONE constraint code a served value
    /// can trip on the shard write — `SQLITE_CONSTRAINT_UNIQUE` on `root_unique` —
    /// is the endpoint's; the sibling constraint codes and the bare
    /// `SQLITE_CONSTRAINT` stay local, and #371's `DiskFull` reading is untouched.
    /// Mutant: compare `f.code == ConstraintViolation` instead of the extended
    /// code → the PRIMARYKEY and bare rows go red (the rule widened past its
    /// evidence); delete the arm → the first assertion reads `StoreCorrupt`.
    #[test]
    fn map_shardtree_err_reads_a_unique_root_collision_as_the_endpoints_and_nothing_else() {
        let storage = |code: std::ffi::c_int| {
            ShardTreeError::Storage(commitment_tree::Error::Query(
                rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None),
            ))
        };
        assert!(
            matches!(
                map_shardtree_err(storage(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE)),
                WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                }
            ),
            "a UNIQUE failure on the served-root write is the endpoint's duplicate root"
        );
        for (code, name) in [
            (rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY, "PRIMARYKEY"),
            (rusqlite::ffi::SQLITE_CONSTRAINT_NOTNULL, "NOTNULL"),
            (rusqlite::ffi::SQLITE_CONSTRAINT, "bare CONSTRAINT"),
        ] {
            assert!(
                matches!(map_shardtree_err(storage(code)), WalletError::StoreCorrupt),
                "{name}: no served value reaches it, so it stays ours"
            );
        }
        assert!(
            matches!(
                map_shardtree_err(storage(rusqlite::ffi::SQLITE_FULL)),
                WalletError::DiskFull
            ),
            "#371's reading survives the new arm"
        );
    }

    /// §4l Q-F2 at the write (INC-021's own geometry, the implementer's pin; F2
    /// through `update_subtree_roots` is the test author's): two Sapling roots,
    /// one `root_hash`, through the REAL keyed store — the typed content-tier
    /// refusal, never `StoreCorrupt`; the transaction rolled back, so NOTHING was
    /// written (the row count through the second connection is what proves it);
    /// and an honest serve straight after is accepted. Before T0-1d this read
    /// `Err(StoreCorrupt)` (the measurement, re-measured at `2f743fb0`).
    #[test]
    fn a_duplicate_root_hash_in_one_serve_is_the_endpoint_signal_at_the_write() {
        let dir = tempdir().expect("tempdir");
        let (mut conn, key) = fresh_conn(dir.path());
        let aux = fresh_aux(dir.path(), &key);
        let rows = || -> i64 {
            aux.query_row("SELECT COUNT(*) FROM sapling_tree_shards", [], |r| r.get(0))
                .expect("count")
        };

        // Two real testnet completion heights, ONE root hash (tag 9 twice).
        let duplicate = SubtreeRoots::from_pools(
            vec![
                parse_sapling_root(&canonical_root(490_074, 9)).expect("ok"),
                parse_sapling_root(&canonical_root(600_000, 9)).expect("ok"),
            ],
            Vec::new(),
            Vec::new(),
        );
        let r = put_subtree_roots(&mut conn, &duplicate, |_| Ok(()));
        assert!(
            matches!(
                r,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "one root hash at two indices is the endpoint's signal, never StoreCorrupt — got {r:?}"
        );
        assert_eq!(
            rows(),
            0,
            "the refused write rolled back: no shard row landed"
        );

        // The honest serve after it: distinct hashes, accepted, both rows present.
        let honest = SubtreeRoots::from_pools(
            vec![
                parse_sapling_root(&canonical_root(490_074, 9)).expect("ok"),
                parse_sapling_root(&canonical_root(600_000, 10)).expect("ok"),
            ],
            Vec::new(),
            Vec::new(),
        );
        put_subtree_roots(&mut conn, &honest, |_| Ok(())).expect("the honest re-serve is accepted");
        assert_eq!(rows(), 2);
    }

    // ── operational 3-lens (real-world edge cases) ───────────────────────────

    #[tokio::test]
    async fn download_range_open_fault_then_retry_converges() {
        // §8 (unstable-network 3-lens): the most common real-world first-sync flow —
        // the block stream faults at OPEN (a flaky link drops before any block); the
        // attempt is typed `Sync` and caches NOTHING, then a healed retry downloads
        // the full span (the cache insert is idempotent). The open-fault arm
        // (`block_open_err`) the pre-commit tests left uncovered.
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![
            Ok(Some(compact_block(200))),
            Ok(Some(compact_block(201))),
        ]);
        client.block_open_err = Some(GrpcError::Transport {
            stall: StallReason::EndpointUnreachable,
        });
        let r1 = dl(&mut client, &cache, 200, 202).await;
        assert!(
            matches!(r1, Err(WalletError::Sync { .. })),
            "a stream-open fault is typed Sync; got {r1:?}"
        );
        assert!(
            cache.read(200, 202).await.expect("read").is_empty(),
            "a failed download caches nothing"
        );
        // the link heals (block_open_err auto-cleared via take()): retry converges
        dl(&mut client, &cache, 200, 202)
            .await
            .expect("the healed retry downloads the full span");
        assert_eq!(
            cache.read(200, 202).await.expect("read").len(),
            2,
            "the retry caches both blocks (idempotent)"
        );
    }

    #[tokio::test]
    async fn fetch_chain_state_wrong_height_anchor_is_caught_by_the_scan_guard() {
        // §8 (money 3-lens): a server that returns a tree state for the WRONG height
        // (mis-anchoring a scan) is caught — `fetch_chain_state` carries the server's
        // claimed height, and `scan_batch`'s guard rejects the mismatch as typed
        // `Sync`, NEVER a scan against a lied-about anchor. The two-stage defense
        // composed end-to-end (the real path, not a hand-built ChainState).
        use super::testing::{FakeScanClient, tree_state_at};
        let dir = tempdir().expect("tempdir");
        let (mut conn, k) = fresh_conn(dir.path());
        let aux = fresh_aux(dir.path(), &k);
        let cache = fresh_cache(dir.path());
        // ask for the anchor below 600_000; the server lies, anchoring at 700_000.
        let mut client = FakeScanClient::with_tree_state(tree_state_at(700_000));
        let from_state = fetch_chain_state(&mut client, 599_999)
            .await
            .expect("decodes at the server's claimed height");
        assert_eq!(
            u64::from(from_state.block_height()),
            700_000,
            "the anchor carries the server's height, not the requested one"
        );
        let r = scan_batch(
            Network::Test,
            &cache,
            &mut conn,
            &aux,
            600_000,
            &from_state,
            10,
            BlockHeight::from_u32(600_010),
        );
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "the wrong-height anchor is caught by the scan guard; got {r:?}"
        );
    }

    #[tokio::test]
    async fn download_range_single_block_span_round_trips() {
        // §8 (boundary 3-lens): the smallest non-empty range `[s, s+1)` — one block,
        // INCLUSIVE wire `[s, s]` — round-trips (the Verify-range fence-post; an
        // off-by-one in the half-open→inclusive glue would surface here).
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![Ok(Some(compact_block(900)))]);
        dl(&mut client, &cache, 900, 901)
            .await
            .expect("a single-block span downloads");
        assert_eq!(
            client.last_block_range,
            Some((900, 900)),
            "[900,901) → inclusive wire [900,900]"
        );
        assert_eq!(cache.read(900, 901).await.expect("read").len(), 1);
    }

    #[tokio::test]
    async fn download_range_empty_range_is_a_noop() {
        // §8 (the defensive `end <= start` guard, the underflow fix): a degenerate
        // range downloads nothing — no RPC, no `end - 1` underflow — and is Ok.
        use super::testing::FakeScanClient;
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut client = FakeScanClient::with_blocks(vec![]);
        dl(&mut client, &cache, 500, 500)
            .await
            .expect("an empty range is a no-op Ok");
        assert_eq!(
            client.last_block_range, None,
            "no block_range RPC is issued for an empty range"
        );
    }

    // ── cooperative-cancel + intra-batch progress (iv-d-3-b) ─────────────────

    #[tokio::test]
    async fn download_range_reports_intra_batch_progress() {
        // §8 (§3.2g iv-d-3-b obligation 2): a download emits a ScanProgress sample
        // every PROGRESS_REPORT_BLOCKS blocks (each re-arms the d-3 watchdog so a
        // slow-but-advancing download is never falsely restarted). A 60-block span
        // ⇒ 60 / PROGRESS_REPORT_BLOCKS samples, the first at the DOWNLOAD frontier
        // `start + PROGRESS_REPORT_BLOCKS`, the last at the span end; each carries the
        // tip. (Asserted against the constant so a cadence change re-checks here.)
        use super::testing::{FakeScanClient, compact_block};
        use crate::money::BlockHeight;
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let span = 60u64;
        let blocks: Vec<_> = (100..100 + span)
            .map(|h| Ok(Some(compact_block(h))))
            .collect();
        let mut client = FakeScanClient::with_blocks(blocks);

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<ScanProgress>::new()));
        let sink = std::sync::Arc::clone(&seen);
        let report = move |p: ScanProgress| sink.lock().expect("lock").push(p);
        let cancel = CancelToken::new();
        let ctx = ScanCtx {
            tip: 200,
            cancel: &cancel,
            report: &report,
            summary: ProgressSnapshot::default(),
        };
        let out = download_range(&mut client, &cache, 100, 100 + span, ctx)
            .await
            .expect("the span downloads");
        // the fixture chain is empty blocks ⇒ zero shielded outputs (SCAN-1)
        assert_eq!(out, DownloadOutcome::Completed { outputs: 0 });

        let seen = seen.lock().expect("lock");
        assert_eq!(
            seen.len(),
            (span / u64::from(PROGRESS_REPORT_BLOCKS)) as usize,
            "one sample per PROGRESS_REPORT_BLOCKS blocks"
        );
        assert_eq!(
            seen[0].frontier,
            BlockHeight::new(100 + PROGRESS_REPORT_BLOCKS),
            "the first sample frontier is the download position start + the cadence"
        );
        assert_eq!(
            seen.last().expect("≥ 1 sample").frontier,
            BlockHeight::new(100 + span as u32),
            "the last sample frontier is the span end"
        );
        assert!(
            seen.iter().all(|p| p.tip == BlockHeight::new(200)),
            "every sample carries the recorded tip"
        );
    }

    #[tokio::test]
    async fn download_range_cancel_mid_stream_aborts_and_caches_nothing() {
        // §8 (§3.2g iv-d-3-b obligations 1 + 3): a cancel flipped DURING the download
        // (here, AT the first intra-batch sample) is honored at the next block — the
        // download returns `Cancelled` (NOT an error), and because the in-flight
        // buffer is only written to the cache on a FULL span, NOTHING is cached. So a
        // mid-download stop loses only re-downloadable work, never a torn cache.
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let blocks: Vec<_> = (100..200).map(|h| Ok(Some(compact_block(h)))).collect();
        let mut client = FakeScanClient::with_blocks(blocks);

        let cancel = CancelToken::new();
        let trip = cancel.clone();
        // Flip cancel at the first intra-batch sample (download frontier start +
        // PROGRESS_REPORT_BLOCKS); the next per-block check then returns Cancelled.
        let report = move |_p: ScanProgress| trip.cancel();
        let ctx = ScanCtx {
            tip: 300,
            cancel: &cancel,
            report: &report,
            summary: ProgressSnapshot::default(),
        };
        let out = download_range(&mut client, &cache, 100, 200, ctx)
            .await
            .expect("a cancelled download is Ok(Cancelled), never an error");
        assert_eq!(out, DownloadOutcome::Cancelled);
        assert!(
            cache.read(100, 200).await.expect("read").is_empty(),
            "a mid-download cancel caches NOTHING (the in-flight buffer is dropped)"
        );
    }

    #[test]
    fn scan_progress_sample_clamps_frontier_to_tip() {
        // §8: ScanProgress::sample carries the raw position + tip and clamps the
        // frontier to the tip (a server streaming past the tip — rejected upstream —
        // can never render a position beyond the chain). The authoritative MONOTONIC
        // UI percent is NOT computed here (it is the d-3-b engine's wallet-summary
        // derivation — see the ScanProgress doc); this type is the per-range position
        // + the watchdog tick only, so there is no percent math to misorder.
        use crate::money::BlockHeight;
        let s = ProgressSnapshot::default();
        let mid = ScanProgress::sample(150, 200, s);
        assert_eq!(mid.frontier, BlockHeight::new(150));
        assert_eq!(mid.tip, BlockHeight::new(200));

        let over = ScanProgress::sample(300, 200, s);
        assert_eq!(
            over.frontier,
            BlockHeight::new(200),
            "a frontier past the tip clamps to the tip"
        );
        assert_eq!(
            ScanProgress::sample(0, 0, s).frontier,
            BlockHeight::new(0),
            "tip 0 (no chain yet) is sane, not a panic"
        );
        // The `unwrap_or(u32::MAX)` saturation path — unreachable in production (heights
        // are u32-range-checked upstream) but the defensive contract is pinned.
        let sat = ScanProgress::sample(u64::from(u32::MAX) + 1, u64::from(u32::MAX) + 1, s);
        assert_eq!(sat.frontier, BlockHeight::new(u32::MAX));
        assert_eq!(sat.tip, BlockHeight::new(u32::MAX));
    }

    // ── The driver-loop seam (iv-d-2b-ii) ────────────────────────────────────

    /// A compact tx with shape-valid Sapling outputs / Orchard actions / Ironwood
    /// actions for the SCAN-1 count tests (32-byte `cmu` / `nullifier` + `cmx`, so
    /// `block_is_scannable` passes them).
    fn counted_tx(
        sapling: usize,
        orchard: usize,
        ironwood: usize,
    ) -> zcash_client_backend::proto::compact_formats::CompactTx {
        use zcash_client_backend::proto::compact_formats::{
            CompactOrchardAction, CompactSaplingOutput, CompactTx,
        };
        let out = CompactSaplingOutput {
            cmu: vec![0u8; 32],
            ..Default::default()
        };
        let action = CompactOrchardAction {
            nullifier: vec![0u8; 32],
            cmx: vec![0u8; 32],
            ..Default::default()
        };
        CompactTx {
            txid: vec![0u8; 32],
            outputs: vec![out; sapling],
            actions: vec![action.clone(); orchard],
            ironwood_actions: vec![action; ironwood],
            ..Default::default()
        }
    }

    #[test]
    fn shielded_outputs_counts_sapling_outputs_orchard_and_ironwood_actions() {
        // SCAN-1 (§4o S1, IT-1b item 2 — ONE `outputs`): the batch's public
        // shielded-output count is the sum of the three pools' compact fields, and
        // Ironwood's `ironwood_actions` (compact tag 9) is one of them. Mutants this
        // row was watched against: any one of the three terms dropped from
        // `shielded_outputs` → the 2/3/1 (+4) split below no longer sums to 10.
        use super::testing::compact_block;
        let mut block = compact_block(280_000);
        block.vtx = vec![counted_tx(2, 3, 1), counted_tx(0, 0, 4)];
        assert_eq!(shielded_outputs(&block), 2 + 3 + 1 + 4);
        assert_eq!(shielded_outputs(&compact_block(1)), 0, "an empty block");
        assert!(
            block_is_scannable(&block),
            "the fixture is shape-valid, so the download path below can carry it"
        );
    }

    #[test]
    fn absorb_adds_every_phase_and_the_chain_count_to_the_pass() {
        // SCAN-1 (review, item 3): the pass sums are accumulated in ONE pure
        // fn, so the identity S1 checks in-process — where the fake chain's network
        // phases and chain count are 0 — is not the only guard on the accumulator.
        // Distinct non-zero values per field, two batches, one throttled; mutants:
        // any one `saturating_add` dropped or crossed onto another field → red.
        let mut pass = SyncPass::default();
        pass.absorb(&BatchPhases {
            anchor_ms: 1,
            dl_ms: 20,
            scan_ms: 300,
            snap_ms: Some(4_000),
            chain_outputs: 50_000,
        });
        pass.absorb(&BatchPhases {
            anchor_ms: 2,
            dl_ms: 30,
            scan_ms: 400,
            snap_ms: None,
            chain_outputs: 60_000,
        });
        assert_eq!(
            (
                pass.anchor_ms,
                pass.dl_ms,
                pass.scan_ms,
                pass.snap_ms,
                pass.chain_outputs
            ),
            (3, 50, 700, 4_000, 110_000)
        );
        assert_eq!(
            pass.wall_ms, 0,
            "wall_ms is the pass's own, never a batch's"
        );
        assert_eq!(pass.batches, 0, "absorb counts phases, not batches");
    }

    #[tokio::test]
    async fn download_range_reports_the_shielded_output_count_of_the_span() {
        // SCAN-1 (§4o S1, S9): the count reaches the caller through
        // `DownloadOutcome::Completed { outputs }` on the SHIPPED download path —
        // tallied at the boundary as each block passes `block_is_scannable`, over
        // the whole span. Mutant: the tally left at 0 (the `saturating_add` line
        // dropped) → `outputs` is 0 here.
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let blocks: Vec<_> = (100..110u64)
            .map(|h| {
                let mut b = compact_block(h);
                b.vtx = vec![counted_tx(1, 2, 3)]; // 6 per block
                Ok(Some(b))
            })
            .collect();
        let mut client = FakeScanClient::with_blocks(blocks);
        let report = |_: ScanProgress| {};
        let cancel = CancelToken::new();
        let ctx = ScanCtx {
            tip: 200,
            cancel: &cancel,
            report: &report,
            summary: ProgressSnapshot::default(),
        };
        let out = download_range(&mut client, &cache, 100, 110, ctx)
            .await
            .expect("the span downloads");
        assert_eq!(out, DownloadOutcome::Completed { outputs: 60 });
    }

    #[test]
    fn block_is_scannable_accepts_wellformed_and_rejects_every_panic_field() {
        // §8 (THE hostile-input crash surface, §4.6): the consumed-whole scanner
        // reads height/hash/prev_hash/tx.index/txid/spend.nf/cmu through accessors
        // that PANIC (an FFI crash) on a wrong length/range. A well-formed block is
        // accepted; each malformed field is rejected — so download_range turns a
        // crash into a typed Sync. tx.index + sapling spend nf were the review's
        // BLOCKER (a Result-returning accessor the scanner `.expect()`s still panics).
        use super::testing::compact_block;
        use zcash_client_backend::proto::compact_formats::{
            CompactSaplingOutput, CompactSaplingSpend, CompactTx,
        };

        let ok_tx = || CompactTx {
            txid: vec![0u8; 32],
            ..Default::default()
        };

        assert!(
            block_is_scannable(&compact_block(280_000)),
            "well-formed ok"
        );

        let mut h = compact_block(1);
        h.height = u64::from(u32::MAX) + 1;
        assert!(!block_is_scannable(&h), "height beyond u32");

        let mut short_hash = compact_block(2);
        short_hash.hash = vec![0u8; 31];
        assert!(
            !block_is_scannable(&short_hash),
            "31-byte hash (header absent)"
        );

        let mut empty_prev = compact_block(3);
        empty_prev.prev_hash = Vec::new();
        assert!(!block_is_scannable(&empty_prev), "empty prev_hash");

        // tx.index ≥ 2^16 — the scanner's `TxIndex::try_from(tx.index).expect()`.
        let mut bad_index = compact_block(4);
        bad_index.vtx = vec![CompactTx {
            index: u64::from(u16::MAX) + 1,
            ..ok_tx()
        }];
        assert!(
            !block_is_scannable(&bad_index),
            "tx.index beyond u16 (TxIndex)"
        );
        // exactly u16::MAX is fine (the boundary)
        let mut max_index = compact_block(4);
        max_index.vtx = vec![CompactTx {
            index: u64::from(u16::MAX),
            ..ok_tx()
        }];
        assert!(
            block_is_scannable(&max_index),
            "tx.index == u16::MAX accepted"
        );

        let mut bad_txid = compact_block(5);
        bad_txid.vtx = vec![CompactTx {
            txid: vec![0u8; 16],
            ..Default::default()
        }];
        assert!(!block_is_scannable(&bad_txid), "16-byte txid");

        // sapling spend nf ≠ 32 — `find_spent`'s `spend.nf().expect()` (2nd pass).
        let mut bad_spend = compact_block(6);
        bad_spend.vtx = vec![CompactTx {
            spends: vec![CompactSaplingSpend { nf: vec![0u8; 16] }],
            ..ok_tx()
        }];
        assert!(!block_is_scannable(&bad_spend), "16-byte sapling spend nf");

        let mut bad_cmu = compact_block(7);
        bad_cmu.vtx = vec![CompactTx {
            outputs: vec![CompactSaplingOutput {
                cmu: vec![0u8; 31],
                ..Default::default()
            }],
            ..ok_tx()
        }];
        assert!(!block_is_scannable(&bad_cmu), "31-byte cmu");
    }

    #[tokio::test]
    async fn download_range_rejects_a_malformed_block_never_caches_it() {
        // §8 (THE crash fix end-to-end, §4.6): a CompactBlock with a non-32-byte
        // hash (header absent) would PANIC `scan_cached_blocks` — an FFI crash /
        // remote DoS from a buggy or hostile lightwalletd. `download_range` rejects
        // it as a typed `Sync` BEFORE the cache, never a panic, never cached.
        use super::testing::{FakeScanClient, compact_block};
        let dir = tempdir().expect("tempdir");
        let cache = fresh_cache(dir.path());
        let mut bad = compact_block(100);
        bad.hash = vec![0u8; 31]; // a real lightwalletd never sends this
        let mut client = FakeScanClient::with_blocks(vec![Ok(Some(bad))]);
        let r = dl(&mut client, &cache, 100, 101).await;
        assert!(
            matches!(r, Err(WalletError::Sync { .. })),
            "a malformed block is typed Sync, never a panic; got {r:?}"
        );
        assert!(
            cache.read(100, 101).await.expect("read").is_empty(),
            "the malformed block is never cached"
        );
    }

    #[test]
    fn next_batch_clamps_to_one_batch_and_is_none_when_up_to_date() {
        // §8: the next batch is the highest-priority suggested range CLAMPED to
        // SYNC_BATCH_BLOCKS from the low end; a wallet with no account (nothing to
        // scan) is `None`. Pins the range glue + the lock-budget batch bound (gate 7).
        use super::testing::chain_tree_state;
        use crate::account;
        use zcash_client_backend::data_api::WalletWrite;

        let dir = tempdir().expect("tempdir");
        let (mut conn, _k) = fresh_conn(dir.path());
        // no account ⇒ nothing to scan
        assert_eq!(
            next_batch(&mut conn).expect("suggest"),
            None,
            "an account-less wallet has no batch to scan"
        );

        // import at birthday 280_000 (treestate @ 279_999), tip far above one batch
        let birthday = account::birthday_from_treestate(chain_tree_state(279_999)).expect("bday");
        account::ensure_account(&mut conn, b"relim-wallet-kat-seed-0123456789", &birthday)
            .expect("acct");
        conn.update_chain_tip(BlockHeight::from_u32(283_000))
            .expect("tip");
        let (from, end) = next_batch(&mut conn).expect("suggest").expect("a batch");
        assert_eq!(
            from, 280_000,
            "scans from the birthday-anchored range start"
        );
        assert_eq!(
            end - from,
            u64::from(SYNC_BATCH_BLOCKS),
            "a range wider than one batch is clamped to SYNC_BATCH_BLOCKS"
        );
    }

    #[tokio::test]
    async fn fetch_tip_converts_and_rejects_out_of_range() {
        // §8: the chain tip is range-checked to BlockHeight; a height beyond u32 is
        // a hostile/garbage response → typed Sync, never the from_u32 panic. The
        // honest tip sits ABOVE the mainnet row (T0-1c), so this row measures the
        // conversion and not the grade.
        use super::testing::FakeChain;
        let mut ok = FakeChain::to_tip(3_500_000);
        let fetched = fetch_tip(&mut ok, Network::Main, None).await.expect("tip");
        assert_eq!(u64::from(fetched.tip), 3_500_000);
        assert_eq!(fetched.standing, TipStanding::AtOrAboveBundle);
        let mut hostile = FakeChain::to_tip(u64::from(u32::MAX) + 1);
        assert!(
            matches!(
                fetch_tip(&mut hostile, Network::Main, None).await,
                Err(WalletError::Sync {
                    stall: StallReason::EndpointMisbehaving
                })
            ),
            "a tip beyond u32 is the content-tier stall, never a panic"
        );
    }

    /// T0-1c (§4k-R) — the grade, driven at its edges on both networks through
    /// `fetch_tip`: one below the bundle's newest row is RETURNED (the pass
    /// continues) and graded `BehindBundle` naming that row; the row itself and
    /// one above it are `AtOrAboveBundle`. The mutants: the comparison inverted
    /// or the reference replaced by `0` flip the first clause; firing at equality
    /// flips the second (the at-newest plant's prediction); one block of slack
    /// flips the first (the one-below plant's). A refusing mutant — `Err` on a
    /// behind tip — flips the first clause's `expect`.
    #[tokio::test]
    async fn fetch_tip_returns_a_tip_below_the_bundles_newest_row_and_grades_it_behind() {
        use super::testing::FakeChain;
        for net in [Network::Main, Network::Test] {
            let newest = root_bind::newest_bundled_height(net);
            assert!(
                newest > 0,
                "{net:?}: an empty bundle would disarm the grade"
            );
            let mut behind = FakeChain::to_tip(u64::from(newest) - 1);
            let fetched = fetch_tip(&mut behind, net, None)
                .await
                .expect("a behind tip is returned, not refused (REPORT AND CONTINUE)");
            assert_eq!(
                fetched,
                FetchedTip {
                    tip: BlockHeight::from_u32(newest - 1),
                    standing: TipStanding::BehindBundle {
                        newest_known: newest
                    },
                },
                "{net:?}: one below the newest bundled row is behind, and names the row"
            );
            let mut at = FakeChain::to_tip(u64::from(newest));
            assert_eq!(
                fetch_tip(&mut at, net, None).await.expect("at the row"),
                FetchedTip {
                    tip: BlockHeight::from_u32(newest),
                    standing: TipStanding::AtOrAboveBundle,
                },
                "{net:?}: the row itself is not behind"
            );
            let mut above = FakeChain::to_tip(u64::from(newest) + 1);
            assert_eq!(
                fetch_tip(&mut above, net, None)
                    .await
                    .expect("above the row"),
                FetchedTip {
                    tip: BlockHeight::from_u32(newest + 1),
                    standing: TipStanding::AtOrAboveBundle,
                },
                "{net:?}: above the row is not behind"
            );
        }
    }

    /// The grade is pure and, for a wallet that has scanned nothing, reads only
    /// the compiled bundle: the same answer for the same `(network, tip)` with no
    /// wallet, no client and no clock — which is what lets `fetch_tip` take it
    /// before any RPC that depends on the number.
    #[test]
    fn tip_standing_reads_only_the_bundle() {
        for net in [Network::Main, Network::Test] {
            let newest = root_bind::newest_bundled_height(net);
            assert_eq!(
                tip_standing(net, newest - 1, None),
                TipStanding::BehindBundle {
                    newest_known: newest
                }
            );
            assert_eq!(
                tip_standing(net, newest, None),
                TipStanding::AtOrAboveBundle
            );
            assert_eq!(
                tip_standing(net, u32::MAX, None),
                TipStanding::AtOrAboveBundle
            );
            assert_eq!(
                tip_standing(net, 0, None),
                TipStanding::BehindBundle {
                    newest_known: newest
                },
                "{net:?}: height 0 is behind every non-empty bundle"
            );
        }
    }

    /// T0-1c-R2 (§4n Q-G1, M1) — the grade's REFERENCE at its boundaries, pure:
    /// the larger of the bundle's row and the wallet's own scanned height less
    /// `REORG_MAX_BLOCKS`. A wallet that has scanned nothing, or less than the
    /// margin, or to below the row plus the margin, grades against the row
    /// alone; one scanned past `row + REORG_MAX_BLOCKS` grades against its own
    /// height less the margin. The mutants: the `max` reverted to the row alone
    /// (G8's named mutant — the third and fourth clauses red); the margin
    /// replaced by `REWIND_DISTANCE_BLOCKS` (the fourth clause red: the reference
    /// would sit 90 blocks higher); the subtraction unsaturated (the second
    /// clause panics in debug).
    #[test]
    fn the_reference_is_the_larger_of_the_row_and_our_own_height_less_the_reorg_margin() {
        let row = 3_459_780;
        assert_eq!(tip_reference(row, None), row, "nothing scanned: the row");
        assert_eq!(
            tip_reference(row, Some(REORG_MAX_BLOCKS - 1)),
            row,
            "scanned less than the margin: the row (the subtraction saturates)"
        );
        assert_eq!(
            tip_reference(row, Some(row + REORG_MAX_BLOCKS)),
            row,
            "scanned to exactly row + margin: the row (a tie)"
        );
        assert_eq!(
            tip_reference(row, Some(row + REORG_MAX_BLOCKS + 1)),
            row + 1,
            "one past the tie: our own height less the margin wins"
        );
        assert_eq!(
            tip_reference(row, Some(row - 1)),
            row,
            "scanned to below the row: the row"
        );
        assert_eq!(
            tip_reference(0, Some(REORG_MAX_BLOCKS + 5)),
            5,
            "an empty bundle (a disarmed row) still leaves our own height as a reference"
        );
    }

    /// T0-1c-R3 (§4n-R): the guard's boundary, pure. `tip + 1` is the empty
    /// range upstream accepts; `tip + 2` is the inverted one it asserts on; a
    /// wallet scanned PAST the tip is upstream's early return and never guarded;
    /// no account, nothing to guard; the add saturates at the top. The seam
    /// rows are `wallet::tests::controller_over_the_production_engine_…_not_a_dead_loop`
    /// and its siblings. Mutants: the threshold made `tip` (the first clause
    /// red); the scanned clause removed (the fifth red); `saturating_add` made
    /// `+` (the last clause panics in debug).
    #[test]
    fn the_birthday_guard_fires_above_tip_plus_one_and_only_when_nothing_is_scanned_past_the_tip() {
        assert!(
            !birthday_beyond_tip(Some(1_001), 1_000, None),
            "birthday = tip + 1: the empty range upstream accepts"
        );
        assert!(
            !birthday_beyond_tip(Some(1_000), 1_000, None),
            "birthday = tip: one block to scan"
        );
        assert!(
            birthday_beyond_tip(Some(1_002), 1_000, None),
            "birthday = tip + 2: the inverted range, guarded"
        );
        assert!(
            birthday_beyond_tip(Some(4_125_171), 280_100, None),
            "INC-024's second geometry (the adjudicator's P-1: 4125171..280101)"
        );
        assert!(
            !birthday_beyond_tip(Some(1_002), 1_000, Some(1_001)),
            "scanned past the tip: upstream returns early, nothing to guard"
        );
        assert!(
            birthday_beyond_tip(Some(1_002), 1_000, Some(1_000)),
            "scanned to the tip exactly: no early return (tip < max_scanned is false) — guarded"
        );
        assert!(
            !birthday_beyond_tip(None, 1_000, None),
            "no account: upstream ignores the range"
        );
        assert!(
            !birthday_beyond_tip(Some(u32::MAX), u32::MAX, None),
            "a tip at the top saturates instead of overflowing"
        );
    }

    /// T0-1c-R2 (§4n G1 / G5, M1), through the grade: for a wallet scanned to
    /// `H` above the row, a tip more than `REORG_MAX_BLOCKS` below `H` is behind
    /// and names `H − REORG_MAX_BLOCKS` as the reference; a tip exactly
    /// `REORG_MAX_BLOCKS` below `H` — the deepest dip a survivable reorg can
    /// produce — is NOT behind (G5's rewind-sized dip); and the bundle's row
    /// still floors a wallet whose own height sits below `row + margin`. Through
    /// `fetch_tip` too, so the scanned height reaches the grade from the fn the
    /// pass calls. The mutants: the `max` reverted to the row alone (the first
    /// clause red — a tip at or above the row would read at-or-above); the
    /// comparison made `<=` (the second clause red); the margin set to one
    /// rewind step (the second clause red: a 100-block dip would be behind).
    #[tokio::test]
    async fn a_tip_below_our_own_scanned_height_by_more_than_the_reorg_margin_is_behind_and_within_it_is_not()
     {
        use super::testing::FakeChain;
        for net in [Network::Main, Network::Test] {
            let row = root_bind::newest_bundled_height(net);
            let h = row + 10 * REORG_MAX_BLOCKS;
            let reference = h - REORG_MAX_BLOCKS;
            assert_eq!(
                tip_standing(net, reference - 1, Some(h)),
                TipStanding::BehindBundle {
                    newest_known: reference
                },
                "{net:?}: one block deeper than the margin below our own height is behind, \
                 and names our own height less the margin — not the row"
            );
            assert_eq!(
                tip_standing(net, reference, Some(h)),
                TipStanding::AtOrAboveBundle,
                "{net:?}: a dip of exactly the margin reads as a reorg, not a behind server"
            );
            assert_eq!(
                tip_standing(net, h + 1, Some(h)),
                TipStanding::AtOrAboveBundle,
                "{net:?}: a tip above our own height is current"
            );
            // A wallet scanned to just past the row: the row still floors it.
            assert_eq!(
                tip_standing(net, row - 1, Some(row + 1)),
                TipStanding::BehindBundle { newest_known: row },
                "{net:?}: below the row is behind whatever we scanned"
            );
            let mut pinned = FakeChain::to_tip(u64::from(reference) - 1);
            assert_eq!(
                fetch_tip(&mut pinned, net, Some(h))
                    .await
                    .expect("returned, not refused"),
                FetchedTip {
                    tip: BlockHeight::from_u32(reference - 1),
                    standing: TipStanding::BehindBundle {
                        newest_known: reference
                    },
                },
                "{net:?}: the scanned height reaches the grade through fetch_tip"
            );
        }
    }

    /// T0-1c decision 4 (Q-E2) — the filter and the grade agree at the row, on
    /// the compiled bundle, per pool: at or above the newest row the `Withheld`
    /// discriminator's filter admits every bundled row (so the residual
    /// under-reporter — to anywhere at or above it — cannot switch a row off);
    /// one below it admits exactly the rows strictly below (so a behind tip does
    /// lose rows, which is the price `EndpointBehind` now charges on glass); and
    /// below the first row nothing is proven. Every clause runs on every pool —
    /// the row this replaces guarded its second clause on a condition false for
    /// all six `(network, pool)` pairs (ruling owed 7), so it measured nothing
    /// there. The third clause is the one that sees the filter's `<=` flipped on
    /// the real bundle (the two `proven_complete_at_or_below_*` rows see it on a
    /// synthetic one).
    #[test]
    fn at_or_above_the_newest_row_the_filter_admits_every_bundled_row_and_one_below_excludes_it() {
        for net in [Network::Main, Network::Test] {
            let newest = root_bind::newest_bundled_height(net);
            for pool in [
                ShieldedProtocol::Sapling,
                ShieldedProtocol::Orchard,
                ShieldedProtocol::Ironwood,
            ] {
                let bundled = root_bind::bundled_counts(net, pool);
                let whole = bundled.iter().map(|&(_, c)| c).max().unwrap_or(0);
                let below_newest = bundled
                    .iter()
                    .filter(|&&(h, _)| h < newest)
                    .map(|&(_, c)| c)
                    .max()
                    .unwrap_or(0);
                let first_row = bundled
                    .first()
                    .map(|&(h, _)| h)
                    .expect("a non-empty bundle");
                assert_eq!(
                    proven_complete_at_or_below(bundled, newest),
                    whole,
                    "{net:?}/{pool:?}: at the row the filter admits every bundled row"
                );
                assert_eq!(
                    proven_complete_at_or_below(bundled, u32::MAX),
                    whole,
                    "{net:?}/{pool:?}: above the row too"
                );
                assert_eq!(
                    proven_complete_at_or_below(bundled, newest - 1),
                    below_newest,
                    "{net:?}/{pool:?}: one below the row admits exactly the rows below it"
                );
                assert_eq!(
                    proven_complete_at_or_below(bundled, first_row - 1),
                    0,
                    "{net:?}/{pool:?}: below the first row nothing is proven"
                );
            }
        }
    }

    /// §4j row 6 — the `Withheld` discriminator's tip filter, driven directly in
    /// BOTH directions (it had a test in neither): a bundled row the tip covers
    /// counts, a row above the tip does not, and the answer is the largest covered
    /// count. The `<=` is the whole mechanism: flipped to `>=`, the rows ABOVE the
    /// tip are what count and the first three assertions go red.
    #[test]
    fn proven_complete_at_or_below_counts_the_rows_the_tip_covers_and_no_others() {
        let bundled = [(100, 0), (200, 1), (300, 2)];
        // Below every row: nothing is proven.
        assert_eq!(proven_complete_at_or_below(&bundled, 99), 0);
        // A row AT the tip counts — the bound is inclusive.
        assert_eq!(proven_complete_at_or_below(&bundled, 200), 1);
        // A row ABOVE the tip does not — the direction an under-reported tip
        // exploits (INC-023), and the one the pass now REPORTS
        // (`SyncStatus::EndpointBehind`, via `fetch_tip`'s grade) for every tip
        // below the bundle's newest row.
        assert_eq!(proven_complete_at_or_below(&bundled, 299), 1);
        // Everything covered.
        assert_eq!(proven_complete_at_or_below(&bundled, 300), 2);
        assert_eq!(proven_complete_at_or_below(&bundled, u32::MAX), 2);
        // No rows at all ⇒ no proof, never a panic.
        assert_eq!(proven_complete_at_or_below(&[], u32::MAX), 0);
    }

    /// The proof is the MAX over the covered rows, independent of their order: a
    /// bundle re-derivation that reorders rows, or a decode skip that drops one,
    /// cannot lower it below what an earlier covered row already proved.
    #[test]
    fn proven_complete_at_or_below_is_the_largest_covered_count() {
        assert_eq!(proven_complete_at_or_below(&[(200, 1), (100, 3)], 250), 3);
        assert_eq!(proven_complete_at_or_below(&[(200, 1), (100, 3)], 150), 3);
        assert_eq!(proven_complete_at_or_below(&[(200, 1), (100, 3)], 50), 0);
    }

    #[test]
    fn reorg_storm_trips_past_the_consecutive_bound() {
        // §8 (gate 7, money-safety DoS): the per-pass reorg-since-progress bound trips
        // exactly past MAX_SCAN_REORGS_PER_PASS — so a forking/hostile endpoint hits
        // a typed wall, never an unbounded rewind/rescan spin.
        assert!(!reorg_storm(0));
        assert!(
            !reorg_storm(MAX_SCAN_REORGS_PER_PASS),
            "at the bound: still tolerated"
        );
        assert!(
            reorg_storm(MAX_SCAN_REORGS_PER_PASS + 1),
            "one past the bound: stop"
        );
    }

    #[test]
    fn classify_batch_accounts_progress_and_bounds_a_reorg_oscillation() {
        // §8 (the driver loop's money-critical reorg ACCOUNTING — the security fold,
        // tested PURE because a real continuity error can't be produced from
        // hand-built blocks). Covers: high-water advance + counter reset, a Scanned
        // that does NOT advance (no reset), a reorg within budget (Rewind), the storm
        // trip (StallReorg), and — the load-bearing case — an adversarial
        // scan/reorg OSCILLATION terminating instead of pinning the loop forever.
        let mut hw = 0u64;
        let mut n = 5u32; // pretend some reorgs have already accrued

        // a Scanned that advances the frontier resets the counter
        assert_eq!(
            classify_batch(
                ScanOutcome::Scanned {
                    received_notes: 0,
                    boundary: BoundaryReport::default()
                },
                1_000,
                &mut hw,
                &mut n,
                0
            ),
            BatchAction::Advance
        );
        assert_eq!((hw, n), (1_000, 0), "new high-water resets the counter");

        // a reorg within budget rewinds and increments
        assert_eq!(
            classify_batch(
                ScanOutcome::Reorg {
                    rewind_height: 990,
                    requeued: true,
                    rewound_blocks: 10
                },
                0,
                &mut hw,
                &mut n,
                0
            ),
            BatchAction::Rewind {
                rewind_height: 990,
                requeued: true,
                rewound_blocks: 10
            }
        );
        assert_eq!((hw, n), (1_000, 1), "reorg keeps the high-water, counts");

        // a Scanned that does NOT pass the high-water does NOT reset (the crux: a
        // re-scan of rewound territory is not progress)
        assert_eq!(
            classify_batch(
                ScanOutcome::Scanned {
                    received_notes: 0,
                    boundary: BoundaryReport::default()
                },
                995,
                &mut hw,
                &mut n,
                0
            ),
            BatchAction::Advance
        );
        assert_eq!(
            (hw, n),
            (1_000, 1),
            "re-covering rewound ground does NOT reset"
        );

        // THE OSCILLATION: alternate a non-advancing Scanned with a reorg. A naive
        // CONSECUTIVE counter would reset every cycle and spin forever; this counter
        // accumulates across the non-advancing Scanned and trips, so the pass ends.
        let mut hw2 = 1_000u64;
        let mut n2 = 0u32;
        let mut stalled = false;
        for cycle in 0..(MAX_SCAN_REORGS_PER_PASS + 5) {
            // a tiny Scanned that re-covers rewound ground (end ≤ high_water)
            assert_eq!(
                classify_batch(
                    ScanOutcome::Scanned {
                        received_notes: 0,
                        boundary: BoundaryReport::default()
                    },
                    900,
                    &mut hw2,
                    &mut n2,
                    0
                ),
                BatchAction::Advance
            );
            match classify_batch(
                ScanOutcome::Reorg {
                    rewind_height: 890,
                    requeued: true,
                    rewound_blocks: 10,
                },
                0,
                &mut hw2,
                &mut n2,
                0,
            ) {
                BatchAction::Rewind { .. } => {}
                BatchAction::StallReorg => {
                    assert_eq!(
                        cycle, MAX_SCAN_REORGS_PER_PASS,
                        "stalls exactly when reorgs-since-progress passes the bound"
                    );
                    stalled = true;
                    break;
                }
                BatchAction::Advance => unreachable!(),
            }
        }
        assert!(
            stalled,
            "an adversarial scan/reorg oscillation TERMINATES (StallReorg)"
        );
    }

    #[test]
    fn classify_batch_stalls_at_the_absolute_cap_whatever_the_high_water_says() {
        // §4q-R P-RR3 (the REQ-1 fold review's row 2): a fork-advance-fork server
        // resets the since-progress epoch at will (every one-block advance is a
        // new high-water), so the epoch bound alone lets it hold ONE pass for
        // `(tip − high_water) × 11` rewinds. The absolute cap on the pass's own
        // rewind count ends it at the `MAX_TOTAL_REORGS_PER_PASS + 1`-th rewind —
        // with a FRESH high-water and a ZERO epoch, the state the epoch bound calls
        // healthy. Mutant: the cap removed → the two `StallReorg` rows go red
        // (`Rewind`); the constant's derivation row pins the maintainer-movable value.
        let reorg = || ScanOutcome::Reorg {
            rewind_height: 990,
            requeued: true,
            rewound_blocks: 10,
        };

        // one below the cap: this is the cap-th rewind, still tolerated
        let (mut hw, mut n) = (2_000u64, 0u32);
        assert_eq!(
            classify_batch(reorg(), 0, &mut hw, &mut n, MAX_TOTAL_REORGS_PER_PASS - 1),
            BatchAction::Rewind {
                rewind_height: 990,
                requeued: true,
                rewound_blocks: 10
            },
            "the cap-th rewind is still within the cap"
        );

        // at the cap: this is the (cap + 1)-th rewind
        let (mut hw, mut n) = (2_000u64, 0u32);
        assert_eq!(
            classify_batch(reorg(), 0, &mut hw, &mut n, MAX_TOTAL_REORGS_PER_PASS),
            BatchAction::StallReorg,
            "the (MAX_TOTAL_REORGS_PER_PASS + 1)-th rewind stalls with a fresh high-water \
             and a zero epoch"
        );

        // past the cap (`pass.reorgs` at the cap + 1): every further rewind stalls
        let (mut hw, mut n) = (2_000u64, 0u32);
        assert_eq!(
            classify_batch(reorg(), 0, &mut hw, &mut n, MAX_TOTAL_REORGS_PER_PASS + 1),
            BatchAction::StallReorg,
            "past the cap every rewind stalls, whatever the high-water says"
        );

        // the cap is three epochs' worth (§4q-R P-RR3; the maintainer may move it) and
        // the predicate trips exactly past it
        assert_eq!(MAX_TOTAL_REORGS_PER_PASS, 3 * MAX_SCAN_REORGS_PER_PASS);
        assert!(
            !reorg_cap(MAX_TOTAL_REORGS_PER_PASS),
            "at the cap: tolerated"
        );
        assert!(reorg_cap(MAX_TOTAL_REORGS_PER_PASS + 1), "one past: stop");
    }

    // ── T0-1d (c): the short-serve rule and its generator ────────────────────

    /// §4l Q-F3, at the predicate: `served < proven` is the whole rule, and it
    /// fires on a strict prefix exactly as it fires on zero — while a serve of
    /// `proven` or more, a refused pool, a lied-about pool and an already-graded
    /// pool are all left alone. Mutant: restore `roots == 0` (the T0-1b shape)
    /// and the first two assertions go red; drop the `Served` key and the
    /// `Unsupported` row goes red (the A10 conflation the first cut made).
    #[test]
    fn a_short_serve_above_a_proven_boundary_is_withheld_and_a_full_one_is_not() {
        let served = |roots| PoolFetch::Served { roots };
        // SHORT: a strict prefix above a proven boundary is under-served.
        assert_eq!(
            withheld(served(1), 2),
            Some(PoolFetch::Withheld { proven: 2 })
        );
        assert_eq!(
            withheld(served(1127), 1128),
            Some(PoolFetch::Withheld { proven: 1128 })
        );
        // ZERO: the T0-1b reading, unchanged.
        assert_eq!(
            withheld(served(0), 1),
            Some(PoolFetch::Withheld { proven: 1 })
        );
        // FULL, and MORE than the bundle can prove: healthy at any count.
        assert_eq!(withheld(served(2), 2), None);
        assert_eq!(withheld(served(3), 2), None);
        // No proof at this tip: an honest zero and an honest one stay `Served`.
        assert_eq!(withheld(served(0), 0), None);
        assert_eq!(withheld(served(1), 0), None);
        // Keyed on the OUTCOME: nothing else is ever re-graded here.
        assert_eq!(withheld(PoolFetch::Unsupported, 5), None);
        assert_eq!(
            withheld(
                PoolFetch::HeightViolation {
                    code: "bundled_frontier"
                },
                5
            ),
            None
        );
        assert_eq!(withheld(PoolFetch::Withheld { proven: 5 }, 5), None);
    }

    /// §4l P8, measured rather than assumed, and the generator's receipt (F6's
    /// implementer half): for every `(network, pool)` the generator fills EVERY
    /// window — `m` heights for the bundle's largest `complete`, each inside its
    /// window, strictly increasing and `MIN_COMPLETION_GAP_BLOCKS` apart, at or
    /// above the activation — and the bind accepts the whole sequence on a fresh
    /// wallet against the SAME unfiltered rows `gather` reads, as does the wire
    /// check at the bundle's newest row. Every prefix is accepted too (the count
    /// bind's own harmless-prefix property, which is what makes the SHORT case a
    /// report and not a refusal). The hashes are pairwise distinct across all
    /// three pools, so the cross-wire assertion has something to see.
    #[test]
    fn the_generator_fills_every_window_on_both_networks_and_the_bind_accepts_it() {
        use std::collections::HashSet;
        use testing::{bind_consistent_heights, bind_consistent_roots};
        for network in [Network::Main, Network::Test] {
            let newest = root_bind::newest_bundled_height(network);
            // Distinct within ONE wallet's three trees — a wallet is one network,
            // so the same `(pool, index)` hash on the other network is not a clash.
            let mut hashes: HashSet<Vec<u8>> = HashSet::new();
            for &pool in SUBTREE_ROOT_POOLS {
                let rows = root_bind::bundled_counts(network, pool);
                let proven = rows.iter().map(|&(_, c)| c).max().unwrap_or(0);
                let heights = bind_consistent_heights(network, pool)
                    .unwrap_or_else(|gap| panic!("a window the bundle cannot hold: {gap:?}"));
                assert_eq!(
                    heights.len() as u64,
                    proven,
                    "{network:?}/{pool:?}: one height per proven subtree"
                );
                for w in heights.windows(2) {
                    assert!(
                        w[1] >= w[0] + root_bind::MIN_COMPLETION_GAP_BLOCKS,
                        "{network:?}/{pool:?}: {} then {} — closer than the gap floor",
                        w[0],
                        w[1]
                    );
                }
                let tightest = heights
                    .iter()
                    .zip(0u64..)
                    .map(|(&h, i)| {
                        let ceiling = rows.iter().filter(|&&(_, c)| c > i).map(|&(h, _)| h).min();
                        ceiling.map_or(u32::MAX, |c| c - h)
                    })
                    .min();
                eprintln!(
                    "generator {network:?}/{pool:?}: rows={} proven={proven} first={:?} last={:?} tightest_slack={tightest:?}",
                    rows.len(),
                    heights.first(),
                    heights.last()
                );

                let wire = bind_consistent_roots(network, pool).expect("same derivation");
                assert_eq!(wire.len(), heights.len());
                for r in &wire {
                    assert!(hashes.insert(r.root_hash.clone()), "a repeated root hash");
                }
                let evidence = root_bind::PoolEvidence {
                    bundled: rows.to_vec(),
                    ..Default::default()
                };
                let ceiling = BlockHeight::from_u32(newest);
                // Parsed per pool, and the bind + wire check run on every prefix
                // (the full sequence is the last prefix).
                match pool {
                    ShieldedProtocol::Sapling => {
                        let parsed: Vec<_> = wire
                            .iter()
                            .map(|r| parse_sapling_root(r).expect("canonical"))
                            .collect();
                        for n in 0..=parsed.len() {
                            root_bind::check_pool(&parsed[..n], &[], &evidence)
                                .expect("the bind accepts every prefix");
                        }
                        validate_root_sequence(
                            &root_heights(&parsed),
                            pool_activation(network, NetworkUpgrade::Sapling),
                            ceiling,
                        )
                        .expect("the wire check accepts the full sequence at the newest row");
                    }
                    ShieldedProtocol::Orchard => {
                        let parsed: Vec<_> = wire
                            .iter()
                            .map(|r| parse_orchard_root(r).expect("canonical"))
                            .collect();
                        for n in 0..=parsed.len() {
                            root_bind::check_pool(&parsed[..n], &[], &evidence)
                                .expect("the bind accepts every prefix");
                        }
                        validate_root_sequence(
                            &root_heights(&parsed),
                            pool_activation(network, NetworkUpgrade::Nu5),
                            ceiling,
                        )
                        .expect("the wire check accepts the full sequence at the newest row");
                    }
                    ShieldedProtocol::Ironwood => {
                        let parsed: Vec<_> = wire
                            .iter()
                            .map(|r| parse_ironwood_root(r).expect("canonical"))
                            .collect();
                        for n in 0..=parsed.len() {
                            root_bind::check_pool(&parsed[..n], &[], &evidence)
                                .expect("the bind accepts every prefix");
                        }
                        validate_root_sequence(
                            &root_heights(&parsed),
                            pool_activation(network, NetworkUpgrade::Nu6_3),
                            ceiling,
                        )
                        .expect("the wire check accepts the full sequence at the newest row");
                    }
                }
            }
        }
    }

    // ════════════════════════════════════════════════════════════════════════
    // S15-F1 phase B — the test author's unit rows, written BLIND to the
    // implementation (`docs/plan/s15-f1-subtree-roots-fetch-only-what-is-new.md`,
    // revision 2: §3.2 the plan, §3.3 step 1 the absolute flood cap and step
    // 4.2 the virtual-sequence validation; §7 rows T13, T14, T16). Appended at
    // the END of this module, in a child module of its own, so no cited line
    // above moves and the adapter below is the only place an implementer
    // signature is named. The wallet-level rows are
    // `wallet::tests::s15_roots`.
    // ════════════════════════════════════════════════════════════════════════
    mod s15_f1 {
        use super::*;
        use std::collections::{BTreeMap, HashMap};

        use crate::constants::{
            SUBTREE_ROOTS_FULL_VERIFY_PASSES, SUBTREE_ROOTS_FULL_VERIFY_SECS,
            SUBTREE_ROOTS_START_GRANULARITY,
        };
        use crate::root_bind::{BoundaryBounds, RewindWatch};

        // ════════════════════════════════════════════════════════════════════
        // ADAPTER: the one place the implementer's signatures are bound.
        // ════════════════════════════════════════════════════════════════════

        /// §3.2 `plan_pool_fetch(..) -> FetchPlan`, mapped into this module's
        /// own [`Planned`]/[`Why`] so the rows below name no implementer type.
        ///
        /// ASSUMES (all open in the contract — rebind here at the merge):
        /// * `plan_pool_fetch` lives in `sync` and is fallible
        ///   (`Result<FetchPlan, WalletError>` — §3.2: "a `None` there returns
        ///   `Sync { Internal }`");
        /// * its input is one struct (named `PlanInput` here) carrying the
        ///   pool's memo (`crate::wallet::PoolMemo`, the §3.1 fields) and the
        ///   plan section's reads: `count`, `recorded`, the brackets, `rewind`,
        ///   the scanned hashes of the recorded completing heights, `tip`,
        ///   `scanned_tip`, and `now` (= `inner.clock.now_unix()`);
        /// * `FetchPlan::From { start, overlap_height }` with `start` an unsigned
        ///   integer and `overlap_height` a `u32` or a consensus `BlockHeight`.
        ///
        /// The exhaustive `FullReason` match pins the six names §3.2 gives.
        fn plan(c: &PlanCase) -> Result<Planned, WalletError> {
            let memo = PoolMemo {
                verified: c.verified,
                chain_rewinds_seen: c.chain_rewinds_seen,
                passes_since_full: c.passes_since_full,
                last_full_at: c.last_full_at,
                incremental_off: c.incremental_off,
                c6_seen: c.c6_seen.clone(),
                ..PoolMemo::default()
            };
            let snapshot = PoolSnapshot {
                count: c.count,
                recorded: c.recorded.clone(),
                bounds: c.brackets.clone(),
                rewind: c.rewind,
                scanned_hashes: c.scanned_hashes.clone(),
            };
            let planned = plan_pool_fetch(&memo, &snapshot, c.tip, c.scanned_tip, c.now)?;
            Ok(match planned {
                FetchPlan::Full(reason) => Planned::Full(match reason {
                    FullReason::Unverified => Why::Unverified,
                    FullReason::Rewound => Why::Rewound,
                    FullReason::NothingStored => Why::NothingStored,
                    FullReason::IncrementalOff => Why::IncrementalOff,
                    FullReason::PassesDue => Why::PassesDue,
                    FullReason::TimeDue => Why::TimeDue,
                }),
                FetchPlan::From {
                    start,
                    overlap_height,
                } => Planned::From {
                    start: u64::from(start),
                    overlap: overlap_height,
                },
            })
        }

        /// §3.3 step 1: the drain with the ABSOLUTE flood cap
        /// (`start + roots.len() >= MAX_SUBTREE_ROOTS_PER_POOL` ⇒ `Fatal`, at
        /// today's pre-push point). ASSUMES `collect_roots_with_hashes` gains the
        /// start as its second argument; returns how many roots were kept.
        async fn collect_from(
            stream: &mut SubtreeRootStream,
            start: u32,
        ) -> Result<usize, RootFetchFault> {
            match drain_roots(stream, parse_orchard_root, start, None).await {
                Ok(Drained::Served { roots, .. }) => Ok(roots.len()),
                Ok(Drained::OverlapMoved) => Err(RootFetchFault::Fatal(internal_fault())),
                Err(fault) => Err(fault.into_fetch_fault()),
            }
        }

        /// §3.3 step 4.2: `validate_root_sequence` "refactored to take heights".
        /// ASSUMES `(&[u32], Option<BlockHeight>, BlockHeight)`; if it takes
        /// `&[BlockHeight]`, map here.
        fn validate_heights(
            heights: &[u32],
            floor: Option<u32>,
            ceiling: u32,
        ) -> Result<(), WalletError> {
            validate_root_sequence(
                heights,
                floor.map(BlockHeight::from_u32),
                BlockHeight::from_u32(ceiling),
            )
        }

        // ════════════════════════════════════════════════════════════════════
        // End of ADAPTER.
        // ════════════════════════════════════════════════════════════════════

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Why {
            Unverified,
            Rewound,
            NothingStored,
            IncrementalOff,
            PassesDue,
            TimeDue,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Planned {
            Full(Why),
            From { start: u64, overlap: u32 },
        }

        /// Everything §3.2 says the plan reads, in this module's shape.
        #[derive(Clone, Debug)]
        struct PlanCase {
            verified: bool,
            chain_rewinds_seen: i64,
            passes_since_full: u32,
            last_full_at: u64,
            incremental_off: bool,
            c6_seen: BTreeMap<u64, (u32, [u8; 32])>,
            count: u64,
            recorded: Vec<Option<u32>>,
            brackets: HashMap<usize, BoundaryBounds>,
            rewind: RewindWatch,
            scanned_hashes: HashMap<u32, Vec<u8>>,
            tip: u32,
            scanned_tip: Option<u32>,
            now: u64,
        }

        const H0: u32 = 1_000_000;
        const NOW: u64 = 1_757_000_000;
        const SCANNED: [u8; 32] = [0xA1; 32];
        const OTHER: [u8; 32] = [0xB2; 32];

        /// Height `i` of the default fixture: ten blocks apart from `H0`.
        fn h(i: usize) -> u32 {
            H0 + 10 * u32::try_from(i).expect("small")
        }

        /// A verified, healthy pool memo with `count` stored roots at `h(i)`,
        /// the scan at the tip, the tip far above the run (so term 2 is absent),
        /// no brackets, nothing scanned, no rewind, the full verification just
        /// done.
        fn case(count: usize) -> PlanCase {
            let tip = h(400) + 10_000;
            PlanCase {
                verified: true,
                chain_rewinds_seen: 0,
                passes_since_full: 0,
                last_full_at: NOW,
                incremental_off: false,
                c6_seen: BTreeMap::new(),
                count: count as u64,
                recorded: (0..count).map(|i| Some(h(i))).collect(),
                brackets: HashMap::new(),
                rewind: RewindWatch::default(),
                scanned_hashes: HashMap::new(),
                tip,
                scanned_tip: Some(tip),
                now: NOW,
            }
        }

        /// A bracket at `i` that the recorded height satisfies.
        fn bracket(c: &mut PlanCase, i: usize) {
            let at = c.recorded[i].expect("a recorded index");
            c.brackets.insert(
                i,
                BoundaryBounds {
                    floor: Some(at - 1),
                    ceiling: Some(at),
                },
            );
        }

        /// Index `i`'s completing block scanned (hash [`SCANNED`]).
        fn scanned(c: &mut PlanCase, i: usize) {
            let at = c.recorded[i].expect("a recorded index");
            c.scanned_hashes.insert(at, SCANNED.to_vec());
        }

        fn floor64(v: u64) -> u64 {
            let g = SUBTREE_ROOTS_START_GRANULARITY as u64;
            v / g * g
        }

        enum Expect {
            Full(Why),
            /// `start` and the raw terms present (term 1 always among them).
            From {
                start: u64,
                terms: Vec<u64>,
            },
            Internal,
        }

        /// Run one table row and check every §7 T16 property on it.
        fn check(name: &str, c: &PlanCase, expect: &Expect) {
            let got = plan(c);
            match expect {
                Expect::Full(why) => match got {
                    Ok(Planned::Full(w)) => {
                        assert_eq!(w, *why, "{name}: the FIRST reason that holds")
                    }
                    other => panic!("{name}: expected Full({why:?}), got {other:?}"),
                },
                Expect::Internal => assert!(
                    matches!(
                        got,
                        Err(WalletError::Sync {
                            stall: StallReason::Internal
                        })
                    ),
                    "{name}: expected Sync {{ Internal }}, got {got:?}"
                ),
                Expect::From { start, terms } => {
                    let min = *terms.iter().min().expect("term 1 always exists");
                    assert_eq!(
                        *start,
                        floor64(min),
                        "{name}: the row's own arithmetic — start == floor64(min(terms))"
                    );
                    for t in terms {
                        assert!(start <= t, "{name}: start {start} must not exceed term {t}");
                    }
                    let idx = usize::try_from(*start).expect("small");
                    let overlap = c.recorded[idx].expect("a recorded overlap");
                    match got {
                        Ok(Planned::From {
                            start: s,
                            overlap: o,
                        }) => {
                            assert_eq!(s, *start, "{name}: start");
                            assert_eq!(o, overlap, "{name}: overlap_height = recorded[start]");
                            assert_eq!(s % SUBTREE_ROOTS_START_GRANULARITY as u64, 0, "{name}");
                        }
                        other => panic!("{name}: expected From {{ {start} }}, got {other:?}"),
                    }
                }
            }
        }

        /// **Gate 7 — the three named constants hold their reasoned values**
        /// (§8: 180 passes, 3,600 s, 64). The boundary BEHAVIOUR at each is
        /// pinned by T16's rows below and by the wallet rows T11a/T11b; this pin
        /// is what makes a silent retune a named failure rather than a moved
        /// boundary every boundary row follows.
        #[test]
        fn the_s15_f1_constants_hold_their_reasoned_values() {
            assert_eq!(SUBTREE_ROOTS_FULL_VERIFY_PASSES, 180);
            assert_eq!(SUBTREE_ROOTS_FULL_VERIFY_SECS, 3_600);
            assert_eq!(SUBTREE_ROOTS_START_GRANULARITY, 64);
        }

        /// **T16 (the reasons)** — §3.2: `Full` with the FIRST reason that holds,
        /// in the order `Unverified, Rewound, NothingStored, IncrementalOff,
        /// PassesDue, TimeDue`; each reason alone; each ordering pair; and each
        /// numeric reason at its boundary (N−1/N passes, T−1/T seconds, a clock
        /// one second and all the way behind `last_full_at`).
        ///
        /// Mutants: any reason's arm removed; the order permuted; `>` for `>=`
        /// on either due rule; the backwards-clock rule dropped; an unsaturated
        /// `clock − last_full_at` (the `now = 0` row panics in a debug build).
        #[test]
        fn plan_pool_fetch_full_reasons_in_order() {
            let n: u32 = SUBTREE_ROOTS_FULL_VERIFY_PASSES;
            let t: u64 = SUBTREE_ROOTS_FULL_VERIFY_SECS;
            let term1 = || Expect::From {
                start: 256,
                terms: vec![299],
            };
            let mut rows: Vec<(&str, PlanCase, Expect)> = Vec::new();

            let mut c = case(300);
            c.verified = false;
            rows.push(("unverified", c, Expect::Full(Why::Unverified)));

            let mut c = case(300);
            c.verified = false;
            c.rewind = RewindWatch {
                observed: 1,
                recorded_at: 0,
            };
            rows.push(("unverified beats rewound", c, Expect::Full(Why::Unverified)));

            let mut c = case(300);
            c.rewind = RewindWatch {
                observed: 1,
                recorded_at: 0,
            };
            c.chain_rewinds_seen = 1;
            rows.push(("the pool's own rewind", c, Expect::Full(Why::Rewound)));

            let mut c = case(300);
            c.rewind = RewindWatch {
                observed: 2,
                recorded_at: 2,
            };
            c.chain_rewinds_seen = 1;
            rows.push(("the chain counter moved", c, Expect::Full(Why::Rewound)));

            let mut c = case(0);
            c.rewind = RewindWatch {
                observed: 2,
                recorded_at: 2,
            };
            c.chain_rewinds_seen = 1;
            rows.push((
                "rewound beats nothing stored",
                c,
                Expect::Full(Why::Rewound),
            ));

            rows.push(("nothing stored", case(0), Expect::Full(Why::NothingStored)));

            let mut c = case(0);
            c.incremental_off = true;
            rows.push((
                "nothing stored beats incremental off",
                c,
                Expect::Full(Why::NothingStored),
            ));

            let mut c = case(300);
            c.incremental_off = true;
            rows.push(("incremental off", c, Expect::Full(Why::IncrementalOff)));

            let mut c = case(300);
            c.incremental_off = true;
            c.passes_since_full = n;
            rows.push((
                "incremental off beats passes due",
                c,
                Expect::Full(Why::IncrementalOff),
            ));

            let mut c = case(300);
            c.passes_since_full = n;
            rows.push(("passes due at N", c, Expect::Full(Why::PassesDue)));

            let mut c = case(300);
            c.passes_since_full = n - 1;
            rows.push(("N − 1 passes is incremental", c, term1()));

            let mut c = case(300);
            c.passes_since_full = n;
            c.now = NOW + t;
            rows.push(("passes due beats time due", c, Expect::Full(Why::PassesDue)));

            let mut c = case(300);
            c.now = NOW + t;
            rows.push(("time due at T", c, Expect::Full(Why::TimeDue)));

            let mut c = case(300);
            c.now = NOW + t - 1;
            rows.push(("T − 1 is incremental", c, term1()));

            let mut c = case(300);
            c.now = NOW - 1;
            rows.push(("a clock one second behind", c, Expect::Full(Why::TimeDue)));

            let mut c = case(300);
            c.now = 0;
            rows.push(("a clock at the epoch", c, Expect::Full(Why::TimeDue)));

            rows.push(("a full verification just now", case(300), term1()));

            for (name, c, expect) in &rows {
                check(name, c, expect);
            }
        }

        /// **T16 (the terms)** — §3.2 terms 1–4, the rounding, and
        /// `overlap_height = recorded[start]`. Every off-by-one row sits on a
        /// 64-block edge (§7's preamble), and the strict-minimum rows put each
        /// term in a DIFFERENT block and peel them off one at a time.
        ///
        /// Mutants (each red on a named row): term 1 `count` / `count − 2`;
        /// term 2 dropped, `>=` for `>`, anchored on `tip` alone, `max` for
        /// `min`, `scanned_tip.unwrap_or(0)`, an unsaturated `− REORG_MAX_BLOCKS`;
        /// term 3 dropped, its `− 1` dropped or doubled, the HIGHEST bracket
        /// used; term 4 dropped, `c6_seen` compared by index alone or by hash
        /// alone, an index shifted; a ceil for the floor; the rounding dropped;
        /// a term read above `count`; a `None` below `count` not refused.
        #[test]
        fn plan_pool_fetch_terms_and_rounding() {
            let mut rows: Vec<(&str, PlanCase, Expect)> = Vec::new();
            let from = |start: u64, terms: &[u64]| Expect::From {
                start,
                terms: terms.to_vec(),
            };

            // ── term 1 ──
            rows.push((
                "count 193: count − 1 = 192 ≡ 0",
                case(193),
                from(192, &[192]),
            ));
            rows.push((
                "count 192: count − 1 = 191 ≡ 63",
                case(192),
                from(128, &[191]),
            ));
            rows.push(("count 65: 64 ≡ 0", case(65), from(64, &[64])));
            rows.push(("count 64: 63 ≡ 63", case(64), from(0, &[63])));
            rows.push(("Ironwood-sized: count 10", case(10), from(0, &[9])));
            rows.push(("one root", case(1), from(0, &[0])));

            // ── term 2: the reorg window, via the scanned tip ──
            let mut c = case(300);
            c.scanned_tip = Some(h(127) + REORG_MAX_BLOCKS);
            rows.push((
                "window: recorded[127] == window_floor ⇒ first index 128",
                c,
                from(128, &[299, 128]),
            ));
            let mut c = case(300);
            c.scanned_tip = Some(h(190) + REORG_MAX_BLOCKS);
            rows.push(("window: first index 191", c, from(128, &[299, 191])));
            let mut c = case(300);
            c.scanned_tip = Some(h(200) + REORG_MAX_BLOCKS);
            rows.push(("window: first index 201", c, from(192, &[299, 201])));

            // ── term 2 anchored on the TIP: a run clustered near it ──
            // Heights 1,000 apart below 160, then 2 apart, so 33 indices sit inside
            // one REORG_MAX_BLOCKS window; the tip sits exactly 100 above h(159).
            let clustered = || {
                let mut c = case(193);
                let mut hs: Vec<Option<u32>> = Vec::new();
                for i in 0..193u32 {
                    hs.push(Some(if i < 160 {
                        H0 + 1_000 * i
                    } else {
                        H0 + 1_000 * 159 + 2 * (i - 159)
                    }));
                }
                c.recorded = hs;
                c.tip = c.recorded[159].expect("set") + REORG_MAX_BLOCKS;
                c
            };
            let mut c = clustered();
            c.scanned_tip = None;
            rows.push((
                "window from the tip when nothing is scanned",
                c,
                from(128, &[192, 160]),
            ));
            let mut c = clustered();
            c.scanned_tip = Some(c.tip + 50);
            rows.push((
                "window: a scan ABOVE the endpoint's tip still anchors on the lower one",
                c,
                from(128, &[192, 160]),
            ));
            let mut c = clustered();
            c.scanned_tip = Some(c.tip);
            rows.push(("window: scan at the tip", c, from(128, &[192, 160])));

            // ── term 2 saturation: a tip below REORG_MAX_BLOCKS ──
            let mut c = case(70);
            c.recorded = (0..70u32).map(|i| Some(1 + i)).collect();
            c.tip = 90;
            c.scanned_tip = None;
            rows.push(("window floor saturates at 0", c, from(0, &[69, 0])));

            // ── term 3: the lowest bracket − 1 ──
            let mut c = case(300);
            bracket(&mut c, 128);
            rows.push(("bracket at 128 ⇒ 127", c, from(64, &[299, 127])));
            let mut c = case(300);
            bracket(&mut c, 129);
            rows.push(("bracket at 129 ⇒ 128", c, from(128, &[299, 128])));
            let mut c = case(300);
            bracket(&mut c, 64);
            bracket(&mut c, 130);
            rows.push(("the LOWEST bracket", c, from(0, &[299, 63])));
            let mut c = case(193);
            c.brackets.insert(
                250,
                BoundaryBounds {
                    floor: Some(h(250) - 1),
                    ceiling: Some(h(250)),
                },
            );
            rows.push((
                "a bracket above the run is no term below it",
                c,
                from(192, &[192, 249]),
            ));

            // ── term 4: a scanned completing block not (or no longer) seen ──
            let mut c = case(300);
            scanned(&mut c, 70);
            rows.push(("scanned, never seen", c, from(64, &[299, 70])));
            let mut c = case(300);
            scanned(&mut c, 70);
            c.c6_seen.insert(70, (h(70), SCANNED));
            rows.push(("scanned and seen with this hash", c, from(256, &[299])));
            let mut c = case(300);
            scanned(&mut c, 70);
            c.c6_seen.insert(70, (h(70), OTHER));
            rows.push((
                "seen, but the scanned hash changed",
                c,
                from(64, &[299, 70]),
            ));
            let mut c = case(300);
            scanned(&mut c, 70);
            c.c6_seen.insert(70, (h(70) + 1, SCANNED));
            rows.push(("seen at another height", c, from(64, &[299, 70])));
            let mut c = case(300);
            scanned(&mut c, 128);
            rows.push(("term 4 at 128 ≡ 0", c, from(128, &[299, 128])));
            let mut c = case(300);
            scanned(&mut c, 127);
            rows.push(("term 4 at 127 ≡ 63", c, from(64, &[299, 127])));
            let mut c = case(300);
            c.count = 193;
            // `recorded` still runs to 300 (rows above a NULL root at 193): a
            // scanned completing block at 250 and a window starting at 250 are
            // both above the run and are no term.
            c.scanned_hashes.insert(h(250), SCANNED.to_vec());
            c.scanned_tip = Some(h(249) + REORG_MAX_BLOCKS);
            rows.push((
                "terms 2 and 4 read only i < count (recorded runs past the NULL root)",
                c,
                from(192, &[192]),
            ));

            // ── the strict minimum, four terms in four blocks, peeled ──
            let all_four = || {
                let mut c = case(193);
                c.scanned_tip = Some(h(149) + REORG_MAX_BLOCKS); // term 2 = 150 ⇒ 128
                bracket(&mut c, 101); // term 3 = 100 ⇒ 64
                scanned(&mut c, 10); // term 4 = 10 ⇒ 0
                c
            };
            rows.push((
                "four terms: term 4 is the minimum",
                all_four(),
                from(0, &[192, 150, 100, 10]),
            ));
            let mut c = all_four();
            c.scanned_hashes.clear();
            rows.push(("three terms: term 3", c, from(64, &[192, 150, 100])));
            let mut c = all_four();
            c.scanned_hashes.clear();
            c.brackets.clear();
            rows.push(("two terms: term 2", c, from(128, &[192, 150])));
            let mut c = all_four();
            c.scanned_hashes.clear();
            c.brackets.clear();
            c.scanned_tip = Some(c.tip);
            rows.push(("term 1 alone", c, from(192, &[192])));

            // ── the snapshot's own invariant: every index below count is Some ──
            let mut c = case(193);
            c.recorded[50] = None;
            rows.push(("a None below count", c, Expect::Internal));
            let mut c = case(193);
            c.recorded.truncate(100);
            rows.push(("recorded shorter than count", c, Expect::Internal));

            for (name, c, expect) in &rows {
                check(name, c, expect);
            }
        }

        /// **T16 (the bracket at index 0)** — §3.2 term 3: "`lowest bracket − 1`
        /// (Full if that is below 0)". A bracket at 0 must re-serve from 0. The
        /// contract names no `FullReason` for this case (none of the six fits)
        /// and also says a `From { start: 0 }` IS a full fetch for the memo, so
        /// either shape is accepted; what is pinned is that nothing above 0 is
        /// asked for, and that `0 − 1` does not wrap.
        ///
        /// Mutants: the term skipped at 0 (asks 256); `0 − 1` wrapping to
        /// `u64::MAX` (term 3 absent ⇒ 256) or panicking.
        #[test]
        fn plan_pool_fetch_a_bracket_at_index_zero_re_serves_from_zero() {
            let mut c = case(300);
            bracket(&mut c, 0);
            match plan(&c) {
                Ok(Planned::Full(_)) | Ok(Planned::From { start: 0, .. }) => {}
                other => panic!("a bracket at 0 must re-serve from 0, got {other:?}"),
            }
        }

        /// **T14** — §3.3 step 1 / ledger row 1: the flood cap is ABSOLUTE —
        /// `start + roots.len() >= MAX_SUBTREE_ROOTS_PER_POOL` ⇒
        /// `Fatal(endpoint_unusable())`, at today's pre-push point. With the raw
        /// start 65,472 (`1023 × 64`), the last legal index 65,535 is reached by
        /// 64 roots; a 65th is a flood. An endless stream stops there too.
        ///
        /// Mutants: a relative cap (`roots.len() >= MAX` — 65 roots accepted,
        /// the endless stream runs 65,536 deep); `>` for `>=` (65 accepted);
        /// a cap one too tight (64 refused).
        #[tokio::test]
        async fn an_incremental_flood_is_capped_at_the_absolute_index_space() {
            const START: u32 = 65_472;
            assert_eq!(
                START + 64,
                MAX_SUBTREE_ROOTS_PER_POOL,
                "the fixture: 64 roots from START reach exactly the last legal index"
            );
            assert_eq!(u64::from(START) % SUBTREE_ROOTS_START_GRANULARITY as u64, 0);
            let roots = |n: u64| -> Vec<Result<Option<SubtreeRoot>, tonic::Status>> {
                (0..n)
                    .map(|i| Ok(Some(canonical_root(1_000_000 + 2 * i, 1))))
                    .collect()
            };
            let flood = |r: &Result<usize, RootFetchFault>| {
                matches!(
                    r,
                    Err(RootFetchFault::Fatal(WalletError::Sync {
                        stall: StallReason::EndpointMisbehaving
                    }))
                )
            };

            let mut s = crate::net::grpc::testing::scripted_stream(roots(64));
            let r = collect_from(&mut s, START).await;
            assert!(
                matches!(r, Ok(64)),
                "64 roots end at index 65,535: legal, got {r:?}"
            );

            let mut s = crate::net::grpc::testing::scripted_stream(roots(65));
            let r = collect_from(&mut s, START).await;
            assert!(flood(&r), "a 65th root is index 65,536: a flood, got {r:?}");

            let mut s = crate::net::grpc::testing::endless_stream(canonical_root(1_000_000, 1));
            let r = collect_from(&mut s, START).await;
            assert!(
                flood(&r),
                "an endless stream from START is stopped at the cap, got {r:?}"
            );

            // The control: from 0, the same 65 roots are nowhere near the cap.
            let mut s = crate::net::grpc::testing::scripted_stream(roots(65));
            let r = collect_from(&mut s, 0).await;
            assert!(matches!(r, Ok(65)), "from 0, 65 roots are legal, got {r:?}");
        }

        /// **T13** — §3.3 step 4.2 / ledger row 2: the wire check runs over the
        /// VIRTUAL sequence `recorded[..start] ++ served` — the activation floor
        /// on index 0, strict monotonicity ACROSS the seam, every prefix height
        /// held to this pass's tip. Each refusing row is one the served part
        /// alone passes, so a check run on the suffix accepts it.
        ///
        /// What this row can and cannot kill: it drives the refactored check on
        /// sequences shaped as `prefix ++ served`, so it kills a check that
        /// skips its first elements or stops at a seam. The CALL-SITE mutant —
        /// the fetch passing `served` instead of the virtual sequence — is not
        /// observable through the wallet at all: the overlap root
        /// (`recorded[start]`, re-served and itself held to the tip and to
        /// monotonicity) dominates every prefix height. Reported as a finding.
        ///
        /// Mutants: validation of the served part only (inside the function);
        /// the tip ceiling `>=` for `>`.
        #[test]
        fn validate_root_sequence_holds_the_stored_prefix_to_the_tip() {
            let virtual_seq = |prefix: &[u32], served: &[u32]| -> Vec<u32> {
                prefix.iter().chain(served).copied().collect()
            };
            // (name, prefix, served, floor, tip, the virtual verdict, and whether
            // the SERVED part alone passes — `true` on a refusing row is what
            // makes it a row only the virtual sequence catches)
            type Row<'a> = (&'a str, &'a [u32], &'a [u32], Option<u32>, u32, bool, bool);
            let rows: [Row<'_>; 7] = [
                (
                    "honest",
                    &[100, 200, 300],
                    &[400, 500],
                    Some(100),
                    500,
                    true,
                    true,
                ),
                (
                    "the last root exactly at the tip",
                    &[100, 200],
                    &[300],
                    Some(100),
                    300,
                    true,
                    true,
                ),
                (
                    "one above the tip",
                    &[100, 200],
                    &[301],
                    Some(100),
                    300,
                    false,
                    false,
                ),
                (
                    "non-increasing across the seam",
                    &[100, 200, 300],
                    &[250, 400],
                    Some(100),
                    500,
                    false,
                    true,
                ),
                (
                    "equal across the seam",
                    &[100, 200, 300],
                    &[300, 400],
                    Some(100),
                    500,
                    false,
                    true,
                ),
                (
                    "a prefix height above the tip",
                    &[100, 2_000],
                    &[],
                    Some(100),
                    1_000,
                    false,
                    true,
                ),
                (
                    "index 0 below the activation",
                    &[50, 200],
                    &[300],
                    Some(100),
                    500,
                    false,
                    true,
                ),
            ];
            for (name, prefix, served, floor, tip, ok, suffix_alone_passes) in rows {
                let v = virtual_seq(prefix, served);
                assert_eq!(
                    validate_heights(&v, floor, tip).is_ok(),
                    ok,
                    "{name}: the virtual sequence {v:?}"
                );
                assert_eq!(
                    validate_heights(served, floor, tip).is_ok(),
                    suffix_alone_passes,
                    "{name}: the fixture's own claim about the served part alone"
                );
            }
        }
    }
}
