//! **T0-1a — the independent PROOF half.** The bind that stops an endpoint
//! choosing `{prefix}_tree_shards.subtree_end_height`.
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4c, item T0-1a
//! (`CONTRACTED`, revised by the design review). Written blind, under
//! §2.1 IT-1/IT-2a/IT-3/IT-6 of `docs/plan/production-readiness.md`: the author
//! of this file has not read the implementation and may not edit it.
//!
//! ## What is driven, and what is observed
//!
//! **Driven** through [`Wallet::update_subtree_roots`] (and, where a scanned
//! block or a reorg is needed, through [`Wallet::sync_once`], which calls it) —
//! never against a storage location, per the contract's stated mechanism choice.
//! What that buys and what it costs is written in the contract; the cost is that
//! a bind correct in the store and never reached from the sync path is invisible
//! here, and the post-build review is briefed at that seam.
//!
//! **Observed** on `{prefix}_tree_shards.subtree_end_height` and
//! `{prefix}_tree_cap` through a SECOND keyed connection (the production `aux_db`
//! pattern). That is upstream's own table — the money-relevant state the attack
//! moves and the state A11 exists to protect — **not** the bind's memory. No
//! assertion in this file names an aux table, a column or a file the implementer
//! might have chosen for question 1, so every case here is answerable by any of
//! the contract's mechanisms (a)/(b)/(c)/(d) or a composition of them.
//!
//! ## The double, and why it is a second one
//!
//! [`ScriptedEndpoint`] is this file's own [`SubtreeRootSource`]. It exists
//! because the contract records that `sync::testing::FakeRootSource` **cannot**
//! answer differently on consecutive passes — `subtree_roots` does
//! `std::mem::take` (`sync.rs:1763-1774`), so a pool's second call yields an
//! empty stream — and because `put_shard_roots` early-returns `Ok(())` on an
//! empty slice, which makes "nothing was written for that pool" satisfiable
//! against a null implementation. This double **clones** its script on every
//! call, so a pool can be served twice, and it **records what it served**
//! ([`ScriptedEndpoint::served`]) so every refuse-case can state, as an
//! assertion, that the endpoint really did stream the roots the case is named
//! after. That is the IT-10 receipt: the mechanism that refused the write cannot
//! be "the double ran out of script".
//!
//! Editing `FakeRootSource` is out of bounds for this half (the implementer may
//! be touching that file); the cost is that there are now two doubles to keep
//! honest, and the fold consolidates them.
//!
//! ## The numbers
//!
//! Mainnet, and the heights are the measured attack's own
//! (`docs/plan/probes/ironwood-a11-bypass.{py,output.txt}`), not fresh
//! inventions: truthful `{0: 3451206, 1: 3459187, 2: 3467168, 3: 3475149}`,
//! compressed `{0: 3428143, 1: 3428144, 2: 3428145, 3: 3428146}`, endpoint tip
//! `3475499`. All three of A11's clauses pass for the compressed sequence today;
//! that is the whole point of the item.
//!
//! ## T0-1a-R — attribution, and why every new row is IRONWOOD
//!
//! The repair contract's **R8** requires every refusal assertion it adds to name
//! the ARM that refused, never a bare `is_err`/`is_ok`. The instrument is
//! [`sync::PoolFetch::HeightViolation`]'s `code`, read here through
//! [`refusal_code`].
//!
//! **That instrument exists only for Ironwood, and using it is not a preference.**
//! `sync::apply_height_bind` carries an Ironwood violation to the caller as an
//! outcome — `Ok([.., HeightViolation { code }, ..])` — and answers a Sapling or
//! Orchard violation with `return Err(endpoint_unusable())`, which DROPS the code:
//! at the caller it is `Sync { EndpointUnreachable }`, indistinguishable from a
//! dead link. So **R5, R6, R7 and R8 are satisfiable only on Ironwood**, and the
//! two rows that refuse through Sapling/Orchard
//! (`the_same_compression_is_refused_for_sapling_and_orchard`,
//! `a_fault_on_a_later_pool_leaves_the_bind_unchanged`) keep their untyped
//! assertions, because nothing better is reachable from a caller. That is §4d owed
//! row 3, which §4e puts explicitly OUT of scope, and it is recorded here rather
//! than worked around.
//!
//! ## What the new rows measure, and the order the arms run in
//!
//! Measured on `t0-1a-join`, the bind consults its arms in this order and returns
//! on the FIRST that refuses: **block-size gap → recorded heights → bundled
//! frontiers → scanned tree size → completing block hash**. Every fixture added by
//! T0-1a-R is shaped so exactly ONE of them can fire — the others are structurally
//! unable to, and each row says which and why. That is what makes "the refusal
//! names the arm" a measurement rather than a coincidence (IT-10), and it is the
//! defect §4d owed row 5 records: a row refused by the cheap floor cannot see the
//! mechanism it is named after die.
//!
//! ## T0-1a-R2 — the ledger records the EVENT, and consumes PER POOL
//!
//! Four rows at the end of the file, written blind from
//! `production-readiness-phase-1.md` §4g against `a02a33e6`. Two things they
//! established that the contract had not: **§4g P1 is reachable** — a fork served
//! within `REWIND_DISTANCE_BLOCKS` of the wallet's scan floor makes
//! `db.truncate_to_height` return upstream's `RequestedRewindInvalid`, which this
//! crate classifies as `StoreCorrupt` ([`induce_a_rewind_the_wallet_cannot_perform`]
//! carries the geometry and the finding) — and **the 0b rows cannot be built in the
//! [`UNDER_SCAN_BASE`] window**, because the scanned arm refuses every Sapling
//! fixture there ([`SAPLING_ERA_SCAN_BASE`] says what was measured). The planted row
//! is the 0a cell the floor does not name: a failed truncate silencing the SCANNED
//! oracle on a wallet that has no recorded height for R12's shape to touch.
//!
//! ## T0-1d — the conflict's exit, INC-021, and the two consume arms
//!
//! Five rows at the end of the file, written blind from
//! `production-readiness-phase-1.md` §4l against `1498a91a` (the T0-1c-R fold
//! `2f743fb0` plus the contract). They drive the shipped `update_subtree_roots`
//! path with this file's [`ScriptedEndpoint`], read upstream's own
//! `{pool}_tree_shards.root_hash` column through the observer connection, and
//! reuse the drives unchanged. `pool_script`'s tag scheme is what
//! makes them cheap: a different tag base at the same index IS the root-hash
//! mutation (F1, [`IRONWOOD_MUTATED_TAG`]), and the same tag twice in one serve
//! IS INC-021's duplicate (F2). The planted row is the negative half of F1's
//! exit — the poison survives a server switch — because "switch servers" is the
//! remedy the surface already names for this cell, and it does not work.

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::OptionalExtension;
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_backend::proto::service::{ShieldedProtocol, SubtreeRoot, TreeState};
use zcash_protocol::consensus::BlockHeight;

use crate::config::{JitterPolicy, LightServerEndpoint, TorPolicy, WalletConfig};
use crate::constants::REORG_MAX_BLOCKS;
use crate::error::WalletError;
use crate::keychain::testvault::TestVault;
use crate::keychain::{KeychainPort, VaultTier};
use crate::money::Network;
use crate::net::grpc::testing::scripted_stream;
use crate::net::grpc::{BlockStream, GrpcError, SubtreeRootStream};
use crate::seal::WalletDbKey;
use crate::seed::{SeedPersistence, SeedSource};
use crate::sync::testing::{DeclaredTreeSizes, display_order_hex};
use crate::sync::{self, ScanClient, SubtreeRootSource};
use crate::wallet::Wallet;

// ── The measured attack's numbers ───────────────────────────────────────────

/// The endpoint-reported tip in the probe.
const TIP: u32 = 3_475_499;
/// NU6.3 mainnet activation — the public constant the attacker already knows.
/// Read from the compiled params in [`nu63_mainnet`]; this literal exists only
/// so the compressed fixture below reads like the probe's own output line.
const ACTIVATION: u64 = 3_428_143;
/// The Ironwood shard-completion heights this proof serves as HONEST.
///
/// **ADJUDICATOR REPAIR (T0-1a) — index 1 moved, 3,459,187 → 3,463,000.**
/// The contract called the probe's four numbers "the truthful heights"; the
/// probe's own `SHARD_END_HEIGHTS` labels indices 1 and 2 `"interpolated"`
/// (`docs/plan/probes/ironwood-unspendability-repro.py:383-388`), and the shipped
/// bundle REFUTES index 1: the mainnet Ironwood frontier at 3,459,780 decodes to
/// one complete subtree, so subtree 1 completed ABOVE that height and 3,459,187 is
/// a number no honest server can serve at index 1. Indices 0 and 3 are the two
/// MEASURED values from the live `GetSubtreeRoots` capture and are unchanged;
/// index 2 (3,467,168) is also interpolated but already sits above the newest
/// bundled row, so no oracle contradicts it. Index 1's replacement is SYNTHETIC
/// and labelled as such: it is above 3,459,780, below index 2, and at least
/// `MIN_COMPLETION_GAP_BLOCKS` from both neighbours.
const TRUTHFUL: [u64; 4] = [3_451_206, 3_463_000, 3_467_168, 3_475_149];
/// The endpoint-compressed heights (probe row C) — `activation + shard_index`.
const COMPRESSED: [u64; 4] = [ACTIVATION, ACTIVATION + 1, ACTIVATION + 2, ACTIVATION + 3];
/// The INFLATED sequence: `{tip-3, tip-2, tip-1, tip}`. Passes all three of
/// today's A11 clauses AND a monotonic bind; it is the trigger for the
/// cap-erasure chain the design review found (contract §4c, review row 3).
const INFLATED: [u64; 4] = [TIP as u64 - 3, TIP as u64 - 2, TIP as u64 - 1, TIP as u64];

/// The RE-CROSSED sequence: the truthful one with index 3 moved DOWN, to a
/// height still above index 2 and still above the newest bundled mainnet
/// treestate row (3,459,780). Two named tests serve exactly this, and the
/// contract requires OPPOSITE answers for them —
/// `an_honest_endpoint_is_accepted_after_a_rewind_that_recrosses_a_shard_boundary`
/// (C15, after a rewind: ACCEPT) and
/// `a_poisoned_bind_is_cleared_by_the_operation_the_contract_names` (C17, with
/// the record standing: REFUSE, then accept after the clearing operation). It is
/// written down once so the two cannot drift apart.
///
/// It is narrow on purpose: nothing about it violates the ordering clause, the
/// activation clause, the ceiling clause, or any bound a bundled Ironwood
/// frontier can impose. The only thing that contradicts it is the wallet's own
/// record of index 3.
const RECROSSED: [u64; 4] = [TRUTHFUL[0], TRUTHFUL[1], TRUTHFUL[2], 3_470_000];

// ── T0-1a-R fixtures ─────────────────────────────────────────────────

/// **The inflation attack, SPACED so the block-size floor cannot reach it**
/// (§4d owed row 5, §4e R7).
///
/// [`INFLATED`] is `{tip-3, tip-2, tip-1, tip}` — four CONSECUTIVE heights — and
/// **measured on `t0-1a-join` it is refused by `completion_gap`**, the arithmetic
/// floor that needs no oracle and no memory. That refusal is correct, and it is
/// also exactly why the adjudicator recorded that C12 *"stays green with the whole
/// counting bind removed"*: the row named for inflation could not see the
/// mechanism that catches inflation.
///
/// This sequence keeps the attack — every subtree claimed complete inside the last
/// 300 blocks, on a wallet with nothing recorded and nothing scanned — and spaces
/// the completions 100 blocks apart, two orders of magnitude clear of the floor.
/// Measured: refused by `bundled_frontier`, the endpoint-independent oracle, which
/// is the mechanism the row names.
const INFLATED_SPACED: [u64; 4] = [
    TIP as u64 - 300,
    TIP as u64 - 200,
    TIP as u64 - 100,
    TIP as u64,
];

/// **R5's fixture: refused by the BUNDLED arm and by nothing else.**
///
/// One Ironwood root at index 0, claiming subtree 0 completed at 3,465,000. The
/// signed binary refutes it: the mainnet Ironwood frontier at **3,452,280** already
/// decodes to one complete subtree, so subtree 0 completed at or below 3,452,280
/// and cannot also have completed 12,720 blocks later. (Measured this session from
/// `root_bind::bundled_counts`: 821 mainnet rows, the last showing ZERO complete
/// Ironwood subtrees is 3,449,780 and the first showing one is 3,452,280, so the
/// bundle pins subtree 0's completion into that 2,500-block window.)
///
/// **Why no other arm can be the refuser** — a single root has no consecutive pair,
/// so the gap floor has nothing to compare; the wallet is fresh, so there is no
/// recorded height and no scanned block, which makes the recorded arm, the scanned
/// arm and the completing-hash arm all abstain by their own explicit branches. The
/// height is above the NU6.3 activation and below the endpoint's tip, so no A11
/// clause reaches it either.
const BUNDLED_ONLY_VIOLATION: [u64; 1] = [3_465_000];

/// R5's discriminator: the SAME one-root shape at the measured completion height,
/// which sits inside the window the bundle allows. The only difference between this
/// and [`BUNDLED_ONLY_VIOLATION`] is the completing height.
const BUNDLED_ONLY_CONTROL: [u64; 1] = [TRUTHFUL[0]];

/// R6's scan window — **ABOVE** the served completion height, which is what makes
/// the wallet's own scanned count contradict the endpoint. The fixture chain
/// declares `ironwood_commitment_tree_size = 0` on every block, so after scanning
/// here the wallet has counted ZERO complete Ironwood subtrees as of 3,452,100 —
/// and an endpoint claiming subtree 0 completed at 3,451,206 is claiming one was
/// complete before a block the wallet itself counted.
const OVER_SCAN_BASE: u64 = 3_452_000;
const OVER_SCAN_TIP: u64 = 3_452_100;

/// R4's and the planted row's scan window — **BELOW** every served completion
/// height, deliberately.
///
/// This is the difference between a row that can be built and a row that cannot.
/// The same zero-commitment fixture chain that makes R6 work makes the scanned arm
/// refuse EVERY root whose completing height is at or below the newest scanned
/// block (it is the structural block the adjudicator recorded for the C6 binding
/// half). Scanning strictly below the served heights turns that arm into an
/// abstention — `served > at_height` holds for every index — so a reorg can be
/// induced without the scan it requires also poisoning the case.
const UNDER_SCAN_BASE: u64 = 3_430_000;
const UNDER_SCAN_TIP: u64 = 3_430_100;

/// One step of R3's walk: strictly LESS than the window the pre-repair code grants
/// unconditionally, **derived from that constant rather than spelled**, so raising
/// or lowering [`REORG_MAX_BLOCKS`] re-aims the fixture instead of silently turning
/// the row vacuous.
const WALK_STEP: u64 = REORG_MAX_BLOCKS as u64 - 1;
/// How many passes R3 drives. `8 * 99 = 792` blocks of drift, ~8× the window the
/// pre-repair code grants for a single pass, with no rewind anywhere.
const WALK_PASSES: u64 = 8;

/// **The DEEP-LOWERED sequence, served by two rows for opposite reasons** — the
/// same device [`RECROSSED`] uses, one sequence written down once so two cases
/// cannot drift apart.
///
/// Index 3 sits at 3,467,400: 232 blocks above index 2 (so the ordering clause and
/// the block-size floor both pass), 7,620 blocks above the newest bundled Ironwood
/// row at 3,459,780 (so no bundled frontier reaches it), and below the endpoint's
/// tip (so the ceiling clause passes). **Nothing in the build can refuse it except
/// a recorded height.**
///
/// * `a_rewind_rebinds_rather_than_disabling_the_recorded_bind` serves it against a
///   record of [`RECROSSED`] — a 2,600-block move, with **no new rewind** — and the
///   contract (R4) requires it REFUSED.
/// * `a_rewind_does_not_license_a_move_no_reorg_could_produce` serves it against a
///   record of [`TRUTHFUL`] — a 7,749-block move — immediately after a rewind of
///   `REWIND_DISTANCE_BLOCKS` = 10 blocks. That one is the planted case, and what
///   it answers is which of the two designs §4e left open was built.
const DEEP_LOWERED: [u64; 4] = [TRUTHFUL[0], TRUTHFUL[1], TRUTHFUL[2], 3_467_400];

/// **ADJUDICATOR REPAIR (T0-1a).** These four constants were INVENTED —
/// `[600_000, 700_000, 800_000, 900_000]` for Sapling and
/// `[1_800_000, …]` for Orchard — and the chain refutes all eight numbers.
/// Mainnet Sapling's subtree 0 completes at **558,822** and mainnet Orchard's at
/// **1,707,429** (`docs/plan/probes/ironwood-subtree-roots-probe.output.txt`, the
/// `max_entries=0` capture), so a bundled treestate row between the real
/// completion and the invented one already shows the subtree complete and any
/// correct counting bind refuses the fixture. Indices 1..3 of a mainnet
/// Sapling/Orchard sequence are NOT recoverable from anything committed to this
/// repository — the capture prints only `first:` and `last:` — so the honest
/// repair is a ONE-root fixture at the measured first-completion height. The
/// attack shape is preserved exactly: one root at the pool ACTIVATION, which is
/// the same "activation + k" compression at k = 0.
const SAPLING_TRUTHFUL: [u64; 1] = [558_822];
const SAPLING_COMPRESSED: [u64; 1] = [419_200];
const ORCHARD_TRUTHFUL: [u64; 1] = [1_707_429];
const ORCHARD_COMPRESSED: [u64; 1] = [1_687_104];

// ── T0-1a-R2 fixtures ────────────────────────────────────────────────

/// **The 0b rows' scan window — BELOW Sapling's measured first completion.**
///
/// R14 and R15 need a pool that is WRITTEN before the pool whose put fails, and
/// upstream's put order is Sapling → Orchard → Ironwood, so the written pool has to
/// be Sapling or Orchard. Both of their honest fixtures sit far below
/// [`UNDER_SCAN_BASE`], and on this proof's zero-commitment chain the SCANNED arm
/// refuses every root whose completing height is at or below the scan tip.
/// **Measured, not argued:** after scanning [`UNDER_SCAN_BASE`]`..=`
/// [`UNDER_SCAN_TIP`], an equal re-serve of [`SAPLING_TRUTHFUL`] — the wallet's own
/// record — is `Err(Sync { EndpointUnreachable })` with no rewind anywhere, and once
/// a rewind has been consumed it is refused again. A 0b row built there would have
/// two arms able to refuse its negative case and no admissible Sapling height at all
/// for its anti-vacuity control. So the rows anchor the wallet a hundred blocks
/// below 558,822 instead: every Sapling and Ironwood height served sits above the
/// scan tip, the scanned arm passes by its own arithmetic on every pass, and the
/// recorded arm is the only oracle that can answer a Sapling move (IT-10). Measured
/// in the same session: the honest pass is accepted here, a real rewind fires
/// (100 → 91 scanned blocks), and both pools' shard rows survive it.
const SAPLING_ERA_SCAN_BASE: u64 = 558_000;
const SAPLING_ERA_SCAN_TIP: u64 = 558_100;

/// Sapling's index-0 height moved DOWN, twice — the move a rewind licenses, and the
/// further move that nothing should.
///
/// Both sit inside the window the signed bundle admits for Sapling subtree 0
/// (measured from `root_bind::bundled_counts`: the last mainnet row showing
/// zero complete Sapling subtrees is 550,000 and the first showing one is 560,000),
/// above [`SAPLING_ERA_SCAN_TIP`], and a single root has no consecutive pair for the
/// gap floor to compare. Nothing but a recorded height can refuse either of them.
const SAPLING_LOWERED: [u64; 1] = [SAPLING_TRUTHFUL[0] - 100];
const SAPLING_LOWERED_AGAIN: [u64; 1] = [SAPLING_TRUTHFUL[0] - 200];

/// A second Ironwood tag base. The recorded heights served with THIS base carry a
/// different `root_hash` at every index, and a different leaf at an already-recorded
/// index is the shardtree `Conflict` §4g P2 names — `Err(Sync { EndpointUnreachable })`
/// out of `put_subtree_roots`, pinned by
/// `sync::tests::mutated_root_reput_is_insert_conflict_not_silent_overwrite`. It is
/// disjoint from [`IRONWOOD_TAG`]`..+3` and still distinct across indices, so the
/// fixture cannot instead trip the same-leaf refusal [`pool_script`] documents.
const IRONWOOD_MUTATED_TAG: u8 = IRONWOOD_TAG + 0x08;

/// How many blocks the 0a rows scan before the endpoint forks: FEWER than
/// [`crate::constants::REWIND_DISTANCE_BLOCKS`], which is the whole of §4g P1's geometry — see
/// [`induce_a_rewind_the_wallet_cannot_perform`].
const P1_SCAN_BLOCKS: u64 = 5;

fn tip() -> BlockHeight {
    BlockHeight::from_u32(TIP)
}

/// The NU6.3 mainnet activation from the COMPILED params, never a literal — and
/// asserted equal to the probe's constant, so a params bump that moved it turns
/// into a named failure here instead of silently re-aiming every fixture.
fn nu63_mainnet() -> u64 {
    use zcash_protocol::consensus::{NetworkUpgrade, Parameters};
    u64::from(u32::from(
        Network::Main
            .consensus()
            .activation_height(NetworkUpgrade::Nu6_3)
            .expect("the pinned params know Nu6_3 on mainnet"),
    ))
}

// ── The double (the contract's named seam, built here) ──────────────────────

/// A 32-byte canonical field element: a small little-endian integer is in range
/// for both `jubjub::Base` and `pallas::Base` (moduli ≈ 2^255). `tag` makes two
/// roots at the same height distinguishable, which matters because Orchard and
/// Ironwood are the identical Rust type all the way down.
fn root_at(height: u64, tag: u8) -> SubtreeRoot {
    let mut root_hash = vec![0u8; 32];
    root_hash[0] = tag;
    SubtreeRoot {
        root_hash,
        // `sync::testing::canonical_root` sets this EMPTY, which is why C6 needs
        // its own constructor (contract §4c, "Third fact from the same reading").
        completing_block_hash: Vec::new(),
        completing_block_height: height,
    }
}

/// [`root_at`] carrying a real 32-byte `completing_block_hash` — the C6 half.
fn root_at_with_hash(height: u64, tag: u8, completing_block_hash: Vec<u8>) -> SubtreeRoot {
    SubtreeRoot {
        completing_block_hash,
        ..root_at(height, tag)
    }
}

/// Per-pool tag bases. Distinct so a cross-wire between Orchard and Ironwood —
/// which share the identical Rust type all the way down and are therefore
/// invisible to the type system — would show up as the wrong bytes in the wrong
/// tree rather than as nothing at all.
const SAPLING_TAG: u8 = 0x10;
const ORCHARD_TAG: u8 = 0x30;
const IRONWOOD_TAG: u8 = 0x50;

/// A whole pool's script: `heights[i]` is the root served at subtree index `i`,
/// with `root_hash` byte `0` set to `tag_base + i`.
///
/// **The tag depends on the INDEX and never on the pass.** That is the property
/// the IT-10 discipline in this file rests on: a re-serve of index `i` carries a
/// byte-identical `root_hash`, so the shardtree's `Conflict` path can never be
/// what refuses a case named for the height dimension. The COMPLETING HEIGHT is
/// the only thing that varies between an honest pass and a hostile one here —
/// which is also the only thing the measured attack varies.
///
/// **The tag must also differ ACROSS indices**, and that is not cosmetic:
/// measured on this tree at HEAD, four roots sharing one `root_hash` make
/// `put_shard_roots` fail (`n=1 same-tag -> Ok`, `n=2..4 same-tag ->
/// Err(StoreCorrupt)`; distinct tags are `Ok` for all four). A fixture that
/// served identical leaves would refuse every pass for a reason that has nothing
/// to do with this item.
///
/// Every root carries a real 32-byte `completing_block_hash`, consistent with
/// the honest chain, for the same IT-10 reason: an empty or wrong hash would let
/// a C6-shaped cross-check refuse a case named for the height dimension.
fn pool_script(heights: &[u64], tag_base: u8) -> Vec<SubtreeRoot> {
    heights
        .iter()
        .enumerate()
        .map(|(i, h)| {
            root_at_with_hash(
                *h,
                tag_base + u8::try_from(i).expect("fixtures use few indices"),
                block_id_on(*h, false),
            )
        })
        .collect()
}

/// What one pool's stream did, recorded by [`ScriptedEndpoint`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Served {
    /// The wire value written into `GetSubtreeRootsArg.shielded_protocol`.
    protocol: i32,
    /// How many roots this double actually streamed on that call.
    roots: usize,
}

/// This file's [`SubtreeRootSource`] — see the module doc for why it is a second
/// double. Two properties `FakeRootSource` does not have:
///
/// 1. **It repeats.** The per-pool script is CLONED into each stream, so the same
///    instance can serve the same pool any number of times. No `std::mem::take`.
/// 2. **It testifies.** Every stream it opens is recorded in [`Self::served`],
///    so a refuse-case can assert the endpoint really streamed N roots for the
///    pool it names — which is what excludes "the write was refused because the
///    double had nothing left to serve" from every negative case below.
struct ScriptedEndpoint {
    sapling: Vec<SubtreeRoot>,
    orchard: Vec<SubtreeRoot>,
    ironwood: Vec<SubtreeRoot>,
    /// Inject a fault when OPENING a pool's stream (`None` = honest). Consumed
    /// on use, so a healed retry serves normally.
    sapling_open_err: Option<GrpcError>,
    orchard_open_err: Option<GrpcError>,
    ironwood_open_err: Option<GrpcError>,
    served: Arc<Mutex<Vec<Served>>>,
}

impl ScriptedEndpoint {
    fn new(
        sapling: Vec<SubtreeRoot>,
        orchard: Vec<SubtreeRoot>,
        ironwood: Vec<SubtreeRoot>,
    ) -> Self {
        Self {
            sapling,
            orchard,
            ironwood,
            sapling_open_err: None,
            orchard_open_err: None,
            ironwood_open_err: None,
            served: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// An endpoint that serves Ironwood only (the other two pools legitimately
    /// empty — the state every pool is in before its first 2^16 notes, and the
    /// state this crate's own `update_subtree_roots_empty_first_sync_then_real_converges`
    /// pins as a NON-error).
    fn ironwood_only(heights: &[u64]) -> Self {
        Self::new(Vec::new(), Vec::new(), pool_script(heights, IRONWOOD_TAG))
    }

    fn receipt(&self) -> Vec<Served> {
        self.served.lock().expect("served").clone()
    }

    /// How many roots this double actually streamed for `protocol`, summed over
    /// every call. The IT-10 receipt.
    fn streamed(&self, protocol: ShieldedProtocol) -> usize {
        self.receipt()
            .iter()
            .filter(|s| s.protocol == protocol as i32)
            .map(|s| s.roots)
            .sum()
    }
}

#[async_trait]
impl SubtreeRootSource for ScriptedEndpoint {
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        // Exhaustive with NO wildcard arm, for the reason production's own match
        // is: a fourth upstream pool must break this double's build rather than
        // let it answer for a pool it has no script for.
        let (script, open_err) = match protocol {
            ShieldedProtocol::Sapling => (&self.sapling, self.sapling_open_err.take()),
            ShieldedProtocol::Orchard => (&self.orchard, self.orchard_open_err.take()),
            ShieldedProtocol::Ironwood => (&self.ironwood, self.ironwood_open_err.take()),
        };
        if let Some(e) = open_err {
            return Err(e);
        }
        // CLONE, never take: this double answers the same question twice. The
        // script is the pool from index 0; a stream opened at `start_index` serves
        // from that index (S15-F1), and the receipt counts what was streamed.
        let items: Vec<Result<Option<SubtreeRoot>, tonic::Status>> = script
            .iter()
            .skip(start_index as usize)
            .cloned()
            .map(|r| Ok(Some(r)))
            .collect();
        self.served.lock().expect("served").push(Served {
            protocol: protocol as i32,
            roots: items.len(),
        });
        Ok(scripted_stream(items))
    }
}

// ── The scan/reorg client (C6a, C7, C14, C15 need scanned blocks) ───────────

/// A deterministic 32-byte block id for `height`, height-encoded so blocks chain
/// (`prev_hash` of `h` = id of `h-1`). `fork` selects a DIFFERENT chain: the ids
/// differ in a byte the height does not touch, so the two chains have the same
/// shape and disagree on every hash.
fn block_id_on(height: u64, fork: bool) -> Vec<u8> {
    let mut v = vec![0u8; 32];
    v[..8].copy_from_slice(&height.to_le_bytes());
    if fork {
        v[31] = 0xAB;
    }
    v
}

/// **B1-10, on this file's own chain double.** What the scripted chain's blocks
/// DECLARE their three commitment trees hold, and the anchor `TreeState` that has
/// to agree with them — the two halves §4x Q4 names, carried together so a fixture
/// cannot set one and forget the other.
///
/// Everything here routes through `sync::testing`'s knob
/// ([`DeclaredTreeSizes`], [`sync::testing::compact_block_with_sizes`],
/// [`sync::testing::frontier_hex`]), which is where the cost of declaring a
/// non-zero size is written down. This type adds only what this file's chain has
/// that `sync::testing`'s does not: the fork-aware block ids.
///
/// **Flat OR RISING (B1-10-R).** A flat chain declares `base` at every
/// height and was all this file had until the BIND-1-R join; the flat form is
/// still what every pre-existing row gets, byte for byte.
///
/// **Why the rising form had to exist, and it is a correction to the sentence
/// this doc used to carry.** The old text said a crossing "necessarily puts the
/// batch anchor's frontier inside the shard being completed", which the BIND-1
/// banner had measured as aborting the pass, and concluded that no fixture could
/// carry one. Both halves of that are wrong. A frontier inside shard 0 does NOT
/// abort the pass when exactly one root is served (measured at the join: a chain
/// declaring 1,000 leaves scans cleanly and the reconcile fires), and a crossing
/// does not need the block to carry a subtree's worth of commitments — it needs
/// the declared size to step from `2^16 - 1` to `2^16`, which is **one**. The
/// three checkers that forced the flatness are satisfied by carrying that one
/// commitment for real: [`Rise::commitments`] Ironwood actions in the block at
/// [`Rise::at`], so each block's declaration equals *anchor + the commitments the
/// block actually contains*, which is exactly what `sync::derive_chain_state`,
/// upstream's `ScanError::TreeSizeMismatch` and `put_blocks_rows` all compare.
///
/// **What a rising chain buys, and it is the reason the crypto audit asked for
/// it.** `root_bind::complete_by` and `::incomplete_through` refuse to rest a
/// durable bound on `observed[0]` — the batch ANCHOR, a raw `GetTreeState` the
/// endpoint served. On a FLAT chain declaring a whole subtree, subtree 0 is
/// already complete at the anchor and incomplete nowhere, so the only ceiling the
/// run can offer IS the anchor and nothing is minted. A rise puts the crossing on
/// two blocks the wallet SCANNED, so the bracket rests on counts of its own.
///
/// [`Self::EMPTY`] is what every pre-B1-10 row gets: three declared zeros and an
/// empty frontier at every anchor.
#[derive(Clone, Default)]
struct ChainShape {
    /// What every block BELOW [`Rise::at`] declares — and every block, when there
    /// is no rise.
    base: DeclaredTreeSizes,
    /// The one block that carries commitments, and how many (B1-10-R).
    rise: Option<Rise>,
}

/// **One block of a [`ChainShape`] that actually carries Ironwood commitments.**
///
/// Placed on a SCANNED block, never on a batch anchor: a bound minted from the
/// anchor is one the endpoint could have chosen outright, which is what
/// `root_bind::complete_by`'s `k >= 1` guard refuses.
#[derive(Clone, Copy)]
struct Rise {
    /// The height of the block carrying the commitments. Every block at or above
    /// it declares `base.ironwood + commitments`; every block below declares
    /// `base`.
    at: u64,
    /// How many Ironwood commitments that one block carries. The crossing
    /// geometry needs only ONE — `base.ironwood` is `2^16 - 1` and the step
    /// reaches `2^16`.
    commitments: u32,
}

/// A compact transaction carrying `n` Ironwood commitments and nothing else.
///
/// The cmx is the 32-byte all-zero encoding, canonical for `pallas::Base`, which
/// is what `derive_chain_state`'s `MerkleHashOrchard::from_cmx` parses and what
/// [`sync::testing::frontier_hex`] fills its nodes with — so the fixture's blocks
/// and its anchors are made of the same filler and the two cannot disagree about
/// a node value. Nothing here is decryptable and nothing needs to be: the
/// property being bought is the COUNT, which is what all three declared-size
/// checkers compare.
fn ironwood_only_tx(n: u32) -> zcash_client_backend::proto::compact_formats::CompactTx {
    use zcash_client_backend::proto::compact_formats::{CompactOrchardAction, CompactTx};
    let action = CompactOrchardAction {
        // Every field at the length the SCANNER's own decode requires —
        // nullifier 32, cmx 32, ephemeral_key 32, ciphertext 52. A short field is
        // a decode error inside `scan_cached_blocks`, not a note that simply
        // fails to decrypt, and the pass would then read as the endpoint's fault
        // before the behaviour under test ever ran.
        nullifier: vec![0u8; 32],
        cmx: vec![0u8; 32],
        ephemeral_key: vec![0u8; 32],
        ciphertext: vec![0u8; 52],
    };
    CompactTx {
        txid: vec![0u8; 32],
        ironwood_actions: vec![action; n as usize],
        ..Default::default()
    }
}

impl ChainShape {
    /// The pre-B1-10 chain: three declared zeros and empty frontiers.
    const EMPTY: Self = Self {
        base: DeclaredTreeSizes::ZERO,
        rise: None,
    };

    /// Every block declares `base`.
    fn declaring(base: DeclaredTreeSizes) -> Self {
        Self { base, rise: None }
    }

    /// `base` below `at`, and `base.ironwood + commitments` at and above it, with
    /// the block at `at` carrying those commitments for real (B1-10-R).
    fn rising(base: DeclaredTreeSizes, rise: Rise) -> Self {
        Self {
            base,
            rise: Some(rise),
        }
    }

    /// What the three trees hold as of `height`. The SAME function answers for a
    /// block's `ChainMetadata` and for the anchor `TreeState` at that height, which
    /// is the only reason the three checkers in [`DeclaredTreeSizes`]' doc agree:
    /// they are comparing two renderings of one number.
    fn sizes_at(&self, height: u64) -> DeclaredTreeSizes {
        match self.rise {
            Some(rise) if height >= rise.at => DeclaredTreeSizes {
                ironwood: self.base.ironwood.saturating_add(rise.commitments),
                ..self.base
            },
            _ => self.base,
        }
    }

    /// This chain's block at `height` on the honest (`fork == false`) or forked
    /// chain.
    fn block(&self, height: u64, fork: bool) -> CompactBlock {
        let mut b = sync::testing::compact_block_with_sizes(height, self.sizes_at(height));
        b.hash = block_id_on(height, fork);
        b.prev_hash = block_id_on(height.wrapping_sub(1), fork);
        // The ONE block that carries the commitments its own declaration claims.
        if let Some(rise) = self.rise.filter(|r| r.at == height) {
            b.vtx = vec![ironwood_only_tx(rise.commitments)];
        }
        b
    }

