//! The IMPLEMENTER's own drives of the patience window's consumer (stage S1,
//! item `window`). Not the item's named tests — those are the test author's,
//! written blind against the contract. These exist because a mechanism that
//! was only read is a claim: each one below was run to find out whether the
//! thing built does what its doc says, and is kept so the next reader can run
//! it too. A child of `net/grpc.rs` so it can reach that file's private types.
//!
//! Two fixtures, and which is which matters:
//! - [`SilentPeer`] ACCEPTS and never answers — the blackhole. A dial over it
//!   succeeds; only an RPC learns that nothing came back.
//! - the REAL clearnet dialer aimed at a loopback port nothing listens on, which
//!   is what `resolve_dialer` hard-wires as the `Preferred` fallback. Under a
//!   paused clock its outcome may read `io` or `timeout`; the ARM is what is
//!   asserted, never the outcome.

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tracing_subscriber::layer::SubscriberExt;

use super::*;
use crate::config::TorRuntime;
use crate::constants::{
    BROADCAST_ISOLATION_KEY_PREFIX, FALLBACK_ESTABLISH_BUDGET_SECS, TOR_PATIENCE_SECS,
    WALLET_SYNC_ISOLATION_KEY,
};
use crate::error::DialError;
use crate::net::dialer::{Dialed, PolicyDialer};
use crate::net::host_dialer::testing::{ScriptedHostDialer, name};
use crate::net::host_dialer::{
    HostDialer, HostTransportDescriptor, IsolationSupport, TransportExposure, TransportHealth,
};
use crate::ports::NetDialer;
use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};

/// Accepts every dial and never answers: the peer end is drained into the void
/// and nothing is ever written back. Counts its dials.
#[derive(Default)]
struct SilentPeer {
    dials: AtomicUsize,
}

