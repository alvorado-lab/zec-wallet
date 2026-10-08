//! The IMPLEMENTER's own drive of the `TorState` stream end to end (stage S1
//! `truth`): the waker, the one derivation, the pump and the once-per-
//! transition line, on a REAL wallet handle. Not the item's named tests —
//! those are the test author's, written blind. A child of `wallet::tests` so
//! it can use that module's fixtures without adding to it.
//!
//! The evidence is driven into the wallet's posture by hand
//! (`tor_posture_for_test`): the network shape the state answers to — an
//! accepted connection whose RPCs time out — is proven at the gRPC layer
//! (`net/grpc_truth_tests.rs`); what this proves is that the WALLET turns the
//! posture's clocks into a delivered event at the right instant, with no host
//! polling and no pass running.

use std::time::Duration;

use super::*;
use crate::config::TorRuntime;
use crate::constants::TOR_PATIENCE_SECS;
use crate::net::tor_posture::{DialArm, PathClass};
use crate::ports::testing::StubDialer;
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};
use tracing_subscriber::layer::SubscriberExt;

/// A channel-backed sink: every emitted state lands in the receiver.
struct ChannelSink(tokio::sync::mpsc::UnboundedSender<TorState>);

impl TorStateSink for ChannelSink {
    fn emit(&mut self, state: TorState) -> bool {
        self.0.send(state).is_ok()
    }
}

/// A sink that has already gone away — the host paused before the event.
struct PausedSink;

impl TorStateSink for PausedSink {
    fn emit(&mut self, _: TorState) -> bool {
        false
    }
}

/// Let the waker and the pump run under the paused clock.
async fn settle() {
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }
}

fn drain(rx: &mut tokio::sync::mpsc::UnboundedReceiver<TorState>) -> Vec<TorState> {
    let mut out = Vec::new();
    while let Ok(s) = rx.try_recv() {
        out.push(s);
    }
    out
}

/// A `Required` wallet whose private path goes silent: the stream says
/// `Active` now, nothing at 59 s, `Unanswered` at `first_failure + 60 s`
/// with no host poll and no pass, the WARN line written exactly once however
/// many readers derive the state, and `Active` again the moment something
/// comes back — one event per transition. A host that paused its sink before
/// the transition gets nothing later and reads it from the snapshot; a fresh
/// subscribe delivers the truth first.
#[tokio::test(start_paused = true)]
async fn impl_probe_a_required_wallet_announces_the_minute_once_and_the_snapshot_agrees() {
    force_wallet_callsites_enabled();
    let lines = CapturedEvents::default();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(CaptureLayer::new(lines.clone())),
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let mut config = cfg(dir.path(), Network::Test, SeedPersistence::SealedKeychain);
    config.tor = TorPolicy::Required {
        runtime: TorRuntime::Dialer(Arc::new(StubDialer) as Arc<dyn crate::ports::NetDialer>),
    };
    let w = Wallet::create_with_vault(config, raw_seed(), Arc::clone(&vault))
        .await
        .expect("create");
    let active = TorState::Active {
        runtime: crate::state::TorRuntimeKind::Dialer,
    };
    let unanswered = TorState::Unanswered {
        runtime: crate::state::TorRuntimeKind::Dialer,
    };

    // A host paused before anything happened: its pump ends at once.
    w.watch_tor_state(PausedSink);
    // A listening host: the current state first.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    w.watch_tor_state(ChannelSink(tx));
    settle().await;
    assert_eq!(
        drain(&mut rx),
        std::slice::from_ref(&active),
        "the current state, once"
    );

    // The force flag the four wallet-level tests use moves nothing here.
    w.tor_posture_for_test().spend_the_minute_for_test();
    assert_eq!(
        w.tor_state(),
        active,
        "the fixture trap: the flag is not evidence"
    );

    // 100 s in, the first RPC over the private path times out (what
    // `net/grpc.rs` reports through the witness). Nothing is due before a
    // minute after THAT.
    tokio::time::advance(Duration::from_secs(100)).await;
    w.tor_posture_for_test()
        .note_rpc_failure(PathClass::Sync, DialArm::Private);
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
    w.tor_posture_for_test()
        .note_rpc_failure(PathClass::Sync, DialArm::Private);
    settle().await;
    assert!(drain(&mut rx).is_empty(), "59 s of failing: nothing to say");
    assert_eq!(w.tor_state(), active);

    // The RPC a second later fails too, and THAT is the transition: the run
    // has now been SEEN to outlast the window, which is what the value claims
    // happened. The waker is woken by the failure — no deadline is armed for
    // this edge, so if the failure did not wake, nothing would have.
    tokio::time::advance(Duration::from_secs(1)).await;
    w.tor_posture_for_test()
        .note_rpc_failure(PathClass::Sync, DialArm::Private);
    settle().await;
    assert_eq!(
        drain(&mut rx),
        std::slice::from_ref(&unanswered),
        "the failure observed a minute after the run began turned the state, and the pump \
         delivered it"
    );
    assert_eq!(
        lines.records_of("wallet.private_path_unanswered").len(),
        1,
        "one WARN line for the transition"
    );
    // Cold reads derive the same state and write no second line.
    assert_eq!(w.tor_state(), unanswered);
    assert_eq!(w.snapshot().await.expect("open").tor, unanswered);
    settle().await;
    assert!(drain(&mut rx).is_empty(), "a read is not a transition");
    assert_eq!(
        lines.records_of("wallet.private_path_unanswered").len(),
        1,
        "still one line"
    );

    // A host that subscribes now gets the truth first, not a stale `Active`.
    let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
    w.watch_tor_state(ChannelSink(tx2));
    settle().await;
    assert_eq!(drain(&mut rx2), std::slice::from_ref(&unanswered));

    // Something comes back over the private path: the clear wakes the
    // publisher and every listener hears `Active` again, once.
    w.tor_posture_for_test()
        .note_private_success(PathClass::Sync, DialArm::Private);
    settle().await;
    assert_eq!(drain(&mut rx), std::slice::from_ref(&active));
    assert_eq!(drain(&mut rx2), std::slice::from_ref(&active));
    assert_eq!(w.tor_state(), active);

    w.close().await.expect("close");
}