    /// The anchor `TreeState` at `height`: the block hash of this chain, and a
    /// frontier per pool whose SIZE is what [`Self::sizes_at`] says — which is what
    /// `put_blocks_rows`' `NonSequentialBlocks` check compares the first block's
    /// declaration against.
    fn tree_state(&self, height: u64, fork: bool) -> TreeState {
        let sizes = self.sizes_at(height);
        TreeState {
            network: "main".to_owned(),
            height,
            // DISPLAY order, through the one predicate — this line was the fixture the
            // sweep MISSED (found by that session's security review): un-reversed, so it
            // served the opposite convention from the blocks beside it. Nothing red, because
            // no row in this module runs the 20 consecutive batches a reconcile needs — which
            // is the same pre-planted shape the fork paths had.
            hash: display_order_hex(block_id_on(height, fork)),
            time: 0,
            sapling_tree: sync::testing::frontier_hex(u64::from(sizes.sapling)),
            orchard_tree: sync::testing::frontier_hex(u64::from(sizes.orchard)),
            ironwood_tree: sync::testing::frontier_hex(u64::from(sizes.ironwood)),
        }
    }
}

/// A full sync client: [`ScriptedEndpoint`]'s root script PLUS a self-consistent
/// mainnet block chain, so the REAL `scan_cached_blocks` runs and fills
/// `blocks.hash` — which is the state C6's cross-check reads, and the state a
/// reorg rewind destroys.
///
/// `fork_blocks` is how a REORG is induced deterministically, with no wall clock
/// and no randomness: the endpoint starts serving blocks from a DIFFERENT chain
/// while the wallet still holds the old one, so the first new block's `prev_hash`
/// disagrees with the wallet's stored block. That is
/// `ScanError::PrevHashMismatch`, which `is_continuity_error()`, which is the arm
/// that runs `db.truncate_to_height(rewind_target(at_height))`
/// (`sync.rs:1409-1415`). Every test that uses it ASSERTS the rewind happened
/// (the `blocks` row count drops) rather than assuming it — the mechanism is
/// named, not hoped for.
///
/// **Measured, because the first shape of this double did not work.** Forking the
/// per-batch ANCHOR (`GetTreeState`) does nothing: `scan_cached_blocks` seeds
/// `prior_block_metadata` from `data_db.block_metadata(from_height - 1)` — the
/// WALLET's own stored block — and not from `from_state`
/// (`zcash_client_backend-0.24.0/src/data_api/chain.rs:651-654`;
/// `scanning/compact.rs:272-277`). A forked-anchor pass at HEAD scanned 100 more
/// blocks and reported no reorg at all. The blocks are what must fork.
struct ScriptedChain {
    roots: ScriptedEndpoint,
    tip: u64,
    fork_blocks: bool,
    /// B1-10: what this chain's blocks DECLARE and what its anchors carry.
    /// [`ChainShape::EMPTY`] (every pre-B1-10 caller) is three zeros and an empty
    /// frontier, which is exactly what this double served before.
    shape: ChainShape,
}

impl ScriptedChain {
    fn new(roots: ScriptedEndpoint, tip: u64) -> Self {
        Self {
            roots,
            tip,
            fork_blocks: false,
            shape: ChainShape::EMPTY,
        }
    }
    /// Blocks from another chain above a higher tip ⇒ a `prev_hash` the wallet's
    /// stored block contradicts ⇒ a continuity error ⇒ a real `truncate_to_height`.
    fn reorging(mut self, tip: u64) -> Self {
        self.fork_blocks = true;
        self.tip = tip;
        self
    }
    /// Serve the chain `shape` describes rather than the three-zeros one (B1-10).
    fn shaped(mut self, shape: ChainShape) -> Self {
        self.shape = shape;
        self
    }
}

#[async_trait]
impl SubtreeRootSource for ScriptedChain {
    async fn subtree_roots(
        &mut self,
        protocol: ShieldedProtocol,
        start_index: u32,
    ) -> Result<SubtreeRootStream, GrpcError> {
        self.roots.subtree_roots(protocol, start_index).await
    }
}

#[async_trait]
impl ScanClient for ScriptedChain {
    async fn block_range(
        &mut self,
        start: u64,
        end_inclusive: u64,
    ) -> Result<BlockStream, GrpcError> {
        let blocks: Vec<_> = (start..=end_inclusive)
            .map(|h| Ok(Some(self.shape.block(h, self.fork_blocks))))
            .collect();
        Ok(scripted_stream(blocks))
    }
    async fn tree_state(&mut self, height: u64) -> Result<TreeState, GrpcError> {
        // The anchor stays on the honest chain: the continuity check reads the
        // wallet's stored block, so forking this would change nothing (see the
        // struct doc) and would only muddy which mechanism the rewind came from.
        Ok(self.shape.tree_state(height, false))
    }
    async fn latest_block_height(&mut self) -> Result<u64, GrpcError> {
        Ok(self.tip)
    }
}

// ── Wallet + observation harness ────────────────────────────────────────────

fn test_vault() -> Arc<dyn KeychainPort> {
    Arc::new(TestVault::new(VaultTier::Tee))
}

fn cfg(db_dir: &Path, network: Network) -> WalletConfig {
    WalletConfig {
        db_dir: db_dir.to_path_buf(),
        network,
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
    SeedSource::raw_bytes(vec![0x24; 32]).expect("valid seed")
}

/// Create a wallet AND capture its SQLCipher key, so every later assertion can
/// read the shard tables through a second connection (the production `aux_db`
/// pattern) WITHOUT the handle being closed.
///
/// The key is reachable only through `store::open`, which needs the single-writer
/// `WalletLock` the live handle holds — hence the close/capture/re-open dance.
/// It runs once per test, before any root is ingested, so nothing it does can be
/// mistaken for the behaviour under test.
async fn wallet_and_key(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    network: Network,
) -> (Wallet, WalletDbKey) {
    let w = Wallet::create_with_vault(cfg(dir, network), raw_seed(), Arc::clone(vault))
        .await
        .expect("create");
    w.close().await.expect("close");
    let key = {
        let lock = crate::lifecycle::WalletLock::acquire(dir).expect("acquire the wallet lock");
        let opened = crate::store::open(&lock, &**vault, network).expect("store open");
        let crate::store::OpenWallet {
            db_key, db, aux_db, ..
        } = opened;
        drop(db);
        drop(aux_db);
        db_key
    };
    let w = Wallet::open_with_vault(cfg(dir, network), Arc::clone(vault))
        .await
        .expect("re-open after capturing the db key");
    (w, key)
}

/// A second keyed connection onto the same wallet file.
fn observe(dir: &Path, key: &WalletDbKey) -> rusqlite::Connection {
    crate::db::open_existing_keyed_connection(&dir.join("wallet.db"), key)
        .expect("a second keyed connection onto the wallet db")
}

/// Every recorded `(shard_index, subtree_end_height)` for a pool, in index order.
/// This is upstream's own table — the column `put_shard` provably cannot write
/// and only `put_shard_roots` fills — NOT the bind's memory. Every refuse-case
/// below states which mechanism refused by naming what this returns.
fn recorded(conn: &rusqlite::Connection, pool: &str) -> Vec<(i64, Option<i64>)> {
    let sql = format!(
        "SELECT shard_index, subtree_end_height FROM {pool}_tree_shards ORDER BY shard_index"
    );
    let mut stmt = conn.prepare(&sql).expect("the shard table exists");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("query shard rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("shard rows decode")
}

/// Just the heights, for a readable assertion against the probe's own arrays.
fn heights(conn: &rusqlite::Connection, pool: &str) -> Vec<Option<i64>> {
    recorded(conn, pool).into_iter().map(|(_, h)| h).collect()
}

fn expect_heights(hs: &[u64]) -> Vec<Option<i64>> {
    hs.iter().map(|h| Some(*h as i64)).collect()
}

fn count_rows(conn: &rusqlite::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap_or_else(|e| panic!("counting {table}: {e}"))
}

/// A typed `Sync` refusal, whatever stall it carries — the shape every
/// endpoint-data fault in this module already has. Deliberately NOT pinned to a
/// specific `StallReason`: which one a height violation deserves is question 5,
/// which the contract leaves to the implementer, and a test that pinned it would
/// be the test choosing the answer.
/// **ADJUDICATOR REPAIR (T0-1a).** As written this accepted only
/// `Err(Sync { .. })`, i.e. *fatal to the pass* — which is one of the two answers
/// the contract's **question 5** explicitly left to the implementer ("fatal to the
/// pass, or a degraded pool?"). The doc comment claimed to leave question 5 open
/// and the predicate closed it. A degraded pool reaches this caller as
/// `Ok([.., PoolFetch::HeightViolation { .. }, ..])`, which is the C5 shape the
/// contract requires to exist, so both are accepted here and the "nothing was
/// written" assertions beside every call site are what carry the refusal's force.
fn is_typed_refusal(
    r: &Result<[sync::PoolFetch; sync::SUBTREE_ROOT_POOLS.len()], WalletError>,
) -> bool {
    match r {
        Err(WalletError::Sync { .. }) => true,
        Ok(outcomes) => outcomes
            .iter()
            .any(|o| matches!(o, sync::PoolFetch::HeightViolation { .. })),
        Err(_) => false,
    }
}

fn describe(r: &Result<[sync::PoolFetch; sync::SUBTREE_ROOT_POOLS.len()], WalletError>) -> String {
    match r {
        Ok(o) => format!("Ok({o:?})"),
        Err(e) => format!("Err({e:?} / code {})", e.code()),
    }
}

/// **Which ARM refused** (§4e R8) — the stable code
/// [`sync::PoolFetch::HeightViolation`] carries, or `None` when nothing was
/// refused as a per-pool outcome.
///
/// `None` covers two very different states and the caller must say which it
/// expects: the pass was ACCEPTED, or the pass was refused fatally through
/// `Err(Sync { .. })` with the arm's identity thrown away. Only the Ironwood arm
/// carries a code out of `apply_height_bind`; see the module doc for why, and why
/// that makes every attribution row in this file an Ironwood row. So a row that
/// asserts on this must ALSO assert on the record, which is the only evidence a
/// Sapling/Orchard refusal leaves behind.
fn refusal_code(
    r: &Result<[sync::PoolFetch; sync::SUBTREE_ROOT_POOLS.len()], WalletError>,
) -> Option<&'static str> {
    match r {
        Ok(outcomes) => outcomes.iter().find_map(|o| match o {
            sync::PoolFetch::HeightViolation { code } => Some(*code),
            _ => None,
        }),
        Err(_) => None,
    }
}

/// **The pass was ACCEPTED** — `Ok`, with no pool carrying a height violation.
///
/// Written as its own predicate rather than as `refusal_code(..) == None`, because
/// that comparison is true of two OPPOSITE states: accepted, and refused fatally
/// through `Err` with the arm's identity dropped. An acceptance assertion written
/// on the `None` alone would pass on a transport fault, on a `StoreCorrupt`, and on
/// every Sapling/Orchard height violation — which is the extractor-vacuity class
/// this file exists to keep out (a proof that passes because its instrument
/// collapsed two answers into one).
fn accepted(r: &Result<[sync::PoolFetch; sync::SUBTREE_ROOT_POOLS.len()], WalletError>) -> bool {
    matches!(r, Ok(outcomes) if !outcomes
        .iter()
        .any(|o| matches!(o, sync::PoolFetch::HeightViolation { .. })))
}

/// The span of blocks this wallet has actually scanned, `None` when it has scanned
/// nothing.
///
/// Every row that scans asserts this, because the SCANNED arm's verdict is a
/// function of where the scan window sits relative to the served completing
/// heights — window above ⇒ it refuses, window below ⇒ it abstains — and a row
/// that did not pin the window down would silently swap one mechanism for another
/// if the fixture chain ever moved.
fn scanned_span(conn: &rusqlite::Connection) -> Option<(u64, u64)> {
    let (lo, hi): (Option<u64>, Option<u64>) = conn
        .query_row("SELECT MIN(height), MAX(height) FROM blocks", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .expect("the blocks table exists");
    lo.zip(hi)
}

// ════════════════════════════════════════════════════════════════════════════
// The named tests. Names are the contract's; a rename is a finding, not a
// liberty (§4c "Named tests").
// ════════════════════════════════════════════════════════════════════════════

/// **C1 — the measured attack fails.**
///
/// An honest pass records the truthful heights; a second pass from the SAME
/// wallet serves the compressed sequence verbatim from the probe
/// (`{0:3428143, 1:3428144, 2:3428145, 3:3428146}`) with otherwise-correct root
/// hashes, and is REFUSED — with nothing from that pool written.
///
/// **Which mechanism refused the write** (the seam note in §4c; IT-10). Three
/// things are asserted together and only the bind satisfies all three:
/// 1. the double STREAMED four Ironwood roots on pass 2 (`streamed(...) == 4`),
///    so `put_shard_roots`' empty-slice early return cannot be the refuser and
///    neither can `FakeRootSource`'s `mem::take`;
/// 2. `update_subtree_roots` returned a typed refusal, so "accepted and
///    overwritten" is excluded;
/// 3. the recorded per-index heights are still the TRUTHFUL ones, so a partial
///    write is excluded.
///
/// The root HASHES are identical across the two passes (`tag = 3`), so the
/// shardtree `Conflict` path cannot be the refuser either. The only thing that
/// changed between the accepted pass and the refused one is
/// `completing_block_height` — which is exactly what the attack changes.
///
/// **WHICH ARM, measured (T0-1a-R, R8) — and it is not the one this row's
/// name suggests.** The refusal is `completion_gap`, not `recorded_height`. The
/// probe's compressed sequence is `activation + {0,1,2,3}`, four heights one block
/// apart, and the block-size floor refuses it before the recorded bind is ever
/// consulted. That is the CORRECT answer for this row — C1 asks whether the
/// measured attack fails, not which oracle catches it, and the cheapest sufficient
/// oracle catching it is a good property — but it has a consequence worth stating
/// where the next reader will find it: **C1 cannot see the recorded bind die.**
/// It is the same shape §4d owed row 5 records for C4 and C12, in the row that
/// carries the measured attack. `a_recorded_height_cannot_be_walked_down_without_a_rewind`
/// is the row that does exercise the recorded bind, and it exists because this one
/// does not. The arm is pinned rather than described so a silent re-attribution
/// here is a named failure.
#[tokio::test]
async fn a_compressed_height_sequence_is_refused_after_an_honest_pass() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    assert_eq!(
        nu63_mainnet(),
        ACTIVATION,
        "the compiled params must still put NU6.3 mainnet where the probe found it; \
         if this moves, every fixture below is re-aimed"
    );
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    // Pass 1 — the honest endpoint (probe row B).
    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("an honest endpoint's truthful heights are ingested");
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "precondition: the truthful heights are what the wallet now holds"
    );

    // Pass 2 — the SAME roots, the compressed heights (probe row C).
    let mut hostile = ScriptedEndpoint::ironwood_only(&COMPRESSED);
    let r = w.update_subtree_roots(&mut hostile, tip(), None).await;

    assert_eq!(
        hostile.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the hostile endpoint really did stream four Ironwood roots, so \
         whatever refused the write, it was not an empty stream"
    );
    assert!(
        is_typed_refusal(&r),
        "the compressed sequence must be REFUSED, not half-believed. \
         Probe row C is a note that cannot be witnessed being marked spendable. Got {}",
        describe(&r)
    );
    assert_eq!(
        refusal_code(&r),
        Some("completion_gap"),
        "R8 — WHICH ARM. The measured attack packs four completions into four \
         consecutive blocks, so the block-size floor refuses it before the recorded \
         bind is reached. If this ever reads `recorded_height` the floor stopped \
         covering the measured attack; if it reads anything else the arm order \
         moved. Either is a finding to be ruled on, not a defect on its face. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "and NOTHING from that pool is written: the recorded heights are still the \
         truthful ones, not the endpoint's compression"
    );
    w.close().await.expect("close");
}

/// **C2 — idempotence survives.** Every sync re-fetches from index 0, so a bind
/// that reads a re-serve as a violation stops the wallet dead. Three passes, the
/// same truthful sequence, all accepted.
#[tokio::test]
async fn an_honest_endpoint_reserving_the_same_heights_is_accepted_twice() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    for pass in 1..=3 {
        let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
        w.update_subtree_roots(&mut honest, tip(), None)
            .await
            .unwrap_or_else(|e| {
                panic!("pass {pass}: an unchanged re-serve must be accepted, got {e:?}")
            });
        assert_eq!(
            honest.streamed(ShieldedProtocol::Ironwood),
            4,
            "pass {pass}: the endpoint served the full sequence again (no mem::take)"
        );
        assert_eq!(
            heights(&obs, "ironwood"),
            expect_heights(&TRUTHFUL),
            "pass {pass}: the recorded heights are unchanged"
        );
    }
    w.close().await.expect("close");
}

/// **C3 — growth survives.** Indices 0..3 at the truthful heights, then a pass
/// with 0..3 identical and a NEW index 4 at a higher height, accepted.
#[tokio::test]
async fn a_growing_root_sequence_is_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("pass 1");

    let grown: Vec<u64> = TRUTHFUL.iter().copied().chain([3_475_300]).collect();
    let mut growing = ScriptedEndpoint::ironwood_only(&grown);
    w.update_subtree_roots(&mut growing, tip(), None)
        .await
        .expect(
            "a new subtree at a HIGHER height must be accepted — this is what a \
                 chain doing its job looks like",
        );

    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&grown),
        "the fifth shard is recorded and the first four are unchanged"
    );
    w.close().await.expect("close");
}

/// **C4 — refusal is whole-pool, never a partial accept.** `put_shard_roots`
/// writes positionally from `start_index`, so a truncated prefix is silently
/// WRONG rather than merely short.
///
/// The hostile pass serves five roots: indices 0 and 1 truthful, indices 2 and 3
/// LOWERED (a violation the wallet's own record contradicts, but one that passes
/// every clause A11 applies today — strictly increasing, above the activation,
/// below the tip), and a NEW index 4 at a legitimately higher height.
///
/// **Index 4 is the observable.** A partial accept — write the valid prefix, or
/// write the new tail — creates a fifth shard row. Its ABSENCE is what "nothing
/// for that pool" means here, and it is not the trivial absence of a row nobody
/// tried to write: the last block of the test serves the same index-4 root in a
/// legal sequence and watches the row appear. So the mechanism that suppressed
/// it is the refusal, not the fixture.
#[tokio::test]
async fn a_height_violation_writes_nothing_for_that_pool() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("pass 1");

    // increasing, >= activation, <= tip — all three of today's clauses pass.
    //
    // **ADJUDICATOR REPAIR (T0-1a) — IT-10.** As written, indices 2 and 3
    // were `TRUTHFUL[1] + 1` and `+ 2`, i.e. three CONSECUTIVE heights. The join
    // refused that with `HeightViolation { code: "completion_gap" }` — the
    // block-size floor, not the recorded-height contradiction this row is named
    // for — so the case could not see the recorded bind break. The lowered indices
    // are now spaced 100 blocks apart: still a contradiction of what the wallet
    // recorded at indices 2 and 3, no longer a contradiction of the gap floor.
    let partly_lowered: [u64; 5] = [
        TRUTHFUL[0],
        TRUTHFUL[1],
        TRUTHFUL[1] + 100,
        TRUTHFUL[1] + 200,
        3_475_300,
    ];
    let mut hostile = ScriptedEndpoint::ironwood_only(&partly_lowered);
    let r = w.update_subtree_roots(&mut hostile, tip(), None).await;

    assert_eq!(
        hostile.streamed(ShieldedProtocol::Ironwood),
        5,
        "IT-10: five roots were streamed, so the empty-slice early return in \
         put_shard_roots is not what stopped the write"
    );
    assert!(
        is_typed_refusal(&r),
        "a height a recorded index contradicts is a typed refusal for the pool, got {}",
        describe(&r)
    );
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "R7/R8 (§4d owed row 5, IT-10) — WHICH ARM. This row is named for the \
         RECORDED bind and must be refused by it. The adjudicator already moved \
         this fixture once, off three consecutive heights that the block-size floor \
         was refusing; without this assertion nothing stops it drifting back, and a \
         row refused by the floor cannot watch the recorded bind die. Got {}",
        describe(&r)
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        4,
        "WHOLE-POOL: no fifth shard row exists, so the valid tail of a violating \
         sequence was not written either"
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "WHOLE-POOL: and the valid PREFIX was not re-written over either"
    );

    // The anti-vacuity control: the same index-4 root, in a legal sequence.
    let grown: Vec<u64> = TRUTHFUL.iter().copied().chain([3_475_300]).collect();
    let mut legal = ScriptedEndpoint::ironwood_only(&grown);
    w.update_subtree_roots(&mut legal, tip(), None)
        .await
        .expect("control: the same fifth root, served legally, IS accepted");
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        5,
        "control: index 4 can be written, so its absence above is attributable to \
         the refusal and not to the fixture"
    );
    w.close().await.expect("close");
}

/// **C12 — INFLATION is refused, not only deflation.** A first pass serving
/// `{tip-3, tip-2, tip-1, tip}` passes every clause today AND would pass a
/// monotonic bind. This is the row that stops the cap-erasure chain the design
/// review measured (§4c review row 3): inflate, induce a rewind, and
/// `truncate_tree_to_subtree_roots` finds no shard at or below the truncation
/// height, so it drops shards, checkpoints **and** `{prefix}_tree_cap` — the one
/// structure that binds root hashes.
///
/// It is a FIRST-CONTACT case on purpose: there is nothing recorded to compare
/// against, so a bind whose only memory is the wallet's own earlier acceptance
/// cannot see it. The contract names two oracles that can (the bundled
/// treestates, and `blocks.{prefix}_commitment_tree_size`).
///
/// **T0-1a-R REPAIR — §4d owed row 5, §4e R7/R8, and this is the row the
/// adjudicator said "still is".** The sentence that used to close this comment
/// read *"which one is used is the implementer's answer to question 1 and this
/// test does not care"*, and not caring is exactly what made the row vacuous:
/// measured on `t0-1a-join`, `{tip-3, tip-2, tip-1, tip}` is refused by
/// `completion_gap` — the arithmetic floor — so **C12 stayed green with the whole
/// counting bind removed**, which is the anti-vacuity finding §4d records against
/// it. Not caring which mechanism refuses is only safe when every mechanism that
/// COULD refuse is one the row would accept dying.
///
/// The row now drives BOTH shapes and names the arm for each:
/// 1. the contract's literal sequence, refused by `completion_gap`. Kept, because
///    it is the shape C12 is written against and because "the cheap floor already
///    covers the obvious form" is worth pinning;
/// 2. [`INFLATED_SPACED`] — the same claim, completions 100 blocks apart, which
///    the floor cannot reach. It is refused by `bundled_frontier`, the signed
///    binary, which is the mechanism this row is actually named for. Kill the
///    counting bind and step 2 goes green-to-red; kill the floor and step 1 does.
#[tokio::test]
async fn an_inflated_height_sequence_is_refused_on_first_contact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "precondition: FIRST CONTACT — this wallet has recorded nothing"
    );

    // (1) The contract's literal sequence: four completions in four consecutive
    //     blocks. The block-size floor reaches it without any oracle at all.
    let mut hostile = ScriptedEndpoint::ironwood_only(&INFLATED);
    let r = w.update_subtree_roots(&mut hostile, tip(), None).await;

    assert_eq!(
        hostile.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the four inflated roots were streamed"
    );
    assert!(
        is_typed_refusal(&r),
        "an endpoint claiming every subtree completed in the last four blocks must \
         not be able to record itself — a bind written as `>=` passes C1 and fails \
         HERE. Got {}",
        describe(&r)
    );
    assert_eq!(
        refusal_code(&r),
        Some("completion_gap"),
        "R8 — WHICH ARM refused the literal sequence. One block cannot carry 2^16 \
         note commitments, so consecutive completions are impossible arithmetic and \
         the floor is the right refuser here. Named so that step 2 below is \
         visibly a DIFFERENT measurement and not a second copy of this one. Got {}",
        describe(&r)
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "and nothing is written, so the erasure chain has no shard heights to work with"
    );

    // (2) THE ROW'S OWN MECHANISM (§4d owed row 5). The same inflation, spaced so
    //     the floor cannot be what refuses it. On a wallet that has recorded
    //     nothing and scanned nothing, the recorded arm, the scanned arm and the
    //     completing-hash arm all abstain through their own explicit branches, so
    //     the counting bind is the only thing left that can answer.
    let mut spaced = ScriptedEndpoint::ironwood_only(&INFLATED_SPACED);
    let r = w.update_subtree_roots(&mut spaced, tip(), None).await;
    assert_eq!(
        spaced.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the spaced inflation was streamed too"
    );
    assert_eq!(
        refusal_code(&r),
        Some("bundled_frontier"),
        "THE ROW'S OWN MECHANISM. An endpoint that spaces its lie past the \
         block-size floor is refused by the SIGNED BINARY: the bundled mainnet \
         Ironwood frontier at 3,452,280 already holds one complete subtree, so \
         subtree 0 cannot have completed 22,919 blocks later. This is the \
         assertion §4d owed row 5 is about — with the whole counting bind removed \
         the literal sequence above still fails and this one no longer does. Got {}",
        describe(&r)
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "and the spaced inflation writes nothing either — still first contact"
    );

    // Anti-vacuity: first contact is not refused wholesale — the TRUTHFUL
    // sequence, from the same fresh state, IS accepted.
    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("control: a truthful first-contact sequence is accepted");
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "control: so the refusal above discriminates, it does not blanket-refuse"
    );
    w.close().await.expect("close");
}

/// **C13 — a hostile pass cannot brick an honest one.**
///
/// `fetch_tip` range-checks `u32` and, since T0-1c, grades the tip against the
/// signed bundle's newest row (`sync::tip_standing`) — a standing the pass
/// REPORTS, never a refusal — with no bound against the previous tip or the wall
/// clock. So an endpoint may claim any
/// tip it likes and A11's ceiling clause admits it. **A bind whose memory
/// advances on a number the endpoint solely attests turns one hostile packet into
/// a permanent refusal from every honest server** — and on Sapling or Orchard
/// that refusal is fatal to the whole pass, i.e. the wallet stops syncing. That
/// is strictly worse than the bug being fixed.
///
/// Pass 1's outcome is deliberately NOT asserted: whether the wallet swallows the
/// hostile sequence (today) or refuses it (after C12) is not this row's question.
/// This row's question is only whether the honest server that follows is served.
#[tokio::test]
async fn an_honest_endpoint_is_accepted_after_a_hostile_pass_claimed_a_high_tip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    // A tip 400k blocks past the real one, and a sequence pinned just under it.
    let liar_tip = BlockHeight::from_u32(3_900_000);
    let sky_high: [u64; 4] = [3_899_996, 3_899_997, 3_899_998, 3_899_999];
    let mut liar = ScriptedEndpoint::ironwood_only(&sky_high);
    let _ = w.update_subtree_roots(&mut liar, liar_tip, None).await;
    assert_eq!(
        liar.streamed(ShieldedProtocol::Ironwood),
        4,
        "the hostile pass really happened"
    );

    // ...and now an honest server, with the real tip and the truthful heights.
    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        r.is_ok(),
        "an honest endpoint must still be served after a hostile pass claimed a \
         high tip. A permanent refusal here is a wallet that has stopped syncing \
         because one packet said a big number. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "and the truthful heights are what the wallet ends up holding"
    );
    w.close().await.expect("close");
}

/// **C16 — a pass that fails on a later pool leaves the bind's memory exactly as
/// it was.** `fetch_subtree_roots` validates all three pools and only then does
/// `put_subtree_roots` run under the db lock, so a fault on pool 3 returns `Err`
/// with nothing written. C4's window is a pool; this window is the PASS.
///
/// Two directions, because "exactly as it was" has two ways to break:
/// * **not advanced** — pass 2's grown Sapling sequence must not be remembered;
/// * **not erased** — the compression pass 1 would have refused must STILL be
///   refused after the faulting pass. This is the half that can go red at HEAD,
///   and it is the half that matters: a bind that resets its memory on any failed
///   pass is a bind an endpoint can clear at will by faulting a later pool.
#[tokio::test]
async fn a_fault_on_a_later_pool_leaves_the_bind_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    // Pass 1: an honest endpoint, Sapling recorded.
    let mut honest = ScriptedEndpoint::new(
        pool_script(&SAPLING_TRUTHFUL, SAPLING_TAG),
        Vec::new(),
        Vec::new(),
    );
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("pass 1");
    assert_eq!(heights(&obs, "sapling"), expect_heights(&SAPLING_TRUTHFUL));

    // Pass 2: ORCHARD arrives legitimately, then IRONWOOD (pool 3) faults.
    //
    // **ADJUDICATOR REPAIR (T0-1a).** The advance observable was a FIFTH
    // Sapling shard at 1,000,000, an invented mainnet completion height the
    // bundled frontiers refute (real Sapling index 1 sits between the measured
    // 558,822 and 916,404, and a bundled row inside that window already shows two
    // complete). Indices 1..3 of a mainnet Sapling sequence are not recoverable
    // from anything in this repository, so the advance is observed on the ORCHARD
    // pool's measured first root instead: a partial write creates an
    // `orchard_tree_shards` row, and its absence is the same measurement.
    let mut faulting = ScriptedEndpoint::new(
        pool_script(&SAPLING_TRUTHFUL, SAPLING_TAG),
        pool_script(&ORCHARD_TRUTHFUL, ORCHARD_TAG),
        Vec::new(),
    );
    faulting.ironwood_open_err = Some(GrpcError::Transport {
        stall: crate::state::StallReason::EndpointUnreachable,
    });
    let r = w.update_subtree_roots(&mut faulting, tip(), None).await;
    assert!(
        is_typed_refusal(&r),
        "a transport fault on the last pool is fatal to the pass, got {}",
        describe(&r)
    );
    assert_eq!(
        recorded(&obs, "orchard").len(),
        0,
        "NOT ADVANCED: pass 2's Orchard root was streamed but never written, so \
         nothing can have learned it"
    );
    assert_eq!(
        heights(&obs, "sapling"),
        expect_heights(&SAPLING_TRUTHFUL),
        "NOT ADVANCED: and Sapling is exactly what pass 1 left"
    );

    // Direction 2 — NOT ERASED. The compression must still be refused.
    let mut compressor = ScriptedEndpoint::new(
        pool_script(&SAPLING_COMPRESSED, SAPLING_TAG),
        Vec::new(),
        Vec::new(),
    );
    let r = w.update_subtree_roots(&mut compressor, tip(), None).await;
    assert_eq!(
        compressor.streamed(ShieldedProtocol::Sapling),
        SAPLING_COMPRESSED.len(),
        "IT-10: the compressed Sapling sequence was streamed"
    );
    assert!(
        is_typed_refusal(&r),
        "NOT ERASED: the failed pass must not have cleared what the wallet knows. \
         If a fault on a later pool wipes the bind, an endpoint clears it at will \
         by dropping the last stream. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "sapling"),
        expect_heights(&SAPLING_TRUTHFUL),
        "and still nothing was written"
    );

    // And the honest re-serve still works, so the memory is intact rather than poisoned.
    let mut honest_again = ScriptedEndpoint::new(
        pool_script(&SAPLING_TRUTHFUL, SAPLING_TAG),
        Vec::new(),
        Vec::new(),
    );
    w.update_subtree_roots(&mut honest_again, tip(), None)
        .await
        .expect("the unchanged honest sequence is still accepted after all of that");
    w.close().await.expect("close");
}

/// **C10 — Sapling and Orchard.** The same assertion as C1 for each of the other
/// two pools. The design review looked for a compensating property in them and
/// found none: their activations sit FURTHER below any modern birthday, which
/// makes compression easier, and no bundled treestate or checkpoint seeds
/// `subtree_end_height`. `validate_root_sequence` is called for all three pools
/// (`sync.rs:601`, `:615`, `:640`) and upstream's unconditional
/// `DO UPDATE SET subtree_end_height` is the single write path for all three, so
/// fixing the Ironwood instance and leaving the siblings in the same function is
/// the exact defect shipped.
#[tokio::test]
async fn the_same_compression_is_refused_for_sapling_and_orchard() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    let mut honest = ScriptedEndpoint::new(
        pool_script(&SAPLING_TRUTHFUL, SAPLING_TAG),
        pool_script(&ORCHARD_TRUTHFUL, ORCHARD_TAG),
        Vec::new(),
    );
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("pass 1");
    assert_eq!(heights(&obs, "sapling"), expect_heights(&SAPLING_TRUTHFUL));
    assert_eq!(heights(&obs, "orchard"), expect_heights(&ORCHARD_TRUTHFUL));

    // SAPLING compressed to `activation + k` — the probe's own attack shape.
    let mut sapling_attack = ScriptedEndpoint::new(
        pool_script(&SAPLING_COMPRESSED, SAPLING_TAG),
        pool_script(&ORCHARD_TRUTHFUL, ORCHARD_TAG),
        Vec::new(),
    );
    let r = w
        .update_subtree_roots(&mut sapling_attack, tip(), None)
        .await;
    assert_eq!(
        sapling_attack.streamed(ShieldedProtocol::Sapling),
        SAPLING_COMPRESSED.len(),
        "IT-10"
    );
    assert!(
        is_typed_refusal(&r),
        "the SAPLING compression must be refused too, got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "sapling"),
        expect_heights(&SAPLING_TRUTHFUL),
        "nothing written for sapling"
    );

    // ORCHARD compressed, Sapling honest — so the Sapling arm cannot be what refuses.
    let mut orchard_attack = ScriptedEndpoint::new(
        pool_script(&SAPLING_TRUTHFUL, SAPLING_TAG),
        pool_script(&ORCHARD_COMPRESSED, ORCHARD_TAG),
        Vec::new(),
    );
    let r = w
        .update_subtree_roots(&mut orchard_attack, tip(), None)
        .await;
    assert_eq!(
        orchard_attack.streamed(ShieldedProtocol::Orchard),
        ORCHARD_COMPRESSED.len(),
        "IT-10"
    );
    assert!(
        is_typed_refusal(&r),
        "the ORCHARD compression must be refused too, got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "orchard"),
        expect_heights(&ORCHARD_TRUTHFUL),
        "nothing written for orchard"
    );
    w.close().await.expect("close");
}

/// **C9 — a fresh wallet is not bricked.** First contact against an honest
/// endpoint, on a wallet that has scanned NOTHING, still writes the roots and
/// still heals the §3a stuck state: `ironwood_tree_shards.subtree_end_height` is
/// the column `put_shard` provably cannot write, and a NULL there is what keeps
/// every Ironwood note out of `mark_stabilized_notes` and drops Ironwood out of
/// `min_shard_tip`.
#[tokio::test]
async fn a_first_contact_wallet_still_ingests_and_still_heals() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    assert_eq!(
        count_rows(&obs, "blocks"),
        0,
        "precondition: nothing scanned"
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "precondition: the stuck state — no ironwood shard rows at all"
    );

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("a fresh wallet must still ingest from an honest endpoint");

    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "THE HEAL: every shard's completing height is recorded, non-NULL, at its \
         own index — with zero scanned blocks"
    );
    assert!(
        heights(&obs, "ironwood").iter().all(|h| h.is_some()),
        "and none of them is NULL, which is the §3a stuck state itself"
    );
    w.close().await.expect("close");
}

// ── The rows that need a scanned chain (C6, C7, C14, C15) ───────────────────

// **`SCAN_BASE` / `SCAN_TIP` / `scan_tip()` are GONE (B1-9).** They were the
// birthday anchor and tip of a Sapling window at 3,428,000 whose doc said "an
// Ironwood root down here would be refused by A11's activation clause rather than
// by the mechanism the case is named after". The two rows that used them were the
// C6 pair, and the binding half is exactly the row §4w (f) measured as VACUOUS:
// mainnet Sapling subtree 0 completed at 558,822, so a bundled row refuted every
// height in that window offline and the count clause answered before the hash
// clause ever ran. The binding half moved to the Ironwood window
// ([`OVER_SCAN_BASE`], [`IRONWOOD_COMPLETING`]) where every earlier oracle abstains
// or AGREES; the abstaining half already sits at `SAPLING_TRUTHFUL[0]` after the
// adjudicator's repair. Nothing else referenced them, so they are deleted
// rather than left as a window no row can use.

/// C14's fixture sits at the top of the chain instead, because inflation is
/// defined relative to the tip and the erasure it triggers is Ironwood-specific.
///
/// **ADJUDICATOR NOTE (T0-1a) — and the block it recorded is LIFTED
/// by B1-10.** The note read: *"This double declares `*_commitment_tree_size = 0`
/// on every block, so once oracle (d) is in the bind the wallet's own scanned count
/// says ZERO subtrees were complete at the scan tip, and every served root whose
/// completing height sits at or below that tip is refused… Making the chain honest
/// needs a non-empty frontier anchor, because upstream's scanner checks each block's
/// declared tree size against the batch's own anchor (`ScanError::TreeSizeMismatch`)
/// and an empty frontier pins that anchor at zero. This repository has no such
/// fixture."* It does now: [`ChainShape`] declares a per-pool size on the blocks AND
/// carries the matching frontier on every anchor, and
/// [`the_declared_tree_size_knob_is_accepted_by_both_checkers`] measures that all
/// three readers take it. The rows in this section that still declare ZERO do so
/// because they WANT the scanned count oracle to refuse, not because they cannot do
/// otherwise.
const CAP_BASE: u64 = 3_475_400;
const CAP_TIP: u64 = TIP as u64;

const NOOP_PROGRESS: &(dyn Fn(sync::ScanProgress) + Sync) = &|_: sync::ScanProgress| {};

/// The endpoint tip the Ironwood scan-window rows serve under — [`OVER_SCAN_TIP`],
/// so A11's ceiling clause (served at or below the reported tip) passes for
/// [`IRONWOOD_COMPLETING`] by arithmetic rather than by luck.
fn ironwood_tip() -> BlockHeight {
    BlockHeight::from_u32(OVER_SCAN_TIP as u32)
}

/// A wallet with an imported account anchored at `anchor`, so `suggest_scan_ranges`
/// has something to suggest and the REAL `scan_cached_blocks` runs.
async fn scanned_wallet(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    anchor: u64,
) -> (Wallet, WalletDbKey) {
    scanned_wallet_shaped(dir, vault, anchor, &ChainShape::EMPTY).await
}

/// [`scanned_wallet`] whose birthday anchor carries the frontier `shape` declares
/// at `anchor` (B1-10). The birthday IS the first batch's `from_state`, so a
/// fixture that shaped its blocks and left the birthday empty would be refused by
/// `put_blocks_rows` before the row's own mechanism ran — which is the half of Q4
/// the adjudicator hit.
async fn scanned_wallet_shaped(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    anchor: u64,
    shape: &ChainShape,
) -> (Wallet, WalletDbKey) {
    let (w, key) = wallet_and_key(dir, vault, Network::Main).await;
    let birthday = crate::account::birthday_from_treestate(shape.tree_state(anchor, false))
        .expect("the anchor treestate decodes");
    w.import_account(birthday).await.expect("import account");
    (w, key)
}

/// One subtree, in leaves — `2^16`, the shard height all three pools share
/// (`root_bind`'s `SUBTREE_LEAVES`, named there from
/// `zcash_client_sqlite-0.22.0/src/lib.rs`). Spelled here rather than imported
/// because `root_bind`'s copy is private, and asserted against the oracle's own
/// arithmetic by [`the_declared_tree_size_knob_is_accepted_by_both_checkers`]: a
/// chain declaring `SUBTREE_LEAVES - 1` must leave the count oracle at ZERO
/// complete subtrees and one declaring `SUBTREE_LEAVES` must move it to one, so a
/// shard height that ever changed would redden that row instead of silently
/// re-aiming every fixture built on this constant.
const SUBTREE_LEAVES: u32 = 1 << 16;

/// The Ironwood window B1-9 works in, and why every oracle but the hash one
/// AGREES there.
///
/// Measured from `root_bind::bundled_counts(Main, Ironwood)` (the numbers
/// [`BUNDLED_ONLY_VIOLATION`]'s doc records): the last mainnet row showing ZERO
/// complete Ironwood subtrees is **3,449,780** and the first showing one is
/// **3,452,280**, so the signed bundle pins subtree 0's completion into that
/// window and ACCEPTS any served height inside it. [`OVER_SCAN_BASE`] `..=`
/// [`OVER_SCAN_TIP`] sits inside the window, which is what lets the wallet SCAN
/// the completing block — the precondition the hash oracle needs and the reason
/// the height cannot simply be moved somewhere more convenient.
const IRONWOOD_COMPLETING: u64 = OVER_SCAN_BASE + 50;