impl SilentPeer {
    fn dials(&self) -> usize {
        self.dials.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl NetDialer for SilentPeer {
    async fn dial(
        &self,
        _host: &str,
        _port: u16,
        _isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        use tokio::io::AsyncReadExt;
        self.dials.fetch_add(1, Ordering::Relaxed);
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

/// A fallback that takes `delay` to connect, then hands back a silent stream.
struct SlowFallback {
    delay: Duration,
    dials: AtomicUsize,
}

#[async_trait]
impl NetDialer for SlowFallback {
    async fn dial(
        &self,
        host: &str,
        port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, DialError> {
        self.dials.fetch_add(1, Ordering::Relaxed);
        tokio::time::sleep(self.delay).await;
        SilentPeer::default().dial(host, port, isolation_key).await
    }
}

/// The `dial_arm` of every `wallet.dial` line captured so far, in order.
fn arms(sink: &CapturedEvents) -> Vec<String> {
    sink.records_of("wallet.dial")
        .into_iter()
        .filter_map(|fields| {
            fields
                .into_iter()
                .find(|(name, _)| name == "dial_arm")
                .map(|(_, value)| value)
        })
        .collect()
}

fn clearnet_lines(sink: &CapturedEvents) -> usize {
    arms(sink).iter().filter(|arm| *arm == "sdk_direct").count()
}

/// A loopback endpoint nothing listens on (bound, read, dropped).
fn dead_loopback_endpoint() -> LightServerEndpoint {
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener.local_addr().expect("addr").port()
    };
    LightServerEndpoint::new(format!("http://127.0.0.1:{port}")).expect("loopback endpoint")
}

fn capture() -> (CapturedEvents, tracing::subscriber::DefaultGuard) {
    force_wallet_callsites_enabled();
    let sink = CapturedEvents::default();
    let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
    (sink.clone(), tracing::subscriber::set_default(subscriber))
}

/// A client over a hand-built plan dialer — what `connect_unchecked` does,
/// minus `resolve_dialer`, whose `Preferred` fallback is hard-wired to the real
/// clearnet dialer.
fn client_over(
    dialer: Arc<dyn PlanDialer>,
    posture: &Arc<TorPosture>,
    isolation_key: &str,
) -> LightwalletdClient {
    let recipe = ChannelRecipe {
        uri: Uri::from_static("http://localhost:9067"),
        connector: DialerConnector {
            dialer,
            tls: None,
            host: "localhost".into(),
            port: 9067,
            isolation_key: Some(isolation_key.to_owned()),
            connection: Arc::default(),
        },
        auth: AuthInterceptor { auth: None },
    };
    let (client, connection) = recipe.open();
    LightwalletdClient {
        client,
        stall_on_failure: StallReason::EndpointUnreachable,
        witness: Some(PrivateRpcWitness {
            posture: Arc::clone(posture),
            class: PathClass::of(Some(isolation_key)),
            connection,
        }),
        recipe,
    }
}

/// THE CONSUMER GAP, both halves, through ONE reused client: a primary that
/// accepts and never answers is left at the minute, by the client's own next
/// RPC — no fresh client, no hand-called `dial()`.
#[tokio::test(start_paused = true)]
async fn impl_probe_one_reused_client_leaves_a_silent_primary_at_the_minute() {
    let (sink, _guard) = capture();
    let silent = Arc::new(SilentPeer::default());
    let policy = TorPolicy::Preferred {
        runtime: TorRuntime::Dialer(Arc::clone(&silent) as Arc<dyn NetDialer>),
    };
    let posture = Arc::new(TorPosture::new());
    let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
        &dead_loopback_endpoint(),
        &policy,
        Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
        &posture,
        None,
    )
    .expect("lazy connect never blocks");

    let start = tokio::time::Instant::now();
    // t = 0, 30, 60: three RPCs, each a full unary timeout over the private arm.
    // The run starts at the FIRST timeout (t = 30), so none of these may leave.
    for pass in 0..3 {
        let r = client.get_latest_block().await;
        assert!(
            matches!(r, Err(GrpcError::Timeout { .. })),
            "pass {pass}: {r:?}"
        );
        assert_eq!(clearnet_lines(&sink), 0, "pass {pass}: inside the minute");
        assert!(!posture.fell_back(), "pass {pass}: nothing latched");
    }
    assert_eq!(start.elapsed(), Duration::from_secs(90));
    assert_eq!(
        silent.dials(),
        3,
        "each failed RPC retired its connection, so each pass dialled again"
    );
    assert!(
        posture.may_switch_to_direct(PathClass::Sync),
        "silent for 90 s and failing for 60: the window is spent, and nothing has dialled on it yet"
    );
    assert!(
        !posture.fell_back(),
        "the latch is set AT the clearnet dial, not at the deadline"
    );

    // t = 90: the same client's next RPC. The primary ACCEPTS again — and that
    // is no longer worth anything.
    let _ = client.get_latest_block().await;
    assert_eq!(
        arms(&sink),
        ["host", "host", "host", "host", "sdk_direct"],
        "one private line per pass, then exactly one clearnet line"
    );
    assert!(posture.fell_back(), "and the switch latched");
}

/// The MONEY shape: a fresh client per send is the one place a second dial
/// already happened — onto the same accepting primary, the same `Ok`.
#[tokio::test(start_paused = true)]
async fn impl_probe_a_blackholed_broadcast_class_switches_too() {
    let (sink, _guard) = capture();
    let silent = Arc::new(SilentPeer::default());
    let policy = TorPolicy::Preferred {
        runtime: TorRuntime::Dialer(Arc::clone(&silent) as Arc<dyn NetDialer>),
    };
    let posture = Arc::new(TorPosture::new());
    let endpoint = dead_loopback_endpoint();
    let send = |n: u32| {
        let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
            &endpoint,
            &policy,
            Some(format!("{BROADCAST_ISOLATION_KEY_PREFIX}-{n:08x}")),
            &posture,
            None,
        )
        .expect("lazy connect never blocks");
        async move { client.send_transaction(vec![0x05, 0x00, 0x00, 0x80]).await }
    };

    for n in 0..3 {
        let r = send(n).await;
        assert!(
            matches!(r, Err(GrpcError::Timeout { .. })),
            "send {n}: {r:?}"
        );
    }
    assert_eq!(clearnet_lines(&sink), 0, "three sends inside the minute");

    let _ = send(3).await;
    assert_eq!(arms(&sink), ["host", "host", "host", "host", "sdk_direct"]);
    assert!(posture.fell_back());
    // The arm names WHO dialled (`host` / `sdk_direct`), never whether
    // the dial was private: a host whose own transport goes direct used to
    // print `private` here (`tracing_guard::ALLOWLIST`, `dial_arm`).
}

/// `Required` over the same blackhole: no fallback, so never a clearnet line
/// and never a latch — however many RPCs fail. Since stage S1 `truth` (Q2)
/// its plan shares the wallet posture and so has the witness too: a private
/// connection that failed an RPC is RETIRED and the next RPC dials afresh
/// (one private dial per failed RPC, the `Preferred` shape before a switch),
/// where it used to ride the one wedged connection for the client's life. A
/// fresh dial is what lets a path that heals carry again.
#[tokio::test(start_paused = true)]
async fn impl_probe_required_never_dials_clearnet_over_the_same_blackhole() {
    let (sink, _guard) = capture();
    let silent = Arc::new(SilentPeer::default());
    let policy = TorPolicy::Required {
        runtime: TorRuntime::Dialer(Arc::clone(&silent) as Arc<dyn NetDialer>),
    };
    let posture = Arc::new(TorPosture::new());
    let mut client = LightwalletdClient::connect_plaintext_over_test_dialer(
        &dead_loopback_endpoint(),
        &policy,
        Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
        &posture,
        None,
    )
    .expect("lazy connect never blocks");

    for _ in 0..10 {
        let r = client.get_latest_block().await;
        assert!(matches!(r, Err(GrpcError::Timeout { .. })), "{r:?}");
    }
    assert_eq!(
        clearnet_lines(&sink),
        0,
        "300 s of silence and not one clearnet dial"
    );
    assert_eq!(
        silent.dials(),
        10,
        "one private dial per failed RPC — the retired connection is replaced, never reused"
    );
    assert!(!posture.fell_back());
    assert!(
        posture.is_unanswered(),
        "and the silence is what a fail-closed wallet's state now reports"
    );
}

/// NO REDIAL STORM. After the switch the predicate stays true for the session,
/// and RPCs go on FAILING (the fallback here is silent too): nothing dials.
#[tokio::test(start_paused = true)]
async fn impl_probe_nothing_redials_after_the_switch() {
    let primary = Arc::new(SilentPeer::default());
    let fallback = Arc::new(SilentPeer::default());
    let posture = Arc::new(TorPosture::new());
    let dialer = Arc::new(PolicyDialer::with_fallback(
        Arc::clone(&primary) as Arc<dyn NetDialer>,
        Arc::clone(&fallback) as Arc<dyn NetDialer>,
        Arc::clone(&posture),
        Default::default(),
    ));
    let mut client = client_over(dialer, &posture, WALLET_SYNC_ISOLATION_KEY);

    for _ in 0..4 {
        let _ = client.get_latest_block().await;
    }
    assert_eq!((primary.dials(), fallback.dials()), (4, 1), "the switch");
    assert!(posture.may_switch_to_direct(PathClass::Sync));

    for _ in 0..10 {
        let r = client.get_latest_block().await;
        assert!(matches!(r, Err(GrpcError::Timeout { .. })), "{r:?}");
    }
    assert_eq!(
        (primary.dials(), fallback.dials()),
        (4, 1),
        "ten failing passes over the clearnet connection and not one further dial"
    );
    assert!(
        posture.may_switch_to_direct(PathClass::Sync),
        "and the predicate is still true: a level would have redialled on every pass"
    );
}

/// Evidence goes to the connection it rode: of two connections of ONE class, a
/// completed RPC over the clearnet one restarts nothing, and one over the
/// private one is not dropped because its sibling fell back.
#[tokio::test(start_paused = true)]
async fn impl_probe_a_witness_reports_against_its_own_connection() {
    let posture = Arc::new(TorPosture::new());
    let witness_on = |arm: DialArm| {
        let connection = Arc::new(ConnectionState::default());
        connection.set_served_by(arm);
        PrivateRpcWitness {
            posture: Arc::clone(&posture),
            class: PathClass::Broadcast,
            connection,
        }
    };
    let private = witness_on(DialArm::Private);
    let clearnet = witness_on(DialArm::Clearnet);

    posture.note_private_failure(PathClass::Broadcast);
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS)).await;
    assert!(posture.may_switch_to_direct(PathClass::Broadcast));

