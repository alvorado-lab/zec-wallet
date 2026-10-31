//! The NU7 wave's step 4s, the spacing function: the test author's rows, written
//! blind against the contract (`docs/plan/nu7-wave.md` §3 "Step 4s", with the
//! reasons in §3 "Step 5" and ADR-0579 Decision 6). Declared under
//! `#[cfg(any())]` in `lib.rs` until the join: at the contract commit the names
//! below do not exist and this file does not compile, which is its red.
//!
//! **The names this file assumes.** The contract's two working names, and the
//! chain-time sum its third named test reads:
//!
//! * `crate::chain_time::target_spacing_secs(params, height)`: the target spacing,
//!   in seconds, between the block at `height` and its parent;
//! * `crate::chain_time::default_expiry_delta(params, height)`: the default expiry
//!   delta, in blocks, of a transaction whose target height is `height`;
//! * `crate::chain_time::chain_time_between_secs(params, from, to)`: the chain time,
//!   in seconds, from the block at `from` to the block at `to`.
//!
//! Each takes `&P` for any `P: Parameters` and a `BlockHeight`. Every result is
//! read through `u64::from`, so an unsigned result of any width joins unedited.
//!
//! **Which heights are `H`.** The pinned crate's own NU7 activation on both
//! networks (mainnet 3,543,405, testnet 4,465,026; each test checks the crate
//! still says so), and a straddling `LocalNetwork` of the shape step 4's tests
//! build on. "No NU7 height" is a `LocalNetwork` with `nu7: None`: each real
//! network's own heights for every other upgrade, which is mainnet and testnet as
//! the pre-release crates described them, and the harness network.
//!
//! **What "between two heights" sums.** The first named test fixes the spacing AT
//! `H` as 25: the block at `H` is the first one targeted 25 s after its parent. So
//! the time from the block at `from` to the block at `to` is the spacing of every
//! height in `(from, to]`, and a span that starts at the block `H - 1` is wholly
//! after the activation. That is also how upstream's `target_time_between` reads,
//! which the equality pin below (A-F6) holds ours to.
//!
//! **Heights below Blossom are not asserted.** The contract says 75 "before the
//! activation"; the crate's `pow_target_spacing` says 150 below Blossom. No row
//! here reads a height below its network's Blossom height, so neither reading is
//! imposed. Since step 7m the module returns the crate's values, 150 s and 20 blocks
//! below Blossom; `stranded::tests` reads the burial there.

use zcash_client_backend::data_api::testing::TestBuilder;
use zcash_primitives::transaction::builder::default_tx_expiry_delta;
use zcash_protocol::consensus::{
    BlockHeight, BranchId, Network, NetworkUpgrade, Parameters, pow_target_spacing,
    target_time_between,
};
use zcash_protocol::local_consensus::LocalNetwork;

use crate::chain_time::{
    chain_time_between as chain_time_between_secs, default_expiry_delta, target_spacing_secs,
};

/// ZIP 218's wall-clock target for an irreversible burial (ADR-0579 Decision 6).
const BURIAL_SECS: u64 = 15_000;

/// The contract's NU7 activation on mainnet, which the pinned crate carries.
const MAINNET_NU7: u32 = 3_543_405;
/// The contract's NU7 activation on testnet, which the pinned crate carries.
const TESTNET_NU7: u32 = 4_465_026;

/// The two real networks.
const NETWORKS: [Network; 2] = [Network::MainNetwork, Network::TestNetwork];

/// `network`'s NU7 activation, checked against the contract's literal so every
/// boundary row below sits on the real activation and not on a moved one.
fn real_activation(network: Network) -> BlockHeight {
    let contract = match network {
        Network::MainNetwork => MAINNET_NU7,
        Network::TestNetwork => TESTNET_NU7,
    };
    let h = network
        .activation_height(NetworkUpgrade::Nu7)
        .expect("the pinned crate carries an NU7 height on both networks");
    assert_eq!(
        h,
        BlockHeight::from_u32(contract),
        "{network:?}: the crate's NU7 height is not the contract's; re-read the contract before \
         trusting any boundary row here"
    );
    h
}

/// `network` with no NU7 height: its own height for every other upgrade. Built
/// over the harness default so a field upstream adds behind a `cfg` is filled.
fn without_nu7(network: Network) -> LocalNetwork {
    let at = |nu| network.activation_height(nu);
    let mut p = TestBuilder::<(), ()>::DEFAULT_NETWORK;
    p.overwinter = at(NetworkUpgrade::Overwinter);
    p.sapling = at(NetworkUpgrade::Sapling);
    p.blossom = at(NetworkUpgrade::Blossom);
    p.heartwood = at(NetworkUpgrade::Heartwood);
    p.canopy = at(NetworkUpgrade::Canopy);
    p.nu5 = at(NetworkUpgrade::Nu5);
    p.nu6 = at(NetworkUpgrade::Nu6);
    p.nu6_1 = at(NetworkUpgrade::Nu6_1);
    p.nu6_2 = at(NetworkUpgrade::Nu6_2);
    p.nu6_3 = at(NetworkUpgrade::Nu6_3);
    p.nu7 = None;
    p
}