/// **B1-10 / Q4 — the mechanism row. What the knob is, and that BOTH checkers take
/// it.**
///
/// Four rows have been blocked since an earlier revision on one sentence: *"Making the chain
/// honest needs a non-empty frontier anchor, because upstream's scanner checks each
/// block's declared tree size against the batch's own anchor
/// (`ScanError::TreeSizeMismatch`) and an empty frontier pins that anchor at zero.
/// This repository has no such fixture."* ([`CAP_BASE`]'s note.) This row is that
/// fixture, and it measures the three readers rather than arguing about them:
///
/// 1. **The hand-built frontier really decodes to the size it claims.** The bytes
///    [`sync::testing::frontier_hex`] writes go back through the SAME upstream
///    reader production uses — `TreeState::to_chain_state` — and the decoded
///    `ChainState`'s per-pool `tree_size()` is asserted, at the boundary sizes
///    (0, 1, 2, 3) and at the two that matter (`SUBTREE_LEAVES - 1` and
///    `SUBTREE_LEAVES`). A mis-encoded frontier cannot pass as a smaller tree.
/// 2. **OURS** — `sync::derive_chain_state` refuses `endpoint_unusable()` when a
///    block's declared size disagrees with the frontier it derives, and the whole
///    scan below runs through it, so a green scan IS that checker's verdict.
/// 3. **UPSTREAM'S** — `ScanError::TreeSizeMismatch` in the scanner and
///    `PutBlocksError::NonSequentialBlocks` in `put_blocks_rows` (the anchor half).
///    Both are inside `scan_cached_blocks`/`put_blocks`, so again a green scan is
///    the verdict — and the row then reads upstream's OWN
///    `blocks.ironwood_commitment_tree_size` column back to prove the declaration
///    was stored rather than merely tolerated.
///
/// **And it reaches all three pools, not only Ironwood**: the same hex is a valid
/// Sapling frontier and a valid Orchard-shaped one (the all-zero node encoding is
/// canonical in both fields), which clause 1 asserts per pool.
///
/// The anti-vacuity clause is the last one: the count oracle's arithmetic is
/// `leaves / SUBTREE_LEAVES`, so a chain declaring `SUBTREE_LEAVES - 1` must leave
/// it at zero. That is what makes B1-9's CONTROL a real discriminator rather than
/// a fixture that happens to be accepted.
#[tokio::test]
async fn the_declared_tree_size_knob_is_accepted_by_both_checkers() {
    // (1) The frontier bytes decode, per pool, to exactly the size claimed.
    for leaves in [0u32, 1, 2, 3, SUBTREE_LEAVES - 1, SUBTREE_LEAVES] {
        for (pool, shape) in [
            (
                "sapling",
                ChainShape::declaring(DeclaredTreeSizes::sapling(leaves)),
            ),
            (
                "ironwood",
                ChainShape::declaring(DeclaredTreeSizes::ironwood(leaves)),
            ),
        ] {
            let state = shape
                .tree_state(OVER_SCAN_BASE, false)
                .to_chain_state()
                .unwrap_or_else(|e| panic!("{pool} frontier for {leaves} leaves decodes: {e}"));
            let decoded = match pool {
                "sapling" => state.final_sapling_tree().tree_size(),
                _ => state.final_ironwood_tree().tree_size(),
            };
            assert_eq!(
                decoded,
                u64::from(leaves),
                "{pool}: the hand-built frontier hex must decode to exactly {leaves} leaves \
                 through upstream's own reader"
            );
        }
    }

    // (2)+(3) A real scan over a chain declaring a non-zero Ironwood tree.
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let shape = ChainShape::declaring(DeclaredTreeSizes::ironwood(SUBTREE_LEAVES - 1));
    let (w, key) = scanned_wallet_shaped(dir.path(), &vault, OVER_SCAN_BASE, &shape).await;
    let obs = observe(dir.path(), &key);

    let mut chain = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        OVER_SCAN_TIP,
    )
    .shaped(shape);
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect(
            "the declared-size chain scans: neither our derived-frontier check nor upstream's \
             TreeSizeMismatch / NonSequentialBlocks refuses it",
        );
    assert_eq!(
        scanned_span(&obs),
        Some((OVER_SCAN_BASE + 1, OVER_SCAN_TIP)),
        "precondition: the whole window really was scanned"
    );

    let stored: i64 = obs
        .query_row(
            "SELECT ironwood_commitment_tree_size FROM blocks WHERE height = ?",
            [OVER_SCAN_TIP as i64],
            |r| r.get(0),
        )
        .expect("the scanned tip carries a declared Ironwood tree size");
    assert_eq!(
        stored,
        i64::from(SUBTREE_LEAVES - 1),
        "upstream stored the declaration this fixture made — the knob is live, not tolerated"
    );
    // ANTI-VACUITY: one leaf short of a subtree is ZERO complete subtrees, which is
    // the count oracle's own arithmetic and the reason B1-9's control discriminates.
    assert_eq!(
        u64::from(SUBTREE_LEAVES - 1) / u64::from(SUBTREE_LEAVES),
        0,
        "SUBTREE_LEAVES - 1 leaves is no complete subtree"
    );
    w.close().await.expect("close");
}

/// **C6, the half that BINDS — B1-9, REPAIRED, because it was VACUOUS for
/// two sessions.** When the wallet HAS scanned the completing block, a root whose
/// `completing_block_hash` disagrees with `blocks.hash` at that height is refused
/// — and the refusal is the `CompletingBlockHash` one, asserted by the variant's
/// own code.
///
/// `SubtreeRoot.completing_block_hash` is served by real lightwalletd (it is in
/// this repo's own probe capture) and was parsed and DISCARDED — nothing in the
/// SDK read it and `zcash_client_sqlite` never reads it either. What it buys,
/// stated no larger than it is: it binds an endpoint to its own earlier statements
/// across RPCs and across time, and where `blocks.hash` was written under a
/// different server, it binds server B to server A. It cannot attribute the lie to
/// either side, so it refuses and never blames. It is NOT the first-contact answer.
///
/// # What was wrong with this row, measured (§4w (f))
///
/// It served SAPLING subtree 0 at `SCAN_BASE + 50` = 3,428,050 over a chain whose
/// every block declared `sapling_commitment_tree_size = 0`. Instrumented in the
/// foreground, the LIE pass and the CONTROL pass came back with the SAME verdict,
/// byte for byte — `BundledFrontier { index: 0, served: 3428050, at_height:
/// 560000, complete: 1 } code=bundled_frontier` — because mainnet Sapling subtree
/// 0 completed at **558,822** and the bundled row `(560_000, 1)` refutes a claim of
/// 3,428,050 offline. The C6 clause never ran on either arm. `is_typed_refusal`
/// passed on the count refusal, and the control's `expect` would have caught it
/// except that the control was refused too — no: the control was ACCEPTED, and the
/// lie was refused by a clause that is not this row's. **Same defect 's
/// adjudicator repaired in this row's sibling**
/// ([`the_hash_cross_check_abstains_explicitly_on_an_unscanned_block`], whose
/// repair comment says exactly this) and did not repair here. INC-015's shape.
///
/// The row's old doc claimed *"the mechanism that refused cannot be the ordering
/// clause, the activation clause, the ceiling clause, the cap, or an empty
/// stream"*. That sentence is false by OMISSION: it does not exclude the two COUNT
/// clauses, and a count clause is what fired. It is deleted rather than qualified.
///
/// # Why every earlier oracle now abstains or AGREES
///
/// `check_pool` consults its arms in one order and returns on the first refusal —
/// gap → recorded → bundled → scanned → completing hash (§4x P5). One root is
/// served, at [`IRONWOOD_COMPLETING`], on the Ironwood pool:
///
/// * **gap** — a single root has no consecutive pair, so there is nothing to
///   compare. ABSTAINS.
/// * **recorded** — the wallet has never had a root written for this pool. Its
///   `ironwood_tree_shards` row exists only because `put_blocks` inserted the
///   anchor frontier, so `subtree_end_height` is NULL and the check has no height
///   to hold the serve to. ABSTAINS (asserted below, as a precondition).
/// * **bundled** — the signed bundle pins Ironwood subtree 0's completion into
///   `(3_449_780, 3_452_280]` and [`IRONWOOD_COMPLETING`] is inside it, so every
///   bundled row AGREES the served height is admissible. (Not an abstention: this
///   is the oracle that used to refuse, now answering the question correctly.)
/// * **scanned** — B1-10's knob is what makes this one agree. The chain declares
///   [`SUBTREE_LEAVES`] Ironwood commitments at every block, so the wallet's own
///   newest scanned block says ONE subtree was complete at [`OVER_SCAN_TIP`], and
///   `check_against_count` admits index 0 at any height at or below it. With the
///   old declaration of ZERO it refused every height the wallet had scanned, at any
///   pool — which is why no served height whatever could reach the hash clause
///   through the old fixture (§4w (f4)).
///
/// So the hash clause is the only one left that can answer, and the two arms
/// differ in `completing_block_hash` and in nothing else — same index, same
/// height, same `root_hash`, same wallet, same pass shape.
///
/// # Why IRONWOOD, and how the variant is asserted
///
/// `sync::apply_height_bind` carries an IRONWOOD violation to the caller as
/// `Ok([.., HeightViolation { code }, ..])` and answers a Sapling/Orchard one with
/// `Err(endpoint_unusable())`, which DROPS the code (the module doc's T0-1a-R
/// note). So the only pool on which a caller-level assertion can name the arm is
/// Ironwood, and the row moved to it. [`refusal_code`] reads
/// `HeightBindRefusal::code()`, which is an exhaustive one-variant-one-string map
/// (`root_bind.rs`: a new variant fails to compile until it has a code), so
/// `Some("completing_block_hash")` names the `CompletingBlockHash` VARIANT and
/// cannot be satisfied by `ScannedTreeSize` or `BundledFrontier`. That is the
/// assertion §4x B1-8 requires and `is_typed_refusal` is not — and
/// `is_typed_refusal` is deliberately not called here.
///
/// The unit-level twin lives at
/// `root_bind::tests::the_hash_cross_check_abstains_on_an_unscanned_block_and_refuses_on_a_scanned_one`;
/// this row is the one that proves the clause is REACHABLE from the shipped pass.
#[tokio::test]
async fn a_completing_block_hash_that_contradicts_a_scanned_block_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    // B1-10: one whole subtree declared complete, on every block of the window.
    let shape = ChainShape::declaring(DeclaredTreeSizes::ironwood(SUBTREE_LEAVES));
    let (w, key) = scanned_wallet_shaped(dir.path(), &vault, OVER_SCAN_BASE, &shape).await;
    let obs = observe(dir.path(), &key);

    let mut chain = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        OVER_SCAN_TIP,
    )
    .shaped(shape);
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the fixture chain scans");

    let h = IRONWOOD_COMPLETING;
    assert_eq!(
        scanned_span(&obs),
        Some((OVER_SCAN_BASE + 1, OVER_SCAN_TIP)),
        "precondition: the scan window is pinned, because every count oracle's verdict \
         is a function of where it sits relative to the served height"
    );
    let stored: Vec<u8> = obs
        .query_row(
            "SELECT hash FROM blocks WHERE height = ?",
            [h as i64],
            |r| r.get(0),
        )
        .expect(
            "precondition: the completing block was actually SCANNED — without \
                 this row the cross-check has nothing to disagree with and the case \
                 would pass by abstention",
        );
    assert_eq!(
        stored,
        block_id_on(h, false),
        "precondition: and `blocks.hash` holds the honest chain's block id"
    );
    let scanned_leaves: i64 = obs
        .query_row(
            "SELECT ironwood_commitment_tree_size FROM blocks WHERE height = ?",
            [OVER_SCAN_TIP as i64],
            |r| r.get(0),
        )
        .expect("the scanned tip declares an Ironwood tree size");
    assert_eq!(
        scanned_leaves,
        i64::from(SUBTREE_LEAVES),
        "precondition (B1-10): the wallet's OWN newest scanned block says one subtree is \
         complete, so the scanned count oracle AGREES the served height is admissible \
         instead of refusing it before the hash clause runs"
    );
    assert_eq!(
        recorded(&obs, "ironwood")
            .into_iter()
            .filter(|(_, height)| height.is_some())
            .count(),
        0,
        "precondition: no subtree_end_height is recorded for this pool, so the recorded \
         oracle abstains"
    );

    // The lie: the right subtree, the right height, a completing block hash from
    // another chain.
    let mut liar = ScriptedEndpoint::new(
        Vec::new(),
        Vec::new(),
        vec![root_at_with_hash(h, IRONWOOD_TAG, block_id_on(h, true))],
    );
    let r = w
        .update_subtree_roots(&mut liar, ironwood_tip(), None)
        .await;
    assert_eq!(
        liar.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the root was streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("completing_block_hash"),
        "THE VARIANT: the refusal must be `HeightBindRefusal::CompletingBlockHash` and not \
         another clause wearing this row's name. `is_typed_refusal` is what let \
         `bundled_frontier` pass here for two sessions (§4w (f3)); a bare \"was it \
         refused\" is not an answer to this row's question. Got {}",
        describe(&r)
    );
    assert_eq!(
        recorded(&obs, "ironwood")
            .into_iter()
            .filter(|(_, height)| height.is_some())
            .count(),
        0,
        "and nothing is written for that pool"
    );

    // THE DISCRIMINATOR: the same root, with the hash the wallet scanned.
    let mut honest = ScriptedEndpoint::new(
        Vec::new(),
        Vec::new(),
        vec![root_at_with_hash(h, IRONWOOD_TAG, block_id_on(h, false))],
    );
    let control = w
        .update_subtree_roots(&mut honest, ironwood_tip(), None)
        .await;
    assert!(
        accepted(&control),
        "control: one byte-set different in completing_block_hash and the \
                 same root is accepted — so the refusal above is attributable to \
                 that field and to nothing else. Got {}",
        describe(&control)
    );
    assert_eq!(heights(&obs, "ironwood"), vec![Some(h as i64)]);
    w.close().await.expect("close");
}

/// **C6, the half that ABSTAINS.** `blocks` holds only blocks this wallet has
/// SCANNED, so on a wallet that has scanned nothing the cross-check has no
/// oracle. It must abstain — and the abstention must be visible as behaviour, not
/// merely absent from the code: a guard that silently abstains on the whole fleet
/// is the vacuity class this programme exists to stop.
///
/// **The abstention is asserted against its own opposite.** The root served here
/// is byte-identical to the one
/// `a_completing_block_hash_that_contradicts_a_scanned_block_is_refused` refuses —
/// same height, same `root_hash`, same wrong `completing_block_hash`. The ONLY
/// difference between the two tests is whether block `SCAN_BASE + 50` has been
/// scanned. One accepts, one refuses; that pair is what makes "it abstains" a
/// measurement rather than a hope, and it is why neither test can pass by the
/// check simply not existing.
#[tokio::test]
async fn the_hash_cross_check_abstains_explicitly_on_an_unscanned_block() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    assert_eq!(
        count_rows(&obs, "blocks"),
        0,
        "precondition: NOTHING is scanned, which is the common case — a wallet \
         mid-heal has scanned no block anywhere near a completed subtree"
    );

    // **ADJUDICATOR REPAIR (T0-1a).** This was `SCAN_BASE + 50` = 3,428,050,
    // i.e. an endpoint claiming mainnet SAPLING subtree 0 completed in 2024. The
    // bundled frontiers refute that outright (the real height is 558,822) and the
    // join refused this row for the COUNT reason, not the hash reason — so the case
    // named for the abstention could not see the abstention. The measured height
    // makes the count checks pass and leaves the hash check as the only thing that
    // could refuse, which is what the row is for.
    let h = SAPLING_TRUTHFUL[0];
    let mut src = ScriptedEndpoint::new(
        vec![root_at_with_hash(h, 1, block_id_on(h, true))],
        Vec::new(),
        Vec::new(),
    );
    let r = w.update_subtree_roots(&mut src, tip(), None).await;
    assert!(
        r.is_ok(),
        "THE ABSTENTION: with no scanned block at that height there is nothing to \
         contradict, and a cross-check that fails closed here refuses every honest \
         endpoint on every fresh wallet. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "sapling"),
        vec![Some(h as i64)],
        "and the root is ingested — the abstention lets the write through rather \
         than dropping it silently"
    );
    w.close().await.expect("close");
}

/// **C7 — the reorg case, which is where the remedy turns into the bug.** An
/// honest endpoint, then a reorg rewind through `db.truncate_to_height`
/// (`sync.rs:1415`), then the SAME honest endpoint re-serving — accepted. If the
/// chosen mechanism cannot satisfy this, that is a finding about the mechanism,
/// not a test to relax.
///
/// **The rewind is asserted, not assumed:** `blocks` loses rows, and that only
/// happens because `scan_batch`'s continuity arm ran `truncate_to_height`.
///
/// The reorg pass deliberately serves NO subtree roots. An empty pool is a
/// legitimate state (this crate's own
/// `update_subtree_roots_empty_first_sync_then_real_converges` pins it), so the
/// drive reaches the rewind whether a bind is present or absent — the reorg
/// cannot be suppressed by the very thing under test refusing that pass.
///
/// **THIS ROW IS GREEN AND IT IS NOT MEASURING WHAT ITS NAME SAYS. Measured at
/// (T0-1a-R), printed from this exact drive:** the post-reorg re-serve returns
/// `Ok([Served { roots: 0 }, Served { roots: 0 }, HeightViolation { code:
/// "scanned_tree_size" }])`. The honest endpoint is **refused**, and the row passes
/// anyway, because both of its assertions are satisfied by a refusal: for Ironwood
/// a height violation IS `Ok` (§4e's precision note), and the record it compares
/// against already holds `TRUTHFUL` — which is exactly what the endpoint re-served.
/// So `is_ok` + `heights == TRUTHFUL` cannot tell "accepted" from "refused, nothing
/// written".
///
/// The cause is the fixture, not the bind: this wallet scans at [`CAP_BASE`], ABOVE
/// the served completing heights, on a chain double that declares
/// `ironwood_commitment_tree_size = 0`, so the wallet's own count contradicts every
/// served root. It is the same structural block the adjudicator recorded for the C6
/// binding half, reaching a second row.
///
/// **REPAIRED AT ADJUDICATION (T0-1a-R), and R10 is relaxed for it on the
/// record.** The paragraph above was written against `t0-1a-join`, where
/// `assert!(accepted(&r), ..)` really would have turned the row red — which is why
/// the test half left it. On the JOINED tree it does not: the rewind gate makes
/// oracle (d) abstain for exactly this pass, so the re-serve is genuinely accepted
/// (`accepted=true, code=None`, driven at adjudication). The assertion is therefore
/// free here, and R10's "one colour change" is not spent by it — this row passes
/// before the repair and after it. What changes is that it can now FAIL: with the
/// scanned leg of the rewind gate removed the pool is refused, and the row says so
/// instead of passing on `is_ok`.
///
/// The vacuity was real and is worth keeping in view: driven at adjudication, a
/// SECOND honest re-serve on the same wallet — with the relaxation already consumed
/// — comes back `is_ok=true, accepted=false, code=Some("scanned_tree_size")`. A row
/// asserting `is_ok` alone would have passed on that too.
#[tokio::test]
async fn an_honest_endpoint_is_accepted_after_a_reorg_rewind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, CAP_BASE).await;
    let obs = observe(dir.path(), &key);

    let mut chain = ScriptedChain::new(ScriptedEndpoint::ironwood_only(&TRUTHFUL), CAP_TIP);
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("pass 1: the honest endpoint scans and ingests");
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    let blocks_before = count_rows(&obs, "blocks");
    assert!(blocks_before > 0, "precondition: blocks were scanned");

    // The reorg.
    let mut reorg = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        CAP_TIP,
    )
    .reorging(CAP_TIP + 100);
    let _ = w
        .sync_once(&mut reorg, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;
    let blocks_after = count_rows(&obs, "blocks");
    assert!(
        blocks_after < blocks_before,
        "THE MECHANISM: a real reorg rewind ran `db.truncate_to_height` — scanned \
         blocks went {blocks_before} → {blocks_after}. If this does not drop, the \
         rest of this test proves nothing about reorgs"
    );

    // The same honest endpoint, afterwards.
    let mut again = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut again, tip(), None).await;
    assert!(
        accepted(&r),
        "after a reorg rewind the same honest endpoint must still be ACCEPTED — not \
         merely `Ok`. For Ironwood a height violation is `Ok([.., HeightViolation, \
         ..])`, and the record this row compares against already holds the sequence \
         being re-served, so `is_ok` plus `heights == TRUTHFUL` is satisfied by a \
         refusal as well as by an acceptance. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    w.close().await.expect("close");
}

/// **C15 — the rewind that crosses a shard boundary.** `completing_block_height`
/// is *"the height of the block that completed this subtree in the main chain"*
/// (`zcash_client_backend-0.24.0/src/proto/service.rs:260-262`) — chain-dependent.
/// After a rewind deep enough to re-cross a completion boundary, an honest
/// endpoint legitimately serves a DIFFERENT, LOWER height at that index. It must
/// be accepted.
///
/// Note `TreeTruncation::Unaffected` leaves the shard rows in place
/// (`zcash_client_sqlite-0.22.0/src/wallet.rs:4247-4249`, consumed as a literal
/// no-op at `:4413`, `:4461`, `:4511`), so the wallet's record and the endpoint's
/// answer can honestly disagree.
///
/// **This is the sharpest row in the file, and it is deliberately paired.** The
/// sequence served here — index 3 moved DOWN from 3,475,149 to 3,470,000 — is
/// byte-for-byte the sequence
/// `a_poisoned_bind_is_cleared_by_the_operation_the_contract_names` requires to
/// be REFUSED. The only difference between the two is that a reorg rewind
/// happened first. A mechanism that cannot tell those two apart fails one of
/// them, and which one it fails says what it is: refuse both and an honest
/// server is permanently rejected after every reorg; accept both and the bind
/// does not bind. The contract says a mechanism that cannot satisfy C7/C15 is a
/// finding about the mechanism.
#[tokio::test]
async fn an_honest_endpoint_is_accepted_after_a_rewind_that_recrosses_a_shard_boundary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, CAP_BASE).await;
    let obs = observe(dir.path(), &key);

    let mut chain = ScriptedChain::new(ScriptedEndpoint::ironwood_only(&TRUTHFUL), CAP_TIP);
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("pass 1");
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    let blocks_before = count_rows(&obs, "blocks");

    let mut reorg = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        CAP_TIP,
    )
    .reorging(CAP_TIP + 100);
    let _ = w
        .sync_once(&mut reorg, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;
    let blocks_after = count_rows(&obs, "blocks");
    assert!(
        blocks_after < blocks_before,
        "THE MECHANISM: the rewind ran ({blocks_before} → {blocks_after} scanned blocks)"
    );

    // On the re-organised chain the last subtree completes EARLIER.
    let mut honest = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        r.is_ok(),
        "an honest endpoint's LOWER completing height at a recorded index, after a \
         rewind that re-crossed the boundary, must be accepted. Refusing it is a \
         wallet permanently stuck on an honest server. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&RECROSSED),
        "and the corrected height is what the wallet now holds"
    );
    w.close().await.expect("close");
}

/// **C14 — the cap survives.** The attack the design review found in the SHIPPED
/// tree: inflate the heights, induce a rewind, and
/// `truncate_tree_to_subtree_roots` (`commitment_tree.rs:730-793`) keeps only
/// shards with `subtree_end_height <= :truncation_height` — none — and then
/// executes `DELETE FROM {prefix}_tree_checkpoints; DELETE FROM
/// {prefix}_tree_shards; DELETE FROM {prefix}_tree_cap;`.
///
/// **The cap is what binds the root hashes.** `put_shard_roots` inserts each root
/// into it with `Retention::Reference` (`commitment_tree.rs:1272-1281`), which is
/// non-prunable, so a differing leaf at the same address is a shardtree
/// `Conflict`. After the erasure the endpoint may serve arbitrary root hashes at
/// any index with nothing left to conflict against — and this tree has a passing
/// named test for the binding that erasure removes
/// (`mutated_root_reput_is_insert_conflict_not_silent_overwrite`, `sync.rs:2342`).
///
/// Two assertions, in the order the chain breaks: the cap is not emptied, and the
/// control it provides still works.
///
/// **THE TWO HALVES ARE NOT THE SAME KIND OF ASSERTION, and §4d owed row 7 says
/// so — relabelled here (T0-1a-R), and the row is kept, not deleted.**
///
/// * *"a differing root at a recorded index is still refused"* is a **reproduction
///   guard**: it exercises a control this repository can break, and the "do not
///   re-implement a root-hash comparison in SDK glue" clause (§4c, Explicitly
///   forbidden) rests on it being live. That half earns its place, which is the
///   adjudicator's Q1 ruling.
/// * *"`ironwood_tree_cap` is not emptied"* is an **UPSTREAM PIN**, not a
///   reproduction. The deletion it watches for lives entirely in
///   `zcash_client_sqlite`'s `truncate_tree_to_subtree_roots`
///   (`commitment_tree.rs:786-793`); **no mutation of any code this workspace owns
///   can reach it**, because nothing here decides whether `ResetToSubtreeRoots`
///   fires or which shards survive a truncation height. It therefore cannot appear
///   in `evals/mutants.tsv` with a killing mutation, and asking it for one is a
///   category error rather than an owed row. What it does buy is real and is why
///   it stays: `zcash_client_sqlite` is pinned `"=0.22.0"` exactly, and this
///   assertion is what turns a future bump that changes the truncation's survivor
///   rule into a named red instead of a silent loss of the root-hash bind.
///
/// The measurement below records that the erasure did not fire at all on this
/// drive, which is a third thing again — a finding about REACHABILITY, kept
/// because a premise corrected without its evidence gets re-asserted.
///
/// **Measured at HEAD (6f918c1a), and it is not what the design review predicted
/// — recorded here rather than left for the next reader to rediscover.** Driving
/// this through a real scan, the inflation IS accepted (`shards=[3475496,
/// 3475497, 3475498, 3475499]`), the reorg IS real (`reorgs: 1`, scanned blocks
/// 99 → 90, so the truncation height is 3,475,490 — BELOW every shard height, the
/// empty-survivor condition), and yet `ironwood_tree_cap`, `ironwood_tree_shards`
/// and `ironwood_tree_checkpoints` all survive untouched (`cap=1`,
/// `ckpt=2` before and after). `ResetToSubtreeRoots` did not fire. The
/// review's premise for it is *"a pool whose checkpoints all sit above the
/// truncation height"*, and this wallet has two Ironwood checkpoints written by
/// the scan that the reorg itself required — so the drive that reaches the rewind
/// is also the drive that fills in the condition the erasure needs to be absent.
/// **This is a finding, not a refutation:** the erasure may still be reachable on
/// a wallet with no Ironwood checkpoints at all (which is the state §3a
/// describes), and this drive cannot construct that state, because inducing a
/// reorg requires scanning. The row stays as a REGRESSION GUARD either way — it
/// pins that the cap survives a rewind and that the cap's control still fires.
#[tokio::test]
async fn an_induced_rewind_after_inflated_heights_does_not_empty_the_tree_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, CAP_BASE).await;
    let obs = observe(dir.path(), &key);

    // Pass 1: honest, at the truthful heights, over a real scanned chain.
    let mut chain = ScriptedChain::new(ScriptedEndpoint::ironwood_only(&TRUTHFUL), CAP_TIP);
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("pass 1: the honest endpoint scans and ingests");
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    let cap_before = count_rows(&obs, "ironwood_tree_cap");
    assert!(
        cap_before > 0,
        "precondition: put_shard_roots wrote the cap (Retention::Reference), which \
         is the structure that binds root hashes"
    );
    let blocks_before = count_rows(&obs, "blocks");

    // Pass 2: the inflation. Its outcome is not this row's question — C12 owns
    // that — but at HEAD it lands and it is what arms the erasure.
    let mut inflate = ScriptedEndpoint::ironwood_only(&INFLATED);
    let _ = w.update_subtree_roots(&mut inflate, tip(), None).await;
    assert_eq!(
        inflate.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the inflated sequence was really offered"
    );

    // Pass 3: the rewind. No roots served, so this pass reaches the rewind
    // whether or not a bind is present.
    let mut reorg = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        CAP_TIP,
    )
    .reorging(CAP_TIP + 100);
    let _ = w
        .sync_once(&mut reorg, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;
    let blocks_after = count_rows(&obs, "blocks");
    assert!(
        blocks_after < blocks_before,
        "THE MECHANISM: the rewind ran ({blocks_before} → {blocks_after} scanned blocks)"
    );

    assert!(
        count_rows(&obs, "ironwood_tree_cap") > 0,
        "THE ERASURE: `ironwood_tree_cap` was emptied. Every shard height sat above \
         the truncation height, so `truncate_tree_to_subtree_roots` found no \
         survivor and dropped the cap — and with it the only control that stops an \
         endpoint serving arbitrary root hashes. State at this point: shards={:?}, \
         checkpoints={}, scanned blocks={}",
        recorded(&obs, "ironwood"),
        count_rows(&obs, "ironwood_tree_checkpoints"),
        count_rows(&obs, "blocks")
    );

    let idx0 = recorded(&obs, "ironwood").first().copied();
    let Some((_, Some(recorded_h))) = idx0 else {
        panic!(
            "THE ERASURE: index 0's shard row is gone too ({:?}) — shards, \
             checkpoints and the cap were dropped together",
            idx0
        );
    };

    // The control the cap provides: a DIFFERENT root hash at a recorded index.
    // Not a height violation — the height is exactly what is recorded — so the
    // only thing that can refuse this is the shardtree `Conflict` path, which is
    // upstream's and which the cap is what enables.
    let mut mutated = ScriptedEndpoint::new(
        Vec::new(),
        Vec::new(),
        vec![root_at_with_hash(
            recorded_h as u64,
            9,
            block_id_on(recorded_h as u64, false),
        )],
    );
    let r = w.update_subtree_roots(&mut mutated, tip(), None).await;
    assert!(
        is_typed_refusal(&r),
        "with the cap intact, a DIFFERING root at a recorded index is still an \
         insert conflict, not a silent overwrite. Got {}",
        describe(&r)
    );
    w.close().await.expect("close");
}

/// **C8 — the rescan case.** A rescan rebuild preserves our aux tables
/// (`db::AUX_TABLES_PRESERVED`, ADR-0534) and REPLACES the wallet file, so the
/// shard rows go and anything of ours that remembers them does not. An honest
/// endpoint's roots after a rescan must be accepted.
#[tokio::test]
async fn an_honest_endpoint_is_accepted_after_a_rescan_rebuild() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;

    {
        let obs = observe(dir.path(), &key);
        let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
        w.update_subtree_roots(&mut honest, tip(), None)
            .await
            .expect("pass 1");
        assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    }

    let w = w
        .rescan_from_with_vault(None, Arc::clone(&vault))
        .await
        .expect("a full-history rescan rebuild");

    let obs = observe(dir.path(), &key);
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "precondition: the rebuild replaced the wallet file, so upstream's shard \
         rows are gone — this is the state that can leave a preserved aux record stale"
    );

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        r.is_ok(),
        "an honest endpoint after a rescan must be accepted. A bind whose memory \
         rode the rebuild while the thing it guards did not is a wallet that \
         refuses the truth. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    w.close().await.expect("close");
}

/// **C17 — the bind is clearable.** Question 4 of the contract: *how is a poisoned
/// or stale bind CLEARED, by whom, and is that operation reachable from a shipped
/// build?* Every other C-row is about refusing correctly; this one is about a
/// wrong refusal having an exit that is not reinstall-and-restore.
///
/// **The operation driven here is `rescan_from`** — the only clearing operation
/// reachable from a shipped build that touches this state at all. If the
/// implementer's answer to question 4 is a different operation, this test needs
/// re-adjudication against that answer rather than relaxing; if the answer is
/// "nothing in the product clears it", the contract says the item does not ship
/// on it and this test is where that shows.
///
/// The poison is deliberately narrow: only index 3 moves DOWN, to a height still
/// above index 2 and still above the newest bundled treestate row, so neither the
/// ordering clause, nor the activation clause, nor the ceiling clause, nor a
/// bundled-frontier bound can be what refuses it. What refuses it is the wallet's
/// own record of index 3 — and that is precisely the thing that has to be
/// clearable.
#[tokio::test]
async fn a_poisoned_bind_is_cleared_by_the_operation_the_contract_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let lowered = RECROSSED;

    {
        let obs = observe(dir.path(), &key);
        let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
        w.update_subtree_roots(&mut honest, tip(), None)
            .await
            .expect("pass 1 records index 3 at 3,475,149");
        assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));

        let mut contradicting = ScriptedEndpoint::ironwood_only(&lowered);
        let r = w
            .update_subtree_roots(&mut contradicting, tip(), None)
            .await;
        assert_eq!(
            contradicting.streamed(ShieldedProtocol::Ironwood),
            4,
            "IT-10: the lowered sequence was streamed"
        );
        assert!(
            is_typed_refusal(&r),
            "PRECONDITION FOR THIS ROW: while the record stands, a lower height at a \
             recorded index is refused. If it is not, there is nothing here to clear \
             and this test cannot see the clearing operation work. Got {}",
            describe(&r)
        );
    }

    // The clearing operation, driven end to end.
    let w = w
        .rescan_from_with_vault(None, Arc::clone(&vault))
        .await
        .expect("rescan rebuild");

    let obs = observe(dir.path(), &key);
    let mut after = ScriptedEndpoint::ironwood_only(&lowered);
    let r = w.update_subtree_roots(&mut after, tip(), None).await;
    assert!(
        r.is_ok(),
        "AFTER THE CLEARING OPERATION the previously-refused sequence is accepted. \
         If it is still refused, nothing in the product clears this bind — not a \
         resync, not an endpoint change, not a rescan — and reinstall-and-restore \
         is the only exit, which is the scare operation this SDK's error taxonomy \
         works to avoid. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&lowered),
        "and the endpoint's answer is what the rebuilt wallet now holds"
    );
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1a-R — the repair contract's rows. Names are this author's: §4e
// specifies each row's PROPERTY ("a named test that…") and names none of them,
// unlike §4c. Every one of them is an IRONWOOD row, and the module doc says why
// that is forced rather than chosen.
// ════════════════════════════════════════════════════════════════════════════

/// Drive a real scan over a window that sits entirely BELOW every completing
/// height these rows serve, and ASSERT that it does.
///
/// **This is the difference between a row that measures its own mechanism and a
/// row that measures the scanned arm by accident.** The fixture chain declares
/// `ironwood_commitment_tree_size = 0` on every block, so the wallet's own count
/// says no Ironwood subtree was complete as of the newest block it scanned. Point
/// that window ABOVE a served completing height and the scanned arm refuses the
/// pass — correctly, because the fixture chain really does contradict it, which is
/// the structural block the adjudicator recorded for the C6 binding half. Point it
/// BELOW and `served > at_height` holds at every index, so the arm abstains and a
/// reorg can be induced without the scan it requires also deciding the case.
async fn scan_below_the_fixtures(w: &Wallet, obs: &rusqlite::Connection) {
    let mut chain = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        UNDER_SCAN_TIP,
    );
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the low fixture chain scans");
    let (lo, hi) = scanned_span(obs).expect("precondition: blocks were actually scanned");
    assert!(
        hi < TRUTHFUL[0],
        "PRECONDITION FOR EVERY ROW BELOW THAT SCANS: the whole scan window \
         ({lo}..={hi}) must sit below every completing height these rows serve \
         (lowest is {}), or the SCANNED arm refuses the pass and the row measures \
         that arm instead of the one it names (IT-10)",
        TRUTHFUL[0]
    );
}

/// Induce a real reorg rewind, and ASSERT it happened rather than assume it — the
/// scanned block count has to drop, and the failure message carries both numbers.
///
/// The rewind is `sync.rs`'s own continuity arm — a block whose `prev_hash` the
/// wallet's stored block contradicts, answered by `db.truncate_to_height` — so the
/// evidence any repair reads is written by the product path, never by a test hook
/// (§4e R11). The pass serves NO roots, so it reaches the rewind whether a bind is
/// present or absent.
async fn induce_rewind(w: &Wallet, obs: &rusqlite::Connection) {
    let before = count_rows(obs, "blocks");
    let mut reorg = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        UNDER_SCAN_TIP,
    )
    .reorging(UNDER_SCAN_TIP + 100);
    let _ = w
        .sync_once(&mut reorg, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;
    let after = count_rows(obs, "blocks");
    assert!(
        after < before,
        "THE MECHANISM: a real reorg rewind must have run `db.truncate_to_height` — \
         scanned blocks went {before} → {after}. If this does not drop, everything \
         after it proves nothing about rewinds"
    );
}

// ── T0-1a-R2 drives ──────────────────────────────────────────────────

/// Scan an empty-endpoint chain up to `tip` and return the span the wallet holds
/// afterwards. The caller states, in its own words, what the window must sit
/// below or above — that is the row's precondition, not this helper's.
async fn scan_to(w: &Wallet, obs: &rusqlite::Connection, tip: u64) -> (u64, u64) {
    let mut chain = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        tip,
    );
    w.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the fixture chain scans");
    scanned_span(obs).expect("precondition: blocks were actually scanned")
}

/// [`induce_rewind`] from any scanned tip. A sibling rather than a parameter on the
/// original, so the four rows that stand on `induce_rewind` keep it byte-identical
/// under this split authorship; the shared lines are the price of that, paid on purpose.
async fn induce_rewind_at(w: &Wallet, obs: &rusqlite::Connection, scanned_tip: u64) {
    let before = count_rows(obs, "blocks");
    let mut reorg = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        scanned_tip,
    )
    .reorging(scanned_tip + 100);
    let _ = w
        .sync_once(&mut reorg, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;
    let after = count_rows(obs, "blocks");
    assert!(
        after < before,
        "THE MECHANISM: a real reorg rewind must have run `db.truncate_to_height` — \
         scanned blocks went {before} → {after}. If this does not drop, everything \
         after it proves nothing about rewinds"
    );
}

fn describe_pass(r: &Result<sync::SyncPass, WalletError>) -> String {
    match r {
        Ok(p) => format!(
            "Ok(batches={} reorgs={} cancelled={})",
            p.batches, p.reorgs, p.cancelled
        ),
        Err(e) => format!("Err({e:?} / code {})", e.code()),
    }
}

