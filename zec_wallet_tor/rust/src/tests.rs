//! The mechanism tests (FR-5 spec §8 P1–P21, plan §2 C3b): the trampoline and
//! the lifecycle over a FAKE engine (scripted readiness, `tokio::io::duplex`
//! streams the test holds the far end of), a RECORDING wallet link and a fake
//! pause clock — no network, no arti, no wallet library.
//!
//! Each test builds its OWN plugin on its own runtime. Two runtime shapes: a
//! multi-thread runtime on real time for the ordering rules, and a
//! current-thread runtime with PAUSED time, driven from a background thread,
//! for the rules that are about durations (tokio auto-advances the clock when
//! every task is waiting, so 180 s passes instantly and exactly).
//!
//! `unsafe` is allowed HERE, in test code only: P17b, P18 and P21 must call
//! the real vtable entries through their C function pointers, exactly as the
//! wallet does. The shipped crate keeps `abi.rs` as its one unsafe module.

#![allow(unsafe_code)]

use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use dialer_tor::{AsyncByteStream, DormantMode, ErrorKind, Readiness, TorDialError};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::Notify;
use zeroize::Zeroizing;

use crate::abi::{
    ZW_DIAL_NOT_READY, ZW_DIAL_OK, ZW_DIAL_REFUSED, ZW_DIAL_RETIRED, ZW_DIAL_UNREACHABLE,
    ZW_EXPOSURE_HIDDEN, ZW_HEALTH_FAILED, ZW_HEALTH_READY, ZW_HEALTH_STARTING,
    ZW_ISOLATION_SUPPORTED, ZWT_RC_BRIDGES_REFUSED, ZWT_RC_DISPOSED, ZWT_RC_INVALID_DATA_DIR,
    ZWT_RC_NOT_INITIALIZED, ZWT_RC_OK, ZWT_RC_RESTART_REQUIRED, ZWT_RC_SLOT_OCCUPIED,
    ZWT_RC_STOPPING, ZwNetDialerV1, ZwTransportDescriptor, vtable_for,
};
use crate::bootstrap::NoJitter;
use crate::constants::{
    BOOTSTRAP_DEADLINE, BOOTSTRAP_RETRY_BASE, CLASS_BOOTSTRAP_DEADLINE, CLASS_CIRCUITS_FAILING,
    CLASS_NOT_REGISTERED, DIAL_DEADLINE, LIVENESS_FAILED_DIALS, QUIESCE_POLL, TOR_DIR_MARKER,
};

/// Long enough for a waiting rebuild to have polled several times
/// ([`QUIESCE_POLL`]): a negative "no mint yet" check after it is not
/// vacuous however the poll is tuned (the code review of the rebuild fold).
const REBUILD_POLLS: Duration = Duration::from_millis(4 * QUIESCE_POLL.as_millis() as u64);
use crate::dial_codes::dial_code;
use crate::engine::{EngineConfig, EngineFactory, TorEngine};
use crate::lifecycle::remove_tor_state;
use crate::status::Phase;
use crate::trampoline::{
    Deps, LinkResolver, PauseClock, Plugin, ReadOutcome, Token, WalletLink, descriptor,
};
use crate::watcher::{Health, Pushed};
use dialer_tor::ClientLedger;

/// A vanilla bridge line `dialer-tor`'s own tests accept (documentation
/// address, a well-formed fingerprint).
const VANILLA: &str = "192.0.2.55:38114 316E643333645F6D79216558614D3931657A5F5F";

/// Paused time lands on a timer's tick, which may round a deadline up by less
/// than a millisecond.
fn about(actual: Duration, expected: Duration) {
    assert!(
        actual >= expected && actual < expected + Duration::from_millis(10),
        "{actual:?} is not {expected:?}"
    );
}

// ─── the fakes ───────────────────────────────────────────────────────────────

#[derive(Clone)]
enum Boot {
    Ok,
    Hang,
}

#[derive(Clone)]
enum Connect {
    Ok,
    /// A stream whose writes sit in a buffer until flushed (arti's shape).
    Buffered,
    Fail(TorDialError),
    Held,
    Panic,
}

struct FakeEngine {
    readiness: Mutex<Readiness>,
    boot: Boot,
    connects: Mutex<VecDeque<Connect>>,
    release: Notify,
    peers: Mutex<Vec<DuplexStream>>,
    keys: Mutex<Vec<Option<String>>>,
    dormant: Mutex<Vec<DormantMode>>,
    boots: Mutex<Vec<tokio::time::Instant>>,
    connect_started: Mutex<Vec<tokio::time::Instant>>,
    /// Set by [`TorEngine::retire`]: a rebuild or a dispose started this
    /// client's shutdown.
    retired: std::sync::atomic::AtomicBool,
}

