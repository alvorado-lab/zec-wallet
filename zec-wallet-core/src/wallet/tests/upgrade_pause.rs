//! ADR-0580, the pre-upgrade send pause (decided 2026-10-30): the
//! rows that pin it, from the pure rule out to every signing entry of a live handle.
//!
//! **The rule.** With `K = PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS` (20) and `tip` the
//! wallet's recorded chain tip, a send is refused when the branch at `tip + 1` (the
//! first block a new transaction can be mined in) differs from the branch at
//! `tip + 1 + K`. For an activation `H` the refused tips are `H - 21` to `H - 2`;
//! `H - 22` is the last allowed before it, and `H - 1`, whose next block is under the
//! new branch, the first allowed after. The refusal is `UpgradeImminent {
//! activation_height: H, blocks_left: H - (tip + 1) }`, at propose and again at sign,
//! where it wins over `ProposalStale` (a host re-proposes on that, into the pause).
//!
//! A child of `wallet::tests` for the live-handle fixtures, and a sibling of
//! `nu7_activation` for the straddling chain.

use std::sync::Arc;

use zcash_client_backend::data_api::WalletWrite;
use zcash_protocol::consensus::{
    BlockHeight, BranchId, MAIN_NETWORK, NetworkUpgrade, Parameters, TEST_NETWORK,
};
use zcash_protocol::local_consensus::LocalNetwork;
use zcash_protocol::value::Zatoshis as ProtoZat;

use super::nu7_activation::{
    AMOUNT, H, NOTE_VALUE, advance_to, header_branch_bytes, raw_of, recipient, straddling,
};
use super::*;
use crate::error::WalletError;
use crate::send::upgrade_pause;
use crate::test_support::HistoryHarness;

/// The pause's answer at `tip`: `None` when a send may go, else the activation and
/// the blocks left. Any other error is a defect of the rule, so it panics.
fn paused<P: Parameters>(params: &P, tip: u32) -> Option<(u32, u32)> {
    match upgrade_pause(params, BlockHeight::from_u32(tip)) {
        Ok(()) => None,
        Err(WalletError::UpgradeImminent {
            activation_height,
            blocks_left,
        }) => Some((activation_height, blocks_left)),
        Err(other) => panic!("the pause answers only UpgradeImminent, got {other:?}"),
    }
}

/// The boundary on both sides of one activation `h`.
fn boundary_rows<P: Parameters>(name: &str, params: &P, h: u32) {
    assert_eq!(
        paused(params, h - 22),
        None,
        "{name}: at the tip H - 22 the next 21 blocks are all under the ending branch: allowed"
    );
    assert_eq!(
        paused(params, h - 21),
        Some((h, 20)),
        "{name}: H - 21 is the first refused tip, with 20 blocks still to be mined"
    );
    assert_eq!(paused(params, h - 11), Some((h, 10)), "{name}: H - 11");
    assert_eq!(
        paused(params, h - 2),
        Some((h, 1)),
        "{name}: H - 2 is the last refused tip, with one block to go"
    );
    assert_eq!(
        paused(params, h - 1),
        None,
        "{name}: at H - 1 the next block is under the new branch: sends work again at once"
    );
    assert_eq!(paused(params, h), None, "{name}: H");
}

