//! The plugin's side of the wallet's vtable (`tor-plugin.md` §3.3): `dial`,
//! `read`, `write`, `close` over the current engine, and THE lock.
//!
//! **One lock.** Every state a verb consults — the phase, the generation, the
//! engine, the op table and the streams — lives in [`PluginState`] under
//! [`Plugin::state`]. No per-stream or per-op lock (the spec forbids them: the
//! natural throughput optimisation reopens the race the pause, rebuild and
//! close rules close).
//!
//! **The op record** is one-way: `Pending → Closed` (close, rebuild, dispose),
//! then removed and completed by its OWNING task — never by the closing thread.
//! A completion is delivered UNDER the lock, which is what makes "never touches
//! an SDK buffer after `close` returns" hold: `close` takes the same lock, so a
//! read either copied before it or finds its op `Closed` and copies nothing.
//! That is deadlock-free because the wallet holds none of its locks across a
//! plugin verb and its completion only stores and wakes (the header's
//! "completions … never re-enter"); a thread-local makes a re-entrant verb
//! REFUSED instead of a self-deadlock, belt and braces.
//!
//! **Staging.** The plugin never hands an SDK pointer to arti: a write is
//! copied at the edge, a read lands in a plugin-owned buffer and is copied to
//! the SDK's under the lock (the security angle's HIGH).
//!
//! **Every write flushes before it completes** — arti's `DataStream` buffers
//! and the SDK's `poll_flush` is a no-op (the header's WRITE COMPLETION MEANS
//! SENT).

use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use dialer_tor::{AsyncByteStream, Readiness, TorDialError};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::runtime::Handle;
use tokio::sync::{Notify, mpsc, watch};
use zeroize::Zeroizing;

use crate::abi::{
    ZW_DIAL_NOT_READY, ZW_DIAL_OK, ZW_DIAL_REFUSED, ZW_DIAL_RETIRED, ZW_EXPOSURE_HIDDEN,
    ZW_ISOLATION_SUPPORTED, ZW_NET_DIALER_AUTH_TOKEN_BYTES, ZW_TRANSPORT_NAME_MAX_BYTES,
    ZwNetDialerV1, ZwTransportDescriptor,
};
use crate::bootstrap::{Backoff, Jitter};
use crate::constants::{
    DIAL_DEADLINE, LIVENESS_FAILED_DIALS, STAGING_BUFFER_BYTES, TRANSPORT_NAME,
};
use crate::dial_codes::dial_code;
use crate::engine::{EngineConfig, EngineFactory, TorEngine};
use crate::status::{Blockage, Phase, Status};
use crate::watcher::{Debounce, Health, Pushed};
use dialer_tor::ClientLedger;

/// The mutation token `register` minted; zeroized on drop.
pub type Token = Zeroizing<[u8; ZW_NET_DIALER_AUTH_TOKEN_BYTES]>;

/// A dial's completion: `(code, stream handle)`.
pub type DialDone = Box<dyn FnOnce(u32, u64) + Send>;
/// A write's completion: `(code, bytes consumed)`.
pub type WriteDone = Box<dyn FnOnce(u32, usize) + Send>;
/// A read's completion. Called under the lock with the bytes to copy into the
/// SDK's buffer (never more than the `cap` the read was accepted with).
pub type ReadDone = Box<dyn for<'a> FnOnce(ReadOutcome<'a>) + Send>;

/// What a read completes with.
#[derive(Debug)]
pub enum ReadOutcome<'a> {
    /// `ZW_DIAL_OK` with these bytes (empty = end of stream).
    Data(&'a [u8]),
    /// Any other code, no bytes.
    Code(u32),
}

/// The wallet's three verbs as the plugin uses them. Production calls the
/// resolved C functions (`abi.rs`); the tests record.
pub trait WalletLink: Send + Sync {
    /// `zec_wallet_register_net_dialer`: the token, or the wallet's `ZW_RC_*`.
    fn register(
        &self,
        dialer: &ZwNetDialerV1,
        descriptor: &ZwTransportDescriptor,
    ) -> Result<Token, i32>;
    /// `zec_wallet_net_dialer_notify`.
    fn notify(&self, token: &Token, descriptor: &ZwTransportDescriptor, retire: bool) -> i32;
    /// `zec_wallet_update_net_dialer(auth, NULL, NULL)` — CLEAR.
    fn clear(&self, token: &Token) -> i32;
}