impl FakeEngine {
    fn new(readiness: Readiness, boot: Boot) -> Self {
        Self {
            readiness: Mutex::new(readiness),
            boot,
            connects: Mutex::new(VecDeque::new()),
            release: Notify::new(),
            peers: Mutex::new(Vec::new()),
            keys: Mutex::new(Vec::new()),
            dormant: Mutex::new(Vec::new()),
            boots: Mutex::new(Vec::new()),
            connect_started: Mutex::new(Vec::new()),
            retired: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn ready() -> Self {
        Self::new(Readiness::Ready, Boot::Ok)
    }

    fn script(&self, c: Connect) {
        self.connects.lock().unwrap().push_back(c);
    }

    fn set_readiness(&self, r: Readiness) {
        *self.readiness.lock().unwrap() = r;
    }

    fn peer(&self, i: usize) -> DuplexStream {
        self.peers.lock().unwrap().remove(i)
    }
}

#[async_trait]
impl TorEngine for FakeEngine {
    async fn bootstrap(&self) -> Result<(), TorDialError> {
        self.boots.lock().unwrap().push(tokio::time::Instant::now());
        match self.boot {
            Boot::Ok => Ok(()),
            Boot::Hang => std::future::pending().await,
        }
    }

    fn readiness(&self) -> Readiness {
        self.readiness.lock().unwrap().clone()
    }

    async fn connect(
        &self,
        _host: &str,
        _port: u16,
        isolation_key: Option<&str>,
    ) -> Result<Box<dyn AsyncByteStream>, TorDialError> {
        self.connect_started
            .lock()
            .unwrap()
            .push(tokio::time::Instant::now());
        self.keys
            .lock()
            .unwrap()
            .push(isolation_key.map(str::to_owned));
        let script = self
            .connects
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Connect::Ok);
        let script = match script {
            Connect::Held => {
                self.release.notified().await;
                Connect::Ok
            }
            other => other,
        };
        match script {
            Connect::Ok | Connect::Held => {
                let (ours, theirs) = tokio::io::duplex(64 * 1024);
                self.peers.lock().unwrap().push(theirs);
                Ok(Box::new(ours))
            }
            Connect::Buffered => {
                let (ours, theirs) = tokio::io::duplex(64 * 1024);
                self.peers.lock().unwrap().push(theirs);
                Ok(Box::new(tokio::io::BufWriter::new(ours)))
            }
            Connect::Fail(e) => Err(e),
            Connect::Panic => panic!("planted: a panic inside the engine's connect"),
        }
    }

    fn retire(&self) {
        self.retired.store(true, Ordering::SeqCst);
    }

    fn set_dormant(&self, mode: DormantMode) {
        self.dormant.lock().unwrap().push(mode);
    }
}

type Make = Box<dyn Fn() -> FakeEngine + Send + Sync>;

struct FakeFactory {
    make: Make,
    /// WEAK: the plugin's rebuild waits for the old engine's last user to let
    /// go (`Arc::strong_count`), and production has no second holder — a
    /// strong list here would make every quiesce run to its bound.
    minted: Mutex<Vec<std::sync::Weak<FakeEngine>>>,
    mints: AtomicUsize,
    /// While set, a mint waits (a slow arti start writing `state/`).
    hold_mint: std::sync::atomic::AtomicBool,
}

impl FakeFactory {
    fn engine(&self, i: usize) -> Arc<FakeEngine> {
        self.minted.lock().unwrap()[i]
            .upgrade()
            .expect("the engine is still alive (the plugin holds it)")
    }
}

#[async_trait]
impl EngineFactory for FakeFactory {
    async fn mint(
        &self,
        _cfg: &EngineConfig,
        live: &Arc<ClientLedger>,
    ) -> Result<Arc<dyn TorEngine>, TorDialError> {
        // `dialer-tor`'s own rule (`spawn_owned`), mirrored: no new client
        // while one is stuck.
        if live.stuck() > 0 {
            return Err(TorDialError::RestartRequired);
        }
        self.mints.fetch_add(1, Ordering::SeqCst);
        while self.hold_mint.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let engine = Arc::new((self.make)());
        self.minted.lock().unwrap().push(Arc::downgrade(&engine));
        Ok(engine)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Call {
    Register { pushed: Pushed, mints_before: usize },
    Notify { pushed: Pushed, retire: bool },
    Clear,
}

struct FakeLink {
    calls: Mutex<Vec<Call>>,
    register_rc: Mutex<i32>,
    factory: Arc<FakeFactory>,
}

fn pushed_of(d: &ZwTransportDescriptor) -> Pushed {
    let health = match d.health {
        ZW_HEALTH_STARTING => Health::Starting,
        ZW_HEALTH_READY => Health::Ready,
        ZW_HEALTH_FAILED => Health::Failed,
        other => panic!("health {other} is outside the closed set"),
    };
    Pushed {
        health,
        readiness: d.readiness,
    }
}

impl WalletLink for FakeLink {
    fn register(&self, _: &ZwNetDialerV1, d: &ZwTransportDescriptor) -> Result<Token, i32> {
        self.calls.lock().unwrap().push(Call::Register {
            pushed: pushed_of(d),
            mints_before: self.factory.mints.load(Ordering::SeqCst),
        });
        match *self.register_rc.lock().unwrap() {
            0 => Ok(Zeroizing::new([7; 32])),
            rc => Err(rc),
        }
    }

    fn notify(&self, _: &Token, d: &ZwTransportDescriptor, retire: bool) -> i32 {
        self.calls.lock().unwrap().push(Call::Notify {
            pushed: pushed_of(d),
            retire,
        });
        0
    }

    fn clear(&self, _: &Token) -> i32 {
        self.calls.lock().unwrap().push(Call::Clear);
        0
    }
}

struct Resolver(Option<Arc<FakeLink>>);

impl LinkResolver for Resolver {
    fn resolve(&self) -> Option<Arc<dyn WalletLink>> {
        self.0.clone().map(|l| l as Arc<dyn WalletLink>)
    }
}

#[derive(Default)]
struct FakeClock(Mutex<Option<Duration>>);

impl PauseClock for Arc<FakeClock> {
    fn now(&self) -> Option<Duration> {
        *self.0.lock().unwrap()
    }
}

impl FakeClock {
    fn set(&self, secs: u64) {
        *self.0.lock().unwrap() = Some(Duration::from_secs(secs));
    }
}

// ─── the harness ─────────────────────────────────────────────────────────────

enum Rt {
    /// Held so the runtime lives exactly as long as the harness.
    Real(#[allow(dead_code)] tokio::runtime::Runtime),
    Paused(Option<tokio::sync::oneshot::Sender<()>>),
}

struct Harness {
    plugin: Arc<Plugin>,
    factory: Arc<FakeFactory>,
    link: Arc<FakeLink>,
    clock: Arc<FakeClock>,
    dir: TempDir,
    handle: tokio::runtime::Handle,
    _rt: Rt,
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Rt::Paused(stop) = &mut self._rt
            && let Some(stop) = stop.take()
        {
            let _ = stop.send(());
        }
    }
}

fn build(make: Make, paused: bool) -> Harness {
    let (rt, handle) = if paused {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()
            .unwrap();
        let handle = rt.handle().clone();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        std::thread::spawn(move || {
            let _ = rt.block_on(stopped);
        });
        (Rt::Paused(Some(stop)), handle)
    } else {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let handle = rt.handle().clone();
        (Rt::Real(rt), handle)
    };
    let factory = Arc::new(FakeFactory {
        make,
        minted: Mutex::new(Vec::new()),
        mints: AtomicUsize::new(0),
        hold_mint: std::sync::atomic::AtomicBool::new(false),
    });
    let link = Arc::new(FakeLink {
        calls: Mutex::new(Vec::new()),
        register_rc: Mutex::new(0),
        factory: factory.clone(),
    });
    let clock = Arc::new(FakeClock::default());
    let plugin = Plugin::new(
        Deps {
            factory: factory.clone(),
            resolver: Box::new(Resolver(Some(link.clone()))),
            clock: Box::new(clock.clone()),
            jitter: Box::new(NoJitter),
        },
        handle.clone(),
    );
    Harness {
        plugin,
        factory,
        link,
        clock,
        dir: tempfile::tempdir().unwrap(),
        handle,
        _rt: rt,
    }
}

fn harness(make: impl Fn() -> FakeEngine + Send + Sync + 'static) -> Harness {
    build(Box::new(make), false)
}

fn paused_harness(make: impl Fn() -> FakeEngine + Send + Sync + 'static) -> Harness {
    build(Box::new(make), true)
}

/// Poll `cond` for up to five seconds of real time.
fn eventually(what: &str, cond: impl Fn() -> bool) {
    for _ in 0..1000 {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for: {what}");
}

type Completions = Arc<Mutex<Vec<(u32, u64)>>>;

impl Harness {
    fn tor_dir(&self) -> String {
        self.dir.path().join("tor").to_str().unwrap().to_owned()
    }

    fn init(&self) -> i32 {
        self.plugin.init(&self.tor_dir(), None, None)
    }

    fn init_ready(&self) {
        assert_eq!(self.init(), ZWT_RC_OK);
        eventually("the plugin reaches Ready", || {
            self.plugin.status().phase == Phase::Ready
        });
    }

    fn calls(&self) -> Vec<Call> {
        self.link.calls.lock().unwrap().clone()
    }

    fn notifies(&self) -> Vec<(Pushed, bool)> {
        self.calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::Notify { pushed, retire } => Some((pushed, retire)),
                _ => None,
            })
            .collect()
    }

    fn generation(&self) -> u64 {
        self.plugin.lock().generation
    }

    /// Dial through the Rust verb, recording completions.
    fn dial(&self, key: Option<&str>) -> (u32, Completions) {
        let seen: Completions = Arc::default();
        let sink = seen.clone();
        let code = self.plugin.dial(
            "lwd.example".into(),
            9067,
            key.map(str::to_owned),
            Box::new(move |code, stream| sink.lock().unwrap().push((code, stream))),
        );
        (code, seen)
    }

    /// Dial and wait for an OK stream.
    fn open_stream(&self) -> u64 {
        let (code, seen) = self.dial(Some("wallet-sync"));
        assert_eq!(code, ZW_DIAL_OK);
        eventually("the dial completes", || !seen.lock().unwrap().is_empty());
        let (code, stream) = seen.lock().unwrap()[0];
        assert_eq!(code, ZW_DIAL_OK);
        assert_ne!(stream, 0, "a stream handle is never zero");
        stream
    }
}

fn completed_once(seen: &Completions) -> (u32, u64) {
    let v = seen.lock().unwrap();
    assert_eq!(v.len(), 1, "exactly one completion, saw {v:?}");
    v[0]
}

// ─── registration ────────────────────────────────────────────────────────────

/// **P1**: `register` happens with the factory never called, at readiness 0
/// STARTING; the one mint follows it.
#[test]
fn the_plugin_registers_before_it_starts_arti() {
    let h = harness(FakeEngine::ready);
    assert_eq!(h.init(), ZWT_RC_OK);
    let calls = h.calls();
    assert_eq!(
        calls[0],
        Call::Register {
            pushed: Pushed {
                health: Health::Starting,
                readiness: 0
            },
            mints_before: 0
        }
    );
    assert_eq!(h.factory.mints.load(Ordering::SeqCst), 1);
    // A second init is idempotent: no second registration, no second mint.
    assert_eq!(h.init(), ZWT_RC_OK);
    let registers = h
        .calls()
        .iter()
        .filter(|c| matches!(c, Call::Register { .. }))
        .count();
    assert_eq!(registers, 1);
    assert_eq!(h.factory.mints.load(Ordering::SeqCst), 1);
}

/// **P2** (the fake-registry half): an occupied slot answers
/// `SLOT_OCCUPIED`, arti is never minted, `NotRegistered { not-registered }`.
#[test]
fn an_occupied_slot_stops_the_plugin_without_starting_arti() {
    let h = harness(FakeEngine::ready);
    *h.link.register_rc.lock().unwrap() = crate::abi::ZW_RC_OCCUPIED;
    assert_eq!(h.init(), ZWT_RC_SLOT_OCCUPIED);
    assert_eq!(h.factory.mints.load(Ordering::SeqCst), 0);
    let status = h.plugin.status();
    assert_eq!(status.phase, Phase::NotRegistered);
    assert_eq!(status.failure_class, Some(CLASS_NOT_REGISTERED));
}

/// **P3** (the plugin half; the real registry's acceptance is the device
/// walk's): "Tor", isolation SUPPORTED, exposure HIDDEN, the name ZERO-FILLED
/// beyond its length.
#[test]
fn the_plugins_descriptor_is_tor_supported_hidden() {
    let d = descriptor(Pushed {
        health: Health::Ready,
        readiness: 100,
    });
    assert_eq!(&d.name[..d.name_len as usize], b"Tor");
    assert!(d.name[d.name_len as usize..].iter().all(|b| *b == 0));
    assert_eq!(d.isolation, ZW_ISOLATION_SUPPORTED);
    assert_eq!(d.exposure, ZW_EXPOSURE_HIDDEN);
    assert_eq!((d.readiness, d.health), (100, ZW_HEALTH_READY));
}

// ─── the dial path ───────────────────────────────────────────────────────────

/// **P20**: a dial while `Suspended`, `Bootstrapping` or `Failed` is refused
/// `NOT_READY` synchronously — no completion, no engine call; the LIVE
/// readiness is re-read even when the phase says `Ready`; `RETIRED` after
/// `dispose`.
#[test]
fn a_dial_while_suspended_or_not_ready_is_refused_synchronously() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);

    // Phase Ready, but arti's live readiness dropped (a drop's confirmations
    // have not moved the phase yet): refused.
    engine.set_readiness(Readiness::Bootstrapping {
        progress: 0.5,
        blockage: None,
    });
    let (code, seen) = h.dial(None);
    assert_eq!(code, ZW_DIAL_NOT_READY);
    engine.set_readiness(Readiness::Ready);

