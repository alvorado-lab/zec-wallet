//! ZIP 218's shielded limits per block, refused at propose (`docs/plan/nu7-wave.md`
//! step 7m: the owed per-block limits, D-M6).
//!
//! From the NU7 activation a block may carry at most 330 Orchard actions, 330 Ironwood
//! actions and 300 Sapling spends plus outputs, and at most 330 of all of them together
//! (ZIP 218, "Shielded pool action limits"). A transaction over any of these by itself
//! fits in no block at or after the activation, and the ZIP says: "Wallets SHOULD NOT
//! create such a transaction for a target height at or after NU7 activation." The pinned
//! `zcash_client_backend` 0.25.0 refuses only a step whose estimated size exceeds
//! `MAX_BLOCK_BYTES` (`ProposalError::TransactionTooLarge`), so without this a wallet
//! holding hundreds of small notes could build, after the activation, a send that passes
//! every check and is never mined.
//!
//! [`check_proposal`] runs on every proposal the SDK makes: in `send::propose_core`, and
//! in `send::propose_shield_with_threshold` for the shield and the sweep. It refuses an
//! over-limit proposal with `WalletError::ProposeFailed`, the class upstream's size
//! refusal takes. The refusal is deterministic, so the host's copy is "check the details",
//! never "try again". A shield or a sweep has no shielded input and one shielded output,
//! so it cannot reach a limit today; it is checked so that a change of change strategy
//! cannot make it one unnoticed.

use orchard::ValuePool;
use zcash_client_backend::proposal::{Proposal, Step};
use zcash_primitives::transaction::builder::BundlePadding;
use zcash_primitives::transaction::components::orchard::bundle_version_for_branch;
use zcash_protocol::PoolType;
use zcash_protocol::consensus::{BlockHeight, BranchId, NetworkUpgrade, Parameters};

use crate::error::WalletError;

/// ZIP 218's `OrchardProtocolBlockActionLimit`: the most Orchard actions a block from the
/// NU7 activation may carry, summed over its transactions. ZIP 218 bounds Ironwood
/// actions by the same constant.
const ORCHARD_PROTOCOL_BLOCK_ACTION_LIMIT: usize = 330;

/// ZIP 218's `SaplingBlockIOLimit`: the most Sapling spends plus outputs a block from the
/// NU7 activation may carry.
const SAPLING_BLOCK_IO_LIMIT: usize = 300;

/// ZIP 218's `GlobalShieldedBudget`: the most shielded cost a block from the NU7
/// activation may carry. Each Orchard action, Ironwood action, Sapling spend and Sapling
/// output costs one unit of it (a Sprout JoinSplit two, but this SDK builds none).
const GLOBAL_SHIELDED_BUDGET: usize = 330;

/// The `code` a refusal here logs on its `wallet.propose_refused` event: a static name,
/// and a member of `send::PROPOSE_ERR_VARIANT_CODES`, the closed set the §5.4 scanner
/// knows.
pub(crate) const REFUSAL_CODE: &str = "zip218_block_limit";

/// What one transaction adds to each of ZIP 218's sums: its `nActionsOrchard`, its
/// `nActionsIronwood`, and its `nSpendsSapling + nOutputsSapling`. The sum's Sprout term
/// is zero here: the v5 and v6 transactions this SDK builds carry no JoinSplit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShieldedCounts {
    orchard_actions: usize,
    ironwood_actions: usize,
    sapling_spends_and_outputs: usize,
}

impl ShieldedCounts {
    /// ZIP 218's shielded cost: one unit for every action in either pool and for every
    /// Sapling spend and output, all from the same budget.
    fn shielded_cost(self) -> usize {
        self.orchard_actions
            .saturating_add(self.ironwood_actions)
            .saturating_add(self.sapling_spends_and_outputs)
    }

