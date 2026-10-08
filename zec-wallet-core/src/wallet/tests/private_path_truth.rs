//! Stage S1 `truth` — the private path tells the truth — the TEST AUTHOR's
//! wallet-level rows (`docs/plan/stage-1-private-path-truth.md` §3.0 F-C and
//! §3.2, written blind against the contract, IT-2a). A child of `wallet.rs`'s
//! test module so it reaches the fixtures there (`cfg`, `test_vault`,
//! `raw_seed`) and the handle's private seams (`broadcast_one`,
//! `inner.sync.once()`); its own file so the cited lines above stay where
//! their watches printed them.
//!
//! Every row reads the USER-VISIBLE surface — `tor_state()`, the sync status,
//! the state stream, the counters — beside the `wallet.dial` capture; none
//! reads a mock's call count, and none leans on `spend_the_minute_for_test`
//! where the new value is the subject (§3.1's fixture trap, restated in §3.2).
//!
//! THE CLOCK, because it shapes every row here. A wallet's DB work runs on the
//! blocking pool, and a sync pass holds a 600-s watchdog timer for its whole
//! life; a paused runtime that parks with nothing to poll leaps to its next
//! timer, so a pass cannot run paused through its DB prologue — the watchdog
//! would fire during the first store read. The rows that need virtual time
//! therefore build their wallets on the real clock and pause only for the
//! drive; the drive is `broadcast_one` (no DB work), or a sync pass paused
//! only once its RPC is in flight (the dial line is on the capture, so the
//! prologue is behind it) and resumed after.
//!
//! THE PRIVATE PEER, and why some rows drive the posture by hand. Under a host
//! runtime the door admits `https` only, so a private peer that ANSWERS would
//! have to complete a TLS handshake against WebPKI roots, which no in-memory
//! peer can. A silent or hanging-up peer is driven end to end (the exit
//! gate's shape); the rows that need a success — the cancel at 59 s, the
//! recovery — record it on the wallet's own posture through the same entry
//! points `net/grpc.rs`'s witness uses, and read the stream.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::*;
use crate::config::TorRuntime;
use crate::constants::{GRPC_UNARY_TIMEOUT_SECS, TOR_PATIENCE_SECS};
use crate::error::DialError;
use crate::lifecycle::LifecyclePhase;
use crate::net::host_dialer::testing::{ScriptedHostDialer, name};
use crate::net::host_dialer::{
    HOST_TRANSPORT_READY, HostDialCode, HostDialer, HostTransportDescriptor, IsolationSupport,
    TransportExposure, TransportHealth,
};
use crate::net::tor_posture::{DialArm, PathClass};
use crate::ports::testing::FailingDialer;
use crate::ports::{AsyncByteStream, NetDialer};
use crate::state::{DialCounts, TorRuntimeKind};
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};

/// Accepts every dial and carries nothing: the peer end drains into the void
/// and never writes — the blackhole. The TLS handshake the connector layers
/// on it hangs until the connector's bound.
struct SilentPeer;

#[async_trait]
impl NetDialer for SilentPeer {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        _isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        use tokio::io::AsyncReadExt;
        let (client_end, mut server_end) = tokio::io::duplex(8192);
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            while let Ok(n) = server_end.read(&mut buf).await {
                if n == 0 {
                    break;
                }
            }
        });
        Ok(Box::new(client_end))
    }
}

/// Accepts every dial and hangs up at once: the peer end is dropped, so the
/// handshake meets EOF immediately — a connection that was MADE and then
/// failed, promptly, which is F-C's other shape (a transport failure over an
/// accepted private connection).
struct HangsUp;

#[async_trait]
impl NetDialer for HangsUp {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        _isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        let (client_end, _server_end) = tokio::io::duplex(64);
        Ok(Box::new(client_end))
    }
}

/// Install the `wallet.dial` capture for this test's thread; the guard must
/// outlive every dial the test drives. Thread-local, so it sees the dials of
/// every wallet this test builds — a row scoping counts to ONE wallet slices
/// the capture by order.
fn capture() -> (CapturedEvents, tracing::subscriber::DefaultGuard) {
    use tracing_subscriber::layer::SubscriberExt;
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(CaptureLayer::new(sink.clone())),
    );
    (sink, guard)
}

/// One `wallet.dial` line as `(arm, class, outcome)`.
type DialLine = (String, String, String);

fn line(arm: &str, class: &str, outcome: &str) -> DialLine {
    (arm.to_owned(), class.to_owned(), outcome.to_owned())
}

/// Every `wallet.dial` line captured so far, in order.
fn dial_lines(sink: &CapturedEvents) -> Vec<DialLine> {
    sink.records_of("wallet.dial")
        .into_iter()
        .map(|fields| {
            let get = |name: &str| {
                fields
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            };
            (get("dial_arm"), get("dial_class"), get("outcome"))
        })
        .collect()
}

/// The counters a run of `wallet.dial` lines adds up to — what the number
/// and the log line must agree on (§3.2 "The counters", §3.2a).
fn tally(lines: &[DialLine]) -> DialCounts {
    let mut counts = DialCounts::default();
    for (arm, _, outcome) in lines {
        let arm = match arm.as_str() {
            "host" => &mut counts.private,
            "sdk_direct" => &mut counts.clearnet,
            other => panic!("a dial line with an arm outside the closed set: {other:?}"),
        };
        let slot = match outcome.as_str() {
            "connected" => &mut arm.connected,
            "not_ready" => &mut arm.not_ready,
            "retired" => &mut arm.retired,
            "unreachable" => &mut arm.unreachable,
            "timeout" => &mut arm.timeout,
            "unsupported" => &mut arm.unsupported,
            "io" => &mut arm.io,
            "transport_failed" => &mut arm.transport_failed,
            other => panic!("a dial line with an outcome outside the closed set: {other:?}"),
        };
        *slot += 1;
    }
    counts
}

/// Wait, on the real clock and bounded, until the capture holds `n` dial
/// lines — the point at which an RPC is in flight over the connection its
/// line names, and a clock may be paused with no DB work behind it.
async fn wait_for_dial_lines(sink: &CapturedEvents, n: usize) {
    let started = std::time::Instant::now();
    while sink.records_of("wallet.dial").len() < n {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "fixture: {n} dial line(s) never appeared within 60 s: {:?}",
            dial_lines(sink)
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// Hold a PAUSED clock to small steps while a REAL socket is in flight (the
/// `net/grpc.rs` fixture, for the same reason: an idle leap onto a dial bound
/// would turn a refused loopback connect into a timeout, or cut it before it
/// writes its line).
fn hold_the_paused_clock_to_small_steps() {
    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
}

/// An `https` loopback endpoint nothing listens on. Under a host runtime the
/// door admits `https` only, and the private leg's TLS rides the test
/// dialer's duplex; the port matters only to a clearnet dial, which no
/// `Required` row makes and every `Preferred` row counts.
async fn dead_https_loopback() -> LightServerEndpoint {
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        listener.local_addr().expect("addr").port()
    };
    LightServerEndpoint::new(format!("https://127.0.0.1:{port}")).expect("loopback endpoint")
}

/// A LIVE loopback listener that accepts and hangs up: a clearnet connection
/// that establishes (its dial line reads `connected`) and then fails fast.
/// `http` for an `Off` wallet (plaintext loopback is the development
/// configuration), `https` for a `Preferred` fallback leg.
async fn accept_and_hang_up_loopback(
    scheme: &str,
) -> (LightServerEndpoint, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let task = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            drop(socket);
        }
    });
    (
        LightServerEndpoint::new(format!("{scheme}://127.0.0.1:{port}"))
            .expect("loopback endpoint"),
        task,
    )
}

/// A wallet under `policy` against `endpoint`, on the real clock (its DB work
/// is why — the module doc).
async fn wallet_over(
    dir: &std::path::Path,
    vault: &Arc<dyn KeychainPort>,
    policy: TorPolicy,
    endpoint: LightServerEndpoint,
) -> Wallet {
    let mut config = cfg(dir, Network::Test, SeedPersistence::SealedKeychain);
    config.endpoint = endpoint;
    config.tor = policy;
    Wallet::create_with_vault(config, raw_seed(), Arc::clone(vault))
        .await
        .expect("create")
}

fn required_over(primary: Arc<dyn NetDialer>) -> TorPolicy {
    TorPolicy::Required {
        runtime: TorRuntime::Dialer(primary),
    }
}

fn preferred_over(primary: Arc<dyn NetDialer>) -> TorPolicy {
    TorPolicy::Preferred {
        runtime: TorRuntime::Dialer(primary),
    }
}

