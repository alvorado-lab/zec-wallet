//! The chain's target time, height by height, across the NU7 activation (ADR-0579
//! Decision 6; `docs/plan/nu7-wave.md` §3 steps 4s and 7m).
//!
//! ZIP 218 divides the target block spacing by three from the NU7 activation: 75 s
//! before it, 25 s from it. A count of blocks therefore no longer says how long a span
//! is, and a rule written in blocks changes its length at the activation. Three
//! functions answer that, each from the activation heights the consensus parameters
//! carry:
//!
//! - [`target_spacing_secs`]: the target time between the block at a height and its
//!   parent;
//! - [`default_expiry_delta`]: the builder's DEFAULT expiry delta for a transaction
//!   that targets a height (a canonical ZIP 318 crossing gets a rolling expiry
//!   instead, so this is not that transaction's expiry);
//! - [`chain_time_between`]: the target time from one height to another, each block
//!   counted at its own spacing, so a span that straddles the activation sums its two
//!   sides.
//!
//! **Upstream's values, in the wallet's units.** Each is the pinned crates' own
//! function: `zcash_protocol`'s `pow_target_spacing` and `target_time_between`, and
//! `zcash_primitives`' `default_tx_expiry_delta` at the branch of the target height,
//! which is how the builder reads it. Step 4s carried its own copy of ZIP 218's ratio
//! and default, a second source time-boxed until upstream shipped these; step 7m
//! swapped it for them (A-F6), so the wallet holds no spacing or delta of its own and
//! this module only reads upstream's in seconds and blocks.
//!
//! **Below Blossom** upstream's values are 150 s and 20 blocks (step 4s's copy kept
//! 75 s and 40 there). A span can start that low (a restore with no birthday, the badge on an
//! old row), but no mainnet or testnet tx0 is live there and the span is millions of blocks, so
//! neither value changes an answer. Parameters without NU7 give 75 s and 40 blocks from Blossom.
//!
//! The burial predicate (`stranded::irreversibly_buried`) reads
//! [`chain_time_between`], and the reclaim margin (`reclaim::is_provably_dead`) reads
//! [`default_expiry_delta`]. [`target_spacing_secs`] has no caller (the grace fix is after 0.0.5).

use zcash_primitives::transaction::builder::default_tx_expiry_delta;
use zcash_protocol::consensus::{
    BlockHeight, BranchId, Parameters, pow_target_spacing, target_time_between,
};

/// The target time between the block at `height` and its parent, in seconds:
/// `pow_target_spacing`'s. That is 25 from the NU7 activation, 75 from Blossom below
/// it (and from Blossom at every height where `params` carry no NU7 height), and 150
/// below Blossom.
#[cfg_attr(not(test), allow(dead_code))] // for the grace fix deferred past 0.0.5 (nu7-wave.md §5)
pub(crate) fn target_spacing_secs<P: Parameters>(params: &P, height: BlockHeight) -> u64 {
    pow_target_spacing(params, height).as_secs()
}

/// The default expiry delta, in blocks, of a transaction that targets `height`:
/// `default_tx_expiry_delta` for the branch in effect at `height`, the branch the
/// builder reads it at. That is 120 from the NU7 activation, 40 from Blossom below it
/// (and from Blossom at every height where `params` carry no NU7 height), and 20 below
/// Blossom. The builder sets such a transaction's expiry to `height` plus this, so
/// `height` is the target height (the tip plus one), not the tip. It is the builder's
/// DEFAULT only: a canonical ZIP 318 crossing gets a rolling expiry from
/// `zcash_protocol::zip318::expiry_height` instead
/// (`zcash_client_backend-0.25.0/src/data_api/wallet.rs:2015-2026`), so a caller must
/// not read this as such a transaction's expiry.
pub(crate) fn default_expiry_delta<P: Parameters>(params: &P, height: BlockHeight) -> u32 {
    default_tx_expiry_delta(BranchId::for_height(params, height))
}

/// The target chain time from the block at `from` to the block at `to`, in seconds:
/// `target_time_between`'s, the sum of [`target_spacing_secs`] over the heights in
/// `(from, to]`, so each block counts the time from its parent at its own spacing. A
/// span that straddles the activation sums 75 s per block below it and 25 s per block
/// from it. Zero when `to <= from`.
pub(crate) fn chain_time_between<P: Parameters>(
    params: &P,
    from: BlockHeight,
    to: BlockHeight,
) -> u64 {
    target_time_between(params, from, to).as_secs()
}

#[cfg(test)]
mod closed_form_tests {
    //! The sum is upstream's closed form; this row holds it exact in `u64` across the
    //! whole height range, as the wallet reads it. Its definition, the per-height sum, and
    //! its agreement with upstream's helpers at the activation are `chain_time_tests`'s
    //! rows.

    use super::*;
    use zcash_client_backend::data_api::testing::TestBuilder;
    use zcash_protocol::local_consensus::LocalNetwork;

    /// The harness network with Blossom at height 0 and NU7 at `nu7`, so every height in
    /// the range is post-Blossom (below Blossom the spacing is the crate's 150 s). Built by
    /// updating upstream's default, as `ironwood_spendability` does, so no field behind a
    /// `cfg` is named here.
    fn local(nu7: Option<u32>) -> LocalNetwork {
        LocalNetwork {
            blossom: Some(BlockHeight::from_u32(0)),
            nu7: nu7.map(BlockHeight::from_u32),
            ..TestBuilder::<(), ()>::DEFAULT_NETWORK
        }
    }

    #[test]
    fn the_sum_does_not_overflow_across_the_whole_height_range() {
        let whole = |params: &LocalNetwork| {
            chain_time_between(
                params,
                BlockHeight::from_u32(0),
                BlockHeight::from_u32(u32::MAX),
            )
        };
        assert_eq!(whole(&local(None)), u64::from(u32::MAX) * 75);
        assert_eq!(
            whole(&local(Some(1_001))),
            1_000 * 75 + (u64::from(u32::MAX) - 1_000) * 25
        );
    }
}
