//! **T0-1 A6 + A6b — the end-to-end assertion, and it is the one that matters.**
//!
//! Contract: `docs/plan/production-readiness-phase-1.md` §4b rows A6 and A6b, owed
//! since an earlier revision as §4b owed rows 1 and 5, and the last thing between `INC-020` and
//! `GUARDED` — the registry row's own clause (d) says it *"may NOT go GUARDED until
//! the first of them lands"*.
//!
//! ## The module name, and the sentence that pointed here before it existed
//!
//! `sync.rs`'s A6 comment once sent the reader to *"`mod ironwood_spendability`
//! below"*, and the adjudicator recorded that the module **did not exist
//! anywhere in the repository** — a tree claiming coverage nobody had written, on
//! the one assertion the contract calls the one that matters. The comment was
//! corrected to say the half was OWED. This file is that half, and it takes the name
//! so the original sentence becomes true rather than merely retracted.
//!
//! ## What was blocking it, and what unblocked it
//!
//! A6 needs an Ironwood note ABOVE a completed subtree whose own 65,536 leaves the
//! wallet has never scanned — the mid-restore shape `INC-020` is about, where the
//! subtree's root can only come from the endpoint. The pinned harness cannot build
//! it: `zcash_client_backend-0.24.0`'s `InitialChainState`
//! (`data_api/testing.rs:1711-1719`) carries `prior_sapling_roots` and
//! `prior_orchard_roots` and **no `prior_ironwood_roots`**, and `TestBuilder::build`
//! writes only those two and inserts only those two frontiers (`:2051-2103`) — the
//! Ironwood tree is left EMPTY while the cached block records its size. Building the
//! pair by hand needs `incrementalmerkletree::frontier::Frontier`, which sat in the
//! lock transitively and in no manifest. The maintainer declared it as a dev-dependency
//! on 2026-09-12; this module is what that decision bought.
//!
//! ## The two halves, and why A6 alone is not enough
//!
//! A6 measures spendability through the same SQL the root write already executed, so
//! passing it again from Rust proves little on its own. **A6b is the assertion a
//! SQL-level fix cannot fake**: the witness/anchor path is a DIFFERENT mechanism —
//! `put_shard_roots` also writes the shardtree CAP (`zcash_client_sqlite-0.22.0
//! wallet/commitment_tree.rs:1269-1283`, `cap.batch_insert` then `put_cap`),
//! `put_shard` never touches the cap, and `ShardTree::root` traverses it
//! (`shardtree-0.7.1 lib.rs:953`). Two parties once confirmed the opposite premise by
//! comparing the two per-shard `INSERT`s; the relevant write was twenty lines above
//! what they compared (§4b, "A6/A6b are OWED").
//!
//! **That premise did not survive being built — read this paragraph as the contract
//! as written, not as what is true.** The CAP fact holds, the conclusion does not: a
//! `ChainState` frontier carries the completed subtree's root as an ommer, so the
//! witness and the anchor are constructible BEFORE any root is served. A6b's
//! assertion is retired; the anchor row below
//! (`the_anchor_comes_from_the_frontiers_ommers_and_not_from_the_subtree_roots`)
//! records what is actually true, and the SQL predicate — which IS what the roots
//! buy — has its own rows (the third mechanism, below).
//!
//! ## What the fixtures drive, so nobody has to infer it
//!
//! The roots go in through **`crate::sync::put_subtree_roots`** — the SDK's own write,
//! the one `INC-020` says had no caller — rather than through upstream's
//! `put_ironwood_subtree_roots` directly, so the wiring under test is ours.
//!
//! ## Three mechanisms in one module, and why
//!
//! Two kinds of fixture live here, and the reader should know which row runs on
//! which before believing either:
//!
//! 1. **The funded `data_api::testing` harness** (`build_fixture`, the first three
//!    rows): a real scan, a real note, a real shardtree — the tree-state half.
//!    Its 66,770-leaf tree cannot complete the note's OWN shard, so it cannot reach
//!    the SQL predicate's deciding geometry.
//! 2. **A modelled mainnet wallet** (`build_modelled_wallet`, the S3 and S2 rows): the
//!    crate's real migrated schema, the crate's own SQL sliced from the pinned
//!    source, and rows written by hand — the SQL predicate half, in the two
//!    geometries where `subtree_end_height` decides. The maintainer ruled this route at
//!    the wrap (decision 1) because the harness cannot produce it.
//! 3. **The probe** (`docs/plan/probes/ironwood-unspendability-repro.py`) stays as
//!    provenance — the matrix the S3/S2 rows port — and is not a guard (`check.py`
//!    refuses a `.py`).
//!
//! They live in ONE module because they are one incident's guard (`INC-020` clause
//! (d)), and the registry names this file. What each does NOT prove is stated on the
//! row: the harness rows say nothing about selection; the modelled rows say nothing
//! about witnesses, and their wallet is plaintext (no SQLCipher key path) with stub
//! key material — the SQL never decodes a note.

#![cfg(test)]

use incrementalmerkletree::frontier::Frontier;
use incrementalmerkletree::{Hashable, Level, Marking};
use orchard::tree::MerkleHashOrchard;
use rand_core::OsRng;
use rusqlite::types::Value;
use rusqlite::{Connection, named_params, params};
use shardtree::error::ShardTreeError;
use std::num::NonZeroU8;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use zcash_client_backend::data_api::chain::{ChainState, CommitmentTreeRoot};
use zcash_client_backend::data_api::testing::{
    AddressType, InitialChainState, IronwoodFvk, TestBuilder,
};
use zcash_client_backend::data_api::{IRONWOOD_SHARD_HEIGHT, WalletCommitmentTrees, WalletWrite};
use zcash_client_sqlite::WalletDb;
use zcash_client_sqlite::util::SystemClock;
use zcash_client_sqlite::wallet::commitment_tree;
use zcash_client_sqlite::wallet::init::init_wallet_db;
use zcash_primitives::block::BlockHash;
use zcash_protocol::consensus::{BlockHeight, Network, NetworkUpgrade, Parameters};
use zcash_protocol::local_consensus::LocalNetwork;
use zcash_protocol::value::Zatoshis as ProtoZat;

use crate::sync::SubtreeRoots;
use crate::test_support::{FileBackedDsf, HarnessState, MemCache};

/// The Ironwood tree size at the wallet's birthday: **one COMPLETE subtree plus a
/// partial one**. The completed subtree is the thing only an endpoint can supply —
/// the wallet never scans its leaves — so every note the wallet then receives sits
/// above a gap that only the roots can fill. `1234` is upstream's own choice for the
/// same shape (`data_api/testing/pool.rs:3867`).
const FRONTIER_TREE_SIZE: u64 = (1 << 16) + 1234;

/// Untrusted-incoming spendable depth (`constants::MIN_CONFIRMATIONS`'s conservative
/// arm) — a mined note must reach it to be selectable under the default policy.
const SPENDABLE_DEPTH: usize = 10;

const NOTE_VALUE: u64 = 500_000;

/// How far the chain tip runs ahead of what the wallet has scanned. Any positive
/// number produces the `ChainTip` range; 20 is comfortably more than one block so
/// the range is unmistakable in a failure dump.
const CHAIN_TIP_LAG_BLOCKS: usize = 20;

/// The harness network with **Ironwood ACTIVE**. `TestBuilder::DEFAULT_NETWORK` leaves
/// `nu6_3` — and `nu6`/`nu6_1`/`nu6_2` — at `None`, which is why every other Ironwood
/// fixture in this crate runs on a chain where the pool has no activation height at
/// all. Every upgrade from Sapling on lands at ONE height here, so the whole fixture
/// chain is post-Ironwood and nothing in it straddles an upgrade boundary.
fn ironwood_network() -> LocalNetwork {
    // Built by UPDATING upstream's own default rather than respelling it: the default
    // has an unstable `nu7` field behind a `cfg` this build does not set, and naming
    // every field here would both trip `unexpected_cfgs` and silently stop tracking a
    // future upgrade upstream adds.
    let h = TestBuilder::<(), ()>::DEFAULT_NETWORK.nu5;
    LocalNetwork {
        nu6: h,
        nu6_1: h,
        nu6_2: h,
        nu6_3: h,
        ..TestBuilder::<(), ()>::DEFAULT_NETWORK
    }
}