    // The clearnet sibling carries; the private one is the LATEST dial of the
    // class (the interleaving one flag per class got wrong).
    clearnet.note_carried();
    assert!(
        posture.may_switch_to_direct(PathClass::Broadcast),
        "a clearnet RPC never restarts the private window"
    );
    clearnet.note_failed();
    assert!(!clearnet.connection.is_retired(), "and never retires");

    private.note_carried();
    assert!(
        !posture.may_switch_to_direct(PathClass::Broadcast),
        "a private RPC is not dropped because a sibling is on clearnet"
    );
    private.note_failed();
    assert!(
        private.connection.is_retired(),
        "a failed private connection is"
    );

    // No dial has completed: nobody's evidence.
    let undialled = PrivateRpcWitness {
        posture: Arc::clone(&posture),
        class: PathClass::Sync,
        connection: Arc::default(),
    };
    undialled.note_failed();
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS)).await;
    assert!(!posture.may_switch_to_direct(PathClass::Sync));
    assert!(!undialled.connection.is_retired());
}

/// A sync class that has been SEEN failing for the whole window: a failure, the
/// minute, another failure. What an accepted dial needs before it may be left.
async fn a_posture_seen_failing_past_the_window() -> Arc<TorPosture> {
    let posture = Arc::new(TorPosture::new());
    posture.note_private_failure(PathClass::Sync);
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS)).await;
    posture.note_private_failure(PathClass::Sync);
    posture
}