    for phase in [Phase::Bootstrapping, Phase::Failed, Phase::Suspended] {
        h.plugin.lock().phase = phase;
        let (code, _) = h.dial(None);
        assert_eq!(code, ZW_DIAL_NOT_READY, "{phase:?}");
    }
    h.plugin.lock().phase = Phase::Ready;
    std::thread::sleep(Duration::from_millis(50));
    assert!(seen.lock().unwrap().is_empty());
    assert!(engine.keys.lock().unwrap().is_empty(), "no engine call");

    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    let (code, _) = h.dial(None);
    assert_eq!(code, ZW_DIAL_RETIRED);
}

/// **P9, the runtime half**: every engine failure reaches the wallet through
/// the ONE table, and no completion ever carries `UNREACHABLE` — even with
/// arti's blockage `Offline` (plan §5 D-17: the plugin builds no producer of
/// code 2).
#[test]
fn every_engine_failure_reaches_the_wallet_through_the_one_table_and_a_superseded_dial_reads_retired()
 {
    let errors = [
        TorDialError::TorNetworkTimeout,
        TorDialError::ExitTimeout,
        TorDialError::RemoteNetworkTimeout,
        TorDialError::RefusedByExitPolicy,
        TorDialError::HostNotFound,
        TorDialError::Setup {
            kind: ErrorKind::Other,
        },
        TorDialError::NotBootstrapped {
            progress: 0.2,
            blockage: Some(dialer_tor::BlockageKind::Offline),
        },
    ];
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    for e in &errors {
        engine.script(Connect::Fail(e.clone()));
        let (code, seen) = h.dial(None);
        assert_eq!(code, ZW_DIAL_OK);
        eventually("the dial completes", || !seen.lock().unwrap().is_empty());
        let (got, stream) = completed_once(&seen);
        assert_eq!(got, dial_code(e), "{e:?}");
        assert_ne!(got, ZW_DIAL_UNREACHABLE, "{e:?}: no completion carries 2");
        assert_eq!(stream, 0);
    }
    // A superseded dial: held, a rebuild, then released → RETIRED.
    engine.script(Connect::Held);
    let (code, seen) = h.dial(None);
    assert_eq!(code, ZW_DIAL_OK);
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    engine.release.notify_one();
    eventually("the held dial completes", || {
        !seen.lock().unwrap().is_empty()
    });
    assert_eq!(completed_once(&seen).0, ZW_DIAL_RETIRED);
}

/// **P18**: the key reaches the engine verbatim; `(NULL, 0)` reaches it as
/// `None` with no pointer read; a non-UTF-8 or over-long key is `REFUSED`
/// synchronously with no completion and no engine call. Driven through the
/// real vtable entry.
#[test]
fn an_isolation_key_reaches_arti_verbatim_an_absent_one_is_none_and_an_invalid_one_is_refused_typed()
 {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let vt = vtable_for(&h.plugin);
    let dial = vt.dial.unwrap();
    let host = b"lwd.example";
    let call = |key: *const u8, len: usize| -> u32 {
        // SAFETY: a live plugin's vtable, spans valid for the call.
        unsafe {
            dial(
                vt.ctx,
                host.as_ptr(),
                host.len(),
                9067,
                key,
                len,
                1,
                std::ptr::null_mut(),
                c_dial_done,
            )
        }
    };
    let engine = h.factory.engine(0);
    assert_eq!(call(b"wallet-sync".as_ptr(), 11), ZW_DIAL_OK);
    let send = b"wallet-send-0123456789abcdef";
    assert_eq!(call(send.as_ptr(), send.len()), ZW_DIAL_OK);
    assert_eq!(call(std::ptr::null(), 0), ZW_DIAL_OK);
    eventually("three engine calls", || {
        engine.keys.lock().unwrap().len() == 3
    });
    // Three tasks reach the engine in any order; compare the set.
    let mut keys = engine.keys.lock().unwrap().clone();
    keys.sort();
    assert_eq!(
        keys,
        [
            None,
            Some("wallet-send-0123456789abcdef".to_owned()),
            Some("wallet-sync".to_owned()),
        ]
    );
    let bad = [0xffu8, 0xfe];
    assert_eq!(call(bad.as_ptr(), bad.len()), ZW_DIAL_REFUSED);
    let long = vec![b'k'; 257];
    assert_eq!(call(long.as_ptr(), long.len()), ZW_DIAL_REFUSED);
    // A NULL pointer with a length is refused, never read.
    assert_eq!(call(std::ptr::null(), 4), ZW_DIAL_REFUSED);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        engine.keys.lock().unwrap().len(),
        3,
        "no engine call for a refusal"
    );
}

extern "C" fn c_dial_done(_sdk: *mut c_void, _op: u64, _code: u32, _stream: u64) {}

/// The dial deadline (the crypto angle's HIGH, plan C3b fold (1)): it is
/// bound to the wallet's HEADER — read here, never restated — and a READY
/// engine whose connect never answers is completed by the plugin, `REFUSED`,
/// at exactly [`DIAL_DEADLINE`], before the wallet's own clock.
#[test]
fn the_dial_deadline_sits_below_the_wallets_budget() {
    let header = include_str!("../../../zec_wallet/rust/include/zec_wallet_net_dialer.h");
    let budget: u64 = header
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("#define ZW_NET_DIALER_DIAL_BUDGET_SECS ")
        })
        .expect("the wallet's header defines its dial budget")
        .trim()
        .trim_end_matches('u')
        .parse()
        .expect("an integer");
    assert!(
        DIAL_DEADLINE < Duration::from_secs(budget),
        "the plugin's dial deadline {DIAL_DEADLINE:?} must sit below the wallet's {budget} s"
    );

    let h = paused_harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    engine.script(Connect::Held);
    let finished: Arc<Mutex<Option<(u32, tokio::time::Instant)>>> = Arc::default();
    let sink = finished.clone();
    let code = h.plugin.dial(
        "lwd.example".into(),
        9067,
        None,
        Box::new(move |code, _| *sink.lock().unwrap() = Some((code, tokio::time::Instant::now()))),
    );
    assert_eq!(code, ZW_DIAL_OK);
    eventually("the deadline completes the dial", || {
        finished.lock().unwrap().is_some()
    });
    let (code, at) = finished.lock().unwrap().unwrap();
    let started = engine.connect_started.lock().unwrap()[0];
    assert_eq!(
        code, ZW_DIAL_REFUSED,
        "the deadline is a code the plugin chose"
    );
    about(at - started, DIAL_DEADLINE);
}

/// The liveness rule (the crypto angle's MEDIUM): arti's readiness stays
/// 100 while every dial fails; three device/Tor-side failures in a row move
/// the plugin to `Failed { circuits-failing }` with health FAILED pushed at
/// the MEASURED 100, at once; a `retry` rebuilds. Failures an exit or the far
/// end controls never trip it.
#[test]
fn a_ready_client_whose_dials_keep_failing_declares_failed_and_rebuilds() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    for _ in 0..5 {
        engine.script(Connect::Fail(TorDialError::ExitTimeout));
        let (_, seen) = h.dial(None);
        eventually("completes", || !seen.lock().unwrap().is_empty());
    }
    assert_eq!(
        h.plugin.status().phase,
        Phase::Ready,
        "exit-side failures never trip it"
    );

    for i in 0..LIVENESS_FAILED_DIALS {
        engine.script(Connect::Fail(TorDialError::TorAccessFailed));
        let (_, seen) = h.dial(None);
        eventually("completes", || !seen.lock().unwrap().is_empty());
        let expect = if i + 1 < LIVENESS_FAILED_DIALS {
            Phase::Ready
        } else {
            Phase::Failed
        };
        assert_eq!(h.plugin.status().phase, expect, "after {} failures", i + 1);
    }
    let status = h.plugin.status();
    assert_eq!(status.failure_class, Some(CLASS_CIRCUITS_FAILING));
    assert_eq!(
        h.notifies().last().unwrap().0,
        Pushed {
            health: Health::Failed,
            readiness: 100
        }
    );
    assert_eq!(h.plugin.retry_bootstrap(), ZWT_RC_OK);
    eventually("a retry rebuilds", || {
        h.factory.mints.load(Ordering::SeqCst) == 2
    });
}

// ─── streams ─────────────────────────────────────────────────────────────────

/// Every write is FLUSHED before it completes (the header's WRITE COMPLETION
/// MEANS SENT): over a stream that holds writes until a flush, the far end has
/// the bytes by the time the completion arrives.
#[test]
fn a_write_completes_only_after_its_bytes_are_flushed() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    engine.script(Connect::Buffered);
    let stream = h.open_stream();
    let mut peer = engine.peer(0);
    let seen: Arc<Mutex<Vec<(u32, usize)>>> = Arc::default();
    let sink = seen.clone();
    let code = h.plugin.write(
        stream,
        b"submit".to_vec(),
        Box::new(move |code, n| sink.lock().unwrap().push((code, n))),
    );
    assert_eq!(code, ZW_DIAL_OK);
    eventually("the write completes", || !seen.lock().unwrap().is_empty());
    assert_eq!(seen.lock().unwrap()[0], (ZW_DIAL_OK, 6));
    let got = h.handle.block_on(async {
        let mut buf = [0u8; 6];
        tokio::time::timeout(Duration::from_millis(200), peer.read_exact(&mut buf))
            .await
            .map(|_| buf)
    });
    assert_eq!(
        got.ok(),
        Some(*b"submit"),
        "the bytes were still in a buffer"
    );
}