/// A wallet whose birthday sits above ONE completed Ironwood subtree, holding a
/// scanned, confirmed Ironwood note above it — and **whose Ironwood subtree roots
/// have NOT been written**. That is the mid-restore state `INC-020` names.
struct Fixture {
    state: HarnessState,
    /// The prior Ironwood subtree roots in the shape [`crate::sync::put_subtree_roots`]
    /// takes them — index 0 upward. What an honest endpoint would serve.
    prior_roots: Vec<CommitmentTreeRoot<MerkleHashOrchard>>,
    /// The checkpoint the witness is taken against. **Read from the tree the wallet
    /// built, not assumed to be the tip.** `put_blocks` UNIONS checkpoint heights
    /// across all three pools (`zcash_client_backend-0.24.0 data_api/ll/wallet.rs:687-731`,
    /// `batch_ensure_heights`) plus the unconditional anchor checkpoint — so the nine
    /// empty blocks after the note carry no Ironwood checkpoint because NO pool moved
    /// in them, not because Ironwood is checkpointed sparsely. (An earlier version of
    /// this sentence said "only where THAT tree moved"; the crypto audit corrected it,
    /// and the wrong reading is what predicted sparse Ironwood checkpoints and hid the
    /// `ResetToSubtreeRoots` geometry.) `root_at_checkpoint_id(tip)` therefore
    /// answers `None` for a reason that has nothing to do with the subtree roots. The
    /// first cut of this file asserted against the tip and both rows went red with
    /// `None` on BOTH sides of the write — an instrument fault that reads exactly like
    /// the defect.
    checkpoint: BlockHeight,
    path: PathBuf,
    _dir: tempfile::TempDir,
}

fn build_fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("wallet.db");
    let network = ironwood_network();

    // The (prior_roots, frontier) pair, generated CONSISTENTLY by upstream's own
    // generator — the one `InitialChainState` has no Ironwood field to carry. A
    // hand-picked pair would be a tree that does not hash, and the witness assertion
    // below would then be measuring the fixture rather than the wiring.
    let mut rng = OsRng;
    let (prior_root_hashes, ironwood_initial_tree): (Vec<MerkleHashOrchard>, _) =
        Frontier::random_with_prior_subtree_roots(
            &mut rng,
            FRONTIER_TREE_SIZE,
            NonZeroU8::new(IRONWOOD_SHARD_HEIGHT).expect("the shard height is non-zero"),
        );
    assert_eq!(
        prior_root_hashes.len(),
        1,
        "the fixture's whole point is ONE completed subtree below the note; a \
         different count means FRONTIER_TREE_SIZE no longer straddles one boundary"
    );

    let birthday_height = network
        .activation_height(NetworkUpgrade::Nu6_3)
        .expect("the fixture network activates Ironwood")
        + 1_000;
    // The completed subtree's end height sits far BELOW the birthday, so the wallet
    // has never scanned a block inside it and cannot derive its root for itself.
    let subtree_end = birthday_height - 500;
    let prior_roots: Vec<_> = prior_root_hashes
        .into_iter()
        .map(|root| CommitmentTreeRoot::from_parts(subtree_end, root))
        .collect();

    let frontier_for_insert = ironwood_initial_tree.clone();
    let mut state = TestBuilder::new()
        .with_network(network)
        .with_data_store_factory(FileBackedDsf { path: path.clone() })
        .with_block_cache(MemCache::default())
        .with_initial_chain_state(move |_rng, _net| InitialChainState {
            chain_state: ChainState::new(
                birthday_height - 1,
                BlockHash([7; 32]),
                Frontier::empty(),
                Frontier::empty(),
                ironwood_initial_tree,
            ),
            prior_sapling_roots: vec![],
            prior_orchard_roots: vec![],
        })
        .with_account_having_current_birthday()
        .build();

    // **The half `TestBuilder::build` cannot do**, and the reason this module needed
    // the dev-dep at all: `build` inserts the Sapling and Orchard frontiers into their
    // trees and has no Ironwood arm, so without this the Ironwood tree is EMPTY and
    // the note below would land at position 0 of an empty tree — spendable with no
    // roots at all, which is the fixture that proves nothing. The subtree ROOTS are
    // deliberately NOT written here; writing them is what the tests do.
    state
        .wallet_mut()
        .with_ironwood_tree_mut::<_, (), ShardTreeError<commitment_tree::Error>>(|tree| {
            tree.insert_frontier(
                frontier_for_insert.clone(),
                incrementalmerkletree::Retention::Checkpoint {
                    id: birthday_height - 1,
                    marking: Marking::Reference,
                },
            )
            .expect("the frontier inserts into an empty ironwood tree");
            Ok(())
        })
        .expect("our store overrides the ironwood tree accessor")
        .expect("the accessor reaches a real tree — see `our_store_really_overrides_the_ironwood_tree_accessor`");

    // One Ironwood note to this account, then enough empty blocks to clear the
    // spendable depth, then scan the whole range.
    let fvk = IronwoodFvk(
        state
            .test_account_orchard()
            .cloned()
            .expect("the test account has an orchard key"),
    );
    let (note_height, _res, _nf) = state.generate_next_block(
        &fvk,
        AddressType::DefaultExternal,
        ProtoZat::const_from_u64(NOTE_VALUE),
    );
    // Enough blocks to clear the spendable depth, SCANNED...
    for _ in 0..(SPENDABLE_DEPTH - 1) {
        state.generate_empty_block();
    }
    state.scan_cached_blocks(note_height, SPENDABLE_DEPTH);

    // ...and then MORE blocks that are NOT scanned, with the chain tip set above
    // them. **This is the geometry, and getting it wrong is what made the first cut
    // of this file measure nothing.** `update_chain_tip` inserts a `ChainTip`-priority
    // scan range for everything above what has been scanned
    // (`zcash_client_sqlite-0.22.0 wallet/scanning.rs:689-695`), which is the steady
    // state of every live wallet — the newest blocks are always queued and not yet
    // scanned. The repo's own probe calls it S3, "the ordinary chain-tip lag"
    // (`docs/plan/probes/ironwood-unspendability-repro.py`), and it is the cheapest
    // geometry in which the subtree roots are load-bearing.
    //
    // A6's wording is what asks for this: *"an Ironwood note over a post-activation
    // scan range that is NOT `Scanned`"*. The first cut of this file scanned every
    // block it generated and then set the tip to the last of them, leaving NO
    // unscanned range — the probe's scenario S1, the one row of its matrix where the
    // roots make no difference at all.
    for _ in 0..CHAIN_TIP_LAG_BLOCKS {
        state.generate_empty_block();
    }
    let tip =
        note_height + u32::try_from(SPENDABLE_DEPTH - 1 + CHAIN_TIP_LAG_BLOCKS).expect("small");
    state
        .wallet_mut()
        .update_chain_tip(tip)
        .expect("update chain tip");

    // The NEWEST Ironwood checkpoint the scan actually left, read back rather than
    // assumed — see the `checkpoint` field's doc for what assuming it cost.
    let checkpoint = {
        use zcash_client_backend::data_api::WalletTest;
        let history = state
            .wallet()
            .get_checkpoint_history(&zcash_protocol::ShieldedPool::Ironwood)
            .expect("reading our own checkpoint history is not a fault");
        let newest = history
            .iter()
            .map(|(h, _)| *h)
            .max()
            .expect("the scan left at least one Ironwood checkpoint");
        assert!(
            newest >= note_height,
            "the newest Ironwood checkpoint ({newest:?}) is BELOW the note's own block \
             ({note_height:?}) — the scan did not checkpoint the block that moved the \
             tree, and no witness for this note could exist at any checkpoint"
        );
        newest
    };

    Fixture {
        state,
        prior_roots,
        checkpoint,
        path,
        _dir: dir,
    }
}