fn active_dialer() -> TorState {
    TorState::Active {
        runtime: TorRuntimeKind::Dialer,
    }
}

fn unanswered_dialer() -> TorState {
    TorState::Unanswered {
        runtime: TorRuntimeKind::Dialer,
    }
}

/// A failing run the wallet is SEEN to be in for the whole minute: the first
/// failure, the window spent, then the failure observed PAST it — which is the
/// edge `TorPosture::is_unanswered` turns on. Returns the FIRST failure's
/// instant, which is what the timing rows measure from.
///
/// TWO failures, and the second is the point. The predicate reads a run's
/// OBSERVED span, not the clock since it opened: one failure followed by an
/// idle gap is a wallet that stopped trying, and the value would then be a
/// claim about a path nobody touched (`Instant` keeps running while an app is
/// merely backgrounded, so that gap is the common case, not a contrived one).
/// Every row below that drives the posture directly comes through here, so no
/// fixture can reach the value on a run the wallet abandoned — which is what
/// they all did before the fold, and what made them pass over a predicate that
/// was reading the clock.
///
/// The caller's paused clock is spent HERE, so a row that asserts on the
/// delivery instant still measures a real minute from the first failure.
async fn a_minute_of_trying(posture: &TorPosture) -> Instant {
    let first = Instant::now();
    posture.note_private_failure(PathClass::Sync);
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS)).await;
    posture.note_private_failure(PathClass::Sync);
    first
}

/// The bytes `broadcast_one` submits: arbitrary — the seam owns no tx
/// semantics, and every private peer here fails before they matter.
fn raw_tx() -> Vec<u8> {
    vec![0x05, 0x00, 0x00, 0x80]
}

// ── the state stream, as a test sees it ──────────────────────────────────────

/// A [`TorStateSink`] over a channel, stamping each event with the (virtual)
/// instant it was delivered. `emit_limit` is a host that pauses: after that
/// many emits the sink reports closed, and the pump must stop.
struct StampedSink {
    tx: mpsc::UnboundedSender<(Instant, TorState)>,
    emitted: usize,
    emit_limit: Option<usize>,
}

impl TorStateSink for StampedSink {
    fn emit(&mut self, state: TorState) -> bool {
        let open = self.tx.send((Instant::now(), state)).is_ok();
        self.emitted += 1;
        open && self.emit_limit.is_none_or(|n| self.emitted < n)
    }
}

type Events = mpsc::UnboundedReceiver<(Instant, TorState)>;

fn subscribe(w: &Wallet) -> Events {
    let (tx, rx) = mpsc::unbounded_channel();
    w.watch_tor_state(StampedSink {
        tx,
        emitted: 0,
        emit_limit: None,
    });
    rx
}

/// A subscription whose host pauses after `emits` deliveries.
fn subscribe_until_paused(w: &Wallet, emits: usize) -> Events {
    let (tx, rx) = mpsc::unbounded_channel();
    w.watch_tor_state(StampedSink {
        tx,
        emitted: 0,
        emit_limit: Some(emits),
    });
    rx
}

#[derive(Debug)]
enum Next {
    Event(Instant, TorState),
    /// Nothing within the wait — under a paused clock the wait is virtual,
    /// so this is "nothing for that long".
    Nothing,
    /// The pump dropped the sink.
    Ended,
}

async fn next_within(events: &mut Events, within: Duration) -> Next {
    match tokio::time::timeout(within, events.recv()).await {
        Ok(Some((at, state))) => Next::Event(at, state),
        Ok(None) => Next::Ended,
        Err(_elapsed) => Next::Nothing,
    }
}

/// The current-first emission every subscription opens with.
async fn current_first(events: &mut Events) -> TorState {
    match next_within(events, Duration::from_secs(5)).await {
        Next::Event(_, state) => state,
        other => panic!("the current state streams first, at once: {other:?}"),
    }
}

async fn expect_event(events: &mut Events, within: Duration, why: &str) -> (Instant, TorState) {
    match next_within(events, within).await {
        Next::Event(at, state) => (at, state),
        Next::Nothing => panic!("{why}: nothing was delivered within {within:?}"),
        Next::Ended => panic!(
            "{why}: the stream ENDED — a pump that closed on a transient fault, or one that \
             never lived past its first emission"
        ),
    }
}

async fn expect_silence(events: &mut Events, for_: Duration, why: &str) {
    match next_within(events, for_).await {
        Next::Nothing => {}
        Next::Event(at, state) => panic!("{why}: {state:?} was delivered at {at:?}"),
        Next::Ended => panic!(
            "{why}: the stream ENDED — a pump that closed on a transient fault, or one that \
             never lived past its first emission"
        ),
    }
}

// ── §3.0 F-C: `Required` and the accepted connection ─────────────────────────

/// §3.0 F-C, SUPERSEDING the wallet's own reading (and `tor_status.rs`'s
/// stale doc): under `Required`, a private path that ACCEPTS the connection
/// and carries nothing reads `Active` — the path is up as far as the wallet
/// can see — with the sync axis `Stalled { EndpointUnreachable }`: the far end
/// did not answer, the path or the server. Today it reads `Unavailable` at
/// that instant, "the private path is unavailable", which is what a wedged
/// SERVER over a healthy circuit gets rendered as (the over-claim).
///
/// Driven through the real engine: one `once()` pass whose first RPC dials
/// the silent peer and sits out its bound. The clock is paused only once that
/// dial is on the capture (the pass's DB prologue is behind it), so the bound
/// is virtual and the pass returns doing no DB work under a paused clock.
#[tokio::test]
async fn required_over_an_accepted_and_silent_path_reads_active_and_endpoint_unreachable() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    assert_eq!(
        w.tor_state(),
        active_dialer(),
        "fixture: nothing has said otherwise yet"
    );

    let inner = Arc::clone(&w.inner);
    let pass = tokio::spawn(async move { inner.sync.once().await });
    wait_for_dial_lines(&sink, 1).await;
    assert_eq!(
        dial_lines(&sink)[0],
        line("host", "sync", "connected"),
        "fixture: the private path ACCEPTED the sync dial"
    );
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(GRPC_UNARY_TIMEOUT_SECS + 1)).await;
    let outcome = pass.await.expect("the pass task did not panic");
    assert!(
        outcome.is_err(),
        "fixture: the pass fails on a path that carries nothing"
    );

    assert_eq!(
        w.inner.sync.status(),
        SyncStatus::Stalled {
            reason: StallReason::EndpointUnreachable
        },
        "an accepted connection that carried nothing is the far end not answering — the path \
         or the server — never \"the private path is unavailable\""
    );
    assert_eq!(
        w.tor_state(),
        active_dialer(),
        "the path ACCEPTED, so nothing is claimed against it: `Active`, with the stall on the \
         sync axis — `Unavailable` is a claim the evidence cannot make"
    );
    assert!(
        dial_lines(&sink).iter().all(|(arm, _, _)| arm == "host"),
        "Required sent nothing in the clear: {:?}",
        dial_lines(&sink)
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

/// §3.0 F-C's other shape, and the same reading: the private path ACCEPTS and
/// then hangs up, so the connection was made and failed — a transport
/// failure `map_unary` classifies, promptly, on the real clock. The word for
/// it is the endpoint's, and the state stays `Active`. (Today every transport
/// failure under `Required` is stamped `TorUnavailable`, the plan's one
/// reason, so this reads `Unavailable` too.)
#[tokio::test]
async fn required_over_a_path_that_accepts_and_hangs_up_reads_active_and_endpoint_unreachable() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(HangsUp)),
        dead_https_loopback().await,
    )
    .await;

    let outcome = w.inner.sync.once().await;
    assert!(
        outcome.is_err(),
        "fixture: the pass fails on a connection that hung up"
    );
    assert_eq!(
        dial_lines(&sink),
        [line("host", "sync", "connected")],
        "fixture: ONE accepted private dial, nothing in the clear"
    );

    assert_eq!(
        w.inner.sync.status(),
        SyncStatus::Stalled {
            reason: StallReason::EndpointUnreachable
        },
        "a connection that was made and then failed is the endpoint's fault to render, \
         never the private path's"
    );
    assert_eq!(
        w.tor_state(),
        active_dialer(),
        "accepted, then failed: the state does not blame the path"
    );
    w.close().await.expect("close");
}

