//! §3.2i-2 2e-2b-vi (#315 slice 2) — the Mechanism-A ephemeral-slot RECLAIM: pure target-selection
//! logic + the outcome ladder. The async money orchestration (self-mint from the shielded pool to
//! the highest provably-dead ephemeral → sign → persist → broadcast) lives in
//! [`crate::wallet::Wallet::reclaim_ephemeral_slots`]; this module owns the parts that are testable
//! WITHOUT a wallet: the deadness gate, the highest-dead selection, and the [`ReclaimOutcome`] the
//! method returns.
//!
//! ## Why a reclaim exists (and why it is a self-mint, not an engine call)
//! The engine has NO un-reserve primitive and NEVER frees a leaked reservation — not on tx0 expiry
//! (`find_gap_start` advances ONLY on a MINED first-use). But `find_gap_start` advances past ANY
//! mined first-use — INCLUDING one we create. So paying a tiny amount from the wallet's OWN shielded
//! pool to the HIGHEST leaked ephemeral address, via a NORMAL single-step send (a plain
//! `Recipient::External` to the address's p2pkh form — reserves nothing, never trips
//! `check_ephemeral_address_reuse`), and mining it, advances `gap_start` PAST every lower leak in ONE
//! transaction — reopening the whole `[gap_start, gap_start + EPHEMERAL_GAP_LIMIT)` window (the lower
//! leaks fall below `gap_start` as harmless dead indices). Proven against the real engine by
//! `send::self_mint_on_the_highest_leaked_ephemeral_reopens_the_whole_window`.
//!
//! ## The deadness gate (the correction — a BLOCKER without it)
//! The reserved-ephemeral enumeration returns EVERY reserved-unused index — including a LIVE,
//! unexpired, in-flight tx0 (a legitimate exchange deposit). "Highest outstanding" is the
//! most-recently-reserved, i.e. the MOST LIKELY LIVE one; self-minting + sweeping its index would
//! race its tx1 and could CANCEL a real deposit. So the reclaim targets ONLY an index whose
//! reservation is PROVABLY DEAD — its exposure height is
//! [`EPHEMERAL_RECLAIM_DEADNESS_BLOCKS`](crate::constants::EPHEMERAL_RECLAIM_DEADNESS_BLOCKS) below
//! the tip (any tx0 that reserved it is past-expiry AND buried) — and picks the HIGHEST such index.
//! No provably-dead index ⇒ [`ReclaimOutcome::NothingToReclaim`] (the ceiling is genuinely
//! transient; the live tx0s self-heal when they mine). The "double-deposit-free" property holds ONLY
//! under this gate.
//!
//! ## Success is the reservation PROBE, never the pressure gauge
//! The v-3 pressure gauge is structurally BLIND to created-tx0 leaks (their
//! `transparent_received_outputs` rows exist from create-persist, so the `exclude_used` read drops
//! them — see [`crate::account::outstanding_ephemeral_reservations`]). So the reclaim NEVER keys any
//! decision or success signal on the gauge; the honest signal is a real reservation succeeding
//! (a subsequent TEX send no longer parking), which only happens once the self-mint MINES + BURIES.
//! The method returns [`ReclaimOutcome::Minted`] on broadcast — an HONEST "initiated", never
//! "succeeded": the window reopens on-chain a few blocks later, and the minted principal returns to
//! the wallet via the existing manual sweep (which reaches every reserved ephemeral over the wide
//! window, including the one we just funded).

use zcash_address::{ToAddress, ZcashAddress};
use zcash_client_backend::wallet::{Exposure, TransparentAddressMetadata};
use zcash_protocol::consensus::NetworkType;
use zcash_protocol::value::Zatoshis as ProtoZat;
use zcash_transparent::address::TransparentAddress;

use crate::error::WalletError;

