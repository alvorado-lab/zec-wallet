//! FR-54 §4.4 #4 — the LIVE proof that a bad exit no longer holds a group's
//! circuit. **This crate's first networked test.** `#[ignore]` AND gated on
//! `DIALER_TOR_LIVE=1`, so neither `just ci` nor a bare `--ignored` run touches
//! the Tor network by accident:
//!
//! ```text
//! cd sdk/dialer-tor
//! DIALER_TOR_LIVE=1 ../../scripts/stage/cargo-slot.sh \
//!     cargo test --test exit_rotation_live -- --ignored --nocapture --test-threads=1
//! ```
//!
//! What it reads: ONLY the `tunnel_id` field of arti's
//! `debug!(tunnel_id = …, "Got a circuit for …")` (target `arti_client::client`,
//! emitted before `begin_stream`, so a FAILED dial shows it too), plus the
//! `kind` field of this crate's own "tor dial failed" line, plus this crate's
//! own "isolation group left a suspect circuit" events (counted). No other arti
//! output is recorded or printed — it names this machine's guard. Safe-logging
//! stays on (the destination in arti's message is scrubbed; the message is
//! matched by prefix and dropped).
//!
//! Runtime: each test pins `flavor = "current_thread"` — the capture is a
//! thread-local `set_default` subscriber, so arti's circuit events must be
//! emitted on the test's own thread to be seen.
//!
//! State: a dedicated PERSISTENT directory per (shape, polarity) under the
//! cargo target dir (`CARGO_TARGET_TMPDIR`) — never an app's, never a fresh
//! tempdir: each fresh one samples a new guard (m7).
//!
//! Shapes (both run under both polarities):
//! - A: five dials to a fresh random `*.invalid` (the exit cannot resolve it),
//!   then the real host — for `None` and for `Some(key)`.
//! - B, the outage's own: five dials to the RFC 5737 TEST-NET literal
//!   `192.0.2.1:443` (unroutable by design, never a third party's port; the
//!   exit times out or reports no route), then the real host. The kind is
//!   ASSERTED to be in the rotate-set and PRINTED for the run record (§8).
//!
//! Polarities — ASSERTED is what this crate does; arti's own circuit choices
//! are RECORDED (measured over four runs: arti sometimes leaves a failing
//! circuit by itself, on a keyed lane and once on nearly every dial, rotation
//! off included). Rotation ON (`ExitRotation::default()`): d1's failure
//! rotates; id(d2) != id(d1) (deterministic: a fresh token cannot join the old
//! token's circuit); no further rotation for any dial that started inside the
//! interval; the real host connects. OFF (`ExitRotation::off()`): no dial
//! rotates; whether arti held the circuit is recorded.
//!
//! The real host is the wallet's default lightwalletd endpoint
//! (`sdk/zec_wallet_ui/lib/features/wallet/wallet_config.dart`
//! `referenceMainnetEndpoint`), connect only, no byte sent.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use dialer_tor::{
    EXIT_ROTATION_INTERVAL, ErrorKind, ExitRotation, TorClientConfig, TorDialError, TorDialer,
    arti_client,
};
use tracing::field::{Field, Visit};
use tracing::subscriber::Interest;
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt as _};

const REAL_HOST: &str = "zec.rocks";
const REAL_PORT: u16 = 443;
const TEST_NET: &str = "192.0.2.1";
const DIALS: usize = 5;
/// Shape B reruns with a fresh key while an exit's POLICY rejects TEST-NET,
/// so the outage's own class (a timeout or no-route) is the one proven.
const SHAPE_B_POLICY_RERUNS: usize = 4;

const ARTI_TARGET: &str = "arti_client::client";
const OUR_TARGET_PREFIX: &str = "dialer_tor";

/// §4.1's rotate-set. Mirrored here only because the predicate is
/// crate-private; `src/error.rs`'s structural guard is the source of truth.
const ROTATE: [ErrorKind; 11] = [
    ErrorKind::RemoteHostResolutionFailed,
    ErrorKind::RemoteHostNotFound,
    ErrorKind::ExitTimeout,
    ErrorKind::RemoteNetworkTimeout,
    ErrorKind::RemoteNetworkFailed,
    ErrorKind::RemoteStreamError,
    ErrorKind::RemoteConnectionRefused,
    ErrorKind::RemoteStreamReset,
    ErrorKind::ExitPolicyRejected,
    ErrorKind::RelayTooBusy,
    ErrorKind::RemoteStreamClosed,
];
/// Kinds the run may see that are NOT in the rotate-set, so a captured
/// `kind` string can be named back.
const OTHERS: [ErrorKind; 5] = [
    ErrorKind::NoExit,
    ErrorKind::CircuitCollapse,
    ErrorKind::TorProtocolViolation,
    ErrorKind::TorNetworkTimeout,
    ErrorKind::LocalNetworkError,
];
/// The outage's own class (Relim ticket #1414): a timeout or no route at
/// the exit.
const OUTAGE_CLASS: [ErrorKind; 3] = [
    ErrorKind::ExitTimeout,
    ErrorKind::RemoteNetworkTimeout,
    ErrorKind::RemoteNetworkFailed,
];