/// The Ironwood note's tree position, read from the row the WALLET wrote rather than
/// computed — a computed position would make this file assert against its own
/// arithmetic. A second connection on the harness file, which is the production aux
/// pattern (`test_support::HistoryHarness::read_conn`).
fn ironwood_note_position(f: &Fixture) -> incrementalmerkletree::Position {
    let conn = Connection::open(&f.path).expect("a second read connection");
    let mut stmt = conn
        .prepare("SELECT commitment_tree_position FROM ironwood_received_notes")
        .expect("the ironwood receive table exists");
    let positions: Vec<u64> = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .expect("query")
        .map(|r| u64::try_from(r.expect("row")).expect("a position is non-negative"))
        .collect();
    assert_eq!(
        positions.len(),
        1,
        "the fixture mines exactly one Ironwood note and the wallet must have \
         DECRYPTED it; {} rows means the note never arrived and every assertion \
         about it would be vacuous",
        positions.len()
    );
    assert!(
        positions[0] >= FRONTIER_TREE_SIZE,
        "the note must sit ABOVE the completed subtree — position {} is inside or \
         below the frontier at {FRONTIER_TREE_SIZE}, which is the geometry where no \
         subtree root is needed and the whole item is unexercised",
        positions[0]
    );
    incrementalmerkletree::Position::from(positions[0])
}

/// Can the wallet build a witness for the note at `position`, as of the recorded tip?
/// This is the exact pair `create_proposed_transactions` runs inside
/// `with_ironwood_tree_mut` (`zcash_client_backend-0.24.0 data_api/wallet.rs:1858-1899`):
/// the checkpoint root — the ANCHOR the proof commits to — and the Merkle path.
fn anchor_and_witness(
    f: &mut Fixture,
    position: incrementalmerkletree::Position,
) -> (Option<MerkleHashOrchard>, bool) {
    f.state
        .wallet_mut()
        .with_ironwood_tree_mut::<_, _, ShardTreeError<commitment_tree::Error>>(|tree| {
            let anchor = tree.root_at_checkpoint_id(&f.checkpoint).ok().flatten();
            let witness = tree
                .witness_at_checkpoint_id_caching(position, &f.checkpoint)
                .ok()
                .flatten()
                .is_some();
            Ok((anchor, witness))
        })
        .expect("reading our own ironwood tree is not a fault")
        .expect("the accessor reaches a real tree")
}

/// Write the fixture's Ironwood subtree roots through **the SDK's own**
/// [`crate::sync::put_subtree_roots`] — the wrapper `INC-020` records as having had no
/// caller anywhere in the tree. `CommitmentTreeRoot` is not `Clone`, so the pair is
/// rebuilt from its own parts rather than copied.
fn write_the_roots(f: &mut Fixture) {
    let ironwood: Vec<_> = f
        .prior_roots
        .iter()
        .map(|r| CommitmentTreeRoot::from_parts(r.subtree_end_height(), *r.root_hash()))
        .collect();
    let roots = SubtreeRoots::from_pools(Vec::new(), Vec::new(), ironwood);
    crate::sync::put_subtree_roots(f.state.wallet_mut(), &roots, |_| Ok(()))
        .expect("the SDK's own subtree-root write");
}

/// **The Ironwood subtree roots this wallet was served reach its real tree, and they
/// are readable back out of it.** The write is [`crate::sync::put_subtree_roots`] —
/// the SDK's own wrapper, which `INC-020` recorded as having *"no caller anywhere in
/// the tree"* — driven against the funded `data_api::testing` harness rather than a
/// bare `Connection`, so what the row exercises is the wiring and not the SQL.
///
/// This is A6's write half, and it is the half that holds. A6's OTHER half — that the
/// note above the subtree is unspendable until the roots land — does not, and the
/// measurement is the sibling row below.
///
/// Mutant: delete the `db.put_ironwood_subtree_roots(0, ironwood)` line in
/// [`crate::sync::put_subtree_roots`] → the AFTER read goes red with `None`, and
/// neither the Sapling nor the Orchard arm can mask it because the fixture serves
/// those pools nothing.
#[test]
fn the_ironwood_subtree_roots_the_sdk_writes_reach_the_wallets_own_tree() {
    let mut f = build_fixture();
    // The note must exist and sit above the completed subtree, or the fixture is not
    // the geometry this file is about.
    let _position = ironwood_note_position(&f);

    let before = f
        .state
        .wallet_mut()
        .get_ironwood_subtree_root(0)
        .expect("reading our own subtree-root table is not a fault");
    assert!(
        before.is_none(),
        "the fixture must start with NO Ironwood subtree root written — that is \
         INC-020's state, and a fixture that starts with one measures nothing"
    );

    write_the_roots(&mut f);

    let after = f
        .state
        .wallet_mut()
        .get_ironwood_subtree_root(0)
        .expect("reading our own subtree-root table is not a fault");
    assert_eq!(
        after.as_ref(),
        Some(f.prior_roots[0].root_hash()),
        "the root the SDK wrote is not the root the wallet reads back. INC-020's \
         (b)/(c) cross-wire class: `SubtreeRoots.orchard` and `.ironwood` have the \
         IDENTICAL Rust type, so a swapped arm compiles and a value comparison is the \
         only thing that sees it"
    );
}