/// The harness network with every upgrade through NU6.3 at the harness's NU5
/// height and NU7 at `nu7`: step 4's straddling seam, or no NU7 at all.
fn harness_network(nu7: Option<BlockHeight>) -> LocalNetwork {
    let below = TestBuilder::<(), ()>::DEFAULT_NETWORK.nu5;
    LocalNetwork {
        nu6: below,
        nu6_1: below,
        nu6_2: below,
        nu6_3: below,
        nu7,
        ..TestBuilder::<(), ()>::DEFAULT_NETWORK
    }
}

/// The straddling harness network's NU7 height: 1,000 blocks above every other
/// upgrade, so every row here (at most 300 blocks below it) sits above Blossom.
fn harness_nu7() -> BlockHeight {
    TestBuilder::<(), ()>::DEFAULT_NETWORK
        .nu5
        .expect("the harness network activates NU5")
        + 1_000
}

/// Every no-NU7 params value, with its Blossom height and the heights it is read
/// at: its Blossom height, both real NU7 activations and the straddling harness
/// height with a block each side, and the top of the range.
fn no_nu7_cases() -> Vec<(&'static str, LocalNetwork, BlockHeight, Vec<BlockHeight>)> {
    let mut switches = Vec::new();
    for h in [MAINNET_NU7, TESTNET_NU7] {
        switches.extend([h - 1, h, h + 1].map(BlockHeight::from_u32));
    }
    let harness = harness_nu7();
    switches.extend([harness - 1, harness, harness + 1]);
    [
        ("mainnet without NU7", without_nu7(Network::MainNetwork)),
        ("testnet without NU7", without_nu7(Network::TestNetwork)),
        ("LocalNetwork { nu7: None }", harness_network(None)),
    ]
    .into_iter()
    .map(|(name, p)| {
        let blossom = p
            .activation_height(NetworkUpgrade::Blossom)
            .expect("Blossom is active");
        let mut rows = vec![blossom];
        rows.extend(switches.iter().copied().filter(|h| *h >= blossom));
        rows.push(BlockHeight::from_u32(u32::MAX));
        (name, p, blossom, rows)
    })
    .collect()
}

// The three readers go through `u64::from` so whichever unsigned width the
// implementer returns joins unedited; where it is already `u64` the conversion
// is the identity, which is all the allow covers.
#[allow(clippy::useless_conversion)]
fn spacing<P: Parameters>(params: &P, height: BlockHeight) -> u64 {
    u64::from(target_spacing_secs(params, height))
}

#[allow(clippy::useless_conversion)]
fn delta<P: Parameters>(params: &P, height: BlockHeight) -> u64 {
    u64::from(default_expiry_delta(params, height))
}

#[allow(clippy::useless_conversion)]
fn chain_time<P: Parameters>(params: &P, from: BlockHeight, to: BlockHeight) -> u64 {
    u64::from(chain_time_between_secs(params, from, to))
}

/// The number of blocks after `from` at which the chain time from `from` first
/// reaches `target`.
fn blocks_to_reach<P: Parameters>(params: &P, from: BlockHeight, target: u64) -> u32 {
    (1..=10_000u32)
        .find(|n| chain_time(params, from, from + *n) >= target)
        .expect("the target is reached within 10,000 blocks at any spacing here")
}

/// One straddling params value's rows for a per-height reading: `before` at
/// `H - 1` and below, `from` at `H`, above it and at the top of the range.
fn assert_switches_at<P: Parameters>(
    name: &str,
    params: &P,
    h: BlockHeight,
    read: fn(&P, BlockHeight) -> u64,
    (before, from): (u64, u64),
) {
    // The boundary, one block each side. A switch at `H + 1` reads `before` at `H`.
    assert_eq!(read(params, h - 1), before, "{name}: at H - 1");
    assert_eq!(read(params, h), from, "{name}: at H");
    // Away from the boundary on each side, so a switch that holds only AT `H` is
    // red too.
    assert_eq!(read(params, h - 300), before, "{name}: at H - 300");
    assert_eq!(read(params, h + 1), from, "{name}: at H + 1");
    assert_eq!(
        read(params, BlockHeight::from_u32(u32::MAX)),
        from,
        "{name}: at the top of the range"
    );
}