/// **`the_pause_refuses_the_last_twenty_blocks_before_each_activation`.** The rule
/// itself, on the crate's mainnet and testnet heights (read through the crate, and
/// pinned to the numbers the wave plans against), on the straddling `LocalNetwork`
/// step 4's rows run on, and on a chain where a second activation falls inside the
/// first one's window: the pause names the first, then runs on to the second.
///
/// Mutants, each red: the window one block short (`next + K - 1`); the branch read at
/// the tip instead of the next block; `blocks_left` counted from the tip.
#[test]
fn the_pause_refuses_the_last_twenty_blocks_before_each_activation() {
    fn at<P: Parameters>(params: &P, nu: NetworkUpgrade) -> u32 {
        u32::from(
            params
                .activation_height(nu)
                .expect("the crate carries the height"),
        )
    }
    let main_nu7 = at(&MAIN_NETWORK, NetworkUpgrade::Nu7);
    let main_nu6_3 = at(&MAIN_NETWORK, NetworkUpgrade::Nu6_3);
    let test_nu7 = at(&TEST_NETWORK, NetworkUpgrade::Nu7);
    let test_nu6_3 = at(&TEST_NETWORK, NetworkUpgrade::Nu6_3);
    assert_eq!(
        (main_nu7, main_nu6_3, test_nu7, test_nu6_3),
        (3_543_405, 3_428_143, 4_465_026, 4_134_000),
        "premise: the pinned crate's mainnet and testnet heights"
    );
    boundary_rows("mainnet NU7", &MAIN_NETWORK, main_nu7);
    boundary_rows("mainnet NU6.3", &MAIN_NETWORK, main_nu6_3);
    boundary_rows("testnet NU7", &TEST_NETWORK, test_nu7);
    boundary_rows("testnet NU6.3", &TEST_NETWORK, test_nu6_3);
    boundary_rows("the straddling LocalNetwork", &straddling(), H);

    // Two activations five blocks apart: Nu6_3 at `x`, NU7 at `x + 5`.
    let x = 100_100;
    let stacked = LocalNetwork {
        nu6_3: Some(BlockHeight::from_u32(x)),
        nu7: Some(BlockHeight::from_u32(x + 5)),
        ..straddling()
    };
    assert_eq!(paused(&stacked, x - 22), None, "stacked: x - 22 is allowed");
    assert_eq!(
        paused(&stacked, x - 21),
        Some((x, 20)),
        "stacked: the window names the first activation"
    );
    assert_eq!(
        paused(&stacked, x - 1),
        Some((x + 5, 5)),
        "stacked: past the first, the second is inside the window, so the pause runs on"
    );
    assert_eq!(
        paused(&stacked, x + 4),
        None,
        "stacked: from the block before the second, the next block is NU7's: allowed"
    );

    // A hostile tip near the top of the height range neither panics nor refuses.
    assert_eq!(paused(&MAIN_NETWORK, u32::MAX - 1), None);
    assert_eq!(paused(&MAIN_NETWORK, u32::MAX), None);
}

type Proposal = zcash_client_backend::proposal::Proposal<
    zcash_client_backend::fees::StandardFeeRule,
    zcash_client_sqlite::ReceivedNoteId,
>;

fn funded_at(tip: u32) -> HistoryHarness {
    let mut harness = HistoryHarness::on_network(straddling());
    harness.mine_received(NOTE_VALUE);
    advance_to(&mut harness, tip);
    harness
}

fn propose(harness: &mut HistoryHarness) -> Result<Proposal, WalletError> {
    use zcash_client_backend::data_api::Account;
    let network = straddling();
    let st = harness.state_mut();
    let aid = st.test_account().expect("account").id();
    let request = zip321::TransactionRequest::new(vec![zip321::Payment::without_memo(
        recipient(),
        ProtoZat::const_from_u64(AMOUNT),
    )])
    .expect("a valid request");
    send::propose_core(st.wallet_mut(), &network, aid, request)
}

fn sign(
    harness: &mut HistoryHarness,
    proposal: &Proposal,
) -> Result<Vec<zcash_protocol::TxId>, WalletError> {
    let network = straddling();
    let st = harness.state_mut();
    let usk = st.test_account().expect("account").usk().clone();
    let prover = prover::tx_prover();
    send::create_signed_core(
        st.wallet_mut(),
        &network,
        usk,
        proposal,
        prover,
        prover,
        crate::consensus::SigningPermit::assume_current_for_test(),
    )
}

fn is_pause(r: &Result<impl std::fmt::Debug, WalletError>, activation: u32, left: u32) -> bool {
    matches!(
        r,
        Err(WalletError::UpgradeImminent {
            activation_height,
            blocks_left,
        }) if *activation_height == activation && *blocks_left == left
    )
}