/// The `Required` case that KEEPS today's reading, with its own row (§3.2
/// "`Required`, two cases"): a dial the private path REFUSES made no
/// connection, so `Unavailable` is right, PROMPTLY — and it stays right past
/// the maintainer's minute. A refused dial is still a failure the posture
/// records, so by the minute the window reads spent; the state must not turn
/// that into the either/or value, which is reserved for a path that accepted.
///
/// Two claims, and the base commit refutes the FIRST: the fixture clause reads
/// the WALLET's posture and expects the refusals on it, which is §3.2 Q2
/// (`Required`'s plan shares the wallet posture) — at base `fail_closed` holds
/// a private posture the wallet never sees (F-B), so the clause is red there.
/// Once it holds, the assertion after it is the row's own: a spent window
/// never outranks the fail-closed stall. The clock is advanced between the
/// two passes and RESUMED before the second, so each pass runs its DB work on
/// a live clock (the module doc).
#[tokio::test]
async fn required_over_a_refused_dial_stays_unavailable_however_long_it_refuses() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(FailingDialer)),
        dead_https_loopback().await,
    )
    .await;

    assert!(w.inner.sync.once().await.is_err());
    assert_eq!(
        w.tor_state(),
        TorState::Unavailable { transport: None },
        "a REFUSED dial reads Unavailable at once — no minute of doubt for a path that \
         made no connection"
    );
    assert_eq!(
        w.inner.sync.status(),
        SyncStatus::Stalled {
            reason: StallReason::TorUnavailable
        }
    );

    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(2 * TOR_PATIENCE_SECS + 1)).await;
    tokio::time::resume();
    assert!(w.inner.sync.once().await.is_err());
    assert!(
        w.tor_posture_for_test()
            .may_switch_to_direct(PathClass::Sync),
        "fixture: two refusals a couple of minutes apart — the sync window reads spent"
    );
    assert_eq!(
        w.tor_state(),
        TorState::Unavailable { transport: None },
        "past the minute a refusing path is STILL unavailable — a spent window never \
         outranks the fail-closed stall, and the either/or value is not for a path that \
         refused"
    );
    assert_eq!(
        w.inner.sync.status(),
        SyncStatus::Stalled {
            reason: StallReason::TorUnavailable
        }
    );
    assert_eq!(
        dial_lines(&sink),
        [
            line("host", "sync", "unreachable"),
            line("host", "sync", "unreachable"),
        ],
        "fixture: two refused private dials, nothing in the clear"
    );
    w.close().await.expect("close");
}

/// THE SIBLING THE FAIL-CLOSED ARM COULD NOT SEE — the fold's fourth finding,
/// end to end. The row above drives the SYNC class, whose refusals reach the
/// state through `SyncStatus::Stalled { TorUnavailable }`. Nothing writes that
/// status for any other class, so a `Required` wallet that SENDS before the
/// sync loop has ever run — status `Idle`, which is where a freshly-opened
/// wallet sits — had its refused broadcast dials seen by the posture and by
/// nobody else: the state read `Active`, then the either/or value at the
/// minute, for a path that was provably refusing connections.
///
/// The claim is about the WORD, not the clock: a path that refuses is not "the
/// path or the server", because a refusal is the one piece of evidence that
/// separates them. The prompt reading for the sync class is untouched.
#[tokio::test]
async fn a_required_send_whose_dials_are_refused_reads_unavailable_not_the_either_or_value() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(FailingDialer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());
    assert_eq!(
        w.inner.sync.status(),
        SyncStatus::Idle,
        "fixture: no sync pass has run, so nothing writes the fail-closed stall"
    );

    tokio::time::pause();
    assert!(
        w.broadcast_one(raw_tx()).await.is_err(),
        "fixture: the send's dial is refused"
    );
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS)).await;
    assert!(w.broadcast_one(raw_tx()).await.is_err());

    let (_, state) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "the minute of a refusing path is delivered",
    )
    .await;
    assert_eq!(
        state,
        TorState::Unavailable { transport: None },
        "the host transport REFUSED both dials: the path is down, and the SDK says so \
         instead of handing the user a choice between two causes it can already decide"
    );
    assert_eq!(
        w.inner.sync.status(),
        SyncStatus::Idle,
        "and it did NOT come from the sync axis — that is the whole finding"
    );
    assert_eq!(w.tor_state(), state, "the snapshot agrees with the event");
    assert_eq!(
        dial_lines(&sink),
        vec![line("host", "broadcast", "unreachable"); 2],
        "two refused private dials, nothing in the clear"
    );
    assert!(
        !w.tor_posture_for_test().fell_back(),
        "Required never latches"
    );

    // …and the reading is not a latch either: one confirmed private RPC ends
    // the run, and with it the claim.
    w.tor_posture_for_test()
        .note_private_success(PathClass::Broadcast, DialArm::Private);
    let (_, back) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "the path carried: the claim ends with the run that carried it",
    )
    .await;
    assert_eq!(back, active_dialer());
    tokio::time::resume();
    w.close().await.expect("close");
}

// ── §3.2: the stream verb and the not-carrying value ─────────────────────────

/// §3.2a's stream verb, the `watch_sync_status` contract: the CURRENT state
/// first, then each transition exactly once, never a close on a fault, and
/// the pump ending — dropping the sink — when the wallet tears down. The
/// transitions are the item's own: not-carrying at the minute, delivered
/// while the host listens (the exit gate's first clause), then the first
/// confirmed private RPC returning the state with ONE event.
#[tokio::test]
async fn watch_tor_state_streams_the_current_state_first_then_each_transition_once_and_ends_on_close()
 {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(
        current_first(&mut events).await,
        active_dialer(),
        "the current state streams first — a re-subscribing host renders at once"
    );

    tokio::time::pause();
    let posture = Arc::clone(w.tor_posture_for_test());
    let first_failure = a_minute_of_trying(&posture).await;
    let (at, state) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "the not-carrying transition is DELIVERED to a host that is listening",
    )
    .await;
    assert_eq!(state, unanswered_dialer());
    let after = at.duration_since(first_failure);
    assert!(
        after >= Duration::from_secs(TOR_PATIENCE_SECS)
            && after < Duration::from_secs(TOR_PATIENCE_SECS + 1),
        "delivered {after:?} after the failure was first observed — the minute, not sooner \
         and not a later wake"
    );
    assert_eq!(
        w.tor_state(),
        unanswered_dialer(),
        "the snapshot agrees with the event"
    );

    // The first completed private RPC returns the state — with ONE event.
    posture.note_private_success(PathClass::Sync, DialArm::Private);
    let (_, back) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "the recovery is delivered",
    )
    .await;
    assert_eq!(back, active_dialer(), "back to Active, nothing in between");
    expect_silence(
        &mut events,
        Duration::from_secs(2 * TOR_PATIENCE_SECS),
        "one transition, one event: nothing follows a recovery",
    )
    .await;
    assert_eq!(w.tor_state(), active_dialer());

    tokio::time::resume();
    w.close().await.expect("close");
    match next_within(&mut events, Duration::from_secs(5)).await {
        Next::Ended => {}
        other => panic!(
            "the pump ends and drops the sink when the wallet tears down, with no duplicate \
             on the way out: {other:?}"
        ),
    }
}

/// §3.2 "The WAKER": the binding deadline is `first_failure + the minute` —
/// NOT `created + the minute` (an idle class has no deadline, and a run
/// usually starts late in a session) — and it fires on its own, with no dial
/// to notice it: the wallet here never dials at all. A run that starts ten
/// minutes in is announced exactly a minute after ITS first failure.
#[tokio::test]
async fn unanswered_fires_at_the_first_failure_plus_the_minute_from_a_run_that_starts_late() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());

    tokio::time::pause();
    expect_silence(
        &mut events,
        Duration::from_secs(10 * TOR_PATIENCE_SECS),
        "ten idle minutes: an idle wallet has no deadline (`created + window` is not it)",
    )
    .await;
    let posture = Arc::clone(w.tor_posture_for_test());
    let first_failure = Instant::now();
    posture.note_private_failure(PathClass::Sync);
    expect_silence(
        &mut events,
        Duration::from_secs(TOR_PATIENCE_SECS - 1),
        "one second short of the minute after the FIRST failure",
    )
    .await;
    // Still trying, and still a second short: a failure INSIDE the window does
    // not turn it either — the span is measured on the run, from ITS first
    // failure, so ten idle minutes before it bought nothing.
    posture.note_private_failure(PathClass::Sync);
    expect_silence(
        &mut events,
        Duration::from_millis(200),
        "a failure a second short of the window is still inside it",
    )
    .await;
    tokio::time::advance(Duration::from_millis(800)).await;
    posture.note_private_failure(PathClass::Sync);
    let (at, state) = expect_event(
        &mut events,
        Duration::from_secs(2),
        "the minute, measured from THIS run's first failure",
    )
    .await;
    assert_eq!(state, unanswered_dialer());
    let after = at.duration_since(first_failure);
    assert!(
        after >= Duration::from_secs(TOR_PATIENCE_SECS)
            && after < Duration::from_secs(TOR_PATIENCE_SECS + 1),
        "fired {after:?} after the first failure — the minute runs from `first_failure`, \
         never from `created`, and it is not read off a later wake"
    );
    assert!(
        dial_lines(&sink).is_empty(),
        "no dial was made to notice it: the run was driven at the posture: {:?}",
        dial_lines(&sink)
    );
    assert_eq!(w.tor_state(), unanswered_dialer());
    tokio::time::resume();
    w.close().await.expect("close");
}