#[test]
fn target_spacing_is_75_before_the_activation_and_25_from_it() {
    // Mutant (switch at `H + 1`): every `H` row reads 75.
    for network in NETWORKS {
        let name = format!("{network:?}");
        assert_switches_at(&name, &network, real_activation(network), spacing, (75, 25));
    }
    let straddling = harness_network(Some(harness_nu7()));
    assert_switches_at(
        "straddling LocalNetwork",
        &straddling,
        harness_nu7(),
        spacing,
        (75, 25),
    );

    // No NU7 height: today's 75 at every height from Blossom, mainnet under the
    // pre-release included, and at the heights where the other networks switch.
    for (name, p, _, rows) in no_nu7_cases() {
        for h in rows {
            assert_eq!(spacing(&p, h), 75, "{name}: the spacing at {h}");
        }
    }
}

#[test]
fn the_default_expiry_delta_is_40_before_the_activation_and_120_from_it() {
    // A transaction targeted at `H - 1` and one targeted at `H`. Mutant (40 at
    // every height): every `H` row reads 40.
    for network in NETWORKS {
        let name = format!("{network:?}");
        assert_switches_at(&name, &network, real_activation(network), delta, (40, 120));
    }
    let straddling = harness_network(Some(harness_nu7()));
    assert_switches_at(
        "straddling LocalNetwork",
        &straddling,
        harness_nu7(),
        delta,
        (40, 120),
    );

    // No NU7 height: today's 40 at every height from Blossom.
    for (name, p, _, rows) in no_nu7_cases() {
        for h in rows {
            assert_eq!(delta(&p, h), 40, "{name}: the default expiry delta at {h}");
        }
    }
}