/// **`a_send_inside_the_window_is_refused_at_propose_and_at_sign_never_stale`.** On the
/// straddling chain, through the same `propose_core` and `create_signed_core` the live
/// send runs:
/// - propose at `H - 22` goes; at `H - 21` and at `H - 2` it is refused, naming `H`;
/// - a proposal made at `H - 22` and signed at `H - 10` is refused at sign: the drift
///   (11) and the branch clause would both let it through, so only the pause stops a
///   `Nu6_3` transaction being signed with ten blocks left;
/// - a proposal made at `H - 30` and signed at `H - 5` is refused with the pause, not
///   `ProposalStale`, though its drift (24) is past the bound: the host must not be told
///   to re-propose into the pause;
/// - at `H - 1` a fresh proposal signs at once, under NU7.
///
/// Mutants, each red: `propose_core` without the pause; `create_signed_core` without
/// it; the pause after the anchor guard in `create_signed_core`.
#[test]
fn a_send_inside_the_window_is_refused_at_propose_and_at_sign_never_stale() {
    let mut harness = funded_at(H - 22);
    assert!(
        propose(&mut harness).is_ok(),
        "at H - 22, the last tip before the window, a send proposes"
    );
    advance_to(&mut harness, H - 21);
    let at_first = propose(&mut harness);
    assert!(
        is_pause(&at_first, H, 20),
        "at H - 21 propose is refused with 20 blocks left: {at_first:?}"
    );
    advance_to(&mut harness, H - 2);
    let at_last = propose(&mut harness);
    assert!(
        is_pause(&at_last, H, 1),
        "at H - 2 propose is refused with one block left: {at_last:?}"
    );

    // Proposed before the window, confirmed inside it, inside the drift bound.
    let mut early = funded_at(H - 22);
    let proposal = propose(&mut early).expect("proposed at H - 22");
    advance_to(&mut early, H - 10);
    let signed = sign(&mut early, &proposal);
    assert!(
        is_pause(&signed, H, 9),
        "confirmed at H - 10, the proposal is refused at sign, not signed under Nu6_3 with \
         nine blocks left: {signed:?}"
    );

    // Proposed before the window, confirmed inside it, past the drift bound.
    let mut older = funded_at(H - 30);
    let proposal = propose(&mut older).expect("proposed at H - 30");
    advance_to(&mut older, H - 5);
    let signed = sign(&mut older, &proposal);
    assert!(
        is_pause(&signed, H, 4),
        "confirmed at H - 5 with a drift of 24, the answer is still the pause, never \
         ProposalStale: {signed:?}"
    );

    // From H - 1 sends work again at once, under the new branch.
    advance_to(&mut older, H - 1);
    let fresh = propose(&mut older).expect("at H - 1 a send proposes again");
    let txids = sign(&mut older, &fresh).expect("and signs");
    assert_eq!(
        header_branch_bytes(&raw_of(&older.read_conn(), &txids[0])),
        u32::from(BranchId::Nu7),
        "signed under NU7's branch"
    );
}

/// Record `tip` as the live handle's chain tip, as a sync pass's head does.
fn record_tip(w: &Wallet, tip: u32) {
    let mut guard = w.inner.db.lock().expect("wallet db mutex poisoned");
    guard
        .db
        .update_chain_tip(BlockHeight::from_u32(tip))
        .expect("update the chain tip");
}