/// **P17a**: a read held inside the engine and released AFTER `close`
/// returned completes RETIRED, exactly once, from its own task — and writes
/// NOTHING into the SDK's buffer (the canary stays intact). A second read on
/// a stream with one outstanding is REFUSED synchronously.
#[test]
fn close_during_a_completion_is_exactly_once_and_touches_no_sdk_buffer_after_close() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    let stream = h.open_stream();
    let mut peer = engine.peer(0);

    let canary = Arc::new(Mutex::new(vec![0xAAu8; 16]));
    let outcomes: Arc<Mutex<Vec<u32>>> = Arc::default();
    let (buf, out) = (canary.clone(), outcomes.clone());
    let code = h.plugin.read(
        stream,
        16,
        Box::new(move |outcome| match outcome {
            ReadOutcome::Data(bytes) => {
                buf.lock().unwrap()[..bytes.len()].copy_from_slice(bytes);
                out.lock().unwrap().push(ZW_DIAL_OK);
            }
            ReadOutcome::Code(code) => out.lock().unwrap().push(code),
        }),
    );
    assert_eq!(code, ZW_DIAL_OK);
    let (second, _) = (h.plugin.read(stream, 16, Box::new(|_| {})), ());
    assert_eq!(second, ZW_DIAL_REFUSED, "one outstanding read per stream");

    h.plugin.close(stream);
    // The late bytes try to reach the buffer; once the plugin has dropped
    // both halves the write itself may fail (a broken pipe) — either way
    // nothing may land in the SDK's buffer.
    h.handle
        .block_on(async { peer.write_all(b"late bytes").await.ok() });
    eventually("the read completes", || {
        !outcomes.lock().unwrap().is_empty()
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(*outcomes.lock().unwrap(), [ZW_DIAL_RETIRED]);
    assert!(
        canary.lock().unwrap().iter().all(|b| *b == 0xAA),
        "a byte reached the SDK's buffer after close returned"
    );
    // Idempotent; the stream is gone.
    h.plugin.close(stream);
    assert_eq!(h.plugin.read(stream, 16, Box::new(|_| {})), ZW_DIAL_RETIRED);
}

static PLANTED: Mutex<Vec<u32>> = Mutex::new(Vec::new());

extern "C" fn c_record_dial(_sdk: *mut c_void, _op: u64, code: u32, _stream: u64) {
    PLANTED.lock().unwrap().push(code);
}

/// **P17b** (the security angle's A4): a panic planted between the op's
/// record and its accept answers REFUSED through the vtable with ZERO
/// completions for that op (a later one would be dropped by the SDK and leak
/// the stream); a panic inside the engine's connect — the owning task —
/// completes the op RETIRED, once.
#[test]
fn a_planted_panic_in_a_vtable_entry_returns_a_code_and_in_a_completion_fails_the_op() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let vt = vtable_for(&h.plugin);
    let host = b"lwd.example";
    let dial = || {
        // SAFETY: a live plugin's vtable, spans valid for the call.
        unsafe {
            vt.dial.unwrap()(
                vt.ctx,
                host.as_ptr(),
                host.len(),
                9067,
                std::ptr::null(),
                0,
                1,
                std::ptr::null_mut(),
                c_record_dial,
            )
        }
    };
    crate::trampoline::PANIC_AFTER_RECORD.with(|p| p.set(true));
    let code = dial();
    crate::trampoline::PANIC_AFTER_RECORD.with(|p| p.set(false));
    assert_eq!(code, ZW_DIAL_REFUSED);
    std::thread::sleep(Duration::from_millis(100));
    assert!(PLANTED.lock().unwrap().is_empty(), "a refused op completed");
    assert_eq!(Plugin::outstanding(&h.plugin.lock()), 0);

    h.factory.engine(0).script(Connect::Panic);
    assert_eq!(dial(), ZW_DIAL_OK);
    eventually("the panicked op completes", || {
        !PLANTED.lock().unwrap().is_empty()
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(*PLANTED.lock().unwrap(), [ZW_DIAL_RETIRED]);
}

// ─── lifecycle ───────────────────────────────────────────────────────────────

/// **P6**: `on_paused` flips to `Suspended`, pushes readiness 0 with
/// `retire = 0` once, sets arti `Soft` once, and does NOT bump the generation
/// — a dial accepted before the pause completes on its merits, one after it
/// is refused.
#[test]
fn a_pause_flips_the_phase_atomically_pushes_zero_and_sets_arti_dormant_without_retiring() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    engine.script(Connect::Held);
    let (code, before) = h.dial(None);
    assert_eq!(code, ZW_DIAL_OK);
    let generation = h.generation();
    let pushes_before = h.notifies().len();

    assert_eq!(h.plugin.on_paused(), ZWT_RC_OK);
    let notifies = h.notifies();
    assert_eq!(notifies.len(), pushes_before + 1, "exactly one push");
    assert_eq!(
        notifies.last().unwrap(),
        &(
            Pushed {
                health: Health::Starting,
                readiness: 0
            },
            false
        )
    );
    assert_eq!(*engine.dormant.lock().unwrap(), [DormantMode::Soft]);
    assert_eq!(h.generation(), generation, "a pause is not a rebuild");
    assert_eq!(h.plugin.status().phase, Phase::Suspended);
    assert_eq!(h.dial(None).0, ZW_DIAL_NOT_READY);

    engine.release.notify_one();
    eventually("the earlier dial completes", || {
        !before.lock().unwrap().is_empty()
    });
    assert_eq!(completed_once(&before).0, ZW_DIAL_OK);
}

/// **P8**: the pause interval is read from the PAUSE clock — forty minutes
/// there is a rebuild (a second mint over the same dirs, `retire = 1`,
/// readiness from 0); one second is not (no mint, `Normal`, the live
/// readiness re-pushed).
#[test]
fn a_resume_after_a_long_pause_rebuilds_from_the_cache_and_a_short_one_does_not() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);

    h.clock.set(1_000);
    h.plugin.on_paused();
    h.clock.set(1_001);
    h.plugin.on_resumed();
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        1,
        "a short pause keeps the client"
    );
    assert_eq!(
        *engine.dormant.lock().unwrap(),
        [DormantMode::Soft, DormantMode::Normal]
    );
    assert_eq!(
        h.notifies().last().unwrap(),
        &(
            Pushed {
                health: Health::Ready,
                readiness: 100
            },
            false
        )
    );
    assert_eq!(h.plugin.status().phase, Phase::Ready);

    h.clock.set(2_000);
    h.plugin.on_paused();
    h.clock.set(2_000 + 40 * 60);
    h.plugin.on_resumed();
    assert!(
        h.notifies().contains(&(
            Pushed {
                health: Health::Starting,
                readiness: 0
            },
            true
        )),
        "a rebuild retires"
    );
    eventually("a second mint", || {
        h.factory.mints.load(Ordering::SeqCst) == 2
    });
    eventually("the new client is ready", || {
        h.plugin.status().phase == Phase::Ready
    });
}

/// **P7**: a dial accepted before a rebuild and released after it completes
/// RETIRED whatever arti said, and a held read on an old-engine stream
/// completes RETIRED from its own task.
#[test]
fn a_dial_accepted_before_a_rebuild_completes_retired_whatever_arti_said() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    let stream = h.open_stream();
    let reads: Arc<Mutex<Vec<u32>>> = Arc::default();
    let out = reads.clone();
    h.plugin.read(
        stream,
        16,
        Box::new(move |o| {
            out.lock().unwrap().push(match o {
                ReadOutcome::Data(_) => ZW_DIAL_OK,
                ReadOutcome::Code(c) => c,
            })
        }),
    );
    engine.script(Connect::Held);
    let (_, held) = h.dial(None);
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    engine.release.notify_one();
    eventually("both complete", || {
        !held.lock().unwrap().is_empty() && !reads.lock().unwrap().is_empty()
    });
    assert_eq!(completed_once(&held).0, ZW_DIAL_RETIRED);
    assert_eq!(*reads.lock().unwrap(), [ZW_DIAL_RETIRED]);
}

/// **P21**: `dispose` with two held ops: `notify(retire = 1)` first, both ops
/// complete RETIRED exactly once, then CLEAR; a dial through the OLD vtable
/// after `dispose` returned answers RETIRED (the `ctx` is alive); a later
/// `init` registers afresh.
#[test]
fn dispose_retires_then_clears_and_completes_every_outstanding_op_once_and_the_trampoline_outlives_the_clear()
 {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    engine.script(Connect::Held);
    engine.script(Connect::Held);
    let (_, a) = h.dial(None);
    let (_, b) = h.dial(None);
    let before = h.calls().len();

    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    assert_eq!(
        completed_once(&a).0,
        ZW_DIAL_RETIRED,
        "completed before dispose returned"
    );
    assert_eq!(completed_once(&b).0, ZW_DIAL_RETIRED);
    let after: Vec<Call> = h.calls()[before..].to_vec();
    assert_eq!(
        after,
        [
            Call::Notify {
                pushed: Pushed {
                    health: Health::Starting,
                    readiness: 0
                },
                retire: true
            },
            Call::Clear
        ]
    );
    let vt = vtable_for(&h.plugin);
    let host = b"lwd.example";
    // SAFETY: the plugin is alive; spans valid for the call.
    let code = unsafe {
        vt.dial.unwrap()(
            vt.ctx,
            host.as_ptr(),
            host.len(),
            9067,
            std::ptr::null(),
            0,
            1,
            std::ptr::null_mut(),
            c_dial_done,
        )
    };
    assert_eq!(code, ZW_DIAL_RETIRED);
    assert_eq!(h.plugin.status().phase, Phase::NotRegistered);

    // This test's own handle on the old client would keep it alive, and a new
    // init refuses while it is (ZWT_RC_STOPPING — the 2026-10-07 review's
    // finding 1, pinned by its own test). Let it go first.
    drop(engine);
    assert_eq!(h.init(), ZWT_RC_OK);
    let registers = h
        .calls()
        .iter()
        .filter(|c| matches!(c, Call::Register { .. }))
        .count();
    assert_eq!(registers, 2, "a later init registers afresh");
}

