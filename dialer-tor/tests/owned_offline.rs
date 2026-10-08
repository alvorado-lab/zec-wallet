//! The owned client over a REAL `arti_client::TorClient`, with no network: the
//! client is built unbootstrapped on its own runtime, used, and shut down
//! (`tor-plugin.md` §12.8). Runs in `just ci` like any other test.

use std::sync::Arc;
use std::time::Duration;

use dialer_tor::{
    ClientLedger, OwnedDialer, OwnedOptions, Shutdown, TorClientConfig, TorDialError, TorDialer,
    arti_client,
};
use tempfile::TempDir;

/// Generous for a loaded machine.
const BUDGET: Duration = Duration::from_secs(10);

/// Storage permissions relaxed for the TEST ONLY, as `offline_refusals.rs`
/// explains.
fn config(dirs: &TempDir) -> TorClientConfig {
    let mut builder = arti_client::config::TorClientConfigBuilder::from_directories(
        dirs.path().join("state"),
        dirs.path().join("cache"),
    );
    builder.storage().permissions().dangerously_trust_everyone();
    builder.build().expect("the test config must build")
}

async fn owned(dirs: &TempDir, ledger: &Arc<ClientLedger>) -> OwnedDialer {
    TorDialer::spawn_owned(config(dirs), ledger, OwnedOptions::default())
        .await
        .expect("an unbootstrapped owned client needs no network")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_real_client_is_counted_once_until_its_runtime_shut_down() {
    let dirs = TempDir::new().expect("tempdir");
    let ledger = Arc::new(ClientLedger::default());
    let dialer = owned(&dirs, &ledger).await;
    assert_eq!(ledger.live(), 1, "counted once while it exists");
    assert_eq!(dialer.shutdown(BUDGET).await, Shutdown::Stopped);
    assert_eq!((ledger.live(), ledger.stuck()), (0, 0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_owned_dial_before_bootstrap_refuses_through_the_owned_runtime() {
    let dirs = TempDir::new().expect("tempdir");
    let ledger = Arc::new(ClientLedger::default());
    let dialer = owned(&dirs, &ledger).await;
    let err = dialer
        .dial("example.com", 443, None)
        .await
        .err()
        .expect("an unbootstrapped client never returns a stream");
    assert!(
        matches!(err, TorDialError::NotBootstrapped { .. }),
        "{err:?}"
    );
    assert!(dialer.readiness().is_ok());
    assert_eq!(dialer.shutdown(BUDGET).await, Shutdown::Stopped);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_owned_verb_after_shutdown_answers_closed() {
    let dirs = TempDir::new().expect("tempdir");
    let ledger = Arc::new(ClientLedger::default());
    let dialer = owned(&dirs, &ledger).await;
    let clone = dialer.clone();
    assert_eq!(clone.shutdown(BUDGET).await, Shutdown::Stopped);
    assert!(dialer.is_closed(), "every clone is the same client");
    assert!(matches!(dialer.readiness(), Err(TorDialError::Closed)));
    assert!(matches!(
        dialer.set_dormant(dialer_tor::DormantMode::Soft),
        Err(TorDialError::Closed)
    ));
    assert!(matches!(dialer.dormant_mode(), Err(TorDialError::Closed)));
    assert!(matches!(
        dialer.bootstrap().await,
        Err(TorDialError::Closed)
    ));
    assert!(matches!(
        dialer.dial("example.com", 443, None).await,
        Err(TorDialError::Closed)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawn_owned_refuses_while_a_client_is_stuck() {
    let dirs = TempDir::new().expect("tempdir");
    let ledger = Arc::new(ClientLedger::default());
    ledger.enter().strand(); // a client whose shutdown overran
    let refused = TorDialer::spawn_owned(config(&dirs), &ledger, OwnedOptions::default()).await;
    assert!(
        matches!(refused, Err(TorDialError::RestartRequired)),
        "{refused:?}"
    );
    assert_eq!(ledger.live(), 1, "nothing new was counted");
    assert!(
        !dirs.path().join("state").exists(),
        "no client touched the state directory"
    );
}

#[test]
fn spawn_owned_works_from_a_current_thread_and_a_multi_thread_runtime() {
    for multi in [false, true] {
        let host = if multi {
            tokio::runtime::Builder::new_multi_thread()
        } else {
            tokio::runtime::Builder::new_current_thread()
        }
        .enable_all()
        .build()
        .expect("a host runtime");
        let dirs = TempDir::new().expect("tempdir");
        let ledger = Arc::new(ClientLedger::default());
        let outcome = host.block_on(async {
            let dialer = owned(&dirs, &ledger).await;
            dialer.shutdown(BUDGET).await
        });
        assert_eq!(outcome, Shutdown::Stopped, "multi-thread host: {multi}");
        assert_eq!(ledger.live(), 0);
    }
}

/// A bridge that accepts a connection and drops it at once, counting each.
struct DeadBridge {
    port: u16,
    accepts: Arc<std::sync::atomic::AtomicUsize>,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

impl DeadBridge {
    fn start() -> Self {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a local listener");
        listener.set_nonblocking(true).expect("non-blocking");
        let port = listener.local_addr().expect("its address").port();
        let accepts = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (counted, stopped) = (Arc::clone(&accepts), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stopped.load(SeqCst) {
                match listener.accept() {
                    Ok((conn, _)) => {
                        counted.fetch_add(1, SeqCst);
                        drop(conn);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(20)),
                }
            }
        });
        Self {
            port,
            accepts,
            stop,
        }
    }

    fn accepts(&self) -> usize {
        self.accepts.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for DeadBridge {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// How long the client may take to try the dead bridge twice: arti retries a
/// refused bridge within seconds.
const RETRIES_WITHIN: Duration = Duration::from_secs(60);
/// How many attempts the test sees before the shutdown, to measure the gap.
const RETRIES_SEEN: usize = 4;
/// How many of the client's own retry gaps the test watches after `Stopped`.
const GAPS_WATCHED: u32 = 3;
/// Ten of the listener's 20 ms accept polls: a connection the kernel queued
/// BEFORE `Stopped` is accepted, and counted, within this.
const BACKLOG_DRAIN: Duration = Duration::from_millis(200);

/// `Stopped` means no further connection attempt: a client bootstrapping
/// through a bridge that drops every connection retries it, until the
/// shutdown, and never after (`tor-plugin.md` §12.4). The client must retry
/// several times first, so the silence after `Stopped` is measured against
/// its longest observed retry gap, never vacuously.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_connection_attempt_follows_stopped() {
    let bridge = DeadBridge::start();
    let dirs = TempDir::new().expect("tempdir");
    let mut builder = arti_client::config::TorClientConfigBuilder::from_directories(
        dirs.path().join("state"),
        dirs.path().join("cache"),
    );
    builder.storage().permissions().dangerously_trust_everyone();
    let line = format!(
        "127.0.0.1:{} 316E643333645F6D79216558614D3931657A5F5F",
        bridge.port
    );
    builder.bridges().bridges().push(
        line.parse::<arti_client::config::BridgeConfigBuilder>()
            .expect("a vanilla bridge line"),
    );
    let config: TorClientConfig = builder.build().expect("the bridged config must build");
    let ledger = Arc::new(ClientLedger::default());
    let dialer = TorDialer::spawn_owned(config, &ledger, OwnedOptions::default())
        .await
        .expect("an owned client");
    let bootstrapping = dialer.clone();
    tokio::spawn(async move {
        let _ = bootstrapping.bootstrap().await;
    });

    // The longest gap between attempts seen, over several retries: arti
    // backs off, so the first gap alone could understate the next one.
    let started = std::time::Instant::now();
    let (mut seen, mut last_at, mut gap) = (0, None::<std::time::Instant>, Duration::ZERO);
    while seen < RETRIES_SEEN {
        let now = bridge.accepts();
        if now > seen {
            let at = std::time::Instant::now();
            if let Some(previous) = last_at {
                gap = gap.max(at - previous);
            }
            (seen, last_at) = (now, Some(at));
        }
        assert!(
            started.elapsed() < RETRIES_WITHIN,
            "the client did not retry the bridge: the test would prove nothing"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    assert_eq!(dialer.shutdown(BUDGET).await, Shutdown::Stopped);
    // An attempt made before `Stopped` may still sit in the listen backlog.
    tokio::time::sleep(BACKLOG_DRAIN).await;
    let at_stop = bridge.accepts();
    let watch = (gap * GAPS_WATCHED).max(Duration::from_secs(2));
    tokio::time::sleep(watch).await;
    assert_eq!(
        bridge.accepts(),
        at_stop,
        "a connection attempt followed Stopped (watched {watch:?}, retry gap {gap:?})"
    );
}

#[test]
fn the_last_drop_of_a_real_client_on_an_async_thread_shuts_it_down() {
    let host = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a host runtime");
    let dirs = TempDir::new().expect("tempdir");
    let ledger = Arc::new(ClientLedger::default());
    host.block_on(async {
        let dialer = owned(&dirs, &ledger).await;
        drop(dialer); // unawaited, on an async thread
    });
    let until = std::time::Instant::now() + BUDGET;
    while ledger.live() != 0 && std::time::Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!((ledger.live(), ledger.stuck()), (0, 0));
}