/// The disposition of ONE [`reclaim_ephemeral_slots`](crate::wallet::Wallet::reclaim_ephemeral_slots)
/// invocation. §5.4: the ONLY money figure is `amount_zat` (the self-mint principal — a fixed,
/// non-secret constant, host-rendered for the disclosure); no address / recipient / txid crosses.
#[derive(Clone, Copy, PartialEq, Eq)]
#[non_exhaustive] // G2: a pub enum must grow without breaking a host's exhaustive match
pub enum ReclaimOutcome {
    /// No provably-dead leaked reservation to reclaim — NO money moved. Either the wallet has
    /// reserved no ephemeral (no TEX send ever), or every outstanding reservation is younger than
    /// the deadness margin (the window is genuinely TRANSIENT — its slots are held by tx0s that may
    /// still mine and self-heal; minting on one could cancel a live deposit, so the reclaim
    /// declines). The host renders "nothing to recover right now".
    NothingToReclaim,
    /// A self-mint was signed, persisted (§6.3), and ACCEPTED by the endpoint, paying `amount_zat`
    /// from the shielded pool to the highest provably-dead ephemeral address. This is an HONEST
    /// "initiated", not "done": the one-time-address window REOPENS once this tx mines + buries (a
    /// few blocks), and the `amount_zat` principal returns to the wallet via the manual sweep. The
    /// host discloses the honest cost (~4 marginal fees across two txs) and the "recover the moved
    /// amount with Recover funds once it confirms" step.
    Minted {
        /// The self-mint principal (a fixed constant — `EPHEMERAL_RECLAIM_MINT_ZAT`
        /// (crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT)); host-rendered, never logged.
        amount_zat: i64,
    },
    /// A self-mint was signed + persisted but the endpoint did NOT accept the broadcast (a transport
    /// miss on a flaky link). Money-safe + self-healing: the persisted-but-unbroadcast tx expires
    /// (~40 blocks) and frees its notes, so a re-run re-mints cleanly (no double-spend, no stuck
    /// state). The host renders "couldn't reach the network — try again"; `amount_zat` is the
    /// attempted principal (never left the wallet).
    NotBroadcast {
        /// The attempted self-mint principal (never spent — the tx did not reach an endpoint).
        amount_zat: i64,
    },
}

/// §5.4 REDACTING `Debug` (NOT derived): `amount_zat` is a money figure (though a fixed public
/// constant here) — keep the money-never-`{:?}`-leak discipline uniform with the sweep/parked DTOs.
impl std::fmt::Debug for ReclaimOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NothingToReclaim => f.write_str("NothingToReclaim"),
            Self::Minted { .. } => f
                .debug_struct("Minted")
                .field("amount_zat", &"<redacted §5.4>")
                .finish(),
            Self::NotBroadcast { .. } => f
                .debug_struct("NotBroadcast")
                .field("amount_zat", &"<redacted §5.4>")
                .finish(),
        }
    }
}

/// `true` iff a reservation exposed at `exposed_at_height` is PROVABLY DEAD at `tip` — any tx0 that
/// reserved it is past-expiry AND that finding is buried beyond the reorg horizon
/// ([`EPHEMERAL_RECLAIM_DEADNESS_BLOCKS`](crate::constants::EPHEMERAL_RECLAIM_DEADNESS_BLOCKS)).
/// SATURATING: an exposure height somehow ABOVE the tip (a mid-reorg read) yields `0`, i.e. NOT
/// dead — fail-safe (never reclaim an index that could still be live).
pub(crate) fn is_provably_dead(exposed_at_height: u32, tip: u32) -> bool {
    tip.saturating_sub(exposed_at_height) >= crate::constants::EPHEMERAL_RECLAIM_DEADNESS_BLOCKS
}

/// Select the HIGHEST provably-dead reserved ephemeral to reclaim (Mechanism A: minting a first-use
/// on the highest dead index advances `gap_start` past EVERY lower leak in one tx). Pure over the
/// enumerated `(address, metadata)` set + the `tip`; returns the address to self-mint to, or `None`
/// when no reservation is provably dead (⇒ [`ReclaimOutcome::NothingToReclaim`]).
///
/// A candidate qualifies iff its exposure is a known [`Exposure::Exposed`] height that
/// [`is_provably_dead`] AND it carries a known [`address_index`](TransparentAddressMetadata::address_index)
/// (the ordering key). An `Exposure::Unknown`/`CannotKnow` or an index-less standalone entry is NOT
/// provably dead ⇒ skipped (fail-safe). "Highest" = the largest child index — precisely the reclaim
/// target that reopens the widest window (indices below it fall under `gap_start` harmlessly). Ties
/// on index cannot occur (an index is reserved at most once), but a stable `max_by_key` is used
/// regardless.
pub(crate) fn highest_dead_target(
    reserved: &[(TransparentAddress, TransparentAddressMetadata)],
    tip: u32,
) -> Option<TransparentAddress> {
    reserved
        .iter()
        .filter_map(|(addr, md)| {
            let idx = md.address_index()?.index();
            let at = match md.exposure() {
                Exposure::Exposed { at_height, .. } => u32::from(at_height),
                Exposure::Unknown | Exposure::CannotKnow => return None,
            };
            is_provably_dead(at, tip).then_some((idx, *addr))
        })
        .max_by_key(|(idx, _)| *idx)
        .map(|(_, addr)| addr)
}