/// NOT IN THE CONTRACT'S LIST (IT-1 +A), grown from a listed one. A success at
/// 59 s cancels the deadline (listed) — and the run AFTER it gets a minute of
/// its own, measured from ITS first failure: not the cancelled deadline (long
/// past), not a minute from the success. A waker armed once at the first
/// 0→non-zero edge and never re-armed, or one that keeps the old deadline
/// through a clear, announces the second run at the wrong instant or never.
#[tokio::test]
async fn a_success_at_59_s_cancels_the_deadline_and_the_next_run_gets_its_own_minute() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());
    let posture = Arc::clone(w.tor_posture_for_test());

    tokio::time::pause();
    posture.note_private_failure(PathClass::Sync);
    expect_silence(
        &mut events,
        Duration::from_secs(TOR_PATIENCE_SECS - 1),
        "inside the minute",
    )
    .await;
    // t = 59 s: the private path carries. The deadline is cancelled.
    posture.note_private_success(PathClass::Sync, DialArm::Private);
    expect_silence(
        &mut events,
        Duration::from_secs(2 * TOR_PATIENCE_SECS + 1),
        "a success at 59 s cancelled the deadline: no event at the minute, or ever, from \
         that run",
    )
    .await;
    assert_eq!(w.tor_state(), active_dialer());

    // A NEW run, three minutes into the session: its own minute.
    let first_failure = Instant::now();
    posture.note_private_failure(PathClass::Sync);
    expect_silence(
        &mut events,
        Duration::from_secs(TOR_PATIENCE_SECS - 1),
        "the new run is inside its own minute",
    )
    .await;
    tokio::time::advance(Duration::from_secs(1)).await;
    posture.note_private_failure(PathClass::Sync);
    let (at, state) = expect_event(
        &mut events,
        Duration::from_secs(2),
        "the new run's own minute",
    )
    .await;
    assert_eq!(state, unanswered_dialer());
    let after = at.duration_since(first_failure);
    assert!(
        after >= Duration::from_secs(TOR_PATIENCE_SECS)
            && after < Duration::from_secs(TOR_PATIENCE_SECS + 1),
        "fired {after:?} after the NEW run's first failure — the run's clock starts where \
         IT started, and the cancelled one bought it nothing"
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

/// §3.2 "`Preferred`", end to end over the exit gate's shape — a private path
/// that ACCEPTS and carries nothing, sends as `Wallet::broadcast_one` makes
/// them (a fresh client per send, the wallet's own posture and counters): the
/// host sees TWO events, not-carrying at the deadline — before any further
/// dial, zero clearnet dials so far — then fell-back AT the clearnet dial,
/// which is the one FR-34's announcement keys on. Once latched the predicate
/// stays true and precedence hides it: never the new value again, and no
/// further event however many sends fail.
#[tokio::test]
async fn preferred_reports_not_carrying_then_the_announced_switch_as_two_events_in_that_order() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    // A live clearnet listener, so the fallback leg CONNECTS (the line reads
    // `connected`) and the latch is set at a dial that was made.
    let (endpoint, _listener) = accept_and_hang_up_loopback("https").await;
    let w = wallet_over(
        dir.path(),
        &vault,
        preferred_over(Arc::new(SilentPeer)),
        endpoint,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());

    tokio::time::pause();
    hold_the_paused_clock_to_small_steps();
    assert!(
        w.broadcast_one(raw_tx()).await.is_err(),
        "fixture: the first send sits out the silent path"
    );
    let first_failure = Instant::now();
    assert_eq!(
        dial_lines(&sink),
        [line("host", "broadcast", "connected")],
        "fixture: accepted, and carried nothing"
    );
    expect_silence(&mut events, Duration::from_secs(10), "inside the minute").await;
    assert!(w.broadcast_one(raw_tx()).await.is_err());
    expect_silence(
        &mut events,
        Duration::from_secs(10),
        "still inside the minute",
    )
    .await;
    // The third send is in flight ACROSS the deadline: nothing dials there.
    assert!(w.broadcast_one(raw_tx()).await.is_err());

    let (at, state) = expect_event(
        &mut events,
        Duration::from_secs(1),
        "the not-carrying transition, delivered on the send that observed the minute",
    )
    .await;
    assert_eq!(state, unanswered_dialer());
    let after = at.duration_since(first_failure);
    assert!(
        after >= Duration::from_secs(TOR_PATIENCE_SECS)
            && after < Duration::from_secs(TOR_PATIENCE_SECS + GRPC_UNARY_TIMEOUT_SECS + 1),
        "{after:?} after the failure was first observed — NEVER before the minute, and \
         within one RPC budget of it. The value turns on a failure OBSERVED past the \
         window, so on a real path it lands on the first attempt at or after the minute \
         rather than on a timer: a wallet that stopped trying is exactly the case it must \
         not fire for"
    );
    let lines = dial_lines(&sink);
    assert_eq!(
        lines,
        vec![line("host", "broadcast", "connected"); 3],
        "three sends, three private dials, none at the deadline and none in the clear: the \
         value came BEFORE the switch, not from it"
    );
    assert_eq!(
        w.tor_state(),
        unanswered_dialer(),
        "the snapshot between the two events"
    );

    // The next send leaves: the SECOND event, at the clearnet dial.
    let _ = w.broadcast_one(raw_tx()).await;
    let (_, state) = expect_event(
        &mut events,
        Duration::from_secs(1),
        "the announced switch is delivered",
    )
    .await;
    assert_eq!(
        state,
        TorState::FellBack,
        "not-carrying, THEN fell-back: FR-34's announcement keys on this one"
    );
    let lines = dial_lines(&sink);
    assert_eq!(
        lines.iter().filter(|l| l.0 == "sdk_direct").count(),
        1,
        "exactly one clearnet dial at the switch: {lines:?}"
    );
    assert_eq!(
        lines.last(),
        Some(&line("sdk_direct", "broadcast", "connected")),
        "{lines:?}"
    );
    assert_eq!(w.tor_state(), TorState::FellBack);

    // Latched: further failing sends, minutes on, deliver nothing more, and
    // the state never reads the new value again while latched.
    for _ in 0..3 {
        let _ = w.broadcast_one(raw_tx()).await;
        expect_silence(
            &mut events,
            Duration::from_secs(TOR_PATIENCE_SECS),
            "latched: no further event, and never the new value again",
        )
        .await;
        assert_eq!(w.tor_state(), TorState::FellBack);
    }
    tokio::time::resume();
    w.close().await.expect("close");
}