/// How the wallet is found (D8). `None` = the wallet image is not loaded.
pub trait LinkResolver: Send + Sync {
    /// Resolve the three verbs together.
    fn resolve(&self) -> Option<Arc<dyn WalletLink>>;
}

/// A clock that COUNTS device sleep (`tor-plugin.md` §3.4 "The clock").
/// `None` = this platform has none, and every resume rebuilds.
pub trait PauseClock: Send + Sync {
    /// Now, on the sleep-counting clock.
    fn now(&self) -> Option<Duration>;
}

/// Where status changes go (the host's Dart callback). Never called under the
/// lock: an outbox task drains to it.
pub type StatusSink = Box<dyn Fn(&Status) + Send + Sync>;

/// What the plugin is built over.
pub struct Deps {
    /// Mints engines.
    pub factory: Arc<dyn EngineFactory>,
    /// Finds the wallet.
    pub resolver: Box<dyn LinkResolver>,
    /// The pause clock.
    pub clock: Box<dyn PauseClock>,
    /// The backoff's and the watcher's jitter.
    pub jitter: Box<dyn Jitter>,
}

pub(crate) struct Registration {
    pub(crate) link: Arc<dyn WalletLink>,
    pub(crate) token: Token,
}

enum OpState {
    Pending,
    Closed,
}

enum Done {
    Dial(DialDone),
    Read(ReadDone),
    Write(WriteDone),
}

impl Done {
    /// Complete with a non-OK code (no bytes, no stream).
    fn with_code(self, code: u32) {
        match self {
            Self::Dial(f) => f(code, 0),
            Self::Read(f) => f(ReadOutcome::Code(code)),
            Self::Write(f) => f(code, 0),
        }
    }
}

struct Op {
    stream: Option<u64>,
    state: OpState,
    cancel: Arc<Notify>,
    done: Done,
}

type Reader = ReadHalf<Box<dyn AsyncByteStream>>;
type Writer = WriteHalf<Box<dyn AsyncByteStream>>;

/// A stream's two halves. A half is TAKEN by the op that uses it — taking it
/// IS the accept — so a second outstanding read or write finds it gone and is
/// refused synchronously.
struct StreamSlot {
    reader: Option<Reader>,
    writer: Option<Writer>,
}

/// Everything under the one lock.
pub(crate) struct PluginState {
    pub(crate) phase: Phase,
    pub(crate) phase_before_pause: Phase,
    /// Starts at 1, bumped by every rebuild and dispose, never reset.
    pub(crate) generation: u64,
    pub(crate) engine: Option<Arc<dyn TorEngine>>,
    pub(crate) engine_cfg: Option<Arc<EngineConfig>>,
    pub(crate) tor_dir: Option<PathBuf>,
    pub(crate) registration: Option<Registration>,
    ops: HashMap<u64, Op>,
    streams: HashMap<u64, StreamSlot>,
    next_op: u64,
    next_handle: u64,
    pub(crate) debounce: Debounce,
    pub(crate) blockage: Option<Blockage>,
    pub(crate) failure_class: Option<&'static str>,
    pub(crate) paused_at: Option<Duration>,
    pub(crate) backoff: Backoff,
    pub(crate) liveness_failures: u32,
    /// The op key from which a failure may count toward liveness again.
    pub(crate) liveness_mark: u64,
    /// Mints (an `init`'s or a rebuild's) still running: dispose waits for
    /// them (bounded), and `clear_state` and `init` refuse while any is,
    /// because a mint writes over `state/` and its client's drop writes again.
    pub(crate) rebuilds_in_flight: usize,
    /// Clients that may still write under the Tor directory: `dialer-tor`'s
    /// ledger, counting each from the moment its own runtime exists until that
    /// runtime has shut down, which ends every task arti started for it
    /// (`tor-plugin.md` §12). `clear_state` and `init` refuse until this is
    /// zero, and answer "restart required" once a client is recorded stuck
    /// (2026-10-07 external review, finding 1: unregistered is not stopped).
    pub(crate) live_clients: Arc<ClientLedger>,
    pub(crate) outbox: Option<mpsc::UnboundedSender<Status>>,
    pub(crate) outbox_done: Option<std::sync::mpsc::Receiver<()>>,
    pub(crate) last_status: Option<Status>,
}