fn live_enabled() -> bool {
    if std::env::var("DIALER_TOR_LIVE").as_deref() == Ok("1") {
        return true;
    }
    eprintln!("SKIP: set DIALER_TOR_LIVE=1 to run the live Tor test");
    false
}

// ── capture ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Captured {
    tunnel_ids: Vec<String>,
    /// Display of the arti `ErrorKind` on this crate's "tor dial failed" line.
    failure_kinds: Vec<String>,
    /// This crate's own rotation events ("isolation group left a suspect
    /// circuit") — what the CODE did, as opposed to arti's circuit choices.
    rotations: usize,
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Captured>>);

impl Capture {
    fn lock(&self) -> std::sync::MutexGuard<'_, Captured> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Default)]
struct Fields {
    message: Option<String>,
    tunnel_id: Option<String>,
    kind: Option<String>,
}

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_debug(field, &value);
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "message" => self.message = Some(format!("{value:?}")),
            "tunnel_id" => self.tunnel_id = Some(format!("{value:?}")),
            "kind" => self.kind = Some(format!("{value:?}")),
            _ => {}
        }
    }
}

fn wanted(meta: &Metadata<'_>) -> bool {
    meta.target() == ARTI_TARGET || meta.target().starts_with(OUR_TARGET_PREFIX)
}

impl<S: Subscriber> Layer<S> for Capture {
    fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
        if wanted(meta) {
            Interest::always()
        } else {
            Interest::never()
        }
    }
    fn enabled(&self, meta: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        wanted(meta)
    }
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut f = Fields::default();
        event.record(&mut f);
        let message = f.message.unwrap_or_default();
        if meta.target() == ARTI_TARGET {
            // ONLY the id; the message (destination scrubbed) is dropped.
            if message.starts_with("Got a circuit")
                && let Some(id) = f.tunnel_id
            {
                self.lock().tunnel_ids.push(id);
            }
        } else if message.contains("isolation group left a suspect circuit") {
            self.lock().rotations += 1;
        } else if message.contains("tor dial failed")
            && let Some(kind) = f.kind
        {
            self.lock().failure_kinds.push(kind);
        }
    }
}

// ── dialer ──────────────────────────────────────────────────────────────────

fn state_root(leg: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("dialer-tor-live-state")
        .join(leg)
}

/// A bootstrapped dialer over a PERSISTENT per-leg state dir. Storage
/// permissions are relaxed for the TEST ONLY (as `offline_refusals.rs` does):
/// the shared target dir's ancestors are not this test's to judge.
async fn live_dialer(leg: &str, policy: ExitRotation) -> TorDialer {
    let root = state_root(leg);
    std::fs::create_dir_all(&root).expect("create the persistent live state dir");
    let mut builder = arti_client::config::TorClientConfigBuilder::from_directories(
        root.join("state"),
        root.join("cache"),
    );
    builder.storage().permissions().dangerously_trust_everyone();
    let config: TorClientConfig = builder.build().expect("live config");
    let dialer = TorDialer::from_config(config)
        .await
        .expect("create the Tor client")
        .exit_rotation(policy);
    let t = Instant::now();
    dialer
        .bootstrap()
        .await
        .expect("bootstrap over the live Tor network");
    eprintln!("[{leg}] bootstrapped in {:?}", t.elapsed());
    dialer
}

/// One dial's observation: its circuit, its outcome, when it started.
struct Dial {
    tunnel_id: String,
    outcome: Result<(), TorDialError>,
    kind: Option<ErrorKind>,
    started: Instant,
    /// Whether THIS crate rotated the group's token on this dial's failure.
    rotated: bool,
}

fn kind_named(display: &str) -> Option<ErrorKind> {
    ROTATE
        .iter()
        .chain(OTHERS.iter())
        .copied()
        .find(|k| k.to_string() == display.trim_matches('"'))
}