/// §3.2 "`Required`, two cases", the accepted-and-silent case end to end, and
/// §3.2 Q2's proof: sharing the wallet posture into `Required`'s plan lets the
/// window spend and the value fire at the minute, and STILL nothing latches
/// and no clearnet packet leaves, however long it goes on. One event for the
/// transition — a wedged path re-observed on every send is not re-announced.
#[tokio::test]
async fn required_reports_not_carrying_at_the_minute_and_never_the_switch() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());
    let posture = Arc::clone(w.tor_posture_for_test());

    tokio::time::pause();
    assert!(w.broadcast_one(raw_tx()).await.is_err());
    let first_failure = Instant::now();
    expect_silence(&mut events, Duration::from_secs(10), "inside the minute").await;
    assert!(w.broadcast_one(raw_tx()).await.is_err());
    expect_silence(
        &mut events,
        Duration::from_secs(10),
        "still inside the minute",
    )
    .await;
    assert!(w.broadcast_one(raw_tx()).await.is_err());

    let (at, state) = expect_event(
        &mut events,
        Duration::from_secs(1),
        "under Required the not-carrying transition is delivered at the minute",
    )
    .await;
    assert_eq!(state, unanswered_dialer());
    let after = at.duration_since(first_failure);
    assert!(
        after >= Duration::from_secs(TOR_PATIENCE_SECS)
            && after < Duration::from_secs(TOR_PATIENCE_SECS + GRPC_UNARY_TIMEOUT_SECS + 1),
        "{after:?} after the first observed failure — never before the minute, and within \
         one RPC budget of it: the value turns on the attempt that OBSERVES the window \
         passed, which under `Required` is the next send"
    );
    assert_eq!(
        dial_lines(&sink),
        vec![line("host", "broadcast", "connected"); 3],
        "three accepted private dials, nothing in the clear"
    );
    assert_eq!(w.tor_state(), unanswered_dialer());

    // Minutes more of the same: the value holds, nothing is re-announced,
    // nothing leaves, nothing latches.
    for send in 1..=5 {
        assert!(w.broadcast_one(raw_tx()).await.is_err());
        expect_silence(
            &mut events,
            Duration::from_secs(30),
            "one transition, one event: a wedged path re-observed on a send is not \
             re-announced",
        )
        .await;
        assert!(
            !posture.fell_back() && dial_lines(&sink).iter().all(|l| l.0 == "host"),
            "Required latched or dialled clearnet on send {send} after the minute: {:?}",
            dial_lines(&sink)
        );
        assert_eq!(w.tor_state(), unanswered_dialer());
    }
    tokio::time::resume();
    w.close().await.expect("close");
}

