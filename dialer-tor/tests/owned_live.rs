//! The owned client over the LIVE Tor network: a real stream outliving the
//! shutdown (`tor-plugin.md` §12.4's one premise not read from arti's
//! sources, and §12.8's `real_stream_after_shutdown_errors_never_panics`).
//! `#[ignore]` AND gated on `DIALER_TOR_LIVE=1`, as `exit_rotation_live.rs`:
//!
//! ```text
//! cd sdk/dialer-tor
//! DIALER_TOR_LIVE=1 ../../scripts/stage/cargo-slot.sh \
//!     cargo test --test owned_live -- --ignored --nocapture --test-threads=1
//! ```
//!
//! It proves: a stream open across `Stopped` errors on read and write and never
//! panics; a reader parked in it is woken; and dropping it AFTER `Stopped`
//! writes nothing under the state directory (so no reference from the stream
//! reaches arti's managers). State: a PERSISTENT directory under the cargo
//! target dir, so each run does not sample a new guard.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dialer_tor::{ClientLedger, OwnedOptions, Shutdown, TorClientConfig, TorDialer, arti_client};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const REAL_HOST: &str = "zec.rocks";
const REAL_PORT: u16 = 443;
const BUDGET: Duration = Duration::from_secs(10);
/// Long enough for any late write a dropped stream could cause to land.
const SETTLE: Duration = Duration::from_secs(3);

fn live_enabled() -> bool {
    if std::env::var("DIALER_TOR_LIVE").as_deref() == Ok("1") {
        return true;
    }
    eprintln!("SKIP: set DIALER_TOR_LIVE=1 to run the live Tor test");
    false
}

fn state_root() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("dialer-tor-live-state")
        .join("owned")
}

/// Every file under `dir`, with its size and modification time. Any read
/// error fails the test: a snapshot that skipped what it could not read
/// could compare two empty maps and pass.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (u64, SystemTime)> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d).expect("read a Tor state directory");
        for entry in entries {
            let entry = entry.expect("read a Tor state directory entry");
            let path = entry.path();
            let meta = entry.metadata().expect("read a Tor state file's metadata");
            if meta.is_dir() {
                stack.push(path);
            } else {
                let modified = meta.modified().expect("read a modification time");
                out.insert(path, (meta.len(), modified));
            }
        }
    }
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "touches the live Tor network; DIALER_TOR_LIVE=1"]
async fn real_stream_after_shutdown_errors_never_panics() {
    if !live_enabled() {
        return;
    }
    let root = state_root();
    std::fs::create_dir_all(&root).expect("create the persistent live state dir");
    let mut builder = arti_client::config::TorClientConfigBuilder::from_directories(
        root.join("state"),
        root.join("cache"),
    );
    builder.storage().permissions().dangerously_trust_everyone();
    let config: TorClientConfig = builder.build().expect("live config");
    let ledger = Arc::new(ClientLedger::default());
    let dialer = TorDialer::spawn_owned(config, &ledger, OwnedOptions::default())
        .await
        .expect("create the owned client");
    dialer
        .bootstrap()
        .await
        .expect("bootstrap over the live Tor network");
    let stream = dialer
        .connect(REAL_HOST, REAL_PORT, None)
        .await
        .expect("a stream to a real host");
    let (mut read_half, mut write_half) = tokio::io::split(stream);

    // A reader parked in the stream (the server waits for a TLS hello).
    let reader = tokio::spawn(async move {
        let mut buf = [0u8; 64];
        let result = read_half.read(&mut buf).await;
        (result, read_half)
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    // Still pending, so its poll returned `Pending` and parked its waker.
    assert!(
        !reader.is_finished(),
        "the reader was not parked: the server answered before the shutdown"
    );

    assert_eq!(dialer.shutdown(BUDGET).await, Shutdown::Stopped);
    assert_eq!((ledger.live(), ledger.stuck()), (0, 0));

    let (read, read_half) = tokio::time::timeout(Duration::from_secs(2), reader)
        .await
        .expect("the parked reader was woken")
        .expect("the reader did not panic");
    assert!(read.is_err(), "a read across Stopped must fail: {read:?}");
    assert!(
        write_half.write_all(b"x").await.is_err(),
        "a write after Stopped must fail"
    );

    // The whole Tor directory: `state/` (guards) and `cache/` (the directory
    // store, its SQLite file and WAL). arti's guard write is synchronous in
    // the circuit manager's drop, so a leaked reference shows at the drop
    // itself; SETTLE covers anything asynchronous.
    let before = snapshot(&root);
    // The snapshot sees the state a late write would change: the guard file
    // a bootstrapped client wrote, not an empty or wrong directory.
    assert!(
        before.iter().any(|(path, (len, _))| {
            path.file_name().is_some_and(|name| name == "guards.json") && *len > 0
        }),
        "no guards.json under the Tor directory: the snapshot proves nothing"
    );
    drop(write_half);
    drop(read_half); // the stream itself, dropped AFTER Stopped, on this thread
    tokio::time::sleep(SETTLE).await;
    let after = snapshot(&root);
    assert_eq!(
        before, after,
        "dropping a stream after Stopped wrote under the Tor directory"
    );
}
