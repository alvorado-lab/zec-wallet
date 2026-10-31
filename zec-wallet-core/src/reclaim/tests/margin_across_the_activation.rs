//! The NU7 wave's step 5, its reclaim half, and step 7m (`docs/plan/nu7-wave.md` §3 steps 5
//! and 7m; ADR-0579 Decision 6): the TEST AUTHOR's rows that read the deadness gate across the
//! activation. The gate has to learn the activation height, which only the network's parameters
//! carry, so it takes them. These rows were written blind against that signature and kept off
//! (`#[cfg(any())]`) until the join, as steps 2b, 4s and 4 did; they run since the join
//! (`2ff82f9fd`, 2026-10-30). Each was first run against throwaway definitions of the name
//! alone, never committed.
//!
//! **The one name this file assumes is read in ONE place, the `named` module below**, which
//! already called the built signature, so the join changed no line of this file:
//!
//! - the gate, `crate::reclaim::is_provably_dead<P: Parameters>(params: &P,
//!   exposed_at_height: u32, tip: u32) -> bool`. The contract names the gate (`reclaim.rs`,
//!   `is_provably_dead`) but not its signature; this takes the parameters first, as
//!   `stranded::irreversibly_buried` does.
//!
//! An index is provably dead once the expiry of the tx0 that reserved it is irreversibly
//! buried. The tx0 targets the block after the exposure; its expiry is that target plus the
//! builder's default delta AT THE TARGET'S BRANCH (`default_tx_expiry_delta`: 40 before NU7, 120
//! from it); and the burial is the predicate's 15,000 s of chain time (200 blocks at 75 s, 600
//! at 25 s, summed across the activation). So the margin is (40 + 1) + 200 = 241 below the
//! activation and (120 + 1) + 600 = 721 from it.

use zcash_client_backend::data_api::testing::TestBuilder;
use zcash_primitives::transaction::builder::default_tx_expiry_delta;
use zcash_protocol::consensus::{
    BlockHeight, BranchId, MAIN_NETWORK, NetworkUpgrade, Parameters, TEST_NETWORK,
    target_time_between,
};
use zcash_protocol::local_consensus::LocalNetwork;

use crate::constants::{EPHEMERAL_RECLAIM_DEADNESS_BLOCKS, IRREVERSIBLE_BURIAL_SECS};

/// THE JOIN POINT for the one name the implementer changes. The rows below never name it.
mod named {
    use zcash_protocol::consensus::Parameters;

    /// The deadness gate on `params`' heights.
    pub(super) fn dead_on<P: Parameters>(params: &P, exposed: u32, tip: u32) -> bool {
        crate::reclaim::is_provably_dead(params, exposed, tip)
    }
}

use named::dead_on;

/// `params`' NU7 activation height.
fn nu7_of<P: Parameters>(params: &P) -> u32 {
    u32::from(
        params
            .activation_height(NetworkUpgrade::Nu7)
            .expect("these params carry an NU7 height"),
    )
}

/// The harness network with every upgrade through NU6.3 at its NU5 height and NU7 1,000 blocks
/// above it: the straddling shape step 4's tests build on, its Blossom below every row here.
fn straddling() -> LocalNetwork {
    let below = TestBuilder::<(), ()>::DEFAULT_NETWORK.nu5;
    LocalNetwork {
        nu6: below,
        nu6_1: below,
        nu6_2: below,
        nu6_3: below,
        nu7: below.map(|h| h + 1_000),
        ..TestBuilder::<(), ()>::DEFAULT_NETWORK
    }
}

/// The boundary, one block each side: not dead `margin - 1` blocks after `exposed`, dead at
/// `margin` and after.
fn assert_dead_only_at<P: Parameters>(name: &str, params: &P, exposed: u32, margin: u32) {
    assert!(
        !dead_on(params, exposed, exposed + margin - 1),
        "{name}: exposed at {exposed}, {} blocks on — a reorg the burial covers can still mine \
         its tx0, must NOT be dead",
        margin - 1
    );
    assert!(
        dead_on(params, exposed, exposed + margin),
        "{name}: exposed at {exposed}, {margin} blocks on — its tx0's expiry is irreversibly \
         buried, dead"
    );
    assert!(
        dead_on(params, exposed, exposed + margin + 1),
        "{name}: exposed at {exposed}, {} blocks on — still dead",
        margin + 1
    );
}