/// §3.2 "Delivery, honestly": `Instant` runs while the app is backgrounded
/// and the sink is paused then, so "announced once" cannot mean "delivered
/// once". One-delivered vs one-lost: a host that paused after the first
/// emission is not driven again (the pump stopped when the sink said so), the
/// transition it missed is in the SNAPSHOT it takes on resume, and the stream
/// it re-opens afterwards starts with that state (snapshot THEN re-subscribe).
#[tokio::test]
async fn the_snapshot_after_a_paused_sink_reads_the_transition_the_sink_missed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    // The host pauses right after the current state lands.
    let mut paused = subscribe_until_paused(&w, 1);
    assert_eq!(current_first(&mut paused).await, active_dialer());

    tokio::time::pause();
    a_minute_of_trying(w.tor_posture_for_test()).await;
    match next_within(&mut paused, Duration::from_secs(1)).await {
        Next::Ended => {}
        other => panic!("a sink that reported closed is not driven again: {other:?}"),
    }
    assert_eq!(
        w.tor_state(),
        unanswered_dialer(),
        "the transition the paused host missed is in the snapshot it takes on resume — \
         ONE-LOST is the honest reading of the paused sink, and this is where it is found"
    );
    let mut resumed = subscribe(&w);
    assert_eq!(
        current_first(&mut resumed).await,
        unanswered_dialer(),
        "the re-opened stream starts with the missed state"
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

/// §3.2 "The seam" / §3.1's fixture trap, through the wallet: the test-only
/// forced minute moves `may_switch_to_direct` and nothing else, so it must
/// not move the new value — through the snapshot or the stream — however
/// long it stands. (Its posture-level twin is
/// `the_forced_minute_does_not_move_unanswered_but_real_stamps_do`.)
#[tokio::test]
async fn the_forced_minute_does_not_move_the_new_value_through_the_wallet() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        preferred_over(Arc::new(FailingDialer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());
    w.tor_posture_for_test().spend_the_minute_for_test();
    assert!(
        w.tor_posture_for_test()
            .may_switch_to_direct(PathClass::Sync),
        "fixture: the switch predicate reads forced"
    );

    tokio::time::pause();
    expect_silence(
        &mut events,
        Duration::from_secs(3 * TOR_PATIENCE_SECS),
        "the forced flag is not evidence, so nothing is announced",
    )
    .await;
    assert_eq!(
        w.tor_state(),
        active_dialer(),
        "and the snapshot does not read it either"
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

/// NOT IN THE CONTRACT'S LIST (IT-1 +A). An `Off` wallet has no private path
/// to be unanswered about: its dials are clearnet and counted, its failures
/// are clearnet failures, and its state is `Off` however they go — no wake
/// is armed for it, and nothing is announced.
#[tokio::test]
async fn an_off_wallet_never_reads_the_new_value_however_its_clearnet_path_fails() {
    let (sink, _guard) = capture();
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let endpoint = {
        let port = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind loopback");
            listener.local_addr().expect("addr").port()
        };
        LightServerEndpoint::new(format!("http://127.0.0.1:{port}")).expect("loopback endpoint")
    };
    let w = wallet_over(dir.path(), &vault, TorPolicy::Off, endpoint).await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, TorState::Off);

    tokio::time::pause();
    hold_the_paused_clock_to_small_steps();
    for send in 1..=4 {
        assert!(w.broadcast_one(raw_tx()).await.is_err());
        expect_silence(
            &mut events,
            Duration::from_secs(30),
            "Off announces no private-path state",
        )
        .await;
        assert_eq!(w.tor_state(), TorState::Off, "after send {send}");
    }
    let lines = dial_lines(&sink);
    assert_eq!(lines.len(), 4, "{lines:?}");
    assert!(
        lines
            .iter()
            .all(|l| l.0 == "sdk_direct" && l.1 == "broadcast"),
        "every Off dial is a clearnet one: {lines:?}"
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

/// NOT IN THE CONTRACT'S LIST (IT-1 +A), from §3.2a's "the same payload as
/// `Active`": over a registered host transport the value names the host's
/// transport exactly as `Active` does — name, isolation, exposure — so a host
/// renders the same transport under a different sentence; and the host's own
/// declaration outranks it at the next read, both ways.
#[tokio::test]
async fn unanswered_carries_the_registered_transports_name_like_active_and_yields_to_its_declaration()
 {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let host = Arc::new(ScriptedHostDialer::ready());
    let w = wallet_over(
        dir.path(),
        &vault,
        TorPolicy::Required {
            runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
        },
        dead_https_loopback().await,
    )
    .await;
    let tor = |state: fn(TorRuntimeKind) -> TorState| {
        state(TorRuntimeKind::HostDialer {
            name: name("Tor"),
            isolation: IsolationSupport::Supported,
            exposure: TransportExposure::Hidden,
        })
    };
    let mut events = subscribe(&w);
    assert_eq!(
        current_first(&mut events).await,
        tor(|runtime| TorState::Active { runtime })
    );

    tokio::time::pause();
    a_minute_of_trying(w.tor_posture_for_test()).await;
    let (_, state) = expect_event(&mut events, Duration::from_secs(5), "delivered").await;
    assert_eq!(
        state,
        tor(|runtime| TorState::Unanswered { runtime }),
        "the payload Active carries: the host's name, isolation and exposure"
    );
    host.set_descriptor(Some(HostTransportDescriptor {
        name: name("Tor"),
        readiness: HOST_TRANSPORT_READY,
        isolation: IsolationSupport::Supported,
        exposure: TransportExposure::Hidden,
        health: TransportHealth::Failed,
    }));
    assert_eq!(
        w.tor_state(),
        TorState::Unavailable {
            transport: Some(name("Tor"))
        },
        "the host declared its transport FAILED: its claim outranks ours"
    );
    host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
    assert_eq!(
        w.tor_state(),
        tor(|runtime| TorState::Unanswered { runtime }),
        "ready again, and still nothing has come back: the value is back"
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

// ── §3.2: the counters (FR-37) ───────────────────────────────────────────────

/// §3.2 "The counters": a connection is one `wallet.dial` line, counted by arm
/// × outcome for the wallet that dialled, and the number and the log line
/// cannot disagree — the counters equal the tally of the capture, for `Off`
/// (every dial clearnet, and counted), `Required` over a registered host
/// (all seven private outcomes, on a scripted descriptor: six on a fresh
/// broadcast client per send, `retired` through the cached sync client — the
/// one outcome a fresh client cannot reach, see the segment) and `Preferred`
/// (a refusal and its clearnet fallback, on one send). Three wallets in one
/// process, each scoped to its own lines; a second wallet starts at zero; and
/// the read is not `require_open`-gated, so a teardown-time read is honest.
#[tokio::test]
async fn dial_counts_equal_the_dial_lines_per_arm_and_outcome_for_off_preferred_and_required() {
    let (sink, _guard) = capture();
    let vault = test_vault();

    // OFF: every dial is a clearnet one, and every one is counted.
    let (live_http, _listener) = accept_and_hang_up_loopback("http").await;
    let dir_a = tempfile::tempdir().expect("tempdir");
    let a = wallet_over(dir_a.path(), &vault, TorPolicy::Off, live_http).await;
    assert_eq!(
        a.dial_counts(),
        DialCounts::default(),
        "nothing dialled yet"
    );
    for _ in 0..2 {
        let _ = a.broadcast_one(raw_tx()).await;
    }
    let _ = a.inner.sync.once().await;
    let a_lines = dial_lines(&sink);
    let a_end = a_lines.len();
    assert!(
        a_end >= 3
            && a_lines
                .iter()
                .all(|l| l.0 == "sdk_direct" && l.2 == "connected"),
        "fixture: two sends and a pass over a live clearnet listener: {a_lines:?}"
    );
    assert_eq!(
        a.dial_counts(),
        tally(&a_lines),
        "Off: every clearnet dial, by outcome"
    );

    // REQUIRED over a registered host: five private refusals and an accept on
    // a fresh broadcast client per send, each instant, each its own line.
    let host = Arc::new(ScriptedHostDialer::ready());
    let dir_b = tempfile::tempdir().expect("tempdir");
    let b = wallet_over(
        dir_b.path(),
        &vault,
        TorPolicy::Required {
            runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
        },
        dead_https_loopback().await,
    )
    .await;
    assert_eq!(
        b.dial_counts(),
        DialCounts::default(),
        "a second wallet in the same process starts at zero"
    );
    let described = |readiness: u8, health: TransportHealth| HostTransportDescriptor {
        name: name("Tor"),
        readiness,
        isolation: IsolationSupport::Supported,
        exposure: TransportExposure::Hidden,
        health,
    };
    let steps: [(
        Option<HostTransportDescriptor>,
        Result<(), HostDialCode>,
        &str,
    ); 6] = [
        (
            Some(described(40, TransportHealth::Starting)),
            Ok(()),
            "not_ready",
        ),
        (
            Some(described(100, TransportHealth::Failed)),
            Ok(()),
            "transport_failed",
        ),
        (
            Some(ScriptedHostDialer::tor_ready()),
            Err(HostDialCode::Unreachable),
            "unreachable",
        ),
        (
            Some(ScriptedHostDialer::tor_ready()),
            Err(HostDialCode::Refused),
            "unsupported",
        ),
        (
            Some(ScriptedHostDialer::tor_ready()),
            Err(HostDialCode::Timeout),
            "timeout",
        ),
        (Some(ScriptedHostDialer::tor_ready()), Ok(()), "connected"),
    ];
    for (descriptor, outcome, expected) in steps {
        host.set_descriptor(descriptor);
        host.set_outcome(outcome);
        let before = dial_lines(&sink).len();
        let _ = b.broadcast_one(raw_tx()).await;
        let lines = dial_lines(&sink);
        assert_eq!(
            lines[before..],
            [line("host", "broadcast", expected)],
            "fixture: one private dial reading `{expected}`: {lines:?}"
        );
    }
    // `retired` is the one outcome a FRESH client cannot reach: a cleared
    // registry is refused at the door (`config::validate_transport`, FR-29
    // §6.1 E1) before any dial, so a send over an empty registry writes no
    // line. The readiness gate's `Retired` arm is for a client that already
    // PASSED the door — the cached sync client, built on a ready descriptor,
    // whose failed RPC retires its connection so the next pass dials afresh,
    // into a registry cleared meanwhile.
    host.set_descriptor(Some(ScriptedHostDialer::tor_ready()));
    host.set_outcome(Ok(()));
    let before = dial_lines(&sink).len();
    let _ = b.inner.sync.once().await;
    assert_eq!(
        dial_lines(&sink)[before..],
        [line("host", "sync", "connected")],
        "fixture: the sync client passed the door and its accepted dial carried nothing: {:?}",
        dial_lines(&sink)
    );
    host.set_descriptor(None);
    let before = dial_lines(&sink).len();
    let _ = b.inner.sync.once().await;
    assert_eq!(
        dial_lines(&sink)[before..],
        [line("host", "sync", "retired")],
        "fixture: the next pass dialled afresh into a cleared registry: {:?}",
        dial_lines(&sink)
    );
    let all = dial_lines(&sink);
    let b_end = all.len();
    assert_eq!(
        b.dial_counts(),
        tally(&all[a_end..b_end]),
        "Required: every private dial, by outcome — all seven, across both drivers"
    );
    assert_eq!(
        a.dial_counts(),
        tally(&all[..a_end]),
        "wallet A's counts did not move for wallet B's dials: per wallet, not per process"
    );

    // PREFERRED: the private refusal AND the clearnet fallback on one send,
    // both counted, each under its arm.
    let dir_c = tempfile::tempdir().expect("tempdir");
    let c = wallet_over(
        dir_c.path(),
        &vault,
        preferred_over(Arc::new(FailingDialer)),
        dead_https_loopback().await,
    )
    .await;
    c.tor_posture_for_test().spend_the_minute_for_test();
    for _ in 0..2 {
        let _ = c.broadcast_one(raw_tx()).await;
    }
    let all = dial_lines(&sink);
    let c_lines = &all[b_end..];
    assert_eq!(
        c_lines,
        [
            line("host", "broadcast", "unreachable"),
            line("sdk_direct", "broadcast", "io"),
            line("host", "broadcast", "unreachable"),
            line("sdk_direct", "broadcast", "io"),
        ],
        "fixture: a refusal and a fallback per send"
    );
    assert_eq!(
        c.dial_counts(),
        tally(c_lines),
        "Preferred: both arms, by outcome"
    );
    assert_eq!(b.dial_counts(), tally(&all[a_end..b_end]));
    assert_eq!(a.dial_counts(), tally(&all[..a_end]));

    // Not `require_open`-gated (like `tor_state`): a teardown-time read
    // carries nothing sensitive and is honest.
    c.inner
        .lifecycle
        .transition(LifecyclePhase::Closing)
        .expect("→ Closing");
    assert_eq!(
        c.dial_counts(),
        tally(c_lines),
        "readable while Closing, and unchanged"
    );
    drop(c);
    a.close().await.expect("close");
    b.close().await.expect("close");
}

/// §3.2 "a second wallet in the same process starts at zero and unproven":
/// wallet A's private path has not carried for the minute — the value is
/// A's; wallet B, opened beside it, reads `Active` (nothing has said
/// otherwise TO IT), its stream carries nothing of A's, its counts are zero,
/// and its own dial moves its own counts and not A's. The posture and the
/// counters are per wallet (`Inner`), never per process.
#[tokio::test]
async fn a_second_wallet_in_the_same_process_starts_at_zero_and_unproven() {
    let (sink, _guard) = capture();
    let vault = test_vault();
    let dir_a = tempfile::tempdir().expect("tempdir");
    let a = wallet_over(
        dir_a.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let dir_b = tempfile::tempdir().expect("tempdir");
    let b = wallet_over(
        dir_b.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let mut b_events = subscribe(&b);
    assert_eq!(current_first(&mut b_events).await, active_dialer());

    tokio::time::pause();
    assert!(a.broadcast_one(raw_tx()).await.is_err());
    assert_eq!(
        dial_lines(&sink).len(),
        1,
        "fixture: A's one accepted private dial"
    );
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS + 1)).await;
    // A tries again a minute on and the path still carries nothing: the run
    // has now been SEEN across the window, which is what the value claims. One
    // send and an idle minute is a wallet that stopped trying.
    assert!(a.broadcast_one(raw_tx()).await.is_err());
    let a_end = dial_lines(&sink).len();
    assert_eq!(
        a_end, 2,
        "fixture: two accepted private dials, neither carrying"
    );
    assert_eq!(
        a.tor_state(),
        unanswered_dialer(),
        "fixture: A's private path has not carried for the minute"
    );

    assert_eq!(
        b.tor_state(),
        active_dialer(),
        "B has seen nothing: unproven, not unanswered — the posture is per wallet"
    );
    expect_silence(
        &mut b_events,
        Duration::from_secs(1),
        "B's stream carries nothing of A's",
    )
    .await;
    assert_eq!(b.dial_counts(), DialCounts::default(), "B starts at zero");

    assert!(b.broadcast_one(raw_tx()).await.is_err());
    let all = dial_lines(&sink);
    assert_eq!(
        all.len(),
        a_end + 1,
        "fixture: A's two dials and B's one: {all:?}"
    );
    assert_eq!(
        a.dial_counts(),
        tally(&all[..a_end]),
        "B's dial did not move A's counts"
    );
    assert_eq!(b.dial_counts(), tally(&all[a_end..]), "and moved B's");
    tokio::time::resume();
    a.close().await.expect("close");
    b.close().await.expect("close");
}

// ── planted after the adjudication (IT-1 +A; ruling §4's survivors) ─────────
//
// `docs/adjudication/s1-truth/ruling.md` §4: each row below is the ONE shape
// that sees a mutant the item's rows above survived — a sibling mechanism
// masked the mutated one in every fixture (a conjunction masks its own
// operands). One claim per row, named for it.

/// M03 — the FAILING conjunct's THRESHOLD, read cold. The waker's deadline is
/// computed from the stamps on its own, so a stream row that observes the
/// minute says nothing about what the predicate reads a second before it;
/// `tor_state()` at 59 s of a failing run asks the predicate itself. The
/// silence conjunct is long true here (ten idle minutes first), so the only
/// thing standing between the read and the value is the run's age: a young
/// run is not the value, however long the silence before it.
#[tokio::test]
async fn a_cold_read_inside_the_minute_of_a_failing_run_reads_active_however_long_the_silence() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;

    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(10 * TOR_PATIENCE_SECS)).await;
    w.tor_posture_for_test()
        .note_private_failure(PathClass::Sync);
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
    assert_eq!(
        w.tor_state(),
        active_dialer(),
        "59 s into a failing run over a path silent for ten minutes: the run is inside its \
         own minute, and a young run is not the value whatever the silence before it"
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

/// M10 — the recovery through a SIBLING class. `Sync` has failed past the
/// minute and the value is out; the first RPC that comes back over the
/// private path is a BROADCAST, a class with no run of its own to end. It is
/// still the path carrying, so the union silence moved and the state returns
/// to `Active` ON THE STREAM from that success's own wake — not at the next
/// unrelated wake, and not only on a cold read (none is taken here before
/// the event).
#[tokio::test]
async fn a_sibling_classes_success_after_unanswered_delivers_active_on_the_stream() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());
    let posture = Arc::clone(w.tor_posture_for_test());

    tokio::time::pause();
    a_minute_of_trying(&posture).await;
    let (_, state) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "fixture: the sync run reaches the minute",
    )
    .await;
    assert_eq!(state, unanswered_dialer(), "fixture: the value is out");

    posture.note_private_success(PathClass::Broadcast, DialArm::Private);
    let (_, back) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "a broadcast RPC came back over the private path while the sync run is open: the \
         success is the wake, and the listening host hears the recovery",
    )
    .await;
    assert_eq!(
        back,
        active_dialer(),
        "back to Active on a sibling class's evidence — the path carries"
    );
    tokio::time::resume();
    w.close().await.expect("close");
}

/// M32 — a SYNC-STATUS change is an input of the state, and the waker must
/// hear it: a `Required` pass over a dialer that REFUSES stalls
/// `TorUnavailable`, which reads `Unavailable`, and the host listening on the
/// stream is told — the stall lands as an EVENT, not only in the next
/// snapshot. The sync run is opened by hand BEFORE the pass so the refusal
/// inside it is no 0 → non-zero edge: the status's own arm is the only wake
/// that can deliver this one, and nothing else masks its absence.
#[tokio::test]
async fn a_sync_stall_is_delivered_on_the_stream_while_the_host_listens() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(FailingDialer)),
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(current_first(&mut events).await, active_dialer());
    w.tor_posture_for_test()
        .note_private_failure(PathClass::Sync);
    expect_silence(
        &mut events,
        Duration::from_millis(200),
        "fixture: a run opening inside the minute moves no state",
    )
    .await;

    assert!(
        w.inner.sync.once().await.is_err(),
        "fixture: the pass fails on a refused dial"
    );
    let (_, state) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "the stall the pass left is delivered to a host that is listening",
    )
    .await;
    assert_eq!(
        state,
        TorState::Unavailable { transport: None },
        "a refused private dial under Required: the fail-closed reading, as an event"
    );
    w.close().await.expect("close");
}