/// **A6 — the geometry where the subtree roots are the SOLE survivor, and the note
/// is lost without them.**
///
/// A `ResetToSubtreeRoots` truncation (`zcash_client_sqlite-0.22.0
/// wallet/commitment_tree.rs:730-793`) runs
/// `DELETE FROM {p}_tree_checkpoints; …_tree_shards; …_tree_cap` and rebuilds the
/// whole tree from **only** those rows with `subtree_end_height IS NOT NULL AND
/// root_hash IS NOT NULL`. `put_shard` writes `root_hash` and `shard_data` and
/// **never** `subtree_end_height`; only `put_shard_roots` does — reached only from
/// the `put_*_subtree_roots` calls in [`crate::sync::put_subtree_roots`]. So every
/// scanned shard and the frontier's own cap contribution are discarded whole, and
/// what is left is exactly what the endpoint served and the wallet wrote.
///
/// `plan_tree_truncation` (`wallet.rs:4247-4267`) returns `ResetToSubtreeRoots` for
/// "no checkpoint at the target, some above, none below" — which upstream's own
/// comment calls *"a pool whose post-migration rescan has so far only reached blocks
/// near the chain tip"*, i.e. **Ironwood after NU6.3**, the population INC-020 is
/// about. The SDK reaches `truncate_to_height` from `sync::rewind_wallet_to` on every
/// reorg.
///
/// **This is the row that makes INC-020's "never clears" concrete**: one reorg on a
/// tip-only Ironwood tree, and with no subtree roots written the wallet has no tree
/// to witness the note from and no way to rebuild one short of a full rescan.
///
/// # What this row is NOT, stated because the first cut of this file got it wrong
///
/// It is not the SQL spendability predicate. That is
/// `witness_stabilized = 1 OR (tip_unscanned = 0 AND max_priority <= Scanned)`
/// (`zcash_client_sqlite-0.22.0 wallet/common.rs:779-785`), both branches gated on
/// `subtree_end_height`, and it is measured at level X by the probe already in the
/// tree — `docs/plan/probes/ironwood-unspendability-repro.py`, whose scenario S3
/// ("the ordinary chain-tip lag") reproduces A6 verbatim with zero blocks rescanned.
/// A Rust row for it needs the note's OWN shard complete and its root served, which
/// THIS fixture's 66,770-leaf tree cannot honestly produce — so that row is NOT on
/// this fixture. It is
/// [`in_s3_the_sdks_root_write_alone_makes_the_note_selectable_with_zero_blocks_rescanned`]
/// below, on the real mainnet schema with the crate's own SQL, the way the maintainer
/// ruled at the wrap (decision 1: port the probe's S3 cell, not the harness).
///
/// Mutant: delete `db.put_ironwood_subtree_roots(0, ironwood)` from
/// [`crate::sync::put_subtree_roots`] → the with-roots arm loses its shard row too
/// and the assertion that separates the two arms goes red.
#[test]
fn a_reset_to_subtree_roots_truncation_keeps_only_what_the_roots_wrote() {
    // Both arms, same fixture, same truncation — the ONLY difference is whether the
    // subtree roots were written first.
    let mut surviving_shards = Vec::new();
    for write_roots in [false, true] {
        let mut f = build_fixture();
        let _position = ironwood_note_position(&f);
        if write_roots {
            write_the_roots(&mut f);
        }

        // Upstream's own fixture move for this classification: with no Ironwood
        // checkpoint at or below the target and some above, `plan_tree_truncation`
        // answers `ResetToSubtreeRoots`
        // (`truncate_to_height_with_tip_only_ironwood_tree_empties_it`,
        // `zcash_client_sqlite-0.22.0 wallet.rs:7416`).
        let target = f.checkpoint;
        {
            let conn = Connection::open(&f.path).expect("a second connection");
            conn.execute(
                "DELETE FROM ironwood_tree_checkpoints WHERE checkpoint_id <= ?1",
                [u32::from(target)],
            )
            .expect("the checkpoint delete");
        }
        f.state
            .wallet_mut()
            .truncate_to_height(target)
            .expect("the truncation itself is not a fault");

        let conn = Connection::open(&f.path).expect("a second connection");
        // `truncate_tree_to_subtree_roots`' OWN survivor predicate, verbatim
        // (`zcash_client_sqlite-0.22.0 wallet/commitment_tree.rs:730-793`): these are
        // the rows a `ResetToSubtreeRoots` rebuild is allowed to keep. Counting them
        // is the honest measurement — stronger than counting shard rows, which a
        // `put_shard` row inflates without being rebuildable from.
        let rebuildable: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM ironwood_tree_shards
                 WHERE subtree_end_height IS NOT NULL AND root_hash IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .expect("count");
        surviving_shards.push(rebuildable);
    }

    assert_eq!(
        surviving_shards[0], 0,
        "WITHOUT the subtree roots this wallet has NOTHING a `ResetToSubtreeRoots` \
         rebuild could keep. Measured after a truncation: every Ironwood shard row it \
         holds was written by `put_shard`, which never sets `subtree_end_height`, so \
         none of them meets the survivor predicate. That is the state INC-020 names — \
         one reorg classified `ResetToSubtreeRoots` and the tree is gone with no way \
         back but a full rescan"
    );
    assert!(
        surviving_shards[1] > 0,
        "WITH the subtree roots written the wallet STILL has nothing rebuildable. The \
         roots are the only writer of `subtree_end_height` and therefore the only \
         thing that survives a reset — if they do not survive, the whole write is \
         pointless"
    );
}