/// The arti kind of a failure: the captured `kind` field first (exact), the
/// variant second (unambiguous ones only).
fn kind_of(err: &TorDialError, captured: Option<&str>) -> Option<ErrorKind> {
    if let Some(k) = captured.and_then(kind_named) {
        return Some(k);
    }
    match err {
        TorDialError::ExitTimeout => Some(ErrorKind::ExitTimeout),
        TorDialError::RemoteNetworkTimeout => Some(ErrorKind::RemoteNetworkTimeout),
        TorDialError::Tor { kind } => Some(*kind),
        _ => None,
    }
}

async fn dial_once(cap: &Capture, d: &TorDialer, host: &str, port: u16, key: Option<&str>) -> Dial {
    let (ids_before, kinds_before, rotations_before) = {
        let c = cap.lock();
        (c.tunnel_ids.len(), c.failure_kinds.len(), c.rotations)
    };
    let started = Instant::now();
    let outcome = d.connect(host, port, key).await.map(drop);
    let c = cap.lock();
    let new_ids = &c.tunnel_ids[ids_before..];
    let tunnel_id = new_ids
        .last()
        .cloned()
        .unwrap_or_else(|| panic!("no \"Got a circuit\" event for a dial: the capture is broken or the dial never reached a circuit ({outcome:?})"));
    let captured_kind = c.failure_kinds[kinds_before..].last().map(String::as_str);
    let kind = outcome
        .as_ref()
        .err()
        .and_then(|e| kind_of(e, captured_kind));
    let rotated = c.rotations > rotations_before;
    Dial {
        tunnel_id,
        outcome,
        kind,
        started,
        rotated,
    }
}

fn random_invalid() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("fr54-{nanos:x}-{}.invalid", std::process::id())
}

enum Run {
    Done { first_kind: Option<ErrorKind> },
    Skipped,
}

/// Five failing dials then the real host, on one group, asserting the
/// polarity's property.
async fn run_group(
    cap: &Capture,
    d: &TorDialer,
    label: &str,
    rotation_on: bool,
    target: (&str, u16),
    key: Option<&str>,
) -> Run {
    let mut dials: Vec<Dial> = Vec::new();
    for i in 0..DIALS {
        let dial = dial_once(cap, d, target.0, target.1, key).await;
        eprintln!(
            "[{label}] d{} tunnel={} outcome={} kind={:?} rotated={} (+{:?})",
            i + 1,
            dial.tunnel_id,
            match &dial.outcome {
                Ok(()) => "connected".to_owned(),
                Err(e) => e.class().to_owned(),
            },
            dial.kind,
            dial.rotated,
            dial.started.elapsed()
        );
        if i == 0 && dial.outcome.is_ok() {
            eprintln!("[{label}] SKIP: d1 SUCCEEDED (an exit answered the unanswerable target)");
            return Run::Skipped;
        }
        let kind = dial.kind;
        assert!(
            dial.outcome.is_err(),
            "[{label}] d{} connected to a target that cannot answer",
            i + 1
        );
        let kind = kind.unwrap_or_else(|| {
            panic!(
                "[{label}] d{}: could not name the arti kind of {:?}",
                i + 1,
                dial.outcome
            )
        });
        assert!(
            ROTATE.contains(&kind),
            "[{label}] d{} failed with {kind:?}, which is not in the rotate-set; this \
             shape does not exercise rotation",
            i + 1
        );
        dials.push(dial);
        if !rotation_on && dials.len() == 2 {
            break; // OFF asserts the first two only.
        }
    }
    let first_kind = dials[0].kind;

    // What is asserted is what THIS CRATE does — its rotation events — plus the
    // one circuit fact its rotation makes deterministic. Arti's own circuit
    // choices are RECORDED, never asserted: measured over four runs, arti
    // sometimes leaves a failing circuit by itself (a keyed timeout lane held 1
    // of 3; one whole run moved on nearly every dial, rotation off included).
    if rotation_on {
        assert!(
            dials[0].rotated,
            "[{label}] ROTATION ON: the first exit failure did not rotate the group"
        );
        // A fresh token can never join the circuit the old token used, so d2's
        // circuit differs from d1's whatever arti does otherwise.
        assert_ne!(
            dials[1].tunnel_id, dials[0].tunnel_id,
            "[{label}] ROTATION ON: d2 reused d1's circuit after the rotation"
        );
        // The bound: no further rotation inside the interval, measured from the
        // first failure (no later than d2's start).
        let bound_from = dials[1].started;
        for (i, dial) in dials.iter().enumerate().skip(1) {
            if dial.started.saturating_duration_since(bound_from) >= EXIT_ROTATION_INTERVAL {
                eprintln!(
                    "[{label}] d{} started past the interval; not asserted",
                    i + 1
                );
                continue;
            }
            assert!(
                !dial.rotated,
                "[{label}] ROTATION ON: d{} rotated again inside the interval — the bound did \
                 not hold",
                i + 1
            );
            eprintln!(
                "RECORD [{label}] d{} {} d2's circuit (arti's choice; no rotation)",
                i + 1,
                if dial.tunnel_id == dials[1].tunnel_id {
                    "kept"
                } else {
                    "LEFT"
                }
            );
        }
        let real = d.connect(REAL_HOST, REAL_PORT, key).await;
        assert!(
            real.is_ok(),
            "[{label}] the real host did not connect after rotation: {:?}",
            real.err().map(|e| e.class())
        );
        eprintln!("[{label}] real host connected (no byte sent)");
    } else {
        for (i, dial) in dials.iter().enumerate() {
            assert!(
                !dial.rotated,
                "[{label}] ROTATION OFF: d{} rotated — off() must leave arti's behaviour alone",
                i + 1
            );
        }
        eprintln!(
            "RECORD [{label}] ROTATION OFF: arti {} the failing circuit on its own",
            if dials[1].tunnel_id == dials[0].tunnel_id {
                "held"
            } else {
                "LEFT"
            }
        );
    }
    Run::Done { first_kind }
}