/// The plugin: the process-lifetime trampoline behind the vtable's `ctx`, and
/// everything the lifecycle drives.
pub struct Plugin {
    pub(crate) state: Mutex<PluginState>,
    pub(crate) drained: Condvar,
    pub(crate) deps: Deps,
    pub(crate) runtime: Handle,
    pub(crate) retry: Notify,
    pub(crate) resumed: Notify,
    pub(crate) generation_tx: watch::Sender<u64>,
}

thread_local! {
    /// Set while this thread is inside a completion into the SDK: a verb the
    /// SDK calls from there is REFUSED rather than deadlocking on the lock.
    static IN_SDK_CALLBACK: Cell<bool> = const { Cell::new(false) };
}

/// Run a completion with [`IN_SDK_CALLBACK`] set.
fn in_callback(f: impl FnOnce()) {
    IN_SDK_CALLBACK.with(|c| c.set(true));
    f();
    IN_SDK_CALLBACK.with(|c| c.set(false));
}

/// Whether a vtable verb arrived re-entrantly, from inside a completion.
pub(crate) fn reentrant() -> bool {
    IN_SDK_CALLBACK.with(Cell::get)
}

/// The descriptor for a pushed pair: "Tor", isolation SUPPORTED, exposure
/// HIDDEN, the name ZERO-FILLED beyond its length (the header's "initialise
/// the whole struct" — the wallet copies it by value). P3.
pub fn descriptor(pushed: Pushed) -> ZwTransportDescriptor {
    let mut name = [0u8; ZW_TRANSPORT_NAME_MAX_BYTES];
    name[..TRANSPORT_NAME.len()].copy_from_slice(TRANSPORT_NAME.as_bytes());
    ZwTransportDescriptor {
        name,
        name_len: TRANSPORT_NAME.len() as u32,
        readiness: pushed.readiness,
        isolation: ZW_ISOLATION_SUPPORTED,
        exposure: ZW_EXPOSURE_HIDDEN,
        health: pushed.health.code(),
    }
}

/// A stream fault's code, the op already known live (§3.3 "Mid-stream
/// failures"): always REFUSED ("this stream will not carry more"). Never
/// TIMEOUT: an EXIT can end a stream with RELAY_END reason TIMEOUT, which arti
/// surfaces as an io `TimedOut` (`tor-cell` `msg.rs:568`), and the table keeps
/// every exit- or destination-controlled failure off the fallback codes (the
/// security angle's L1). Never UNREACHABLE — Relim's arm is not copied.
pub(crate) fn io_code(_error: &std::io::Error) -> u32 {
    ZW_DIAL_REFUSED
}

/// Whether a dial failure counts toward the liveness rule: the kinds the
/// device, the path to Tor or the Tor network produce — which include a guard
/// or an on-path censor (`LocalNetworkError` is a guard-channel IO error,
/// `TorAccessFailed` a relay or bridge not answering). Never an exit- or
/// destination-side kind. See [`LIVENESS_FAILED_DIALS`].
fn counts_against_liveness(error: &TorDialError) -> bool {
    matches!(
        error,
        TorDialError::TorAccessFailed
            | TorDialError::LocalNetworkError
            | TorDialError::TorNetworkTimeout
    )
}

/// Removes an op the verb recorded but never ACCEPTED (a panic between the
/// record and the spawn): silently, with no completion, because the verb then
/// answers non-zero and the header forbids a completion after that (the
/// security angle's A4). Once accepted it becomes the owning task's drop
/// guard: an op still in the table when the task ends is completed RETIRED.
struct OpGuard {
    plugin: Arc<Plugin>,
    key: Option<u64>,
    accepted: bool,
}

impl OpGuard {
    fn take(mut self) -> u64 {
        self.key.take().expect("an op guard is taken once")
    }
}

impl Drop for OpGuard {
    fn drop(&mut self) {
        let Some(key) = self.key.take() else { return };
        let mut s = self.plugin.lock();
        if let Some(op) = s.ops.remove(&key) {
            op.cancel.notify_one();
            if self.accepted {
                in_callback(|| op.done.with_code(ZW_DIAL_RETIRED));
            }
        }
        self.plugin.after_removal(&s);
    }
}

#[cfg(test)]
thread_local! {
    /// P17b's plant: a panic between an op's record and its accept. Per
    /// THREAD, so a test running beside P17b is never hit by it (the verb runs
    /// on the caller's thread).
    pub(crate) static PANIC_AFTER_RECORD: Cell<bool> = const { Cell::new(false) };
}