/// M33 — a host descriptor push has no wake of its own (ruling VERDICT row
/// 7: seen at the next derivation), so a host that subscribes AFTER the push
/// must be told the truth as of its call: the pre-subscribe refresh, never
/// the channel's last value. No cold read between the push and the
/// subscribe — that would refresh the channel and mask a missing refresh.
#[tokio::test]
async fn a_fresh_subscribe_after_a_silent_descriptor_push_starts_with_the_pushed_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let host = Arc::new(ScriptedHostDialer::ready());
    let w = wallet_over(
        dir.path(),
        &vault,
        TorPolicy::Required {
            runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
        },
        dead_https_loopback().await,
    )
    .await;
    let mut listening = subscribe(&w);
    assert_eq!(
        current_first(&mut listening).await,
        TorState::Active {
            runtime: TorRuntimeKind::HostDialer {
                name: name("Tor"),
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
            }
        },
        "fixture: the channel holds Active"
    );

    host.set_descriptor(Some(HostTransportDescriptor {
        name: name("Tor"),
        readiness: HOST_TRANSPORT_READY,
        isolation: IsolationSupport::Supported,
        exposure: TransportExposure::Hidden,
        health: TransportHealth::Failed,
    }));
    let mut fresh = subscribe(&w);
    assert_eq!(
        current_first(&mut fresh).await,
        TorState::Unavailable {
            transport: Some(name("Tor"))
        },
        "the host declared its transport FAILED before this subscribe: the first event is \
         the truth as of the call, not the value the channel last saw"
    );
    w.close().await.expect("close");
}

/// M34 — the cold read is ONE derivation with the stream: a `tor_state()`
/// that sees a state input move with no wake (the host's push) refreshes the
/// channel the pumps ride, so a host already listening hears it too — a
/// snapshot can never be fresher than the stream beside it. A fresh
/// subscribe would refresh on its own and say nothing about the cold read;
/// the observer here is the stream that was ALREADY open.
#[tokio::test]
async fn a_cold_read_after_a_silent_descriptor_push_moves_the_stream_a_host_is_listening_to() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let host = Arc::new(ScriptedHostDialer::ready());
    let w = wallet_over(
        dir.path(),
        &vault,
        TorPolicy::Required {
            runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
        },
        dead_https_loopback().await,
    )
    .await;
    let mut events = subscribe(&w);
    assert_eq!(
        current_first(&mut events).await,
        TorState::Active {
            runtime: TorRuntimeKind::HostDialer {
                name: name("Tor"),
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
            }
        },
        "fixture: the channel holds Active"
    );

    host.set_descriptor(Some(HostTransportDescriptor {
        name: name("Tor"),
        readiness: HOST_TRANSPORT_READY,
        isolation: IsolationSupport::Supported,
        exposure: TransportExposure::Hidden,
        health: TransportHealth::Failed,
    }));
    assert_eq!(
        w.tor_state(),
        TorState::Unavailable {
            transport: Some(name("Tor"))
        },
        "fixture: the cold read sees the push"
    );
    let (_, state) = expect_event(
        &mut events,
        Duration::from_secs(5),
        "the cold read refreshed the channel the pump rides: the listening host hears what \
         the snapshot saw",
    )
    .await;
    assert_eq!(
        state,
        TorState::Unavailable {
            transport: Some(name("Tor"))
        },
        "the stream and the snapshot agree about the same instant"
    );
    w.close().await.expect("close");
}

/// M28 — a sink that reports closed on a LATER emit (the first transition,
/// not the current-first) is not driven again: the pump's loop breaks and
/// drops it. `the_snapshot_after_a_paused_sink_…` closes on the FIRST
/// emission, which is the pump's `return` before the loop; this is the
/// loop's own break.
#[tokio::test]
async fn a_sink_that_closes_on_a_later_emit_is_not_driven_again() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    // The host pauses on the SECOND delivery.
    let mut paused = subscribe_until_paused(&w, 2);
    assert_eq!(current_first(&mut paused).await, active_dialer());
    let posture = Arc::clone(w.tor_posture_for_test());

    tokio::time::pause();
    a_minute_of_trying(&posture).await;
    let (_, state) = expect_event(
        &mut paused,
        Duration::from_secs(5),
        "fixture: the transition on which the sink reports closed",
    )
    .await;
    assert_eq!(
        state,
        unanswered_dialer(),
        "fixture: delivered, and the sink said closed"
    );

    // The next transition finds no sink: the pump ended on the closed report.
    posture.note_private_success(PathClass::Sync, DialArm::Private);
    match next_within(&mut paused, Duration::from_secs(1)).await {
        Next::Ended => {}
        other => panic!(
            "a sink that reported closed on a later emit is not driven again — the pump \
             breaks and drops it: {other:?}"
        ),
    }
    tokio::time::resume();
    w.close().await.expect("close");
}