// ── Shape A ─────────────────────────────────────────────────────────────────

async fn shape_a(rotation_on: bool) {
    let cap = Capture::default();
    let _guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(cap.clone()));
    let (leg, policy) = if rotation_on {
        ("shape-a-on", ExitRotation::default())
    } else {
        ("shape-a-off", ExitRotation::off())
    };
    let d = live_dialer(leg, policy).await;
    for key in [None, Some("fr54-live-shape-a")] {
        let label = format!("{leg} key={}", key.is_some());
        let host = random_invalid();
        let _ = run_group(&cap, &d, &label, rotation_on, (&host, 443), key).await;
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "live Tor network; DIALER_TOR_LIVE=1 and --ignored"]
async fn shape_a_invalid_name_rotation_on() {
    if live_enabled() {
        shape_a(true).await;
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "live Tor network; DIALER_TOR_LIVE=1 and --ignored"]
async fn shape_a_invalid_name_rotation_off() {
    if live_enabled() {
        shape_a(false).await;
    }
}

// ── Shape B ─────────────────────────────────────────────────────────────────

async fn shape_b(rotation_on: bool) {
    let cap = Capture::default();
    let _guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(cap.clone()));
    let (leg, policy) = if rotation_on {
        ("shape-b-on", ExitRotation::default())
    } else {
        ("shape-b-off", ExitRotation::off())
    };
    let d = live_dialer(leg, policy).await;
    let target = (TEST_NET, 443);
    let mut recorded: Vec<(String, Option<ErrorKind>)> = Vec::new();

    let label = format!("{leg} key=false");
    if let Run::Done { first_kind } = run_group(&cap, &d, &label, rotation_on, target, None).await {
        recorded.push((label, first_kind));
    }
    // Keyed: a fresh key per attempt is a fresh group with no rotation state,
    // so a policy rejection can be rerun without disturbing the bound.
    for attempt in 0..=SHAPE_B_POLICY_RERUNS {
        let key = format!("fr54-live-shape-b-{attempt}");
        let label = format!("{leg} key=true attempt={attempt}");
        if let Run::Done { first_kind } =
            run_group(&cap, &d, &label, rotation_on, target, Some(&key)).await
        {
            let policy_rejected = first_kind == Some(ErrorKind::ExitPolicyRejected);
            recorded.push((label, first_kind));
            if !policy_rejected {
                break;
            }
            eprintln!("[{leg}] the exit's POLICY rejected TEST-NET; rerunning on a fresh key");
        }
    }

    for (label, kind) in &recorded {
        eprintln!("RECORD [{label}] shape B first-dial kind = {kind:?}");
    }
    if rotation_on {
        assert!(
            recorded
                .iter()
                .any(|(_, k)| k.is_some_and(|k| OUTAGE_CLASS.contains(&k))),
            "shape B never produced the outage's own class (timeout / no route at the exit) \
             in {} runs: {recorded:?}",
            recorded.len()
        );
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "live Tor network; DIALER_TOR_LIVE=1 and --ignored"]
async fn shape_b_test_net_rotation_on() {
    if live_enabled() {
        shape_b(true).await;
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "live Tor network; DIALER_TOR_LIVE=1 and --ignored"]
async fn shape_b_test_net_rotation_off() {
    if live_enabled() {
        shape_b(false).await;
    }
}

/// Not live: the gate itself, so `--ignored` without the variable is inert.
#[test]
fn the_live_gate_is_off_without_the_variable() {
    if std::env::var_os("DIALER_TOR_LIVE").is_none() {
        assert!(!live_enabled());
    }
}