impl Plugin {
    /// A plugin over `deps`, its tasks on `runtime`. Idle until `init`.
    pub fn new(deps: Deps, runtime: Handle) -> Arc<Self> {
        let (generation_tx, _) = watch::channel(1);
        Arc::new(Self {
            state: Mutex::new(PluginState {
                phase: Phase::Idle,
                phase_before_pause: Phase::Idle,
                generation: 1,
                engine: None,
                engine_cfg: None,
                tor_dir: None,
                registration: None,
                ops: HashMap::new(),
                streams: HashMap::new(),
                next_op: 1,
                next_handle: 1,
                debounce: Debounce::default(),
                blockage: None,
                failure_class: None,
                paused_at: None,
                backoff: Backoff::default(),
                liveness_failures: 0,
                liveness_mark: 0,
                rebuilds_in_flight: 0,
                live_clients: Arc::new(ClientLedger::default()),
                outbox: None,
                outbox_done: None,
                last_status: None,
            }),
            drained: Condvar::new(),
            deps,
            runtime,
            retry: Notify::new(),
            resumed: Notify::new(),
            generation_tx,
        })
    }

    /// The one lock. A poisoned lock is recovered: every critical section
    /// leaves the state consistent before any call that could panic.
    pub(crate) fn lock(&self) -> MutexGuard<'_, PluginState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The current status.
    pub fn status(&self) -> Status {
        status_of(&self.lock())
    }

    // ─── the vtable verbs ────────────────────────────────────────────────

    /// `dial`, after the edge has validated and copied the host and the key.
    /// Returns the synchronous code: `ZW_DIAL_OK` = accepted, exactly one
    /// completion follows; anything else = refused now, none follows.
    pub fn dial(
        self: &Arc<Self>,
        host: String,
        port: u16,
        isolation_key: Option<String>,
        done: DialDone,
    ) -> u32 {
        if reentrant() {
            return ZW_DIAL_REFUSED;
        }
        let (key, cancel, engine) = {
            let mut s = self.lock();
            match s.phase {
                Phase::Ready => {}
                Phase::NotRegistered | Phase::Idle => return ZW_DIAL_RETIRED,
                Phase::Bootstrapping | Phase::Failed | Phase::Suspended => {
                    return ZW_DIAL_NOT_READY;
                }
            }
            let Some(engine) = s.engine.clone() else {
                return ZW_DIAL_NOT_READY;
            };
            // The LIVE re-read: the phase follows what was PUSHED, which a
            // drop's confirmations delay; a dial never trusts it alone.
            if !matches!(engine.readiness(), Readiness::Ready) {
                return ZW_DIAL_NOT_READY;
            }
            let key = s.next_op;
            s.next_op += 1;
            let cancel = Arc::new(Notify::new());
            s.ops.insert(
                key,
                Op {
                    stream: None,
                    state: OpState::Pending,
                    cancel: cancel.clone(),
                    done: Done::Dial(done),
                },
            );
            (key, cancel, engine)
        };
        let mut guard = OpGuard {
            plugin: self.clone(),
            key: Some(key),
            accepted: false,
        };
        #[cfg(test)]
        if PANIC_AFTER_RECORD.with(Cell::get) {
            panic!("planted: a panic between the record and the accept");
        }
        guard.accepted = true;
        self.runtime.spawn(async move {
            let connect = tokio::time::timeout(
                DIAL_DEADLINE,
                engine.connect(&host, port, isolation_key.as_deref()),
            );
            let outcome = tokio::select! {
                r = connect => Some(r),
                () = cancel.notified() => None,
            };
            let plugin = guard.plugin.clone();
            plugin.finish_dial(guard.take(), outcome);
        });
        ZW_DIAL_OK
    }