/// **P10**: a bootstrap that never finishes reports `Failed
/// { bootstrap-deadline }` at exactly `BOOTSTRAP_DEADLINE`, readiness stays
/// below 100 throughout, and the next attempts are spaced `BASE × 2ⁿ` after
/// each failure (paused time; no jitter). The halving and the cap are the
/// schedule's own test.
#[test]
fn the_bootstrap_deadline_reports_failed_keeps_readiness_below_ready_and_backs_off() {
    let h = paused_harness(|| {
        FakeEngine::new(
            Readiness::Bootstrapping {
                progress: 0.3,
                blockage: None,
            },
            Boot::Hang,
        )
    });
    assert_eq!(h.init(), ZWT_RC_OK);
    let engine = h.factory.engine(0);
    eventually("three attempts", || engine.boots.lock().unwrap().len() >= 3);
    let boots = engine.boots.lock().unwrap().clone();
    about(
        boots[1] - boots[0],
        BOOTSTRAP_DEADLINE + BOOTSTRAP_RETRY_BASE,
    );
    about(
        boots[2] - boots[1],
        BOOTSTRAP_DEADLINE + BOOTSTRAP_RETRY_BASE * 2,
    );
    let failed: Vec<Pushed> = h
        .notifies()
        .into_iter()
        .map(|(p, _)| p)
        .filter(|p| p.health == Health::Failed)
        .collect();
    assert!(!failed.is_empty());
    assert!(h.notifies().iter().all(|(p, _)| p.readiness < 100));
    assert_eq!(
        failed[0].readiness, 30,
        "FAILED carries the readiness measured"
    );
    assert_eq!(
        h.plugin.status().failure_class,
        Some(CLASS_BOOTSTRAP_DEADLINE)
    );
}

/// **P11**: a paste refused by `dialer-tor`'s bounds or parser fails `init`
/// with its class BEFORE registration (`register` never called); a refused
/// `set_bridges` leaves the running client untouched.
#[test]
fn bridge_lines_are_refused_typed_before_registration_and_never_echoed() {
    let h = harness(FakeEngine::ready);
    let nine = format!("{VANILLA}\n").repeat(9);
    let long = format!("{VANILLA} {}", "A".repeat(250));
    for (paste, class) in [
        (nine.as_str(), dialer_tor::CLASS_TOO_MANY_LINES),
        (long.as_str(), dialer_tor::CLASS_LINE_TOO_LONG),
        ("obfs4 192.0.2.55:38114", dialer_tor::CLASS_PT_UNSUPPORTED),
    ] {
        let rc = h
            .plugin
            .init(&h.tor_dir(), Some(Zeroizing::new(paste.to_owned())), None);
        assert_eq!(rc, ZWT_RC_BRIDGES_REFUSED, "{class}");
        assert_eq!(h.plugin.status().failure_class, Some(class));
    }
    assert!(
        h.calls().is_empty(),
        "register is never called for a refused paste"
    );
    assert_eq!(h.factory.mints.load(Ordering::SeqCst), 0);

    h.init_ready();
    let mints = h.factory.mints.load(Ordering::SeqCst);
    let rc = h
        .plugin
        .set_bridges(Some(Zeroizing::new("obfs4 192.0.2.55:38114".to_owned())));
    assert_eq!(rc, ZWT_RC_BRIDGES_REFUSED);
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        mints,
        "the client is untouched"
    );
    assert_eq!(h.plugin.status().phase, Phase::Ready);
}

/// **P12**: `<tor_dir>/state` and `/cache` exist 0700 after `init`, with the
/// marker; a relative dir is refused before registration; `clear_state` is
/// refused while running, removes only what the plugin made after `dispose`,
/// is idempotent, and refuses — removing nothing — a directory without the
/// marker, a `..` path, and follows no symlink.
#[test]
fn arti_state_lives_in_the_tor_dir_privately_and_clear_state_removes_it() {
    let h = harness(FakeEngine::ready);
    assert_eq!(
        h.plugin.init("relative/tor", None, None),
        ZWT_RC_INVALID_DATA_DIR
    );
    assert!(h.calls().is_empty());

    h.init_ready();
    let tor = std::path::PathBuf::from(h.tor_dir());
    #[cfg(unix)]
    for sub in ["state", "cache"] {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(tor.join(sub))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "{sub}");
    }
    assert!(tor.join(TOR_DIR_MARKER).is_file());
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_NOT_INITIALIZED);

    // A symlinked subtree is unlinked, its target kept.
    let elsewhere = h.dir.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("keep"), b"x").unwrap();
    std::fs::remove_dir_all(tor.join("cache")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&elsewhere, tor.join("cache")).unwrap();

    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_OK);
    assert!(!tor.exists(), "the emptied tor_dir is removed");
    assert!(
        elsewhere.join("keep").is_file(),
        "a symlink is never followed"
    );
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_OK, "idempotent");

    // The wrong directory — an app data root with the OS's own `cache`.
    let root = h.dir.path().join("app-data");
    std::fs::create_dir_all(root.join("cache")).unwrap();
    std::fs::write(root.join("cache").join("os-file"), b"x").unwrap();
    assert_eq!(
        remove_tor_state(root.to_str().unwrap()),
        ZWT_RC_INVALID_DATA_DIR
    );
    assert!(
        root.join("cache").join("os-file").is_file(),
        "nothing removed"
    );
    let climbing = format!("{}/../app-data", h.tor_dir());
    assert_eq!(remove_tor_state(&climbing), ZWT_RC_INVALID_DATA_DIR);
}

/// **P13** (the flow half): every event a real flow emits on the plugin's
/// threads and on the calling thread carries only [`EVENT_FIELDS`] — no path,
/// host, key or bridge — and the flow emitted some (anti-vacuity).
#[test]
fn plugin_events_are_fields_free_and_artis_are_dropped() {
    use crate::log::capture::Capture;
    use crate::log::{EVENT_FIELDS, plugin_dispatch};
    let capture = Capture::default();
    let dispatch = plugin_dispatch(tracing::Dispatch::new(capture.clone()));
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .on_thread_start({
            let dispatch = dispatch.clone();
            move || std::mem::forget(tracing::dispatcher::set_default(&dispatch))
        })
        .build()
        .unwrap();
    let mut h = harness(FakeEngine::ready);
    h.plugin = Plugin::new(
        Deps {
            factory: h.factory.clone(),
            resolver: Box::new(Resolver(Some(h.link.clone()))),
            clock: Box::new(h.clock.clone()),
            jitter: Box::new(NoJitter),
        },
        rt.handle().clone(),
    );
    tracing::dispatcher::with_default(&dispatch, || {
        h.init_ready();
        let engine = h.factory.engine(0);
        engine.script(Connect::Fail(TorDialError::TorAccessFailed));
        let (_, seen) = h.dial(Some("wallet-sync"));
        eventually("completes", || !seen.lock().unwrap().is_empty());
        h.plugin.set_bridges(Some(Zeroizing::new(format!(
            "{VANILLA}\nobfs4 192.0.2.55:1"
        ))));
        h.plugin
            .set_bridges(Some(Zeroizing::new(VANILLA.to_owned())));
        h.plugin.on_paused();
        h.plugin.on_resumed();
        h.plugin.dispose();
    });
    let seen = capture.0.lock().unwrap().clone();
    assert!(
        seen.len() >= 3,
        "anti-vacuity: the flow logged {} events",
        seen.len()
    );
    for event in &seen {
        for field in &event.fields {
            assert!(
                EVENT_FIELDS.contains(&field.as_str()),
                "{} carries field `{field}`, outside the allowlist",
                event.target
            );
        }
    }
    drop(h);
    drop(rt);
}

/// **P16** (plan C3b: the plugin's own twin of the SDK's extraction policy,
/// since `extraction_policy.rs` cannot see this workspace): no `relim-*`
/// dependency, `publish = false` until the extraction, and every locked
/// package resolves from crates.io except this crate and its ONE path
/// dependency, `dialer-tor` (the maintainer's crates.io publish flips it).
#[test]
fn the_plugin_crate_joins_the_extraction_policy() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("\npublish = false\n"));
    let lock = include_str!("../Cargo.lock");
    let packages: Vec<&str> = lock.split("[[package]]").skip(1).collect();
    assert!(
        packages.len() > 100,
        "anti-vacuity: {} packages",
        packages.len()
    );
    let mut local = Vec::new();
    for package in &packages {
        let name = package
            .lines()
            .find_map(|l| l.strip_prefix("name = \""))
            .and_then(|l| l.strip_suffix('"'))
            .expect("every package has a name");
        assert!(
            !name.starts_with("relim"),
            "{name}: a Relim crate in the plugin's graph"
        );
        if !package
            .contains("\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n")
        {
            local.push(name);
        }
    }
    local.sort_unstable();
    assert_eq!(local, ["dialer-tor", "zec_wallet_tor"]);
}