/// **`every_signing_entry_refuses_inside_the_window_before_any_seed_is_pulled`.** A
/// live testnet handle whose seed lives with the host (a counting port), its tip
/// recorded at `H - 11` for testnet's NU7. Queueing is not signing, so a send queues.
/// Every entry that would sign answers the pause, typed, and none of them pulls the
/// seed: propose, the shield's propose, the sign of a proposal, the parked row's
/// "Send now", the sweep and the reclaim. The background drain defers the queued row
/// (counted `retried`, left queued) without a pull either. At `H - 1` the same handle's
/// propose is past the pause (an unfunded wallet's answer, not the pause).
///
/// Mutants, each red: the shield's propose without the pause; the sign's pre-check
/// dropped (an unknown proposal then answers from the registry); the parked sign's
/// pre-check dropped (the seed is pulled and the row answers "still queued"); the
/// drain's pre-check dropped (the seed is pulled); the sweep's and the reclaim's entry
/// checks dropped (each answers "nothing to do").
#[tokio::test]
async fn every_signing_entry_refuses_inside_the_window_before_any_seed_is_pulled() {
    use crate::seed::testing::StagedSeedPort;

    let h = u32::from(
        TEST_NETWORK
            .activation_height(NetworkUpgrade::Nu7)
            .expect("testnet NU7"),
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let port = Arc::new(StagedSeedPort::new(FR12_SENTINEL_SEED.to_vec()));
    let w = create_none_then_reopen_with_port(
        dir.path(),
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(testnet_birthday())
        .await
        .expect("import via port");
    let pulls = port.pulls();
    record_tip(&w, h - 11);

    let queued = w
        .queue_send(one_payment(Some(40_000)))
        .await
        .expect("queueing is not signing: a send queues inside the window");
    let created_at = w
        .list_parked_sends()
        .await
        .expect("parked")
        .into_iter()
        .find(|p| p.id == queued.value())
        .expect("the queued send is parked")
        .created_at;

    let proposed = w.propose(one_payment(Some(40_000))).await;
    assert!(is_pause(&proposed, h, 10), "propose: {proposed:?}");
    let shield = w.propose_shield().await;
    assert!(is_pause(&shield, h, 10), "the shield's propose: {shield:?}");
    let signed = w.sign_proposal(424_242).await;
    assert!(
        is_pause(&signed, h, 10),
        "the sign answers the pause before it even looks the proposal up: {signed:?}"
    );
    let parked = w.authorize_parked_send(queued.value(), created_at).await;
    assert!(is_pause(&parked, h, 10), "the parked Send now: {parked:?}");
    let swept = w.sweep_ephemeral_funds().await;
    assert!(is_pause(&swept, h, 10), "the sweep: {swept:?}");
    let reclaimed = w.reclaim_ephemeral_slots().await;
    assert!(is_pause(&reclaimed, h, 10), "the reclaim: {reclaimed:?}");

    let pass = w
        .resubmit_queued_sends(&sync::CancelToken::new())
        .await
        .expect("the drain runs");
    assert_eq!(
        pass.retried, 1,
        "the drain defers the queued send inside the window"
    );
    assert_eq!(
        w.list_parked_sends().await.expect("parked").len(),
        1,
        "and leaves it queued"
    );
    assert_eq!(
        port.pulls(),
        pulls,
        "no entry pulled the seed inside the window"
    );

    // Past the window the same handle's propose is not the pause.
    record_tip(&w, h - 1);
    let after = w.propose(one_payment(Some(40_000))).await;
    assert!(
        !matches!(after, Err(WalletError::UpgradeImminent { .. })),
        "at H - 1 sends work again: {after:?}"
    );
    w.close().await.expect("close");
}

/// A testnet handle whose seed lives with the host (a counting port), with an account.
async fn port_wallet(dir: &std::path::Path) -> (Wallet, Arc<crate::seed::testing::StagedSeedPort>) {
    let vault = test_vault();
    let port = Arc::new(crate::seed::testing::StagedSeedPort::new(
        FR12_SENTINEL_SEED.to_vec(),
    ));
    let w = create_none_then_reopen_with_port(
        dir,
        &vault,
        FR12_SENTINEL_SEED,
        Some(port.clone() as Arc<dyn WalletSeedPort>),
    )
    .await;
    w.import_account(testnet_birthday())
        .await
        .expect("import via port");
    (w, port)
}

fn testnet_nu7() -> u32 {
    u32::from(
        TEST_NETWORK
            .activation_height(NetworkUpgrade::Nu7)
            .expect("testnet NU7"),
    )
}

/// A probe for which the chain moves while it answers: it records `tip` on the handle, as a
/// sync landing mid-sweep would, then returns one funded output for the address asked.
struct ProbeThatOpensTheWindow {
    inner: Arc<Inner>,
    tip: u32,
}

#[async_trait::async_trait]
impl crate::ephemeral_detect::EphemeralUtxoProbe for ProbeThatOpensTheWindow {
    async fn probe_ephemeral(
        &self,
        address: zcash_transparent::address::TransparentAddress,
    ) -> Result<Vec<crate::transparent::TransparentUtxoRecord>, ()> {
        {
            let mut guard = self.inner.db.lock().expect("wallet db mutex poisoned");
            guard
                .db
                .update_chain_tip(BlockHeight::from_u32(self.tip))
                .expect("update the chain tip");
        }
        Ok(vec![crate::transparent::TransparentUtxoRecord {
            txid: vec![0x61; 32],
            index: 0,
            script: script_paying(&address),
            value_zat: 50_000,
            height: u64::from(self.tip - 30),
        }])
    }
}

/// **`a_window_that_opens_mid_sweep_returns_what_it_swept_and_pulls_no_seed`** (step 4's
/// second fold). The sweep's entry check passes at the tip `H - 40`; while it waits on the
/// network for the first funded one-time address, a sync records the tip `H - 11`. That
/// address must not pull the seed (the per-address check runs before the pull), and the
/// sweep returns the honest partial result, never an error: the caught address `failed`
/// (its funds stay on-chain, re-sweepable), the unprobed one `truncated`, nothing swept.
/// The host then says some funds need another try, which is true.
///
/// Mutants, each red: the per-address check dropped (the seed is pulled before the
/// propose's own refusal stops it); the pause propagated as an error instead of `Paused`.
#[tokio::test]
async fn a_window_that_opens_mid_sweep_returns_what_it_swept_and_pulls_no_seed() {
    let h = testnet_nu7();
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, port) = port_wallet(dir.path()).await;
    record_tip(&w, h - 40);
    reserve_one_ephemeral(&w).await;
    reserve_one_ephemeral(&w).await;
    let pulls = port.pulls();
    let probe = ProbeThatOpensTheWindow {
        inner: Arc::clone(&w.inner),
        tip: h - 11,
    };
    let summary = w
        .sweep_ephemeral_funds_with(&probe, &FakeBroadcaster::accepting())
        .await
        .expect("a window that opens mid-sweep is the partial result, never an error");
    assert_eq!(
        (
            summary.scanned,
            summary.swept,
            summary.failed,
            summary.truncated,
            summary.recovered_zat
        ),
        (2, 0, 1, 1, 0),
        "scanned both, swept none, the caught address failed, the other left for a later run"
    );
    assert_eq!(
        port.pulls(),
        pulls,
        "the address the window caught pulled no seed"
    );
    w.close().await.expect("close");
}

/// **`the_wallet_deposit_sender_answers_the_pause_before_a_swap_claims_its_quote`** (step
/// 4's second fold, the security review's swap finding). The swap service asks the wallet's
/// deposit sender before an OutOfZec execute claims its quote. Inside the window the answer
/// is the swap twin of the pause, with the wallet's own heights; past it, `Ok`.
///
/// Mutant: the sender answers `Ok` for the pause. Red.
#[cfg(feature = "swap")]
#[tokio::test]
async fn the_wallet_deposit_sender_answers_the_pause_before_a_swap_claims_its_quote() {
    use crate::swap::DepositSender;
    let h = testnet_nu7();
    let dir = tempfile::tempdir().expect("tempdir");
    let (w, _port) = port_wallet(dir.path()).await;
    record_tip(&w, h - 11);
    let sender = WalletDepositSender {
        inner: Arc::downgrade(&w.inner),
    };
    let paused = sender.upgrade_pause().await;
    assert!(
        matches!(
            paused,
            Err(crate::error::SwapError::UpgradeImminent {
                activation_height,
                blocks_left: 10,
            }) if activation_height == h
        ),
        "inside the window the deposit sender answers the pause: {paused:?}"
    );
    record_tip(&w, h - 1);
    let after = sender.upgrade_pause().await;
    assert!(after.is_ok(), "past the window it answers Ok: {after:?}");
    w.close().await.expect("close");
}

/// **`the_test_seam_nests_and_restores_the_pause`** (step 4's second fold, the code
/// review). An inner suspension's drop leaves an outer one in force, and the outer's drop
/// restores the pause.
///
/// Mutant: the guard's drop clears the flag instead of restoring it. Red.
#[test]
fn the_test_seam_nests_and_restores_the_pause() {
    use crate::send::upgrade_pause_testing::{suspend, suspended};
    assert!(
        !suspended(),
        "premise: the pause is in force on a fresh test thread"
    );
    let outer = suspend();
    {
        let _inner = suspend();
        assert!(suspended());
    }
    assert!(
        suspended(),
        "the inner guard's drop leaves the outer suspension in force"
    );
    drop(outer);
    assert!(!suspended(), "the outer guard's drop restores the pause");
}