    #[allow(clippy::type_complexity)]
    fn finish_dial(
        self: &Arc<Self>,
        key: u64,
        outcome: Option<
            Result<Result<Box<dyn AsyncByteStream>, TorDialError>, tokio::time::error::Elapsed>,
        >,
    ) {
        let mut s = self.lock();
        let Some(op) = s.ops.remove(&key) else {
            return;
        };
        let Done::Dial(done) = op.done else {
            unreachable!("a dial key holds a dial")
        };
        let (code, handle) = match (op.state, outcome) {
            // Closed by a rebuild, a dispose, or cancelled: RETIRED whatever
            // arti said, and a stream it produced is dropped here (P7).
            (OpState::Closed, _) | (OpState::Pending, None) => (ZW_DIAL_RETIRED, 0),
            // The plugin's own deadline: a code IT chose, before the wallet's.
            // It also counts toward liveness: arti bounds a stream's BEGIN at
            // its own `connect_timeout` (10 s, `arti-client` `client.rs:1566`)
            // and reports that as an exit/remote kind, so a dial still pending
            // at 20 s spent the rest acquiring a circuit — Tor-side time no
            // destination controls (the crypto angle's MEDIUM: on a
            // blackholed path this is the COMMON outcome, and uncounted it
            // left health READY over a dead path).
            (OpState::Pending, Some(Err(_elapsed))) => {
                tracing::warn!(
                    class = "dial-deadline",
                    "a tor dial did not connect in time"
                );
                self.note_liveness_failure(&mut s, key);
                (ZW_DIAL_REFUSED, 0)
            }
            (OpState::Pending, Some(Ok(Err(error)))) => {
                if counts_against_liveness(&error) {
                    self.note_liveness_failure(&mut s, key);
                }
                (dial_code(&error), 0)
            }
            (OpState::Pending, Some(Ok(Ok(stream)))) => {
                s.liveness_failures = 0;
                let handle = s.next_handle;
                s.next_handle += 1;
                let (reader, writer) = tokio::io::split(stream);
                s.streams.insert(
                    handle,
                    StreamSlot {
                        reader: Some(reader),
                        writer: Some(writer),
                    },
                );
                (ZW_DIAL_OK, handle)
            }
        };
        in_callback(|| done(code, handle));
        self.after_removal(&s);
    }

    /// `read`: up to `cap` bytes. `done` copies into the SDK's buffer.
    pub fn read(self: &Arc<Self>, stream: u64, cap: usize, done: ReadDone) -> u32 {
        if reentrant() {
            return ZW_DIAL_REFUSED;
        }
        let (key, cancel, mut reader) = {
            let mut s = self.lock();
            if matches!(s.phase, Phase::NotRegistered | Phase::Idle) {
                return ZW_DIAL_RETIRED;
            }
            let Some(slot) = s.streams.get_mut(&stream) else {
                return ZW_DIAL_RETIRED;
            };
            let Some(reader) = slot.reader.take() else {
                return ZW_DIAL_REFUSED;
            };
            let (key, cancel) = s.record(Some(stream), Done::Read(done));
            (key, cancel, reader)
        };
        let guard = OpGuard {
            plugin: self.clone(),
            key: Some(key),
            accepted: true,
        };
        self.runtime.spawn(async move {
            let mut staging = vec![0u8; cap.min(STAGING_BUFFER_BYTES)];
            let outcome = tokio::select! {
                r = reader.read(&mut staging) => Some(r),
                () = cancel.notified() => None,
            };
            let plugin = guard.plugin.clone();
            plugin.finish_read(guard.take(), stream, reader, outcome, &staging);
        });
        ZW_DIAL_OK
    }

    fn finish_read(
        &self,
        key: u64,
        stream: u64,
        reader: Reader,
        outcome: Option<std::io::Result<usize>>,
        staging: &[u8],
    ) {
        let mut s = self.lock();
        let Some(op) = s.ops.remove(&key) else {
            return;
        };
        let Done::Read(done) = op.done else {
            unreachable!("a read key holds a read")
        };
        match (op.state, outcome) {
            (OpState::Pending, Some(Ok(n))) => {
                if let Some(slot) = s.streams.get_mut(&stream) {
                    slot.reader = Some(reader);
                }
                in_callback(|| done(ReadOutcome::Data(&staging[..n])));
            }
            (OpState::Pending, Some(Err(e))) => {
                in_callback(|| done(ReadOutcome::Code(io_code(&e))));
            }
            (OpState::Closed, _) | (OpState::Pending, None) => {
                in_callback(|| done(ReadOutcome::Code(ZW_DIAL_RETIRED)));
            }
        }
        self.after_removal(&s);
    }

