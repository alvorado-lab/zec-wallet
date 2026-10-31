//! The NU7 wave's step 2b: the test author's row for the second named test, "a
//! test-only failing generator at the seam" (`docs/plan/nu7-wave.md` §3 "Step
//! 2b"). Written blind against the contract, which leaves the hook to the
//! implementer: at the contract commit the hook did not exist and this file did
//! not compile (its red, recorded by the test author), so it was declared under
//! `#[cfg(any())]` until the join (`docs/adjudication/nu7-2b/ruling.md` §2).
//!
//! **The hook it arms:** `crate::rng::testing::fail_next_draw()`, which returns
//! an `EntropyFailure` guard. While the guard lives, the next `wallet_rng()`
//! handle taken on the arming thread, or inside `run_blocking` work that thread
//! dispatches, panics the way `UnwrapErr<SysRng>` panics when the operating
//! system cannot supply entropy: "rand_core::UnwrapErr: failed to unwrap"
//! (`rand_core-0.10.1/src/unwrap_err.rs:52`). It fires ONCE, so every handle
//! after it reads the operating system. That is what turns the contract's
//! mutant (catch the panic and retry) red: a retry inside the create would draw
//! a live generator and sign. The row holds the guard across the first sign and
//! drops it before the second.
//!
//! **Why the row builds its own runtime.** The create runs on tokio's blocking
//! pool (`runtime::run_blocking`), never on the arming thread; the failure
//! reaches it only because `run_blocking` carries the dispatching thread's
//! armed state onto the blocking thread. The row signs inside `tokio::spawn`, to
//! read the panic from the `JoinError`, so that task must run on the arming
//! thread: a current-thread runtime runs it there. On a multi-thread runtime it
//! ran on a worker the guard never reached, and the sign returned `Ok` (measured
//! at the join). Nothing else is spawned on that runtime, so no other work can
//! draw the failure.
//!
//! **Which mechanism refuses the second sign** (IT-10). Not this file, and not a
//! test lock: the wallet's own shared DB mutex, poisoned by the panic inside the
//! locked create (`wallet.rs`'s `sign_proposal_upstream` holds it across
//! `send::create_signed_core`; `runtime.rs`'s `run_blocking` re-raises the
//! panic). The row asserts the poison directly, then the refusal.

use std::sync::Arc;

use super::delivery_obligation::funded_wallet;
use super::*;
use crate::rng::testing::fail_next_draw;

/// The rows the engine stamped `created`: what a create persisted (the stamp
/// `create_proposed_transactions` writes, which the decrypt path never does).
fn signed_rows(w: &Wallet) -> i64 {
    let aux = w.inner.aux_db.lock().expect("aux db");
    aux.query_row(
        "SELECT COUNT(*) FROM transactions WHERE created IS NOT NULL",
        [],
        |r| r.get::<_, i64>(0),
    )
    .expect("count")
}

/// The text of a panic payload (`panic!` with arguments carries a `String`).
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default()
}

/// Contract step 2b, named test 2 (R-8): an entropy failure panics inside the
/// locked create, so the wallet's shared DB mutex is poisoned, nothing is
/// persisted, and the next sign on that wallet is refused.
///
/// Mutant (the contract's): catch the panic inside the create and retry with a
/// fresh generator. Red: the retry draws a live generator and signs, so the
/// first sign returns `Ok` instead of panicking. A second mutant, catch the
/// panic and return `SignFailed`, is red at the same assertion.
#[test]
fn an_entropy_failure_panics_inside_the_locked_create_and_nothing_signs_twice() {
    // A current-thread runtime: the sign's task runs on the thread that arms
    // the failure, so the sign's `run_blocking` carries it to the create (the
    // module doc).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a current-thread runtime");
    rt.block_on(async {
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = test_vault();
        let w = Arc::new(
            funded_wallet(
                &vault,
                cfg(dir.path(), Network::Test, SeedPersistence::SealedKeychain),
            )
            .await,
        );
        // Two live proposals over the same funds (proposing locks nothing): the
        // one the failure hits, and the next sign on the same wallet.
        let first = w
            .propose_shield()
            .await
            .expect("propose")
            .expect("funds above the threshold");
        let next = w
            .propose_shield()
            .await
            .expect("propose")
            .expect("funds above the threshold");
        assert_eq!(signed_rows(&w), 0, "fixture: nothing is signed yet");

        let failure = fail_next_draw();

        let signer = Arc::clone(&w);
        let id = first.proposal_id;
        let outcome = tokio::spawn(async move { signer.sign_proposal_upstream(id).await }).await;
        let payload = match outcome {
            Err(e) if e.is_panic() => e.into_panic(),
            Ok(result) => panic!(
                "an entropy failure inside the create must panic there; the sign returned \
                 {result:?} instead (a caught panic, a retry, or a generator drawn outside \
                 the create)"
            ),
            Err(e) => panic!("the sign task was cancelled, not panicked: {e}"),
        };
        let text = panic_text(&*payload);
        // provenance: contract §3 step 2b item 3 (`UnwrapErr`); the text is
        // rand_core-0.10.1/src/unwrap_err.rs:52
        assert!(
            text.contains("rand_core::UnwrapErr: failed to unwrap"),
            "the sign panicked, but not with the generator's failure: {text:?}"
        );
        assert!(
            w.inner.db.is_poisoned(),
            "the generator failed OUTSIDE the locked create: the wallet's shared DB mutex \
             is not poisoned, so nothing stops a second sign"
        );
        assert_eq!(
            signed_rows(&w),
            0,
            "the failed create persisted a transaction"
        );
        drop(failure);

        // The next sign on that wallet, its generator live again.
        let signer = Arc::clone(&w);
        let id = next.proposal_id;
        let again = tokio::spawn(async move { signer.sign_proposal_upstream(id).await }).await;
        match again {
            Ok(Ok(txids)) => panic!(
                "the wallet signed again after its create panicked ({} transaction(s))",
                txids.len()
            ),
            Ok(Err(_refused)) => {}
            Err(e) if e.is_panic() => {
                let text = panic_text(&*e.into_panic());
                // provenance: implementation, at the contract commit (the shared
                // DB lock's `.expect` in `wallet.rs`)
                assert!(
                    text.contains("poisoned"),
                    "the second sign was refused, but not by the poisoned lock: {text:?}"
                );
            }
            Err(e) => panic!("the second sign task was cancelled, not refused: {e}"),
        }
        assert_eq!(signed_rows(&w), 0, "nothing was signed, once or twice");
    });
}