// ─── the review folds ───────────────────────────────────────────────────

/// A dial the plugin ends at its own deadline COUNTS toward the liveness rule
/// (the crypto angle's MEDIUM): on a blackholed path every dial elapses,
/// arti's readiness stays latched at 100, and without the count the wallet
/// would never hear the path died. Three elapses → `Failed { circuits-failing }`
/// with health FAILED, then a rebuild after one base retry interval.
#[test]
fn dial_deadline_elapses_count_toward_the_liveness_rule() {
    let h = paused_harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    for i in 0..LIVENESS_FAILED_DIALS {
        engine.script(Connect::Held);
        let (code, seen) = h.dial(None);
        assert_eq!(code, ZW_DIAL_OK, "dial {i}");
        eventually("the deadline ends the dial", || {
            !seen.lock().unwrap().is_empty()
        });
        assert_eq!(completed_once(&seen).0, ZW_DIAL_REFUSED);
    }
    // Paused time runs the recovery at once, so the wallet's record — the
    // pushes — is what is asserted: FAILED at the measured 100, then the
    // rebuild's retire. (The phase itself is back to Ready by now.)
    eventually("the recovery rebuilds", || {
        h.factory.mints.load(Ordering::SeqCst) == 2
    });
    let pushes = h.notifies();
    let failed = pushes
        .iter()
        .position(|(p, _)| {
            *p == Pushed {
                health: Health::Failed,
                readiness: 100,
            }
        })
        .expect("FAILED at the measured readiness was pushed");
    assert!(
        pushes[failed..]
            .iter()
            .any(|(p, retire)| *retire && p.readiness == 0),
        "the rebuild retired after the FAILED push: {pushes:?}"
    );
}

/// A rebuild asked for while the app is paused stays paused (the arch
/// angle's MEDIUM): `set_bridges` from the background mints the new client
/// DORMANT, the phase stays `Suspended`, and nothing above readiness 0 is
/// pushed until the resume.
#[test]
fn a_rebuild_while_paused_stays_suspended_and_mints_dormant() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    h.clock.set(10);
    h.plugin.on_paused();
    let pushes = h.notifies().len();
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    eventually("the new client is minted", || {
        h.factory.mints.load(Ordering::SeqCst) == 2
    });
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(h.plugin.status().phase, Phase::Suspended);
    assert_eq!(
        *h.factory.engine(1).dormant.lock().unwrap(),
        [DormantMode::Soft]
    );
    assert!(
        h.notifies()[pushes..].iter().all(|(p, _)| p.readiness == 0),
        "a paused plugin pushed readiness above 0: {:?}",
        &h.notifies()[pushes..]
    );
    h.clock.set(11);
    h.plugin.on_resumed();
    eventually("Ready after the resume", || {
        h.plugin.status().phase == Phase::Ready
    });
}

/// Dials in flight TOGETHER that elapse together are ONE piece of liveness
/// evidence, not three (the fold review's MEDIUM): the wallet opens
/// several isolation classes at once, and on a slow-but-working path they all
/// wait on one circuit. Three concurrent elapses must not declare FAILED.
#[test]
fn concurrent_dials_that_elapse_together_count_once() {
    let h = paused_harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    let mut all = Vec::new();
    for _ in 0..LIVENESS_FAILED_DIALS {
        engine.script(Connect::Held);
        let (code, seen) = h.dial(None);
        assert_eq!(code, ZW_DIAL_OK);
        all.push(seen);
    }
    eventually("all three elapse", || {
        all.iter().all(|s| !s.lock().unwrap().is_empty())
    });
    for seen in &all {
        assert_eq!(completed_once(seen).0, ZW_DIAL_REFUSED);
    }
    assert_eq!(h.plugin.status().phase, Phase::Ready);
    assert!(
        h.notifies().iter().all(|(p, _)| p.health != Health::Failed),
        "three concurrent elapses declared FAILED"
    );
    assert_eq!(h.factory.mints.load(Ordering::SeqCst), 1, "no rebuild");
}

/// `dispose` right after a rebuild began returns only once that rebuild's task
/// has ended — its drop of the old client and any mint over `state/` cannot
/// follow a `clear_state` (the fold review's MEDIUM).
#[test]
fn dispose_waits_for_a_rebuild_in_flight() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    assert_eq!(h.plugin.lock().rebuilds_in_flight, 0);
    let mints = h.factory.mints.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        mints,
        "a mint ran after dispose returned"
    );
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_OK);
}

/// External review 2026-10-07, finding 1: UNREGISTERED IS NOT STOPPED. A client
/// that outlives `dispose`'s bound (here a second holder, as a lagging task
/// would be) writes its state when it finally drops, so `clear_state` must not
/// remove the state before then and `init` must not mint a second client over
/// it. Both answer `ZWT_RC_STOPPING` and change nothing; once the client is
/// gone, both go through.
#[test]
fn clear_state_and_init_wait_for_a_client_that_outlived_dispose() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    // What a lagging task holds: the plugin's own handle on the client.
    let lagging = h.plugin.lock().engine.clone().expect("the running client");
    assert_eq!(
        h.plugin.dispose(),
        ZWT_RC_OK,
        "dispose is bounded and returns"
    );
    assert_eq!(h.plugin.status().phase, Phase::NotRegistered);

    let state = std::path::Path::new(&h.tor_dir()).join(crate::constants::STATE_SUBDIR);
    assert!(state.exists(), "the fixture wrote a state dir");
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_STOPPING);
    assert!(state.exists(), "a refused clear_state removed nothing");
    let mints = h.factory.mints.load(Ordering::SeqCst);
    assert_eq!(h.init(), ZWT_RC_STOPPING);
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        mints,
        "a refused init minted nothing"
    );

    drop(lagging);
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_OK);
    assert!(!state.exists(), "the state is gone once the client is");
}

/// The same rule for an `init`'s MINT in flight (the code review's case): a
/// dispose lands while arti is still starting, outlives its bound, and
/// returns. `clear_state` refuses while that mint runs AND while the client it
/// produced (superseded, never installed) is being dropped; once both have
/// ended, the removal goes through.
#[test]
fn clear_state_refuses_while_an_init_mint_is_in_flight() {
    let h = harness(FakeEngine::ready);
    h.factory.hold_mint.store(true, Ordering::SeqCst);
    let (plugin, dir) = (h.plugin.clone(), h.tor_dir());
    let init = std::thread::spawn(move || plugin.init(&dir, None, None));
    eventually("the mint started", || {
        h.factory.mints.load(Ordering::SeqCst) == 1
    });
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK, "bounded, with the mint held");
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_STOPPING);

    h.factory.hold_mint.store(false, Ordering::SeqCst);
    assert_eq!(
        init.join().unwrap(),
        ZWT_RC_DISPOSED,
        "the minted client was superseded"
    );
    eventually("the superseded client finished dropping", || {
        h.plugin.clear_state(&h.tor_dir()) == ZWT_RC_OK
    });
}

/// `Tracked` lowers the live count only AFTER the client's own drop has run —
/// arti writes its state in that drop, and a `Weak` would already read dead
/// there (the security review of the fix).
#[test]
fn a_tracked_client_counts_live_until_its_drop_has_finished() {
    struct DropProbe {
        live: Arc<ClientLedger>,
        seen_during_drop: Arc<AtomicUsize>,
        inner: FakeEngine,
    }
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.seen_during_drop
                .store(self.live.live(), Ordering::SeqCst);
        }
    }
    #[async_trait]
    impl TorEngine for DropProbe {
        async fn bootstrap(&self) -> Result<(), TorDialError> {
            self.inner.bootstrap().await
        }
        fn readiness(&self) -> Readiness {
            self.inner.readiness()
        }
        async fn connect(
            &self,
            host: &str,
            port: u16,
            isolation_key: Option<&str>,
        ) -> Result<Box<dyn AsyncByteStream>, TorDialError> {
            self.inner.connect(host, port, isolation_key).await
        }
        fn set_dormant(&self, mode: DormantMode) {
            self.inner.set_dormant(mode);
        }
    }
    let live = Arc::new(ClientLedger::default());
    let seen = Arc::new(AtomicUsize::new(usize::MAX));
    let probe: Arc<dyn TorEngine> = Arc::new(DropProbe {
        live: live.clone(),
        seen_during_drop: seen.clone(),
        inner: FakeEngine::ready(),
    });
    let tracked = crate::engine::Tracked::wrap(probe, &live);
    assert_eq!(live.live(), 1);
    drop(tracked);
    assert_eq!(
        seen.load(Ordering::SeqCst),
        1,
        "still counted live while the client's drop ran"
    );
    assert_eq!(live.live(), 0, "and not after it");
}