#[test]
fn the_reclaim_margin_is_dead_only_at_its_margin_after_the_activation() {
    // An index exposed at the activation, just after it and far after it: not dead at 720, dead
    // at 721, which is (120 + 1) + 600. Mutant (the constant 141): dead at 141; red.
    fn rows<P: Parameters>(name: &str, params: &P) {
        let h = nu7_of(params);
        for exposed in [h, h + 1, h + 100_000] {
            assert_dead_only_at(name, params, exposed, 721);
        }
    }
    assert_eq!(
        nu7_of(&MAIN_NETWORK),
        3_543_405,
        "mainnet NU7, as the contract has it"
    );
    assert_eq!(
        nu7_of(&TEST_NETWORK),
        4_465_026,
        "testnet NU7, as the contract has it"
    );
    rows("mainnet", &MAIN_NETWORK);
    rows("testnet", &TEST_NETWORK);
    rows("straddling LocalNetwork", &straddling());
}

#[test]
fn the_reclaim_margin_straddling_the_activation_takes_the_target_branch() {
    // Exposed at `H - 1`: the tx0 targets `H`, so its delta is NU7's 120 and the margin 721.
    // Exposed at `H - 2`: it targets `H - 1`, the branch before, so its delta is 40; its expiry
    // `H + 39` is buried in 600 blocks of 25 s, and the margin is 641. Mutant (the branch at the
    // exposure height): `H - 1` takes 40 and reads dead at 641; red.
    fn rows<P: Parameters>(name: &str, params: &P) {
        let h = nu7_of(params);
        assert_dead_only_at(name, params, h - 1, 721);
        assert_dead_only_at(name, params, h - 2, 641);
    }
    rows("mainnet", &MAIN_NETWORK);
    rows("testnet", &TEST_NETWORK);
    rows("straddling LocalNetwork", &straddling());
}

#[test]
fn the_deadness_gate_is_dead_exactly_at_the_derived_margin_below_the_activation() {
    // The gate's half of `reclaim::tests::deadness_margin_meets_the_engine_arithmetic_for_a_full_
    // reorg_burial`, which derives the constant and cannot call the gate at the base: an index
    // exposed above Blossom and far below the activation is dead exactly
    // `EPHEMERAL_RECLAIM_DEADNESS_BLOCKS` blocks on, and not one block before.
    assert_dead_only_at(
        "mainnet, far below the activation",
        &MAIN_NETWORK,
        2_000_000,
        EPHEMERAL_RECLAIM_DEADNESS_BLOCKS,
    );
    assert_dead_only_at(
        "testnet, far below the activation",
        &TEST_NETWORK,
        4_000_000,
        EPHEMERAL_RECLAIM_DEADNESS_BLOCKS,
    );
}

#[test]
fn a_pre_activation_expiry_buried_across_the_activation_sums_both_spacings() {
    // Not in the contract's list. An index exposed below `H - 1` has a tx0 under the pre-NU7
    // branch (delta 40), but the burial of its expiry is counted through the predicate, 75 s a
    // block below `H` and 25 s from it, so the margin climbs from 241 to 641 as the exposure
    // nears `H`. Rows: (blocks below `H`, margin). Mutant (241 below the target's activation and
    // 721 from it, never through the predicate): dead at 241 in every row but the last; red.
    const ROWS: [(u32, u32); 6] = [
        (2, 641),   // the expiry `H + 39`: 600 blocks of 25 s
        (42, 641),  // `H - 1`: 600 blocks of 25 s after it
        (43, 639),  // `H - 2`: one block of 75 s, then 597 of 25 s
        (100, 525), // `H - 59`: 58 blocks of 75 s (4,350 s), then 426 of 25 s
        (241, 243), // `H - 200`: 199 blocks of 75 s (14,925 s), then 3 of 25 s
        (242, 241), // `H - 201`: 200 blocks of 75 s, wholly before `H`
    ];
    fn rows<P: Parameters>(name: &str, params: &P) {
        let h = nu7_of(params);
        for (below, margin) in ROWS {
            assert_dead_only_at(name, params, h - below, margin);
        }
    }
    rows("mainnet", &MAIN_NETWORK);
    rows("testnet", &TEST_NETWORK);
    rows("straddling LocalNetwork", &straddling());
}