    /// Whether a transaction with these counts fits every ZIP 218 limit by itself. With
    /// the ZIP's numbers the global budget (330) also bounds each Orchard-protocol pool;
    /// the per-pool clauses stay so that the check follows the ZIP if its numbers change.
    fn within_limits(self) -> bool {
        self.orchard_actions <= ORCHARD_PROTOCOL_BLOCK_ACTION_LIMIT
            && self.ironwood_actions <= ORCHARD_PROTOCOL_BLOCK_ACTION_LIMIT
            && self.sapling_spends_and_outputs <= SAPLING_BLOCK_IO_LIMIT
            && self.shielded_cost() <= GLOBAL_SHIELDED_BUDGET
    }
}

/// The ZIP 218 counts of the transaction `step` describes, as the builder will build it at
/// a target height under `branch`. `None` where upstream cannot count a bundle, which is
/// where the builder would refuse to build it too, so the caller fails closed. It is never
/// `None` for a branch at or after NU7, whose Orchard protocol revision has both pools.
///
/// Every count is upstream's own, with the padding the build path uses
/// (`zcash_client_backend` 0.25.0, `create_proposed_transactions`):
/// - Orchard: [`Step::orchard_action_count`] with `BundlePadding::DEFAULT`, the Orchard
///   bundle's padding outside the PCZT path (which this SDK does not use), at the
///   branch's Orchard bundle version. From NU6.3 that version disables cross-address
///   transfers, so a spend and an output never share an action: `spends + outputs`,
///   padded to two. Before NU6.3 it was `max(spends, outputs)`.
/// - Ironwood: [`Step::ironwood_action_count`] with [`Step::ironwood_bundle_padding`], the
///   padding the change strategy recorded and the builder reads back (unpadded only for a
///   canonical ZIP 318 crossing): `max(spends, outputs)`, padded.
/// - Sapling: the step's spends and its payment and change outputs, padded by
///   `sapling_crypto::builder::BundleType::DEFAULT`, the type the builder's standard
///   configuration uses (`BuildConfig::sapling_builder_config`): every spend, and at least
///   two outputs once the bundle exists.
///
/// The change strategy's dummy outputs (`TransactionBalance::dummy_outputs`) are that same
/// padding, recorded in advance, so the padded counts include them and nothing is added on
/// top. A prior step's output that this step spends adds nothing: the builder accepts
/// only a transparent one (`ProposalNotSupported` otherwise).
fn step_counts<N>(step: &Step<N>, branch: BranchId) -> Option<ShieldedCounts> {
    let orchard_version = bundle_version_for_branch(branch, ValuePool::Orchard)?;
    let ironwood_version = bundle_version_for_branch(branch, ValuePool::Ironwood)?;
    let orchard_actions = step
        .orchard_action_count(BundlePadding::DEFAULT, orchard_version)
        .ok()?;
    let ironwood_actions = step
        .ironwood_action_count(step.ironwood_bundle_padding(), ironwood_version)
        .ok()?;

    let sapling = sapling_crypto::builder::BundleType::DEFAULT;
    let sapling_spends = step.input_count_in_pool(PoolType::SAPLING);
    let sapling_outputs =
        step.output_count_in_pool(PoolType::SAPLING) + step.change_count_in_pool(PoolType::SAPLING);
    let sapling_spends_and_outputs = sapling
        .num_spends(sapling_spends)
        .ok()?
        .checked_add(sapling.num_outputs(sapling_spends, sapling_outputs).ok()?)?;

    Some(ShieldedCounts {
        orchard_actions,
        ironwood_actions,
        sapling_spends_and_outputs,
    })
}