/// Plan §5: while a client is stuck, `clear_state` and `init` answer
/// `ZWT_RC_RESTART_REQUIRED` (only a restart ends it) and change nothing. (How
/// a runtime's overrun becomes "stuck" is `dialer-tor`'s, tested there:
/// `an_overrun_shutdown_stays_counted_and_stuck`.)
#[test]
fn clear_state_and_init_answer_restart_required_after_a_stuck_shutdown() {
    let h = harness(FakeEngine::ready);
    let live = h.plugin.lock().live_clients.clone();
    live.enter().strand(); // a client whose shutdown overran
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_RESTART_REQUIRED);
    assert_eq!(h.init(), ZWT_RC_RESTART_REQUIRED);
    assert_eq!(h.factory.mints.load(Ordering::SeqCst), 0, "nothing minted");
}

/// Plan §5, the crypto audit's fold: a clear refused because a client is stuck
/// is RECORDED, and the next `init` — here a new plugin over the same
/// directory, as after a restart — removes the old state before it creates
/// anything, whether or not the host asks again.
#[test]
fn a_clear_refused_for_a_stuck_client_is_finished_by_the_next_init() {
    use crate::constants::{CLEAR_PENDING_MARKER, STATE_SUBDIR};
    let h = harness(FakeEngine::ready);
    h.init_ready();
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    let tor = std::path::PathBuf::from(h.tor_dir());
    let old_guard = tor.join(STATE_SUBDIR).join("guards-from-before");
    std::fs::write(&old_guard, b"identity").expect("a stand-in guard file");
    h.plugin.lock().live_clients.enter().strand(); // a client stuck past its bound

    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_RESTART_REQUIRED);
    assert!(
        old_guard.exists(),
        "nothing removed while the client may write"
    );
    assert!(
        tor.join(CLEAR_PENDING_MARKER).exists(),
        "the clear is recorded"
    );

    let after_restart = harness(FakeEngine::ready);
    assert_eq!(
        after_restart.plugin.init(&h.tor_dir(), None, None),
        ZWT_RC_OK
    );
    assert!(
        !old_guard.exists(),
        "the old identity is gone before Tor starts"
    );
    assert!(
        !tor.join(CLEAR_PENDING_MARKER).exists(),
        "and the record with it"
    );
}

/// The security review of the 2026-10-07 range: a clear refused with
/// `ZWT_RC_STOPPING` is recorded too — a host that meets it and is then
/// force-quit still gets the reset it asked for at the next start.
#[test]
fn a_clear_refused_while_stopping_is_finished_by_the_next_init() {
    use crate::constants::{CLEAR_PENDING_MARKER, STATE_SUBDIR};
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let lagging = h.plugin.lock().engine.clone().expect("the running client");
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    let tor = std::path::PathBuf::from(h.tor_dir());
    let old_guard = tor.join(STATE_SUBDIR).join("guards-from-before");
    std::fs::write(&old_guard, b"identity").expect("a stand-in guard file");

    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_STOPPING);
    assert!(
        old_guard.exists(),
        "nothing removed while the client may write"
    );
    assert!(
        tor.join(CLEAR_PENDING_MARKER).exists(),
        "the clear is recorded"
    );

    drop(lagging);
    let after_restart = harness(FakeEngine::ready);
    assert_eq!(
        after_restart.plugin.init(&h.tor_dir(), None, None),
        ZWT_RC_OK
    );
    assert!(
        !old_guard.exists(),
        "the old identity is gone before Tor starts"
    );
    assert!(!tor.join(CLEAR_PENDING_MARKER).exists());
}

/// The security review of the 2026-10-07 range: through a SYMLINKED `tor_dir`
/// a refused clear records nothing, because `remove_tor_state` refuses that
/// path — a record there would fail every later `init` for good. And it says
/// so: `ZWT_RC_INVALID_DATA_DIR`, never "restart to finish" for a reset that
/// nothing recorded (the external review of `962a91de7`).
#[cfg(unix)]
#[test]
fn a_refused_clear_records_nothing_through_a_symlinked_tor_dir() {
    use crate::constants::CLEAR_PENDING_MARKER;
    let h = harness(FakeEngine::ready);
    let real = h.dir.path().join("real-tor");
    std::fs::create_dir(&real).expect("the real directory");
    let link = h.dir.path().join("linked-tor");
    std::os::unix::fs::symlink(&real, &link).expect("a symlinked tor_dir");
    let link = link.to_str().expect("utf-8").to_owned();
    assert_eq!(h.plugin.init(&link, None, None), ZWT_RC_OK);
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    h.plugin.lock().live_clients.enter().strand();

    assert_eq!(
        h.plugin.clear_state(&link),
        ZWT_RC_INVALID_DATA_DIR,
        "not recorded, and the answer says so"
    );
    assert!(
        !real.join(CLEAR_PENDING_MARKER).exists(),
        "no record where the removal would refuse"
    );
    let after_restart = harness(FakeEngine::ready);
    assert_eq!(
        after_restart.plugin.init(&link, None, None),
        ZWT_RC_OK,
        "the next start is not locked out"
    );
}

/// The external review of `962a91de7`: a refused clear whose record cannot be
/// SAVED answers `ZWT_RC_INVALID_DATA_DIR` — nothing removed, nothing pending —
/// instead of the "call again later" or "restart to finish" a host would act
/// on while the next `init` found no record and started on the old identity.
/// Both refusals, stopping and stuck.
#[test]
fn a_reset_that_cannot_be_recorded_is_never_reported_as_pending() {
    use crate::constants::{CLEAR_PENDING_MARKER, STATE_SUBDIR};
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let lagging = h.plugin.lock().engine.clone().expect("the running client");
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    let tor = std::path::PathBuf::from(h.tor_dir());
    // A directory where the record goes: the write fails as a full disk would.
    std::fs::create_dir(tor.join(CLEAR_PENDING_MARKER)).expect("a blocked record");

    assert_eq!(
        h.plugin.clear_state(&h.tor_dir()),
        ZWT_RC_INVALID_DATA_DIR,
        "stopping, and the record failed"
    );
    h.plugin.lock().live_clients.enter().strand();
    assert_eq!(
        h.plugin.clear_state(&h.tor_dir()),
        ZWT_RC_INVALID_DATA_DIR,
        "stuck, and the record failed"
    );
    assert!(tor.join(STATE_SUBDIR).exists(), "nothing removed");

    // The control: the blocked record was the cause. Unblocked, the same call
    // records the reset and answers "restart to finish".
    std::fs::remove_dir(tor.join(CLEAR_PENDING_MARKER)).expect("unblock the record");
    assert_eq!(h.plugin.clear_state(&h.tor_dir()), ZWT_RC_RESTART_REQUIRED);
    assert!(
        tor.join(CLEAR_PENDING_MARKER).is_file(),
        "recorded once it can be"
    );
    drop(lagging);
}

/// The code review of the 2026-10-07 folds: a pending-clear record whose
/// directory marker something outside the plugin removed cannot be finished
/// (the removal refuses an unmarked directory), so `init` drops the record
/// instead of refusing on every start.
#[test]
fn a_pending_clear_without_its_directory_marker_does_not_lock_init_out() {
    use crate::constants::{CLEAR_PENDING_MARKER, TOR_DIR_MARKER};
    let h = harness(FakeEngine::ready);
    h.init_ready();
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    let tor = std::path::PathBuf::from(h.tor_dir());
    std::fs::write(tor.join(CLEAR_PENDING_MARKER), b"").expect("a stray record");
    std::fs::remove_file(tor.join(TOR_DIR_MARKER)).expect("the marker removed");

    let after_restart = harness(FakeEngine::ready);
    assert_eq!(
        after_restart.plugin.init(&h.tor_dir(), None, None),
        ZWT_RC_OK,
        "the start is not locked out"
    );
    assert!(
        !tor.join(CLEAR_PENDING_MARKER).exists(),
        "the record is dropped"
    );
}

/// `tor-plugin.md` §12: a rebuild RETIRES the old client and waits for its
/// shutdown to FINISH (the ledger back at zero) before a new client opens
/// `state/`, so two clients never share it (the overlap the audit plan's §6
/// deferred). The old client's shutdown is held open here by an extra ledger
/// entry, as an owned runtime still shutting down holds one.
#[test]
fn a_rebuild_awaits_the_old_clients_shutdown_before_it_mints() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let old = h.factory.engine(0);
    let shutting_down = h.plugin.lock().live_clients.enter();
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    eventually("the old client was retired", || {
        old.retired.load(Ordering::SeqCst)
    });
    std::thread::sleep(REBUILD_POLLS);
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        1,
        "no mint while the old client may still write"
    );
    drop(shutting_down);
    eventually("the rebuild minted once the old client stopped", || {
        h.factory.mints.load(Ordering::SeqCst) == 2
    });
}

/// The reviews of the built diff: a rebuild that finds NO engine (an earlier
/// mint still running, so `s.engine` is empty) waits too — that earlier
/// client may hold `state/` — and mints only once nothing is counted. The
/// held ledger entry stands for the half-built client.
#[test]
fn a_rebuild_that_finds_no_engine_still_waits_for_every_client() {
    let h = harness(FakeEngine::ready);
    h.factory.hold_mint.store(true, Ordering::SeqCst);
    let (plugin, dir) = (h.plugin.clone(), h.tor_dir());
    let init = std::thread::spawn(move || plugin.init(&dir, None, None));
    eventually("the first mint started", || {
        h.factory.mints.load(Ordering::SeqCst) == 1
    });
    let half_built = h.plugin.lock().live_clients.enter();
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    std::thread::sleep(REBUILD_POLLS);
    h.factory.hold_mint.store(false, Ordering::SeqCst);
    let _ = init.join().expect("init returned");
    std::thread::sleep(REBUILD_POLLS);
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        1,
        "the rebuild minted while an earlier client was still counted"
    );
    drop(half_built);
    eventually("the rebuild minted once nothing was counted", || {
        h.factory.mints.load(Ordering::SeqCst) == 2
    });
}