/// The clock alone never leaves an ACCEPTING path: one failure and a long idle
/// gap (an app backgrounded mid-RPC) spend the window, and the first dial after
/// it is still served privately. Only that connection's own failure sends the
/// NEXT dial to clearnet.
#[tokio::test(start_paused = true)]
async fn impl_probe_an_idle_gap_never_walks_an_accepting_path_to_clearnet() {
    let posture = Arc::new(TorPosture::new());
    let fallback = Arc::new(SilentPeer::default());
    let dialer = PolicyDialer::with_fallback(
        Arc::new(SilentPeer::default()) as Arc<dyn NetDialer>,
        Arc::clone(&fallback) as Arc<dyn NetDialer>,
        Arc::clone(&posture),
        Default::default(),
    );
    let dial = || dialer.dial_attributed("zec.example", 443, Some(WALLET_SYNC_ISOLATION_KEY));

    posture.note_rpc_failure(PathClass::Sync, DialArm::Private);
    tokio::time::advance(Duration::from_secs(10 * TOR_PATIENCE_SECS)).await;
    assert!(
        posture.may_switch_to_direct(PathClass::Sync),
        "the premise: by the clock this window is long spent"
    );

    let resumed = dial().await.expect("the private path accepts");
    assert_eq!(resumed.served_by, DialArm::Private, "and is USED");
    assert_eq!(fallback.dials(), 0);
    assert!(!posture.fell_back());

    // It was a blackhole after all: its first RPC fails, and THAT is evidence.
    posture.note_rpc_failure(PathClass::Sync, DialArm::Private);
    let next = dial().await.expect("the fallback accepts");
    assert_eq!(
        next.served_by,
        DialArm::Clearnet,
        "one RPC later, it leaves"
    );
    assert!(posture.fell_back());

    // …and had it carried instead, the run would simply have ended.
    let healthy = Arc::new(TorPosture::new());
    healthy.note_rpc_failure(PathClass::Sync, DialArm::Private);
    tokio::time::advance(Duration::from_secs(10 * TOR_PATIENCE_SECS)).await;
    healthy.note_private_success(PathClass::Sync, DialArm::Private);
    assert!(!healthy.may_switch_to_direct(PathClass::Sync));
}

/// The clearnet leg has ONE deadline, and it travels with the stream.
#[tokio::test(start_paused = true)]
async fn impl_probe_the_fallback_leg_is_held_to_one_budget() {
    let (sink, _guard) = capture();
    let switch_over = |delay: u64, posture: Arc<TorPosture>| {
        let fallback = Arc::new(SlowFallback {
            delay: Duration::from_secs(delay),
            dials: AtomicUsize::new(0),
        });
        let dialer = PolicyDialer::with_fallback(
            Arc::new(SilentPeer::default()) as Arc<dyn NetDialer>,
            Arc::clone(&fallback) as Arc<dyn NetDialer>,
            posture,
            Default::default(),
        );
        (dialer, fallback)
    };

    let (dialer, _) = switch_over(
        FALLBACK_ESTABLISH_BUDGET_SECS - 1,
        a_posture_seen_failing_past_the_window().await,
    );
    let switched_at = tokio::time::Instant::now();
    let Dialed {
        served_by,
        establish_by,
        ..
    } = dialer
        .dial_attributed("zec.example", 443, Some(WALLET_SYNC_ISOLATION_KEY))
        .await
        .expect("a fallback inside the budget connects");
    assert_eq!(served_by, DialArm::Clearnet);
    assert_eq!(
        establish_by,
        Some(switched_at + Duration::from_secs(FALLBACK_ESTABLISH_BUDGET_SECS)),
        "the deadline is fixed at the switch, so TLS gets what the connect left"
    );

    let (dialer, fallback) = switch_over(
        FALLBACK_ESTABLISH_BUDGET_SECS + 1,
        a_posture_seen_failing_past_the_window().await,
    );
    let before = clearnet_lines(&sink);
    let r = dialer
        .dial_attributed("zec.example", 443, Some(WALLET_SYNC_ISOLATION_KEY))
        .await;
    assert!(matches!(r, Err(DialError::Timeout)), "cut at the budget");
    assert_eq!(fallback.dials.load(Ordering::Relaxed), 1, "it WAS dialled");
    assert_eq!(
        clearnet_lines(&sink),
        before + 1,
        "and a clearnet dial cut short still leaves its line"
    );
}

