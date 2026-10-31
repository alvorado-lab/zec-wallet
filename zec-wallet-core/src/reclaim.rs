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
//! reservation is PROVABLY DEAD — any tx0 that reserved it is past its expiry AND that expiry is
//! irreversibly buried ([`is_provably_dead`]: 241 blocks below the tip before the NU7 activation,
//! [`EPHEMERAL_RECLAIM_DEADNESS_BLOCKS`](crate::constants::EPHEMERAL_RECLAIM_DEADNESS_BLOCKS), and
//! 721 from it) — and picks the HIGHEST such index.
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
use zcash_protocol::consensus::{BlockHeight, NetworkType, Parameters};
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
    /// (~50 minutes: 40 blocks before NU7, 120 from it) and frees its notes, so a re-run re-mints
    /// cleanly (no double-spend, no stuck state). The host renders "couldn't reach the network —
    /// try again"; `amount_zat` is the attempted principal (never left the wallet).
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

/// `true` iff a reservation exposed at `exposed_at_height` is PROVABLY DEAD at `tip` on `params`:
/// any tx0 that reserved it is past its expiry AND that expiry is
/// [irreversibly buried](crate::stranded::irreversibly_buried) at `tip` (ADR-0579 Decision 6).
///
/// The reservation stamps the reserve-time tip, so the tx0 targets the NEXT block,
/// `exposed_at_height + 1`, and the builder gives it the default delta of THAT height's branch
/// ([`crate::chain_time::default_expiry_delta`]): an index exposed at the block before the NU7
/// activation targets the activation and takes NU7's 120, not the 40 of the branch it was exposed
/// under. Its last mineable block is that target plus the delta, and the index is dead once the
/// burial predicate says that block is irreversibly buried, each block of the span counted at its
/// own spacing. So the margin `tip − exposed_at_height` is `(delta + 1)` plus the burial depth:
/// 241 below the activation ([`EPHEMERAL_RECLAIM_DEADNESS_BLOCKS`](crate::constants::EPHEMERAL_RECLAIM_DEADNESS_BLOCKS)),
/// `(120 + 1) + 600` = 721 from it, and between the two where the span straddles it.
///
/// FAIL-SAFE: an exposure height somehow AT or ABOVE the tip (a mid-reorg read) puts the expiry
/// above the tip, which the predicate reads as not buried, so NOT dead (never reclaim an index that
/// could still be live). The heights saturate at the top of the range, which delays, never hastens.
pub(crate) fn is_provably_dead<P: Parameters>(
    params: &P,
    exposed_at_height: u32,
    tip: u32,
) -> bool {
    let target = BlockHeight::from_u32(exposed_at_height) + 1;
    let expiry = target + crate::chain_time::default_expiry_delta(params, target);
    crate::stranded::irreversibly_buried(params, u32::from(expiry), tip)
}