/// The code review of the folds: a mint still in flight may not have counted
/// its client yet (the ledger reads zero), so a rebuild also waits out any
/// OTHER mint — the check and the mint are one step. Nothing is held in the
/// ledger here: only the in-flight count can hold the rebuild back.
#[test]
fn a_rebuild_waits_out_a_mint_that_has_not_counted_its_client_yet() {
    let h = harness(FakeEngine::ready);
    h.factory.hold_mint.store(true, Ordering::SeqCst);
    let (plugin, dir) = (h.plugin.clone(), h.tor_dir());
    let init = std::thread::spawn(move || plugin.init(&dir, None, None));
    eventually("the first mint started", || {
        h.factory.mints.load(Ordering::SeqCst) == 1
    });
    assert_eq!(
        h.plugin.lock().live_clients.live(),
        0,
        "nothing counted yet"
    );
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    std::thread::sleep(REBUILD_POLLS);
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        1,
        "the rebuild minted beside a mint still in flight"
    );
    h.factory.hold_mint.store(false, Ordering::SeqCst);
    let _ = init.join().expect("init returned");
    eventually("the rebuild minted once the other mint ended", || {
        h.factory.mints.load(Ordering::SeqCst) == 2
    });
}

/// §12: a rebuild whose old client's shutdown OVERRAN mints nothing over
/// `state/`: the mint answers `RestartRequired` (`dialer-tor`'s rule, which
/// the fake factory mirrors), and the plugin reports the terminal setup
/// failure instead of a client it cannot start.
#[test]
fn a_rebuild_after_an_overran_shutdown_mints_nothing() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let shutting_down = h.plugin.lock().live_clients.enter();
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_OK);
    std::thread::sleep(Duration::from_millis(100));
    shutting_down.strand(); // its shutdown overran
    eventually("the rebuild gave up", || {
        h.plugin.status().phase == Phase::NotRegistered
    });
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        1,
        "nothing minted over state/"
    );
}

/// §12: `dispose` starts the client's shutdown itself (`retire`), so a late
/// holder cannot keep the client running past the unregistration.
#[test]
fn dispose_retires_the_client() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    assert!(engine.retired.load(Ordering::SeqCst));
}

/// Plan §5 (the security review's H3, made honest by the final code review):
/// while a client is stuck no new client may start, so `set_bridges` REFUSES
/// with `ZWT_RC_RESTART_REQUIRED` before storing anything — never an OK for
/// bridges that are not in use. The current client and its config stay.
#[test]
fn set_bridges_refuses_while_a_client_is_stuck() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let mints = h.factory.mints.load(Ordering::SeqCst);
    let cfg_before = h.plugin.lock().engine_cfg.clone().expect("a config");
    h.plugin.lock().live_clients.enter().strand();
    assert_eq!(h.plugin.set_bridges(None), ZWT_RC_RESTART_REQUIRED);
    let s = h.plugin.lock();
    assert!(
        Arc::ptr_eq(s.engine_cfg.as_ref().expect("a config"), &cfg_before),
        "the bridge config is unchanged"
    );
    assert!(s.engine.is_some(), "the current client is kept");
    drop(s);
    assert_eq!(
        h.factory.mints.load(Ordering::SeqCst),
        mints,
        "nothing minted"
    );
}

/// The final code review of plan §5: a LONG pause normally rebuilds, but no
/// rebuild may start while a client is stuck — the resume must then wake the
/// current client like a short pause, never leave the plugin `Suspended` with
/// nothing coming to end it.
#[test]
fn a_long_resume_while_a_client_is_stuck_wakes_the_current_client() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let engine = h.factory.engine(0);
    h.plugin.lock().live_clients.enter().strand();
    h.clock.set(1_000);
    h.plugin.on_paused();
    h.clock.set(1_000 + 40 * 60);
    h.plugin.on_resumed();
    assert_ne!(h.plugin.status().phase, Phase::Suspended, "the plugin woke");
    assert_eq!(
        *engine.dormant.lock().unwrap(),
        [DormantMode::Soft, DormantMode::Normal],
        "the current client was woken, not replaced"
    );
    assert_eq!(h.factory.mints.load(Ordering::SeqCst), 1, "no rebuild");
}

/// Plan §5, the REAL factory, offline (an unbootstrapped arti client needs no
/// network): it mints over a temp Tor directory, counts the client while it
/// exists, and its runtime's shutdown — started from an ASYNC thread, as the
/// plugin's drops are — releases the count without a panic.
#[test]
fn the_real_arti_factory_counts_its_client_until_the_runtime_shut_down() {
    use crate::engine::ArtiFactory;
    let dir = tempfile::tempdir().expect("temp dir");
    let state_dir = dir.path().join("state");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&state_dir).expect("state dir");
    std::fs::create_dir_all(&cache_dir).expect("cache dir");
    let client = dialer_tor::tor_config(
        &state_dir,
        &cache_dir,
        dialer_tor::BridgeLines::parse("").expect("no bridges"),
    )
    .expect("an arti config");
    let cfg = EngineConfig {
        state_dir,
        cache_dir,
        client,
    };
    let live = Arc::new(ClientLedger::default());
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("the test's plugin-like runtime");
    rt.block_on(async {
        // Minted AND wrapped exactly as the plugin does (`Tracked::wrap`), so a
        // client counted twice — once by its runtime, once by the wrapper —
        // shows here as a count of two.
        let minted = ArtiFactory
            .mint(&cfg, &live)
            .await
            .expect("an offline mint");
        let engine = crate::engine::Tracked::wrap(minted, &live);
        assert_eq!(live.live(), 1, "the client is counted once while it exists");
        drop(engine); // on an async thread, like the plugin's own drops
    });
    eventually("the client's runtime shut down", || live.live() == 0);
    assert_eq!(live.stuck(), 0);
}

/// A vtable verb called from INSIDE a completion — a contract violation the
/// wallet's header forbids — is REFUSED, never a self-deadlock on the one lock
/// (the arch angle's LOW: the guard had no test).
#[test]
fn a_verb_called_from_inside_a_completion_is_refused_not_deadlocked() {
    let h = harness(FakeEngine::ready);
    h.init_ready();
    let inner: Arc<Mutex<Option<u32>>> = Arc::default();
    let (plugin, sink) = (h.plugin.clone(), inner.clone());
    let code = h.plugin.dial(
        "lwd.example".into(),
        9067,
        None,
        Box::new(move |_, _| {
            let again = plugin.dial("lwd.example".into(), 9067, None, Box::new(|_, _| {}));
            *sink.lock().unwrap() = Some(again);
        }),
    );
    assert_eq!(code, ZW_DIAL_OK);
    eventually("the completion ran", || inner.lock().unwrap().is_some());
    assert_eq!(*inner.lock().unwrap(), Some(ZW_DIAL_REFUSED));
}

/// A mid-stream timeout completes REFUSED, never the fallback code TIMEOUT:
/// an exit can end a stream with RELAY_END TIMEOUT, which arti surfaces as an
/// io `TimedOut` (the security angle's L1).
#[test]
fn a_mid_stream_timeout_is_refused_not_a_fallback_code() {
    use crate::trampoline::io_code;
    for kind in [
        std::io::ErrorKind::TimedOut,
        std::io::ErrorKind::ConnectionReset,
        std::io::ErrorKind::BrokenPipe,
    ] {
        assert_eq!(
            io_code(&std::io::Error::from(kind)),
            ZW_DIAL_REFUSED,
            "{kind:?}"
        );
    }
}

/// The status reaches the host's callback off the lock, and `dispose` returns
/// only after the last one was delivered — no callback after dispose (a Dart
/// listener closed at dispose must never be called).
#[test]
fn the_status_reaches_the_host_and_none_arrives_after_dispose() {
    let h = harness(FakeEngine::ready);
    let seen: Arc<Mutex<Vec<Phase>>> = Arc::default();
    let sink = seen.clone();
    // A host callback that takes a moment (a real one crosses into Dart): a
    // dispose that did not WAIT for the outbox would return before the last
    // status landed — the mutant this test first let through.
    let rc = h.plugin.init(
        &h.tor_dir(),
        None,
        Some(Box::new(move |s| {
            std::thread::sleep(Duration::from_millis(20));
            sink.lock().unwrap().push(s.phase);
        })),
    );
    assert_eq!(rc, ZWT_RC_OK);
    eventually("Ready is delivered", || {
        seen.lock().unwrap().contains(&Phase::Ready)
    });
    assert_eq!(h.plugin.dispose(), ZWT_RC_OK);
    let at_dispose = seen.lock().unwrap().len();
    assert_eq!(seen.lock().unwrap().last(), Some(&Phase::NotRegistered));
    h.plugin.on_paused();
    h.plugin.on_resumed();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(seen.lock().unwrap().len(), at_dispose);
}