/// **§4g P1, ESTABLISHED: `db.truncate_to_height` returns `Err` through the
/// product path, from a hostile endpoint alone, with no local fault.**
///
/// The geometry is the one the contract could see and had not driven: a continuity
/// error DETECTED within [`crate::constants::REWIND_DISTANCE_BLOCKS`] of the wallet's own scan floor.
/// `sync::rewind_target` is `detection − REWIND_DISTANCE_BLOCKS`, so the target lands
/// BELOW the lowest block the wallet holds. Upstream's `select_truncation_height`
/// (`zcash_client_sqlite-0.22.0/src/wallet.rs:4092`) then finds no `blocks` row at or
/// below it and answers `RequestedRewindInvalid`; `WalletDb::truncate_to_height`
/// (`lib.rs:1877`, `transactionally`) propagates it unchanged. Measured: the wallet
/// scans [`P1_SCAN_BLOCKS`] blocks, the fork is served from the next height, and
/// `sync_once` returns with `blocks` untouched.
///
/// **The classification this helper used to report as a FINDING is now FIXED, and
/// this line moved with it exactly as its own doc said it would.**
/// `RequestedRewindInvalid` had no arm in
/// `sync::ClassifyStoreFault for SqliteClientError` and fell to `_ => StoreCorrupt`
/// — the seed-restore scare — on a wallet that is intact, in a state one hostile
/// endpoint can put it in on every pass. The security and crypto passes found
/// the same hole on SCAN-2's new undo path and it was closed for BOTH callers in
/// `sync::truncate_at_or_below`: the refusal is now the ENDPOINT's
/// (`Sync { EndpointMisbehaving }`), never the store's. So the pass here returns
/// **`Err(Sync { EndpointMisbehaving })`**.
///
/// The `matches!` below still pins it only as EVIDENCE that the pass failed inside
/// the continuity arm rather than in the download — the rows' claims are about the
/// ledger and are unchanged. The discriminator survives the reclassification: a
/// malformed block is `Sync { EndpointUnreachable }` and a network-half failure is
/// a different stall, so `EndpointMisbehaving` here still comes from nothing but
/// the truncate's refusal.
///
/// The geometry is asserted before the drive, so the helper can never quietly
/// measure a rewind that succeeded.
async fn induce_a_rewind_the_wallet_cannot_perform(w: &Wallet, obs: &rusqlite::Connection) {
    let (lo, hi) = scanned_span(obs).expect("precondition: blocks were scanned");
    let detection = hi + 1;
    let target = detection.saturating_sub(u64::from(crate::constants::REWIND_DISTANCE_BLOCKS));
    assert!(
        target < lo,
        "THE GEOMETRY (§4g P1): the fork is served from {detection}, so the continuity \
         error fires there and `rewind_target` is {target}; the wallet's lowest scanned \
         block is {lo}. The target must sit BELOW it, or upstream finds a checkpointed \
         block to truncate to, the rewind SUCCEEDS, and everything after this helper \
         measures a real rewind instead of a failed one"
    );
    let before = count_rows(obs, "blocks");
    let mut reorg = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        hi,
    )
    .reorging(hi + 100);
    let r = w
        .sync_once(&mut reorg, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;
    assert!(
        matches!(
            &r,
            Err(WalletError::Sync {
                stall: crate::state::StallReason::EndpointMisbehaving
            })
        ),
        "THE MECHANISM: the pass must fail INSIDE `scan_batch`'s continuity arm, on \
         `db.truncate_to_height`'s refusal — upstream `RequestedRewindInvalid`, which \
         since S267 is the ENDPOINT's fault and not the store's (the helper's doc says \
         why, and why this line moved). It is only the evidence that the TRUNCATE is \
         what failed, not the download. Got {}",
        describe_pass(&r)
    );
    assert_eq!(
        count_rows(obs, "blocks"),
        before,
        "and NO rewind was performed: the truncate that returned `Err` deleted nothing. \
         This is the state §4g 0a is about — every assertion after it asks what the \
         ledger says about a rewind that did not happen"
    );
    assert_eq!(
        scanned_span(obs),
        Some((lo, hi)),
        "the scan window is exactly what it was before the pass"
    );
}

/// **The 0b drive shared by R14 and R15** (§4g question Q-R4): one pass that WRITES
/// Sapling and then FAILS on Ironwood's put, on a wallet with one observed rewind.
/// Every step asserts its own precondition, so the two rows that diverge after it
/// cannot pass on a drive that silently did something else.
///
/// 1. Scan [`SAPLING_ERA_SCAN_BASE`]`..=`[`SAPLING_ERA_SCAN_TIP`] — below every height
///    served here, so the scanned arm passes by arithmetic on every pass (the fixture
///    doc says why the window is not [`UNDER_SCAN_BASE`]).
/// 2. An honest pass records [`SAPLING_TRUTHFUL`] and [`TRUTHFUL`].
/// 3. A real rewind, through the product path; both records survive it (asserted —
///    a record the rewind deleted would let the rows pass by abstention).
/// 4. Pass 2: Sapling served [`SAPLING_LOWERED`] — a downward move at a recorded
///    index, admitted under the relaxation and WRITTEN — and Ironwood served its
///    recorded heights with every `root_hash` mutated ([`IRONWOOD_MUTATED_TAG`]),
///    which the bind accepts (equal heights, no violation) and the shardtree refuses
///    at put time (§4g P2). The pass returns `Err(Sync { .. })` from
///    `put_subtree_roots`, AFTER Sapling's put committed — that is the window 0b is
///    about, and it is not C16's (a fault before anything is written).
///
/// What the drive leaves behind is exactly Q-R4's question: Sapling's record moved
/// once, Ironwood's did not, and what the ledger now says about each is what R14
/// and R15 measure.
async fn write_sapling_then_fail_on_ironwoods_put(w: &Wallet, obs: &rusqlite::Connection) {
    let (lo, hi) = scan_to(w, obs, SAPLING_ERA_SCAN_TIP).await;
    assert!(
        hi < SAPLING_LOWERED_AGAIN[0] && hi < TRUTHFUL[0],
        "PRECONDITION: the scan window ({lo}..={hi}) sits below every completing height \
         these rows serve (lowest Sapling {}, lowest Ironwood {}), so the scanned arm \
         cannot be what refuses a move and cannot be what refuses a control (IT-10)",
        SAPLING_LOWERED_AGAIN[0],
        TRUTHFUL[0]
    );

    let mut honest = ScriptedEndpoint::new(
        pool_script(&SAPLING_TRUTHFUL, SAPLING_TAG),
        Vec::new(),
        pool_script(&TRUTHFUL, IRONWOOD_TAG),
    );
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        accepted(&r),
        "pass 1: the honest endpoint records both pools, got {}",
        describe(&r)
    );
    assert_eq!(heights(obs, "sapling"), expect_heights(&SAPLING_TRUTHFUL));
    assert_eq!(heights(obs, "ironwood"), expect_heights(&TRUTHFUL));

    induce_rewind_at(w, obs, SAPLING_ERA_SCAN_TIP).await;
    assert_eq!(
        heights(obs, "sapling"),
        expect_heights(&SAPLING_TRUTHFUL),
        "the Sapling shard row survived the rewind, so there is a record for pass 2 \
         to move — and for pass 3 to be held to"
    );
    assert_eq!(
        heights(obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "and Ironwood's survived too, so there is a recorded leaf for pass 2's mutated \
         root to conflict with"
    );

    let mut pass2 = ScriptedEndpoint::new(
        pool_script(&SAPLING_LOWERED, SAPLING_TAG),
        Vec::new(),
        pool_script(&TRUTHFUL, IRONWOOD_MUTATED_TAG),
    );
    let r = w.update_subtree_roots(&mut pass2, tip(), None).await;
    assert_eq!(
        pass2.streamed(ShieldedProtocol::Sapling),
        1,
        "IT-10: pass 2 streamed the lowered Sapling root"
    );
    assert_eq!(
        pass2.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: pass 2 streamed the four mutated Ironwood roots"
    );
    assert!(
        matches!(&r, Err(WalletError::Sync { .. })),
        "pass 2 must fail at PUT time on the later pool — the shardtree `Conflict` §4g \
         P2 names, surfaced by `map_shardtree_err` as `Err(Sync {{ .. }})`. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(obs, "sapling"),
        expect_heights(&SAPLING_LOWERED),
        "PASS 2's SAPLING WRITE LANDED: the record moved once, {} → {}. Without this \
         the rows after it cannot tell 'consumed per pool' from 'nothing was written' — \
         and it is also the evidence that the pass failed AFTER Sapling's put, on a \
         later pool's put, and not on the bind (where nothing would have been written)",
        SAPLING_TRUTHFUL[0],
        SAPLING_LOWERED[0]
    );
    assert_eq!(
        heights(obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "and Ironwood's record still reads what pass 1 wrote (pass 2 served the same \
         heights, so this says only that nothing else reached the tree)"
    );
}

/// **R3 — no unconditional window** (§4e R3; §4d owed row 1; question Q-R1).
///
/// *By what evidence does the wallet know a rewind has happened since it recorded
/// a given shard height?* Measured on `t0-1a-join`: **none is required.** The
/// recorded-height check grants `±REORG_MAX_BLOCKS` on every pass, forever, with no
/// rewind anywhere in the drive — which turns *equality-when-recorded* into
/// *drift-when-recorded*, and a window an endpoint may walk is not a bind.
///
/// **The drive.** An honest pass records the truthful sequence. Then the endpoint
/// re-serves it [`WALK_PASSES`] times, each pass moving index 3 down by
/// [`WALK_STEP`] — one block LESS than the window, so the pre-repair code accepts
/// every step and the record follows it down. Total drift: 792 blocks, ~8× the
/// window that is supposed to bound it, produced by an endpoint that only ever asks
/// for less than the wallet already grants.
///
/// **The FIRST pass is the assertion**, because that is where the property lives:
/// a wallet that has observed no rewind has no reason to accept any move at all.
/// The remaining seven exist so the failure message shows the walk rather than a
/// single refused step, and so a repair that merely tightens the constant (99 → 50)
/// is still red here rather than quietly halved.
///
/// **Nothing else can be the refuser.** No block was ever scanned, so the scanned
/// arm and the completing-hash arm abstain through their own explicit branches; the
/// walked sequence stays strictly increasing and its consecutive gaps are thousands
/// of blocks, so the floor cannot reach it; index 3 stays above the newest bundled
/// Ironwood row throughout, so no frontier reaches it either. The wallet's own
/// record of index 3 is the only thing in the build that can answer.
///
/// **Predicted killing mutation:** restore the tolerance to unconditional —
/// `served.abs_diff(rec) > REORG_MAX_BLOCKS` with no rewind evidence consulted.
/// The first step is then 99 ≤ 100 and is accepted, and both the arm assertion and
/// the record assertion go red on pass 1.
#[tokio::test]
async fn a_recorded_height_cannot_be_walked_down_without_a_rewind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    assert_eq!(
        count_rows(&obs, "blocks"),
        0,
        "PRECONDITION: no block has ever been scanned, so no rewind can have been \
         observed by any mechanism — which is the whole question this row asks"
    );

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("pass 1: the honest sequence is recorded");
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));

    for pass in 1..=WALK_PASSES {
        let walked: [u64; 4] = [
            TRUTHFUL[0],
            TRUTHFUL[1],
            TRUTHFUL[2],
            TRUTHFUL[3] - WALK_STEP * pass,
        ];
        assert!(
            walked[3] > TRUTHFUL[2] + 1,
            "FIXTURE: the walk must stay strictly above index 2 with room to spare, \
             or the ordering clause or the block-size floor becomes the refuser and \
             this row stops measuring the recorded bind. index2={} index3={}",
            TRUTHFUL[2],
            walked[3]
        );
        let mut drifting = ScriptedEndpoint::ironwood_only(&walked);
        let r = w.update_subtree_roots(&mut drifting, tip(), None).await;
        assert_eq!(
            drifting.streamed(ShieldedProtocol::Ironwood),
            4,
            "pass {pass}: IT-10 — the endpoint really streamed four roots"
        );
        if pass == 1 {
            assert_eq!(
                refusal_code(&r),
                Some("recorded_height"),
                "THE FIRST STEP MUST BE REFUSED, and by the recorded bind. This \
                 endpoint moved a height the wallet already recorded by {WALK_STEP} \
                 blocks and no rewind has been observed — there is no honest reason \
                 for a completed subtree's height to move on a chain that never \
                 reorganised. A window granted unconditionally is not a bind; it is \
                 a rate limit on the same attack. Got {}",
                describe(&r)
            );
        }
        assert_eq!(
            heights(&obs, "ironwood"),
            expect_heights(&TRUTHFUL),
            "pass {pass} of {WALK_PASSES}: the record must not have moved. Serving \
             index 3 at {} against a record of {} is a {WALK_STEP}-block move, and \
             {WALK_PASSES} of them walk it {} blocks — {}× the window a single pass \
             is granted",
            walked[3],
            TRUTHFUL[3],
            WALK_STEP * WALK_PASSES,
            WALK_PASSES
        );
    }

    // ANTI-VACUITY. A bind that refused everything after the first drift would
    // satisfy every assertion above. The unchanged honest sequence must still be
    // accepted afterwards, so the refusals discriminate rather than latch.
    let mut still_honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut still_honest, tip(), None).await;
    assert!(
        accepted(&r),
        "ANTI-VACUITY: after refusing the walk, the wallet must still accept the \
         sequence it recorded. A bind that latches shut on the first refusal passes \
         everything above and is a wallet that has stopped syncing. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    w.close().await.expect("close");
}

/// **R4 — the abstention is not permanent** (§4e R4).
///
/// One reorg must not switch the recorded bind off for the life of the wallet.
/// Whatever evidence the repair reads to know a rewind happened, that evidence has
/// to be spent: after the rewind is answered and the honest post-rewind heights are
/// re-recorded, a later pass with **no new rewind** is bound again — by the NEW
/// record, not the old one.
///
/// **The drive, and what each step is for.**
/// 1. Scan a window strictly below every served height, so the scanned arm abstains
///    (see [`scan_below_the_fixtures`]) and a reorg is reachable.
/// 2. Record [`TRUTHFUL`].
/// 3. Serve [`RECROSSED`] with no rewind yet ⇒ **refused by `recorded_height`**.
///    This is the precondition: without it, step 5 cannot tell "the rewind was
///    answered" from "nothing was ever bound".
/// 4. Induce a real rewind through the product path.
/// 5. Serve the byte-identical [`RECROSSED`] ⇒ **accepted**, and re-recorded. This
///    is the step that depends on the repair, and it is the same pairing C15/C17
///    already make: one sequence, two answers, decided by whether a rewind happened.
/// 6. Serve [`DEEP_LOWERED`] with **no new rewind** ⇒ **refused**, by the record
///    step 5 wrote. This is R4's own claim.
///
/// **Predicted killing mutation:** make the rewind evidence permanent — set it once
/// and never clear or re-derive it (e.g. a latch, or a highwater that is written but
/// never compared against the CURRENT record). Steps 1-5 are unaffected and step 6
/// goes green-to-red, because a second move is then just as licensed as the first.
/// A second mutation kills it from the other side: drop step 5's re-record (accept
/// the pass but write nothing), and step 6 is refused against the STALE record with
/// the wrong numbers, which the record assertion catches.
///
/// **Status on `t0-1a-join`: RED at step 5**, because the pre-repair check answers
/// by magnitude — 5,149 > 100 — and never asks whether a rewind happened. That is
/// the blocker being repaired, not a finding.
///
/// **AND STEP 5 IS C15's ASSERTION, ON A SCAN WINDOW THAT DOES NOT POISON IT —
/// which makes this row a diagnostic for C15 as well as a proof of R4.** C15 builds
/// its wallet at [`CAP_BASE`], a window ABOVE the served heights, so once the
/// recorded arm stops refusing, the SCANNED arm refuses instead: measured this
/// session, C15's post-reorg oracle is `(3,475,490, 0 complete)`, which contradicts
/// every index of a sequence whose highest member is 3,470,000. So a correct repair
/// of §4d owed row 1 can leave **C15 red with `scanned_tree_size`** while this row
/// goes green — same property, same rewind, different fixture window. If that is
/// what the join shows, the remaining gap is C15's scan window and not the bind;
/// the one-line remedy is to build C15's wallet at [`UNDER_SCAN_BASE`] /
/// [`UNDER_SCAN_TIP`] instead, which this row proves is a state where the honest
/// sequence is accepted, the rewind still fires, and the shard rows still survive
/// it. That edit is NOT made here: §4e R1 requires C15 to pass unedited, so
/// changing the fixture under it is the adjudicator's call to make, not this
/// author's to pre-empt.
#[tokio::test]
async fn a_rewind_rebinds_rather_than_disabling_the_recorded_bind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, UNDER_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);

    scan_below_the_fixtures(&w, &obs).await;

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("the honest sequence is recorded over a scanned chain");
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));

    // (3) The precondition: bound BEFORE any rewind.
    let mut early = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut early, tip(), None).await;
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "PRECONDITION: with the record standing and no rewind observed, the lowered \
         sequence is refused by the recorded bind. If it is not, step 5's acceptance \
         proves nothing — an unconditional accept looks identical. Got {}",
        describe(&r)
    );

    // (4) The rewind, through the product path.
    induce_rewind(&w, &obs).await;
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "the rewind left the Ironwood shard rows in place (`TreeTruncation::Unaffected` \
         is a literal no-op per pool), so there is still a record for step 5 to \
         contradict. If these rows had been deleted, step 5 would pass by abstention"
    );

    // (5) The same bytes, now legitimate.
    let mut after = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut after, tip(), None).await;
    assert!(
        accepted(&r),
        "AFTER A REAL REWIND the honest endpoint's lower completing height must be \
         accepted — this is C15's property, driven here because R4 cannot be stated \
         without it. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&RECROSSED),
        "and the corrected height is RE-RECORDED. An accept that writes nothing \
         leaves the wallet bound to a chain it is no longer on, and step 6 would then \
         be measuring the stale record instead of the new one"
    );

    // (6) R4's own claim: bound again, by the NEW record, with no new rewind.
    let mut again = ScriptedEndpoint::ironwood_only(&DEEP_LOWERED);
    let r = w.update_subtree_roots(&mut again, tip(), None).await;
    assert_eq!(
        again.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the second lowered sequence was streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "THE ABSTENTION IS NOT PERMANENT. One reorg licensed one correction; it does \
         not license the next one. Serving index 3 at {} against the {} this wallet \
         just re-recorded is a fresh {}-block move with no fresh rewind behind it, \
         and it must be refused by the recorded bind. If this is accepted, an \
         endpoint that can induce ONE reorg — or wait for one — owns every recorded \
         height for the life of the wallet. Got {}",
        DEEP_LOWERED[3],
        RECROSSED[3],
        RECROSSED[3] - DEEP_LOWERED[3],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&RECROSSED),
        "and nothing was written: the record is still what the rewind justified"
    );
    w.close().await.expect("close");
}

/// **R5 — the counting bind is killable: the BUNDLED oracle** (§4e R5; §4d owed
/// row 2).
///
/// The adjudicator's anti-vacuity finding was that oracles (c) and (d) — the
/// contract's own answer to question 2, and the only mechanism that reaches the
/// index dimension — **reddened none of the sixteen named tests**. Built, argued,
/// and nothing in the proof set would have noticed them dying. This row and its
/// sibling are the two guards that owed row 2 asks for.
///
/// **The fixture is one root, and every part of that is load-bearing.** A single
/// root has no consecutive pair, so the block-size floor has nothing to compare and
/// cannot fire. The wallet is fresh: no shard row, so the recorded arm abstains;
/// no scanned block, so the scanned arm and the completing-hash arm abstain, each
/// through its own explicit branch. The height is above the NU6.3 activation and
/// below the endpoint's reported tip, so no A11 clause reaches it. **The signed
/// binary is the only thing left in the build that can answer**, and it does:
/// measured this session, the mainnet Ironwood frontier at 3,452,280 decodes to one
/// complete subtree, so an endpoint claiming subtree 0 completed at 3,465,000 is
/// contradicted by a number that shipped inside the binary and needed no network at
/// all.
///
/// **The discriminator is the height and nothing else.** The control serves the
/// same one-root shape at the measured completion height, and is accepted.
///
/// **Predicted killing mutation:** delete the bundled loop from `check_pool`
/// (`for &(at_height, complete) in &ev.bundled { check_against_count(..) }`), or
/// make `bundled_counts` return an empty slice, or drop `bundled` from `gather`.
/// The refusal then disappears entirely — not changes arm, disappears — because no
/// other arm has any evidence to work from, and the row goes red on both the arm
/// assertion and the "nothing written" assertion.
#[tokio::test]
async fn a_bundled_frontier_violation_is_refused_by_the_bundled_arm() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    assert_eq!(
        count_rows(&obs, "blocks"),
        0,
        "PRECONDITION: nothing scanned, so the scanned arm and the completing-hash \
         arm have no oracle and must abstain"
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "PRECONDITION: nothing recorded, so the recorded arm must abstain"
    );
    assert_eq!(
        BUNDLED_ONLY_VIOLATION.len(),
        1,
        "PRECONDITION: ONE root, so the block-size floor has no consecutive pair to \
         compare and is structurally unable to be the refuser"
    );

    let mut hostile = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_VIOLATION);
    let r = w.update_subtree_roots(&mut hostile, tip(), None).await;
    assert_eq!(
        hostile.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the root was streamed, so an empty stream is not the refuser"
    );
    assert_eq!(
        refusal_code(&r),
        Some("bundled_frontier"),
        "THE SIGNED BINARY IS THE REFUSER. Subtree 0's completion is pinned by the \
         bundle into (3,449,780, 3,452,280]; an endpoint placing it at {} is refuted \
         offline, at first contact, by a wallet that has scanned nothing and \
         recorded nothing. Every other arm abstains here by construction, so a \
         missing or wrong code means the counting bind is not reached — which is \
         the mechanism §4d owed row 2 says no test could kill. Got {}",
        BUNDLED_ONLY_VIOLATION[0],
        describe(&r)
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "and nothing was written for the pool"
    );

    // THE DISCRIMINATOR: same shape, same wallet, same pass — only the height moves.
    let mut honest = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_CONTROL);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        accepted(&r),
        "control: the same one-root shape at the height the bundle admits is \
         ACCEPTED, so the refusal above is attributable to the completing height and \
         not to the fixture's shape. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&BUNDLED_ONLY_CONTROL),
        "control: and it is recorded"
    );
    w.close().await.expect("close");
}

/// **R6 — the counting bind is killable: the SCANNED oracle** (§4e R6; §4d owed
/// row 2).
///
/// [`a_bundled_frontier_violation_is_refused_by_the_bundled_arm`]'s sibling, for
/// the half of the counting bind that reaches ABOVE the newest bundled row:
/// `blocks.{prefix}_commitment_tree_size`, a number the wallet COUNTED while
/// scanning and that upstream validates against the batch's own anchor.
///
/// **Two wallets, and the ONLY difference between them is whether blocks were
/// scanned.** Both are served the byte-identical single root at the measured
/// completion height — the same fixture the bundled row above uses as its accepted
/// control, so the bundled arm demonstrably admits it. Wallet A has scanned a
/// window ABOVE that height on a fixture chain that declares zero note commitments,
/// so the wallet's own count contradicts the endpoint and the pass is refused.
/// Wallet B has scanned nothing, so the arm abstains and the same root is accepted.
///
/// That pairing is what makes this a measurement rather than a coincidence: the
/// height, the root hash, the pool, the tip and the endpoint script are identical
/// on both sides, and the recorded arm abstains on both (neither wallet has a shard
/// row). The scan is the whole difference.
///
/// **Why the bundled arm cannot be the refuser on wallet A:** 3,451,206 sits inside
/// the window the bundle pins for subtree 0, which is exactly why it is the control
/// for R5. **Why the completing-hash arm cannot:** wallet A's scanned span starts
/// at [`OVER_SCAN_BASE`] and the served completing height is below it — asserted,
/// not assumed — so the span filter in the hash read skips it and the arm abstains.
/// **Why the floor cannot:** one root, so there is no consecutive pair.
///
/// **Predicted killing mutation:** delete the scanned arm from `check_pool`
/// (`if let Some((at_height, complete)) = ev.scanned { .. }`), or make
/// `read_scanned_tree_size` return `Ok(None)`, or drop `scanned` from `gather`.
/// Wallet A then accepts, and the row goes red on the arm assertion and on the
/// "nothing written" assertion. Wallet B is unaffected by that mutation, which is
/// what keeps the pair honest.
#[tokio::test]
async fn a_scanned_tree_size_violation_is_refused_by_the_scanned_arm() {
    // ── Wallet A: it scanned, and its own count contradicts the endpoint ──────
    let dir_a = tempfile::tempdir().expect("tempdir");
    let vault_a = test_vault();
    let (wa, key_a) = scanned_wallet(dir_a.path(), &vault_a, OVER_SCAN_BASE).await;
    let obs_a = observe(dir_a.path(), &key_a);

    let mut chain = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        OVER_SCAN_TIP,
    );
    wa.sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("the fixture chain scans");
    let (lo, hi) = scanned_span(&obs_a).expect("precondition: blocks were scanned");
    assert!(
        lo > BUNDLED_ONLY_CONTROL[0],
        "PRECONDITION: the whole scan window ({lo}..={hi}) sits ABOVE the served \
         completing height ({}), which is what makes the wallet's own count \
         contradict the endpoint — and which also puts the height outside \
         `read_scanned_hashes`' span, so the completing-hash arm abstains",
        BUNDLED_ONLY_CONTROL[0]
    );
    assert_eq!(
        recorded(&obs_a, "ironwood").len(),
        0,
        "PRECONDITION: no shard row, so the recorded arm abstains on this wallet too"
    );

    let mut src = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_CONTROL);
    let r = wa.update_subtree_roots(&mut src, tip(), None).await;
    assert_eq!(
        src.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the root was streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("scanned_tree_size"),
        "THE WALLET'S OWN COUNT IS THE REFUSER. It scanned every block up to {hi} \
         and counted zero complete Ironwood subtrees there; an endpoint claiming \
         subtree 0 completed at {} — below a block the wallet itself counted — is \
         contradicted by a number no endpoint wrote. This is the arm that extends \
         the bind ABOVE the newest bundled row, and §4d owed row 2 records that \
         nothing in the proof set could kill it. Got {}",
        BUNDLED_ONLY_CONTROL[0],
        describe(&r)
    );
    assert_eq!(
        recorded(&obs_a, "ironwood").len(),
        0,
        "and nothing was written for the pool"
    );
    wa.close().await.expect("close a");

    // ── Wallet B: byte-identical root, nothing scanned, so the arm abstains ───
    let dir_b = tempfile::tempdir().expect("tempdir");
    let vault_b = test_vault();
    let (wb, key_b) = wallet_and_key(dir_b.path(), &vault_b, Network::Main).await;
    let obs_b = observe(dir_b.path(), &key_b);
    assert_eq!(
        count_rows(&obs_b, "blocks"),
        0,
        "PRECONDITION: wallet B differs from wallet A in exactly one respect"
    );

    let mut src = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_CONTROL);
    let r = wb.update_subtree_roots(&mut src, tip(), None).await;
    assert!(
        accepted(&r),
        "THE ABSTENTION: with nothing scanned there is no count to contradict, and a \
         scanned bind that failed closed here would refuse every honest endpoint on \
         every fresh wallet — the vacuity class inverted into a brick. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs_b, "ironwood"),
        expect_heights(&BUNDLED_ONLY_CONTROL),
        "and the root is ingested, so the refusal on wallet A is attributable to the \
         scan and to nothing else"
    );
    wb.close().await.expect("close b");
}

/// **THE PLANTED ROW (IT-1 +A) — the assertion floor does not name this case, and
/// what it answers is which of the two designs §4e left open was actually built.**
///
/// §4e's "what this contract does NOT decide" leaves the implementer a choice for
/// what the recorded check does once a rewind IS observed: *"abstain at that index
/// and let the counting oracles carry the pass, or a bounded window derived from
/// `blocks.{prefix}_commitment_tree_size` on both sides of the rewind"*. It calls
/// **either** sufficient. This row measures the difference, because on this pool
/// they are not close.
///
/// **The prediction, and it is checkable.** Ironwood's counting oracles do not
/// reach here. The bundled Ironwood table stops at 3,459,780 (measured: 821 mainnet
/// rows, 4 of them showing a non-zero Ironwood count), and any wallet whose scanned
/// blocks sit below the served heights gets an abstention from the scanned arm as
/// well — which is the ordinary state of the mid-heal wallet this whole item exists
/// for. So above 3,459,780 the recorded height is the ONLY oracle Ironwood has, and
/// *"let the counting oracles carry the pass"* resolves to *let nothing carry it*.
///
/// The drive induces a rewind of `REWIND_DISTANCE_BLOCKS` = 10 blocks and then asks
/// the endpoint's claim to be believed: index 3 moved **7,749 blocks**. A reorg
/// moves a completion boundary by (commitments gained or lost) / (commitments per
/// block); ten orphaned blocks cannot carry 7,749 blocks' worth. Measured slack if
/// the answer is "abstain": index 3 may be placed anywhere in (3,467,168,
/// 3,475,499] — **8,331 blocks** — by any endpoint, once, after any reorg.
///
/// **How to read a red here, and the first draft of this paragraph was wrong.** It
/// predicted the row would be GREEN on `t0-1a-join`; the run refutes that and the
/// prediction is corrected rather than quietly dropped. Measured on the join, the
/// row splits:
///
/// * the **substantive assertion** — the 7,749-block move is refused by
///   `recorded_height` — is **GREEN**, because the pre-repair check refuses it for
///   the right outcome by the wrong reasoning (magnitude, not evidence);
/// * the **anti-vacuity control** — the honest [`RECROSSED`] correction accepted
///   after the same rewind — is **RED**, and it is red for exactly the reason C15
///   is red. The pre-repair check cannot tell the two moves apart at all, so it
///   refuses both.
///
/// So on the join this row is red-because-the-repair-is-pending, like C15. It only
/// becomes the design discriminator it was planted to be once C15 goes green:
/// **at that point the control passes and the substantive assertion is the one to
/// read.** If the substantive assertion is then red while the control is green, the
/// abstaining design was chosen and the window §4e believed the counting oracles
/// would close is open by 8,331 blocks on the one pool the item is about. That is a
/// finding for the adjudicator to rule on, not a defect on its face — and it is
/// worth more as a named red than as a paragraph nobody runs.
///
/// **RULED AT RE-ADJUDICATION (T0-1a-R round 2): ACCEPTED RED, WITH AN
/// OWNER. This row is not a defect report against the direction floor and must not
/// be relaxed to pass** (§4e-R2 (d), option 3 of the three it names).
///
/// The paragraph above was written before the floor existed and it guessed wrong
/// about which design would be built, so the guess is corrected rather than
/// deleted. §4e-R2 (c) put the floor on DIRECTION, not on distance: after a rewind
/// a served height at or below the record is accepted and one above it is refused.
/// The abstaining design was NOT chosen. **The recorded arm does not abstain here —
/// it answers, and it accepts, because this move is downward.**
///
/// **Why not option 1 (satisfy it).** The only mechanism that refuses this 7,749
/// block drop and still accepts C15's honest 5,149 block drop is a magnitude
/// threshold somewhere between the two. That is the ±`REORG_MAX_BLOCKS` guess §4d's
/// Q4 ruled unsound and this item exists to have removed, and §4e-R2 (a) measured
/// why no better discriminator is available: `rewind_target` is
/// `detection_height − REWIND_DISTANCE_BLOCKS`, a heuristic and not a fork
/// discovery, so the wallet cannot tell a ten-block reorg from a deep fork detected
/// ten blocks up. **C15 and this row serve the same evidence and demand opposite
/// answers, and the wallet has nothing that separates them.**
///
/// **Why not option 2 (re-aim it upward).** The upward case now has its own row,
/// `a_rewind_does_not_license_an_upward_move_through_the_product_path`, added at
/// re-adjudication because nothing in this file could see the floor work. Re-aiming
/// this one would duplicate that and leave the downward gap with no named red — and
/// a gap with no red is a gap nobody sees.
///
/// **So it stays red, and what it pins is §4b owed row 7** — *nothing binds above
/// the newest bundled row* — ruled OPEN by §4d's Q6 and still open. Closing owed
/// row 7 is what turns this row green. **Owner: the contract author and the
/// implementer jointly, against owed row 7, not against this item.**
///
/// **Measured at re-adjudication, and it is why the row earns its red:** the
/// downward move is not a one-off. An endpoint induces a rewind by serving fork
/// blocks, so it gets a fresh relaxed pass per sync pass; six consecutive passes
/// were driven and index 3 was walked 6,000 blocks down, the wallet re-recording
/// the endpoint's answer each time. The walk is bounded below only by the newest
/// bundled row, which is the same 3,459,780 this row's message names.
///
/// **Predicted killing mutation** (for the state where this row is expected green,
/// i.e. after owed row 7 closes): make the post-rewind branch abstain
/// unconditionally at any index whose recorded height disagrees. The substantive
/// assertion goes red while the control stays green.
#[tokio::test]
async fn a_rewind_does_not_license_a_move_no_reorg_could_produce() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, UNDER_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);

    scan_below_the_fixtures(&w, &obs).await;

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("the honest sequence is recorded");
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));

    induce_rewind(&w, &obs).await;
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "the shard rows survived the rewind, so there is a record left to move"
    );

    let mut greedy = ScriptedEndpoint::ironwood_only(&DEEP_LOWERED);
    let r = w.update_subtree_roots(&mut greedy, tip(), None).await;
    assert_eq!(
        greedy.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the sequence was streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "RED ON PURPOSE — this row pins §4b owed row 7 and is NOT a defect in the \
         direction floor. Read the paragraph above the test before changing \
         anything. The wallet rewound {} blocks and the endpoint answered by moving \
         index 3 by {} — from {} to {}. A completion boundary shifts by the \
         commitments a reorg gains or loses divided by the commitments per block, so \
         ten orphaned blocks cannot move it by {}. Above the newest bundled Ironwood \
         row (3,459,780) no counting oracle reaches this index and this wallet's \
         scanned blocks sit below the served heights, so nothing else answers \
         either. §4e-R2 (c) put the floor on DIRECTION: the recorded arm does NOT \
         abstain here — it answers, and it accepts, because the move is DOWNWARD and \
         downward motion cannot arm the cap erasure. Downward is nonetheless the \
         ORIGINAL measured attack (INC-020 row C). Making this row green needs an \
         oracle above 3,459,780, which is owed row 7 and which this build does not \
         have; it does NOT need a magnitude threshold, and one must not be added — \
         the only constant that would satisfy this row and keep C15 green is a guess \
         between 5,149 and 7,749 blocks, which is the ±REORG_MAX_BLOCKS class §4d Q4 \
         ruled unsound. Got {}",
        crate::constants::REWIND_DISTANCE_BLOCKS,
        TRUTHFUL[3] - DEEP_LOWERED[3],
        TRUTHFUL[3],
        DEEP_LOWERED[3],
        TRUTHFUL[3] - DEEP_LOWERED[3],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "and nothing was written"
    );

    // ANTI-VACUITY, and it is the same pairing R4 makes: a move a reorg CAN
    // produce is still accepted on this wallet, so the refusal above discriminates
    // by size rather than latching after a rewind.
    let mut plausible = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut plausible, tip(), None).await;
    assert!(
        accepted(&r),
        "ANTI-VACUITY: the honest post-rewind correction is still accepted, so this \
         row is measuring the SIZE of the move and not merely re-asserting that a \
         recorded height cannot change. If both are refused, the wallet is stuck on \
         an honest server and C15 is red for the same reason. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&RECROSSED));
    w.close().await.expect("close");
}

/// **The tail inflation the round-1 adjudication drove, held to the direction
/// floor through the PRODUCT PATH** (§4e-R2 (c); added at re-adjudication).
///
/// The floor was built to close exactly this attack, and measured at
/// re-adjudication **nothing in this file could see it work**: with the upward half
/// of the floor deleted — a rewind back to being a full abstention, which is the
/// round-1 behaviour ruled `CONTRACT_WRONG` — `sync_bind_proof::` stayed at 19
/// passed / 2 failed with the identical two reds. Its only cover was two
/// `root_bind::tests` unit cases that hand-build a `PoolEvidence` and never open a
/// wallet. A mechanism whose whole justification is a driven attack, covered only
/// by a constructed struct, is the §4d owed-row-2 shape returning on the fix for
/// §4d owed row 1.
///
/// So the attack is driven here as it was driven against the join: a wallet that
/// records the truthful sequence, a REAL endpoint-induced rewind through
/// `sync_once`, and then the same endpoint moving the tail to within 200 blocks of
/// the tip. Index 0 is left where the signed bundle admits it and the tail is
/// spaced 100 blocks apart, so — asserted by
/// `root_bind::tests::a_rewind_does_not_license_the_tail_inflation_the_adjudicator_drove`
/// at the arm level — neither the bundled frontiers nor the block-size floor can be
/// the refuser. The recorded arm is the only thing left, and it must refuse UPWARD.
///
/// **The anti-vacuity control is the other half of the floor**, not a decoration:
/// the honest DOWNWARD correction after the same rewind is still accepted. A floor
/// that refused both would pass the first assertion and fail this one, and that is
/// the difference between a direction floor and simply switching the relaxation off.
///
/// **Predicted killing mutation:** in `root_bind::check_recorded_heights`, make the
/// post-rewind branch `served > rec && false` (or drop the branch back to
/// `return Ok(())`). The first assertion goes red while the control stays green.
#[tokio::test]
async fn a_rewind_does_not_license_an_upward_move_through_the_product_path() {
    // Index 0 INSIDE the window the newest bundled Ironwood row admits, the tail
    // moved into the last 200 blocks and spaced 100 apart.
    let tail_inflated: [u64; 4] = [TRUTHFUL[0], TIP as u64 - 200, TIP as u64 - 100, TIP as u64];

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, UNDER_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);
    scan_below_the_fixtures(&w, &obs).await;

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    w.update_subtree_roots(&mut honest, tip(), None)
        .await
        .expect("the honest sequence is recorded");
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "precondition: there is a record for the floor to be measured against"
    );

    induce_rewind(&w, &obs).await;

    let mut greedy = ScriptedEndpoint::ironwood_only(&tail_inflated);
    let r = w.update_subtree_roots(&mut greedy, tip(), None).await;
    assert_eq!(
        greedy.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the inflated sequence was streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "A REWIND LICENSES A CORRECTION, NOT AN INFLATION. Index 1 moved UP from {} \
         to {} after a rewind the endpoint induced by serving fork blocks. Upward \
         motion is the cap-erasure chain's necessary condition — every shard must \
         sit above the truncation height for the survivor set to be empty — so this \
         is the direction the floor exists to refuse. Got {}",
        TRUTHFUL[1],
        tail_inflated[1],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "and nothing was written for the pool"
    );

    // ANTI-VACUITY: the floor is on DIRECTION, so the honest downward correction
    // after the same rewind is still accepted. Without this the row would also pass
    // on a build that simply refused everything after a rewind, which is the
    // remedy-worse-than-the-bug shape C15 exists to catch.
    let mut correcting = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r2 = w.update_subtree_roots(&mut correcting, tip(), None).await;
    assert!(
        accepted(&r2),
        "ANTI-VACUITY: the honest post-rewind correction moves index 3 DOWN and must \
         still be accepted, so the refusal above is attributable to the DIRECTION of \
         the move and not to the rewind having switched the bind off. Got {}",
        describe(&r2)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&RECROSSED));
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1a-R2 — the ledger records the EVENT, and consumes PER POOL.
// Contract: `production-readiness-phase-1.md` §4g. Written blind against
// `a02a33e6`; the drives are the section above.
// ════════════════════════════════════════════════════════════════════════════