/// **A6b, RE-SCOPED BY MEASUREMENT — the anchor is NOT what the subtree roots buy.**
///
/// A6b was contracted on the premise that the witness/anchor half is a different
/// mechanism a SQL-level fix cannot fake, because `put_shard_roots` writes the
/// shardtree CAP and `put_shard` never touches it. The CAP fact is true. **The
/// conclusion drawn from it is not:** a `ChainState` frontier carries the ommers
/// along its whole left-hand path, and one of those ommers IS the root of the
/// completed subtree below the note, so the witness and the anchor are already
/// constructible before any subtree root is served. `sync::scan_batch` passes a
/// `from_state` on every batch, so the production path always has one.
///
/// Driven in four geometries, all four with a witness and an anchor BEFORE the write:
/// the birthday frontier inserted by hand; that insert skipped (`scan_cached_blocks`
/// inserts the `from_state` frontier itself); `truncate_to_height` rewinds of 1, 3
/// and 5 blocks with the roots absent then a re-scan; and the same rewinds with the
/// roots present.
///
/// **This does not weaken A6** — the sibling row above — and reading it as though it
/// did is the error this file shipped once. Spendability is decided in SQL by
/// `witness_stabilized` and the shard scan state, not by whether a Merkle path can be
/// computed. A6b's own assertion is therefore retired and replaced by this statement
/// of what is actually true.
///
/// **No mutant row, on purpose, and it is the one banked test in this module.** The
/// row records a measured property of upstream's frontier (the ommers carry the
/// completed subtree's root), not a behaviour of our code, so there is no line of
/// ours whose deletion it should catch: writing the roots at index 1 instead of 0
/// was tried at the review fold and leaves it green exactly as the doc
/// predicts, while reddening the read-back row. Its one clause with teeth — the
/// anchor must not MOVE across the write — would fire on a write that corrupts the
/// cap, which no mutant of `put_subtree_roots` can produce short of editing the
/// pinned crate. It is carried by the guard ratchet as debt rather than given a
/// row it did not earn.
#[test]
fn the_anchor_comes_from_the_frontiers_ommers_and_not_from_the_subtree_roots() {
    let mut f = build_fixture();
    let position = ironwood_note_position(&f);

    let (anchor_before, witness_before) = anchor_and_witness(&mut f, position);
    assert!(
        witness_before && anchor_before.is_some(),
        "no witness or anchor before the roots — if this fires, A6b's original \
         premise was right after all and this row must be replaced by it"
    );

    write_the_roots(&mut f);

    let (anchor_after, witness_after) = anchor_and_witness(&mut f, position);
    assert!(
        witness_after,
        "writing the subtree roots REMOVED a witness that was there before — a defect \
         in the write, not a fact about anchors"
    );
    assert_eq!(
        anchor_before, anchor_after,
        "the checkpoint anchor MOVED when the subtree roots were written. The anchor \
         is the root a spend proof commits to; two values for one checkpoint means one \
         of them is wrong, and that is a money-path defect rather than the no-op this \
         row records"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The SQL spendability predicate ITSELF — the probe's S3 cell, ported to Rust
// ─────────────────────────────────────────────────────────────────────────────
//
// `INC-020`'s cost is the predicate at `zcash_client_sqlite-0.22.0
// wallet/common.rs:779-785`, and since an earlier revision the only thing measuring it was
// `docs/plan/probes/ironwood-unspendability-repro.py` — a Python probe `check.py`
// refuses as a guard by design. The maintainer ruled at the wrap (decision 1):
// port the probe's S3 cell to a Rust `#[test]`, pure SQLite plus the crate's own
// SQL, no harness — which is what makes it buildable at all, since the harness
// fixture above cannot complete the note's own shard.
//
// What is REAL here, and what is modelled, stated so the reader does not have to
// infer it:
//
// - The SCHEMA is real: `init_wallet_db` runs the crate's own migrations against
//   `Network::MainNetwork`, so every table, view, trigger and constraint —
//   `v_ironwood_shard_scan_ranges` with the real NU6.3 activation substituted, the
//   real `UNIQUE(root_hash)`, foreign keys ON — is the one a mainnet wallet has.
//   (The probe built the schema from extracted DDL; this is the stronger half.)
// - The QUERIES are the crate's own text, sliced out of the pinned source at run
//   time by the probe's own needles — nothing retyped, every needle asserted unique
//   — and `rarray(:exclude)` is the real virtual table (the probe had to substitute
//   it; rusqlite's `array` feature gives us the genuine one).
// - The REPAIR is [`crate::sync::put_subtree_roots`] — the SDK's own write, through
//   the crate's real `put_shard_roots` (cap insert, discontinuity check, UPSERT) —
//   not an extracted UPSERT literal. That is what makes this a guard for OUR fix
//   rather than a re-measurement of upstream: delete the Ironwood arm and the repair
//   does nothing.
// - The ROWS are modelled, exactly as the probe models them: one account, one mined
//   transaction, one Ironwood note at position 200,000 (shard 3), four shard rows in
//   `put_shard`'s shape (no `subtree_end_height`), the S3 scan queue. Stub key
//   material — the predicate never decodes a note; that is the layer ABOVE the SQL,
//   which the probe's honesty note 9 names and which this row does not reach either.
// - Shards 1 and 2's completion heights are interpolated (probe honesty note 3);
//   0 and 3 were measured from the live capture. The predicate turns on
//   NULL-vs-set and on ordering, not on the value.

/// The mainnet Ironwood subtree completion heights the probe models: shards 0 and
/// 3 MEASURED (`docs/plan/probes/ironwood-subtree-roots-probe.output.txt`, the first
/// and last root's `completing_block_height`), 1 and 2 interpolated.
///
/// (The `S3_` prefix on these constants names the probe's modelled wallet — the
/// S2 row shares every one of them; the two rows differ in the scan queue alone.)
const S3_SHARD_END_HEIGHTS: [u32; 4] = [3_451_206, 3_459_187, 3_467_168, 3_475_149];
/// Deliberately BEFORE NU6.3 activation (3,428,143), as the probe's is.
const S3_BIRTHDAY: u32 = 3_400_000;
const S3_CHAIN_TIP: u32 = 3_475_499;
/// 10 confirmations.
const S3_ANCHOR: u32 = S3_CHAIN_TIP - 9;
const S3_TARGET: u32 = S3_CHAIN_TIP + 1;
/// `scanning.rs:565`: `max_scanned - (PRUNING_DEPTH - 1)`, `PRUNING_DEPTH = 100`
/// (`zcash_client_sqlite-0.22.0 lib.rs:180`), `max_scanned` = the tip.
///
/// **Bound from the modelled tip, not derived from the fixture's `blocks` table** —
/// the probe's honesty note 4, carried over. The crate derives it from
/// `block_max_scanned`; a real wallet's `blocks` table reaches its scanned tip, but
/// this fixture holds ONE block row (the note's, for the foreign key), from which the
/// crate would derive a floor 5,500 blocks too low and no shard could latch. Binding
/// the floor a tip-scanned wallet would derive is the modelled half; the predicate's
/// gate on `subtree_end_height IS NOT NULL` — the thing under test — is unaffected
/// by where the floor comes from, and the row's BEFORE arm shows the floor alone
/// latches nothing.
const S3_PRUNING_FLOOR: u32 = S3_CHAIN_TIP - 99;
const S3_NOTE_HEIGHT: u32 = 3_470_000;
/// `200_000 >> 16 == 3`: the note sits in shard 3, whose root the endpoint serves
/// and whose completion (3,475,149) sits BELOW the anchor — the geometry in which
/// the served height is what keeps the ChainTip range out of the note's shard.
const S3_NOTE_POSITION: u64 = 200_000;
const S3_NOTE_VALUE: u64 = 100_000_000;
/// `wallet/scanning.rs:35-45`. `priority_code` is crate-private, so these are
/// literals. **Only `PRIORITY_SCANNED` is pinned** — by [`assert_priority_codes_hold`],
/// because it is the one the rows BIND (`:scanned_priority`) and the one the crate's
/// own `v_ironwood_shard_unscanned_ranges` view names in its `priority > …` clause,
/// read back out of `sqlite_master` before any row binds it (crypto audit).
/// `PRIORITY_CHAIN_TIP` and `PRIORITY_HISTORIC` are fixture INPUTS — the priorities
/// written into the modelled `scan_queue` — and are checked against nothing: a
/// renumbering of those two would change what geometry the rows model, not what
/// they bind, and the rows' own preconditions (`tip_unscanned` TRUE in S2, the
/// note's shard reading ChainTip in S3) are what would notice.
const PRIORITY_SCANNED: i64 = 10;
const PRIORITY_CHAIN_TIP: i64 = 50;
/// `:min_value` is bound from the crate's own `zip317::MARGINAL_FEE` in [`select`],
/// not from a local literal.
const S3_ACCOUNT_UUID: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
/// `Historic` — `wallet/scanning.rs:35-45`; the probe's S2 range priority.
const PRIORITY_HISTORIC: i64 = 20;

/// The probe's scan-queue geometries this module ports, as `(start, end, priority)`
/// ranges. Both are refused with NULL completion heights and selectable with them,
/// and they are refused by DIFFERENT conjuncts of `common.rs:779-785` — which is why
/// both are here rather than one (crypto audit).
enum Geometry {
    /// S3, "the ordinary chain-tip lag": `Scanned` up to the anchor, `ChainTip`
    /// above it. Refused by the note's shard `max_priority` (the ChainTip range is
    /// cross-joined onto every shard); `tip_unscanned` stays FALSE.
    S3ChainTipLag,
    /// S2, "the defect" in the probe's own words: ONE `Historic` range 25,000 blocks
    /// BELOW the note, below the anchor, in a different shard — the mid-restore
    /// back-fill every wallet in INC-020's population is in. Refused by the OTHER
    /// conjunct: `unscanned_tip_exists`'s `IFNULL(range.subtree_end_height,
    /// :anchor_height)` degenerates with NULL heights, so `tip_unscanned` is TRUE.
    S2HistoricGapBelow,
}

impl Geometry {
    fn scan_ranges(self) -> Vec<(u32, u32, i64)> {
        match self {
            Geometry::S3ChainTipLag => vec![
                (S3_BIRTHDAY, S3_ANCHOR + 1, PRIORITY_SCANNED),
                (S3_ANCHOR + 1, S3_TARGET, PRIORITY_CHAIN_TIP),
            ],
            Geometry::S2HistoricGapBelow => vec![
                (S3_BIRTHDAY, 3_445_000, PRIORITY_HISTORIC),
                (3_445_000, S3_TARGET, PRIORITY_SCANNED),
            ],
        }
    }
}

/// The priority codes this module binds are the crate's: read the migrated
/// `v_ironwood_shard_unscanned_ranges` view's DDL back out of `sqlite_master` and
/// assert it names `Scanned`'s code in its `priority > ` clause. A pin bump that
/// renumbered `ScanPriority` would otherwise leave every row here green while it
/// bound a different `:scanned_priority` than the wallet does.
fn assert_priority_codes_hold(conn: &Connection) {
    let ddl: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'view' AND name = 'v_ironwood_shard_unscanned_ranges'",
            [],
            |r| r.get(0),
        )
        .expect("the crate's unscanned-ranges view exists in the migrated schema");
    let expected = format!("priority > {PRIORITY_SCANNED}");
    assert!(
        ddl.contains(&expected),
        "the migrated view no longer says `{expected}` — the crate renumbered \
         ScanPriority and these literals must follow it. View DDL: {ddl}"
    );
}

/// The three statements the cell runs, in the crate's own words.
struct CrateSql {
    /// `mark_stabilized_notes`' UPDATE (`wallet/scanning.rs`), binds `:pruning_floor`
    /// and `:shard_height`.
    mark_stabilized: String,
    /// `unscanned_tip_exists` (`wallet/common.rs:147-167`), binds `:anchor_height`.
    unscanned_tip_exists: String,
    /// `select_spendable_notes_matching_value`'s query, `LockFilter::Unfiltered`,
    /// `ValueSelection::Accumulate`, every placeholder substituted the way the crate
    /// substitutes it for Ironwood.
    select_spendable: String,
}

/// The pinned `zcash_client_sqlite` source, where cargo unpacked it to compile this
/// very test binary — `$CARGO_HOME/registry/src/<index>/<crate>-<version>` — with the
/// version read from `sdk/Cargo.lock` so a pin bump moves this row with it.
///
/// **Fails if it cannot be found; never skips.** A guard that silently passes when
/// its subject is absent is INC-010's shape.
fn pinned_crate_source() -> PathBuf {
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.lock");
    let lock = std::fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", lock_path.display()));
    // EXACTLY ONE locked version. `Cargo.lock` sorts `[[package]]` stanzas by name
    // then version ASCENDING, so with two locked versions `find` would hand back the
    // LOWER one — and 0.21.0 is unpacked in this registry, still contains the S3
    // needles, and has no Ironwood at all. A test that sliced its SQL would be green
    // on a query the wallet never runs (security review).
    let stanzas = lock.matches("name = \"zcash_client_sqlite\"\n").count();
    assert_eq!(
        stanzas, 1,
        "sdk/Cargo.lock holds {stanzas} `zcash_client_sqlite` stanzas; this row reads \
         the crate's SQL from ONE pinned source and cannot choose between two"
    );
    let at = lock
        .find("name = \"zcash_client_sqlite\"\n")
        .expect("sdk/Cargo.lock names zcash_client_sqlite");
    let version = lock[at..]
        .lines()
        .nth(1)
        .and_then(|l| l.strip_prefix("version = \""))
        .and_then(|l| l.strip_suffix('"'))
        .expect("a `version = \"…\"` line follows the name line in the lock");
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME is set")).join(".cargo")
        });
    let registry_src = cargo_home.join("registry").join("src");
    let wanted = format!("zcash_client_sqlite-{version}");
    let indexes = std::fs::read_dir(&registry_src)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", registry_src.display()));
    // EXACTLY ONE unpacked copy across the registry indexes — a second index (a
    // mirror, a sparse-vs-git switch) holding the same name is two sources for one
    // pin, and "the first one read_dir happens to yield" is not a choice.
    let candidates: Vec<PathBuf> = indexes
        .map(|index| index.expect("registry index entry").path().join(&wanted))
        .filter(|candidate| candidate.join("src").is_dir())
        .collect();
    match candidates.as_slice() {
        [one] => one.clone(),
        [] => panic!(
            "{wanted} is not unpacked under {}. This binary was compiled against it, so \
             cargo unpacked it somewhere: point CARGO_HOME at that registry",
            registry_src.display()
        ),
        many => panic!(
            "{wanted} is unpacked in {} registry indexes ({many:?}); this row reads the \
             crate's SQL from ONE source and will not pick between copies",
            many.len()
        ),
    }
}

