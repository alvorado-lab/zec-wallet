//! Stage S8 `obligation`, §3.2 row 10 — the BRIDGE half of
//! `the_four_delivery_states_cross_the_bridge` (the core half is the core
//! crate's `wallet::tests::delivery_obligation_named`). The TEST AUTHOR's row,
//! written blind (IT-2a) against the names declared in the run's
//! `contractFindings`: `api::state::DeliveryState { Persisted, RetryPending,
//! Accepted, Confirmed, Unknown }` (the bridge's forward-compatibility arm, as
//! every bridge enum carries one), `api::state::TxSummary::delivery:
//! Option<DeliveryState>`, and `convert.rs`'s `From<rw::DeliveryState> for
//! api_state::DeliveryState` beside the field on `From<rw::TxSummary>`.
//!
//! Its own file, because `convert.rs` is the implementer's; in the crate,
//! because the crate is a cdylib/staticlib and a `tests/*.rs` cannot link it.
//! No FRB codegen is touched: the regen is the fold's.

use crate::api::state as api_state;
use zec_wallet_core as rw;

/// Each core state crosses to its own bridge arm — never to `Unknown`, which
/// exists for a core variant this build does not know — and the history row
/// carries it, where the activity list reads transaction status.
#[test]
fn the_four_delivery_states_cross_the_bridge() {
    let crossings = [
        (rw::DeliveryState::Persisted, "persisted"),
        (rw::DeliveryState::RetryPending, "retry_pending"),
        (rw::DeliveryState::Accepted, "accepted"),
        (rw::DeliveryState::Confirmed, "confirmed"),
    ];
    for (core, name) in crossings {
        let arm = match api_state::DeliveryState::from(core) {
            api_state::DeliveryState::Persisted => "persisted",
            api_state::DeliveryState::RetryPending => "retry_pending",
            api_state::DeliveryState::Accepted => "accepted",
            api_state::DeliveryState::Confirmed => "confirmed",
            api_state::DeliveryState::Unknown => "unknown",
        };
        assert_eq!(arm, name, "{core:?} crosses to its own arm, never Unknown");
    }

    let row = rw::TxSummary {
        txid: rw::TxId::from_display_hex(&"ab".repeat(32)).expect("a txid"),
        batch_id: None,
        mined_height: None,
        status: rw::TxStatus::Pending,
        net_amount: rw::ZatBalance::new(-1_000).expect("in range"),
        fee: None,
        has_memo: false,
        has_transparent_output: false,
        timestamp: None,
        delivery: Some(rw::DeliveryState::RetryPending),
        expiry_height: None,
    };
    let bridged = api_state::TxSummary::from(row);
    assert!(
        matches!(
            bridged.delivery,
            Some(api_state::DeliveryState::RetryPending)
        ),
        "the per-transaction delivery state rides the history DTO across the bridge"
    );
    assert!(
        matches!(bridged.status, api_state::TxStatus::Pending),
        "beside the status the row already carried"
    );
}

/// Stage S2 `outcome` (§3.5c, FR-41's residual) — the BRIDGE half: the height at
/// which a wallet-created send's unknown outcome resolves crosses on the history
/// DTO as the core read it, and a row with none (a receive) crosses as `null`.
/// The two-case reading rule is asserted against the REAL view by the core half
/// of the same name (`history::tests::real_view`), where the wallet's own scan
/// exists to be read against.
#[test]
fn a_wallet_created_send_reports_the_height_its_unknown_outcome_resolves_at_and_the_rule_reads_it()
{
    let row = |expiry_height: Option<u32>, net: i64| rw::TxSummary {
        txid: rw::TxId::from_display_hex(&"cd".repeat(32)).expect("a txid"),
        batch_id: None,
        mined_height: None,
        status: rw::TxStatus::Pending,
        net_amount: rw::ZatBalance::new(net).expect("in range"),
        fee: None,
        has_memo: false,
        has_transparent_output: false,
        timestamp: None,
        delivery: None,
        expiry_height: expiry_height.map(rw::BlockHeight::new),
    };

    let send = api_state::TxSummary::from(row(Some(2_400_040), -1_000));
    assert_eq!(
        send.expiry_height,
        Some(2_400_040),
        "a send's expiry height crosses the bridge unchanged"
    );

    let received = api_state::TxSummary::from(row(None, 1_000));
    assert_eq!(
        received.expiry_height, None,
        "a row with no expiry height crosses as null, never as 0"
    );
}