/// **R12 — a failed truncate does not arm the relaxation** (§4g 0a; question Q-R3).
///
/// *By what evidence does the ledger say a rewind HAPPENED, as opposed to was
/// ATTEMPTED?* At `a02a33e6`: none. `scan_batch`'s continuity arm runs
/// `root_bind::note_rewind` and only then `db.truncate_to_height`; when the truncate
/// returns `Err`, the pass fails, nothing was rewound, and `subtree_root_rewind.chain`
/// is already one higher. The relaxation is armed, persistently, on a wallet whose
/// scanned blocks are exactly what they were — and re-armed on every retry, because
/// the same endpoint produces the same failure on every pass (measured:
/// `chain` reads 2 after the second attempt).
///
/// **The `Err` is reached through the product path** — §4g P1, UNVERIFIED when the
/// contract was written and established here: [`induce_a_rewind_the_wallet_cannot_perform`]
/// carries the geometry, the upstream line, the returned error and its classification.
///
/// **The drive.** A wallet scans [`P1_SCAN_BLOCKS`] blocks below every fixture; an
/// honest pass records [`TRUTHFUL`]; the endpoint forks from the next height and the
/// truncate fails. Then the same endpoint serves [`RECROSSED`] — index 3 moved DOWN,
/// the move a REAL rewind licenses (C15, R4) — and it must be REFUSED by the
/// recorded arm with nothing written: no rewind happened, so the record is not stale
/// and there is no correction to admit. Then the ledger's own reading is asserted to
/// be the BINDING one, read through `obs` by the crate's own reader
/// (`root_bind::read_rewind_watch`, the function the bind calls) — so the refusal
/// cannot be some other arm answering while the ledger still says "rewound". The
/// reader is used rather than the table's columns because §4g R19 leaves the
/// ledger's SHAPE to the implementer; the reader is the one thing every shape keeps.
/// Last, [`TRUTHFUL`] re-served equal is accepted: bound, not latched.
///
/// **Which mechanism refuses the downward move, post-fix, and why nothing else can
/// (IT-10):** the recorded arm's BOUND branch, `served != rec` at index 3. The scan
/// window is five blocks at [`UNDER_SCAN_BASE`], below every served height, so the
/// scanned arm passes by arithmetic; [`RECROSSED`] is the sequence two green rows
/// already prove admissible to every other arm; the completing hashes are unscanned.
/// Ironwood carries the code out of `apply_height_bind`, so it is asserted (§4g R16).
///
/// **What would make it pass for the wrong reason, and what excludes each:** the
/// pass failing BEFORE the continuity arm, so `note_rewind` never ran (the helper
/// pins the error variant to the truncate and the block count to "nothing rewound");
/// the endpoint not streaming (the receipt); a bind that refuses every Ironwood move
/// after any failure (the equal re-serve control).
///
/// **Status at `a02a33e6`: RED** — the downward move after a failed truncate is
/// ACCEPTED and written, and the ledger reads `observed 1 > recorded_at 0`. That is
/// §4g 0a, measured through the product path.
///
/// **Predicted killing mutation:** note-before-truncate with no compensation (the
/// pre-fix order), or a compensation that runs only on `Ok`. Under either the
/// downward move is admitted and the first assertion goes red; the ledger assertion
/// then names the reading that admitted it.
#[tokio::test]
async fn a_failed_truncate_does_not_arm_the_relaxation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, UNDER_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);

    let (lo, hi) = scan_to(&w, &obs, UNDER_SCAN_BASE + P1_SCAN_BLOCKS).await;
    assert!(
        hi < TRUTHFUL[0],
        "PRECONDITION: the scan window ({lo}..={hi}) sits below every completing height \
         served here (lowest {}), so the scanned arm cannot be the refuser (IT-10)",
        TRUTHFUL[0]
    );

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        accepted(&r),
        "pass 1: the honest sequence is recorded, got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));

    induce_a_rewind_the_wallet_cannot_perform(&w, &obs).await;
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "the record stands — nothing was rewound, so nothing about it is stale"
    );

    let mut greedy = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut greedy, tip(), None).await;
    assert_eq!(
        greedy.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: the lowered sequence was streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "A TRUNCATE THAT RETURNED `Err` IS NOT A REWIND. The wallet's scanned blocks are \
         exactly what they were; its record of index 3 ({}) is still the answer for the \
         chain it is on; and the endpoint that induced the failed truncate now moves that \
         index DOWN to {} — the move a real rewind would license (C15). It must be \
         refused by the recorded bind. If it is accepted, the ledger counted the ATTEMPT \
         as the event, and an endpoint that can put a wallet into this geometry (a fork \
         within {} blocks of its scan floor) buys a relaxed pass per retry with no rewind \
         ever performed. Got {}",
        TRUTHFUL[3],
        RECROSSED[3],
        crate::constants::REWIND_DISTANCE_BLOCKS,
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "and nothing was written"
    );

    let watch = crate::root_bind::read_rewind_watch(&obs, ShieldedProtocol::Ironwood)
        .expect("the crate's own ledger reader, through the observation connection");
    assert!(
        !watch.rewound_since_record(),
        "THE LEDGER'S OWN READING IS THE BINDING ONE. After a pass whose truncate returned \
         `Err`, `root_bind::read_rewind_watch` — the function the bind calls — must not \
         say this wallet has rewound since Ironwood's heights were recorded. Read through \
         `obs`, a second keyed connection onto the same file (the ledger is an aux table \
         in it). Got {watch:?}. This is what attributes the refusal above to the ledger \
         rather than to another arm answering while the ledger still relaxes"
    );

    // ANTI-VACUITY: bound, not latched.
    let mut still_honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut still_honest, tip(), None).await;
    assert!(
        accepted(&r),
        "ANTI-VACUITY: the recorded sequence re-served equal is still accepted after the \
         failed pass, so the refusal above is the bind discriminating and not a wallet \
         that stopped accepting Ironwood. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));
    w.close().await.expect("close");
}

/// **THE PLANTED ROW (IT-1 +A) — a failed truncate does not silence the SCANNED
/// oracle, on the wallet that has no recorded height to relax.**
///
/// §4g frames 0a entirely as "the relaxation is armed" — the recorded arm's direction
/// floor — and R12 accordingly drives a downward move AT A RECORDED INDEX. But the
/// ledger is not one arm's gate. `root_bind::check_pool` reads `rewound_since_record`
/// ONCE and three oracles answer to it: the recorded arm relaxes, the scanned arm
/// ABSTAINS, the completing-hash arm abstains (the module doc's own table). So what a
/// failed truncate arms is not only a direction floor; it is the silencing of the
/// wallet's own scanned count. And the population on which that matters most has NO
/// recorded height for R12's shape to touch: the mid-heal Ironwood wallet, NULL until
/// `put_shard_roots` first runs — INC-020's own — where §4e-R2 (c3) already records
/// that the scanned arm and the bundle are the only two oracles left. R12 cannot see
/// this cell: it needs a record. I predict the contract missed it because 0a was
/// written from the wrap review's sentence "the relaxation is armed", and
/// "relaxation" names the recorded arm's behaviour and nothing else's.
///
/// **The drive.** A wallet with no Ironwood record scans five blocks at
/// [`OVER_SCAN_BASE`] — a window ABOVE [`BUNDLED_ONLY_CONTROL`]'s completing height,
/// which is R6's geometry: the wallet counted zero complete Ironwood subtrees at its
/// scan tip, so an endpoint claiming subtree 0 completed below that tip is
/// contradicted by a number no endpoint wrote. The precondition pass proves the arm
/// binds (`scanned_tree_size`, nothing written). Then the endpoint forks from the next
/// height and the truncate fails ([`induce_a_rewind_the_wallet_cannot_perform`]:
/// `blocks` untouched, so the count the wallet holds is still its count). Then the
/// same root again — and it must be refused by the same arm, with nothing written. At
/// `a02a33e6` it is ACCEPTED and WRITTEN: the endpoint has bought a completion height
/// that the wallet's own scan contradicts, on the pool this item exists for, with no
/// rewind performed.
///
/// **Which mechanism refuses it, and why nothing else can (IT-10).** The scanned arm,
/// by the precondition pass on the same bytes. No shard row, so the recorded arm
/// abstains (asserted); the height sits inside the bundle's admitted window (R5's
/// fixture doc: 3,449,780 .. 3,452,280), so the bundled arm admits it; one root has no
/// gap; the completing hash is unscanned. The control serves a height ABOVE this row's
/// scan tip and inside the same bundle window, and is accepted — so the arm
/// discriminates on the count and has not latched after the failed pass.
///
/// **Status at `a02a33e6`: RED** — `Ok([.., Served { roots: 1 }])`, and the row is
/// written.
///
/// **Predicted killing mutations.** R12's (note-before-truncate without
/// compensation), which reds both rows together. And one R12 cannot see: a repair
/// that un-arms only the recorded arm — resolving the failed-truncate case inside
/// `check_recorded_heights` rather than in the one reading `check_pool` takes — leaves
/// R12 green and this row red.
#[tokio::test]
async fn a_failed_truncate_does_not_silence_the_scanned_oracle() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, OVER_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);

    let (lo, hi) = scan_to(&w, &obs, OVER_SCAN_BASE + P1_SCAN_BLOCKS).await;
    assert!(
        lo > BUNDLED_ONLY_CONTROL[0],
        "PRECONDITION: the scan window ({lo}..={hi}) sits ABOVE the served completing \
         height ({}) — R6's geometry, where the wallet's own count contradicts the \
         endpoint and the completing-hash arm abstains",
        BUNDLED_ONLY_CONTROL[0]
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "PRECONDITION: no shard row — the recorded arm abstains, which is the population \
         this row is about and the cell R12 cannot reach"
    );

    let mut before = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_CONTROL);
    let r = w.update_subtree_roots(&mut before, tip(), None).await;
    assert_eq!(
        refusal_code(&r),
        Some("scanned_tree_size"),
        "PRECONDITION: with no rewind observed, the scanned arm refuses this root (R6). \
         Without this the assertion after the failed truncate cannot tell 'still bound' \
         from 'never bound'. Got {}",
        describe(&r)
    );
    assert_eq!(recorded(&obs, "ironwood").len(), 0);

    induce_a_rewind_the_wallet_cannot_perform(&w, &obs).await;

    let mut after = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_CONTROL);
    let r = w.update_subtree_roots(&mut after, tip(), None).await;
    assert_eq!(
        after.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the root was streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("scanned_tree_size"),
        "THE SCANNED COUNT IS STILL THE WALLET'S OWN. The truncate returned `Err`, \
         `blocks` is untouched, and the wallet still holds a count of zero complete \
         Ironwood subtrees at {hi} — nothing about that memory is stale, because no \
         rewind happened. The root the scanned arm refused before the failed pass must \
         be refused after it. If it is accepted, a failed truncate has switched off the \
         only oracle that reaches above the bundle on a wallet with no record — \
         INC-020's population — and the endpoint that induced the failure has written a \
         completion height the wallet's own scan contradicts. Got {}",
        describe(&r)
    );
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "and nothing was written for the pool"
    );

    let watch = crate::root_bind::read_rewind_watch(&obs, ShieldedProtocol::Ironwood)
        .expect("the crate's own ledger reader, through the observation connection");
    assert!(
        !watch.rewound_since_record(),
        "THE LEDGER'S OWN READING IS THE BINDING ONE (as in R12, and for the same \
         reason: it is what attributes the refusal to the ledger). Got {watch:?}"
    );

    // ANTI-VACUITY: a height the wallet's count CANNOT contradict — above this
    // row's scan tip, inside the bundle's admitted window — is accepted, so the
    // scanned arm discriminates rather than latching after the failed pass.
    let above_the_scan: [u64; 1] = [OVER_SCAN_TIP];
    assert!(
        above_the_scan[0] > hi,
        "FIXTURE: the control must sit above the scan tip {hi}, or it measures the \
         same refusal twice"
    );
    let mut control = ScriptedEndpoint::ironwood_only(&above_the_scan);
    let r = w.update_subtree_roots(&mut control, tip(), None).await;
    assert!(
        accepted(&r),
        "ANTI-VACUITY: a completion height above the wallet's scan tip is not \
         contradicted by its count and must be accepted, so the refusal above is the \
         count discriminating and not a wallet that refuses every Ironwood root after a \
         failed pass. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&above_the_scan));
    w.close().await.expect("close");
}

/// **R14 — a later pool's failed put does not keep an earlier pool's relaxation
/// armed** (§4g 0b; question Q-R4).
///
/// *When one pass writes pool A's roots and then fails on pool B's put, what is pool
/// A's relaxation on the next pass?* At `a02a33e6`: still armed. The consume loop in
/// `wallet.rs` runs after all three puts and only if all three returned `Ok`, so
/// Sapling's `recorded_at` never advances past the rewind it just spent, and the next
/// pass is relaxed for Sapling again with no new rewind behind it. One rewind buys
/// unbounded downward steps on any pool that is not the last one put — the ratchet —
/// and the endpoint chooses when the later pool's put fails (P2: a mutated
/// `root_hash` at a recorded index, repeatable at will).
///
/// **The drive** is [`write_sapling_then_fail_on_ironwoods_put`]. Then pass 3 serves
/// Sapling a FURTHER downward move, [`SAPLING_LOWERED_AGAIN`], and it must be
/// REFUSED — bound, by the record pass 2 wrote — with the record exactly what pass 2
/// left. The anti-vacuity control follows: the record pass 2 wrote, re-served equal,
/// is accepted, so the refusal is the bind and not a latch.
///
/// **Which mechanism refuses pass 3, post-fix, and why nothing else can (IT-10).**
/// The recorded arm's BOUND branch (`served != rec`), reached because the ledger says
/// Sapling's relaxation was consumed by pass 2's accepted write. The scan window sits
/// below every height served, so the scanned arm passes by arithmetic (the fixture
/// doc measures the alternative geometry and why it was rejected);
/// [`SAPLING_LOWERED_AGAIN`] is inside the bundle's admitted window; one root has no
/// gap; the completing hash is unscanned. Sapling carries no code out of
/// `apply_height_bind` (module doc), so the assertion is `is_typed_refusal` AND the
/// record (§4g R16).
///
/// **What would make it pass for the wrong reason, and what excludes each:** nothing
/// written in pass 2 (the drive asserts the record moved once); the endpoint not
/// streaming (the receipt); a bind that refuses every Sapling move on this wallet
/// (the equal re-serve control); pass 2 failing before any put — a transport fault, a
/// bind refusal — which is C16's window and not this one, and which the same "moved
/// once" assertion excludes.
///
/// **Status at `a02a33e6`: RED at pass 3** — accepted and written; the record walks
/// from 558,722 to 558,622 on a rewind that was already spent. That is §4g's ratchet,
/// measured through the product path.
///
/// **Predicted killing mutation:** restore the consume to a single loop after all
/// three puts (the pre-fix shape), or consume only the pool whose put ran LAST. Pass 3
/// is then accepted again and both assertions go red.
#[tokio::test]
async fn a_later_pools_failed_put_does_not_keep_an_earlier_pools_relaxation_armed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, SAPLING_ERA_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);

    write_sapling_then_fail_on_ironwoods_put(&w, &obs).await;

    let mut ratchet = ScriptedEndpoint::new(
        pool_script(&SAPLING_LOWERED_AGAIN, SAPLING_TAG),
        Vec::new(),
        Vec::new(),
    );
    let r = w.update_subtree_roots(&mut ratchet, tip(), None).await;
    assert_eq!(
        ratchet.streamed(ShieldedProtocol::Sapling),
        1,
        "IT-10: pass 3 streamed the further-lowered root"
    );
    assert!(
        is_typed_refusal(&r),
        "THE RATCHET. Pass 2 spent the rewind on Sapling — its downward move was accepted \
         and WRITTEN — and then failed on Ironwood's put. Pass 3 moves Sapling down \
         again, {} → {}, with no new rewind anywhere, and it must be REFUSED: the \
         relaxation was consumed by the write that landed, not by the pass that happened \
         to contain it. If this is accepted, one rewind plus a later pool's put failure \
         (endpoint-chosen, repeatable) buys unbounded downward steps on every pool that \
         is not the last one put. Got {}",
        SAPLING_LOWERED[0],
        SAPLING_LOWERED_AGAIN[0],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "sapling"),
        expect_heights(&SAPLING_LOWERED),
        "and Sapling's record is exactly what pass 2 left — nothing of pass 3 was written"
    );

    // ANTI-VACUITY: bound, not latched.
    let mut same = ScriptedEndpoint::new(
        pool_script(&SAPLING_LOWERED, SAPLING_TAG),
        Vec::new(),
        Vec::new(),
    );
    let r = w.update_subtree_roots(&mut same, tip(), None).await;
    assert!(
        accepted(&r),
        "ANTI-VACUITY: the record pass 2 wrote, re-served equal, is still accepted — so \
         the refusal above is the bind discriminating and not a wallet that has stopped \
         accepting Sapling. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "sapling"), expect_heights(&SAPLING_LOWERED));
    w.close().await.expect("close");
}

/// **R15 — the pool whose put failed keeps its relaxation for the honest re-serve**
/// (§4g 0b, the half that must SURVIVE the fix; implementer decision 5).
///
/// The same drive. Pass 2 wrote nothing for Ironwood — its put was the one refused —
/// so Ironwood's pre-reorg record still stands and the honest correction has not yet
/// been made. Pass 3 serves it: [`RECROSSED`], index 3 moved DOWN, with the honest
/// tags. It must be ACCEPTED and written. Consumption is per pool and tied to a
/// non-empty ACCEPTED WRITE; a refused put is neither.
///
/// Then the other half of that sentence: pass 4 serves [`DEEP_LOWERED`] with no new
/// rewind and is refused by the record pass 3 wrote — so the relaxation R15 protects
/// is one honest write wide, consumed by the write that used it, exactly as R4 says.
///
/// **GREEN at `a02a33e6`, on purpose, and not vacuous.** At HEAD nothing is consumed
/// when a later put fails, so the failed pool's relaxation trivially survives; the row
/// exists to guard the half of the behaviour a 0b repair could break. Its killing
/// mutant is the adjudicator's to capture from the post-fix run: **consume every pool
/// the bind approved, regardless of which puts landed** — consume before the put, or
/// consume every `Some(consume_rewind)` slot once any put succeeded. Under that mutant
/// Ironwood is disarmed with nothing written, pass 3 is refused with `recorded_height`,
/// and the wallet is bound to a stale row with a rescan as its only exit — the
/// deadlock the gate exists to prevent. A second mutant kills pass 4: never consume
/// Ironwood (the last pool dropped from a per-pool interleave).
///
/// **Which mechanism refuses pass 4 (IT-10):** the recorded arm's bound branch and
/// nothing else — the scan window is below every served height, [`DEEP_LOWERED`] sits
/// 7,620 blocks above the newest bundled row, and the gap floor has 232 blocks of
/// room. Ironwood carries the code, so it is asserted (§4g R16).
///
/// **What would make pass 3 pass for the wrong reason:** an Ironwood record the rewind
/// deleted (acceptance by abstention) — excluded by the drive's post-rewind record
/// assertion and by the re-record assertion here; a direction floor that abstains
/// entirely — excluded by pass 4.
#[tokio::test]
async fn a_pool_whose_put_failed_keeps_its_relaxation_for_the_honest_re_serve() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, SAPLING_ERA_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);

    write_sapling_then_fail_on_ironwoods_put(&w, &obs).await;

    let mut honest_again = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut honest_again, tip(), None).await;
    assert_eq!(
        honest_again.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: pass 3 streamed the honest correction"
    );
    assert!(
        accepted(&r),
        "THE HONEST RE-SERVE. Ironwood's put failed in pass 2, so nothing was written for \
         it and its post-rewind relaxation must still stand: the honest DOWNWARD \
         correction at index 3 ({} → {}) is accepted. A repair that consumed Ironwood on \
         pass 2 — a pass that wrote nothing for it — leaves this wallet bound to a \
         pre-reorg row with a rescan as the only exit. Got {}",
        TRUTHFUL[3],
        RECROSSED[3],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&RECROSSED),
        "and the correction is RE-RECORDED, so the acceptance was a write and not an \
         abstention over a deleted row"
    );

    // The other half: consumed by the write that used it.
    let mut again = ScriptedEndpoint::ironwood_only(&DEEP_LOWERED);
    let r = w.update_subtree_roots(&mut again, tip(), None).await;
    assert_eq!(
        again.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: pass 4 streamed the second lowered sequence"
    );
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "CONSUMED BY THE WRITE THAT USED IT. Pass 3's accepted write is the corrected \
         record, so Ironwood's relaxation is spent; serving index 3 at {} against the {} \
         just re-recorded is a fresh {}-block move with no fresh rewind behind it, and \
         it must be refused by the recorded bind. If it is accepted, a pool whose put \
         once failed stays relaxed for the life of the wallet. Got {}",
        DEEP_LOWERED[3],
        RECROSSED[3],
        RECROSSED[3] - DEEP_LOWERED[3],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&RECROSSED),
        "and nothing was written"
    );
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// T0-1d — §4l F1–F4 and one plant. Names are the contract's; a rename
// is a finding, not a liberty. Written blind against `1498a91a`.
// ════════════════════════════════════════════════════════════════════════════

/// **A bundled-arm violation that a STANDING RECORD does not refuse first.**
///
/// §4l F3 names `[3_465_000]` and attributes it to `bundled_frontier` — which is
/// what refuses it on a FRESH wallet (R5's fixture). On the drive F3 prescribes
/// the record of 3,451,206 is standing at index 0 and a rewind has been
/// observed, so 3,465,000 is an UPWARD move at a recorded index and the recorded
/// arm's direction floor (§4e-R2 (c)), consulted BEFORE the bundled arm, refuses
/// it as `recorded_height`. F3 serves the contract's literal and pins what the
/// drive measures; THIS fixture is how the bundled arm is reached with the
/// record standing: a DOWNWARD move (admitted by the direction floor after a
/// rewind) to a height the signed binary refutes — the last mainnet row showing
/// zero complete Ironwood subtrees is 3,449,780 (R5's doc), so subtree 0 cannot
/// have completed at 3,449,000. One root, so no gap; above the NU6.3 activation;
/// below the scan window's opposite side ([`UNDER_SCAN_TIP`] < 3,449,000, so the
/// scanned arm abstains); the completing hash is unscanned.
const BELOW_BUNDLE_WINDOW: [u64; 1] = [3_449_000];

/// Upstream's own `{pool}_tree_shards.root_hash` at `index` — the column
/// `put_shard_roots` writes (`commitment_tree.rs`, the `INSERT … ON CONFLICT
/// (shard_index) DO UPDATE SET … root_hash`) and nothing this crate owns does;
/// `None` when no row exists or the row carries no hash.
fn recorded_root_hash(conn: &rusqlite::Connection, pool: &str, index: i64) -> Option<Vec<u8>> {
    conn.query_row(
        &format!("SELECT root_hash FROM {pool}_tree_shards WHERE shard_index = ?"),
        [index],
        |r| r.get::<_, Option<Vec<u8>>>(0),
    )
    .optional()
    .expect("query the shard row")
    .flatten()
}

/// The hash [`pool_script`] gives index `i` under `tag_base`: `[tag_base + i, 0 × 31]`.
fn scripted_hash(tag_base: u8, i: usize) -> Vec<u8> {
    let mut v = vec![0u8; 32];
    v[0] = tag_base + u8::try_from(i).expect("fixtures use few indices");
    v
}

/// The red-first / post-fix record line for a row (IT-3 +A): what the seam
/// returned, printed whether or not the row then passes (`--nocapture`).
fn record_bind(
    row: &str,
    r: &Result<[sync::PoolFetch; sync::SUBTREE_ROOT_POOLS.len()], WalletError>,
) {
    println!("[T0-1d {row}] {}", describe(r));
}

/// **A TYPED refusal that blames neither the store nor this device** (§4l F1):
/// `Err(Sync { .. })` with any stall but `Internal`. Deliberately not pinned to
/// a `StallReason` — whether the mid-state stays `EndpointMisbehaving` with the
/// exit written into the doc and copy, or becomes a distinct reason, is §4l
/// decision 1 and the implementer's; a row that pinned it would be choosing.
fn is_typed_and_not_local(
    r: &Result<[sync::PoolFetch; sync::SUBTREE_ROOT_POOLS.len()], WalletError>,
) -> bool {
    matches!(r, Err(WalletError::Sync { stall }) if *stall != crate::state::StallReason::Internal)
}

/// **F1 — DEFECT (§4l (a)).** A first-contact poison has an exit, and the exit
/// is proven.
///
/// *When `put_shard_roots` refuses a served root because a DIFFERENT root is
/// already recorded at that address, whom does the wallet blame, on what
/// evidence, what does it tell the user to do — and what does the user's remedy
/// actually clear?* (Q-F1). The bind is fenced to the height column (INC-021's
/// fence), so the FIRST server to serve index 0 on a fresh wallet writes
/// whatever `root_hash` it likes into `ironwood_tree_shards.root_hash` and the
/// cap; every honest server thereafter conflicts at that address
/// (`InsertionError::Conflict` out of the cap's `batch_insert`, which runs
/// BEFORE any shard row is touched — so nothing of the second server's lands,
/// and the transaction rolls back besides). The poison is LOCAL. P6: two
/// candidate liars, and the conflict alone cannot tell them apart.
///
/// **The drive.** Fresh wallet (asserted). Server A serves index 0 at the
/// measured completion with hash `X` (`pool_script` under [`IRONWOOD_TAG`]) —
/// accepted, and `X` is read back from the shard row. Server B serves the SAME
/// height at the same index with hash `Y` ([`IRONWOOD_MUTATED_TAG`]: the tag
/// base is the mutation, the height is identical, so no height arm can be the
/// refuser) — the pass is a TYPED refusal that is not `StoreCorrupt` and not
/// `Internal`, and the shard row still carries `X`. Then the exit: `rescan_from`
/// on the same handle (P7 — the rebuild drops the shard rows, MEASURED here
/// through the observer: zero rows after it), and server B again — ACCEPTED,
/// the shard row now carries `Y`.
///
/// **What the row asserts and what it does not.** The exit, not the name (§4l
/// decision 1): the mid-state's `StallReason` is printed, not pinned. The row
/// cannot see the doc or the copy, which is where (a)'s attribution defect
/// lives; if every clause here is green at the contract commit, the exit
/// already works and the finding is that (a) is a surface/doc item the floor's
/// own text cannot make red — reported as measured (IT-3).
///
/// **IT-10.** A's root is streamed and written (`X` read back); B's is streamed
/// (receipt) and refused with `X` still in place; after the rescan the shard
/// table is empty (the rebuild, not a partial write, is what cleared it), and
/// B's acceptance writes `Y` (read back).
#[tokio::test]
async fn a_first_contact_poison_is_not_blamed_on_every_later_honest_server_forever() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let x = scripted_hash(IRONWOOD_TAG, 0);
    let y = scripted_hash(IRONWOOD_MUTATED_TAG, 0);

    {
        let obs = observe(dir.path(), &key);
        assert_eq!(
            recorded(&obs, "ironwood").len(),
            0,
            "PRECONDITION: first contact — nothing recorded, so A's root is the first at index 0"
        );
        assert_eq!(
            count_rows(&obs, "blocks"),
            0,
            "PRECONDITION: nothing scanned"
        );

        // Server A: the first root at index 0, hash X.
        let mut a = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_CONTROL);
        let r = w.update_subtree_roots(&mut a, tip(), None).await;
        assert!(
            accepted(&r),
            "server A's index 0 is accepted on first contact, got {}",
            describe(&r)
        );
        assert_eq!(
            heights(&obs, "ironwood"),
            expect_heights(&BUNDLED_ONLY_CONTROL)
        );
        assert_eq!(
            recorded_root_hash(&obs, "ironwood", 0),
            Some(x.clone()),
            "X WRITTEN: upstream's shard row carries server A's root hash — the record \
             every later server is now held to"
        );

        // Server B: the same height, hash Y.
        let mut b = ScriptedEndpoint::new(
            Vec::new(),
            Vec::new(),
            pool_script(&BUNDLED_ONLY_CONTROL, IRONWOOD_MUTATED_TAG),
        );
        let r = w.update_subtree_roots(&mut b, tip(), None).await;
        record_bind("F1 server B against A's record", &r);
        assert_eq!(
            b.streamed(ShieldedProtocol::Ironwood),
            1,
            "IT-10: B's root was streamed"
        );
        assert!(
            !matches!(&r, Err(WalletError::StoreCorrupt)),
            "F1: a conflict between two servers' roots is not THIS wallet's corruption — \
             'restore from your recovery phrase' is the wrong next step. Got {}",
            describe(&r)
        );
        assert!(
            is_typed_and_not_local(&r),
            "F1: the conflict is a TYPED refusal that blames neither the store nor this \
             device (the shape is the implementer's — §4l decision 1). Got {}",
            describe(&r)
        );
        assert_eq!(
            recorded_root_hash(&obs, "ironwood", 0),
            Some(x.clone()),
            "NOTHING OF B's WRITTEN: index 0 still carries X (the cap conflict fires before \
             any shard row is touched, and the put is one transaction)"
        );
        assert_eq!(
            heights(&obs, "ironwood"),
            expect_heights(&BUNDLED_ONLY_CONTROL),
            "and the height record is untouched"
        );
    }

    // THE EXIT (P7): the rescan rebuild, on the same handle.
    let w = w
        .rescan_from_with_vault(None, Arc::clone(&vault))
        .await
        .expect("a full-history rescan rebuild");
    let obs = observe(dir.path(), &key);
    assert_eq!(
        recorded(&obs, "ironwood").len(),
        0,
        "P7 MEASURED: the rebuild dropped the shard rows, so A's X is gone with them and \
         there is nothing at index 0 for B to conflict with"
    );
    println!(
        "[T0-1d F1] after the rescan: ironwood_tree_cap rows = {}",
        count_rows(&obs, "ironwood_tree_cap")
    );

    let mut b_again = ScriptedEndpoint::new(
        Vec::new(),
        Vec::new(),
        pool_script(&BUNDLED_ONLY_CONTROL, IRONWOOD_MUTATED_TAG),
    );
    let r = w.update_subtree_roots(&mut b_again, tip(), None).await;
    record_bind("F1 server B after the rescan", &r);
    assert_eq!(
        b_again.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: B's root was streamed again"
    );
    assert!(
        accepted(&r),
        "THE EXIT: after the rescan rebuild the server that was refused is ACCEPTED — \
         the poison was a root THIS wallet recorded from a previous server, and the \
         rebuild is what clears it. If this is refused, nothing in the product clears \
         a first-contact poison and every honest server is 'misbehaving' forever. \
         Got {}",
        describe(&r)
    );
    assert_eq!(
        recorded_root_hash(&obs, "ironwood", 0),
        Some(y),
        "and the shard row now carries Y — B's root, written, not a silent no-op"
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&BUNDLED_ONLY_CONTROL)
    );
    w.close().await.expect("close");
}

/// **F2 — DEFECT (INC-021).** A duplicate root hash within one serve is the
/// endpoint signal, not `StoreCorrupt`.
///
/// *When one serve carries two identical `root_hash` values, what does the
/// wallet publish, and by what evidence is that an endpoint signal and not
/// local corruption?* (Q-F2). Upstream's shard table is `UNIQUE (root_hash)`
/// (`zcash_client_sqlite-0.22.0/src/wallet/db.rs`, `CONSTRAINT root_unique`),
/// so the second root's `INSERT` in `put_shard_roots` fails on the constraint,
/// surfaces as `commitment_tree::Error::Query`, and `map_shardtree_err` maps
/// `Query(_)` to `StoreCorrupt` — an intact wallet telling its user to restore
/// from the recovery phrase because a server sent the same bytes twice. The
/// evidence it is the endpoint's: the whole put is one transaction, so nothing
/// was written; the wallet's own record is untouched; and the same heights with
/// distinct hashes are accepted immediately after. (§4l's premise cites
/// shardtree's `QueryError`; measured here, the origin is the SQL constraint —
/// same arm, same defect, corrected provenance.)
///
/// **The drive.** An honest pass records index 0 (`[3_451_206]`, hash `T`), so
/// there is a record to be untouched. The duplicate serve carries indices 0 and
/// 1 at the measured heights with the SAME tag: index 0 byte-identical to the
/// record (no conflict), index 1 duplicating index 0's hash — the only thing
/// wrong. The pass must be `Err(Sync { stall: EndpointMisbehaving })`, never
/// `StoreCorrupt`; the record reads `[3_451_206]`; then the honest serve of the
/// same two heights with distinct hashes is accepted and both are read back.
///
/// **IT-10.** Two roots streamed (receipt); the heights are the measured ones
/// and index 1 is above the newest bundled row, so no height arm refuses them
/// (the honest follow-up proves it: same heights, accepted). The discriminator
/// between the refused serve and the accepted one is index 1's hash byte.
///
/// At the contract commit: `Err(StoreCorrupt)` (the `n=2 same-tag`
/// measurement, re-measured by this row's red-first).
#[tokio::test]
async fn a_duplicate_root_hash_within_one_serve_is_the_endpoint_signal_not_store_corrupt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);

    let first: [u64; 1] = [TRUTHFUL[0]];
    let mut honest = ScriptedEndpoint::ironwood_only(&first);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        accepted(&r),
        "pass 1: index 0 is recorded, got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&first));
    assert_eq!(
        recorded_root_hash(&obs, "ironwood", 0),
        Some(scripted_hash(IRONWOOD_TAG, 0)),
        "precondition: index 0 carries the honest tag"
    );

    // Two roots, one tag: index 1 duplicates index 0's hash.
    let mut duplicate = ScriptedEndpoint::new(
        Vec::new(),
        Vec::new(),
        vec![
            root_at_with_hash(TRUTHFUL[0], IRONWOOD_TAG, block_id_on(TRUTHFUL[0], false)),
            root_at_with_hash(TRUTHFUL[1], IRONWOOD_TAG, block_id_on(TRUTHFUL[1], false)),
        ],
    );
    let r = w.update_subtree_roots(&mut duplicate, tip(), None).await;
    record_bind("F2 duplicate serve", &r);
    assert_eq!(
        duplicate.streamed(ShieldedProtocol::Ironwood),
        2,
        "IT-10: both roots were streamed"
    );
    assert!(
        !matches!(&r, Err(WalletError::StoreCorrupt)),
        "F2 (INC-021): two identical root hashes in one serve are BAD SERVER DATA, never \
         this wallet's corruption — StoreCorrupt sends an intact wallet to its recovery \
         phrase because a server repeated itself. Got {}",
        describe(&r)
    );
    assert!(
        matches!(
            &r,
            Err(WalletError::Sync {
                stall: crate::state::StallReason::EndpointMisbehaving
            })
        ),
        "F2: the duplicate is the endpoint signal — `Err(Sync {{ stall: EndpointMisbehaving }})`, \
         'switch servers'. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&first),
        "the pool's recorded heights are untouched — the put is one transaction and it \
         rolled back"
    );
    assert_eq!(
        recorded_root_hash(&obs, "ironwood", 0),
        Some(scripted_hash(IRONWOOD_TAG, 0)),
        "and index 0's hash is what pass 1 wrote"
    );

    // The same heights, distinct hashes: accepted, so the refusal above is the
    // duplicate's and not the heights'.
    let mut distinct = ScriptedEndpoint::ironwood_only(&TRUTHFUL[..2]);
    let r = w.update_subtree_roots(&mut distinct, tip(), None).await;
    assert!(
        accepted(&r),
        "control: the same two heights with distinct hashes are accepted after the \
         refused serve, so the wallet is intact and the refusal was the duplicate's. \
         Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL[..2]));
    assert_eq!(
        recorded_root_hash(&obs, "ironwood", 1),
        Some(scripted_hash(IRONWOOD_TAG, 1)),
        "and index 1 is written with its own hash"
    );
    w.close().await.expect("close");
}

