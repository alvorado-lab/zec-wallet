//! Stage S2 `budget` (FR-40, contract `docs/plan/stage-2-the-hosts-lifecycle.md`
//! §3.5b) — the bounded sync through the shipped engine: `sync_for(budget)`
//! runs ONE pass whose deadline fires the pass's own cancel token, and reports
//! `{scanned_to, tip, finished, resubmitted}` read from the wallet's database.
//! A child of `wallet::tests` for its fixtures (`wallet_with_account_at`,
//! `ScriptedPassClient`, `h_above_the_row`), in its own file so the cited lines
//! of the parent module stay where their watches printed them.
//!
//! The controller-level rows (the deadline on the virtual clock, the
//! resubmission reserve at its boundary) live beside the controller's other
//! rows in `sync_controller.rs`; these rows are the ones that need a real
//! database under the report.

use std::time::Duration;

use super::*;
use crate::constants::GRPC_UNARY_TIMEOUT_SECS;

/// A budget long enough for the fixture's pass and short of the resubmission
/// reserve — the pass finishes, the outbox drive is skipped.
fn budget_short_of_the_reserve() -> Duration {
    SyncController::RESUBMIT_RESERVE - Duration::from_secs(1)
}

/// A budget that covers the pass AND the reserve.
fn budget_over_the_reserve() -> Duration {
    SyncController::RESUBMIT_RESERVE * 5
}

/// A wallet driven to `h` by a real scan, then a second pass over a chain
/// `EXTRA` blocks longer whose first block download never completes: the pass
/// is inside a batch when the budget runs out. The deadline fires the pass's
/// own token, the pass returns cancelled with nothing half-written, and the
/// report reads how far the DATABASE got — `h`, the first pass's frontier —
/// against the tip the second pass recorded. The call returns inside
/// `budget + GRPC_UNARY_TIMEOUT_SECS`, and not before its budget (it was the
/// deadline that ended it, not a fault). A later unbounded pass resumes and
/// finishes from there: the database is consistent, not merely unchanged.
#[tokio::test]
async fn a_bounded_sync_stops_within_its_bound_and_reports_how_far_it_got() {
    const EXTRA: u32 = 500;
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_with_account_at(dir.path(), &vault, u64::from(h) - 250).await;

    let ctl = w.controller_over(ScriptedPassClient::current(h));
    let first = w
        .bounded_sync(&ctl, budget_short_of_the_reserve())
        .await
        .expect("the first pass");
    assert!(first.finished, "precondition: the first pass reaches H");
    assert_eq!(
        w.scanned_tip().await.expect("scanned"),
        Some(BlockHeight::new(h)),
        "precondition: the wallet holds H"
    );
    drop(ctl);

    // Every later pass serves no roots: on this synthetic chain a non-empty
    // re-serve after the scan is refused by the scanned-count oracle (the
    // `a_switch_onto_a_server_behind…` row's reason) — a stall, not this story.
    let (started_tx, mut started_rx) = tokio::sync::oneshot::channel();
    let (_release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    let mut longer = ScriptedPassClient::honest(h + EXTRA);
    longer.chain.download_gate = Some((started_tx, release_rx));
    let ctl2 = w.controller_over(longer);

    let budget = Duration::from_secs(2);
    let t0 = std::time::Instant::now();
    let report = w
        .bounded_sync(&ctl2, budget)
        .await
        .expect("a pass the deadline stops is a report, never an error");
    let elapsed = t0.elapsed();

    let (gated_from, _) = started_rx
        .try_recv()
        .expect("the fixture's own non-vacuity: the pass was INSIDE a download when it was cut");
    assert!(
        gated_from > u64::from(h),
        "the cut batch is above what the wallet holds (it asked from {gated_from})"
    );
    assert!(
        !report.finished,
        "a pass cut by its deadline is not finished"
    );
    assert!(
        !report.resubmitted,
        "a pass cut by its deadline drives no resubmission"
    );
    assert_eq!(
        report.tip,
        Some(BlockHeight::new(h + EXTRA)),
        "the tip is the one this pass recorded, not the last pass's"
    );
    assert_eq!(
        report.scanned_to,
        Some(BlockHeight::new(h)),
        "how far it got: the database's own frontier — the first pass's H"
    );
    assert!(
        report.scanned_to < report.tip,
        "an unfinished pass reports a frontier below the tip"
    );
    assert!(
        elapsed >= budget,
        "the deadline ended the pass, not something earlier ({elapsed:?} < {budget:?})"
    );
    assert!(
        elapsed <= budget + Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS),
        "the call returns within budget + GRPC_UNARY_TIMEOUT_SECS; took {elapsed:?}"
    );
    assert_eq!(
        w.scanned_tip().await.expect("scanned"),
        Some(BlockHeight::new(h)),
        "no part of the cut batch is in the database"
    );
    drop(ctl2);

    // Consistent, not merely unchanged: an unbounded pass over the same chain
    // resumes from H and finishes at the new tip.
    let ctl3 = w.controller_over(ScriptedPassClient::honest(h + EXTRA));
    let resumed = w
        .bounded_sync(&ctl3, budget_short_of_the_reserve())
        .await
        .expect("the resumed pass");
    assert!(resumed.finished, "the resumed pass finishes");
    assert_eq!(resumed.scanned_to, Some(BlockHeight::new(h + EXTRA)));
    assert_eq!(resumed.tip, Some(BlockHeight::new(h + EXTRA)));
    drop(ctl3);
    w.close().await.expect("close");
}