/// Select the HIGHEST provably-dead reserved ephemeral to reclaim (Mechanism A: minting a first-use
/// on the highest dead index advances `gap_start` past EVERY lower leak in one tx). Pure over the
/// enumerated `(address, metadata)` set + the `tip` on `params`; returns the address to self-mint
/// to, or `None` when no reservation is provably dead (⇒ [`ReclaimOutcome::NothingToReclaim`]).
///
/// A candidate qualifies iff its exposure is a known [`Exposure::Exposed`] height that
/// [`is_provably_dead`] AND it carries a known [`address_index`](TransparentAddressMetadata::address_index)
/// (the ordering key). An `Exposure::Unknown`/`CannotKnow` or an index-less standalone entry is NOT
/// provably dead ⇒ skipped (fail-safe). "Highest" = the largest child index — precisely the reclaim
/// target that reopens the widest window (indices below it fall under `gap_start` harmlessly). Ties
/// on index cannot occur (an index is reserved at most once), but a stable `max_by_key` is used
/// regardless.
pub(crate) fn highest_dead_target<P: Parameters>(
    params: &P,
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
            is_provably_dead(params, at, tip).then_some((idx, *addr))
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
    use zcash_protocol::consensus::{BlockHeight, MAIN_NETWORK};
    use zcash_transparent::keys::{NonHardenedChildIndex, TransparentKeyScope};

    /// A mainnet tip above Blossom and below the NU7 activation, where an index's margin is
    /// `EPHEMERAL_RECLAIM_DEADNESS_BLOCKS`.
    const PRE_NU7_TIP: u32 = 2_000_000;

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
        let tip = PRE_NU7_TIP;
        // Exactly AT the margin is dead (boundary: accept).
        assert!(is_provably_dead(
            &MAIN_NETWORK,
            tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS,
            tip
        ));
        // One block YOUNGER than the margin is NOT dead (could still hold a live tx0).
        assert!(!is_provably_dead(
            &MAIN_NETWORK,
            tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS + 1,
            tip
        ));
        // Comfortably old ⇒ dead; brand-new (this block) ⇒ not dead.
        assert!(is_provably_dead(&MAIN_NETWORK, 0, tip));
        assert!(!is_provably_dead(&MAIN_NETWORK, tip, tip));
        // SATURATING: exposure above tip (a mid-reorg read) is NOT dead — fail-safe.
        assert!(!is_provably_dead(&MAIN_NETWORK, tip + 5, tip));
    }

    #[test]
    fn deadness_margin_meets_the_engine_arithmetic_for_a_full_reorg_burial() {
        // MONEY-SAFETY DERIVATION PIN, re-derived for the irreversible burial (ADR-0579 Decision 6;
        // `docs/plan/nu7-wave.md` §3 steps 5 and 7m). Both inputs are DERIVED from the audited
        // sources, never restated: the delta is the builder's own `default_tx_expiry_delta` at the
        // tx0's target branch, and the depth is the burial predicate's, found by asking
        // `irreversibly_buried` where it first holds. A change to either moves the margin in
        // lockstep — the test must never pass merely because it echoes the definition. The gate's
        // own boundary at this margin is the row
        // `the_deadness_gate_is_dead_exactly_at_the_derived_margin_below_the_activation`, in
        // `margin_across_the_activation` below, which reads the gate's height-aware signature.
        use zcash_primitives::transaction::builder::default_tx_expiry_delta;
        use zcash_protocol::consensus::{BranchId, MAIN_NETWORK};
        let h = 2_000_000u32; // an exposure above Blossom and far below the NU7 activation
        let target = BlockHeight::from_u32(h + 1);
        let delta = default_tx_expiry_delta(BranchId::for_height(&MAIN_NETWORK, target));
        // A reservation stamps `exposed = the reserve-time chain tip H`; the tx0's target is `H+1`, so
        // its expiry (LAST mineable block) is `E = H + 1 + delta`.
        let expiry = h + 1 + delta;
        // The burial depth: the first tip at which `E` is irreversibly owned by the chain. Short of it,
        // a reorg the burial covers can rewind the chain below `E` and mine the tx0 there.
        let depth = (1..=1_000u32)
            .find(|d| crate::stranded::irreversibly_buried(&MAIN_NETWORK, expiry, expiry + d))
            .expect("the expiry is buried within 1,000 blocks");
        // So the LAST still-live tip is `E + depth − 1` and the FIRST provably-dead tip `E + depth`:
        // offset `(delta + 1) + depth` from `exposed`. The constant IS that offset below the
        // activation (guards the off-by-one `delta + depth`, one block short of the burial).
        assert_eq!(
            EPHEMERAL_RECLAIM_DEADNESS_BLOCKS,
            (delta + 1) + depth,
            "the deadness margin must be (tx-expiry-delta + 1) + the burial depth",
        );
        // Current concrete value — a TRIPWIRE, not the derivation: if this fails, the engine's
        // pre-NU7 delta or the burial changed. Re-derive the scenario above, then update this line.
        assert_eq!(EPHEMERAL_RECLAIM_DEADNESS_BLOCKS, 241);
    }

    /// **The tester-build gate** (`docs/plan/nu7-wave.md` §5): no tester build ships from a
    /// tree where this is red if that build can still be running at mainnet height 3,543,404.
    /// From the NU7 activation the builder's default expiry delta is 120 blocks
    /// (`default_tx_expiry_delta` at the `Nu7` branch, `zcash_primitives` 0.31). An index
    /// exposed at `H - 1`, the block before the activation, has a tx0 that targets `H`, so its
    /// expiry is `H + 120`, and until the burial predicate holds for that block a reorg the
    /// burial covers can still mine the tx0: 600 blocks of 25 s. The last live tip is therefore
    /// `(H - 1) + 720`, which the contract fixes (step 5: "not dead at 720, dead at 721"). Both
    /// terms are read from the crate and the predicate, never from `REORG_MAX_BLOCKS`; a margin
    /// derived from the pre-NU7 delta (141, then 241) reads the index dead at that tip and the
    /// reclaim could self-mint over a deposit in flight (the dead side, 721, is another row's).
    #[test]
    fn reclaim_margin_covers_the_nu7_delta() {
        use zcash_primitives::transaction::builder::default_tx_expiry_delta;
        use zcash_protocol::consensus::{BranchId, NetworkUpgrade, Parameters};
        // The mainnet NU7 activation, read from the pinned crate, and the block before it.
        let nu7 = MAIN_NETWORK
            .activation_height(NetworkUpgrade::Nu7)
            .expect("mainnet NU7");
        let h = u32::from(nu7) - 1;
        // The tx0 targets `h + 1 = H` and takes the delta of THAT block's branch.
        let delta = default_tx_expiry_delta(BranchId::for_height(&MAIN_NETWORK, nu7));
        let expiry = u32::from(nu7) + delta;
        // The burial depth of the expiry block: the first count of blocks after it at which the
        // predicate holds. One block short of it the tx0 is still mineable after a reorg.
        let depth = (1..=10_000u32)
            .find(|d| crate::stranded::irreversibly_buried(&MAIN_NETWORK, expiry, expiry + d))
            .expect("the expiry is buried within 10,000 blocks");
        let last_live_tip = expiry + depth - 1;
        assert_eq!(
            last_live_tip - h,
            720,
            "the contract's last live offset, (120 + 1) + 600 - 1, re-derived from the crate's \
             delta ({delta}) and the burial depth ({depth})"
        );
        assert!(
            !is_provably_dead(&MAIN_NETWORK, h, last_live_tip),
            "the reclaim margin does not cover a tx0 built under NU7's {delta}-block default \
             expiry: an index exposed at {h} reads dead at tip {last_live_tip}, where a reorg the \
             burial covers can still mine its tx0 (NU7 wave step 5)"
        );
    }

    // The NU7 wave's step 5 (its reclaim half) and step 7m: the test author's rows that read the
    // height-aware deadness gate across the activation, joined at adjudication.
    #[cfg(test)]
    mod margin_across_the_activation;

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
        let tip = PRE_NU7_TIP;
        let old = tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS - 10; // comfortably dead
        // Indices 0,1,2 all dead (reserved at old heights); the highest (2) is the target — minting
        // on it advances gap_start past 0 and 1 too.
        let reserved = vec![
            (ephemeral(0xA0), meta(0, old)),
            (ephemeral(0xA2), meta(2, old)),
            (ephemeral(0xA1), meta(1, old)),
        ];
        assert_eq!(
            highest_dead_target(&MAIN_NETWORK, &reserved, tip),
            Some(ephemeral(0xA2)),
            "the highest dead index is the reclaim target"
        );
    }

    #[test]
    fn never_targets_a_live_index_even_if_it_is_the_highest() {
        // The BLOCKER guard: leaks 0,1 are dead but index 2 is a LIVE in-flight tx0 (young).
        // "Highest outstanding" would be 2 — but self-minting on it races a legitimate deposit, so
        // the reclaim targets the highest DEAD (1), never the live one.
        let tip = PRE_NU7_TIP;
        let old = tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS - 10;
        let young = tip - 5; // in-flight, not provably dead
        let reserved = vec![
            (ephemeral(0xB0), meta(0, old)),
            (ephemeral(0xB1), meta(1, old)),
            (ephemeral(0xB2), meta(2, young)),
        ];
        assert_eq!(
            highest_dead_target(&MAIN_NETWORK, &reserved, tip),
            Some(ephemeral(0xB1)),
            "the live highest index is skipped; the highest DEAD index wins"
        );
    }

    #[test]
    fn no_dead_reservation_is_nothing_to_reclaim() {
        // Every reservation is young (a genuinely transient ceiling — the tx0s may still mine). No
        // target ⇒ the method returns NothingToReclaim (never mint on a maybe-live index).
        let tip = PRE_NU7_TIP;
        let young = tip - 5;
        let reserved = vec![
            (ephemeral(0xC0), meta(0, young)),
            (ephemeral(0xC1), meta(1, young)),
        ];
        assert_eq!(highest_dead_target(&MAIN_NETWORK, &reserved, tip), None);
        // An empty set (no ephemeral ever reserved) is also None.
        assert_eq!(highest_dead_target(&MAIN_NETWORK, &[], tip), None);
    }

    #[test]
    fn skips_reservations_with_unknown_exposure_or_no_index() {
        // Fail-safe: an `Unknown` exposure is not provably dead (skip); a well-known dead index is
        // still selected alongside it.
        let tip = PRE_NU7_TIP;
        let old = tip - EPHEMERAL_RECLAIM_DEADNESS_BLOCKS - 10;
        let unknown = TransparentAddressMetadata::derived(
            TransparentKeyScope::EPHEMERAL,
            NonHardenedChildIndex::from_index(9).expect("idx"),
            Exposure::Unknown,
            None,
        );
        let reserved = vec![(ephemeral(0xD9), unknown), (ephemeral(0xD3), meta(3, old))];
        assert_eq!(
            highest_dead_target(&MAIN_NETWORK, &reserved, tip),
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