    /// `write`: `data` is the edge's copy of the SDK's bytes (never the SDK's
    /// pointer). Completes once the bytes are FLUSHED into arti.
    pub fn write(self: &Arc<Self>, stream: u64, data: Vec<u8>, done: WriteDone) -> u32 {
        if reentrant() {
            return ZW_DIAL_REFUSED;
        }
        let (key, cancel, mut writer) = {
            let mut s = self.lock();
            if matches!(s.phase, Phase::NotRegistered | Phase::Idle) {
                return ZW_DIAL_RETIRED;
            }
            let Some(slot) = s.streams.get_mut(&stream) else {
                return ZW_DIAL_RETIRED;
            };
            let Some(writer) = slot.writer.take() else {
                return ZW_DIAL_REFUSED;
            };
            let (key, cancel) = s.record(Some(stream), Done::Write(done));
            (key, cancel, writer)
        };
        let guard = OpGuard {
            plugin: self.clone(),
            key: Some(key),
            accepted: true,
        };
        self.runtime.spawn(async move {
            let send = async {
                let n = writer.write(&data).await?;
                writer.flush().await?;
                Ok::<usize, std::io::Error>(n)
            };
            let outcome = tokio::select! {
                r = send => Some(r),
                () = cancel.notified() => None,
            };
            let plugin = guard.plugin.clone();
            plugin.finish_write(guard.take(), stream, writer, outcome);
        });
        ZW_DIAL_OK
    }

    fn finish_write(
        &self,
        key: u64,
        stream: u64,
        writer: Writer,
        outcome: Option<std::io::Result<usize>>,
    ) {
        let mut s = self.lock();
        let Some(op) = s.ops.remove(&key) else {
            return;
        };
        let Done::Write(done) = op.done else {
            unreachable!("a write key holds a write")
        };
        let (code, n) = match (op.state, outcome) {
            (OpState::Pending, Some(Ok(n))) => {
                if let Some(slot) = s.streams.get_mut(&stream) {
                    slot.writer = Some(writer);
                }
                (ZW_DIAL_OK, n)
            }
            (OpState::Pending, Some(Err(e))) => (io_code(&e), 0),
            (OpState::Closed, _) | (OpState::Pending, None) => (ZW_DIAL_RETIRED, 0),
        };
        in_callback(|| done(code, n));
        self.after_removal(&s);
    }

    /// `close`: idempotent. Marks every op on the stream `Closed` and drops
    /// the halves no op holds; each op then completes RETIRED from its own
    /// task, and no SDK buffer for the stream is touched after this returns.
    pub fn close(&self, stream: u64) {
        if reentrant() {
            return;
        }
        let mut s = self.lock();
        s.streams.remove(&stream);
        for op in s.ops.values_mut() {
            if op.stream == Some(stream) && matches!(op.state, OpState::Pending) {
                op.state = OpState::Closed;
                op.cancel.notify_one();
            }
        }
    }

    // ─── shared by the lifecycle ─────────────────────────────────────────

    /// After an op left the table: wake a dispose waiting for the drain.
    fn after_removal(&self, s: &PluginState) {
        if s.ops.is_empty() {
            self.drained.notify_all();
        }
    }

    /// Mark every outstanding op `Closed` and drop every stream — the half of
    /// a rebuild and a dispose that must happen in the same critical section
    /// as the generation bump.
    pub(crate) fn close_all(s: &mut PluginState) {
        s.streams.clear();
        for op in s.ops.values_mut() {
            if matches!(op.state, OpState::Pending) {
                op.state = OpState::Closed;
                op.cancel.notify_one();
            }
        }
    }

    /// How many ops are outstanding (dispose's drain).
    pub(crate) fn outstanding(s: &PluginState) -> usize {
        s.ops.len()
    }

    /// Push a pair to the wallet OUTSIDE the watcher's rule (a lifecycle
    /// transition), record it, and emit the status.
    pub(crate) fn push(&self, s: &mut PluginState, pushed: Pushed, retire: bool) {
        s.debounce.record(pushed);
        self.send(s, pushed, retire);
        self.emit(s);
    }

    /// The `notify` itself. A non-zero rc is logged, never retried (the
    /// wallet's rule: a non-zero verb return is fatal to that call).
    pub(crate) fn send(&self, s: &PluginState, pushed: Pushed, retire: bool) {
        if let Some(reg) = &s.registration {
            let rc = reg.link.notify(&reg.token, &descriptor(pushed), retire);
            if rc != 0 {
                tracing::warn!(rc, "the wallet refused a readiness push");
            }
        }
    }