/// A pass that reaches the tip reports `finished` with the frontier AT the tip,
/// and says whether it drove the outbox: a budget whose remainder after the
/// pass is short of the resubmission reserve skips it (`resubmitted: false` —
/// the next `start_sync` pass carries the obligation), one that covers it runs
/// it (`resubmitted: true`).
#[tokio::test]
async fn a_bounded_sync_that_finishes_reports_finished_and_whether_it_resubmitted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let h = h_above_the_row();
    let w = wallet_with_account_at(dir.path(), &vault, u64::from(h) - 250).await;

    let ctl = w.controller_over(ScriptedPassClient::current(h));
    let short = w
        .bounded_sync(&ctl, budget_short_of_the_reserve())
        .await
        .expect("the short-budget pass");
    assert!(short.finished, "the pass reaches the tip");
    assert_eq!(short.tip, Some(BlockHeight::new(h)));
    assert_eq!(
        short.scanned_to,
        Some(BlockHeight::new(h)),
        "a finished pass's frontier is the tip"
    );
    assert!(
        !short.resubmitted,
        "less than the reserve left after the pass: the resubmission is skipped, and said so"
    );

    drop(ctl);

    // The second pass serves no roots (the first row's note on re-serves).
    let ctl2 = w.controller_over(ScriptedPassClient::honest(h));
    let long = w
        .bounded_sync(&ctl2, budget_over_the_reserve())
        .await
        .expect("the long-budget pass");
    assert!(long.finished, "the second pass is at the tip already");
    assert_eq!(long.scanned_to, Some(BlockHeight::new(h)));
    assert!(
        long.resubmitted,
        "the reserve fits: the resubmission ran, and said so"
    );
    drop(ctl2);
    w.close().await.expect("close");
}

/// Not named by the contract: the refusal it states ("refuses typed while
/// `start_sync` runs") through the handle's OWN controller, and its end — once
/// the loop is stopped the same call is no longer refused. The endpoint is one
/// nothing listens on (the `a_switch_emits_the_sync_server_span…` precedent):
/// the loop starts and stalls; the bounded pass after `stop_sync` fails its
/// dial or runs out of budget, either of which is not the refusal.
#[tokio::test]
async fn a_bounded_sync_is_refused_typed_while_the_loop_runs_and_not_after_it_stops() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let mut c = cfg(dir.path(), Network::Test, SeedPersistence::SealedKeychain);
    c.endpoint = LightServerEndpoint::new("https://127.0.0.1:1").expect("endpoint");
    let w = Wallet::create_with_vault(c, raw_seed(), Arc::clone(&vault))
        .await
        .expect("create");

    w.start_sync().expect("start the loop");
    let refused = w.sync_for(Duration::from_secs(1)).await;
    assert!(
        matches!(refused, Err(WalletError::SyncRunning)),
        "a bounded sync while the loop runs is refused typed; got {refused:?}"
    );

    w.stop_sync().await;
    let after = w.sync_for(Duration::from_secs(1)).await;
    assert!(
        !matches!(after, Err(WalletError::SyncRunning)),
        "with the loop stopped the bounded sync runs; got {after:?}"
    );
    w.close().await.expect("close");
}