/// **F3 — CONTROL (§4l (b)).** A refused pool keeps its relaxation for the
/// honest re-serve.
///
/// The module doc names three cases that must not consume the rewind
/// relaxation (served nothing / refused / own put failed); only the third has a
/// killer (R15). This is the second. The drive is R4's: scan below the
/// fixtures, record [`TRUTHFUL`], a real rewind through the product path, then
/// pass 2 serves Ironwood something REFUSED — twice, by two different arms —
/// and pass 3 serves the honest downward correction [`RECROSSED`], which must
/// be ACCEPTED and written: nothing was written for the pool in pass 2, so its
/// relaxation still stands. Pass 4 then serves [`DEEP_LOWERED`] with no new
/// rewind and is refused by the record pass 3 wrote (R4/R15's other half:
/// consumed by the write that used it — so the relaxation this row protects is
/// one honest write wide, not permanent).
///
/// **Pass 2a is the contract's literal, `[3_465_000]`, and it pins the arm the
/// DRIVE measures.** §4l attributes it to `bundled_frontier`, which is what
/// refuses it on a fresh wallet (R5). Here the record of 3,451,206 stands and a
/// rewind has been observed, so 3,465,000 is an UPWARD move at a recorded index
/// and the recorded arm's direction floor — consulted before the bundled arm —
/// refuses it first. **Pass 2b reaches the arm the contract names** with
/// [`BELOW_BUNDLE_WINDOW`]: a DOWNWARD move the direction floor admits, to a
/// height the signed binary refutes. Either refusal, consumed, would red pass
/// 3; both are served so the row cannot pass on one arm's quirk.
///
/// **Named mutant (§4l F3, F8):** consume on REFUSAL — in `apply_height_bind`'s
/// Ironwood arm, move `consume_rewind[IRONWOOD_SLOT] = …` above the
/// `if let Err(refusal)`. Pass 2a consumes; pass 3 is refused with
/// `recorded_height`; the row goes red on pass 3's `accepted` and on the
/// record. GREEN at the contract commit (the arm is built right today; the
/// row is the killer it lacked).
///
/// **IT-10.** Every refusal names its arm; the record is asserted after each
/// pass; the receipt shows each pass streamed; the scan window sits below every
/// served height (asserted by the drive), so the scanned arm abstains on every
/// pass; the completing hashes are unscanned.
#[tokio::test]
async fn a_refused_pool_keeps_its_relaxation_for_the_honest_re_serve() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, UNDER_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);
    scan_below_the_fixtures(&w, &obs).await;
    assert!(
        UNDER_SCAN_TIP < BELOW_BUNDLE_WINDOW[0],
        "FIXTURE: the below-window height sits above the scan tip, so the scanned arm \
         cannot be what refuses pass 2b"
    );

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(accepted(&r), "pass 1 records, got {}", describe(&r));
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));

    induce_rewind(&w, &obs).await;
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "the shard rows survived the rewind, so there is a record for pass 3 to correct"
    );

    // Pass 2a: the contract's literal — refused, by the arm the drive reaches first.
    let mut upward = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_VIOLATION);
    let r = w.update_subtree_roots(&mut upward, tip(), None).await;
    record_bind("F3 pass 2a [3_465_000]", &r);
    assert_eq!(
        upward.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: pass 2a streamed"
    );
    assert!(
        is_typed_refusal(&r),
        "pass 2a: `[3_465_000]` against a record of 3,451,206 is refused, got {}",
        describe(&r)
    );
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "WHICH ARM (IT-10). With the record standing and a rewind observed, 3,465,000 at \
         index 0 is an UPWARD move and the direction floor refuses it before the bundled \
         arm is consulted — §4l's `bundled_frontier` attribution for this literal holds \
         on a FRESH wallet (R5), not on this drive. Pinned to what the drive measures; \
         pass 2b reaches the bundled arm. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "pass 2a wrote nothing"
    );

    // Pass 2b: the arm the contract names — a downward move the floor admits,
    // refuted by the signed binary.
    let mut below = ScriptedEndpoint::ironwood_only(&BELOW_BUNDLE_WINDOW);
    let r = w.update_subtree_roots(&mut below, tip(), None).await;
    record_bind("F3 pass 2b [3_449_000]", &r);
    assert_eq!(
        below.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: pass 2b streamed"
    );
    assert_eq!(
        refusal_code(&r),
        Some("bundled_frontier"),
        "WHICH ARM (IT-10). 3,449,000 is BELOW the record (admitted by the direction \
         floor after a rewind) and below the last bundled row that shows zero complete \
         Ironwood subtrees (3,449,780), so the signed binary is the refuser. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "pass 2b wrote nothing either"
    );

    // Pass 3: the honest correction — ACCEPTED, the relaxation survived two refusals.
    let mut correcting = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut correcting, tip(), None).await;
    record_bind("F3 pass 3 RECROSSED", &r);
    assert_eq!(
        correcting.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: pass 3 streamed the correction"
    );
    assert!(
        accepted(&r),
        "F3: two refused serves wrote nothing for Ironwood, so its post-rewind relaxation \
         must still stand and the honest DOWNWARD correction at index 3 ({} → {}) is \
         accepted. A build that consumes on refusal leaves this wallet bound to a \
         pre-reorg row with a rescan as its only exit — the stale-row deadlock the gate \
         exists to prevent. Got {}",
        TRUTHFUL[3],
        RECROSSED[3],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&RECROSSED),
        "and the correction is RE-RECORDED — a write, not an abstention"
    );

    // Pass 4: consumed by the write that used it (R4/R15), so the relaxation the
    // row protects is one honest write wide.
    let mut again = ScriptedEndpoint::ironwood_only(&DEEP_LOWERED);
    let r = w.update_subtree_roots(&mut again, tip(), None).await;
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "CONSUMED BY THE WRITE THAT USED IT: a further downward move with no fresh rewind \
         is refused by the record pass 3 wrote. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&RECROSSED));
    w.close().await.expect("close");
}

/// **F4 — CONTROL (§4l (b)).** A pool served nothing keeps its relaxation.
///
/// The first of the module doc's three no-consume cases. Same drive as F3;
/// pass 2 serves Ironwood ZERO roots. **What this seam observes (§4l F4: "the
/// row states which it observed") — MEASURED at the contract commit:**
/// `Ok([.., Withheld { proven: 1 }])`. The `Withheld` grading is already a
/// `PoolFetch` outcome at `update_subtree_roots`, not only a surface value —
/// the bundle proves one Ironwood subtree complete below this tip, so the zero
/// is graded withheld here and the surface carries it up. The row admits
/// `Served { roots: 0 }` too (the outcome below the first bundled proof) and
/// prints which it saw; what it refuses is a refusal or a protocol error in
/// that slot. Pass 3's honest correction is ACCEPTED and written; pass 4's
/// further move is refused by the new record.
///
/// **Named mutant (§4l F4, F8):** drop the `(ironwood_served > 0)` guard on the
/// consume — consume on a ZERO serve. Pass 2 consumes, pass 3 is refused with
/// `recorded_height`, the row goes red. The rewind pass itself is a zero serve
/// too, but its root put runs BEFORE its scan arms the relaxation, so under the
/// mutant it is this row's pass 2 that spends it — which is why no existing row
/// could see the guard die (§4l (b)). GREEN at the contract commit.
///
/// **IT-10.** The receipt shows Ironwood was OPENED and streamed zero (not
/// refused at the door, not skipped); the record is unchanged after pass 2;
/// pass 3's write is read back; the scan window sits below every served height.
#[tokio::test]
async fn a_pool_served_nothing_keeps_its_relaxation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet(dir.path(), &vault, UNDER_SCAN_BASE).await;
    let obs = observe(dir.path(), &key);
    scan_below_the_fixtures(&w, &obs).await;

    let mut honest = ScriptedEndpoint::ironwood_only(&TRUTHFUL);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(accepted(&r), "pass 1 records, got {}", describe(&r));
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&TRUTHFUL));

    induce_rewind(&w, &obs).await;
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "the shard rows survived the rewind"
    );

    // Pass 2: Ironwood opened, zero roots.
    let mut nothing = ScriptedEndpoint::ironwood_only(&[]);
    let r = w.update_subtree_roots(&mut nothing, tip(), None).await;
    record_bind("F4 pass 2 zero serve", &r);
    assert!(
        nothing
            .receipt()
            .iter()
            .any(|s| s.protocol == ShieldedProtocol::Ironwood as i32 && s.roots == 0),
        "IT-10: the Ironwood stream was OPENED and streamed zero roots — asked, not \
         refused at the door, not skipped; receipt {:?}",
        nothing.receipt()
    );
    let slot = sync::SUBTREE_ROOT_POOLS
        .iter()
        .position(|p| *p == ShieldedProtocol::Ironwood)
        .expect("Ironwood is a subtree-root pool");
    match &r {
        Ok(outcomes) => {
            let observed = &outcomes[slot];
            println!("[T0-1d F4] the zero serve reads {observed:?} at this seam");
            assert!(
                matches!(
                    observed,
                    sync::PoolFetch::Served { roots: 0 } | sync::PoolFetch::Withheld { .. }
                ),
                "F4: a zero serve is `Served {{ roots: 0 }}`, or `Withheld {{ proven }}` when \
                 the bundle proves a subtree complete at this tip (§4l F4: the row states \
                 which it observed — at the contract commit, `Withheld {{ proven: 1 }}`: the \
                 grading is already at this seam) — never a refusal, never a protocol \
                 error. Got {}",
                describe(&r)
            );
        }
        Err(_) => panic!(
            "a zero serve is not a failure at this seam (the crate's own \
             `update_subtree_roots_empty_first_sync_then_real_converges` pins it); got {}",
            describe(&r)
        ),
    }
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&TRUTHFUL),
        "a zero serve writes nothing"
    );

    // Pass 3: the honest correction — ACCEPTED.
    let mut correcting = ScriptedEndpoint::ironwood_only(&RECROSSED);
    let r = w.update_subtree_roots(&mut correcting, tip(), None).await;
    record_bind("F4 pass 3 RECROSSED", &r);
    assert_eq!(
        correcting.streamed(ShieldedProtocol::Ironwood),
        4,
        "IT-10: pass 3 streamed the correction"
    );
    assert!(
        accepted(&r),
        "F4: a pass that served Ironwood nothing wrote nothing for it, so its post-rewind \
         relaxation must still stand and the honest DOWNWARD correction ({} → {}) is \
         accepted. Consuming on a zero serve leaves the wallet bound to a stale row it \
         cannot correct — a rescan as the only exit. Got {}",
        TRUTHFUL[3],
        RECROSSED[3],
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&RECROSSED),
        "and the correction is RE-RECORDED"
    );

    // Pass 4: consumed by the write that used it.
    let mut again = ScriptedEndpoint::ironwood_only(&DEEP_LOWERED);
    let r = w.update_subtree_roots(&mut again, tip(), None).await;
    assert_eq!(
        refusal_code(&r),
        Some("recorded_height"),
        "CONSUMED BY THE WRITE THAT USED IT: a further move with no fresh rewind is \
         refused by the record pass 3 wrote. Got {}",
        describe(&r)
    );
    assert_eq!(heights(&obs, "ironwood"), expect_heights(&RECROSSED));
    w.close().await.expect("close");
}

/// **THE PLANTED ROW (IT-1 +A) — CONTROL.** A first-contact poison survives a
/// server SWITCH; only the rescan clears it.
///
/// F1 asserts the exit that works. This row asserts the one that does not, and
/// it is the one the surface names today: `EndpointMisbehaving` says "switch
/// servers", and for this cell switching servers is exactly what the user
/// will do — to the honest server B, again, and be told again to switch. §4l
/// (a) says why: the poison is LOCAL, in `{prefix}_tree_shards` / `_tree_cap`,
/// a root THIS wallet recorded from server A. The drive: A poisons index 0, B
/// is refused; the handle is closed and re-opened against another endpoint
/// (the D4 switch — `close()` + `open()` with a new `WalletConfig.endpoint`,
/// the only switch a host can perform, `Inner.endpoint` being immutable after
/// open); B is refused AGAIN with `X` still in the shard row; then the rescan,
/// and B is accepted.
///
/// **PREDICTION:** two misses this pins. (1) An implementer who resolves (a)
/// at the surface with copy that leans on the remedy the reason already
/// carries — "switch servers", maybe "if this persists, …" — has named, first,
/// a remedy that cannot work for this cell; this row is the measurement that
/// it cannot, so the copy has to name the rescan (§4l decision 1's first
/// shape) or a distinct reason has to say it. (2) The mutant this row reds: a
/// repair that makes the switch "work" by dropping the shard rows or the cap
/// on re-open or on an endpoint change — money-relevant state discarded on a
/// config change, with every recorded height going with it. GREEN at the
/// contract commit (the poison does survive a switch today); the fold should
/// see it stay green.
///
/// IT-10 as F1's, plus: the wallet file, key and observer are the same across
/// the switch (one `wallet.db`, one `WalletDbKey`), so "still refused" is about
/// the record and not about a different wallet.
#[tokio::test]
async fn a_first_contact_poison_survives_a_server_switch_and_is_cleared_only_by_the_rescan() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let x = scripted_hash(IRONWOOD_TAG, 0);
    let y = scripted_hash(IRONWOOD_MUTATED_TAG, 0);

    {
        let obs = observe(dir.path(), &key);
        assert_eq!(
            recorded(&obs, "ironwood").len(),
            0,
            "PRECONDITION: first contact"
        );
        let mut a = ScriptedEndpoint::ironwood_only(&BUNDLED_ONLY_CONTROL);
        let r = w.update_subtree_roots(&mut a, tip(), None).await;
        assert!(accepted(&r), "server A records X, got {}", describe(&r));
        assert_eq!(recorded_root_hash(&obs, "ironwood", 0), Some(x.clone()));

        let mut b = ScriptedEndpoint::new(
            Vec::new(),
            Vec::new(),
            pool_script(&BUNDLED_ONLY_CONTROL, IRONWOOD_MUTATED_TAG),
        );
        let r = w.update_subtree_roots(&mut b, tip(), None).await;
        assert!(
            is_typed_and_not_local(&r),
            "precondition: server B is refused against A's record, got {}",
            describe(&r)
        );
        assert_eq!(recorded_root_hash(&obs, "ironwood", 0), Some(x.clone()));
    }
    w.close().await.expect("close before the switch");

    // THE SWITCH: the same wallet, re-opened against another server.
    let switched = WalletConfig {
        endpoint: LightServerEndpoint::new("https://other.example:443").expect("endpoint"),
        ..cfg(dir.path(), Network::Main)
    };
    let w = Wallet::open_with_vault(switched, Arc::clone(&vault))
        .await
        .expect("re-open against the other endpoint");
    {
        let obs = observe(dir.path(), &key);
        let mut b_after_switch = ScriptedEndpoint::new(
            Vec::new(),
            Vec::new(),
            pool_script(&BUNDLED_ONLY_CONTROL, IRONWOOD_MUTATED_TAG),
        );
        let r = w
            .update_subtree_roots(&mut b_after_switch, tip(), None)
            .await;
        record_bind("PLANT server B after the switch", &r);
        assert_eq!(
            b_after_switch.streamed(ShieldedProtocol::Ironwood),
            1,
            "IT-10: streamed"
        );
        assert!(
            is_typed_and_not_local(&r),
            "PLANT: the poison is LOCAL — switching servers does not clear a root this \
             wallet recorded from server A, so the honest server is refused again after \
             the switch. If this is ACCEPTED, the switch cleared money-relevant state on \
             a config change. Got {}",
            describe(&r)
        );
        assert_eq!(
            recorded_root_hash(&obs, "ironwood", 0),
            Some(x),
            "and X is still what the shard row carries"
        );
        assert_eq!(
            heights(&obs, "ironwood"),
            expect_heights(&BUNDLED_ONLY_CONTROL)
        );
    }

    // ONLY THE RESCAN CLEARS IT.
    let w = w
        .rescan_from_with_vault(None, Arc::clone(&vault))
        .await
        .expect("rescan rebuild");
    let obs = observe(dir.path(), &key);
    let mut b_after_rescan = ScriptedEndpoint::new(
        Vec::new(),
        Vec::new(),
        pool_script(&BUNDLED_ONLY_CONTROL, IRONWOOD_MUTATED_TAG),
    );
    let r = w
        .update_subtree_roots(&mut b_after_rescan, tip(), None)
        .await;
    assert!(
        accepted(&r),
        "the rescan is the exit: server B is accepted after it, got {}",
        describe(&r)
    );
    assert_eq!(recorded_root_hash(&obs, "ironwood", 0), Some(y));
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// BIND-1 (§4x) — the SCAN is the oracle above the newest bundled row
//
// DEFECT rows against a mechanism this base does not have: a reconcile that runs
// when a batch commits, compares the recorded completion heights against what the
// wallet's own scan counted, refutes, reports and corrects (§4w (d) names the
// function and the seam). They are RED here by construction.
//
// **The fixtures are the test author's** (§4x "The seam"): the implementer builds
// `root_bind`/`sync` production against the contract's sentences, not against a
// fixture it can read. What is fixed between the halves is B1-10's knob.
//
// ── THE THIRD CHECKER, MEASURED THIS SESSION, AND IT IS A FINDING ────────────
//
// §4x Q4 asks whether a per-block declared commitment tree size can be made to
// pass "ours" (`derive_chain_state`) and "upstream's" (`ScanError::TreeSizeMismatch`,
// plus `put_blocks_rows`' anchor check). The answer is YES —
// [`the_declared_tree_size_knob_is_accepted_by_both_checkers`] measures it. But a
// THIRD reader decides whether the rows below can exist at all, and Q4 does not
// name it: **the shardtree/cap, where a SERVED subtree root hash meets the
// anchor frontier's own ommers.**
//
// `update_tree` inserts `from_state`'s frontier into the pool's `ShardTree` on
// every batch (`zcash_client_backend-0.24.0 data_api/ll/wallet.rs:1766`). A
// frontier at position `p` carries one ommer per set bit of `p`, and an ommer at
// level ≥ 16 IS a claim about the root of the shards below it. `put_shard_roots`
// has already written the ENDPOINT'S root hashes for those same shards. On a real
// chain the two agree. On a synthetic fixture they cannot, because this file
// cannot compute a Pedersen/Sinsemilla root — so the insert is a shardtree
// `Conflict`, `classify_shard_fault` answers `StoreCorrupt`, and **the pass
// scans NOTHING**.
//
// Measured (four instrumented shapes, every edit reverted):
//
// ```text
// roots served, blocks declare 3*2^16       -> pass = Err(StoreCorrupt / RW-STORE-004) span=None
// roots served, blocks declare 3*2^16, +note-> pass = Err(StoreCorrupt / RW-STORE-004) span=None
// roots 0..3 served, blocks declare 4*2^16+1-> pass = Err(StoreCorrupt / RW-STORE-004) span=None
// NO roots served, blocks declare 3*2^16    -> pass = Ok(batches=1) span=Some((3467400,3467499))
// ONE root served (hash = the FILLER node),
//   blocks declare 2^16 + 1                 -> pass = Ok(batches=1) span=Some((3450001,3450100))
// ```
//
// The last line is the ONLY shape that holds both halves, and it is why every row
// below is built on it: at `2^16 + 1` leaves the frontier sits at position `2^16`,
// whose ONLY ommer is the single level-16 node the wallet calls subtree 0's root —
// and the fixture serves that exact value ([`filler_root`]). Two served roots put
// an ommer at level 17, which is a claim about the COMBINATION of two shard roots,
// and no served pair can satisfy it.
//
// **What that costs the contract, named:** the refuted index must be 0 and the
// frontier must sit exactly one leaf above it, so `complete_subtrees` is 1 at
// every block of the window. §4w (d)'s case (2) (the COMPRESSION lie — `rec[i]`
// inside the range with `complete_subtrees ≤ i`) and case (1) (a boundary CROSSED
// inside the range) both require the frontier to sit at or below the refuted
// shard, i.e. inside a shard whose root the endpoint served — the geometry the
// measurement above shows aborts the pass. **So B1-1 and B1-2's defect half are
// not buildable as integration rows at this base**, and they are reported as a
// contract finding rather than written as rows that can never go green (§4x B1-6
// requires the plant to be the only red after this item). Only case (3) — the
// INFLATION half, B1-3 — survives, and it is written here for both pool
// dispositions.
// ════════════════════════════════════════════════════════════════════════════

/// One whole subtree, plus the first leaf of the next. The only frontier size at
/// which this file can serve a subtree root AND scan a chain whose commitment tree
/// is non-empty — see the section banner above for the measurement.
const ONE_SUBTREE_PLUS_ONE: u32 = SUBTREE_LEAVES + 1;

/// B1-3's Ironwood window. The batch is `[IW_SCAN_ANCHOR + 1, IW_SCAN_TIP]` and it
/// sits inside the window the signed bundle allows for Ironwood subtree 0 —
/// `(3_449_780, 3_452_280]`, the two rows [`BUNDLED_ONLY_VIOLATION`]'s doc
/// measured — so the bundled oracle ACCEPTS every height this row serves and
/// cannot be what refuses one.
const IW_SCAN_ANCHOR: u64 = 3_450_000;
const IW_SCAN_TIP: u64 = 3_450_100;
/// **The geometry B1-3 is named for, pinned at compile time**: the scan window
/// sits ENTIRELY BELOW the recorded height, which is what makes the contradiction
/// an INFLATION rather than a crossing. A fixture edit that moved either number
/// past the other would leave the row measuring something else under its old name.
const _: () = assert!(IW_INFLATED > IW_SCAN_TIP);

/// The INFLATED record: subtree 0 claimed complete ABOVE the batch the wallet's
/// own scan proves it was already complete in. Inside the bundle's window, so
/// nothing but the scan can refuse it — which is the whole point of §4f owed row 1.
const IW_INFLATED: u64 = 3_451_206;
/// B1-3's CONTROL record: the same shape at a height the scan AGREES with (below
/// the batch, still inside the bundle's window). Nothing may be reported and
/// nothing written for it.
const IW_CONSISTENT: u64 = 3_449_900;

/// B1-4's Sapling window, chosen the same way: inside the window the bundle allows
/// for Sapling subtree 0 — `(550_000, 560_000]`, the two rows
/// [`SAPLING_LOWERED`]'s doc measured.
const SAP_SCAN_ANCHOR: u64 = 555_000;
const SAP_SCAN_TIP: u64 = 555_100;
/// The same inflation on a REQUIRED pool.
const SAP_INFLATED: u64 = 556_000;

/// A subtree root whose `root_hash` is the 32-byte all-zero node — the FILLER
/// [`sync::testing::frontier_hex`] writes, and therefore the exact value the
/// anchor frontier's single level-16 ommer claims subtree 0's root is.
///
/// Serving this is not a shortcut, it is the only consistent thing to serve: the
/// section banner's measurement shows that any OTHER value makes the frontier
/// insert a shardtree `Conflict` and the pass scan nothing. All-zero is a
/// canonical field element for `jubjub::Base` AND `pallas::Base`, so
/// `parse_sapling_root` and `parse_orchard_root` both take it.
///
/// No `completing_block_hash`, so the C6 clause abstains — this row is about the
/// height dimension and nothing else (IT-10).
fn filler_root(height: u64) -> SubtreeRoot {
    SubtreeRoot {
        root_hash: vec![0u8; 32],
        completing_block_hash: Vec::new(),
        completing_block_height: height,
    }
}

/// **What the PASS reported for one pool** — the returned value, never a captured
/// log (§4x P6: an event emitted inside `scan_batch`'s blocking section runs on a
/// tokio worker and a thread-local `CaptureLayer` never sees it).
///
/// `SyncPass::root_outcomes` is the carrier that already exists for exactly this
/// sentence — it is what `sync_controller::emit_synced` turns into
/// `SyncStatus::UpToDateDegraded`, and its own doc says it exists so a refused
/// pool is distinguishable after the fact. So "the pool reports the refusal" is a
/// `PoolFetch::HeightViolation` in this array.
///
/// **Blind-split note — this is the one seam §4x did NOT fix.** The contract says
/// *"the pool reports the refusal (Ironwood: degraded, a `HeightViolation`-class
/// code naming the reason)"* and names no field. This reader picks the EXISTING
/// carrier rather than inventing an API the implementer would have to guess: a
/// build that reports the same fact through a different field or a different code
/// reddens here for a SHAPE reason, not a behaviour one, and the adjudicator
/// should read such a red as "the halves chose different carriers".
fn pass_refusal_code(p: &sync::SyncPass) -> Option<&'static str> {
    p.root_outcomes.as_ref().and_then(|outcomes| {
        outcomes.iter().find_map(|o| match o {
            sync::PoolFetch::HeightViolation { code } => Some(*code),
            _ => None,
        })
    })
}

/// Drive one BIND-1 pass: a wallet anchored at `anchor` over a chain declaring
/// [`ONE_SUBTREE_PLUS_ONE`] for `pool`, with `recorded` already ingested as that
/// pool's subtree-0 completion height. Returns the pass result and the observer.
///
/// The ingest is ASSERTED accepted here, because every row below is about what the
/// SCAN does to an accepted record: a row whose record never landed would pass by
/// abstention.
async fn bind1_recorded_then_scan(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    pool: ShieldedProtocol,
    anchor: u64,
    scan_tip: u64,
    recorded_height: u64,
) -> (Wallet, WalletDbKey, Result<sync::SyncPass, WalletError>) {
    let sizes = match pool {
        ShieldedProtocol::Sapling => DeclaredTreeSizes::sapling(ONE_SUBTREE_PLUS_ONE),
        ShieldedProtocol::Orchard => DeclaredTreeSizes::default(),
        ShieldedProtocol::Ironwood => DeclaredTreeSizes::ironwood(ONE_SUBTREE_PLUS_ONE),
    };
    let shape = ChainShape::declaring(sizes);
    let (w, key) = scanned_wallet_shaped(dir, vault, anchor, &shape).await;
    let obs = observe(dir, &key);

    assert_eq!(
        count_rows(&obs, "blocks"),
        0,
        "precondition: nothing is scanned yet, so the scanned count oracle abstains and \
         the record lands on its own"
    );
    let roots = vec![filler_root(recorded_height)];
    let mut src = match pool {
        ShieldedProtocol::Sapling => ScriptedEndpoint::new(roots, Vec::new(), Vec::new()),
        ShieldedProtocol::Orchard => ScriptedEndpoint::new(Vec::new(), roots, Vec::new()),
        ShieldedProtocol::Ironwood => ScriptedEndpoint::new(Vec::new(), Vec::new(), roots),
    };
    let ingest = w.update_subtree_roots(&mut src, tip(), None).await;
    assert_eq!(
        src.streamed(pool),
        1,
        "IT-10: the root was really streamed for the pool under test"
    );
    assert!(
        accepted(&ingest),
        "precondition: the record must LAND — above the newest bundled row nothing binds \
         it at ingest, which is §4f owed row 1 and the reason this contract exists. Got {}",
        describe(&ingest)
    );
    drop(obs);

    let mut chain = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        scan_tip,
    )
    .shaped(shape);
    let pass = w
        .sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;
    (w, key, pass)
}

/// **B1-3 — DEFECT. An inflated completion height is refuted at the true
/// boundary, and a consistent one is left alone.**
///
/// The endpoint records "Ironwood subtree 0 completed at [`IW_INFLATED`]" on a
/// wallet that has scanned nothing, which every oracle admits: the bundled rows
/// pin subtree 0's completion into `(3_449_780, 3_452_280]` and the claim is
/// inside it, the wallet has no record and no scanned block, and the root carries
/// no completing hash. Then the wallet scans a batch ENTIRELY BELOW that height
/// over a chain whose every block declares [`ONE_SUBTREE_PLUS_ONE`] commitments —
/// so the wallet's OWN count says subtree 0 was already complete at
/// [`IW_SCAN_TIP`], and a subtree cannot complete twice.
///
/// That is §4w (d)'s case (3): `rec[i] = r` above the scanned range with
/// `i < count_at_end`. It is the half of the reconcile that matters most on a
/// FIRST-CONTACT wallet, because an upward move is the cap erasure's necessary
/// condition (`root_bind`'s direction-floor doc: the cap goes only when every
/// shard sits ABOVE the truncation height) — and at ingest nothing above the
/// newest bundled row can refuse it.
///
/// **What must hold**
///
/// 1. **It reports** — a `HeightViolation` for Ironwood on the returned
///    `SyncPass` ([`pass_refusal_code`], per P6). Ironwood DEGRADES: the batch is
///    already committed and a pass-fatal answer here would block the very scan
///    that heals, so `sync_once` is `Ok`.
/// 2. **The row is CORRECTED to the height the scan counted, and a BRACKET is
///    persisted beside it.** REWRITTEN AT THE BIND-1-R JOIN, and the old
///    expectation was `[None, None]`, which is unreachable twice over: the
///    repair's `withdraw_suffix` refuses to leave a pool's whole prefix NULL
///    while it holds a `root_hash` row (an empty survivor prefix is
///    `truncate_tree_to_subtree_roots`' cap erasure), and on the rising chain the
///    scan CROSSES the boundary, so the reconcile has the truth in hand and
///    rewrites the row rather than withdrawing it.
/// 3. **The refuted number is refused when the endpoint re-serves it, and the
///    truth is accepted.** With the row still holding the lie, a build that had
///    only the equality bind would ACCEPT the re-serve (equal to the row) and
///    REFUSE the truth. Both directions are asserted, and the refusal is asserted
///    by VARIANT.
/// 4. **THE CONTROL, inside the row** (§4x B1-2's second clause): the same
///    fixture with the record already at [`IW_CROSS`] — the one height inside the
///    bracket — reports NOTHING and leaves the row exactly where it was. A
///    reconcile that rewrote rows it has no disagreement with would pass clauses
///    1-3 and fail here.
///
/// **The chain is the rising one** ([`bind1r_two_pass`]), not the flat chain this
/// row used to drive. On a flat chain declaring a whole subtree at every block the
/// only bound the run can offer is the batch ANCHOR — a raw `GetTreeState` the
/// endpoint served — and `root_bind::complete_by` now refuses to rest a durable
/// bound there. Clause 2's bracket cannot exist without the rise.
#[tokio::test]
async fn an_inflated_completion_height_is_refuted_at_the_true_boundary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key, pass1, pass, _before, _t) = bind1r_two_pass(dir.path(), &vault, IW_INFLATED).await;
    let obs = observe(dir.path(), &key);

    // (1) It reports.
    assert_eq!(
        (pass_refusal_code(&pass1), pass_refusal_code(&pass)),
        (None, Some("scanned_boundary")),
        "B1-3 (1): the pass that CROSSES the boundary must REPORT that the recorded \
         completion height is contradicted \
         by the wallet's own scan — a HeightViolation in the Ironwood slot of \
         `SyncPass::root_outcomes`, the carrier `emit_synced` turns into UpToDateDegraded. \
         At the base nothing reports: the row is simply wrong and the wallet is silent \
         about it. \
         THE CODE IS ITS OWN, and asserting it by name is the point: `scanned_boundary` is \
         the SCAN-TIME reconcile, `scanned_tree_size` is the INGEST-time count oracle \
         (`check_against_count` with `CountOracle::Scanned`). Two oracles reading the same \
         column at two different moments; a row that accepted either could not say which \
         one answered — the conflation that let the C6 binding half pass on \
         `bundled_frontier` for two sessions (§4w (f3)). \
         PASS 1 IS SILENT AND MUST BE: it stops below the crossing, so nothing it counted \
         shows subtree 0 complete and an INFLATED record — one above everything observed — \
         is not refutable yet. That asymmetry is the row's subject: an inflation is caught \
         at the TRUE BOUNDARY and nowhere earlier. Got pass1 batches={} \
         root_outcomes={:?} / pass2 batches={} root_outcomes={:?}",
        pass1.batches,
        pass1.root_outcomes,
        pass.batches,
        pass.root_outcomes
    );

    // (2) The row is CORRECTED to the height the wallet's own scan counted, and
    //     the BRACKET is what binds it thereafter. This fixture crosses the
    //     boundary INSIDE the observed run, so clause (a) has the truth in hand
    //     and rewrites rather than withdrawing — the cap clamp never engages.
    //     (An earlier comment here read "the row keeps the refuted number", which
    //     was true only of the flat fixture this row no longer uses.)
    assert_eq!(
        heights(&obs, "ironwood"),
        vec![Some(IW_CROSS as i64)],
        "B1-3 (2): the row is CORRECTED to the height the wallet's own scan counted the \
         boundary at. \
         THE OLD EXPECTATION WAS `[None, None]` AND IT IS UNREACHABLE TWICE OVER, which is \
         why this clause is rewritten rather than relaxed. (a) `withdraw_suffix` clamps a \
         withdrawal away from index 0 while the pool holds a `root_hash` row — an empty \
         survivor prefix is the cap erasure R1 is about — so a NULL at index 0 cannot \
         happen on a pool with served roots at all. (b) On the RISING chain the scan \
         crosses the boundary inside the observed run, so `reconcile_boundaries`' clause \
         (a) knows the true height and REWRITES the row instead of withdrawing it: a \
         withdrawal is what happens when the truth is not in hand, and here it is. That \
         also makes this the product-path row for `a_crossed_boundary_is_corrected_to_\
         the_height_the_scan_counted`, which BIND-1's owed row 1 recorded as proven only \
         as arithmetic. \
         ONE element, not two: the scan reaches exactly `2^16` commitments, so upstream \
         creates no shard 1"
    );
    assert_eq!(
        boundary_bracket(&obs, "ironwood", 0),
        Some((Some(IW_CROSS as i64 - 1), Some(IW_CROSS as i64))),
        "B1-3 (2): and the index is BOUND rather than believed — the scan counted subtree \
         0 still incomplete at {} and complete at {IW_CROSS}, both blocks it scanned \
         itself, so the bracket refutes the recorded {IW_INFLATED} from above. This is what \
         replaces the NULL: the row is not the guard any more, the bracket is. Got {:?}",
        IW_CROSS - 1,
        boundary_bracket(&obs, "ironwood", 0)
    );

    // (3) The lie is refused on re-serve; the truth is accepted.
    let mut liar = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_INFLATED)]);
    let again = w.update_subtree_roots(&mut liar, tip(), None).await;
    assert_eq!(
        refusal_code(&again),
        Some("refuted_height"),
        "B1-3 (3): the refuted number must be REFUSED when the endpoint re-serves it, by \
         VARIANT. Nothing else can answer: the row holds {IW_CROSS} after clause 2, so the \
         equality bind refuses this with `recorded_height` — measured, and it is what a \
         build with no durable refutation answers. A row that accepted ANY typed refusal \
         would pass on that. Got {}",
        describe(&again)
    );
    let mut honest = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_CROSS)]);
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(
        accepted(&r),
        "B1-3 (3): and the height the same scan counted the boundary at must be ACCEPTED — \
         clause (2) has just asserted the row now holds that very height, so this is the \
         equality bind agreeing with the wallet's own count rather than with anything the \
         endpoint said. A build that left {IW_INFLATED} in the row instead would refuse \
         every honest server until a rescan. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        vec![Some(IW_CROSS as i64)],
        "B1-3 (3): and the accepted serve left index 0 on the honest height rather than \
         moving it — the scan had already corrected it there (clause 2), so what this \
         measures is that an ACCEPTED write agrees with the scan rather than overwriting \
         it with something else"
    );
    w.close().await.expect("close");

    // (4) THE CONTROL — §4x B1-2's second clause, run as this row's discriminator.
    let dir2 = tempfile::tempdir().expect("tempdir");
    let vault2 = test_vault();
    let (w2, key2, pass1b, pass2, _before2, _t2) =
        bind1r_two_pass(dir2.path(), &vault2, IW_CROSS).await;
    let obs2 = observe(dir2.path(), &key2);
    assert_eq!(
        (pass_refusal_code(&pass1b), pass_refusal_code(&pass2)),
        (None, None),
        "B1-3 (4) CONTROL: a recorded height the scan AGREES with must produce NO report. \
         {IW_CROSS} is the block the scan counted the boundary at, so it is the one height \
         inside the bracket. A reconcile that refuses whatever it looks at passes clauses \
         1-3 and fails here, and it would turn every honest endpoint into a degraded pool \
         on the first batch. Got {:?} / {:?}",
        pass1b.root_outcomes,
        pass2.root_outcomes
    );
    assert_eq!(
        heights(&obs2, "ironwood"),
        vec![Some(IW_CROSS as i64)],
        "B1-3 (4) CONTROL: and NOTHING is written — the row is exactly where the ingest \
         left it"
    );
    w2.close().await.expect("close");
}

/// **B1-4 — CONTROL. The same contradiction on a REQUIRED pool ends the pass.**
///
/// §4x leaves the disposition for Sapling/Orchard at scan time to the implementer
/// (IT-1b item 1: match the ingest rule, or degrade-and-continue) and requires the
/// stated behaviour to be built for all three pools through ONE function. The
/// floor's own sentence is *"the same contradiction on a Sapling row ends the pass
/// `EndpointMisbehaving` (pass-fatal, as the ingest refusal is)"*, and that is what
/// this row asserts: consistency with `sync::apply_height_bind`, where a Sapling or
/// Orchard height violation is `return Err(endpoint_unusable())` and only Ironwood
/// degrades.
///
/// The fixture is [`an_inflated_completion_height_is_refuted_at_the_true_boundary`]'s,
/// moved to the Sapling window the bundle allows (`(550_000, 560_000]` — the rows
/// [`SAPLING_LOWERED`]'s doc measured) so that, exactly as on Ironwood, the bundled
/// oracle accepts the claim and only the wallet's own scan can refuse it.
///
/// **What the row buys that B1-3 does not.** It is the only place the DISPOSITION
/// is measured. A build that made the scan-time refusal degrade on every pool would
/// be green on B1-3 and red here, and the difference matters: for Sapling and
/// Orchard a wrong height is not a pool the user can do without, it is the wallet's
/// own witness data for the money it already holds.
///
/// **At the base**: no reconcile, so the pass is `Ok` and the row keeps the
/// inflated height — asserted below, so the red cannot be mistaken for a fixture
/// that failed to scan.
#[tokio::test]
async fn a_scan_contradiction_on_a_required_pool_is_fatal_to_the_pass() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key, pass) = bind1_recorded_then_scan(
        dir.path(),
        &vault,
        ShieldedProtocol::Sapling,
        SAP_SCAN_ANCHOR,
        SAP_SCAN_TIP,
        SAP_INFLATED,
    )
    .await;
    let obs = observe(dir.path(), &key);
    assert!(
        matches!(
            pass,
            Err(WalletError::Sync {
                stall: crate::state::StallReason::EndpointMisbehaving
            })
        ),
        "B1-4: on a REQUIRED pool a recorded completion height the wallet's own scan \
         contradicts must end the pass as the ENDPOINT's fault, the way the ingest \
         refusal already does (`apply_height_bind` answers a Sapling violation with \
         `endpoint_unusable()` and carries only Ironwood as a degraded pool). At the base \
         the pass is a clean `Ok` and the contradiction is invisible. Got {}",
        describe_pass(&pass)
    );
    assert_eq!(
        scanned_span(&obs),
        Some((SAP_SCAN_ANCHOR + 1, SAP_SCAN_TIP)),
        "and the batch is still COMMITTED — the refusal is about the pass's verdict, not \
         about throwing away scanned blocks (the un-scanning of a committed batch would be \
         a second, larger decision this contract does not take)"
    );
    w.close().await.expect("close");
}

