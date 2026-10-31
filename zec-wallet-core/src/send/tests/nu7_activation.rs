//! The NU7 wave's step 4, the activation check (`docs/plan/nu7-wave.md` §3 step 4;
//! ADR-0579 Decisions 4 and 6): the TEST AUTHOR's send-side rows, written blind to
//! the implementation (IT-2a). A child of `send::tests` so they reuse its funded
//! harness and its two-step and deposit fixtures, on a chain that STRADDLES the
//! activation instead of the harness's default one.
//!
//! Every reconcile goes through [`reconcile_at`], the one call of
//! `reconcile_inflight` here: the step gives the reader the consensus parameters,
//! so the join re-points that one function and nothing else.

use super::*;
use zcash_client_backend::data_api::testing::{AddressType, TestBuilder};

/// The NU7 activation on the straddling chain (the harness's first block is its
/// Sapling activation, 100,000).
const H: u32 = 100_060;

/// The straddling chain: every upgrade from Sapling to NU6.3 at the harness's one
/// height, NU7 at [`H`]. A send below `H` is signed under `Nu6_3`.
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

/// `funded`, on the straddling chain, with the tip at `tip`: one note mined at the
/// harness's first block and every block to `tip` scanned.
fn funded_on_the_straddle(value: u64, tip: u32) -> HarnessState {
    let mut st = TestBuilder::new()
        .with_network(straddling())
        .with_data_store_factory(InMemDsf)
        .with_block_cache(MemCache::default())
        .with_account_from_sapling_activation(BlockHash([0; 32]))
        .build();
    let dfvk = st.test_account_sapling().cloned().expect("sapling dfvk");
    let (note_h, _, _) = st.generate_next_block(
        &dfvk,
        AddressType::DefaultExternal,
        ProtoZat::const_from_u64(value),
    );
    let mut last = note_h;
    while u32::from(last) < tip {
        let (h, _) = st.generate_empty_block();
        last = h;
    }
    let count = (u32::from(last) - u32::from(note_h) + 1) as usize;
    st.scan_cached_blocks(note_h, count);
    st.wallet_mut()
        .update_chain_tip(last)
        .expect("update chain tip");
    assert_eq!(u32::from(last), tip);
    st
}

fn chain_tip_to(st: &mut HarnessState, tip: u32) {
    st.wallet_mut()
        .update_chain_tip(BlockHeight::from_u32(tip))
        .expect("update chain tip");
}

/// THE JOIN POINT: reader 1 (`rebroadcast_group`) through its one caller, against
/// the base signature; the params are passed and unused until the join.
fn reconcile_at(st: &HarnessState, aux: &mut Connection, now_unix: u64) -> Reconciled {
    let network = straddling();
    let row = crate::intent_store::list_in_flight(aux).expect("in flight")[0].clone();
    reconcile_inflight(st.wallet(), &network, aux, &row, now_unix).expect("reconcile")
}