#[test]
fn the_reclaim_margin_reads_the_network_s_own_activation() {
    // Not in the contract's list. The margin reads the activation its params carry, never one
    // network's height for every network: mainnet activates NU7 below testnet, so an index
    // exposed at mainnet's activation has 721 on mainnet and 241 on testnet, and one exposed at
    // testnet's has 721 on both. Params with no NU7 height keep 241 at every height. Mutant
    // (mainnet's height wired in, the shape a gate that takes no params would need): testnet
    // reads 721 at mainnet's activation; red.
    let main_h = nu7_of(&MAIN_NETWORK);
    let test_h = nu7_of(&TEST_NETWORK);
    assert!(
        main_h + 1_000 < test_h,
        "premise: mainnet's activation sits well below testnet's"
    );
    assert_dead_only_at("mainnet at its activation", &MAIN_NETWORK, main_h, 721);
    assert_dead_only_at(
        "testnet at mainnet's activation",
        &TEST_NETWORK,
        main_h,
        241,
    );
    assert_dead_only_at("testnet at its activation", &TEST_NETWORK, test_h, 721);
    assert_dead_only_at(
        "mainnet at testnet's activation",
        &MAIN_NETWORK,
        test_h,
        721,
    );
    let no_nu7 = LocalNetwork {
        nu7: None,
        ..straddling()
    };
    for exposed in [nu7_of(&straddling()) - 1, main_h, test_h] {
        assert_dead_only_at("params with no NU7 height", &no_nu7, exposed, 241);
    }
}

/// The contract's deadness, read from the crate alone: the tx0 targets the block after
/// `exposed`; its expiry is that target plus `default_tx_expiry_delta` at the target's branch;
/// and it is dead once the crate's `target_time_between` from the expiry reaches the burial's
/// 15,000 s.
fn crate_says_dead<P: Parameters>(params: &P, exposed: u32, tip: u32) -> bool {
    let target = BlockHeight::from_u32(exposed + 1);
    let expiry = target + default_tx_expiry_delta(BranchId::for_height(params, target));
    target_time_between(params, expiry, BlockHeight::from_u32(tip)).as_secs()
        >= IRREVERSIBLE_BURIAL_SECS
}

/// The gate agrees with [`crate_says_dead`] at every tip up to 800 blocks after each exposure.
fn assert_agrees_with_the_crate<P: Parameters>(
    name: &str,
    params: &P,
    exposures: impl IntoIterator<Item = u32>,
) {
    for exposed in exposures {
        for tip in exposed..=exposed + 800 {
            assert_eq!(
                dead_on(params, exposed, tip),
                crate_says_dead(params, exposed, tip),
                "{name}: exposed at {exposed}, tip {tip}"
            );
        }
    }
}

#[test]
fn the_reclaim_margin_is_the_crate_s_expiry_delta_buried_at_every_exposure() {
    // Step 7m: the margin against `default_tx_expiry_delta`, over the predicate's depth, at every
    // exposure from 300 blocks below the activation to 30 after it, and far after it. Each
    // mutant above is red here too.
    fn rows<P: Parameters>(name: &str, params: &P) {
        let h = nu7_of(params);
        assert_agrees_with_the_crate(name, params, (h - 300..=h + 30).chain([h + 100_000]));
    }
    rows("mainnet", &MAIN_NETWORK);
    rows("testnet", &TEST_NETWORK);
    rows("straddling LocalNetwork", &straddling());
}

#[test]
fn below_blossom_the_reclaim_margin_is_the_crate_s_too() {
    // Step 7m swaps step 4s's spacing function for the crate's (A-F6), whose spacing below
    // Blossom is 150 s and whose default delta there is 20. A restore with no birthday scans
    // from below Blossom, but no tx0 is live there on mainnet or testnet; the wallet harness
    // reserves one there (testnet at 281,000, below testnet's Blossom at 584,000): there the
    // margin is (20 + 1) + 100 = 121. Kept apart so the below-Blossom reading is ruled alone.
    fn rows<P: Parameters>(name: &str, params: &P) {
        let blossom = u32::from(
            params
                .activation_height(NetworkUpgrade::Blossom)
                .expect("these params carry a Blossom height"),
        );
        assert_agrees_with_the_crate(name, params, blossom - 400..=blossom + 50);
        assert_dead_only_at(name, params, blossom - 1_000, 121);
    }
    rows("mainnet", &MAIN_NETWORK);
    rows("testnet", &TEST_NETWORK);
}