/// **B1-5 — DEFECT. A served root the scan contradicts is the ENDPOINT's fault,
/// not our store's.**
///
/// §4w (b)'s finding, driven. A served-root HASH that the wallet's own scanned
/// leaves refute IS detected — `shardtree-0.7.1 prunable.rs` raises a `Conflict`
/// when `update_tree` inserts the batch anchor's frontier over a shard whose root
/// the endpoint already supplied — but it surfaces through
/// `PutBlocksError::ShardTreeForBlockRange` → `map_scan_err` →
/// `classify_shard_fault`'s `_ => StoreCorrupt` arm. **So an endpoint lie caught at
/// scan time reads as OUR corruption**, and `StoreCorrupt` is the class whose
/// remedy on glass is restore-from-seed. The INGEST path already classifies the
/// same class as `EndpointMisbehaving` (`sync::map_shardtree_err`'s `Insert` arm,
/// T0-1d), so the two doors into one fault disagree about whose fault it is.
///
/// # The discriminator is the root hash and nothing else
///
/// Both arms serve ONE Ironwood root at the same admissible height
/// ([`IW_CONSISTENT`] — inside the window the bundle allows, at or below what the
/// scan counts), over the same chain declaring [`ONE_SUBTREE_PLUS_ONE`]. They
/// differ in `root_hash`:
///
/// * the CONTROL serves [`filler_root`] — the 32-byte all-zero node, which is
///   exactly what the anchor frontier's single level-16 ommer claims subtree 0's
///   root is, so the two agree and the batch commits;
/// * the LIE serves a different canonical field element at the same index, which
///   the same frontier refutes.
///
/// So the ingest is accepted in both arms (asserted), the declared size is the
/// same in both arms (asserted), and what changes is one 32-byte field.
///
/// # What must hold
///
/// 1. The pass ends `Err(Sync { EndpointMisbehaving })` — the endpoint's fault,
///    the class whose remedy is "switch servers; if every server is refused,
///    rescan" (T0-1d's copy), never `StoreCorrupt`.
/// 2. **The wallet stays open and nothing of the user's is destroyed** — the shard
///    row is still there and the handle still closes cleanly. A misclassification
///    here is not a cosmetic string: `StoreCorrupt` is the one class that tells a
///    user their wallet file is broken.
/// 3. The CONTROL scans cleanly, which is what makes clause 1 attributable to the
///    served hash rather than to B1-10's declared size.
///
/// **At the base**: `Err(StoreCorrupt / code RW-STORE-004)`, with `span = None` —
/// and that second number is worth as much as the class. The pass does not commit
/// a partial batch and then complain; it scans NOTHING, on every pass, for as long
/// as the endpoint keeps serving that root. An endpoint that serves one bad
/// 32-byte field can stop a wallet syncing entirely and have it blamed on the
/// wallet's own storage.
#[tokio::test]
async fn a_served_root_the_scan_contradicts_is_the_endpoints_fault_not_store_corrupt() {
    let shape = ChainShape::declaring(DeclaredTreeSizes::ironwood(ONE_SUBTREE_PLUS_ONE));

    // ── THE CONTROL, first, so a failure here is read as a fixture fault and not
    // as the behaviour under test.
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, key) = scanned_wallet_shaped(dir.path(), &vault, IW_SCAN_ANCHOR, &shape).await;
        let obs = observe(dir.path(), &key);
        let mut honest =
            ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_CONSISTENT)]);
        let ingest = w.update_subtree_roots(&mut honest, tip(), None).await;
        assert!(
            accepted(&ingest),
            "control precondition: the height is admissible, so the ingest lands. Got {}",
            describe(&ingest)
        );
        let mut chain = ScriptedChain::new(
            ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
            IW_SCAN_TIP,
        )
        .shaped(shape.clone());
        let pass = w
            .sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
            .await;
        assert!(
            pass.is_ok(),
            "THE CONTROL: with a served root the anchor frontier AGREES with, the same \
             chain scans cleanly — so the refusal below is attributable to the 32-byte \
             root hash and not to the declared tree size B1-10 introduces. Got {}",
            describe_pass(&pass)
        );
        assert_eq!(
            scanned_span(&obs),
            Some((IW_SCAN_ANCHOR + 1, IW_SCAN_TIP)),
            "and the control really scanned the window"
        );
        w.close().await.expect("close");
    }

    // ── THE LIE: the same height, a root hash the same frontier refutes.
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = scanned_wallet_shaped(dir.path(), &vault, IW_SCAN_ANCHOR, &shape).await;
    let obs = observe(dir.path(), &key);
    let mut liar = ScriptedEndpoint::new(
        Vec::new(),
        Vec::new(),
        vec![root_at(IW_CONSISTENT, IRONWOOD_TAG)],
    );
    let ingest = w.update_subtree_roots(&mut liar, tip(), None).await;
    assert_eq!(
        liar.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the root was streamed"
    );
    assert!(
        accepted(&ingest),
        "precondition: the HEIGHT is admissible and no oracle at ingest reads the root \
         HASH, so the lie is written — which is the state this row is about. Got {}",
        describe(&ingest)
    );
    assert_eq!(heights(&obs, "ironwood"), vec![Some(IW_CONSISTENT as i64)]);

    let mut chain = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        IW_SCAN_TIP,
    )
    .shaped(shape);
    let pass = w
        .sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await;

    // (1) The class.
    assert!(
        matches!(
            pass,
            Err(WalletError::Sync {
                stall: crate::state::StallReason::EndpointMisbehaving
            })
        ),
        "B1-5: a served root the wallet's own scan contradicts is the ENDPOINT's fault. \
         The ingest door already says so (`map_shardtree_err`'s Insert arm answers \
         EndpointMisbehaving); the scan door answers `StoreCorrupt`, which on glass is \
         restore-from-seed for a wallet whose file is intact. One class, one grep. Got {}",
        describe_pass(&pass)
    );

    // (2) The wallet stays open and nothing of the user's went with it.
    assert_eq!(
        heights(&obs, "ironwood"),
        vec![Some(IW_CONSISTENT as i64)],
        "B1-5: and the refusal destroys nothing — the wallet is intact, which is exactly \
         why calling it StoreCorrupt is the wrong sentence"
    );
    w.close()
        .await
        .expect("the wallet stays open and closes cleanly");
}

// ════════════════════════════════════════════════════════════════════════════
// BIND-1-R (§4x-R) — THE REFUTATION MUST BE DURABLE, AND A WITHDRAWAL MUST NOT
// OPEN A GAP. Written blind from the contract at `edcd9e4b`; every geometry
// below was MEASURED at that base before a row was built on it.
//
// ## What was measured, because three of the floor's rows rest on it
//
// All five runs below are this file's own fixtures, driven in the foreground at
// the base, with the probe deleted afterwards.
//
// **HOW TO READ (m1)-(m6): they were measured at `edcd9e4b`, BEFORE the repair,
// and they are the record of what the defect did. (m4) is the one that was WRONG
// and it carries its own withdrawal. The heights and table shapes quoted in (m1)
// and (m2) are the FLAT fixture's, which the crypto audit's CRITICAL then took
// away — the live geometry is [`bind1r_two_pass`]'s and its doc describes it.**
//
// **(m1) The re-serve IS re-accepted, which is the whole of P3 rendered in a
// fixture.** On the [`ONE_SUBTREE_PLUS_ONE`] chain, an Ironwood record at
// `IW_SCAN_ANCHOR + 50` is refuted by the scan (`scanned_boundary`, the row
// NULLed), and then:
//
// ```text
// re-serve  = Ok([.., Served { roots: 1 }])  heights = [Some(3450050), None]
// re-serve2 = Ok([.., Served { roots: 1 }])  heights = [Some(3450050), None]
// truth     = Ok([.., HeightViolation { code: "recorded_height" }])
//             heights = [Some(3450050), None]
// ```
//
// The lie comes straight back, twice, and the HONEST height is then refused by
// the equality bind against the lie the wallet just re-accepted. That is the
// fold review's CRITICAL-adjacent finding driven end to end, and it is why R2
// and R3 are one geometry: at the base the wallet ends up stuck on the lie, not
// merely un-protected against it.
//
// **(m2) Why every earlier oracle abstains or AGREES here** (§4x P5 — a row that
// wants one clause to be the one that refuses must say why the others cannot
// be). One root is served, so `check_completion_gap`'s `windows(2)` is empty and
// it abstains; the row is NULL after the refutation, so `check_recorded_heights`
// abstains (P2); [`IW_REFUTED`] sits inside `(3_449_780, 3_452_280]`, the window
// `bundled_counts(Main, Ironwood)` allows for subtree 0, so the BUNDLED oracle
// AGREES; the newest scanned block is [`IW_SCAN_TIP`] with one complete subtree,
// and index 0 must complete at or below it, so the SCANNED count oracle AGREES
// (3,450,050 ≤ 3,450,100 — this is exactly the blind spot the review names: the
// ingest count oracle reads only the NEWEST scanned block); and [`filler_root`]
// carries no `completing_block_hash`, so the C6 clause abstains. **Nothing that
// exists at the base can refuse this re-serve**, which is what makes R2's
// control twin a measurement rather than a decoration.
//
// **(m3) `ResetToSubtreeRoots` is UNREACHABLE through the product path, and the
// reason is structural rather than a fixture that has not been tried yet.**
// `select_truncation_height` (`zcash_client_sqlite-0.22.0 wallet.rs:4091-4123`)
// picks `MAX(height) FROM blocks WHERE height <= :requested AND <per-pool
// tolerance>`, so the truncation height must be a row in `blocks`; and
// `plan_tree_truncation` (`:4221-4269`) returns `ResetToSubtreeRoots` only when
// the pool retains NO checkpoint at or below it. Every batch checkpoints its own
// ANCHOR, which is one block below the batch's first block, so
// `min(checkpoint) < min(blocks)` always holds and no block height can have an
// empty checkpoint prefix. Measured three ways: the reorg drive returns
// `Err(Sync { EndpointMisbehaving })` (upstream `RequestedRewindInvalid` — no
// height qualified at all) with the cap, the shards and the checkpoints all
// untouched; the checkpoints of a freshly scanned wallet are
// `[3450000, 3450096]` against blocks `3450001..=3450100`; and checkpoint
// PRUNING does not lift the floor either —
//
// ```text
// to +500:  blocks=500  iw_ckpt n=8   min=3450000
// to +3000: blocks=3000 iw_ckpt n=53  min=3450000
// to +8000: blocks=8000 iw_ckpt n=139 min=3450000
// ```
//
// 139 retained checkpoints and the lowest is still the birthday. So R1 takes the
// fallback its own row licenses — *"if that branch is not reachable in a
// fixture, the row becomes a unit row over the survivor-set query itself, and
// the reason is written down"* — and this is the reason. It extends C14's
// measurement (`an_induced_rewind_after_inflated_heights_does_not_empty_the_tree_cap`,
// which recorded that the erasure "did not fire" without saying it could not):
// it cannot fire, on any wallet this repository can build.
//
// **(m4) ~~A CROSSING cannot be observed through the product path~~ — WITHDRAWN
// AT THE JOIN, by this half, because it was WRONG and the way it was wrong is
// worth keeping.** The original text ran: `crossing_height` needs the observed
// run to JUMP from `complete_subtrees <= i` to `> i` between two contiguous
// heights; all three checkers force a block's declared size to equal *anchor +
// the commitments the block actually carries*; a chain with no transactions is
// therefore flat; so `truth: Some(h)` is unreachable from any fixture in this
// file. Every premise there is true and the conclusion does not follow. **A
// crossing at index 0 does not need a block carrying a subtree's worth of
// commitments — it needs the count to step from `2^16 - 1` to `2^16`, which is
// ONE.** Carry that one commitment for real ([`Rise`]) and all three checkers are
// satisfied by construction, because the declaration now IS *anchor + what the
// block contains*. Measured on the rising chain: `derive_chain_state` returns
// `Ok(65536)` over the window, the pass is `Ok(batches=1)` scanning
// `3450001..=3450100`, and the reconcile corrects the row to the crossing height.
// The error was arguing from "a subtree is 65,536 leaves" to "a fixture must
// carry 65,536 commitments" without asking how far the count actually had to
// move; the fix cost one `CompactOrchardAction`.
//
// **What is still out of reach is R5's half, and for a different reason.** An
// anchor the wallet's own blocks contradict WITHIN a batch cannot be built: the
// anchor and its own batch's blocks must agree or the batch is refused before the
// reconcile runs (measured — a pass whose anchor jumps across a pass boundary is
// `Err(Sync { EndpointMisbehaving })` with the batch un-scanned). The repair's
// `anchor_standing` is nevertheless exercised from the OTHER side by every row
// here: the two-pass drive exists precisely because a bracket is minted only from
// an anchor the wallet can check against its own `blocks` row.
//
// **(m5) The banner's "not buildable as integration rows" is REFUTED for
// the COMPRESSION half.** That banner concluded that only the inflation case
// survives because a frontier inside a shard whose root the endpoint served
// aborts the pass. Measured at a geometry it did not try — ONE root served and
// the blocks declaring 1,000 leaves, i.e. a frontier INSIDE shard 0 — the pass
// is `Ok(batches=2)`, the span is the whole window, and the reconcile refutes
// through clause (c), the compression lie: `code=Some("scanned_boundary")`,
// `heights=[None]`. Two served roots still abort (the banner's own
// measurement stands for that); ONE does not. Recorded here rather than acted
// on, because no row of THIS contract needs it: the compression lie's re-serve
// is refused at the base by the ingest count oracle (the newest scanned block
// shows zero complete subtrees), so it produces no red for R2.
//
// **(m6) The cut the fold review drew is in the wrong place, and R2's CONTROL
// arm is the measurement that says so.** §4x-run's finding reads *"the ingest
// scanned-count oracle reads only the NEWEST scanned block and catches
// inflation, never compression."* `check_against_count` with
// `CountOracle::Scanned` refuses index *i* when `i < complete` and
// `served > at_height`, and when `i >= complete` and `served <= at_height` —
// so at index 0 on a wallet showing one complete subtree the ONLY refusal is
// `served > at_height`. **Every height at or below the newest scanned block is
// accepted, in either direction**, including one the wallet's own earlier
// blocks flatly refute. The real cut is *above the scan frontier* (caught) vs
// *at or below it* (missed), not inflation vs compression. Measured
// first-contact, with no refutation anywhere in the story: a wallet that scans
// `3450001..=3450100` over the [`ONE_SUBTREE_PLUS_ONE`] chain and is THEN served
// a root claiming subtree 0 completed at [`IW_REFUTED`] — a block whose own
// predecessor the wallet counted as already holding a whole subtree — answers
// `Ok([.., Served { roots: 1 }])` and writes it, and nothing ever looks again
// (`read_scanned_sizes` reads only the CURRENT batch's range). **A per-pool
// `reconciled_through` watermark — §4x-R's Q3, the answer R8 is written for —
// does not close this**: the batch that carried the evidence was reconciled
// successfully, so the watermark is already above it, and a record written
// afterwards sits below the mark forever. Stated here because a repair that
// implements Q3 exactly as asked will leave this open, and a residual nobody
// wrote down is the one that gets re-discovered.
// ════════════════════════════════════════════════════════════════════════════

/// **The chain R2/R3 are driven over: `2^16 - 1` leaves, stepping to `2^16` at
/// [`IW_CROSS`]** (B1-10-R). One block below a subtree boundary, and the
/// crossing block carries the single commitment that takes it over.
const IW_BELOW_CROSS: u32 = SUBTREE_LEAVES - 1;

/// **The block in which subtree 0 actually completes, and therefore the ONE
/// height an honest endpoint can serve for index 0 on this chain.**
///
/// It is a block the wallet SCANS, which is the whole point: `root_bind`'s
/// `complete_by` refuses to rest a ceiling on `observed[0]`, the batch anchor the
/// endpoint supplied, so the bracket exists only because the crossing sits on two
/// counted blocks — `(IW_CROSS - 1, IW_CROSS]`. Inside the window the signed
/// bundle allows for Ironwood subtree 0 (`(3_449_780, 3_452_280]`) and at or
/// below [`IW_SCAN_TIP`], so every other oracle admits it too.
const IW_CROSS: u64 = IW_SCAN_ANCHOR + 60;

/// Where the first pass of the drive stops — BELOW [`IW_CROSS`], so the crossing
/// falls in the SECOND pass, whose first batch is then anchored on a block this
/// wallet has already scanned. `root_bind::anchor_standing` mints a bracket only
/// from an anchor it can check against the wallet's own `blocks` row, so the
/// two-pass shape is the mechanism and not a convenience.
const IW_MID: u64 = IW_CROSS - 20;

/// **The number the endpoint gets written and the wallet's own scan then
/// refutes** — 50 blocks BELOW the true crossing, so it falls at or under the
/// bracket's floor.
///
/// Every ingest oracle that exists without the bracket admits it, which is what
/// makes the re-serve re-acceptable at the base and the whole of (m2): the bundle
/// allows it, the wallet has scanned nothing when it first lands, and after the
/// scan the ingest count oracle — which reads only the NEWEST scanned block —
/// only refuses a height ABOVE [`IW_SCAN_TIP`].
const IW_REFUTED: u64 = IW_CROSS - 50;

/// **Upstream's OWN survivor-set query, run here rather than reasoned about.**
///
/// The SELECT and the index-gap break below are
/// `truncate_tree_to_subtree_roots`' (`zcash_client_sqlite-0.22.0
/// wallet/commitment_tree.rs:743-783`) verbatim: the rows with a non-NULL
/// height at or below the truncation height and a non-NULL root hash, in shard
/// order, collected only while `shard_index == roots.len()`. What upstream does
/// with the result is the part R1 cannot drive (see (m3)): it `DELETE`s
/// `{prefix}_tree_checkpoints`, `{prefix}_tree_shards` and `{prefix}_tree_cap`
/// unconditionally and re-puts exactly this vector. **An EMPTY vector is
/// therefore the cap's erasure**, which is §4x-R's P1 and the fold review's
/// CRITICAL.
fn truncation_survivors(
    conn: &rusqlite::Connection,
    pool: &str,
    truncation_height: u64,
) -> Vec<(i64, i64)> {
    let sql = format!(
        "SELECT shard_index, subtree_end_height
         FROM {pool}_tree_shards
         WHERE subtree_end_height IS NOT NULL
           AND subtree_end_height <= ?1
           AND root_hash IS NOT NULL
         ORDER BY shard_index"
    );
    let mut stmt = conn.prepare(&sql).expect("the shard table exists");
    let rows: Vec<(i64, i64)> = stmt
        .query_map([truncation_height as i64], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("the survivor query runs")
        .collect::<Result<Vec<_>, _>>()
        .expect("survivor rows decode");
    let mut out: Vec<(i64, i64)> = Vec::new();
    for (shard_index, height) in rows {
        // Upstream: `if shard_index != roots.len() { break }` — re-insertion via
        // `put_shard_roots` requires contiguity from index zero.
        if shard_index != i64::try_from(out.len()).expect("a small shard count") {
            break;
        }
        out.push((shard_index, height));
    }
    out
}

/// **The one drive R1, R2 and R3 all stand on** — a POPULATED Ironwood prefix
/// whose index 0 the wallet's own scan then refutes.
///
/// Returns the wallet, its key, the pass, and the survivor prefix as it stood
/// BEFORE the refutation, so R1 can compare the same query at the same
/// truncation height on both sides of the withdrawal rather than asserting an
/// absolute shape the repair is free to choose.
///
/// Every step asserts its own precondition: a row whose record never landed, or
/// whose scan never ran, or whose refutation never fired, would let each of the
/// three rows below pass by abstention. The refutation itself is asserted by the
/// code the join adjudicated (`scanned_boundary` — the SCAN-TIME reconcile,
/// deliberately distinct from the INGEST count oracle's `scanned_tree_size`).
///
/// # The chain is RISING, and that is the crypto audit's CRITICAL, not a detail
///
/// This drive used to run over a FLAT chain declaring `2^16 + 1` leaves at every
/// block. The rows were green on it — and they were green on a bound that rested
/// on `observed[0]`, the batch ANCHOR, which is a raw `GetTreeState` the endpoint
/// served: every product-path bracket it produced was `floor = None,
/// ceiling = 3_450_000`. A durable bound the endpoint could have chosen outright
/// is R3's deadlock with the sign flipped, and the audit found it by reading the
/// fixtures the rows passed on rather than the rows. `root_bind::complete_by` now
/// refuses `k == 0` and `::incomplete_through` refuses `k < 2`, so on the flat
/// chain NOTHING is minted and the re-serve is accepted exactly as at the base.
///
/// The rise is the fixture that answers it: `2^16 - 1` leaves up to
/// [`IW_CROSS`] − 1 and `2^16` from [`IW_CROSS`] on, with that one block carrying
/// the one Ironwood commitment its declaration claims. The bracket the scan mints
/// is `(IW_CROSS - 1, IW_CROSS]` and BOTH ends are blocks this wallet counted.
async fn bind1r_two_pass(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
    recorded_height: u64,
) -> (
    Wallet,
    WalletDbKey,
    sync::SyncPass,
    sync::SyncPass,
    Vec<(i64, i64)>,
    u64,
) {
    let shape = ChainShape::rising(
        DeclaredTreeSizes::ironwood(IW_BELOW_CROSS),
        Rise {
            at: IW_CROSS,
            commitments: 1,
        },
    );
    let (w, key) = scanned_wallet_shaped(dir, vault, IW_SCAN_ANCHOR, &shape).await;
    let obs = observe(dir, &key);
    assert_eq!(
        count_rows(&obs, "blocks"),
        0,
        "precondition: nothing is scanned yet, so the scanned count oracle abstains and \
         the record lands on its own"
    );

    let mut src = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(recorded_height)]);
    let ingest = w.update_subtree_roots(&mut src, tip(), None).await;
    assert_eq!(
        src.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the root was really streamed for the pool under test"
    );
    assert!(
        accepted(&ingest),
        "precondition: the record must LAND — above the newest bundled row nothing binds \
         it at ingest, which is §4f owed row 1 and the reason BIND-1 exists. Got {}",
        describe(&ingest)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        vec![Some(recorded_height as i64)],
        "precondition: index 0 holds the endpoint's number"
    );
    // At or above every recorded height, so the survivor query is asked where the
    // prefix really is populated — a truncation height BELOW the record would
    // return an empty set for a reason that has nothing to do with a withdrawal.
    let truncation_at = recorded_height.max(IW_SCAN_TIP);
    let before = truncation_survivors(&obs, "ironwood", truncation_at);
    assert_eq!(
        before,
        vec![(0, recorded_height as i64)],
        "precondition: the prefix is POPULATED — upstream's own survivor query returns \
         index 0, so a truncation at {truncation_at} would re-put it and keep the cap. \
         This is the state P1 says this fold is the first writer able to break"
    );
    assert!(
        count_rows(&obs, "ironwood_tree_cap") > 0,
        "precondition: `put_shard_roots` wrote the cap (`Retention::Reference`), which is \
         the structure that binds root hashes — C14's row is what it buys"
    );
    drop(obs);

    // ── PASS 1, entirely BELOW the crossing. Its batches are anchored on the
    // birthday and on blocks it is scanning for the first time, so every anchor is
    // `AnchorStanding::Unverifiable` and NO bracket can be minted here. What it
    // does do is refute the record: the wallet counts `IW_BELOW_CROSS` leaves at
    // the very block the record NAMES, one short of a subtree.
    let mut chain1 = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        IW_MID,
    )
    .shaped(shape.clone());
    let pass1 = w
        .sync_once(&mut chain1, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect("pass 1 scans the window below the crossing");
    {
        let obs = observe(dir, &key);
        assert_eq!(
            scanned_span(&obs),
            Some((IW_SCAN_ANCHOR + 1, IW_MID)),
            "precondition: pass 1 really scanned, and it stopped BELOW the crossing"
        );
    }

    // ── PASS 2, THROUGH the crossing, and its first batch is anchored on
    // [`IW_MID`] — a block THIS WALLET SCANNED in pass 1. That is the whole reason
    // the drive is two passes: `root_bind::anchor_standing` reads the wallet's own
    // `blocks.{prefix}_commitment_tree_size` at the anchor height, and a bracket is
    // minted ONLY from an anchor that standing calls `Verified`. A single-pass
    // drive anchors every batch on a height it has not scanned, so it mints
    // nothing — which is what the flat fixture was quietly doing before the crypto
    // audit read it.
    let mut chain2 = ScriptedChain::new(
        ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
        IW_SCAN_TIP,
    )
    .shaped(shape);
    let pass = w
        .sync_once(&mut chain2, &sync::CancelToken::new(), NOOP_PROGRESS)
        .await
        .expect(
            "Ironwood DEGRADES on a scan-time refusal; the pass itself completes (the batch \
             is already committed, and a pass-fatal answer here would block the scan that \
             heals)",
        );
    let obs = observe(dir, &key);
    assert_eq!(
        scanned_span(&obs),
        Some((IW_SCAN_ANCHOR + 1, IW_SCAN_TIP)),
        "precondition: pass 2 carried the scan THROUGH the crossing at {IW_CROSS}"
    );
    drop(obs);
    (w, key, pass1, pass, before, truncation_at)
}

/// **The refuting drive R1, R2 and R3 stand on** — [`bind1r_two_pass`] with the
/// endpoint's number at [`IW_REFUTED`], plus the three things those rows need to
/// have happened before their own clause means anything: the scan REPORTED the
/// refutation on both passes, and the bracket it minted rests on two blocks this
/// wallet counted.
///
/// Returns the wallet, its key, pass 2, the survivor prefix as it stood BEFORE the
/// withdrawal, and the truncation height it was measured at.
async fn refute_index_zero(
    dir: &Path,
    vault: &Arc<dyn KeychainPort>,
) -> (Wallet, WalletDbKey, sync::SyncPass, Vec<(i64, i64)>, u64) {
    let (w, key, pass1, pass, before, truncation_at) =
        bind1r_two_pass(dir, vault, IW_REFUTED).await;
    let obs = observe(dir, &key);
    assert_eq!(
        pass_refusal_code(&pass1),
        Some("scanned_boundary"),
        "precondition: the REFUTATION fired on pass 1. `scanned_boundary` is the \
         scan-time reconcile's own code, adjudicated at the S269 join precisely so it \
         cannot be confused with the ingest count oracle's `scanned_tree_size` — two \
         oracles over the same column at two moments (§4w (f3)'s lesson). Got \
         batches={} root_outcomes={:?}",
        pass1.batches,
        pass1.root_outcomes
    );
    assert_eq!(
        pass_refusal_code(&pass),
        Some("scanned_boundary"),
        "precondition: the refutation is still live on pass 2, which is the pass that \
         crosses the boundary and therefore the pass that can mint a bracket. Got \
         batches={} root_outcomes={:?}",
        pass.batches,
        pass.root_outcomes
    );
    assert_eq!(
        boundary_bracket(&obs, "ironwood", 0),
        Some((Some(IW_CROSS as i64 - 1), Some(IW_CROSS as i64))),
        "precondition, and it is the one the crypto audit's CRITICAL turns on: the bracket \
         this drive mints must rest on two blocks THE WALLET SCANNED — `(IW_CROSS - 1, \
         IW_CROSS]` — and not on the batch anchor, which is a raw `GetTreeState` the \
         endpoint served. Every product-path bracket the FLAT fixture produced was \
         `floor = None, ceiling = 3_450_000`, the anchor height, and the rows below were \
         green on it. Got {:?}",
        boundary_bracket(&obs, "ironwood", 0)
    );
    (w, key, pass, before, truncation_at)
}

/// The persisted bracket for one pool and shard, read through the observer
/// connection — `(floor, ceiling)`, `None` when no row exists.
///
/// This IS a storage location and this file's discipline is to drive through the
/// product path and observe upstream's tables. The exception is deliberate and
/// narrow: it is asserted only as a PRECONDITION, to say which evidence the
/// refusals below are resting on, because "a bound that rests on the endpoint's
/// own anchor" and "a bound that rests on two scanned blocks" are indistinguishable
/// from the refusal alone — which is exactly how the flat fixture passed.
fn boundary_bracket(
    conn: &rusqlite::Connection,
    pool: &str,
    shard: i64,
) -> Option<(Option<i64>, Option<i64>)> {
    conn.query_row(
        "SELECT floor_height, ceiling_height FROM subtree_boundary_bound
         WHERE pool = ?1 AND shard_index = ?2",
        rusqlite::params![pool, shard],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
    .expect("the bracket table exists")
}

/// **R1 — DEFECT, and it is the CRITICAL. A withdrawal must not leave the
/// truncation reading an empty survivor set.**
///
/// §4x-R P1, driven as far as this repository can drive it.
/// `truncate_tree_to_subtree_roots` collects the shards with a non-NULL
/// `subtree_end_height` at or below the truncation height, **stops at the first
/// index gap**, then `DELETE`s `{prefix}_tree_checkpoints`,
/// `{prefix}_tree_shards` AND `{prefix}_tree_cap` unconditionally and re-puts
/// only what it collected. A NULL at index 0 makes that set EMPTY, so the cap —
/// the structure that makes a differing root hash at a recorded index an insert
/// `Conflict` rather than a silent overwrite (C14) — is dropped with nothing
/// re-put in its place. BIND-1 is the first writer in this tree able to put a
/// NULL into a populated prefix, which is why `root_bind.rs:716-717`'s *"upward
/// motion is the erasure's necessary condition"* is false as the code stands.
///
/// # Why this is a query row and not a truncation row, and it is not a shortcut
///
/// The contract licenses exactly this fallback — *"if that branch is not
/// reachable in a fixture, the row becomes a unit row over the survivor-set
/// query itself, and the reason is written down"* — and (m3) in this section's
/// banner is the reason, measured three ways. `ResetToSubtreeRoots` cannot fire
/// on any wallet this repository can build: `select_truncation_height` demands a
/// truncation height that is a row in `blocks`, `plan_tree_truncation` demands a
/// pool with no checkpoint at or below it, and every batch checkpoints its own
/// anchor one block below its first block. So the half this row CANNOT observe
/// is upstream's `DELETE`, which is upstream's code and pinned `=0.22.0`; the
/// half it CAN observe is the input that decides it, which is entirely ours.
///
/// # What must hold
///
/// The SAME query at the SAME truncation height that returned a populated prefix
/// before the withdrawal must not return an empty one after it. Stated as a
/// before/after comparison on purpose: §4x-R's IT-1b leaves the durability
/// store's shape open, and a repair may legitimately keep the refuted number,
/// write a different admissible one, or mark the row some other way — what it
/// may not do is leave index 0 with no height at all while the prefix is
/// populated.
///
/// **At the base**: `before = [(0, 3450050)]`, `after = []`. The refutation
/// NULLs the row (`root_bind::apply_boundary_correction`'s
/// `SET subtree_end_height = ?2` with `truth = None`) and the survivor set
/// empties — the CRITICAL, reproduced on a real wallet file through the product
/// path.
#[tokio::test]
async fn a_withdrawal_never_leaves_a_gap_the_truncation_reads_as_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key, _pass, before, truncation_at) = refute_index_zero(dir.path(), &vault).await;
    let obs = observe(dir.path(), &key);

    let after = truncation_survivors(&obs, "ironwood", truncation_at);

    // **RE-SCOPED AT THE BIND-1-R JOIN, and the row is weaker on purpose.**
    // As written this asserted `!after.is_empty()` — the withdrawal may take the
    // endpoint's NUMBER but not the ROW — because §4x-R R1 called the empty
    // survivor set THE CRITICAL. It is not, and the reason is measured: the branch
    // that consumes this query, `ResetToSubtreeRoots`, CANNOT FIRE in this build.
    // It requires no checkpoint at or below the truncation height
    // (`zcash_client_sqlite-0.22.0 wallet.rs:4227-4250`), while `update_tree`
    // inserts the batch ANCHOR's frontier with `Retention::Checkpoint`
    // unconditionally, one block BELOW the batch's first block
    // (`zcash_client_backend-0.24.0 data_api/ll/wallet.rs:1766-1772`), and a
    // truncation height must be a row in `blocks`. This half measured that three
    // ways and the orchestrator verified both upstream spans.
    //
    // Keeping the old assertion cost more than it bought: exempting index 0 from
    // the withdrawal leaves the endpoint's lie ACTIVE in the two upstream
    // consumers that read that column — `mark_stabilized_notes` for the latch and
    // `v_ironwood_shard_scan_ranges` for the shard window — so the wallet kept
    // refusing re-serves of a number it was still using. The adjudication took
    // the withdrawal and gave up the non-emptiness.
    //
    // What the row protects now is DENSITY, which is what the CRITICAL was always
    // really about: upstream's collection breaks at the first index gap, so a
    // height that survives OUTSIDE the collected prefix belongs to a shard the
    // truncation is about to delete and never re-put. An all-withdrawn table
    // satisfies this trivially and honestly — it is the fresh-wallet state, "this
    // wallet does not know", not a gap.
    //
    // **RESIDUAL, unguarded by decision:** if a future change makes
    // `ResetToSubtreeRoots` reachable — upstream stops checkpointing the batch
    // anchor, or a truncation height can fall below every `blocks` row — a fully
    // withdrawn table erases that pool's cap at the next truncation. The thing to
    // re-check is upstream's checkpointing.
    // **THIS ROW IS NOT THE GUARD FOR THE SUFFIX RULE, and saying so is the point.**
    // Measured at the join, by the orchestrator, with `withdraw_suffix`'s
    // `shard_index >= ?1` mutated to `= ?1` (a single-row withdrawal — the hole
    // this row's own message describes): **this row stays GREEN.** The fixture
    // records exactly ONE index, so withdrawing "index 0 only" and "the suffix
    // from 0" produce the identical table and the mutation is invisible here.
    // The same mutant reddens `root_bind::tests::a_suffix_withdrawal_leaves_the_
    // recorded_heights_a_dense_prefix` (`FAILED. 32 passed; 1 failed`, driven on
    // this tree), which IS the guard.
    //
    // Left in, weakened and labelled, rather than deleted or grown a second
    // recorded index: it still pins the end-to-end shape through the product path
    // — a real wallet file, a real scan, upstream's own survivor query — and the
    // property it cannot discriminate is guarded one layer down with a watched
    // red. A row that looks like a guard and is not is the failure this programme
    // exists to stop, so it says which it is.
    //
    // **RE-EXAMINED AT THE JOIN once the rising chain existed, and the label
    // STANDS — with a correction to what it would take.** The suggestion was that
    // two recorded indices would make it discriminating. They would not: the
    // mutant is `withdraw_suffix`'s `shard_index >= ?1` becoming `= ?1`, and a
    // withdrawal from index 0 is CLAMPED to 1, so with rows {0, 1} both forms NULL
    // exactly row 1 and the tables are identical. It takes THREE recorded indices
    // for `>= 1` (rows 1 and 2 NULLed, survivors `[(0, h0)]`) and `= 1` (row 1
    // NULLed, row 2 left holding `h2` behind a gap) to differ — which needs the
    // endpoint to serve three roots at three bundle-admissible heights and the
    // pass to survive it, and the banner measured two served roots aborting
    // the pass on the geometry it tried. Not cheap, and the property is already
    // guarded by `root_bind::tests::a_suffix_withdrawal_leaves_the_recorded_\
    // heights_a_dense_prefix` with a watched red. **This row therefore has NO
    // entry in `evals/mutants.tsv`, deliberately** — the registry's own rule is
    // that a test with no mutant row does not count as a guard, and that is the
    // true statement about this one.
    let recorded_rows = recorded(&obs, "ironwood");
    let dense_prefix: Vec<i64> = after.iter().map(|&(index, _)| index).collect();
    assert!(
        dense_prefix
            .iter()
            .enumerate()
            .all(|(i, &ix)| ix == i as i64),
        "R1: upstream collects survivors ORDER BY shard_index and breaks at the first \
         index gap, so whatever it returns must be a dense prefix from 0. Got \
         {after:?} (before the refutation, {before:?})"
    );
    for (index, height) in &recorded_rows {
        assert!(
            height.is_none() || dense_prefix.contains(index),
            "R1 (the property the CRITICAL was really about): shard {index} still holds \
             height {height:?}, but it is OUTSIDE the prefix upstream's truncation \
             re-puts ({after:?}) — so that height names a shard root the truncation \
             deletes and never restores. A withdrawal must be a SUFFIX; a single-row \
             hole leaves exactly this. Full table: {recorded_rows:?}"
        );
    }
    assert!(
        count_rows(&obs, "ironwood_tree_cap") > 0,
        "R1: the cap this wallet still holds — asserted so a future change that empties \
         it by some other route cannot hide behind the query above"
    );
    w.close().await.expect("close");
}

