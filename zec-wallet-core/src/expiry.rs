//! The expiry kernel: whether a transaction this wallet signed can still be mined
//! (ADR-0579 Decision 4; `docs/plan/nu7-wave.md` §3 step 4).
//!
//! A transaction signed under consensus branch `B` can be mined only in a block whose
//! branch is `B` (ZIP 259: its signatures commit to the branch id). So it has two
//! bounds: its stored ZIP-203 expiry, and the activation height `H` that ends `B`'s
//! range. Its effective expiry is the earlier of the two, never later than `H - 1`.
//!
//! **One kernel, five readers.** This module is the only place that compares an expiry
//! with a height. The resubmission pass (`send::rebroadcast_group`), the delivery
//! obligation (`delivery::enumerate`, `delivery::DeliveryView::state_of`), the history
//! page (`history`) and the endpoint status check (`enhance::chain_status`) each ask it
//! and keep no comparison of their own, so no two of them can disagree about whether a
//! payment is dead.
//!
//! **`H` comes from the crate.** It is the upper bound of the transaction's own branch,
//! `BranchId::height_bounds(params).1`, never the hand-kept `KNOWN_UPGRADES` (whose job
//! is the anchor ceiling). The branch is read from the transaction's own header
//! ([`crate::enhance::header_branch`]).
//!
//! **The failure direction.** Every way of not knowing leaves the stored expiry
//! standing: an unreadable header, a branch the crate does not know, a branch with no
//! next activation, a `None` from `height_bounds`. An unreadable header never shortens
//! an expiry.
//!
//! §5.4: heights and branch ids only; nothing here sees a txid, an amount or an address.

use zcash_protocol::consensus::{BlockHeight, BranchId, Parameters};

use crate::state::CancelPhase;

/// The kernel's answer: the last height a transaction can be mined at, if any.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EffectiveExpiry {
    /// Stored expiry 0 ("never expires") and no next activation: any height.
    Never,
    /// The last height it can be mined at.
    Through(u32),
    /// The next activation is at height 0 or 1, so `H - 1` would floor at 0, which a
    /// stored expiry reads as "never". It can be mined nowhere.
    Nowhere,
}

impl EffectiveExpiry {
    /// Whether the transaction is past its expiry with the chain at `tip`: no block
    /// after `tip` can carry it. `Through(h)` is past when `h <= tip`, the edge the
    /// wallet has always used (the next block, `tip + 1`, is above `h`).
    pub(crate) fn is_past(self, tip: u32) -> bool {
        match self {
            Self::Never => false,
            Self::Through(last) => last <= tip,
            Self::Nowhere => true,
        }
    }

    /// Whether a block at `height` can carry the transaction.
    pub(crate) fn admits(self, height: u32) -> bool {
        match self {
            Self::Never => true,
            Self::Through(last) => height <= last,
            Self::Nowhere => false,
        }
    }

    /// Whether the transaction is past its expiry AND that expiry is irreversibly
    /// buried at `tip` (ADR-0579 Decision 6), so no reorg the wallet follows can make
    /// it minable again. Every irreversible step waits for this, never for the bare
    /// expiry: a requeue, a strand, and through the requeue a swap deposit's purge.
    pub(crate) fn is_buried<P: Parameters>(self, params: &P, tip: u32) -> bool {
        match self {
            Self::Never => false,
            Self::Through(last) => {
                last <= tip && crate::stranded::irreversibly_buried(params, last, tip)
            }
            Self::Nowhere => true,
        }
    }
}

/// THE kernel: the effective expiry of a transaction whose consensus-serialized bytes
/// start with `header` (the first 12 bytes are enough) and whose stored expiry is
/// `stored` (`0` = none), under `params`.
pub(crate) fn effective_expiry<P: Parameters>(
    params: &P,
    header: &[u8],
    stored: u32,
) -> EffectiveExpiry {
    effective_expiry_before(next_activation(params, header), stored)
}

/// The activation that ends the branch `header` names: `H`, or `None` where the header
/// is unreadable, the branch is one the crate does not know, or the branch has no
/// later activation under `params`.
pub(crate) fn next_activation<P: Parameters>(params: &P, header: &[u8]) -> Option<BlockHeight> {
    let branch = BranchId::try_from(crate::enhance::header_branch(header)?).ok()?;
    branch.height_bounds(params)?.1
}

/// The kernel given `H` directly. `height_bounds` never yields `H = 0` and yields
/// `H = 1` only where a lower bound is 0, so [`EffectiveExpiry::Nowhere`] is reached
/// (and tested) here.
pub(crate) fn effective_expiry_before(
    next_activation: Option<BlockHeight>,
    stored: u32,
) -> EffectiveExpiry {
    let Some(activation) = next_activation else {
        return match stored {
            0 => EffectiveExpiry::Never,
            stored => EffectiveExpiry::Through(stored),
        };
    };
    let last = u32::from(activation).saturating_sub(1);
    if last == 0 {
        return EffectiveExpiry::Nowhere;
    }
    match stored {
        0 => EffectiveExpiry::Through(last),
        stored => EffectiveExpiry::Through(stored.min(last)),
    }
}

/// The history reading of a transaction a network upgrade cancelled: `Some((H, its
/// phase))`, `H` the activation height the network stopped accepting it at, when the
/// upgrade made it unminable at `tip` while its stored expiry alone would not have, or
/// would not have yet. `None` for every other transaction: one that can still be mined,
/// and one whose own stored expiry ended it first (plain `Expired`). For
/// [`EffectiveExpiry::Nowhere`] it is still the `H` the inner function was given.
///
/// The phase (ADR-0579 Decision 4): not final until the effective expiry, the block
/// before `H`, is irreversibly buried; then final but not released until the stored
/// expiry passes, the height the engine frees the notes at (never, for a stored 0);
/// then released.
pub(crate) fn upgrade_cancel<P: Parameters>(
    params: &P,
    header: &[u8],
    stored: u32,
    tip: u32,
) -> Option<(u32, CancelPhase)> {
    // No next activation leaves the stored expiry standing, so nothing was cancelled.
    let activation = next_activation(params, header)?;
    let effective = effective_expiry_before(Some(activation), stored);
    match effective {
        EffectiveExpiry::Never => return None,
        EffectiveExpiry::Through(last) if stored != 0 && last >= stored => return None,
        EffectiveExpiry::Through(_) | EffectiveExpiry::Nowhere => {}
    }
    if !effective.is_past(tip) {
        return None;
    }
    let released = effective_expiry_before(None, stored).is_past(tip);
    let phase = if !effective.is_buried(params, tip) {
        CancelPhase::NotFinal
    } else if released {
        CancelPhase::Released
    } else {
        CancelPhase::FinalNotReleased
    };
    Some((u32::from(activation), phase))
}

#[cfg(test)]
mod tests {
    //! Unit rows for this module's own helpers. The contract's named rows for the
    //! kernel and its readers live with the test author's file.

    use super::*;

    #[test]
    fn past_and_admits_are_complements_one_block_apart() {
        let through = EffectiveExpiry::Through(100);
        assert!(!through.is_past(99));
        assert!(through.is_past(100));
        assert!(through.admits(100));
        assert!(!through.admits(101));
        assert!(!EffectiveExpiry::Never.is_past(u32::MAX));
        assert!(EffectiveExpiry::Nowhere.is_past(0));
        assert!(!EffectiveExpiry::Nowhere.admits(1));
    }
}