/// The burial rows: (start, blocks to reach 15,000 s, the last counted block's
/// spacing, what the span covers).
fn burial_rows(h: BlockHeight) -> [(BlockHeight, u32, u64, &'static str); 4] {
    [
        // The 200 blocks from `h - 200` to `h - 1`.
        (h - 201, 200, 75, "wholly before H"),
        // The 600 blocks from `h` to `h + 599`, counted from the block at `h - 1`.
        // Under the start-height mutant this row reads 75 s a block.
        (h - 1, 600, 25, "wholly after H, from the block at H - 1"),
        // 100 blocks before (7,500 s) and 300 from it (7,500 s).
        (h - 101, 400, 25, "100 before H and 300 from it"),
        // One block before (75 s) and 597 from it (14,925 s): the tightest straddle.
        (h - 2, 598, 25, "1 before H and 597 from it"),
    ]
}

/// One straddling params value's burial rows: where the 15,000 s target lands,
/// and the chain time one block short of it and at it.
fn assert_burial_rows<P: Parameters>(name: &str, params: &P, h: BlockHeight) {
    for (from, lands_at, last, what) in burial_rows(h) {
        assert_eq!(
            blocks_to_reach(params, from, BURIAL_SECS),
            lands_at,
            "{name}, {what}: the block count at which 15,000 s of chain time is reached"
        );
        assert_eq!(
            chain_time(params, from, from + lands_at),
            BURIAL_SECS,
            "{name}, {what}: the chain time at the count"
        );
        assert_eq!(
            chain_time(params, from, from + (lands_at - 1)),
            BURIAL_SECS - last,
            "{name}, {what}: the chain time one block short of the count"
        );
    }
}

#[test]
fn chain_time_between_two_heights_sums_each_side_of_the_activation() {
    // Mutant (the spacing at the start height for the whole span): the rows that
    // start below `H` and count blocks from `H` read 75 s a block and land early.
    for network in NETWORKS {
        let name = format!("{network:?}");
        assert_burial_rows(&name, &network, real_activation(network));
    }
    let straddling = harness_network(Some(harness_nu7()));
    assert_burial_rows("straddling LocalNetwork", &straddling, harness_nu7());

    // No NU7 height: 200 blocks wherever the span starts, the real activations'
    // neighbourhoods included (only rows whose spans stay above Blossom).
    for (name, p, blossom, rows) in no_nu7_cases() {
        let top = BlockHeight::from_u32(u32::MAX - 1_000);
        let inside = |h: &BlockHeight| *h - 301 > blossom && *h < top;
        for h in rows.into_iter().filter(inside) {
            for (from, ..) in burial_rows(h) {
                assert_eq!(
                    blocks_to_reach(&p, from, BURIAL_SECS),
                    200,
                    "{name}: from {from}"
                );
                assert_eq!(
                    chain_time(&p, from, from + 200),
                    BURIAL_SECS,
                    "{name}: from {from}"
                );
            }
        }
    }
}

#[test]
fn chain_time_of_an_empty_or_reversed_span_is_zero() {
    // Not in the contract's list. A tip below the mined height is what a reorg
    // leaves behind, and the burial predicate reads the chain time from the mined
    // height to the tip: a reversed span must read no time at all, never a wrapped
    // one that would make a reorged transaction look buried.
    for network in NETWORKS {
        let h = real_activation(network);
        for at in [h - 1, h, h + 1] {
            assert_eq!(
                chain_time(&network, at, at),
                0,
                "{network:?}: empty at {at}"
            );
        }
        assert_eq!(
            chain_time(&network, h, h - 1),
            0,
            "{network:?}: one block back"
        );
        assert_eq!(
            chain_time(&network, h + 599, h - 201),
            0,
            "{network:?}: a reorg back across the activation"
        );
    }
}

/// One straddling params value's sums: one block's chain time is its spacing, a
/// window's is the sum of its blocks' spacings, and a span split at any height
/// is the sum of its halves.
fn assert_sums_of_the_one_spacing<P: Parameters>(name: &str, params: &P, h: BlockHeight) {
    for at in [h - 1, h, h + 1] {
        assert_eq!(
            chain_time(params, at - 1, at),
            spacing(params, at),
            "{name}: one block's chain time at {at} is its spacing"
        );
    }
    // The span `(h - 10, h + 10]`: the nine blocks `h - 9` to `h - 1` before the
    // activation and the eleven `h` to `h + 10` from it.
    let window: u64 = (u32::from(h) - 9..=u32::from(h) + 10)
        .map(|x| spacing(params, BlockHeight::from_u32(x)))
        .sum();
    assert_eq!(
        window,
        9 * 75 + 11 * 25,
        "{name}: nine blocks before H and eleven from it"
    );
    assert_eq!(
        chain_time(params, h - 10, h + 10),
        window,
        "{name}: a window's chain time is the sum of its blocks' spacings"
    );
    let (a, c) = (h - 300, h + 300);
    for b in [h - 300, h - 2, h - 1, h, h + 1, h + 300] {
        assert_eq!(
            chain_time(params, a, c),
            chain_time(params, a, b) + chain_time(params, b, c),
            "{name}: the span split at {b}"
        );
    }
}

#[test]
fn chain_time_is_the_sum_of_the_one_spacing_and_splits_at_any_height() {
    // Not in the contract's list. The sum must be built from the one spacing
    // function, so that the burial, the grace and the reclaim margin never read a
    // second copy of the spacing.
    for network in NETWORKS {
        let name = format!("{network:?}");
        assert_sums_of_the_one_spacing(&name, &network, real_activation(network));
    }
    let straddling = harness_network(Some(harness_nu7()));
    assert_sums_of_the_one_spacing("straddling LocalNetwork", &straddling, harness_nu7());
}

/// One params value held equal to the crate's own helpers at the boundary rows.
fn assert_equals_the_crate<P: Parameters>(name: &str, params: &P, h: BlockHeight) {
    for at in [h - 1, h, h + 1] {
        assert_eq!(
            spacing(params, at),
            pow_target_spacing(params, at).as_secs(),
            "{name}: the spacing at {at}"
        );
        assert_eq!(
            delta(params, at),
            u64::from(default_tx_expiry_delta(BranchId::for_height(params, at))),
            "{name}: the default expiry delta at {at}"
        );
    }
    for (from, n, ..) in burial_rows(h) {
        for to in [from + (n - 1), from + n] {
            assert_eq!(
                chain_time(params, from, to),
                target_time_between(params, from, to).as_secs(),
                "{name}: the chain time from {from} to {to}"
            );
        }
    }
}

#[test]
fn the_spacing_function_equals_the_crate_s_own_at_the_boundary_rows() {
    // The A-F6 pin (plan §3 "Step 5"): the pinned crate ships `pow_target_spacing`,
    // `target_time_between` and `default_tx_expiry_delta`, so ours is held equal
    // to them at the boundary rows above, on every params value above. Step 7m
    // swapped ours for them; the pin now holds the heights and the branch the
    // module reads them at, and its units.
    for network in NETWORKS {
        let name = format!("{network:?}");
        let h = real_activation(network);
        assert_equals_the_crate(&name, &network, h);
        assert_equals_the_crate(&format!("{name} without NU7"), &without_nu7(network), h);
    }
    let h = harness_nu7();
    assert_equals_the_crate("straddling LocalNetwork", &harness_network(Some(h)), h);
    assert_equals_the_crate("LocalNetwork { nu7: None }", &harness_network(None), h);
}