// ── the surface around the state: the stream's lifetime and the log's lock ───

/// THE STREAM'S LIFETIME ACROSS A SERVER SWITCH. `watch_tor_state` promises a
/// host it "completes only when the host cancels the subscription or the
/// wallet is closed", and a host renders its transport chip from it — so a
/// switch that silently ended the stream would leave that chip frozen on the
/// OLD session's word while the new session did whatever it liked. That is an
/// over-claim of protection, the one direction this state exists to prevent.
///
/// Both halves, in one drive: the channel is `Carried` (like `incoming_tx`),
/// so the pre-switch subscription is still delivering afterwards; and the
/// rebuilt session RE-PUBLISHES into it, so what it delivers is the new
/// session's truth and not the value the old session left behind. The host's
/// FAILED declaration is made with no wake and no cold read before the
/// switch, so the channel still holds `Active` when the swap begins: whatever
/// arrives is the switch's own publish.
///
/// Watched against: the assembler's `tor_state_tx` replaced by a fresh
/// `watch::channel` (the carry undone — the stream ENDS); and, separately,
/// the assembler's `publish_tor_state_into` call removed (the stream lives
/// and says the old session's `Active` for ever).
#[tokio::test]
async fn the_state_stream_survives_a_server_switch_and_reports_the_new_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let host = Arc::new(ScriptedHostDialer::ready());
    let w = wallet_over(
        dir.path(),
        &vault,
        TorPolicy::Required {
            runtime: TorRuntime::HostDialer(Arc::clone(&host) as Arc<dyn HostDialer>),
        },
        dead_https_loopback().await,
    )
    .await;
    let mut listening = subscribe(&w);
    assert_eq!(
        current_first(&mut listening).await,
        TorState::Active {
            runtime: TorRuntimeKind::HostDialer {
                name: name("Tor"),
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
            }
        },
        "fixture: the channel holds Active"
    );

    host.set_descriptor(Some(HostTransportDescriptor {
        name: name("Tor"),
        readiness: HOST_TRANSPORT_READY,
        isolation: IsolationSupport::Supported,
        exposure: TransportExposure::Hidden,
        health: TransportHealth::Failed,
    }));
    let w = switched_via_honest_oracle(w, custom("https://mine.example:443"), Network::Test).await;

    let (_, state) = expect_event(
        &mut listening,
        Duration::from_secs(5),
        "a subscription taken before the server switch is still delivering after it",
    )
    .await;
    assert_eq!(
        state,
        TorState::Unavailable {
            transport: Some(name("Tor"))
        },
        "and what it delivers is the REBUILT session's truth — the host declared its \
         transport FAILED before the swap, and a carried channel that was never \
         re-published would still be saying Active"
    );

    // AND it is the rebuilt session's LIVE channel, not a farewell from the
    // old one: an un-carried channel that the assembler happened to publish
    // into before dropping it delivers exactly one event and then ends, which
    // is indistinguishable from the above. The host clears its declaration on
    // the NEW session and the same subscription must hear the recovery.
    host.set_descriptor(Some(HostTransportDescriptor {
        name: name("Tor"),
        readiness: HOST_TRANSPORT_READY,
        isolation: IsolationSupport::Supported,
        exposure: TransportExposure::Hidden,
        health: TransportHealth::Ready,
    }));
    assert_eq!(
        w.tor_state(),
        TorState::Active {
            runtime: TorRuntimeKind::HostDialer {
                name: name("Tor"),
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
            }
        },
        "fixture: the rebuilt session's cold read sees the cleared declaration"
    );
    let (_, back) = expect_event(
        &mut listening,
        Duration::from_secs(5),
        "the carried subscription rides the REBUILT session's own channel — a later \
         transition reaches it too",
    )
    .await;
    assert_eq!(
        back,
        TorState::Active {
            runtime: TorRuntimeKind::HostDialer {
                name: name("Tor"),
                isolation: IsolationSupport::Supported,
                exposure: TransportExposure::Hidden,
            }
        },
        "the recovery the new session derived"
    );
    w.close().await.expect("close");
}

/// THE LOG LINE AND THE CHANNEL'S WRITE LOCK. This SDK invites hosts to
/// install their own `tracing` layers (FR-35), and tokio runs
/// `send_if_modified`'s closure while holding the channel's WRITE LOCK — so a
/// WARN emitted from inside that closure runs host code under our lock. A
/// host layer that reacted to `wallet.private_path_unanswered` by reading the
/// transport state back (attaching it to a crash report is the obvious use)
/// would re-enter `publish_tor_state` one frame below itself and take the
/// same lock: a deadlock with no timeout, no diagnostic, and every later
/// reader queued behind it. The shipped `DeviceLogLayer` writes straight to
/// the platform sink, which is exactly why nobody would meet this until a
/// host added a layer of its own.
///
/// The probe runs the re-entrant `publish_tor_state` on ANOTHER THREAD and
/// waits two real seconds for it, deliberately: same-thread re-entry
/// deadlocks FOREVER, which is a hang rather than a verdict, while a second
/// thread detects exactly the same held lock and recovers either way. What is
/// asserted is the property — no user code runs inside the closure — not the
/// message.
///
/// Watched against: the `tracing::warn!` moved back inside
/// `send_if_modified`'s closure in `publish_tor_state_into`.
#[tokio::test]
async fn a_host_log_layer_that_reads_the_state_on_our_warn_does_not_deadlock() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tracing_subscriber::layer::SubscriberExt;

    /// The message of one event, so the layer reacts to OUR line only.
    struct Message(String);
    impl tracing::field::Visit for Message {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
    }

    /// A host log layer that reads the wallet back on our WARN.
    struct ReadsTheStateBack {
        inner: Weak<Inner>,
        ran: Arc<AtomicBool>,
        blocked: Arc<AtomicBool>,
    }
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for ReadsTheStateBack {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut message = Message(String::new());
            event.record(&mut message);
            if !message.0.contains("private_path_unanswered") {
                return;
            }
            self.ran.store(true, Ordering::SeqCst);
            let Some(inner) = self.inner.upgrade() else {
                return;
            };
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            // The host's own read, on its own thread. It has no subscriber of
            // its own (`set_default` is thread-local), so it cannot recurse.
            std::thread::spawn(move || {
                inner.publish_tor_state();
                let _ = done_tx.send(());
            });
            if done_rx.recv_timeout(Duration::from_secs(2)).is_err() {
                self.blocked.store(true, Ordering::SeqCst);
            }
        }
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let vault = test_vault();
    let w = wallet_over(
        dir.path(),
        &vault,
        required_over(Arc::new(SilentPeer)),
        dead_https_loopback().await,
    )
    .await;
    let ran = Arc::new(AtomicBool::new(false));
    let blocked = Arc::new(AtomicBool::new(false));
    force_wallet_callsites_enabled();
    let _guard =
        tracing::subscriber::set_default(tracing_subscriber::registry().with(ReadsTheStateBack {
            inner: Arc::downgrade(&w.inner),
            ran: Arc::clone(&ran),
            blocked: Arc::clone(&blocked),
        }));

    let posture = Arc::clone(w.tor_posture_for_test());
    tokio::time::pause();
    posture.note_private_failure(PathClass::Sync);
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS + 1)).await;
    // The run must be SEEN to have outlasted the minute, not merely to have
    // opened before it: `is_unanswered` reads the OBSERVED failure span
    // (`kept_failing_past_the_window`), so one failure plus an idle clock is
    // an abandoned run and correctly reads `Active`. This second failure is
    // what a wallet that is still trying actually does, and it is the whole
    // point of the conjunct. (The fixture stamped one failure and advanced the
    // clock until the review fold: it was right against the clock-only
    // predicate it was written on, and this test's SUBJECT — that a host log
    // layer reading the state back on our WARN does not deadlock — is
    // untouched either way.)
    posture.note_private_failure(PathClass::Sync);
    assert_eq!(
        w.tor_state(),
        unanswered_dialer(),
        "fixture: the minute is spent and the value is out"
    );
    tokio::time::resume();

    assert!(
        ran.load(Ordering::SeqCst),
        "anti-vacuity: the host's layer never saw wallet.private_path_unanswered, so this \
         row proved nothing about what runs under the lock"
    );
    assert!(
        !blocked.load(Ordering::SeqCst),
        "a host log layer that reads the transport state back on our WARN must not find \
         the state channel's write lock held — the closure compares and assigns and \
         nothing else, and the line is emitted after it returns"
    );
    w.close().await.expect("close");
}