/// Slice the ONE string literal in `text` that contains `needle` — the probe's
/// `literal_containing`, line for line: back to the nearest `"`, forward to the
/// next. The crate's SQL literals carry no embedded `"` and no escapes, and both
/// are asserted rather than assumed. Works for `"…"` and `r#"…"#` alike (the raw
/// form's delimiters are also `"`).
fn literal_containing(text: &str, needle: &str) -> String {
    let n = text.matches(needle).count();
    assert_eq!(
        n, 1,
        "needle {needle:?} found {n} times in the pinned crate source, expected exactly \
         one — the crate changed shape under this row; re-derive the needle from the \
         source rather than loosening the match"
    );
    let at = text.find(needle).expect("counted exactly once");
    let start = text[..at]
        .rfind('"')
        .expect("an opening quote before the needle")
        + 1;
    let end = at
        + text[at..]
            .find('"')
            .expect("a closing quote after the needle");
    let body = &text[start..end];
    assert!(
        !body.contains('\\'),
        "the literal around {needle:?} contains an escape; the quote-to-quote slice is \
         not safe on it"
    );
    body.to_string()
}

/// Build the cell's three statements from the pinned source — the probe's
/// `build_sql`, restricted to the S3 cell's three links, same needles, same
/// substitutions (`LockFilter::Unfiltered` ⇒ `eligible_condition` "1", no tier
/// column, `wallet/locking.rs:277,377`; `ValueSelection::Accumulate`).
fn crate_sql(src: &Path) -> CrateSql {
    let read = |rel: &str| {
        let path = src.join("src").join(rel);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let common = read("wallet/common.rs");
    let scanning = read("wallet/scanning.rs");
    const PREFIX: &str = "ironwood";
    const OUTPUT_INDEX_COL: &str = "action_index";
    const NOTE_RECONSTRUCTION_COLS: &str = "rho, rseed, note_version";
    // `DEFAULT_TX_EXPIRY_DELTA` — `zcash_primitives transaction/builder.rs:53`.
    let tx_unexpired = literal_containing(&common, ".expiry_height = 0  -- the tx will not expire")
        .replace("{tx}", "stx")
        .replace("{DEFAULT_TX_EXPIRY_DELTA}", "40");
    let spent_clause =
        literal_containing(&common, "stx.id_tx = rns.transaction_id\n        WHERE {}")
            .replace("{table_prefix}", PREFIX)
            .replace("{}", &tx_unexpired);
    let mark_stabilized = literal_containing(&scanning, "SET witness_stabilized = 1")
        .replace("{table_prefix}", PREFIX);
    let unscanned_tip_exists = literal_containing(
        &common,
        "AND IFNULL(range.subtree_end_height, :anchor_height)",
    )
    .replace("{table_prefix}", PREFIX);
    let result_columns = literal_containing(
        &common,
        "max_shielding_input_height, min_shielding_input_trust",
    )
    .replace("{output_index_col}", OUTPUT_INDEX_COL)
    .replace("{note_reconstruction_cols}", NOTE_RECONSTRUCTION_COLS);
    let crossing = literal_containing(
        &common,
        "SELECT * from eligible WHERE so_far >= :target_value",
    );
    let selection_tail = literal_containing(&common, "FROM eligible WHERE so_far < :target_value")
        .replace("{result_columns}", &result_columns)
        .replace("{crossing_note_subquery}", &crossing);
    let select_spendable = literal_containing(&common, "WITH eligible AS (")
        .replace("{table_prefix}", PREFIX)
        .replace("{output_index_col}", OUTPUT_INDEX_COL)
        .replace("{note_reconstruction_cols}", NOTE_RECONSTRUCTION_COLS)
        .replace("{tier_column}", "0")
        .replace(
            "{window_frame}",
            "ORDER BY rn.commitment_tree_position ROWS UNBOUNDED PRECEDING",
        )
        .replace("{eligible_condition}", "1")
        .replace("{selection_tail}", &selection_tail)
        .replace("{}", &spent_clause);
    assert!(
        select_spendable.contains("rarray(:exclude)"),
        "the selection query no longer binds `:exclude` through rarray — the row's \
         parameter list must follow the crate"
    );
    for (name, sql) in [
        ("mark_stabilized", &mark_stabilized),
        ("unscanned_tip_exists", &unscanned_tip_exists),
        ("select_spendable", &select_spendable),
    ] {
        assert!(
            !sql.contains('{'),
            "{name}: an unsubstituted placeholder survived — the crate added one this \
             row does not know: {sql}"
        );
    }
    CrateSql {
        mark_stabilized,
        unscanned_tip_exists,
        select_spendable,
    }
}

/// A mainnet-schema wallet in the probe's modelled S3 state: subtree roots NOT
/// written (every `subtree_end_height` NULL), the scan queue `Scanned` up to the
/// anchor and `ChainTip` above it.
struct ModelledWallet {
    /// The connection the SDK's write goes through — [`crate::sync::put_subtree_roots`]
    /// takes an owned-connection `WalletDb`, so the file gets two.
    db: WalletDb<Connection, Network, SystemClock, OsRng>,
    /// The cell's own connection: foreign keys ON, `rarray` loaded.
    conn: Connection,
    sql: CrateSql,
    _dir: tempfile::TempDir,
}

fn build_modelled_wallet(geometry: Geometry) -> ModelledWallet {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("s3.db");
    let mut db = WalletDb::for_path(&path, Network::MainNetwork, SystemClock, OsRng)
        .expect("open a plaintext wallet file");
    init_wallet_db(&mut db, None)
        .expect("the crate's own migrations build the real mainnet schema, account-less");

    let conn = Connection::open(&path).expect("the cell's second connection");
    rusqlite::vtab::array::load_module(&conn).expect("rarray, the crate's own vtab");
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .expect("foreign keys ON, as the crate leaves them after migrating");
    assert_priority_codes_hold(&conn);
    // The probe's rows, in FK order. Stub key material: the SQL never decodes it.
    conn.execute(
        "INSERT INTO accounts (id, uuid, account_kind, hd_seed_fingerprint, hd_account_index,
                               ufvk, uivk, birthday_height)
         VALUES (1, ?1, 0, X'AA', 0, 'ufvk-stub', 'uivk-stub', ?2)",
        params![S3_ACCOUNT_UUID.to_vec(), S3_BIRTHDAY],
    )
    .expect("the account row");
    conn.execute(
        "INSERT INTO blocks (height, hash, time, sapling_tree) VALUES (?1, X'00', 0, X'00')",
        [S3_NOTE_HEIGHT],
    )
    .expect("the note's block row (the probe has no blocks table; the FK wants one)");
    conn.execute(
        "INSERT INTO transactions (id_tx, txid, block, mined_height, min_observed_height,
                                   expiry_height, trust_status)
         VALUES (1, X'01', ?1, ?1, ?1, 0, 1)",
        [S3_NOTE_HEIGHT],
    )
    .expect("the mined, trusted transaction");
    conn.execute(
        "INSERT INTO ironwood_received_notes (id, transaction_id, action_index, account_id,
             diversifier, value, rho, rseed, nf, is_change, commitment_tree_position,
             recipient_key_scope, witness_stabilized, note_version)
         VALUES (1, 1, 0, 1, X'00', ?1, X'00', X'00', X'11', 0, ?2, 0, 0, 1)",
        params![S3_NOTE_VALUE, S3_NOTE_POSITION],
    )
    .expect("the one Ironwood note, in shard 3");
    for shard in 0u8..4 {
        // `put_shard`'s column list — no `subtree_end_height` — which is the state
        // every scanned Ironwood shard is in until the roots are served.
        conn.execute(
            "INSERT INTO ironwood_tree_shards (shard_index, root_hash, shard_data)
             VALUES (?1, ?2, X'00')",
            params![shard, vec![shard; 32]],
        )
        .expect("a scanned shard row with NO completion height");
    }
    // The scan queue is the ONLY thing the two geometries differ in.
    for (start, end, priority) in geometry.scan_ranges() {
        conn.execute(
            "INSERT INTO scan_queue (block_range_start, block_range_end, priority)
             VALUES (?1, ?2, ?3)",
            params![start, end, priority],
        )
        .expect("a scan-queue range");
    }
    let sql = crate_sql(&pinned_crate_source());
    ModelledWallet {
        db,
        conn,
        sql,
        _dir: dir,
    }
}