/// **`a_partly_mined_tex_group_strands_at_irreversible_burial_of_the_block_before_it`**
/// (R-2; ADR-0579 Decision 4, "the irreversible steps wait for irreversible burial of
/// the effective expiry", and Decision 6's consumer table: the strand at `send.rs`'s
/// earlier-leg burial check follows the predicate). A TEX two-step built at a target
/// below `H` whose tx0 mined and whose tx1 did not: tx1 is signed under `Nu6_3`, so it
/// is dead from the tip `H - 1`, its effective expiry, and the group can never
/// complete. The terminal `Stranded` waits until BOTH tx0, the irreversible step, and
/// `H - 1`, tx1's effective expiry, are irreversibly buried by the predicate (15,000 s
/// of chain time, summed across the activation); until then the row waits,
/// re-broadcasting nothing.
///
/// tx0 is mined at `H - 39`: 38 blocks at 75 s (2,850 s) to `H - 1`, then 486 at
/// 25 s, so tx0 is buried at the tip `H + 485`. `H - 1` is buried 600 blocks at 25 s
/// later, at the tip `H + 599`, so that is where the group strands.
///
/// Mutants, each red: strand once tx0 is buried and tx1 is past its bare effective
/// expiry (the `H + 485` row); count `REORG_MAX_BLOCKS` for tx1's burial (the
/// `H + 485` row); measure tx1's burial from its stored expiry (the `H + 599` row).
#[test]
fn a_partly_mined_tex_group_strands_at_irreversible_burial_of_the_block_before_it() {
    let mut st = funded_on_the_straddle(NOTE_VALUE, H - 40);
    let (mut aux, _id, group) = tex_two_step_sent_row(&mut st);
    let tx1_expiry = created_tx_expiry(&st, TxId::from_bytes(group[1]));
    assert!(
        tx1_expiry >= H,
        "premise: tx1's stored expiry ({tx1_expiry}) reaches past the activation, so only \
         the upgrade can kill it before then"
    );
    let (mined, _) = st.generate_next_block_including(TxId::from_bytes(group[0]));
    st.scan_cached_blocks(mined, 1);
    assert_eq!(u32::from(mined), H - 39, "premise: tx0 is mined at H - 39");
    let tx0_buried_at = H + 485;
    let stranded_at = H + 599;
    let network = straddling();
    assert_eq!(
        crate::chain_time::chain_time_between(
            &network,
            mined,
            BlockHeight::from_u32(tx0_buried_at)
        ),
        15_000,
        "premise: the spacing function agrees that tx0 is buried at H + 485"
    );
    assert_eq!(
        crate::chain_time::chain_time_between(
            &network,
            BlockHeight::from_u32(H - 1),
            BlockHeight::from_u32(stranded_at)
        ),
        15_000,
        "premise: the spacing function agrees that H - 1 is buried at H + 599"
    );
    let now = 0;

    chain_tip_to(&mut st, H - 2);
    assert!(
        matches!(reconcile_at(&st, &mut aux, now), Reconciled::Rebroadcast { txs: ref r, .. } if r.len() == 1),
        "at the tip H - 2 tx1 can still be mined: re-broadcast it alone"
    );
    for (tip, why) in [
        (H - 1, "tx1 dead from here; tx0 still shallow"),
        (H - 39 + 100, "tx0 100 blocks deep, H - 1 far from buried"),
        (tx0_buried_at - 1, "tx0 one block short of 15,000 s"),
        (
            tx0_buried_at,
            "tx0 buried, H - 1 not: tx1's effective expiry is still reversible",
        ),
        (stranded_at - 1, "H - 1 one block short of 15,000 s"),
    ] {
        chain_tip_to(&mut st, tip);
        assert_eq!(
            reconcile_at(&st, &mut aux, now),
            Reconciled::AwaitingBurial,
            "at the tip {tip} ({why}) the group waits: no re-broadcast, no terminal"
        );
    }
    chain_tip_to(&mut st, stranded_at);
    assert_eq!(
        reconcile_at(&st, &mut aux, now),
        Reconciled::Stranded,
        "at the tip H + 599 tx0 and H - 1 are both irreversibly buried, and tx1 can never \
         be mined: terminal Stranded"
    );
    assert!(
        crate::intent_store::list_in_flight(&aux)
            .expect("in flight")
            .is_empty(),
        "the stranded row leaves the reconcile loop"
    );
}

/// **`a_swap_deposit_killed_by_the_upgrade_ends_through_its_hold_and_purge`** (R-2;
/// `send.rs`'s deposit hold and the drain's purge). A swap deposit signed at target
/// `H - 10` (stored expiry `H + 30`) whose quote has lapsed: from the tip `H - 1` it
/// can never be mined, and it is never broadcast into the dead quote. It is HELD
/// until the block before the activation is irreversibly buried (the tip
/// `H - 1 + 600`), then requeued, and the drain's purge deletes it, never
/// re-proposing it to the dead quote. Nothing new rebuilds it.
///
/// Mutant (L-7): measure the purge from the stored expiry. Red (still held at
/// `H + 599`).
///
/// The tip `H - 11` is inside the pre-upgrade pause's window (ADR-0580), so the
/// deposit is signed with the pause suspended: it stands for one an older build
/// signed there, which the hold and the purge must still end.
#[test]
fn a_swap_deposit_killed_by_the_upgrade_ends_through_its_hold_and_purge() {
    let mut st = funded_on_the_straddle(NOTE_VALUE, H - 11);
    let deadline: i64 = 1_800_000_000;
    let lapsed_now = 1_900_000_000u64;
    let (mut aux, _id, group) = {
        let _inside_the_window = upgrade_pause_testing::suspend();
        deposit_sent_row(&mut st, deadline)
    };
    assert_eq!(
        created_tx_expiry(&st, TxId::from_bytes(group[0])),
        H + 30,
        "premise: the deposit carries the builder's pre-NU7 expiry"
    );
    let buried_at = H - 1 + 600;
    for (tip, why) in [
        (H - 1, "dead from here, not buried"),
        (
            H + 30 + 100,
            "the stored expiry's 100-block burial: not this deposit's",
        ),
        (buried_at - 1, "one block short of the burial of H - 1"),
    ] {
        chain_tip_to(&mut st, tip);
        assert_eq!(
            reconcile_at(&st, &mut aux, lapsed_now),
            Reconciled::DepositBroadcastHold,
            "at the tip {tip} ({why}) the lapsed deposit is held"
        );
        assert_eq!(
            crate::intent_store::list_in_flight(&aux)
                .expect("in flight")
                .len(),
            1,
            "the row stays Sent while it is held"
        );
    }
    chain_tip_to(&mut st, buried_at);
    assert_eq!(
        reconcile_at(&st, &mut aux, lapsed_now),
        Reconciled::Requeued,
        "at the burial of H - 1 the dead deposit requeues"
    );
    assert_eq!(
        purge_lapsed_queued_deposits(&mut aux, lapsed_now).expect("purge"),
        1,
        "and the drain's purge deletes it: never re-proposed to a dead quote"
    );
}