/// Refuse a proposal that ZIP 218 keeps out of every block: from the NU7 activation, a
/// step whose transaction exceeds a per-block limit by itself, or which cannot be counted.
///
/// The test is ZIP 218's `IsNU7Activated` at the proposal's target height, the height the
/// builder signs for, read from the consensus parameters (`is_nu_active`). The hand-kept
/// `KNOWN_UPGRADES` is not consulted. Below the activation every proposal passes: the
/// limits do not exist there. Each step is its own transaction and is checked alone.
///
/// The refusal is [`WalletError::ProposeFailed`], as for upstream's
/// `TransactionTooLarge`: deterministic on the wallet's notes and the request, so the copy
/// is "check the details". Its event carries a static `code` and no amount or address
/// (§5.4).
pub(crate) fn check_proposal<P: Parameters, F, N>(
    params: &P,
    proposal: &Proposal<F, N>,
) -> Result<(), WalletError> {
    let target = BlockHeight::from(proposal.min_target_height());
    if !params.is_nu_active(NetworkUpgrade::Nu7, target) {
        return Ok(());
    }
    let branch = BranchId::for_height(params, target);
    let fits = proposal
        .steps()
        .iter()
        .all(|step| step_counts(step, branch).is_some_and(ShieldedCounts::within_limits));
    if fits {
        return Ok(());
    }
    tracing::warn!(
        target: "zec_wallet_core",
        outcome = "propose_refused_unmapped",
        code = REFUSAL_CODE,
        "wallet.propose_refused"
    );
    Err(WalletError::ProposeFailed)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use incrementalmerkletree::Position;
    use orchard::keys::{FullViewingKey, SpendingKey};
    use orchard::note::{NoteVersion, RandomSeed, Rho};
    use zcash_address::{ToAddress, ZcashAddress};
    use zcash_client_backend::data_api::testing::TestBuilder;
    use zcash_client_backend::data_api::wallet::TargetHeight;
    use zcash_client_backend::fees::{ChangeValue, StandardFeeRule, TransactionBalance};
    use zcash_client_backend::proposal::ShieldedInputs;
    use zcash_client_backend::wallet::{Note, ReceivedNote};
    use zcash_protocol::TxId;
    use zcash_protocol::consensus::{MAIN_NETWORK, NetworkType, TEST_NETWORK};
    use zcash_protocol::local_consensus::LocalNetwork;
    use zcash_protocol::value::Zatoshis as ProtoZat;

    use super::*;
    use crate::test_support::HistoryHarness;

    /// The NU7 activation on the straddling chain below. The harness's first block is the
    /// chain's Sapling activation (100,000), which leaves room for its notes to mature and
    /// for sends on both sides of the activation.
    const H: u32 = 100_060;

    /// Every upgrade from Sapling to NU6.3 at the harness's one height and NU7 at [`H`],
    /// so a target below `H` is under `Nu6_3`, the branch mainnet runs before the
    /// activation. The shape of `wallet::tests::nu7_activation::straddling`, which is
    /// private to `wallet::tests`.
    fn straddling() -> LocalNetwork {
        let below = TestBuilder::<(), ()>::DEFAULT_NETWORK.nu5;
        LocalNetwork {
            nu6: below,
            nu6_1: below,
            nu6_2: below,
            nu6_3: below,
            nu7: Some(BlockHeight::from_u32(H)),
            ..TestBuilder::<(), ()>::DEFAULT_NETWORK
        }
    }

    fn counts(orchard: usize, ironwood: usize, sapling: usize) -> ShieldedCounts {
        ShieldedCounts {
            orchard_actions: orchard,
            ironwood_actions: ironwood,
            sapling_spends_and_outputs: sapling,
        }
    }

    /// A note of `value` zatoshis in the Orchard-protocol `pool` (version-2 plaintext for
    /// Orchard, version 3 for Ironwood). Only the pool and the value matter to the counts.
    fn orchard_family_note(pool: ValuePool, value: u64) -> Note {
        let version = match pool {
            ValuePool::Orchard => NoteVersion::V2,
            ValuePool::Ironwood => NoteVersion::V3,
        };
        let sk: SpendingKey = Option::from(SpendingKey::from_bytes([0x2a; 32])).expect("key");
        let recipient = FullViewingKey::from(&sk).address_at(0u32, zip32::Scope::External);
        let rho: Rho = Option::from(Rho::from_bytes(&[0; 32])).expect("rho");
        let rseed: RandomSeed =
            Option::from(RandomSeed::from_bytes([0x1b; 32], &rho)).expect("rseed");
        let note = Option::from(orchard::Note::from_parts(
            recipient,
            orchard::value::NoteValue::from_raw(value),
            rho,
            rseed,
            version,
        ))
        .expect("note");
        Note::Orchard { note, pool }
    }

    fn sapling_note(value: u64) -> Note {
        let (_, recipient) = sapling_crypto::zip32::ExtendedSpendingKey::master(&[7; 32])
            .expect("key")
            .default_address();
        Note::Sapling(sapling_crypto::Note::from_parts(
            recipient,
            sapling_crypto::value::NoteValue::from_raw(value),
            sapling_crypto::Rseed::AfterZip212([9; 32]),
        ))
    }

    fn notes(n: usize, make: impl Fn() -> Note) -> Vec<Note> {
        (0..n).map(|_| make()).collect()
    }

    /// A validated one-step proposal targeting `target` that spends `notes` and returns
    /// `change`, paying the rest as fee. Built through upstream's own constructor, so the
    /// step is one the engine could have produced; nothing in the counts reads a value.
    fn proposal_at(
        target: u32,
        notes: Vec<Note>,
        change: Vec<ChangeValue>,
    ) -> Proposal<StandardFeeRule, u32> {
        let spent: u64 = notes.iter().map(|n| n.value().into_u64()).sum();
        let returned: u64 = change.iter().map(|c| c.value().into_u64()).sum();
        let fee = ProtoZat::from_u64(spent - returned).expect("a fee");
        let mut received = notes.into_iter().enumerate().map(|(i, note)| {
            ReceivedNote::from_parts(
                i as u32,
                TxId::from_bytes([0; 32]),
                i as u16,
                note,
                zip32::Scope::External,
                Position::from(i as u64),
                Some(BlockHeight::from_u32(target - 10)),
                None,
            )
        });
        let head = received.next().expect("at least one note");
        let inputs = ShieldedInputs::from_parts((head, received.collect::<Vec<_>>()).into());
        Proposal::single_step(
            zip321::TransactionRequest::empty(),
            BTreeMap::new(),
            vec![],
            Some(inputs),
            BlockHeight::from_u32(target - 3),
            TransactionBalance::new(change, fee).expect("a balance"),
            StandardFeeRule::Zip317,
            TargetHeight::from(BlockHeight::from_u32(target)),
            crate::account::spendable_policy(),
            false,
            true,
        )
        .expect("a valid step")
    }

    fn zat(value: u64) -> ProtoZat {
        ProtoZat::from_u64(value).expect("in range")
    }

    /// A consolidation of `n` Ironwood notes into one Ironwood change output: `n` Ironwood
    /// actions (from two), and nothing in any other pool.
    fn ironwood_consolidation(target: u32, n: usize) -> Proposal<StandardFeeRule, u32> {
        proposal_at(
            target,
            notes(n, || orchard_family_note(ValuePool::Ironwood, 10_000)),
            vec![ChangeValue::ironwood(zat(5_000), None)],
        )
    }

    fn only_step_counts<F, N>(proposal: &Proposal<F, N>, branch: BranchId) -> ShieldedCounts {
        assert_eq!(proposal.steps().len(), 1, "harness: a one-step proposal");
        step_counts(proposal.steps().first(), branch).expect("countable at this branch")
    }

    // ── The four limits, at the limit and one past it ────────────────────────────

    /// `OrchardProtocolBlockActionLimit` for Orchard: 330 actions fit, 331 do not.
    #[test]
    fn the_orchard_action_limit_admits_330_and_refuses_331() {
        assert!(
            counts(330, 0, 0).within_limits(),
            "330 Orchard actions are ZIP 218's limit itself, so they fit"
        );
        assert!(
            !counts(331, 0, 0).within_limits(),
            "331 Orchard actions are one over ZIP 218's limit"
        );
    }

    /// `OrchardProtocolBlockActionLimit` for Ironwood: the same 330 and 331.
    #[test]
    fn the_ironwood_action_limit_admits_330_and_refuses_331() {
        assert!(
            counts(0, 330, 0).within_limits(),
            "330 Ironwood actions are ZIP 218's limit itself, so they fit"
        );
        assert!(
            !counts(0, 331, 0).within_limits(),
            "331 Ironwood actions are one over ZIP 218's limit"
        );
    }

    /// `SaplingBlockIOLimit`: 300 spends plus outputs fit, 301 do not, though 301 is
    /// still inside the global budget.
    #[test]
    fn the_sapling_limit_admits_300_spends_and_outputs_and_refuses_301() {
        assert!(
            counts(0, 0, 300).within_limits(),
            "300 Sapling spends plus outputs are ZIP 218's limit itself, so they fit"
        );
        assert!(
            !counts(0, 0, 301).within_limits(),
            "301 Sapling spends plus outputs are one over ZIP 218's Sapling limit, though \
             under the global budget"
        );
    }

    /// `GlobalShieldedBudget`: 330 units across the pools fit, 331 do not, with every
    /// pool under its own limit. The ZIP's own two examples are rows here.
    #[test]
    fn the_global_budget_admits_330_across_pools_and_refuses_331() {
        assert!(
            counts(30, 0, 300).within_limits(),
            "300 Sapling and 30 Orchard are 330, the budget itself"
        );
        assert!(
            !counts(31, 0, 300).within_limits(),
            "300 Sapling and 31 Orchard are 331, one over the budget"
        );
        assert!(
            counts(200, 130, 0).within_limits(),
            "200 Orchard and 130 Ironwood are 330: the two pools share one budget"
        );
        assert!(
            !counts(200, 131, 0).within_limits(),
            "200 Orchard and 131 Ironwood are 331, one over the shared budget"
        );
        assert!(
            !counts(200, 200, 0).within_limits(),
            "ZIP 218's example: 200 and 200 satisfy both per-pool limits but exceed the budget"
        );
        assert!(
            counts(30, 0, 150 + 150).within_limits(),
            "ZIP 218's rationale: 150 Sapling spends, 150 outputs and 30 Orchard actions fit"
        );
    }

    // ── The counting, against what the builder builds ────────────────────────────

    /// Each pool is counted with the padding and the bundle version the builder uses at
    /// an NU7 target. Orchard from NU6.3 is `spends + outputs`, so 329 spends with one
    /// change output are 330 actions where `max` would say 329; Ironwood is
    /// `max(spends, outputs)`; Sapling counts every spend and pads its outputs to two;
    /// a single spend still costs two actions in an Orchard-protocol pool.
    #[test]
    fn each_pool_is_counted_as_the_builder_pads_it() {
        let target = H;
        // Orchard: 329 spends and one change output, cross-address disabled from NU6.3.
        // The change returns less than the spends remove (the turnstile).
        let orchard = proposal_at(
            target,
            notes(329, || orchard_family_note(ValuePool::Orchard, 10)),
            vec![ChangeValue::orchard(zat(1), None)],
        );
        assert_eq!(
            only_step_counts(&orchard, BranchId::Nu7),
            counts(330, 0, 0),
            "an Orchard spend and an Orchard output never share an action from NU6.3"
        );

        // One Orchard spend, its change sent to Ironwood: each bundle padded to two.
        let crossing = proposal_at(
            target,
            notes(1, || orchard_family_note(ValuePool::Orchard, 10_000)),
            vec![ChangeValue::ironwood(zat(5_000), None)],
        );
        assert_eq!(
            only_step_counts(&crossing, BranchId::Nu7),
            counts(2, 2, 0),
            "a one-spend Orchard bundle and a one-output Ironwood bundle are two actions each"
        );

        // Ironwood: a spend and an output share an action.
        assert_eq!(
            only_step_counts(&ironwood_consolidation(target, 331), BranchId::Nu7),
            counts(0, 331, 0),
            "331 Ironwood spends and one output are max(331, 1) actions"
        );

        // Sapling: five spends and three change outputs are eight; one output is padded
        // to two.
        let sapling = proposal_at(
            target,
            notes(5, || sapling_note(10_000)),
            vec![
                ChangeValue::sapling(zat(5_000), None),
                ChangeValue::sapling(zat(5_000), None),
                ChangeValue::sapling(zat(5_000), None),
            ],
        );
        assert_eq!(
            only_step_counts(&sapling, BranchId::Nu7),
            counts(0, 0, 8),
            "five Sapling spends and three Sapling outputs are eight"
        );
        let padded = proposal_at(
            target,
            notes(5, || sapling_note(10_000)),
            vec![ChangeValue::sapling(zat(5_000), None)],
        );
        assert_eq!(
            only_step_counts(&padded, BranchId::Nu7),
            counts(0, 0, 7),
            "one Sapling output is padded to two: five spends and two outputs"
        );
    }

    // ── The refusal, across the activation ───────────────────────────────────────

    /// One network's rows: at its NU7 activation `h`, a proposal one action over the
    /// Ironwood limit is refused and one at the limit passes; the same over-limit
    /// proposal targeting `h - 1` passes, because below the activation the limits do not
    /// exist.
    fn activation_rows<P: Parameters>(name: &str, params: &P) {
        let h = u32::from(
            params
                .activation_height(NetworkUpgrade::Nu7)
                .expect("the crate carries this network's NU7 height"),
        );
        assert!(
            matches!(
                check_proposal(params, &ironwood_consolidation(h, 331)),
                Err(WalletError::ProposeFailed)
            ),
            "{name}: 331 Ironwood actions targeting the activation {h} are refused"
        );
        let at_limit = check_proposal(params, &ironwood_consolidation(h, 330));
        assert!(
            at_limit.is_ok(),
            "{name}: 330 Ironwood actions targeting the activation {h} fit. Got {at_limit:?}"
        );
        let below = check_proposal(params, &ironwood_consolidation(h - 1, 331));
        assert!(
            below.is_ok(),
            "{name}: the same 331 targeting {}, below the activation, pass. Got {below:?}",
            h - 1
        );
    }

    /// ZIP 218 applies "for a target height at or after NU7 activation": the refusal
    /// starts at the activation on mainnet, on testnet and on the straddling chain, each
    /// height read from the crate, and the same proposal passes one block earlier.
    #[test]
    fn an_over_limit_proposal_is_refused_from_nu7_and_allowed_before_it() {
        activation_rows("straddling", &straddling());
        activation_rows("mainnet", &MAIN_NETWORK);
        activation_rows("testnet", &TEST_NETWORK);
    }

    /// The refusal is the existing deterministic `ProposeFailed`, never the retryable
    /// `ProposeTransient`, so a host does not tell the user to wait for a send that will
    /// never fit. Its event is `wallet.propose_refused` with the static code, a member of
    /// the closed set, and nothing §5.4 forbids.
    #[test]
    fn the_refusal_is_propose_failed_with_a_static_code_in_the_closed_set() {
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use tracing_subscriber::layer::SubscriberExt;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        let refused = check_proposal(&straddling(), &ironwood_consolidation(H, 331));
        assert!(
            matches!(refused, Err(WalletError::ProposeFailed)),
            "an over-limit proposal is the deterministic ProposeFailed. Got {refused:?}"
        );
        assert!(
            crate::send::PROPOSE_ERR_VARIANT_CODES.contains(&REFUSAL_CODE),
            "the refusal's code is in the closed set the §5.4 scanner exempts"
        );
        let records = sink.records_of("wallet.propose_refused");
        let last = records.last().cloned().unwrap_or_default();
        assert!(
            last.iter()
                .any(|(n, v)| n == "outcome" && v == "propose_refused_unmapped")
                && last.iter().any(|(n, v)| n == "code" && v == REFUSAL_CODE),
            "the refusal's event names its class and its code: {last:?}"
        );
        assert_5_4_clean(&sink.fields());
    }

    // ── Through `propose_core`, on a funded wallet ───────────────────────────────

    fn recipient() -> ZcashAddress {
        ZcashAddress::from_transparent_p2pkh(NetworkType::Regtest, [0x71; 20])
    }

    fn advance_to(harness: &mut HistoryHarness, tip: u32) {
        let now = harness.current_tip();
        harness.mine_empty((tip - now) as usize);
        harness.scan();
        assert_eq!(harness.current_tip(), tip, "harness: the scanned tip");
    }

    fn propose(
        harness: &mut HistoryHarness,
        amount: u64,
    ) -> Result<Proposal<StandardFeeRule, zcash_client_sqlite::ReceivedNoteId>, WalletError> {
        use zcash_client_backend::data_api::Account;
        let network = straddling();
        let st = harness.state_mut();
        let aid = st.test_account().expect("account").id();
        let request = zip321::TransactionRequest::new(vec![zip321::Payment::without_memo(
            recipient(),
            zat(amount),
        )])
        .expect("a valid request");
        crate::send::propose_core(st.wallet_mut(), &network, aid, request)
    }

    /// A wallet holding 330 Sapling notes of 10,000 zatoshis, asked to send 160 notes'
    /// worth: each note nets half its value after its fee, so the engine selects more
    /// than 300. Below the activation the proposal is built (its count over the Sapling
    /// limit, the premise). From the activation `propose_core` refuses the same request
    /// with `ProposeFailed` and this module's code, while a small send still goes.
    #[test]
    fn a_send_over_the_sapling_limit_is_refused_at_propose_from_nu7() {
        use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};
        use tracing_subscriber::layer::SubscriberExt;
        const OVER: u64 = 160 * 10_000;
        const SMALL: u64 = 20_000;

        let mut harness = HistoryHarness::on_network(straddling());
        harness.mine_received_multi(&[10_000; 330]);
        harness.scan();

        // Below the activation (target H - 29): built, and over the Sapling limit.
        advance_to(&mut harness, H - 30);
        let before = propose(&mut harness, OVER).expect("below NU7 the send is proposed");
        let target = BlockHeight::from(before.min_target_height());
        let counted = only_step_counts(&before, BranchId::for_height(&straddling(), target));
        assert!(
            counted.sapling_spends_and_outputs > SAPLING_BLOCK_IO_LIMIT,
            "premise: the request needs more than {SAPLING_BLOCK_IO_LIMIT} Sapling spends \
             plus outputs, got {counted:?}"
        );

        // From the activation (tip H - 1, target H): refused, with this module's code.
        advance_to(&mut harness, H - 1);
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);
        let after = propose(&mut harness, OVER);
        assert!(
            matches!(after, Err(WalletError::ProposeFailed)),
            "from NU7 the same send is the deterministic ProposeFailed. Got {:?}",
            after.map(|p| p.steps().len())
        );
        let records = sink.records_of("wallet.propose_refused");
        let last = records.last().cloned().unwrap_or_default();
        assert!(
            last.iter().any(|(n, v)| n == "code" && v == REFUSAL_CODE),
            "the refusal is this module's, not another ProposeFailed: {last:?}"
        );
        assert!(
            propose(&mut harness, SMALL).is_ok(),
            "a send within the limits still goes from the same wallet after NU7"
        );
    }
}