/// One evaluation of the cell: the latch pass, the tip gate, the selection, and —
/// for the reader — the note's own shard's `max_priority` from the crate's view.
struct Cell {
    stabilized: i64,
    tip_unscanned: bool,
    selected: usize,
    note_shard_max_priority: Option<i64>,
}

/// The selection query, the crate's own text, with `:tip_unscanned` bound to the
/// value given — the real one from [`run_cell`], or a forced one for the
/// anti-vacuity checks.
fn select(w: &ModelledWallet, tip_unscanned: bool) -> usize {
    let exclude: Rc<Vec<Value>> = Rc::new(Vec::new());
    let mut stmt = w
        .conn
        .prepare(&w.sql.select_spendable)
        .expect("the crate's selection query prepares against the crate's schema");
    let ids = stmt
        .query_map(
            named_params! {
                ":account_uuid": S3_ACCOUNT_UUID.to_vec(),
                ":anchor_height": S3_ANCHOR,
                ":target_height": S3_TARGET,
                ":target_value": 1u64,
                ":exclude": exclude,
                ":scanned_priority": PRIORITY_SCANNED,
                ":tip_unscanned": i64::from(tip_unscanned),
                ":min_value": zcash_primitives::transaction::fees::zip317::MARGINAL_FEE.into_u64(),
            },
            |row| row.get::<_, i64>("id"),
        )
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows");
    ids.len()
}