/// TLS IS A LEG of that one budget, not a leg of its own: over `https`, a
/// clearnet connect that took 2 s leaves a wedged handshake 3 s, not 25.
#[tokio::test(start_paused = true)]
async fn impl_probe_a_fallback_tls_handshake_spends_what_the_connect_left() {
    let posture = a_posture_seen_failing_past_the_window().await;
    let mut connector = DialerConnector {
        dialer: Arc::new(PolicyDialer::with_fallback(
            Arc::new(SilentPeer::default()) as Arc<dyn NetDialer>,
            Arc::new(SlowFallback {
                delay: Duration::from_secs(2),
                dials: AtomicUsize::new(0),
            }) as Arc<dyn NetDialer>,
            Arc::clone(&posture),
            Default::default(),
        )),
        tls: Some(Arc::new(GrpcTls::new())),
        host: "example.com".into(),
        port: 443,
        isolation_key: Some(WALLET_SYNC_ISOLATION_KEY.to_owned()),
        connection: Arc::default(),
    };
    let started = tokio::time::Instant::now();
    let res = tower::Service::call(&mut connector, Uri::from_static("https://example.com")).await;
    let err = match res {
        Ok(_) => panic!("a silent peer never completes a handshake"),
        Err(e) => e,
    };
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert_eq!(
        started.elapsed(),
        Duration::from_secs(FALLBACK_ESTABLISH_BUDGET_SECS),
        "connect + handshake together are the budget"
    );
    assert_eq!(
        connector.connection.served_by(),
        Some(DialArm::Clearnet),
        "and the arm was recorded before the handshake, so the RPC that fails on it is attributed"
    );
}

/// A bootstrap never counts as failing; a transport its host declared FAILED
/// does — from the first refusal that says so, for the maintainer's minute.
#[tokio::test(start_paused = true)]
async fn impl_probe_a_declared_failure_spends_the_window_and_a_bootstrap_never_does() {
    let descriptor = |health| HostTransportDescriptor {
        name: name("Tor"),
        readiness: 40,
        isolation: IsolationSupport::Supported,
        exposure: TransportExposure::Hidden,
        health,
    };
    let host = Arc::new(ScriptedHostDialer::with(
        Some(descriptor(TransportHealth::Starting)),
        Ok(()),
    ));
    let fallback = Arc::new(SilentPeer::default());
    let posture = Arc::new(TorPosture::new());
    let dialer = PolicyDialer::with_fallback(
        crate::net::readiness_gate::gated(&(Arc::clone(&host) as Arc<dyn HostDialer>)),
        Arc::clone(&fallback) as Arc<dyn NetDialer>,
        Arc::clone(&posture),
        Default::default(),
    );
    let dial = || dialer.dial("zec.example", 443, Some(WALLET_SYNC_ISOLATION_KEY));

    for _ in 0..5 {
        assert!(matches!(dial().await, Err(DialError::NotReady)));
        tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS)).await;
    }
    assert_eq!(fallback.dials(), 0, "five windows of bootstrap");

    host.set_descriptor(Some(descriptor(TransportHealth::Failed)));
    assert!(matches!(dial().await, Err(DialError::TransportFailed)));
    assert_eq!(
        fallback.dials(),
        0,
        "the run starts at the first declared failure"
    );
    tokio::time::advance(Duration::from_secs(TOR_PATIENCE_SECS)).await;
    assert!(dial().await.is_ok(), "a minute of it switches");
    assert_eq!(fallback.dials(), 1);
    assert!(posture.fell_back());
    assert!(
        host.keys_seen().is_empty(),
        "and the host was never asked to dial"
    );
}