    /// Queue the status for the host if it changed. Never calls the host here.
    pub(crate) fn emit(&self, s: &mut PluginState) {
        let now = status_of(s);
        if s.last_status.as_ref() != Some(&now) {
            s.last_status = Some(now.clone());
            if let Some(outbox) = &s.outbox {
                let _ = outbox.send(now);
            }
        }
    }

    /// One dial failure the liveness rule counts. Only while `Ready`: a count
    /// earned while suspended or bootstrapping must not trip the rule on the
    /// first failure after a resume (the crypto angle's LOW).
    ///
    /// And only EVIDENCE OVER TIME: a failure counts only if its dial was
    /// accepted AFTER the previous counted failure (op keys are monotonic, so
    /// `liveness_mark` is the next key at that moment). Dials in flight
    /// together — the wallet opens several isolation classes at once — fail
    /// together on one slow circuit and count ONCE; counted per dial, three
    /// concurrent elapses on a slow-but-working bridge would declare FAILED
    /// and throw away the circuit arti was about to finish (the fold
    /// review's MEDIUM).
    fn note_liveness_failure(self: &Arc<Self>, s: &mut PluginState, op_key: u64) {
        if s.phase != Phase::Ready || op_key < s.liveness_mark {
            return;
        }
        s.liveness_mark = s.next_op;
        s.liveness_failures += 1;
        if s.liveness_failures >= LIVENESS_FAILED_DIALS {
            self.declare_path_failed(s);
        }
    }

    /// The liveness rule tripped: `Failed` with its class, FAILED pushed at
    /// once with the readiness MEASURED, and a rebuild after ONE base retry
    /// interval — never the doubling bootstrap backoff, so the plugin's own
    /// schedule does not hold FAILED for a whole patience window by itself.
    /// Under a `Preferred` wallet FAILED is switch-eligible after that window
    /// (ADR-0553 v4), which is the maintainer's ruling for a declared failure;
    /// under `Required` it is fail-closed.
    fn declare_path_failed(self: &Arc<Self>, s: &mut PluginState) {
        let measured = s
            .engine
            .as_ref()
            .map_or(0, |e| crate::watcher::readiness_percent(&e.readiness()));
        s.phase = Phase::Failed;
        s.failure_class = Some(crate::constants::CLASS_CIRCUITS_FAILING);
        s.liveness_failures = 0;
        tracing::warn!(
            class = crate::constants::CLASS_CIRCUITS_FAILING,
            "tor dials keep failing; rebuilding"
        );
        self.push(
            s,
            Pushed {
                health: Health::Failed,
                readiness: measured,
            },
            false,
        );
        let delay = self
            .deps
            .jitter
            .stretch(crate::constants::BOOTSTRAP_RETRY_BASE);
        let generation = s.generation;
        let plugin = self.clone();
        self.runtime
            .spawn(async move { plugin.recover(generation, delay).await });
    }
}

impl PluginState {
    /// True when no Tor client of this plugin can write under the Tor
    /// directory any more: no mint in flight, and every client minted has
    /// FINISHED — for the production client, its own runtime has shut down,
    /// which ends arti's background tasks too. Unregistered alone is not
    /// enough (2026-10-07 external review, finding 1; plan §5).
    pub(crate) fn state_writers_quiet(&self) -> bool {
        self.rebuilds_in_flight == 0 && self.engine.is_none() && self.live_clients.live() == 0
    }

    /// True when a client's shutdown overran its bound: it stays counted for
    /// the life of the process, and only a restart ends it.
    pub(crate) fn clients_stuck(&self) -> bool {
        self.live_clients.stuck() > 0
    }

    fn record(&mut self, stream: Option<u64>, done: Done) -> (u64, Arc<Notify>) {
        let key = self.next_op;
        self.next_op += 1;
        let cancel = Arc::new(Notify::new());
        self.ops.insert(
            key,
            Op {
                stream,
                state: OpState::Pending,
                cancel: cancel.clone(),
                done,
            },
        );
        (key, cancel)
    }
}

/// The status of a state: the readiness is the one LAST PUSHED.
pub(crate) fn status_of(s: &PluginState) -> Status {
    Status {
        phase: s.phase,
        readiness: s.debounce.last().map_or(0, |p| p.readiness),
        blockage: s.blockage,
        failure_class: s.failure_class,
    }
}
