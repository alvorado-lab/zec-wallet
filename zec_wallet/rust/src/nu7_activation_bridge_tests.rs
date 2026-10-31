//! The NU7 wave's step 4, the activation check — the BRIDGE half of
//! `a_send_the_upgrade_cancelled_carries_both_heights_and_its_phase` (the core half
//! is the core crate's `wallet::tests::nu7_activation_named`). The TEST AUTHOR's
//! rows, written blind (IT-2a) against the names the orchestrator decided on
//! 2026-10-30:
//!
//! - core: `rw::TxStatus::CancelledByUpgrade { stopped_at: rw::BlockHeight,
//!   released_at: Option<rw::BlockHeight>, phase: rw::CancelPhase }` (`stopped_at`
//!   is `H`, the activation; `released_at` the stored expiry, `None` for a stored 0)
//!   and `rw::CancelPhase { NotFinal, FinalNotReleased, Released }`, re-exported at
//!   the core crate's root beside `TxStatus`;
//! - bridge: `api::state::TxStatus::CancelledByUpgrade { stopped_at: u32,
//!   released_at: Option<u32>, phase: CancelPhase }` (Dart `stoppedAt` /
//!   `releasedAt`) and `api::state::CancelPhase { NotFinal, FinalNotReleased,
//!   Released, Unknown }` (the bridge's forward-compatibility arm, as every bridge
//!   enum carries one), with `convert.rs`'s `From` arms;
//! - `bridge_abi::BRIDGE_ABI_VERSION == 12` (the orchestrator's number for step 4,
//!   2026-10-29) and the Dart `kBridgeAbiVersion` with it.
//!
//! The host renders the phase and computes nothing (S-M3), so what crosses must be
//! the core's reading exactly: each phase to its own arm, never `Unknown`, and the
//! two heights as the core read them. Its own file because `convert.rs` is the
//! implementer's. Declared under `#[cfg(any())]` until the join.

use crate::api::state as api_state;
use zec_wallet_core as rw;

fn phase_name(p: &api_state::CancelPhase) -> &'static str {
    match p {
        api_state::CancelPhase::NotFinal => "not_final",
        api_state::CancelPhase::FinalNotReleased => "final_not_released",
        api_state::CancelPhase::Released => "released",
        api_state::CancelPhase::Unknown => "unknown",
    }
}

/// Each phase of a cancelled send crosses to its own bridge arm, beside both
/// heights, and the history DTO keeps `expiry_height` as the release height.
#[test]
fn a_cancelled_by_upgrade_reading_crosses_the_bridge_with_both_heights_and_its_phase() {
    let stopped = 3_543_405u32;
    let released = 3_543_435u32;
    for (core, name) in [
        (rw::CancelPhase::NotFinal, "not_final"),
        (rw::CancelPhase::FinalNotReleased, "final_not_released"),
        (rw::CancelPhase::Released, "released"),
    ] {
        let status = rw::TxStatus::CancelledByUpgrade {
            stopped_at: rw::BlockHeight::new(stopped),
            released_at: Some(rw::BlockHeight::new(released)),
            phase: core,
        };
        match api_state::TxStatus::from(status) {
            api_state::TxStatus::CancelledByUpgrade {
                stopped_at,
                released_at,
                phase,
            } => {
                assert_eq!(
                    (stopped_at, released_at),
                    (stopped, Some(released)),
                    "the two heights cross as the core read them"
                );
                assert_eq!(
                    phase_name(&phase),
                    name,
                    "{core:?} crosses to its own arm, never Unknown"
                );
            }
            _ => panic!(
                "a cancelled reading crossed to another status: the host would \
                         render a cancelled send as pending or retrying"
            ),
        }
    }

    let row = rw::TxSummary {
        txid: rw::TxId::from_display_hex(&"ef".repeat(32)).expect("a txid"),
        batch_id: None,
        mined_height: None,
        status: rw::TxStatus::CancelledByUpgrade {
            stopped_at: rw::BlockHeight::new(stopped),
            released_at: Some(rw::BlockHeight::new(released)),
            phase: rw::CancelPhase::NotFinal,
        },
        net_amount: rw::ZatBalance::new(-20_000).expect("in range"),
        fee: None,
        has_memo: false,
        has_transparent_output: false,
        timestamp: None,
        delivery: None,
        expiry_height: Some(rw::BlockHeight::new(released)),
    };
    let bridged = api_state::TxSummary::from(row);
    assert_eq!(
        bridged.expiry_height,
        Some(released),
        "TxSummary.expiry_height stays the release height across the bridge, never H - 1"
    );
    assert!(
        matches!(
            bridged.status,
            api_state::TxStatus::CancelledByUpgrade { .. }
        ),
        "the history row carries the cancelled reading"
    );
}

/// A send stored with no expiry (stored 0) has no release height: the core reads
/// `released_at: None` beside `TxSummary.expiry_height: None`, and both cross as
/// `None` (Dart `releasedAt` and `expiryHeight` null), never `Some(0)`, which a host
/// would read as "released at height 0". Its phase crosses as the core read it, in
/// the two phases such a send can reach (the engine never frees its notes, so it is
/// never released).
///
/// Mutant: `convert.rs` crosses a missing release height as `Some(0)`. Red.
#[test]
fn a_cancelled_send_with_no_stored_expiry_crosses_the_bridge_with_released_at_null() {
    let stopped = 3_543_405u32;
    for (core, name) in [
        (rw::CancelPhase::NotFinal, "not_final"),
        (rw::CancelPhase::FinalNotReleased, "final_not_released"),
    ] {
        let row = rw::TxSummary {
            txid: rw::TxId::from_display_hex(&"0e".repeat(32)).expect("a txid"),
            batch_id: None,
            mined_height: None,
            status: rw::TxStatus::CancelledByUpgrade {
                stopped_at: rw::BlockHeight::new(stopped),
                released_at: None,
                phase: core,
            },
            net_amount: rw::ZatBalance::new(-20_000).expect("in range"),
            fee: None,
            has_memo: false,
            has_transparent_output: false,
            timestamp: None,
            delivery: None,
            expiry_height: None,
        };
        let bridged = api_state::TxSummary::from(row);
        assert_eq!(
            bridged.expiry_height, None,
            "{core:?}: a send stored with no expiry crosses with no expiry height"
        );
        match bridged.status {
            api_state::TxStatus::CancelledByUpgrade {
                stopped_at,
                released_at,
                phase,
            } => {
                assert_eq!(
                    (stopped_at, released_at),
                    (stopped, None),
                    "{core:?}: no release height crosses as null, never Some(0)"
                );
                assert_eq!(phase_name(&phase), name, "{core:?} crosses to its own arm");
            }
            _ => panic!("a cancelled reading crossed to another status"),
        }
    }
}

/// The wire contract moved: a Dart half generated before the new `TxStatus` arm
/// would decode it one tag off (`Unknown`'s tag moves), so the version moves with
/// it, on both sides.
#[test]
fn the_bridge_abi_is_12_for_the_cancelled_reading() {
    assert_eq!(crate::bridge_abi::BRIDGE_ABI_VERSION, 12);
    let dart = include_str!("../../lib/src/bridge_abi.dart");
    assert!(
        dart.contains("const int kBridgeAbiVersion = 12;"),
        "the Dart constant moves with the native one"
    );
}