fn run_cell(w: &ModelledWallet) -> Cell {
    w.conn
        .execute(
            &w.sql.mark_stabilized,
            named_params! {
                ":pruning_floor": S3_PRUNING_FLOOR,
                ":shard_height": i64::from(IRONWOOD_SHARD_HEIGHT),
            },
        )
        .expect("mark_stabilized_notes' UPDATE");
    let stabilized: i64 = w
        .conn
        .query_row(
            "SELECT witness_stabilized FROM ironwood_received_notes WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .expect("the latch");
    let tip_unscanned: bool = w
        .conn
        .query_row(
            &w.sql.unscanned_tip_exists,
            named_params! { ":anchor_height": S3_ANCHOR },
            |r| r.get(0),
        )
        .expect("unscanned_tip_exists");
    let selected = select(w, tip_unscanned);
    let note_shard_max_priority: Option<i64> = w
        .conn
        .query_row(
            "SELECT max_priority FROM v_ironwood_shards_scan_state WHERE shard_index = ?1",
            [S3_NOTE_POSITION >> IRONWOOD_SHARD_HEIGHT],
            |r| r.get(0),
        )
        .expect("the note's shard in the crate's scan-state view");
    Cell {
        stabilized,
        tip_unscanned,
        selected,
        note_shard_max_priority,
    }
}

fn scan_queue(conn: &Connection) -> Vec<(u32, u32, i64)> {
    let mut stmt = conn
        .prepare(
            "SELECT block_range_start, block_range_end, priority FROM scan_queue
             ORDER BY block_range_start",
        )
        .expect("prepare");
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows")
}

fn block_heights(conn: &Connection) -> Vec<u32> {
    let mut stmt = conn
        .prepare("SELECT height FROM blocks ORDER BY height")
        .expect("prepare");
    stmt.query_map([], |r| r.get(0))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows")
}

/// **The SQL spendability predicate, in the S3 geometry, flipped by the SDK's own
/// root write and by nothing else — the guard `INC-020` clause (d) waited for.**
///
/// The probe's matrix row, reproduced: with every `subtree_end_height` NULL the
/// crate's `v_ironwood_shard_scan_ranges` EXPANDS — the join's `OR
/// shard.subtree_end_height IS NULL` is satisfied for every shard, so the ChainTip
/// range above the anchor lands on the note's shard, its `max_priority` reads
/// ChainTip, `mark_stabilized_notes` cannot latch it, and
/// `select_spendable_notes_matching_value` returns nothing. `tip_unscanned` is
/// FALSE throughout — this cell is refused purely by the cross-joined shard
/// priority, the half of `common.rs:779-785` that the survey's headline named.
///
/// Then ONE call to [`crate::sync::put_subtree_roots`] — the four roots an honest
/// endpoint serves — and, with **zero scan-queue rows touched and zero blocks
/// rescanned**, the same query on the same connection selects the note and the
/// latch flips 0 → 1. That is "call the RPC", measured on the predicate the
/// incident names, in the geometry where it decides.
///
/// The gate is shown to answer BOTH ways from this schema before the latch is
/// earned (`:tip_unscanned` forced to 1 refuses, 0 selects), and the OR branch is
/// shown to bypass the tip gate once it is — so the 0 above is the gate firing,
/// not a fixture that cannot select.
///
/// # What this row covers, and what it does not
///
/// It covers the SHARD-PRIORITY conjunct of `common.rs:779-785` (`max_priority <=
/// Scanned`), the SQL half of INC-020 clause (d). The other conjunct of the same
/// NULL — `unscanned_tip_exists`' `IFNULL(range.subtree_end_height, :anchor_height)`
/// — is a different refusal in a different geometry and has its own row,
/// [`in_s2_a_historic_gap_below_the_note_refuses_it_through_the_tip_gate_until_the_roots_land`].
/// Neither row says anything about a WITNESS: the wallet here has a modelled tree
/// (`shard_data` stubs, `empty_root` hashes) that the row never deserialises, and
/// the harness rows above are the witness half on a fixture that cannot reach this
/// geometry. Selection and witness are proven on two fixtures that cannot be joined,
/// and a regression that restored one and broke the other on ONE wallet is not
/// caught here (security review).
///
/// Mutant: delete `db.put_ironwood_subtree_roots(0, ironwood)` from
/// [`crate::sync::put_subtree_roots`] → the repair writes no completion height, the
/// view stays expanded, and the AFTER cell still selects nothing.
#[test]
fn in_s3_the_sdks_root_write_alone_makes_the_note_selectable_with_zero_blocks_rescanned() {
    let mut w = build_modelled_wallet(Geometry::S3ChainTipLag);

    // db A — the probe's S3 row, NULL heights.
    let before = run_cell(&w);
    assert_eq!(
        before.note_shard_max_priority,
        Some(PRIORITY_CHAIN_TIP),
        "with no completion height on any shard, the crate's own view joins the \
         ChainTip range above the anchor onto the note's shard (3): the view EXPANDS \
         rather than degenerates, and this number is the mechanism"
    );
    assert!(
        !before.tip_unscanned,
        "S3 is refused by the shard priority, not by the tip gate — the probe's row \
         has tip_unscanned FALSE, and a TRUE here means the fixture is not S3"
    );
    assert_eq!(
        before.stabilized, 0,
        "a shard with a NULL completion height cannot latch its notes: \
         mark_stabilized_notes' EXISTS requires `subtree_end_height IS NOT NULL`"
    );
    assert_eq!(
        before.selected, 0,
        "INC-020, at the predicate: an Ironwood note 5,500 blocks deep in a trusted, \
         mined transaction is NOT selectable while the ordinary chain-tip lag stands \
         and no subtree root has been written"
    );
    let queue_before = scan_queue(&w.conn);
    let blocks_before = block_heights(&w.conn);

    // THE REPAIR — the SDK's own write, the four roots an honest endpoint serves.
    serve_the_four_roots(&mut w);
    assert_eq!(
        scan_queue(&w.conn),
        queue_before,
        "the repair touches ZERO scan-queue rows — it is a write of four heights, not \
         a rescan"
    );
    assert_eq!(
        block_heights(&w.conn),
        blocks_before,
        "and rescans ZERO blocks"
    );

    // Anti-vacuity, BEFORE the latch is earned: the same schema must be able to
    // answer both ways, or a 0 above is indistinguishable from a broken fixture.
    assert_eq!(
        select(&w, true),
        0,
        "with the latch still 0 and `:tip_unscanned` forced to 1, the AND branch \
         refuses — the gate is live"
    );
    assert_eq!(
        select(&w, false),
        1,
        "with the roots written and `:tip_unscanned` 0, the AND branch selects: the \
         note's shard now reads Scanned"
    );

    // db B — the probe's S3 row after the repair.
    let after = run_cell(&w);
    assert_eq!(
        after.note_shard_max_priority,
        Some(PRIORITY_SCANNED),
        "with completion heights written the view collapses to each shard's own \
         block extent, and the ChainTip range no longer reaches shard 3"
    );
    assert!(
        !after.tip_unscanned,
        "the tip gate is unchanged by the repair"
    );
    assert_eq!(
        after.stabilized, 1,
        "witness_stabilized False → True: the note's shard is complete, fully Scanned \
         and below the pruning floor, so mark_stabilized_notes latches it"
    );
    assert_eq!(
        after.selected, 1,
        "and the note is selectable — INC-020's repair, measured on INC-020's \
         predicate: one write, no rescan"
    );
    assert_eq!(
        select(&w, true),
        1,
        "once latched, `witness_stabilized = 1` bypasses the tip gate — the OR \
         branch at common.rs:779-785, which is why a stale latch is dangerous \
         (root_bind's un-latch rows) and why earning it honestly matters"
    );
}

/// The repair, shared by the S3 and S2 rows: the four roots an honest endpoint
/// serves, through [`crate::sync::put_subtree_roots`] — the SDK's own write, so a
/// deleted Ironwood arm leaves both rows' AFTER cells unselectable.
fn serve_the_four_roots(w: &mut ModelledWallet) {
    let ironwood: Vec<_> = S3_SHARD_END_HEIGHTS
        .iter()
        .zip(0u8..)
        .map(|(&height, shard)| {
            CommitmentTreeRoot::from_parts(
                BlockHeight::from_u32(height),
                MerkleHashOrchard::empty_root(Level::from(shard)),
            )
        })
        .collect();
    let roots = SubtreeRoots::from_pools(Vec::new(), Vec::new(), ironwood);
    crate::sync::put_subtree_roots(&mut w.db, &roots, |_| Ok(()))
        .expect("the SDK's own subtree-root write against the real mainnet schema");
}

/// **The OTHER conjunct of the same NULL — the probe's S2, "this is the defect" in
/// its own narration — refused through the TIP gate, and repaired by the same
/// write.**
///
/// One `Historic` range 25,000 blocks BELOW the note, below the anchor, in a
/// different shard: the mid-restore back-fill that every wallet in INC-020's
/// population is in. With every `subtree_end_height` NULL, `unscanned_tip_exists`
/// (`common.rs:147-167`) degenerates — `:anchor_height BETWEEN
/// range.subtree_start_height AND IFNULL(range.subtree_end_height, :anchor_height)`
/// is TRUE for any unscanned range starting at or below the anchor — so
/// `tip_unscanned` is TRUE and the AND branch refuses through ITS FIRST conjunct,
/// whatever the note's own shard says. After the write every shard's window ends
/// below the anchor, the historic range falls outside all of them, and the same
/// note is selected with zero blocks rescanned.
///
/// The S3 row cannot see this: its `tip_unscanned` is FALSE on both arms by
/// construction. Ported because a flip of INC-020 to GUARDED on S3 alone would have
/// claimed the whole predicate while guarding one conjunct (crypto audit).
///
/// Mutant: the same one as the S3 row — delete the Ironwood arm of
/// [`crate::sync::put_subtree_roots`] → the tip gate stays TRUE after the "repair"
/// and the AFTER cell selects nothing.
#[test]
fn in_s2_a_historic_gap_below_the_note_refuses_it_through_the_tip_gate_until_the_roots_land() {
    let mut w = build_modelled_wallet(Geometry::S2HistoricGapBelow);

    let before = run_cell(&w);
    assert!(
        before.tip_unscanned,
        "S2 is refused through the TIP gate: with NULL completion heights the \
         historic range below the anchor satisfies unscanned_tip_exists' degenerate \
         IFNULL. A FALSE here means the fixture is not S2"
    );
    assert_eq!(
        before.stabilized, 0,
        "no shard can latch its notes without a completion height"
    );
    assert_eq!(
        before.selected, 0,
        "INC-020 in the mid-restore geometry: a back-fill range 25,000 blocks below \
         the note, in another shard, keeps the note unselectable while no subtree \
         root has been written"
    );
    let queue_before = scan_queue(&w.conn);
    let blocks_before = block_heights(&w.conn);

    serve_the_four_roots(&mut w);
    assert_eq!(
        scan_queue(&w.conn),
        queue_before,
        "zero scan-queue rows touched"
    );
    assert_eq!(
        block_heights(&w.conn),
        blocks_before,
        "zero blocks rescanned"
    );

    let after = run_cell(&w);
    assert!(
        !after.tip_unscanned,
        "with completion heights written, the historic range sits below every \
         shard's window that reaches the anchor: the tip gate opens"
    );
    assert_eq!(
        after.stabilized, 1,
        "and the note's shard latches — it is complete, Scanned and below the floor"
    );
    assert_eq!(
        after.selected, 1,
        "and the note is selectable: the same write repairs the OTHER conjunct"
    );
}