/// Build the self-mint's ZIP-321 request: pay `amount_zat` to `target`'s PLAIN p2pkh form on
/// `network`. The recipient is the ordinary transparent (p2pkh) encoding — NOT the TEX encoding —
/// so [`propose_core`](crate::send) lowers it as a normal single-step `Recipient::External` send
/// that reserves NO ephemeral and never trips `check_ephemeral_address_reuse` (proven by the
/// vectors-first engine test). An engine ephemeral is always p2pkh; a `ScriptHash` target is a
/// structural surprise ⇒ fail-closed [`ProposeFailed`](WalletError::ProposeFailed) (never mint to an
/// address the wallet could not have derived). The `amount_zat` is the fixed
/// [`EPHEMERAL_RECLAIM_MINT_ZAT`](crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT) constant, so
/// `const_from_u64` is total (no overflow path).
pub(crate) fn mint_request(
    network: NetworkType,
    target: TransparentAddress,
    amount_zat: u64,
) -> Result<zip321::TransactionRequest, WalletError> {
    let TransparentAddress::PublicKeyHash(hash) = target else {
        return Err(WalletError::ProposeFailed);
    };
    let recipient = ZcashAddress::from_transparent_p2pkh(network, hash);
    let payment = zip321::Payment::without_memo(recipient, ProtoZat::const_from_u64(amount_zat));
    zip321::TransactionRequest::new(vec![payment]).map_err(|_| WalletError::ProposeFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::EPHEMERAL_RECLAIM_DEADNESS_BLOCKS;
    use zcash_client_backend::wallet::GapMetadata;
    use zcash_primitives::transaction::builder::DEFAULT_TX_EXPIRY_DELTA;
    use zcash_protocol::consensus::BlockHeight;
    use zcash_transparent::keys::{NonHardenedChildIndex, TransparentKeyScope};

    fn ephemeral(tag: u8) -> TransparentAddress {
        TransparentAddress::PublicKeyHash([tag; 20])
    }

    /// A reserved-ephemeral metadata with a known index + `Exposed` height (the shape the engine
    /// enumeration returns for a reserved ephemeral). `next_check_time = None` (irrelevant here).
    fn meta(index: u32, exposed_at: u32) -> TransparentAddressMetadata {
        TransparentAddressMetadata::derived(
            TransparentKeyScope::EPHEMERAL,
            NonHardenedChildIndex::from_index(index).expect("valid index"),
            Exposure::Exposed {
                at_height: BlockHeight::from_u32(exposed_at),
                gap_metadata: GapMetadata::DerivationUnknown,
            },
            None,
        )
    }

    #[test]
    fn deadness_gate_is_the_expiry_plus_reorg_margin() {
        let tip = 1_000;
        // Exactly AT the margin is dead (boundary: accept).
        assert!(is_provably_dead(
            tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS,
            tip
        ));
        // One block YOUNGER than the margin is NOT dead (could still hold a live tx0).
        assert!(!is_provably_dead(
            tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS + 1,
            tip
        ));
        // Comfortably old ⇒ dead; brand-new (this block) ⇒ not dead.
        assert!(is_provably_dead(0, tip));
        assert!(!is_provably_dead(tip, tip));
        // SATURATING: exposure above tip (a mid-reorg read) is NOT dead — fail-safe.
        assert!(!is_provably_dead(tip + 5, tip));
    }

    #[test]
    fn deadness_margin_meets_the_engine_arithmetic_for_a_full_reorg_burial() {
        // MONEY-SAFETY DERIVATION PIN (crypto+security review; review C hardened this to
        // DERIVE from the engine symbol, not restate the literal). Re-derive the boundary from first
        // principles against the AUDITED constants so an upstream expiry-delta bump moves the whole
        // scenario in lockstep — the test must never pass merely because it echoes the definition.
        let delta = DEFAULT_TX_EXPIRY_DELTA; // the Builder's expiry delta (engine symbol, not a literal)
        let reorg = crate::constants::REORG_MAX_BLOCKS;
        // A reservation stamps `exposed = the reserve-time chain tip H`; the tx0's target is `H+1`, so
        // its expiry (LAST mineable block) is `E = H + 1 + delta`. After a maximal `reorg`-deep reorg
        // from tip `T` the next block to mine is `(T − reorg) + 1`, so the tx0 is STILL mineable while
        // `(T − reorg) + 1 ≤ E`, i.e. `T ≤ E − 1 + reorg`. Measuring from `exposed`, the LAST still-live
        // tip is offset `(1 + delta) − 1 + reorg = delta + reorg`, and the FIRST provably-dead tip is
        // offset `delta + reorg + 1` — which is exactly `(delta + 1) + reorg`, the margin.
        let h = 500_000u32;
        let last_live_tip = h + (delta + reorg); // one short of burial: a maximal reorg re-enables tx0
        let first_dead_tip = h + (delta + reorg + 1); // tx0's expiry buried beyond a full reorg horizon
        assert!(
            !is_provably_dead(h, last_live_tip),
            "one block short of a full reorg horizon — a maximal reorg re-enables the tx0, must NOT be dead",
        );
        assert!(
            is_provably_dead(h, first_dead_tip),
            "the tx0's expiry block is buried beyond a full reorg horizon — dead",
        );
        // The constant IS that first-dead offset, derived from the engine symbol (guards a revert to the
        // off-by-one `delta + reorg` = 140, which left the boundary one block short of a full reorg).
        assert_eq!(
            EPHEMERAL_RECLAIM_DEADNESS_BLOCKS,
            delta + reorg + 1,
            "the deadness margin must be (tx-expiry-delta + 1) + reorg-horizon",
        );
        // Current concrete value — a TRIPWIRE, not the derivation: if this fails, the engine's expiry
        // delta or the reorg horizon changed. Re-derive the scenario above, then update this line.
        assert_eq!(EPHEMERAL_RECLAIM_DEADNESS_BLOCKS, 141);
    }

    #[test]
    fn mint_amount_clears_the_engine_sweep_economic_floor() {
        // MONEY-SAFETY FLOOR PIN (review C, H4): the reclaim's "the moved principal returns via
        // the sweep" promise holds ONLY while the minted output is a SELECTABLE sweep input. The engine
        // excludes any UTXO ≤ MARGINAL_FEE from input selection BEFORE aggregation (the
        // EPHEMERAL_SWEEP_THRESHOLD_ZAT doc, engine-verified), and the sweep-back tx pays ~MINIMUM_FEE.
        // Tie MINT_ZAT to the ENGINE fee constants so a drive-by lowering below sweepability trips HERE
        // (a LOST-FUNDS guard: below the floor the principal strands as unsweepable dust, silently
        // breaking the honest-cost promise).
        use zcash_primitives::transaction::fees::zip317::{MARGINAL_FEE, MINIMUM_FEE};
        let marginal = MARGINAL_FEE.into_u64();
        let minimum = MINIMUM_FEE.into_u64();
        assert!(
            crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT > marginal,
            "mint principal ({}) must exceed the engine's per-UTXO dust floor ({marginal}) to be a \
             selectable sweep input",
            crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT,
        );
        assert!(
            crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT > minimum,
            "mint principal ({}) must exceed the sweep-back fee (~{minimum}) so the recovered amount \
             nets positive",
            crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT,
        );
    }

    #[test]
    fn picks_the_highest_provably_dead_index() {
        let tip = 1_000;
        let old = tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS - 10; // comfortably dead
        // Indices 0,1,2 all dead (reserved at old heights); the highest (2) is the target — minting
        // on it advances gap_start past 0 and 1 too.
        let reserved = vec![
            (ephemeral(0xA0), meta(0, old)),
            (ephemeral(0xA2), meta(2, old)),
            (ephemeral(0xA1), meta(1, old)),
        ];
        assert_eq!(
            highest_dead_target(&reserved, tip),
            Some(ephemeral(0xA2)),
            "the highest dead index is the reclaim target"
        );
    }

    #[test]
    fn never_targets_a_live_index_even_if_it_is_the_highest() {
        // The BLOCKER guard: leaks 0,1 are dead but index 2 is a LIVE in-flight tx0 (young).
        // "Highest outstanding" would be 2 — but self-minting on it races a legitimate deposit, so
        // the reclaim targets the highest DEAD (1), never the live one.
        let tip = 1_000;
        let old = tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS - 10;
        let young = tip - 5; // in-flight, not provably dead
        let reserved = vec![
            (ephemeral(0xB0), meta(0, old)),
            (ephemeral(0xB1), meta(1, old)),
            (ephemeral(0xB2), meta(2, young)),
        ];
        assert_eq!(
            highest_dead_target(&reserved, tip),
            Some(ephemeral(0xB1)),
            "the live highest index is skipped; the highest DEAD index wins"
        );
    }

    #[test]
    fn no_dead_reservation_is_nothing_to_reclaim() {
        // Every reservation is young (a genuinely transient ceiling — the tx0s may still mine). No
        // target ⇒ the method returns NothingToReclaim (never mint on a maybe-live index).
        let tip = 1_000;
        let young = tip - 5;
        let reserved = vec![
            (ephemeral(0xC0), meta(0, young)),
            (ephemeral(0xC1), meta(1, young)),
        ];
        assert_eq!(highest_dead_target(&reserved, tip), None);
        // An empty set (no ephemeral ever reserved) is also None.
        assert_eq!(highest_dead_target(&[], tip), None);
    }

    #[test]
    fn skips_reservations_with_unknown_exposure_or_no_index() {
        // Fail-safe: an `Unknown` exposure is not provably dead (skip); a well-known dead index is
        // still selected alongside it.
        let tip = 1_000;
        let old = tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS - 10;
        let unknown = TransparentAddressMetadata::derived(
            TransparentKeyScope::EPHEMERAL,
            NonHardenedChildIndex::from_index(9).expect("idx"),
            Exposure::Unknown,
            None,
        );
        let reserved = vec![(ephemeral(0xD9), unknown), (ephemeral(0xD3), meta(3, old))];
        assert_eq!(
            highest_dead_target(&reserved, tip),
            Some(ephemeral(0xD3)),
            "an unknown-exposure entry (index 9) is skipped; the known dead index 3 wins"
        );
    }

    #[test]
    fn mint_request_lowers_to_a_plain_transparent_payment() {
        // The self-mint recipient is the p2pkh encoding (single-step; the vectors-first test proves
        // the engine lowers it as External, reserving nothing). One leg, the exact amount, no memo.
        let req = mint_request(
            NetworkType::Regtest,
            ephemeral(0x7A),
            crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT,
        )
        .expect("builds");
        let payments: Vec<_> = req.payments().values().cloned().collect();
        assert_eq!(payments.len(), 1, "one payment leg");
        assert_eq!(
            payments[0].amount(),
            Some(ProtoZat::const_from_u64(
                crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT
            )),
            "the mint pays exactly the reclaim constant"
        );
        // The recipient re-encodes as a plain transparent (p2pkh) address — NOT a TEX, so the send
        // path treats it as single-step.
        assert!(
            payments[0].recipient_address().encode().starts_with('t'),
            "a plain transparent recipient, not a TEX one-time address"
        );
    }

    #[test]
    fn mint_request_refuses_a_non_p2pkh_target() {
        // Fail-closed: an engine ephemeral is always p2pkh, so a scripthash is a structural
        // surprise — never mint to an address the wallet could not have derived.
        assert!(matches!(
            mint_request(
                NetworkType::Regtest,
                TransparentAddress::ScriptHash([0x11; 20]),
                crate::constants::EPHEMERAL_RECLAIM_MINT_ZAT,
            ),
            Err(WalletError::ProposeFailed)
        ));
    }

    #[test]
    fn reclaim_outcome_debug_redacts_the_amount() {
        let dbg = format!("{:?}", ReclaimOutcome::Minted { amount_zat: 50_000 });
        assert!(dbg.contains("<redacted §5.4>"), "amount redacted: {dbg}");
        assert!(
            !dbg.contains("50000") && !dbg.contains("50_000"),
            "no raw amount"
        );
        assert_eq!(
            format!("{:?}", ReclaimOutcome::NothingToReclaim),
            "NothingToReclaim"
        );
    }
}