/// **R2 — DEFECT. A refuted height is refused when the endpoint re-serves it.**
///
/// §4x-R P3/P4, driven end to end. The scan refutes index 0, the row goes NULL,
/// the pool badges — and then nothing remembers. `check_recorded_heights`
/// ABSTAINS on a NULL row (`root_bind.rs:808-810`); the ingest count oracle
/// reads only the NEWEST scanned block, so a height BELOW that block passes it;
/// and the reconcile itself is blind on the next pass, because its evidence was
/// in a batch that has gone by (its own doc: *"a record the wallet scanned in an
/// EARLIER batch is examined by the batch that contained it and not again"*).
/// The lie comes straight back in, at the same index, on the next pass.
///
/// P3 measured this on `check_pool` as a 15,367-block window at index 1; this
/// row is the same shape at index 0 through the product path, and (m2) in this
/// section's banner enumerates why every oracle that exists at the base abstains
/// or AGREES with the re-serve.
///
/// # The two arms, and why the control is the attribution
///
/// * **THE LIE** — the wallet that refuted [`IW_REFUTED`] must refuse it.
/// * **THE CONTROL** — a wallet that scanned the SAME window over the SAME chain
///   and never refuted anything must still ACCEPT the same height at the same
///   index. Both wallets reach the serve with an all-NULL `ironwood_tree_shards`
///   and the same `blocks` table, so every oracle that exists at the base gives
///   both arms the same verdict. A refusal on one and an acceptance on the other
///   can therefore come from nothing but the refutation's own memory — which is
///   what makes this row's red attributable without naming the store.
///
/// **The code is asserted BY NAME and the name is a guess this contract did not
/// fix.** §4x-R fixes no string for the new refusal, so `refuted_height` is this
/// half's choice, exactly as `scanned_tree_size` was at the join before the
/// implementer minted `scanned_boundary` and the adjudicator moved the row. A
/// build that refuses durably under a DIFFERENT code reddens here for a SHAPE
/// reason and the adjudication is to move this row to the implementer's code —
/// but a build that refuses through one of the EXISTING codes has not built a
/// durable refutation at all, it has re-derived one of the oracles (m2) shows
/// cannot answer here, and that red is a behaviour red.
///
/// **At the base**: `Ok([.., Served { roots: 1 }])` and the row holds 3,450,050
/// again — measured, (m1).
#[tokio::test]
async fn a_refuted_height_is_refused_when_the_endpoint_re_serves_it() {
    // ── THE CONTROL, first, so a failure here is read as a fixture fault and not
    // as the behaviour under test.
    {
        let shape = ChainShape::declaring(DeclaredTreeSizes::ironwood(ONE_SUBTREE_PLUS_ONE));
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let (w, key) = scanned_wallet_shaped(dir.path(), &vault, IW_SCAN_ANCHOR, &shape).await;
        let obs = observe(dir.path(), &key);
        let mut chain = ScriptedChain::new(
            ScriptedEndpoint::new(Vec::new(), Vec::new(), Vec::new()),
            IW_SCAN_TIP,
        )
        .shaped(shape);
        let pass = w
            .sync_once(&mut chain, &sync::CancelToken::new(), NOOP_PROGRESS)
            .await
            .expect("the control's chain scans");
        assert_eq!(
            scanned_span(&obs),
            Some((IW_SCAN_ANCHOR + 1, IW_SCAN_TIP)),
            "control precondition: the SAME window, over the SAME chain"
        );
        assert_eq!(
            pass_refusal_code(&pass),
            None,
            "control precondition: this wallet never recorded anything, so nothing was \
             refuted and there is no refutation to remember. Got {:?}",
            pass.root_outcomes
        );
        let mut ep = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_REFUTED)]);
        let r = w.update_subtree_roots(&mut ep, tip(), None).await;
        // THE ATTRIBUTION, stated first and on its own: whatever else this build
        // does with this serve, it must not be the REFUTATION that answers — there
        // is no refutation on this wallet to answer with. This clause is what makes
        // the lie arm's red attributable, and it survives even if the acceptance
        // clause below ever has to be relaxed.
        assert_ne!(
            refusal_code(&r),
            Some("refuted_height"),
            "R2 CONTROL: this wallet refuted NOTHING, so a durable-refutation refusal here \
             would mean the store is keyed on something other than the refutation — an \
             index, a pool, a whole table — and the lie arm below would then be passing on \
             that instead. Got {}",
            describe(&r)
        );
        assert!(
            accepted(&r),
            "R2 CONTROL: the height itself is ADMISSIBLE — the bundle allows it, the \
             scanned count oracle agrees with it, and nothing is recorded to contradict \
             it. A build that refuses here has not made a refutation durable, it has \
             pinned the index, and R3 is where that shows up as the deadlock the NULL \
             existed to avoid. \
             ADJUDICATOR, READ THIS BEFORE CALLING IT A DEFECT: the acceptance asserted \
             here is the BASE's behaviour and this contract's own premise, and it is also \
             a hole nothing in §4x-R closes — the wallet has scanned the very blocks that \
             refute {IW_REFUTED} (its batch anchor already showed subtree 0 complete) and \
             accepts it anyway, because the ingest count oracle reads only the NEWEST \
             scanned block. A build that closed THAT by re-reading the blocks it already \
             holds at ingest would redden this clause while being strictly better; that is \
             a finding about the fixture, not about the build, and the `refuted_height` \
             clause above is the one that carries the attribution. Got {}",
            describe(&r)
        );
        assert_eq!(
            heights(&obs, "ironwood").first().copied(),
            Some(Some(IW_REFUTED as i64)),
            "R2 CONTROL: and it is WRITTEN, so the refusal in the lie arm below is about \
             the wallet's memory of a refutation and not about the number"
        );
        w.close().await.expect("close");
    }

    // ── THE LIE: the same height, at the same index, on a wallet that refuted it.
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key, _pass, _before, _t) = refute_index_zero(dir.path(), &vault).await;
    let obs = observe(dir.path(), &key);

    let mut again = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_REFUTED)]);
    let r = w.update_subtree_roots(&mut again, tip(), None).await;
    assert_eq!(
        again.streamed(ShieldedProtocol::Ironwood),
        1,
        "IT-10: the endpoint really re-served the refuted root"
    );
    assert_eq!(
        refusal_code(&r),
        Some("refuted_height"),
        "R2: a height this wallet's own scan has already REFUTED must not be re-writable \
         by re-serving it. At the base the correction is what is durable and the \
         refutation is not, so `check_recorded_heights` abstains on the NULLed row, the \
         ingest count oracle agrees (the newest scanned block is above the served \
         height), and the lie is written again — measured `Ok([.., Served {{ roots: 1 }}])`. \
         ASSERTED BY VARIANT, never by `is_typed_refusal`: a row that accepted any typed \
         refusal would have passed at the base on nothing, and one that accepted \
         `recorded_height` or `scanned_tree_size` would be passing on an oracle the \
         control arm just proved cannot answer here. The STRING is this half's guess \
         (§4x-R fixes none); a different string for the same mechanism is a shape red for \
         the adjudicator, an EXISTING string is a behaviour red. Got {} outcomes={:?}",
        describe(&r),
        r.as_ref().ok()
    );
    let after = heights(&obs, "ironwood");
    assert!(
        after.first() != Some(&Some(IW_REFUTED as i64)),
        "R2: and the refused number is NOT back in the row. The refusal and the write are \
         two separate claims — a build that reported a refusal and wrote anyway would \
         leave the wallet in exactly the state this row exists to stop. Got {after:?}"
    );
    w.close().await.expect("close");
}

/// **R3 — DEFECT, and the hard half. After a refutation the TRUTH is still
/// accepted.**
///
/// §4x-R Q2: *"with the lie refused durably, what un-sticks the wallet?"* A
/// repair that satisfies R2 by pinning the index harder — a permanent NULL, a
/// blanket refusal at a refuted index, a relaxation that never re-arms — refuses
/// the honest server too, and that is the deadlock the NULL existed to avoid. It
/// is worse than the bug: the bug is a wallet that can be lied to, the deadlock
/// is a wallet that cannot be told the truth.
///
/// # One wallet, because two fixtures cannot prove it
///
/// The contract's own words: *"prove BOTH halves on one row: the lie refused
/// twice, the truth accepted once."* This row drives exactly that sequence on
/// one wallet, in one order, with no reset in between: re-serve [`IW_REFUTED`],
/// re-serve it again, then serve [`IW_CONSISTENT`] — a height the same scan
/// AGREES with (below the batch anchor, inside the window the bundle allows for
/// subtree 0, at or below the newest scanned block). Split across two fixtures,
/// a build could satisfy each half in a state the other never reaches.
///
/// # The assertion ORDER is deliberate, and it is the lesson
///
/// The un-stick is asserted FIRST. B1-3's clauses 2-4 went into the fold
/// having never been RUN, because the row's own base failed at clause 1 and the
/// later clauses never printed a value. Here the durability clauses are the ones
/// that redden at the base, so putting them first would hide the un-stick behind
/// them and this row would ship with its own subject unmeasured.
///
/// **At the base** the un-stick clause is what reddens, and it reddens for the
/// base's own reason rather than for a missing feature: the two re-serves are
/// ACCEPTED and WRITTEN, so by the time the honest height arrives the row holds
/// the lie again and `check_recorded_heights` equality-binds against it —
/// measured `Ok([.., HeightViolation {{ code: "recorded_height" }}])`, the row
/// still `Some(3450050)`. The wallet is stuck on a number it had already proved
/// false, and only a rescan gets it out.
#[tokio::test]
async fn the_truth_is_still_accepted_after_a_refutation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key, _pass, _before, _t) = refute_index_zero(dir.path(), &vault).await;
    let obs = observe(dir.path(), &key);

    // Drive the whole sequence before asserting any of it, so every clause has a
    // value to report even when an earlier one is going to fail.
    let mut lie1 = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_REFUTED)]);
    let r1 = w.update_subtree_roots(&mut lie1, tip(), None).await;
    let mut lie2 = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_REFUTED)]);
    let r2 = w.update_subtree_roots(&mut lie2, tip(), None).await;
    let mut truth = ScriptedEndpoint::new(Vec::new(), Vec::new(), vec![filler_root(IW_CROSS)]);
    let r3 = w.update_subtree_roots(&mut truth, tip(), None).await;
    let landed = heights(&obs, "ironwood");
    assert_eq!(
        (
            lie1.streamed(ShieldedProtocol::Ironwood),
            lie2.streamed(ShieldedProtocol::Ironwood),
            truth.streamed(ShieldedProtocol::Ironwood),
        ),
        (1, 1, 1),
        "IT-10: all three serves really streamed a root"
    );

    // (1) THE UN-STICK — asserted first, so it is MEASURED whatever the durability
    // clauses below do.
    assert!(
        accepted(&r3),
        "R3 (1): an honest endpoint must be able to establish the truth at a refuted index \
         WITHOUT a rescan. {IW_CROSS} is the block in which this wallet's own scan counted \
         subtree 0 completing — it is the ONE height inside the bracket the scan minted, \
         `(IW_CROSS - 1, IW_CROSS]`, and every other oracle admits it too (inside the \
         bundle's window for subtree 0, at or below the newest scanned block). So nothing \
         here has any ground to refuse it. \
         THIS IS THE CLAUSE A REPAIR THAT PINS HARDER FAILS: a durable refusal keyed on \
         the INDEX rather than on the refuted VALUE, or a bracket that never re-opens, \
         refuses this serve and leaves the wallet needing a rescan — which §4x-R R3 calls \
         worse than the bug. Got {} (r1={}, r2={})",
        describe(&r3),
        describe(&r1),
        describe(&r2)
    );
    assert_eq!(
        landed.first().copied(),
        Some(Some(IW_CROSS as i64)),
        "R3 (2): and the truth is what the row holds. \
         WEAKLY DISCRIMINATING, and said so rather than dressed up: on this fixture the \
         scan-time reconcile has ALREADY corrected the row to {IW_CROSS} (the crossing is \
         inside the scanned span, so `reconcile_boundaries`' clause (a) knows the true \
         boundary and rewrites the row rather than withdrawing it), so this value is in \
         place before the serve. What the clause still buys is that the accepted serve did \
         not MOVE it — an acceptance that wrote something else, or wrote nothing and left \
         a stale row, would show here. Got {landed:?}"
    );

    // (3)/(4) THE DURABILITY — the lie refused TWICE, not once.
    assert_eq!(
        refusal_code(&r1),
        Some("refuted_height"),
        "R3 (3): the first re-serve of the refuted height must be refused, by VARIANT. \
         This is R2's clause, carried here because the contract requires both halves on \
         ONE wallet: a build that refuses the lie in R2's fixture and accepts it in this \
         one has not made the refutation a property of the wallet. Got {}",
        describe(&r1)
    );
    assert_eq!(
        refusal_code(&r2),
        Some("refuted_height"),
        "R3 (4): and the SECOND re-serve too. A one-shot refusal — consumed by the first \
         re-serve the way the rewind relaxation is consumed by the first accepted write — \
         would pass clause 3 and hand the index back to the endpoint on the very next \
         pass, which is the shape §4x-R's IT-1b item 2 asks to be stated rather than \
         discovered. Got {}",
        describe(&r2)
    );
    w.close().await.expect("close");
}

// ════════════════════════════════════════════════════════════════════════════
// S15-F1 — THE ATTACK CATALOGUE AT A NON-ZERO START (the built-diff fold, F2)
// ════════════════════════════════════════════════════════════════════════════
//
// `docs/plan/s15-f1-subtree-roots-fetch-only-what-is-new.md` §7 and §11. Every
// fixture above serves at most a few dozen roots, so since S15-F1 every one of
// its passes plans `From { start: 0 }` — a FULL fetch — and the catalogue never
// reaches the incremental bind (`root_bind::check_pool_from` over
// `recorded[..start] ++ served`). The review (security M2, crypto M2, arch
// MAJOR) asked for the multi-pass attack rows to run at `start > 0`.
//
// The harness: [`S15_ROOTS`] = 200 bind-consistent roots, so a verified pool
// plans `start = floor64(199) = 192` and the endpoint streams 8. Each attack
// runs twice on fresh wallets, the honest full pass first in both:
// * INCREMENTAL — the same `Inner`: the hostile pass asks from 192 (the receipt
//   says 8, so no fallback re-fetched it from 0);
// * FORCED FULL — the wallet closed and reopened before the hostile pass: a
//   fresh `Inner`, an unverified memo, the hostile pass from 0.
// and the two verdicts must be IDENTICAL (`describe`), refused, and leave every
// recorded height where the honest pass put it. Every attack keeps index 192 —
// the overlap — honest: an attack that moved it would fall back to a full fetch
// (`roots_overlap_moved`) and the pairing would compare full with full.
//
// The rows above are untouched; these are their incremental twins.

/// Roots per pool in the S15-F1 fixtures: more than 128, so `count − 1 = 199`
/// sits in its own 64-block and the incremental start is 192.
const S15_ROOTS: usize = 200;
/// `floor64(S15_ROOTS − 1)`.
const S15_START: usize = 192;
/// What an incremental pass streams from a script of [`S15_ROOTS`].
const S15_SUFFIX: usize = S15_ROOTS - S15_START;
/// The index the C6 row scans and lies about — inside the suffix, not the
/// overlap.
const S15_C6_INDEX: usize = 195;
/// The interior index the "below the start" row lies about.
const S15_INTERIOR_INDEX: usize = 50;

/// 200 honest Ironwood completion heights: the bundle-proven generator, then
/// extended 2 blocks apart from 10 above the newest bundled row (every bundled
/// row then says "not yet complete" of the extension — `s15_roots`'
/// `ironwood_heights`), and the LAST index 1,000 blocks above the one before it,
/// so a walk down by `WALK_STEP` × `WALK_PASSES` = 792 blocks never reaches
/// index 198 or the block-size floor (IT-10: the recorded arm stays the refuser).
fn s15_ironwood() -> Vec<u64> {
    let mut h: Vec<u64> =
        sync::testing::bind_consistent_heights(Network::Main, ShieldedProtocol::Ironwood)
            .expect("the mainnet Ironwood windows fill")
            .into_iter()
            .map(u64::from)
            .collect();
    h.truncate(S15_ROOTS - 1);
    let above = u64::from(crate::root_bind::newest_bundled_height(Network::Main)) + 10;
    let proven = h.len();
    for i in proven..S15_ROOTS - 1 {
        h.push(above + 2 * (i - proven) as u64);
    }
    let last = h[S15_ROOTS - 2] + 1_000;
    h.push(last);
    assert!(
        last + 500 < u64::from(TIP) - u64::from(REORG_MAX_BLOCKS),
        "FIXTURE: the last root and its inflation sit below the reorg window, so the \
         plan's window term is absent and the start is 192"
    );
    h
}

/// The first `n` honest mainnet Sapling completion heights (every prefix is
/// accepted by the count bind).
fn s15_sapling(n: usize) -> Vec<u64> {
    let all = sync::testing::bind_consistent_heights(Network::Main, ShieldedProtocol::Sapling)
        .expect("the mainnet Sapling windows fill");
    assert!(all.len() >= n, "the bundle proves too few Sapling subtrees");
    all[..n].iter().copied().map(u64::from).collect()
}

/// A wide pool script: `root_hash` distinct per `(pool, index)` at any index
/// (`wide_root_hash` — [`pool_script`]'s one-byte tag overflows past ~170), the
/// completing hash of the honest chain at every height.
fn s15_script(pool: ShieldedProtocol, heights: &[u64]) -> Vec<SubtreeRoot> {
    heights
        .iter()
        .enumerate()
        .map(|(i, &h)| SubtreeRoot {
            root_hash: sync::testing::wide_root_hash(pool, i as u64),
            completing_block_hash: block_id_on(h, false),
            completing_block_height: h,
        })
        .collect()
}

/// One endpoint's three scripts, re-servable.
#[derive(Clone, Default)]
struct S15Scripts {
    sapling: Vec<SubtreeRoot>,
    orchard: Vec<SubtreeRoot>,
    ironwood: Vec<SubtreeRoot>,
    /// Time out the Ironwood OPEN (whatever the start): a fault no retry
    /// absorbs, at a start of 0 or above (§3.3 step 2 — a transport fault at a
    /// non-zero start is retried from 0, a timeout is not).
    ironwood_times_out: bool,
}

impl S15Scripts {
    fn ironwood(heights: &[u64]) -> Self {
        Self {
            ironwood: s15_script(ShieldedProtocol::Ironwood, heights),
            ..Self::default()
        }
    }

    fn endpoint(&self) -> ScriptedEndpoint {
        let mut e = ScriptedEndpoint::new(
            self.sapling.clone(),
            self.orchard.clone(),
            self.ironwood.clone(),
        );
        if self.ironwood_times_out {
            e.ironwood_open_err = Some(GrpcError::Timeout {
                stall: crate::state::StallReason::EndpointUnreachable,
            });
        }
        e
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum S15Mode {
    Incremental,
    ForcedFull,
}

/// What one hostile pass did: the verdict, the arm, and the IT-10 receipt
/// (roots streamed per pool, in `SUBTREE_ROOT_POOLS` order).
struct S15Pass {
    verdict: String,
    refused: bool,
    code: Option<&'static str>,
    streamed: [usize; 3],
}

/// What a whole drive left behind.
struct S15Drive {
    passes: Vec<S15Pass>,
    /// Every pool's recorded heights after the hostile passes.
    after: [Vec<Option<i64>>; 3],
    /// The honest endpoint re-served once more at the end (anti-vacuity).
    honest_again: String,
    honest_again_accepted: bool,
}

/// The honest full pass, `before` (a fixture step on the observer connection),
/// then — on a REOPENED wallet in [`S15Mode::ForcedFull`] — every hostile pass in
/// order, then the honest endpoint once more.
async fn s15_drive(
    mode: S15Mode,
    honest: &S15Scripts,
    before: &dyn Fn(&rusqlite::Connection),
    hostiles: &[S15Scripts],
) -> S15Drive {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);
    let mut first = honest.endpoint();
    let r = w.update_subtree_roots(&mut first, tip(), None).await;
    assert!(
        accepted(&r),
        "{mode:?}: the honest full pass is accepted, got {}",
        describe(&r)
    );
    before(&obs);
    let w = match mode {
        S15Mode::Incremental => w,
        S15Mode::ForcedFull => {
            w.close().await.expect("close before the forced-full pass");
            Wallet::open_with_vault(cfg(dir.path(), Network::Main), Arc::clone(&vault))
                .await
                .expect("reopen: a fresh Inner, an unverified memo")
        }
    };
    let mut passes = Vec::new();
    for hostile in hostiles {
        let mut ep = hostile.endpoint();
        let r = w.update_subtree_roots(&mut ep, tip(), None).await;
        passes.push(S15Pass {
            verdict: describe(&r),
            refused: is_typed_refusal(&r),
            code: refusal_code(&r),
            streamed: [
                ep.streamed(ShieldedProtocol::Sapling),
                ep.streamed(ShieldedProtocol::Orchard),
                ep.streamed(ShieldedProtocol::Ironwood),
            ],
        });
    }
    let after = [
        heights(&obs, "sapling"),
        heights(&obs, "orchard"),
        heights(&obs, "ironwood"),
    ];
    let mut again = honest.endpoint();
    let r = w.update_subtree_roots(&mut again, tip(), None).await;
    let drive = S15Drive {
        passes,
        after,
        honest_again: describe(&r),
        honest_again_accepted: accepted(&r),
    };
    w.close().await.expect("close");
    drive
}

/// Run `hostiles` in both modes and assert what every pair owes: each pass
/// refused in both, with the SAME verdict; the incremental pass streamed only
/// the suffix of every pool `suffix_pools` names (and the forced-full one the
/// whole script); nothing recorded moved; the honest re-serve accepted.
async fn s15_pair(
    name: &str,
    honest: &S15Scripts,
    before: &dyn Fn(&rusqlite::Connection),
    hostiles: &[S15Scripts],
    suffix_pools: &[usize],
) -> (S15Drive, S15Drive) {
    let inc = s15_drive(S15Mode::Incremental, honest, before, hostiles).await;
    let full = s15_drive(S15Mode::ForcedFull, honest, before, hostiles).await;
    let expected_after = [
        expect_heights_of(&honest.sapling),
        expect_heights_of(&honest.orchard),
        expect_heights_of(&honest.ironwood),
    ];
    for (k, (i, f)) in inc.passes.iter().zip(&full.passes).enumerate() {
        assert!(
            i.refused && f.refused,
            "{name} pass {k}: refused in BOTH modes — incremental {} / forced full {}",
            i.verdict,
            f.verdict
        );
        assert_eq!(
            i.verdict, f.verdict,
            "{name} pass {k}: THE SAME VERDICT at start 192 as at start 0. An attack \
             the full fetch refuses and the incremental one does not (or refuses \
             differently) is the bind running on the suffix alone"
        );
        for &slot in suffix_pools {
            let script = [
                &hostiles[k].sapling,
                &hostiles[k].orchard,
                &hostiles[k].ironwood,
            ][slot];
            assert_eq!(
                i.streamed[slot],
                script.len() - S15_START,
                "{name} pass {k} slot {slot}: the incremental pass streamed only the \
                 suffix from 192 — no fallback re-fetched it from 0"
            );
            assert_eq!(
                f.streamed[slot],
                script.len(),
                "{name} pass {k} slot {slot}: the forced-full pass streamed it all"
            );
        }
    }
    for (mode, d) in [("incremental", &inc), ("forced full", &full)] {
        assert_eq!(
            d.after, expected_after,
            "{name} ({mode}): nothing the attack served was written"
        );
        assert!(
            d.honest_again_accepted,
            "{name} ({mode}): ANTI-VACUITY — the honest sequence is still accepted, \
             got {}",
            d.honest_again
        );
    }
    (inc, full)
}

/// The recorded heights a script leaves.
fn expect_heights_of(script: &[SubtreeRoot]) -> Vec<Option<i64>> {
    script
        .iter()
        .map(|r| Some(r.completing_block_height as i64))
        .collect()
}

fn s15_no_fixture(_: &rusqlite::Connection) {}

/// **F2 — C1 at a non-zero start: a compressed suffix.** Two compressions of
/// indices 193..=199 after an honest pass, each keeping the overlap (192)
/// honest: (a) packed into consecutive blocks just above index 192's height —
/// strictly increasing, so the BIND's block-size floor is the refuser
/// (`completion_gap`); (b) the measured attack's own shape, `activation + k` —
/// below index 192's height, so the sequence is not strictly increasing and
/// validation refuses it (an `Err`, as at start 0).
///
/// Mutant (the one the registry watched): the incremental bind skipped at a
/// non-zero start — `check_pool_from` not running `check_heights` when
/// `start > 0` — so (a) is accepted incrementally and refused in full. The
/// call-site mutant "`validate_root_sequence` run on the served suffix only" is
/// NOT killed here: it is unobservable at wallet level, because the re-served
/// overlap root dominates every prefix height (plan §11, ruling 4); the
/// refactored check itself is pinned unit-level by
/// `validate_root_sequence_holds_the_stored_prefix_to_the_tip`.
#[tokio::test]
async fn a_compressed_suffix_is_refused_the_same_at_an_incremental_start() {
    let honest_heights = s15_ironwood();
    let honest = S15Scripts::ironwood(&honest_heights);
    let packed = {
        let mut h = honest_heights.clone();
        for (k, i) in (S15_START + 1..S15_ROOTS).enumerate() {
            h[i] = honest_heights[S15_START] + 1 + k as u64;
        }
        S15Scripts::ironwood(&h)
    };
    let to_activation = {
        let mut h = honest_heights.clone();
        for (k, i) in (S15_START + 1..S15_ROOTS).enumerate() {
            h[i] = nu63_mainnet() + k as u64;
        }
        S15Scripts::ironwood(&h)
    };
    let (inc, _) = s15_pair(
        "compressed suffix",
        &honest,
        &s15_no_fixture,
        &[packed, to_activation],
        &[2],
    )
    .await;
    assert_eq!(
        inc.passes[0].code,
        Some("completion_gap"),
        "(a) WHICH ARM: the packed suffix is refused by the block-size floor across \
         the seam. Got {}",
        inc.passes[0].verdict
    );
    assert!(
        inc.passes[1].verdict.starts_with("Err("),
        "(b) the seam's ordering clause is fatal, as at start 0. Got {}",
        inc.passes[1].verdict
    );
}

/// **F2 — R3 at a non-zero start: the walk down.** The honest pass records 200
/// roots; the endpoint then re-serves them `WALK_PASSES` times, each moving the
/// last index (199) down by `WALK_STEP` — every pass incremental from 192 in the
/// same `Inner` (a refused pool is not written, so the memo stays verified).
/// The recorded arm refuses every step, at 192 exactly as at 0.
///
/// Mutant: the incremental bind skipping the recorded arm for served indices
/// (`check_pool_from` given an empty recorded set above `start`).
#[tokio::test]
async fn a_walk_down_is_refused_the_same_at_an_incremental_start() {
    let honest_heights = s15_ironwood();
    let honest = S15Scripts::ironwood(&honest_heights);
    let walks: Vec<S15Scripts> = (1..=WALK_PASSES)
        .map(|pass| {
            let mut h = honest_heights.clone();
            h[S15_ROOTS - 1] -= WALK_STEP * pass;
            assert!(
                h[S15_ROOTS - 1] > h[S15_ROOTS - 2] + 2,
                "FIXTURE: room to walk"
            );
            S15Scripts::ironwood(&h)
        })
        .collect();
    let (inc, full) = s15_pair("walk down", &honest, &s15_no_fixture, &walks, &[2]).await;
    for (k, (i, f)) in inc.passes.iter().zip(&full.passes).enumerate() {
        assert_eq!(
            (i.code, f.code),
            (Some("recorded_height"), Some("recorded_height")),
            "walk step {k}: the recorded bind refuses the move in both modes. Got {} / {}",
            i.verdict,
            f.verdict
        );
    }
}

/// **F2 — inflation after an honest pass, at a non-zero start.** The last
/// recorded index moved UP by 500 blocks (still below the tip and the reorg
/// window, so no clause of A11 reaches it): only the wallet's own record can
/// refuse it, at 192 exactly as at 0.
///
/// Mutant: the recorded arm comparing only DOWNWARD moves, or only below
/// `start`.
#[tokio::test]
async fn an_inflated_recorded_height_is_refused_the_same_at_an_incremental_start() {
    let honest_heights = s15_ironwood();
    let honest = S15Scripts::ironwood(&honest_heights);
    let mut inflated = honest_heights.clone();
    inflated[S15_ROOTS - 1] += 500;
    let (inc, _) = s15_pair(
        "inflation after honest",
        &honest,
        &s15_no_fixture,
        &[S15Scripts::ironwood(&inflated)],
        &[2],
    )
    .await;
    assert_eq!(
        inc.passes[0].code,
        Some("recorded_height"),
        "WHICH ARM: the record. Got {}",
        inc.passes[0].verdict
    );
}

/// **F2 — C6 at a non-zero start.** The honest pass records 200 roots with
/// nothing scanned; then the completing block of index 195 is SCANNED (a
/// `blocks` row with the honest hash and NULL tree sizes, so the scanned-count
/// oracle abstains — `s15_roots`' fixture of what a scan leaves); then the
/// endpoint serves a DIFFERENT completing hash for 195. Incrementally the plan's
/// term 4 re-serves from 192 and C6 refuses — exactly as the full fetch does.
///
/// Mutants: term 4 dropped (the suffix from 192 is served anyway here — see T8
/// for that one); `check_pool_from` passing the prefix's empty hashes in place
/// of the served ones (C6 abstains incrementally: accepted).
#[tokio::test]
async fn a_completing_hash_contradiction_is_refused_the_same_at_an_incremental_start() {
    let honest_heights = s15_ironwood();
    let honest = S15Scripts::ironwood(&honest_heights);
    let height = honest_heights[S15_C6_INDEX];
    let scan = move |obs: &rusqlite::Connection| {
        obs.execute(
            "INSERT INTO blocks (height, hash, time, sapling_tree) VALUES (?1, ?2, 0, X'')",
            rusqlite::params![height, block_id_on(height, false)],
        )
        .expect("the scanned completing block");
    };
    let mut lie = honest.clone();
    lie.ironwood[S15_C6_INDEX].completing_block_hash = block_id_on(height, true);
    let (inc, _) = s15_pair("C6 contradiction", &honest, &scan, &[lie], &[2]).await;
    assert_eq!(
        inc.passes[0].code,
        Some("completing_block_hash"),
        "WHICH ARM: C6. Got {}",
        inc.passes[0].verdict
    );
}

/// **F2 — C16 at a non-zero start: a fault on a later pool leaves the bind
/// unchanged.** Sapling and Ironwood both hold 200 honest roots. Pass 1 grows
/// Sapling to 201 and TIMES OUT Ironwood's open (a timeout: a transport fault at
/// a non-zero start is retried from 0 and absorbed, by design — §3.3 step 2 —
/// so the original row's `Transport` would not fault the incremental pass at
/// all); nothing is written. Pass 2 compresses Sapling's suffix (packed above
/// index 192): still refused — the failed pass erased nothing. Both modes, the
/// same verdicts.
///
/// Mutants: a failed pass that writes the earlier pool's suffix (index 200
/// appears); a failed pass that resets the memo or the bind (pass 2 accepted).
#[tokio::test]
async fn a_fault_on_a_later_pool_leaves_the_bind_unchanged_at_an_incremental_start() {
    let sapling = s15_sapling(S15_ROOTS + 1);
    let ironwood = s15_ironwood();
    let honest = S15Scripts {
        sapling: s15_script(ShieldedProtocol::Sapling, &sapling[..S15_ROOTS]),
        ironwood: s15_script(ShieldedProtocol::Ironwood, &ironwood),
        ..S15Scripts::default()
    };
    let faulting = S15Scripts {
        sapling: s15_script(ShieldedProtocol::Sapling, &sapling),
        ironwood_times_out: true,
        ..honest.clone()
    };
    let compressor = {
        let mut h = sapling[..S15_ROOTS].to_vec();
        for (k, i) in (S15_START + 1..S15_ROOTS).enumerate() {
            h[i] = sapling[S15_START] + 1 + k as u64;
        }
        S15Scripts {
            sapling: s15_script(ShieldedProtocol::Sapling, &h),
            ..honest.clone()
        }
    };
    let (inc, full) = s15_pair(
        "fault on a later pool",
        &honest,
        &s15_no_fixture,
        &[faulting, compressor],
        &[0],
    )
    .await;
    for (mode, d) in [("incremental", &inc), ("forced full", &full)] {
        assert!(
            d.passes[0].verdict.starts_with("Err("),
            "{mode}: the timeout on the last pool is fatal to the pass. Got {}",
            d.passes[0].verdict
        );
        assert_eq!(
            d.passes[0].streamed[2], 0,
            "{mode}: Ironwood's open timed out"
        );
        assert!(
            d.passes[1].verdict.starts_with("Err("),
            "{mode}: NOT ERASED — the Sapling compression is still refused. Got {}",
            d.passes[1].verdict
        );
    }
}

/// **F2 — a lie only at an INTERIOR index below the start** (ledger row 7: the
/// one place the incremental pass does not look). The endpoint moves index 50
/// up by one block. The incremental pass (from 192, 8 roots) cannot see it and
/// must not write it: accepted, row 50 unchanged. The next FULL verification —
/// here a fresh `Inner` — serves index 50 and refuses it (the block-size floor:
/// index 51 is now one block above it); still nothing written.
///
/// Mutants: an incremental put that rewrites the prefix it did not serve (row
/// 50 moves); a reopen that carries the memo (the second pass is incremental
/// and accepted).
#[tokio::test]
async fn an_interior_lie_below_the_start_waits_for_the_next_full_verification() {
    let honest_heights = s15_ironwood();
    let mut lied = honest_heights.clone();
    lied[S15_INTERIOR_INDEX] += 1;
    assert_eq!(
        honest_heights[S15_INTERIOR_INDEX + 1] - lied[S15_INTERIOR_INDEX],
        1,
        "FIXTURE: index 50 is in the 2-apart extension, so the lie leaves a 1-block gap"
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);
    let mut honest = S15Scripts::ironwood(&honest_heights).endpoint();
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(accepted(&r), "the honest full pass, got {}", describe(&r));

    let mut liar = S15Scripts::ironwood(&lied).endpoint();
    let r = w.update_subtree_roots(&mut liar, tip(), None).await;
    assert_eq!(
        liar.streamed(ShieldedProtocol::Ironwood),
        S15_SUFFIX,
        "the incremental pass asked from 192"
    );
    assert!(
        accepted(&r),
        "the incremental pass never sees index 50 — accepted, got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&honest_heights),
        "and wrote nothing below its start: index 50 is the honest height"
    );

    w.close().await.expect("close");
    let w = Wallet::open_with_vault(cfg(dir.path(), Network::Main), Arc::clone(&vault))
        .await
        .expect("reopen: the next full verification");
    let mut liar = S15Scripts::ironwood(&lied).endpoint();
    let r = w.update_subtree_roots(&mut liar, tip(), None).await;
    assert_eq!(
        liar.streamed(ShieldedProtocol::Ironwood),
        S15_ROOTS,
        "a fresh Inner fetches from 0"
    );
    assert_eq!(
        refusal_code(&r),
        Some("completion_gap"),
        "the full verification refuses the interior lie. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&honest_heights),
        "and still nothing written"
    );
    w.close().await.expect("close");
}

/// **Ledger row 7 IN-SESSION (the final fold, crypto LOW)** — the interior lie
/// of [`an_interior_lie_below_the_start_waits_for_the_next_full_verification`],
/// refused by the TIME-DUE full verification of the SAME `Inner` (no reopen).
/// On an injected clock: the honest full pass; then the lie at index 50, served
/// incrementally from 192 and accepted (nothing written below the start) on the
/// next pass and again at `SUBTREE_ROOTS_FULL_VERIFY_SECS − 1`; at
/// `SUBTREE_ROOTS_FULL_VERIFY_SECS` the same session asks from 0 and refuses it
/// (`completion_gap`), still writing nothing. No existing row proves this on a
/// lie: the T11 rows (`the_full_verification_runs_every_n_passes` / `…_t_seconds`)
/// time the boundary on an honest endpoint only.
///
/// Mutant: the time-due full verification never firing (`TimeDue`'s elapsed
/// clause dropped) — the due pass stays incremental and accepts the lie.
#[tokio::test]
async fn an_interior_lie_below_the_start_is_refused_by_the_time_due_full_verification() {
    use crate::constants::SUBTREE_ROOTS_FULL_VERIFY_SECS;
    const T0: u64 = 1_757_000_000;
    let honest_heights = s15_ironwood();
    let mut lied = honest_heights.clone();
    lied[S15_INTERIOR_INDEX] += 1;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let (w, key) = wallet_and_key(dir.path(), &vault, Network::Main).await;
    let obs = observe(dir.path(), &key);
    w.close()
        .await
        .expect("close, to reopen on the injected clock");
    let clock = sync::testing::ManualClock::at(T0);
    let w = Wallet::open_with_vault_and_seed_port(
        cfg(dir.path(), Network::Main),
        Arc::clone(&vault),
        None,
        Some(clock.port()),
    )
    .await
    .expect("reopen on the injected clock");
    let mut honest = S15Scripts::ironwood(&honest_heights).endpoint();
    let r = w.update_subtree_roots(&mut honest, tip(), None).await;
    assert!(accepted(&r), "the honest full pass, got {}", describe(&r));

    for (secs, what) in [
        (0, "the next pass"),
        (SUBTREE_ROOTS_FULL_VERIFY_SECS - 1, "one second before due"),
    ] {
        clock.set(T0 + secs);
        let mut liar = S15Scripts::ironwood(&lied).endpoint();
        let r = w.update_subtree_roots(&mut liar, tip(), None).await;
        assert_eq!(
            liar.streamed(ShieldedProtocol::Ironwood),
            S15_SUFFIX,
            "{what}: incremental from 192"
        );
        assert!(
            accepted(&r),
            "{what}: the incremental pass never sees index 50, got {}",
            describe(&r)
        );
    }
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&honest_heights),
        "nothing written below the start"
    );

    clock.set(T0 + SUBTREE_ROOTS_FULL_VERIFY_SECS);
    let mut liar = S15Scripts::ironwood(&lied).endpoint();
    let r = w.update_subtree_roots(&mut liar, tip(), None).await;
    assert_eq!(
        liar.streamed(ShieldedProtocol::Ironwood),
        S15_ROOTS,
        "the time-due pass of the SAME Inner fetches from 0"
    );
    assert_eq!(
        refusal_code(&r),
        Some("completion_gap"),
        "the in-session full verification refuses the interior lie. Got {}",
        describe(&r)
    );
    assert_eq!(
        heights(&obs, "ironwood"),
        expect_heights(&honest_heights),
        "and still nothing written"
    );
    w.close().await.expect("close");
}
