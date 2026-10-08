//! The HOST-SWITCHED device-log layer — the SDK's connectivity log, present in
//! every build a user runs and SILENT until the host turns it on
//! (`docs/plan/on-device-log-layer-phase-1.md`; maintainer ruling 2026-09-19:
//! *"sdk definitely should include logging layer so you will be able to debug
//! connectivity first on the android. and then on iphone"*).
//!
//! Before this layer a sync or connectivity report from a tester's phone was
//! unfalsifiable on both platforms: the only logcat layer was gated on
//! `debug_assertions` (never true in a build anyone ships) or on the
//! MEASUREMENT door, and Apple had nothing. A host's own subscriber cannot
//! rescue it — this library and the host's each link their own `tracing-core`,
//! so each has its own global dispatcher.
//!
//! WHO TURNS IT ON: THE HOST, NEVER THE SDK (FR-35; maintainer, the same day, on
//! reading the first cut of this module as an always-on layer: *"it shouldnt be
//! always on … configurabale … depends on the host settings visible to user"*).
//! The host owns its user's consent — Relim asks *Keep logging / Stop logging*
//! at onboarding — and consent does not reach a user who opted out, so a
//! library that logs regardless breaks honest-off from inside code the user
//! never saw. The layer is installed with its [`Gate`] CLOSED; a host that says
//! nothing gets no device log, exactly as before this module existed. The
//! host's verb is `api::meta::set_device_log`, live, with no re-init.
//!
//! WHAT MAKES IT SAFE TO SHIP IN EVERY BUILD: it ENFORCES §5.4 AT RUNTIME. Every field
//! of every event is put to the core's one policy predicate
//! (`zec_wallet_core::log_policy::field_is_loggable` — the predicate the capture
//! guard's tests assert through) and a field that fails it is WITHHELD: the line
//! still prints, with `withheld=<n>` appended, so a gap shows up as a gap rather
//! than as a leak or as silence. The capture guard sees only the paths a test
//! drives; this sees every path that runs on a device.
//!
//! It is deliberately NOT `tracing_subscriber::fmt`: that layer's `on_close`
//! links the strings `time.busy` / `time.idle` whenever the TYPE is instantiated
//! (`FmtSpan` is a runtime setting), and those two strings are what
//! `just wallet-device-timing-negative-witness` uses to tell a measurement
//! artifact from a distributable. An always-on fmt layer would blind it.
//!
//! SCOPE: events only — no span lines and no span context — at INFO and above,
//! on this SDK's targets only ([`is_sdk_target`]). Dependency crates
//! (`zcash_client_backend`, `zcash_client_sqlite`, `h2`, `tonic`) log addresses,
//! txids and note commitments at info/debug; they are refused at the callsite
//! cache by the per-layer filter and again, independently, inside `on_event`.
//! The sync batch TIMINGS ride a span, so this log says nothing about them: its
//! silence between connectivity transitions is not "sync is not running".
//!
//! TWO ARMS, ONE LINE (S5, `docs/plan/stage-5-the-device-log-a-user-can-send.md`
//! §3.1): the finished line goes to the platform log AND to the one stream a host
//! registered (`api::meta::watch_device_log`) — the same line, after the same
//! gate check, from the same `on_event` call ([`FanOut`]). The host arm is one
//! critical section ([`HostSlot`]) that [`quiesce_for_sever`] takes too, so once
//! a wipe has begun no line is enqueued on the host stream.

#![deny(unsafe_code)]
// Four platforms have a sink — Android (logcat) and Apple (the unified log), in
// the maintainer's order, and Linux and Windows (stderr, FR-42). On any other
// target nothing installs this layer, the host's verb answers `Off`, and
// outside tests only the host slot's verb and the sever's quiesce are
// referenced.
#![cfg_attr(
    not(any(
        test,
        target_os = "android",
        target_vendor = "apple",
        target_os = "linux",
        target_os = "windows"
    )),
    allow(dead_code)
)]

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{FilterFn, Filtered, LevelFilter, filter_fn};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use zec_wallet_core::log_policy::{field_is_loggable, is_sdk_target};

use crate::api::meta::{DeviceLogLevel, DeviceLogLine, DeviceLogSeverity};
use crate::frb_generated::StreamSink;

/// The log tag of an event the CORE emitted — its module targets and the
/// explicit `wallet.<surface>` families alike.
pub(crate) const CORE_TAG: &str = "zec_wallet_core";
/// The log tag of an event the BRIDGE emitted (the host-dialer lifecycle).
pub(crate) const BRIDGE_TAG: &str = "zec_wallet";

/// Cap on one written line, in bytes. Every value this layer prints is a count,
/// a code or a short closed-vocabulary word, so a real line is tens of bytes;
/// the cap exists for the one open-ended shape the allowlist admits
/// (`error = %e`), so a pathological `Display` cannot flood a device log that
/// other apps' diagnostics share. Well under logd's 4096-byte truncation.
pub(crate) const DEVICE_LOG_LINE_MAX_BYTES: usize = 512;

/// The quietest level this layer prints. `tracing` orders levels by VERBOSITY
/// (`ERROR < WARN < INFO < DEBUG < TRACE`), so "INFO and above" is `<=`.
const DEVICE_LOG_MAX_VERBOSITY: Level = Level::INFO;

/// Where a finished line goes. The platform difference is this trait and
/// nothing else: scope, level, filtering and formatting are shared, which is
/// what lets ordinary host tests drive the real layer through a memory sink.
pub(crate) trait LogSink: Send + Sync + 'static {
    /// Write one line. `meta` is the event's (its level picks the platform
    /// priority); `tag` is [`CORE_TAG`] or [`BRIDGE_TAG`].
    fn write(&self, meta: &Metadata<'_>, tag: &'static str, line: &str);
}

/// THE HOST'S SWITCH (FR-35). OFF until the host sets it; read on every event,
/// so a change takes effect for the next one with no re-install. Three levels,
/// the maintainer's (2026-09-19): *"'Off' / 'Errors only' / 'Detailed logging'"* —
/// the DEFAULTS among them are the host's (its debug build, its production
/// build, its user's choice); this SDK's own default is OFF.
///
/// It lives in `on_event` and NOT in the filter, and that placement is the
/// mechanism: `tracing` caches a callsite's interest the first time the callsite
/// is hit, so a gate read by the filter would freeze at whatever it said then —
/// a wallet that logged nothing before the host's first call would go on
/// logging nothing after it. The filter answers the STATIC question (is this
/// callsite ours, at a level we print) and the gate answers the live one.
///
/// A handle, not a bare static, so each test owns its own: the suite runs in
/// parallel in one process, and a shared switch would let one test turn another
/// test's log off.
#[derive(Clone, Debug, Default)]
pub(crate) struct Gate(Arc<AtomicU8>);

impl Gate {
    pub(crate) fn set(&self, level: DeviceLogLevel) {
        self.0.store(encode(level), Ordering::Relaxed);
    }

    /// `Relaxed` is enough: there is no second value to order this against, and
    /// a stale read on one event is a line written (or dropped) an instant
    /// around the host's call — the call itself is what the host can observe.
    pub(crate) fn level(&self) -> DeviceLogLevel {
        decode(self.0.load(Ordering::Relaxed))
    }

    /// `Off`, `SeqCst` — the sever's store ([`quiesce`]). This does NOT carry
    /// the host stream's "no line after the quiesce": `level` loads `Relaxed`,
    /// so no ordering edge reaches it. That guarantee is the sink taken under
    /// the host slot's lock, which every host send holds. The store only
    /// narrows the window before that lock, and the platform arm's residual.
    fn shut(&self) {
        self.0.store(encode(DeviceLogLevel::Off), Ordering::SeqCst);
    }
}

/// `Off` is ZERO on purpose: `AtomicU8::default()` — what a fresh [`Gate`] and
/// the process gate start as — is then OFF by construction, not by an
/// initialiser somebody has to remember.
const fn encode(level: DeviceLogLevel) -> u8 {
    match level {
        DeviceLogLevel::Off => 0,
        DeviceLogLevel::Errors => 1,
        DeviceLogLevel::Detailed => 2,
    }
}

/// A byte that is not a level is OFF — the privacy-correct reading of a value
/// this module never writes.
const fn decode(raw: u8) -> DeviceLogLevel {
    match raw {
        1 => DeviceLogLevel::Errors,
        2 => DeviceLogLevel::Detailed,
        _ => DeviceLogLevel::Off,
    }
}

/// The most VERBOSE `tracing` level a host level lets through, or `None` for
/// nothing at all. `tracing` orders levels by verbosity
/// (`ERROR < WARN < INFO < DEBUG`), so "lets through" is `event <= ceiling`.
///
/// `Errors` is WARN and above, not ERROR alone: in this SDK WARN is where
/// "something went wrong and we carried on" lives — a private path that would
/// not connect, a fall-back to clearnet, a stalled sync — and a user who chose
/// "errors only" is asking to be told exactly that. `Detailed` is this layer's
/// whole static scope ([`DEVICE_LOG_MAX_VERBOSITY`]); there is no level above
/// it, so no host setting is a road to DEBUG on a user's phone.
const fn ceiling(level: DeviceLogLevel) -> Option<Level> {
    match level {
        DeviceLogLevel::Off => None,
        DeviceLogLevel::Errors => Some(Level::WARN),
        DeviceLogLevel::Detailed => Some(DEVICE_LOG_MAX_VERBOSITY),
    }
}

/// Whether the host level `setting` lets an event at `level` through.
fn admits(setting: DeviceLogLevel, level: Level) -> bool {
    ceiling(setting).is_some_and(|ceiling| level <= ceiling)
}

/// The ONE gate the installed layer reads and the host's verb writes.
pub(crate) fn process_gate() -> &'static Gate {
    static GATE: OnceLock<Gate> = OnceLock::new();
    GATE.get_or_init(Gate::default)
}

/// Whether a subscriber carrying this layer is the process's global default —
/// set once by the install site, never cleared (a hot restart's second install
/// loses to the first, which is still there and still reads [`process_gate`]).
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Record that the install succeeded. Called from the ONE install site, which
/// exists only where a sink does.
#[cfg_attr(
    not(any(
        target_os = "android",
        target_vendor = "apple",
        target_os = "linux",
        target_os = "windows"
    )),
    allow(dead_code)
)]
pub(crate) fn mark_installed() {
    INSTALLED.store(true, Ordering::Relaxed);
}

/// The host's verb. Records the host's choice and answers what is TRUE after
/// it: the level at which a line can now actually be written.
pub(crate) fn set_level(requested: DeviceLogLevel) -> DeviceLogLevel {
    process_gate().set(requested);
    effective_level()
}

/// The EFFECTIVE level — what the host asked for IF anything here can write
/// it, otherwise `Off`. `Off` for a host that asked for more means nothing is
/// installed to write through: a platform with no sink yet, or a process where
/// another subscriber already owns the global `tracing` default. The host
/// renders THIS, not its own setting, so its settings row cannot claim a log
/// that does not exist.
pub(crate) fn effective_level() -> DeviceLogLevel {
    effective(INSTALLED.load(Ordering::Relaxed), process_gate().level())
}

/// The decision, on its own so each input has a test that can see only it.
fn effective(installed: bool, requested: DeviceLogLevel) -> DeviceLogLevel {
    if installed {
        requested
    } else {
        DeviceLogLevel::Off
    }
}

/// Which of the SDK's targets the PLATFORM arm prints. The layer's filter is
/// always [`Scope::AllSdkTargets`] (the host arm gets every SDK target in every
/// build); this narrows only what reaches the platform sink ([`FanOut`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Every SDK target — every Apple build, and every Android build nobody
    /// opened a door in. Also the layer's filter, in every build.
    AllSdkTargets,
    /// Every SDK target EXCEPT the core's module target: the debug/measurement
    /// build installs the louder fmt layer on exactly that target (DEBUG +
    /// span closes), and printing its INFO events twice would move the counts a
    /// measurement parser takes from that log.
    // … and the only one that constructs this.
    #[cfg_attr(
        not(any(
            test,
            all(target_os = "android", any(debug_assertions, zec_wallet_device_timing))
        )),
        allow(dead_code)
    )]
    BesideTheLoudLayer,
}

/// THE predicate — read by the per-layer filter (so a refused callsite is
/// disabled at the callsite cache) AND by `on_event` (so the layer is correct
/// even installed without its filter), both at [`Scope::AllSdkTargets`]; and
/// by [`FanOut`] at the platform arm's own scope. Each reader is tested where
/// the others cannot help it.
fn prints(scope: Scope, meta: &Metadata<'_>) -> bool {
    if !meta.is_event() || *meta.level() > DEVICE_LOG_MAX_VERBOSITY {
        return false;
    }
    let target = meta.target();
    if !is_sdk_target(target) {
        return false;
    }
    match scope {
        Scope::AllSdkTargets => true,
        Scope::BesideTheLoudLayer => !is_core_module(target),
    }
}

/// `zec_wallet_core` or `zec_wallet_core::…` — the target the debug/measurement
/// fmt layer claims. That layer's filter READS THIS FUNCTION (`api::meta::
/// loud_logcat_layer`), so "leave the core module to the loud layer" and "the
/// loud layer prints the core module" are one predicate: no target is printed
/// by both layers or by neither. (It used `Targets`, which matches by bare
/// prefix — `zec_wallet_core_x` would have reached the loud, UNENFORCED layer
/// while this one, on a `::` boundary, left it alone: the arch review.)
pub(crate) fn is_core_module(target: &str) -> bool {
    target
        .strip_prefix(CORE_TAG)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
}

/// The bridge's own events get the bridge's tag; everything else this layer
/// prints was emitted by the core.
fn tag_of(target: &str) -> &'static str {
    let bridge = target
        .strip_prefix(BRIDGE_TAG)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"));
    if bridge { BRIDGE_TAG } else { CORE_TAG }
}

/// The layer, WITHOUT its callsite filter. Production installs it through
/// [`device_log_layer`]; it is `pub(crate)` on its own so a test can prove
/// `on_event`'s scope check holds with no filter in front of it.
pub(crate) struct DeviceLogLayer<P> {
    sink: FanOut<P>,
    gate: Gate,
}

impl<P: LogSink> DeviceLogLayer<P> {
    pub(crate) fn new(sink: FanOut<P>, gate: Gate) -> Self {
        Self { sink, gate }
    }
}

/// The layer as installed: [`DeviceLogLayer`] behind a per-layer filter on the
/// same predicate. A PER-LAYER filter, not `Layer::enabled` — a layer's own
/// `enabled` answering `false` disables the event for EVERY layer in the stack,
/// which would silence the debug/measurement fmt layer installed beside this
/// one. With per-layer filters on every layer, a callsite no filter wants is
/// `Interest::never()` — dependency callsites cost one cached atomic load.
///
/// The filter is [`Scope::AllSdkTargets`] in every build: the platform arm's
/// narrower scope is applied in [`FanOut`], so the HOST arm still gets the
/// core's module target in a debug Android build (S5 §3.1 fact 2). The cost is
/// one `prints` call per core-module event there, in builds nobody ships.
pub(crate) fn device_log_layer<S, P>(
    sink: FanOut<P>,
    gate: Gate,
) -> Filtered<DeviceLogLayer<P>, FilterFn<impl Fn(&Metadata<'_>) -> bool>, S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    P: LogSink,
{
    DeviceLogLayer::new(sink, gate).with_filter(device_log_filter())
}

/// The installed layer's filter, on its own — so a test can put it in front of
/// a layer that prints EVERYTHING and watch the filter with nothing behind it
/// to mask a hole in it (`prints` is read twice; each reader gets its own test).
///
/// The level HINT is what keeps a door-closed build cheap: without it a
/// `filter_fn` has no hint, the subscriber's max level is unbounded, and every
/// `debug!`/`trace!` callsite in the dependency graph pays a callsite-interest
/// lookup where, before an earlier revision, a release build installed no subscriber at all.
/// With it `tracing`'s global max level is INFO and those callsites stop at one
/// static comparison.
fn device_log_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    filter_fn(|meta| prints(Scope::AllSdkTargets, meta))
        .with_max_level_hint(LevelFilter::from_level(DEVICE_LOG_MAX_VERBOSITY))
}

impl<S, P> Layer<S> for DeviceLogLayer<P>
where
    S: Subscriber,
    P: LogSink,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // The host's switch first: OFF means nothing is formatted, let alone
        // written — honest-off stops the log, not an indicator of it.
        let Some(ceiling) = ceiling(self.gate.level()) else {
            return;
        };
        let meta = event.metadata();
        if *meta.level() > ceiling || !prints(Scope::AllSdkTargets, meta) {
            return;
        }
        let mut line = LineBuilder::default();
        event.record(&mut line);
        self.sink
            .write(&self.gate, meta, tag_of(meta.target()), &line.finish());
    }
}

/// THE ONE SINK the layer writes to: the platform log and the host's stream,
/// fed the SAME finished line (S5 §3.1 mechanism 1). The gate, the ceiling,
/// §5.4 withholding, ASCII-only, the cap and SDK-targets-only therefore hold for
/// the host by construction — there is no second copy of those rules to drift.
///
/// The arms differ in two things only: the platform arm keeps its build's
/// [`Scope`] (beside the loud layer it leaves the core module to that layer),
/// and the host arm has its own budget and its own close ([`HostSlot`]).
///
/// Built by struct literal, as the contract prints it: the install's text is
/// what `extraction_policy.rs::the_device_log_is_installed_closed_in_every_build`
/// reads, so a constructor would hide which slot and scope an arm was handed.
pub(crate) struct FanOut<P> {
    pub(crate) platform: P,
    pub(crate) platform_scope: Scope,
    pub(crate) host: HostSlot,
}

impl<P: LogSink> FanOut<P> {
    /// One line, once to each arm. The gate is read again before each: a sever
    /// ([`quiesce`]) that lands after `on_event`'s read stops the host arm
    /// exactly (its read is under the lock the sever takes) and the platform arm
    /// for every event that has not reached this read yet.
    fn write(&self, gate: &Gate, meta: &Metadata<'_>, tag: &'static str, line: &str) {
        if prints(self.platform_scope, meta) && admits(gate.level(), *meta.level()) {
            self.platform.write(meta, tag, line);
        }
        self.host.send_line(gate, *meta.level(), tag, line);
    }
}

/// How many lines a minute the HOST arm sends (S5 §3.1 mechanism 5): a token
/// bucket, refilled at this rate, holding at most this many. There is no queue
/// of our own and FRB's post exposes no port depth, so this is what bounds a
/// host that listens and never drains: 60 × 512 B ≈ 30 KB a minute. A line
/// over budget still reaches the platform log, and leaves a gap in `seq`.
pub(crate) const HOST_LINES_PER_MINUTE: u32 = 60;

/// Where the host arm hands a line. Production wraps FRB's `StreamSink`
/// ([`watch_process`]), which cannot be built in a host test; the tests use a
/// memory impl. `false` means GONE (the Dart side cancelled or its isolate
/// died): the slot then drops this sink.
pub(crate) trait HostLineSink: Send + 'static {
    fn send(&self, line: DeviceLogLine) -> bool;
}

/// The host arm's clock — injected so a test can move a minute without a sleep.
type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

/// [`HOST_LINES_PER_MINUTE`] as a bucket: one token earned every
/// `60 s / HOST_LINES_PER_MINUTE`, at most [`HOST_LINES_PER_MINUTE`] held.
struct TokenBucket {
    tokens: u32,
    /// When the tokens held were last brought up to date; a partial token's
    /// time is carried, not lost.
    refilled_at: Instant,
    clock: Clock,
}

impl TokenBucket {
    fn new(clock: Clock) -> Self {
        Self {
            tokens: HOST_LINES_PER_MINUTE,
            refilled_at: clock(),
            clock,
        }
    }

    /// Spend one token, or answer `false` if none is left.
    fn take(&mut self) -> bool {
        let now = (self.clock)();
        let per_token = Duration::from_secs(60) / HOST_LINES_PER_MINUTE;
        let earned =
            now.saturating_duration_since(self.refilled_at).as_nanos() / per_token.as_nanos();
        let earned = u32::try_from(earned).unwrap_or(u32::MAX);
        let room = HOST_LINES_PER_MINUTE - self.tokens;
        if earned >= room {
            self.tokens = HOST_LINES_PER_MINUTE;
            self.refilled_at = now;
        } else if earned > 0 {
            self.tokens += earned;
            self.refilled_at += per_token * earned;
        }
        if self.tokens == 0 {
            return false;
        }
        self.tokens -= 1;
        true
    }
}

/// Everything the host arm decides with, behind ONE lock.
struct HostState {
    /// The one registered stream. A sever takes it and nothing but the host's
    /// next watch puts one back, so `None` IS the closed state — there is no
    /// separate flag to drift from it.
    sink: Option<Box<dyn HostLineSink>>,
    /// The next line's number. Taken for every line the gate lets through,
    /// before the sink/budget checks, so a line this subscription did
    /// not get is a gap in the numbers it did. Restarts at 0 at every sever.
    seq: u64,
    budget: TokenBucket,
    /// Test 13's pause point, inside the critical section just before the send.
    #[cfg(test)]
    pause: Option<Box<dyn Fn() + Send>>,
}

/// THE HOST ARM: one registered stream, and the one lock its check-and-send
/// and the sever share (S5 §3.1 mechanisms 2, 3 and 6). A handle, like
/// [`Gate`], so each test owns its own; production has one
/// ([`process_host_slot`]).
///
/// The sink is never cloned out of the slot: it is used under the lock, and
/// dropped in the slot, which is what closes the Dart stream (FRB closes it
/// when the last `StreamSink` drops) — so no reference outlives a sever.
///
/// Both paths lock through [`HostSlot::lock`], which recovers a POISONED lock:
/// the state's invariants are trivial and the sever overwrites every field that
/// matters, so a panic inside one `send` can neither disarm the sever nor turn
/// every later event into a panic.
#[derive(Clone)]
pub(crate) struct HostSlot(Arc<Mutex<HostState>>);

impl Default for HostSlot {
    fn default() -> Self {
        Self::with_clock(Instant::now)
    }
}

impl HostSlot {
    /// A slot whose budget reads `clock` — `Instant::now` in production.
    pub(crate) fn with_clock(clock: impl Fn() -> Instant + Send + Sync + 'static) -> Self {
        Self(Arc::new(Mutex::new(HostState {
            sink: None,
            seq: 0,
            budget: TokenBucket::new(Arc::new(clock)),
            #[cfg(test)]
            pause: None,
        })))
    }

    fn lock(&self) -> MutexGuard<'_, HostState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Register the host's stream. It REPLACES any earlier one — dropping that
    /// sink closes its Dart stream — and re-opens a slot a sever closed. Never
    /// fails: a hot restart's new isolate, or a re-subscribe after unlock,
    /// simply takes over. `seq` carries on; only a sever restarts it.
    pub(crate) fn watch(&self, sink: Box<dyn HostLineSink>) {
        self.lock().sink = Some(sink);
    }

    /// The host arm, as one critical section: the gate re-read, `seq`, the
    /// checks and the post all happen under the lock the sever takes, so every
    /// line this sends was checked against a gate and a slot no sever had yet
    /// closed. The post is FRB's non-blocking enqueue, so the lock is held for
    /// microseconds, and its failure path does not re-enter `tracing`.
    fn send_line(&self, gate: &Gate, level: Level, tag: &'static str, text: &str) {
        let mut guard = self.lock();
        let state = &mut *guard;
        if !admits(gate.level(), level) {
            return;
        }
        let seq = state.seq;
        state.seq += 1;
        if state.sink.is_none() || !state.budget.take() {
            return;
        }
        #[cfg(test)]
        if let Some(pause) = &state.pause {
            pause();
        }
        let line = DeviceLogLine {
            seq,
            severity: severity_of(level),
            tag: tag.to_owned(),
            text: text.to_owned(),
        };
        let gone = state.sink.as_ref().is_some_and(|sink| !sink.send(line));
        if gone {
            state.sink = None;
        }
    }

    /// Test 13's hook: `pause` runs inside the host arm's critical section,
    /// after every check and just before the send.
    #[cfg(test)]
    pub(crate) fn set_pause_hook(&self, pause: impl Fn() + Send + 'static) {
        self.lock().pause = Some(Box::new(pause));
    }
}

/// The DTO's severity for an event's level. The layer admits nothing more
/// verbose than INFO, so anything else is `Info`.
fn severity_of(level: Level) -> DeviceLogSeverity {
    if level == Level::ERROR {
        DeviceLogSeverity::Error
    } else if level == Level::WARN {
        DeviceLogSeverity::Warn
    } else {
        DeviceLogSeverity::Info
    }
}

/// The ONE host slot the installed layer writes to and the host's verb fills.
pub(crate) fn process_host_slot() -> &'static HostSlot {
    static SLOT: OnceLock<HostSlot> = OnceLock::new();
    SLOT.get_or_init(HostSlot::default)
}

/// THE SEVER'S QUIESCE, on a given gate and slot: `Off` into the gate, then,
/// under the host arm's lock, `seq` back to 0 and the sink taken and dropped
/// (which closes the Dart stream). After this returns no line is enqueued on
/// the host stream — every enqueue is under that lock, after a check of the
/// gate and of the sink — and nothing re-opens the log but the host's own
/// `set_device_log` and `watch_device_log`.
///
/// It waits for at most ONE in-progress host enqueue, never for a platform
/// write. The platform arms keep a stated residual: a thread past its gate
/// read may still finish one platform line after this returns.
pub(crate) fn quiesce(gate: &Gate, slot: &HostSlot) {
    gate.shut();
    let mut state = slot.lock();
    state.seq = 0;
    drop(state.sink.take());
}

/// [`quiesce`] on the process gate and the process slot — the FIRST statement
/// of every wipe (`convert::wipe_wallet`) and of the duress sever
/// (`convert::sever_custody`), before any validation: the SDK
/// cannot tell a duress wipe from a user's Delete, so nothing may be logged or
/// streamed on that path, even when the call then fails. The host re-arms.
pub(crate) fn quiesce_for_sever() {
    quiesce(process_gate(), process_host_slot());
}

/// Adapts FRB's `StreamSink` to [`HostLineSink`]. `add` fails once Dart has
/// cancelled the stream or its isolate is gone: that is "gone".
struct FrbHostLineSink(StreamSink<DeviceLogLine>);

impl HostLineSink for FrbHostLineSink {
    fn send(&self, line: DeviceLogLine) -> bool {
        self.0.add(line).is_ok()
    }
}

/// The host's verb (`api::meta::watch_device_log`): register `sink` in the
/// process slot, replacing any earlier stream.
pub(crate) fn watch_process(sink: StreamSink<DeviceLogLine>) {
    process_host_slot().watch(Box::new(FrbHostLineSink(sink)));
}

#[cfg(test)]
mod host_arm_tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    /// A clock the test moves by hand, in milliseconds past a fixed start.
    fn manual_clock() -> (Arc<AtomicU64>, Clock) {
        let start = Instant::now();
        let millis = Arc::new(AtomicU64::new(0));
        let read = Arc::clone(&millis);
        let clock: Clock =
            Arc::new(move || start + Duration::from_millis(read.load(Ordering::Relaxed)));
        (millis, clock)
    }

    /// The bucket refills one token per second, carries a partial second, and
    /// never holds more than a minute's worth.
    ///
    /// Watched against: `refilled_at = now` on a partial refill (the carried
    /// half-second is lost, and the 1.5 s + 0.5 s token never arrives).
    #[test]
    fn the_host_budget_refills_a_token_a_second_and_carries_the_remainder() {
        let (millis, clock) = manual_clock();
        let mut bucket = TokenBucket::new(clock);
        for _ in 0..HOST_LINES_PER_MINUTE {
            assert!(bucket.take());
        }
        assert!(!bucket.take(), "a minute's burst, then nothing");
        millis.store(1_500, Ordering::Relaxed);
        assert!(bucket.take(), "one token after a second");
        assert!(!bucket.take(), "and only one");
        millis.store(2_000, Ordering::Relaxed);
        assert!(bucket.take(), "the carried half second makes the next");
        millis.store(3_600_000, Ordering::Relaxed);
        let mut after_an_hour = 0;
        while bucket.take() {
            after_an_hour += 1;
        }
        assert_eq!(
            after_an_hour, HOST_LINES_PER_MINUTE,
            "never more than the burst"
        );
    }

    #[derive(Clone, Default)]
    struct Received(Arc<Mutex<Vec<DeviceLogLine>>>);

    impl HostLineSink for Received {
        fn send(&self, line: DeviceLogLine) -> bool {
            self.0.lock().expect("receiver poisoned").push(line);
            true
        }
    }

    /// The pause hook really is INSIDE the lock the quiesce takes: a sever
    /// started while a send is parked there returns only after that send, and
    /// nothing is sent after it.
    ///
    /// Watched against: the hook called after the guard is dropped (the
    /// quiesce returns while the send is still parked).
    #[test]
    fn the_pause_hook_holds_the_lock_the_quiesce_takes() {
        use std::sync::Barrier;
        use std::sync::atomic::AtomicBool;

        let gate = Gate::default();
        gate.set(DeviceLogLevel::Detailed);
        let slot = HostSlot::default();
        let received = Received::default();
        slot.watch(Box::new(received.clone()));
        let parked = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let (p, r) = (Arc::clone(&parked), Arc::clone(&release));
        slot.set_pause_hook(move || {
            p.wait();
            r.wait();
        });

        let sender = {
            let (gate, slot) = (gate.clone(), slot.clone());
            std::thread::spawn(move || slot.send_line(&gate, Level::INFO, CORE_TAG, "before"))
        };
        parked.wait();
        let quiesced = Arc::new(AtomicBool::new(false));
        let severer = {
            let (gate, slot, quiesced) = (gate.clone(), slot.clone(), Arc::clone(&quiesced));
            std::thread::spawn(move || {
                quiesce(&gate, &slot);
                quiesced.store(true, Ordering::SeqCst);
            })
        };
        std::thread::sleep(Duration::from_millis(50));
        assert!(
            !quiesced.load(Ordering::SeqCst),
            "the quiesce waits for the parked send"
        );
        release.wait();
        sender.join().expect("the sender finishes");
        severer.join().expect("the quiesce finishes");
        slot.send_line(&gate, Level::ERROR, CORE_TAG, "after");
        let texts: Vec<String> = received
            .0
            .lock()
            .expect("receiver poisoned")
            .iter()
            .map(|l| l.text.clone())
            .collect();
        assert_eq!(texts, ["before"]);
    }
}

/// Builds one line from one event's fields, putting each to the §5.4 policy.
#[derive(Default)]
struct LineBuilder {
    message: Option<String>,
    fields: String,
    withheld: usize,
}

impl LineBuilder {
    fn put(&mut self, name: &str, value: &str) {
        if !field_is_loggable(name, value) {
            self.withheld += 1;
            return;
        }
        if name == "message" {
            self.message = Some(value.to_owned());
        } else {
            // `write!` to a `String` cannot fail.
            let _ = write!(self.fields, " {name}={value}");
        }
    }

    fn finish(self) -> String {
        let mut line = self
            .message
            .unwrap_or_else(|| "<message withheld>".to_owned());
        // `withheld=` goes BEFORE the fields: the cap below cuts from the END,
        // and the marker is the one thing on the line that must survive it — a
        // long `error` beside a refused field is exactly the case the cap exists
        // for, and a gap that the cap erased would read as no gap at all.
        if self.withheld > 0 {
            let _ = write!(line, " withheld={}", self.withheld);
        }
        line.push_str(&self.fields);
        // PRINTABLE ASCII ONLY. A value must not forge a second record or
        // repaint this one: `\n`/`\r` are the obvious breaks, but U+2028/U+2029
        // split lines for any parser built on Python's `splitlines()` (ours
        // are), and the bidi overrides (U+202A–202E, U+2066–2069) let a value
        // DISPLAY `dial_arm=host` over an `sdk_direct` line — none of which
        // `char::is_control` covers. Everything this SDK logs is ASCII by
        // construction; anything else becomes a space. (It also keeps NUL out:
        // the Android writer silently drops a line containing one.)
        let mut line: String = line
            .chars()
            .map(|c| {
                if c.is_ascii_graphic() || c == ' ' {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        if line.len() > DEVICE_LOG_LINE_MAX_BYTES {
            // ASCII now, so every byte offset is a char boundary.
            line.truncate(DEVICE_LOG_LINE_MAX_BYTES);
            line.push_str("...");
        }
        line
    }
}

// The SAME two recorders as the capture guard's `Grab` (`tracing_guard.rs`), so a
// value is rendered for the policy here exactly as it is rendered for the policy
// in a test: `record_str` raw, everything else through `Debug`.
impl Visit for LineBuilder {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.put(field.name(), &format!("{value:?}"));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.put(field.name(), value);
    }
}

/// Android: logcat, one writer per tag (`adb logcat -s zec_wallet_core:V
/// zec_wallet:V`). `paranoid-android`'s WRITER only — never its fmt layer (the
/// module doc says why) — so no `unsafe` enters this crate for logging.
#[cfg(target_os = "android")]
pub(crate) struct LogcatSink {
    core: paranoid_android::AndroidLogMakeWriter,
    bridge: paranoid_android::AndroidLogMakeWriter,
}

#[cfg(target_os = "android")]
impl LogcatSink {
    pub(crate) fn new() -> Self {
        Self {
            core: paranoid_android::AndroidLogMakeWriter::new(CORE_TAG.to_owned()),
            bridge: paranoid_android::AndroidLogMakeWriter::new(BRIDGE_TAG.to_owned()),
        }
    }
}

#[cfg(target_os = "android")]
impl LogSink for LogcatSink {
    fn write(&self, meta: &Metadata<'_>, tag: &'static str, line: &str) {
        use std::io::Write as _;
        use tracing_subscriber::fmt::MakeWriter as _;
        let make = if tag == BRIDGE_TAG {
            &self.bridge
        } else {
            &self.core
        };
        // The writer flushes to logd when it drops; a log write that fails has
        // nowhere to be reported, and must never take the caller down with it.
        let _ = make.make_writer_for(meta).write_all(line.as_bytes());
    }
}

/// The unified-log SUBSYSTEM every Apple line is filed under; the CATEGORY is
/// the tag ([`CORE_TAG`] / [`BRIDGE_TAG`]), so one predicate reads both:
/// `log stream --level info --predicate 'subsystem == "zec_wallet"'`, or the
/// same filter in Console.app with a device attached.
#[cfg(target_vendor = "apple")]
pub(crate) const APPLE_LOG_SUBSYSTEM: &str = "zec_wallet";

/// Apple (iOS and macOS): the unified log, one `os_log` handle per tag, through
/// the pinned `oslog` shim — the safe wrapper over the `os_log` macro family, so
/// no `unsafe` enters this crate for logging here either.
///
/// WHAT A LEVEL BECOMES, and why it is not the obvious mapping. `os_log`'s
/// `info` type is kept IN MEMORY ONLY — it never reaches the disk store, so a
/// line logged at it is gone by the time a tester's phone is plugged in, which
/// is the only moment this log is read. So INFO is filed as `default` (the
/// lowest PERSISTED type) and WARN and ERROR both as `error`; `fault` is
/// reserved by the OS for system-level failures and captures far more than a
/// line. The unified log has no "warning" type, and two `tracing` levels share
/// one type, so the LEVEL IS WRITTEN INTO THE LINE — `WARN wallet.dial …` —
/// where Android carries it in the record's priority instead.
///
/// The persisted types are the cost the host's user is told about: a line
/// written here is readable on disk for days (FR-35; the maintainer chose
/// diagnosability with that stated, 2026-09-19).
#[cfg(target_vendor = "apple")]
pub(crate) struct OsLogSink {
    core: oslog::OsLog,
    bridge: oslog::OsLog,
}

#[cfg(target_vendor = "apple")]
impl OsLogSink {
    /// `None` if the OS would not hand out a log handle. `oslog::OsLog::new`
    /// ASSERTS on a null handle; this runs inside `init_app`, where a panic is
    /// a failed init and an unavailable WALLET — so the assertion is fenced, and
    /// the cost of a missing handle is a device log that reports itself `Off`.
    pub(crate) fn new() -> Option<Self> {
        std::panic::catch_unwind(|| Self {
            core: oslog::OsLog::new(APPLE_LOG_SUBSYSTEM, CORE_TAG),
            bridge: oslog::OsLog::new(APPLE_LOG_SUBSYSTEM, BRIDGE_TAG),
        })
        .ok()
    }
}

#[cfg(target_vendor = "apple")]
impl LogSink for OsLogSink {
    fn write(&self, meta: &Metadata<'_>, tag: &'static str, line: &str) {
        let log = if tag == BRIDGE_TAG {
            &self.bridge
        } else {
            &self.core
        };
        let level = *meta.level();
        // `prints` admits nothing more verbose than INFO; anything else here is
        // ERROR or WARN.
        let kind = if level == Level::INFO {
            oslog::Level::Default
        } else {
            oslog::Level::Error
        };
        log.with_level(kind, &format!("{level} {line}"));
    }
}

/// Linux and Windows (S5 `stderr`, FR-42): the process's stderr, where a
/// desktop host's own console output already goes. stderr has no priority and
/// no category, so the LEVEL and the TAG are WRITTEN INTO THE LINE —
/// `WARN zec_wallet_core wallet.dial …`. Android carries the tag as the logcat
/// tag, and Apple as the unified log's category. Here the text is the only
/// place it can go.
///
/// ONE `write_all` per line, on a stderr opened (and locked) per line: stderr
/// is shared with the host, and a line written in pieces could be interleaved
/// with the host's own output. A failed write — a closed pipe — is dropped: a
/// diagnostic has nowhere to report its own failure and must never take the
/// wallet's caller down with it.
///
/// The residual, stated: a write that BLOCKS is not dropped. If the host's
/// stderr is a pipe whose reader has stopped draining (a full pipe buffer, a
/// suspended log collector), the emitting SDK thread waits inside
/// `write_all` until the reader drains. This is the same residual as a
/// stalled logd on Android. It is reached only while the host has the log on,
/// and the sever's quiesce never waits on it (§3.1). A host that pipes stderr
/// keeps that pipe drained, or leaves the log `Off`.
///
/// `open` is the seam a host test drives this through; production opens the
/// real stderr ([`StderrSink::new`]).
#[cfg(any(test, target_os = "linux", target_os = "windows"))]
pub(crate) struct StderrSink<M = fn() -> std::io::StderrLock<'static>> {
    pub(crate) open: M,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl StderrSink {
    pub(crate) fn new() -> Self {
        Self {
            open: || std::io::stderr().lock(),
        }
    }
}

#[cfg(any(test, target_os = "linux", target_os = "windows"))]
impl<M, W> LogSink for StderrSink<M>
where
    M: Fn() -> W + Send + Sync + 'static,
    W: std::io::Write,
{
    fn write(&self, meta: &Metadata<'_>, tag: &'static str, line: &str) {
        let framed = format!("{} {tag} {line}\n", meta.level());
        let _ = (self.open)().write_all(framed.as_bytes());
    }
}

/// The in-memory sink the host tests drive the REAL layer through — this
/// module's, and the cabi module's (which owns the five bridge events and the
/// machinery that fires them).
#[cfg(test)]
pub(crate) mod test_sink {
    use super::{DeviceLogLevel, FanOut, Gate, HostSlot, LogSink, Scope, device_log_layer};
    use std::sync::{Arc, Mutex};
    use tracing::{Level, Metadata};
    use tracing_subscriber::layer::SubscriberExt;

    /// One written line: `(level, tag, line)`.
    pub(crate) type Written = (Level, &'static str, String);

    #[derive(Clone, Default)]
    pub(crate) struct MemorySink(Arc<Mutex<Vec<Written>>>);

    impl MemorySink {
        pub(crate) fn lines(&self) -> Vec<Written> {
            self.0.lock().expect("sink poisoned").clone()
        }
    }

    impl LogSink for MemorySink {
        fn write(&self, meta: &Metadata<'_>, tag: &'static str, line: &str) {
            self.0
                .lock()
                .expect("sink poisoned")
                .push((*meta.level(), tag, line.to_owned()));
        }
    }

    /// A gate the host has already set to `Detailed` — this test's own, never
    /// the process's (the suite runs in parallel in one process).
    pub(crate) fn open_gate() -> Gate {
        let gate = Gate::default();
        gate.set(DeviceLogLevel::Detailed);
        gate
    }

    /// `sink` as the platform arm at `scope`, with a host slot nobody watches —
    /// this test's own.
    pub(crate) fn platform_only(sink: MemorySink, scope: Scope) -> FanOut<MemorySink> {
        FanOut {
            platform: sink,
            platform_scope: scope,
            host: HostSlot::default(),
        }
    }

    /// Run `emit` under the layer AS INSTALLED (behind its filter) with the
    /// host's switch ON, on this thread, and hand back what reached the
    /// platform sink.
    pub(crate) fn installed(scope: Scope, emit: impl FnOnce()) -> Vec<Written> {
        let sink = MemorySink::default();
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            platform_only(sink.clone(), scope),
            open_gate(),
        ));
        tracing::subscriber::with_default(subscriber, emit);
        sink.lines()
    }

    /// Serialises every test that writes the PROCESS gate or the process host
    /// slot. Since S5 that includes every test that calls a wipe verb (a wipe
    /// quiesces the log), so a wipe in one test would otherwise close another
    /// test's stream or turn its gate Off mid-assertion. Since S16's bridge it
    /// also serialises the tests that sever through the process registry
    /// (`convert::SEVERS_IN_FLIGHT`): a sever quiesces the log too, and the
    /// registry's UNRESOLVED gate is process-wide, so one test's sever in flight
    /// would refuse another test's door. A tokio mutex: it does not poison, and
    /// an async test may hold it across an `.await`.
    pub(crate) static PROCESS_LOG: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
}

#[cfg(test)]
mod tests {
    use super::test_sink::{MemorySink, Written, installed, open_gate, platform_only};
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    /// Run `emit` under the BARE layer — no filter in front of `on_event`.
    fn bare(scope: Scope, emit: impl FnOnce()) -> Vec<Written> {
        let sink = MemorySink::default();
        let subscriber = tracing_subscriber::registry().with(DeviceLogLayer::new(
            platform_only(sink.clone(), scope),
            open_gate(),
        ));
        tracing::subscriber::with_default(subscriber, emit);
        sink.lines()
    }

    /// The one event the gate tests drive — the line a host most wants, and an
    /// identifying field so a count is a count of THIS line.
    fn a_dial() {
        tracing::info!(
            target: "zec_wallet_core",
            dial_arm = "host",
            dial_class = "sync",
            outcome = "connected",
            "wallet.dial"
        );
    }

    fn dials(sink: &MemorySink) -> usize {
        sink.lines()
            .iter()
            .filter(|(_, _, line)| line.contains("dial_arm=host"))
            .count()
    }

    /// FR-35 — DEFAULT OFF. A host that says nothing gets ZERO lines, and the
    /// zero is the GATE's: the same event through the same subscriber counts
    /// one the moment the gate opens, so an instrument that simply sees nothing
    /// cannot pass this. `Gate::default()` is what `process_gate()` starts as.
    ///
    /// Watched against: `Gate::default()` constructed open; the gate check
    /// removed from `on_event`.
    #[test]
    fn a_host_that_says_nothing_gets_no_device_log() {
        let sink = MemorySink::default();
        let gate = Gate::default();
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            platform_only(sink.clone(), Scope::AllSdkTargets),
            gate.clone(),
        ));
        tracing::subscriber::with_default(subscriber, || {
            a_dial();
            a_dial();
            assert_eq!(dials(&sink), 0, "the gate starts OFF: {:?}", sink.lines());
            assert_eq!(gate.level(), DeviceLogLevel::Off);
            gate.set(DeviceLogLevel::Detailed);
            a_dial();
            assert_eq!(
                dials(&sink),
                1,
                "the control: the same event, the same subscriber, counted once it is on"
            );
        });
    }

    /// FR-35 — the switch is LIVE in both directions with no re-install: the
    /// count after the host turns it off equals the count at that moment, and
    /// turning it on again resumes the log through the subscriber already in
    /// place. This is the property a filter-side gate would fail — `tracing`
    /// caches a callsite's interest at first hit, so the SAME callsite is driven
    /// in every state here.
    ///
    /// Watched against: the gate read once at construction instead of per event.
    #[test]
    fn the_switch_is_live_in_both_directions_without_a_reinstall() {
        let sink = MemorySink::default();
        let gate = Gate::default();
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            platform_only(sink.clone(), Scope::AllSdkTargets),
            gate.clone(),
        ));
        tracing::subscriber::with_default(subscriber, || {
            use DeviceLogLevel::{Detailed, Off};
            let mut counts = Vec::new();
            for level in [Off, Detailed, Detailed, Off, Off, Detailed] {
                gate.set(level);
                a_dial();
                counts.push(dials(&sink));
            }
            assert_eq!(
                counts,
                [0, 1, 2, 2, 2, 3],
                "off → on → on → off → off → on, one event each, one callsite throughout"
            );
        });
    }

    /// FR-35, the maintainer's three levels. `Errors` carries WARN and above —
    /// "something went wrong and we carried on" — and NOT the INFO lines;
    /// `Detailed` carries both; neither carries DEBUG, because no host level is
    /// a road to DEBUG on a user's phone. One event of each level is driven in
    /// each state through ONE subscriber, so the level really is live.
    ///
    /// Watched against: `ceiling(Errors)` raised to `Level::INFO` (Errors and
    /// Detailed become the same setting); lowered to `Level::ERROR` (a failed
    /// private dial, which is a WARN, vanishes from "errors only").
    #[test]
    fn errors_only_carries_warn_and_above_and_detailed_carries_info_too() {
        use DeviceLogLevel::{Detailed, Errors, Off};
        let sink = MemorySink::default();
        let gate = Gate::default();
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            platform_only(sink.clone(), Scope::AllSdkTargets),
            gate.clone(),
        ));
        let one_of_each = || {
            tracing::debug!(target: "zec_wallet_core", "wallet.debug");
            tracing::info!(target: "zec_wallet_core", "wallet.info");
            tracing::warn!(target: "zec_wallet_core", "wallet.warn");
            tracing::error!(target: "zec_wallet_core", "wallet.error");
        };
        tracing::subscriber::with_default(subscriber, || {
            let mut seen = Vec::new();
            for level in [Off, Errors, Detailed, Errors, Off] {
                let before = sink.lines().len();
                gate.set(level);
                one_of_each();
                let written: Vec<String> = sink.lines()[before..]
                    .iter()
                    .map(|(_, _, line)| line.clone())
                    .collect();
                seen.push((level, written));
            }
            let errors = vec!["wallet.warn".to_owned(), "wallet.error".to_owned()];
            let detailed = vec![
                "wallet.info".to_owned(),
                "wallet.warn".to_owned(),
                "wallet.error".to_owned(),
            ];
            assert_eq!(
                seen,
                [
                    (Off, vec![]),
                    (Errors, errors.clone()),
                    (Detailed, detailed),
                    (Errors, errors),
                    (Off, vec![]),
                ]
            );
        });
    }

    /// The gate's byte: every level round-trips, `Off` is the ZERO value a fresh
    /// gate starts as, and a byte that is not a level reads as `Off`.
    ///
    /// Watched against: `decode`'s wildcard arm returning `Detailed`.
    #[test]
    fn the_gates_byte_round_trips_and_anything_else_is_off() {
        use DeviceLogLevel::{Detailed, Errors, Off};
        for level in [Off, Errors, Detailed] {
            assert_eq!(decode(encode(level)), level);
        }
        assert_eq!(encode(Off), 0, "a zeroed gate is OFF by construction");
        for raw in [3u8, 7, 200, u8::MAX] {
            assert_eq!(decode(raw), Off);
        }
    }

    /// The EFFECTIVE level is what the host asked for ONLY where something is
    /// installed to write it; otherwise `Off`, whatever was asked — the fact the
    /// host's settings row renders (a platform with no sink yet must not claim a
    /// log). Each input is seen alone.
    ///
    /// Watched against: `effective` returning `requested` unconditionally;
    /// returning `Off` unconditionally.
    #[test]
    fn the_effective_level_needs_a_sink_and_is_otherwise_the_hosts_choice() {
        use DeviceLogLevel::{Detailed, Errors, Off};
        for requested in [Off, Errors, Detailed] {
            assert_eq!(effective(false, requested), Off, "nothing can write");
            assert_eq!(effective(true, requested), requested, "installed");
        }
    }

    /// The volume split from the measurement configuration: a span that opens,
    /// records and closes prints NOTHING, and an event inside it prints without
    /// the span's name or fields in front of it.
    ///
    /// Watched against: an `on_close` that writes the span's name.
    #[test]
    fn the_device_log_prints_events_and_never_span_lines() {
        let lines = installed(Scope::AllSdkTargets, || {
            let span = tracing::info_span!(
                target: "zec_wallet_core",
                "wallet.sync",
                from = 10u32,
                to = 20u32,
                outcome = tracing::field::Empty
            );
            let _entered = span.enter();
            tracing::info!(target: "zec_wallet_core", batches = 3u32, outcome = "ok", "wallet.sync");
            span.record("outcome", "advanced");
        });
        assert_eq!(
            lines,
            [(
                Level::INFO,
                CORE_TAG,
                "wallet.sync batches=3 outcome=ok".to_owned()
            )],
            "one line for the one event; the span contributes no line and no prefix"
        );
    }

    /// INFO and above — on the BARE layer, so it is `on_event`'s own level check
    /// that is watched and not the filter's.
    ///
    /// Watched against: `DEVICE_LOG_MAX_VERBOSITY` raised to `Level::DEBUG`.
    #[test]
    fn the_device_log_drops_everything_below_info() {
        let lines = bare(Scope::AllSdkTargets, || {
            tracing::trace!(target: "zec_wallet_core", "wallet.trace");
            tracing::debug!(target: "zec_wallet_core", dropped = 2u32, "wallet.dial.addrs_truncated");
            tracing::info!(target: "zec_wallet_core", "wallet.info");
            tracing::warn!(target: "zec_wallet_core", "wallet.private_path_fell_back");
            tracing::error!(target: "zec_wallet_core", "wallet.error");
        });
        let seen: Vec<(Level, &str)> = lines.iter().map(|(l, _, s)| (*l, s.as_str())).collect();
        assert_eq!(
            seen,
            [
                (Level::INFO, "wallet.info"),
                (Level::WARN, "wallet.private_path_fell_back"),
                (Level::ERROR, "wallet.error"),
            ]
        );
    }

    /// THE RUNTIME BELT, name gate: a field the allowlist does not carry never
    /// reaches the sink — and the line says something was withheld, so the gap
    /// is visible on the device that hit it.
    ///
    /// Watched against: `put` writing every field regardless of the policy.
    #[test]
    fn a_field_outside_the_allowlist_is_withheld_and_counted() {
        let lines = installed(Scope::AllSdkTargets, || {
            tracing::info!(
                target: "zec_wallet_core",
                host = "zec.rocks",
                port = 443u16,
                outcome = "connected",
                "wallet.dial"
            );
        });
        assert_eq!(
            lines,
            [(
                Level::INFO,
                CORE_TAG,
                "wallet.dial withheld=2 outcome=connected".to_owned()
            )]
        );
        let (_, _, line) = &lines[0];
        assert!(
            !line.contains("zec.rocks") && !line.contains("443"),
            "neither the refused name nor its value may appear: {line}"
        );
    }

    /// THE RUNTIME BELT, value scan: an ALLOWLISTED name does not bless what is
    /// put in it. `error` is the name the policy itself calls highest-risk.
    ///
    /// Watched against: the value dropped from the predicate's arguments
    /// (`field_is_loggable(name, "")`).
    #[test]
    fn a_forbidden_value_under_an_allowlisted_name_is_withheld() {
        let lines = installed(Scope::AllSdkTargets, || {
            tracing::warn!(
                target: "zec_wallet_core",
                error = %"connect https://user:pw@192.0.2.1:443 refused",
                "wallet.sync"
            );
            // …and a message that interpolates one is withheld as a message.
            tracing::warn!(target: "zec_wallet_core", "sent to address {}", "u1qqqq");
        });
        let text: Vec<&str> = lines.iter().map(|(_, _, s)| s.as_str()).collect();
        assert_eq!(
            text,
            ["wallet.sync withheld=1", "<message withheld> withheld=1"]
        );
    }

    /// The scope, on the BARE layer: a dependency's event — the crates that log
    /// addresses and txids at info — is dropped by `on_event` itself, with no
    /// filter in front of it. So is a crate that merely BORROWS the SDK's prefix.
    ///
    /// Watched against: `is_sdk_target(target)` replaced by `true` in `prints`.
    #[test]
    fn dependency_crate_events_are_still_dropped() {
        let lines = bare(Scope::AllSdkTargets, || {
            tracing::info!(target: "zcash_client_backend::scanning", outcome = "ok", "scanned");
            tracing::warn!(target: "h2::proto", "frame");
            tracing::error!(target: "tonic::transport", "channel");
            tracing::info!(target: "zec_wallet_tor::dialer", "connect");
            tracing::info!(target: "zec_wallet_corex", "borrowed");
            tracing::info!(target: "zec_wallet_core::net::dialer", "ours");
            // No `target:` — this crate's MODULE path, which is the bridge's.
            tracing::info!("the bridge's own module path");
        });
        let seen: Vec<(&str, &str)> = lines.iter().map(|(_, t, s)| (*t, s.as_str())).collect();
        assert_eq!(
            seen,
            [
                (CORE_TAG, "ours"),
                (BRIDGE_TAG, "the bridge's own module path"),
            ]
        );
    }

    /// A layer that writes EVERY event it is handed, checking nothing — what
    /// stands behind the production filter in the test below, so that a hole in
    /// the filter has nothing to hide behind.
    struct PrintEverything(MemorySink);

    impl<S: Subscriber> Layer<S> for PrintEverything {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut line = LineBuilder::default();
            event.record(&mut line);
            self.0.write(event.metadata(), "unchecked", &line.finish());
        }
    }

    /// The FILTER alone — the conjunct the test above cannot see, because there
    /// `on_event`'s own check would mask a filter that let everything through.
    /// Here the layer behind the production filter prints whatever reaches it,
    /// so the filter is the only thing between a dependency's event and the
    /// sink. (In production this same filter is what makes a dependency
    /// callsite `Interest::never()`: with one subscriber, all of whose layers
    /// are per-layer-filtered, a callsite no filter wants is disabled at the
    /// callsite cache and its event is never built.)
    ///
    /// Watched against: `device_log_filter` returning `filter_fn(|_| true)`.
    #[test]
    fn the_filter_alone_refuses_dependencies_and_everything_below_info() {
        let sink = MemorySink::default();
        let subscriber = tracing_subscriber::registry()
            .with(PrintEverything(sink.clone()).with_filter(device_log_filter()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!(target: "h2::proto", "frame");
            tracing::info!(target: "zcash_client_sqlite", "stored");
            tracing::info!(target: "zec_wallet_tor", "connect");
            tracing::debug!(target: "zec_wallet_core", "wallet.debug");
            tracing::info!(target: "zec_wallet_core", "wallet.info");
            tracing::warn!(target: "zec_wallet", "wallet.host_dialer_retired");
            tracing::error!(target: "wallet.consensus", "wallet.consensus");
        });
        let text: Vec<String> = sink.lines().into_iter().map(|(_, _, s)| s).collect();
        assert_eq!(
            text,
            [
                "wallet.info",
                "wallet.host_dialer_retired",
                "wallet.consensus"
            ]
        );
    }

    /// Beside the debug/measurement fmt layer this one leaves the core's MODULE
    /// target alone (that layer prints it, louder) and still carries what that
    /// layer's `Targets` filter never matched: the bridge's events and the
    /// core's `wallet.<surface>` families.
    ///
    /// Watched against: `Scope::BesideTheLoudLayer => true`.
    #[test]
    fn beside_the_loud_layer_the_core_module_target_is_left_to_it() {
        let lines = installed(Scope::BesideTheLoudLayer, || {
            tracing::info!(target: "zec_wallet_core", "wallet.dial");
            tracing::info!(target: "zec_wallet_core::store", "module path");
            tracing::info!(target: "zec_wallet", "wallet.host_dialer_registered");
            tracing::warn!(target: "wallet.consensus", outcome = "capable_time_untrusted", "wallet.consensus");
        });
        let seen: Vec<(&str, &str)> = lines.iter().map(|(_, t, s)| (*t, s.as_str())).collect();
        assert_eq!(
            seen,
            [
                (BRIDGE_TAG, "wallet.host_dialer_registered"),
                (CORE_TAG, "wallet.consensus outcome=capable_time_untrusted"),
            ]
        );
    }

    /// A value cannot forge a second log record or repaint this one, and one
    /// line is bounded. The separators are the ones `char::is_control` MISSES:
    /// U+2028/U+2029 split lines for any `splitlines()`-based parser, and the
    /// bidi overrides let a value DISPLAY a different line than it is.
    ///
    /// Watched against: the map reduced to `c.is_control()` (the U+2028 and
    /// U+202E cases red); the cap removed.
    #[test]
    fn a_line_is_one_line_and_it_is_bounded() {
        let long = "x".repeat(4 * DEVICE_LOG_LINE_MAX_BYTES);
        let lines = installed(Scope::AllSdkTargets, || {
            for breaker in [
                "\n", "\r", "\0", "\u{2028}", "\u{2029}", "\u{202e}", "\u{2066}",
            ] {
                let forged =
                    format!("refused{breaker}I zec_wallet_core: wallet.dial dial_arm=host");
                tracing::warn!(target: "zec_wallet_core", error = %forged, "wallet.sync");
            }
            tracing::warn!(target: "zec_wallet_core", error = %long, "wallet.sync");
        });
        assert_eq!(lines.len(), 8);
        for (_, _, line) in &lines[..7] {
            assert!(
                line.chars().all(|c| c.is_ascii_graphic() || c == ' '),
                "only printable ASCII reaches the sink: {line:?}"
            );
            assert!(
                line.starts_with("wallet.sync error=refused I zec_wallet_core:"),
                "the breaker became a space and the value stayed on its line: {line:?}"
            );
        }
        let (_, _, capped) = &lines[7];
        assert!(
            capped.len() <= DEVICE_LOG_LINE_MAX_BYTES + "...".len() && capped.ends_with("..."),
            "a long value is cut at the cap and says so: {} bytes",
            capped.len()
        );
    }

    /// The cap cuts from the END, so the one thing that must survive it comes
    /// FIRST: a long open-ended value beside a refused field is exactly the case
    /// the cap exists for, and a marker the cap erased would read as "nothing
    /// was withheld".
    ///
    /// Watched against: `withheld=` written after the fields.
    #[test]
    fn the_withheld_marker_survives_the_cap() {
        let long = "x".repeat(4 * DEVICE_LOG_LINE_MAX_BYTES);
        let lines = installed(Scope::AllSdkTargets, || {
            tracing::warn!(
                target: "zec_wallet_core",
                error = %long,
                host = "zec.rocks",
                "wallet.sync"
            );
        });
        let (_, _, line) = &lines[0];
        assert!(
            line.starts_with("wallet.sync withheld=1 error=xxx") && line.ends_with("..."),
            "{line}"
        );
    }

    /// THE APPLE SINK, END TO END, on a real Apple OS: a line goes through the
    /// REAL layer and the REAL `os_log` sink, and is read back out of the unified
    /// log by the tool a person would use. `#[ignore]`d because it shells out to
    /// `log show` (seconds, and only on macOS) — `just
    /// wallet-device-log-apple-witness` runs it; it is the macOS stand-in for the
    /// iPhone walk, which needs the maintainer's hands.
    ///
    /// `log show` runs WITHOUT `--info`, and that is the point: it then prints
    /// only the PERSISTED types. `os_log`'s `info` type lives in memory only, so
    /// an INFO line filed there is gone by the time a tester's phone is plugged
    /// in — this asserts the INFO line was filed where it can still be read.
    ///
    /// Watched against: INFO filed as `oslog::Level::Info` (the INFO marker
    /// vanishes from the persisted store; the WARN one stays).
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "shells out to `log show`; run by `just wallet-device-log-apple-witness`"]
    fn the_apple_sink_writes_lines_the_unified_log_keeps() {
        // `count` is an allowlisted name; the value makes THIS run's lines
        // findable among any other run's.
        let marker = u64::from(std::process::id()) * 1_000_000
            + u64::from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock after the epoch")
                    .subsec_micros(),
            );
        let sink = OsLogSink::new().expect("the OS hands out a log handle");
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            FanOut {
                platform: sink,
                platform_scope: Scope::AllSdkTargets,
                host: HostSlot::default(),
            },
            open_gate(),
        ));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "zec_wallet_core", count = marker, "wallet.device_log_witness");
            tracing::warn!(target: "zec_wallet", count = marker, "wallet.device_log_witness");
        });

        let needle = format!("wallet.device_log_witness count={marker}");
        let mut shown = String::new();
        // The log daemon commits asynchronously; ask again rather than sleep.
        for _attempt in 0..20 {
            let out = std::process::Command::new("/usr/bin/log")
                .args([
                    "show",
                    "--last",
                    "2m",
                    "--style",
                    "compact",
                    "--predicate",
                    &format!("subsystem == \"{APPLE_LOG_SUBSYSTEM}\""),
                ])
                .output()
                .expect("`log show` runs");
            shown = String::from_utf8_lossy(&out.stdout).into_owned();
            if shown.matches(&needle).count() >= 2 {
                break;
            }
        }
        let mine: Vec<&str> = shown.lines().filter(|l| l.contains(&needle)).collect();
        assert_eq!(
            mine.len(),
            2,
            "both lines are in the PERSISTED store (no `--info` was passed): {mine:?}"
        );
        // `[subsystem:category]` is how the compact style prints the pair. The
        // bridge's tag is a prefix of the core's AND equals the subsystem, so
        // the whole bracket is matched — a bare `contains(BRIDGE_TAG)` would be
        // true of every line here.
        let under = |tag: &str| format!("[{APPLE_LOG_SUBSYSTEM}:{tag}]");
        assert!(
            mine.iter()
                .any(|l| l.contains(&format!("INFO {needle}")) && l.contains(&under(CORE_TAG))),
            "the INFO line, level in the text, under the core's category: {mine:?}"
        );
        assert!(
            mine.iter()
                .any(|l| l.contains(&format!("WARN {needle}")) && l.contains(&under(BRIDGE_TAG))),
            "the WARN line under the bridge's category: {mine:?}"
        );
    }

    /// A writer that keeps every `write` call as its own chunk, so a line
    /// written in two calls shows as two chunks — stderr is shared with the
    /// host, and a line split across calls can be interleaved with the host's.
    #[derive(Clone, Default)]
    struct Chunks(Arc<Mutex<Vec<String>>>);

    impl std::io::Write for Chunks {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("chunks poisoned")
                .push(String::from_utf8_lossy(buf).into_owned());
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// S5 `stderr` (FR-42): on Linux and Windows the platform arm is stderr,
    /// and each event is ONE write of `"<LEVEL> <tag> <line>\n"`. The level and
    /// the tag are in the text because stderr has no priority or category to
    /// carry them. The line after them is exactly the line the layer hands
    /// every other platform sink. Driven through the real layer, with the
    /// writer as the seam. The two events carry different tags, so the tag is
    /// the event's own.
    ///
    /// Watched against: the level prefix dropped; the tag dropped; the line
    /// written in two calls (the prefix, then the rest).
    #[test]
    fn the_stderr_sink_writes_one_prefixed_line_per_event() {
        let emit = || {
            a_dial();
            tracing::warn!(target: "zec_wallet", outcome = "refused", "wallet.dial");
        };
        let chunks = Chunks::default();
        let writer = chunks.clone();
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            FanOut {
                platform: StderrSink {
                    open: move || writer.clone(),
                },
                platform_scope: Scope::AllSdkTargets,
                host: HostSlot::default(),
            },
            open_gate(),
        ));
        tracing::subscriber::with_default(subscriber, emit);

        let expected: Vec<String> = installed(Scope::AllSdkTargets, emit)
            .into_iter()
            .map(|(level, tag, line)| format!("{level} {tag} {line}\n"))
            .collect();
        assert_eq!(
            expected.len(),
            2,
            "the control: two events print: {expected:?}"
        );
        assert!(
            expected[0].starts_with(&format!("INFO {CORE_TAG} wallet.dial"))
                && expected[1].starts_with(&format!("WARN {BRIDGE_TAG} wallet.dial")),
            "the control: the two events carry the two tags: {expected:?}"
        );
        assert_eq!(
            *chunks.0.lock().expect("chunks poisoned"),
            expected,
            "one write per event, the level and the tag in front of the platform line"
        );
    }

    /// A write that fails — a closed stderr pipe — is dropped: the event does
    /// not panic, and the next event is written. A diagnostic must never take
    /// the wallet's caller down.
    ///
    /// Watched against: the write's error unwrapped.
    #[test]
    fn a_failed_stderr_write_is_ignored() {
        struct Closed;
        impl std::io::Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let opened = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&opened);
        let subscriber = tracing_subscriber::registry().with(device_log_layer(
            FanOut {
                platform: StderrSink {
                    open: move || {
                        count.fetch_add(1, Ordering::Relaxed);
                        Closed
                    },
                },
                platform_scope: Scope::AllSdkTargets,
                host: HostSlot::default(),
            },
            open_gate(),
        ));
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tracing::subscriber::with_default(subscriber, || {
                a_dial();
                a_dial();
            });
        }));
        assert!(run.is_ok(), "a failed stderr write panicked the event");
        assert_eq!(
            opened.load(Ordering::Relaxed),
            2,
            "both events reached the writer"
        );
    }

    // ---- S5 `sink`: the host-registered stream (stage-5 plan §3.1) ----
    //
    // Every test below owns its `Gate` and `HostSlot`, as the gate tests above
    // own their gate: the suite runs in parallel in one process.

    use crate::api::meta::{DeviceLogLine, DeviceLogSeverity};
    use std::sync::atomic::{AtomicU64, AtomicUsize};
    use std::sync::mpsc;
    use std::sync::{Mutex, PoisonError};
    use std::time::{Duration, Instant};

    /// How long a test waits on another thread before calling it stuck. Long
    /// enough never to fire on a loaded machine; a hang would be worse.
    const STUCK: Duration = Duration::from_secs(10);

    /// What one host subscription received — shared between the test and the
    /// [`HostLineSink`] the slot owns, so the test reads it while the slot
    /// holds the sink, and after the slot has dropped it.
    #[derive(Default)]
    struct Received {
        lines: Mutex<Vec<DeviceLogLine>>,
        sends: AtomicUsize,
        /// Set when the slot DROPS its sink — what closes a Dart stream (fact 4).
        closed: AtomicBool,
    }

    #[derive(Clone, Default)]
    struct HostTap(Arc<Received>);

    impl HostTap {
        /// A live subscription: every `send` is delivered.
        fn sink(&self) -> Box<dyn HostLineSink> {
            Box::new(TapSink {
                rx: self.0.clone(),
                alive: true,
            })
        }

        /// A subscription whose Dart side is gone: every `send` answers `false`.
        fn gone_sink(&self) -> Box<dyn HostLineSink> {
            Box::new(TapSink {
                rx: self.0.clone(),
                alive: false,
            })
        }

        fn with_lines<T>(&self, read: impl FnOnce(&[DeviceLogLine]) -> T) -> T {
            read(&self.0.lines.lock().unwrap_or_else(PoisonError::into_inner))
        }

        fn seqs(&self) -> Vec<u64> {
            self.with_lines(|lines| lines.iter().map(|l| l.seq).collect())
        }

        fn texts(&self) -> Vec<String> {
            self.with_lines(|lines| lines.iter().map(|l| l.text.clone()).collect())
        }

        fn sends(&self) -> usize {
            self.0.sends.load(Ordering::SeqCst)
        }

        fn is_closed(&self) -> bool {
            self.0.closed.load(Ordering::SeqCst)
        }
    }

    struct TapSink {
        rx: Arc<Received>,
        alive: bool,
    }

    impl HostLineSink for TapSink {
        fn send(&self, line: DeviceLogLine) -> bool {
            self.rx.sends.fetch_add(1, Ordering::SeqCst);
            if self.alive {
                self.rx
                    .lines
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(line);
            }
            self.alive
        }
    }

    impl Drop for TapSink {
        fn drop(&mut self) {
            self.rx.closed.store(true, Ordering::SeqCst);
        }
    }

    /// The layer AS INSTALLED over a fan-out: `platform` behind
    /// `platform_scope`, the host arm on `slot`, both behind `gate`. A
    /// `Dispatch`, so the threaded tests can share one subscriber.
    fn fanned(
        platform: &MemorySink,
        platform_scope: Scope,
        slot: &HostSlot,
        gate: &Gate,
    ) -> tracing::Dispatch {
        tracing::Dispatch::new(tracing_subscriber::registry().with(device_log_layer(
            FanOut {
                platform: platform.clone(),
                platform_scope,
                host: slot.clone(),
            },
            gate.clone(),
        )))
    }

    fn through(dispatch: &tracing::Dispatch, emit: impl FnOnce()) {
        tracing::dispatcher::with_default(dispatch, emit);
    }

    /// `n` INFO lines on the core's family target, numbered by an allowlisted
    /// field so each is a distinct line.
    fn syncs(n: u32) {
        for count in 0..n {
            tracing::info!(target: "zec_wallet_core", count, outcome = "ok", "wallet.sync");
        }
    }

    fn platform_texts(platform: &MemorySink) -> Vec<String> {
        platform.lines().into_iter().map(|(_, _, s)| s).collect()
    }

    /// The budget's clock, moved by hand, so the budget tests need no sleep.
    #[derive(Clone)]
    struct HandClock {
        base: Instant,
        offset_ms: Arc<AtomicU64>,
    }

    impl HandClock {
        fn new() -> Self {
            Self {
                base: Instant::now(),
                offset_ms: Arc::new(AtomicU64::new(0)),
            }
        }

        fn advance(&self, by: Duration) {
            let ms = u64::try_from(by.as_millis()).expect("a test-sized duration");
            self.offset_ms.fetch_add(ms, Ordering::SeqCst);
        }

        /// A slot whose budget reads this clock.
        fn slot(&self) -> HostSlot {
            let clock = self.clone();
            HostSlot::with_clock(move || {
                clock.base + Duration::from_millis(clock.offset_ms.load(Ordering::SeqCst))
            })
        }
    }

    /// Assertion 1 — the host stream sits behind the host's ONE switch. With a
    /// host sink attached and the gate at its default, an event yields no host
    /// line; the same event, the same subscriber, counts one once the host opens
    /// the gate — so a tap that simply sees nothing cannot pass.
    ///
    /// Watched against: the gate ignored on the host's path — `on_event`'s read
    /// and the host arm's re-read both removed. Either alone is masked here by
    /// the other; the re-read alone is
    /// `a_line_waiting_on_the_host_lock_rereads_the_gate`'s.
    #[test]
    fn a_host_sink_gets_nothing_until_the_host_opens_the_gate() {
        let gate = Gate::default();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, || {
            a_dial();
            a_dial();
            assert_eq!(tap.sends(), 0, "the gate starts OFF: {:?}", tap.texts());
            gate.set(DeviceLogLevel::Detailed);
            a_dial();
        });
        assert_eq!(
            tap.texts(),
            ["wallet.dial dial_arm=host dial_class=sync outcome=connected"],
            "the control: the same event reaches the host once the gate is open"
        );
    }

    /// Assertion 2 — `Errors` is WARN and above on the host arm too, and
    /// `Detailed` adds INFO; DEBUG reaches it at neither.
    ///
    /// Watched against: `ceiling(Errors)` returns INFO.
    #[test]
    fn the_host_arm_gets_the_errors_only_ceiling() {
        let gate = Gate::default();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        let one_of_each = || {
            tracing::debug!(target: "zec_wallet_core", "wallet.debug");
            tracing::info!(target: "zec_wallet_core", "wallet.info");
            tracing::warn!(target: "zec_wallet_core", "wallet.warn");
        };
        through(&dispatch, || {
            gate.set(DeviceLogLevel::Errors);
            one_of_each();
            gate.set(DeviceLogLevel::Detailed);
            one_of_each();
        });
        assert_eq!(
            tap.texts(),
            ["wallet.warn", "wallet.info", "wallet.warn"],
            "Errors: the WARN alone; Detailed: INFO and WARN; DEBUG at neither"
        );
    }

    /// Assertion 3 — the load-bearing premise: the host is handed the SAME
    /// finished line the platform sink is handed, byte for byte, including a
    /// line with a withheld field and one cut at the 512-byte cap (the fixture
    /// the plan prices: a 600-byte `error` beside one refused field).
    ///
    /// Watched against: the host arm re-renders the fields.
    #[test]
    fn the_host_text_is_the_line_the_platform_sink_was_handed() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let platform = MemorySink::default();
        let dispatch = fanned(&platform, Scope::AllSdkTargets, &slot, &gate);
        let long = "x".repeat(600);
        through(&dispatch, || {
            a_dial();
            tracing::warn!(target: "zec_wallet_core", host = "zec.rocks", outcome = "refused", "wallet.dial");
            tracing::warn!(target: "zec_wallet_core", error = %long, host = "zec.rocks", "wallet.sync");
            tracing::info!(target: "zec_wallet", "wallet.host_dialer_registered");
        });
        let handed = platform_texts(&platform);
        assert_eq!(
            handed.len(),
            4,
            "the control: four platform lines {handed:?}"
        );
        assert_eq!(
            tap.texts(),
            handed,
            "the host's text IS the platform's line"
        );
        let capped = &handed[2];
        assert_eq!(
            capped.len(),
            DEVICE_LOG_LINE_MAX_BYTES + "...".len(),
            "the priced fixture really is cut at the cap: {capped}"
        );
        assert!(
            capped.starts_with("wallet.sync withheld=1 error=xxx") && capped.ends_with("..."),
            "{capped}"
        );
    }

    /// Assertion 4 — §5.4 holds on the host arm by construction: a name outside
    /// the allowlist, and a FORBIDDEN value under an allowlisted name, are each
    /// absent from the host line, which says `withheld=1`.
    ///
    /// Watched against: the host arm fed from the raw visitor.
    #[test]
    fn a_forbidden_field_is_withheld_from_the_host_line() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, || {
            tracing::info!(target: "zec_wallet_core", host = "zec.rocks", outcome = "connected", "wallet.dial");
            tracing::warn!(
                target: "zec_wallet_core",
                error = %"connect https://user:pw@192.0.2.1:443 refused",
                "wallet.sync"
            );
        });
        let texts = tap.texts();
        assert_eq!(
            texts,
            [
                "wallet.dial withheld=1 outcome=connected",
                "wallet.sync withheld=1"
            ]
        );
        for text in &texts {
            assert!(
                !text.contains("zec.rocks") && !text.contains("user:pw"),
                "neither the refused name's value nor the forbidden value reaches the host: {text}"
            );
        }
    }

    /// Assertion 5 — on the BARE layer, so it is `on_event`'s own scope check
    /// the host arm relies on and not the filter's: a dependency's event, a
    /// borrowed prefix, and a DEBUG event of ours never reach the host.
    ///
    /// Watched against: `is_sdk_target` bypassed on the host arm.
    #[test]
    fn dependency_and_debug_events_never_reach_the_host() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let subscriber = tracing_subscriber::registry().with(DeviceLogLayer::new(
            FanOut {
                platform: MemorySink::default(),
                platform_scope: Scope::AllSdkTargets,
                host: slot.clone(),
            },
            gate.clone(),
        ));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "zcash_client_backend::scanning", outcome = "ok", "scanned");
            tracing::warn!(target: "h2::proto", "frame");
            tracing::error!(target: "tonic::transport", "channel");
            tracing::info!(target: "zec_wallet_corex", "borrowed");
            tracing::debug!(target: "zec_wallet_core", "wallet.debug");
            tracing::info!(target: "zec_wallet_core", "ours");
        });
        assert_eq!(tap.texts(), ["ours"], "only the SDK's own INFO line");
    }

    /// Assertion 6 — beside the debug/measurement fmt layer, the PLATFORM arm
    /// still leaves the core's module target to it, while the HOST arm gets
    /// every SDK target: a host stream is complete in the builds developers run.
    ///
    /// Watched against: the filter built from `platform_scope`.
    #[test]
    fn beside_the_loud_layer_the_host_still_gets_the_core_module() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let platform = MemorySink::default();
        let dispatch = fanned(&platform, Scope::BesideTheLoudLayer, &slot, &gate);
        through(&dispatch, || {
            tracing::info!(target: "zec_wallet_core::store", "module path");
            tracing::info!(target: "zec_wallet", "wallet.host_dialer_registered");
        });
        assert_eq!(
            tap.texts(),
            ["module path", "wallet.host_dialer_registered"],
            "the host arm gets the core module"
        );
        assert_eq!(
            platform_texts(&platform),
            ["wallet.host_dialer_registered"],
            "the platform arm still leaves the core module to the loud layer"
        );
    }

    /// Assertion 6, the filter's half, in the style of
    /// `the_filter_alone_refuses_dependencies_and_everything_below_info`: the
    /// installed filter, with a layer behind it that prints everything, admits
    /// the core's module target — in every build, because it no longer takes
    /// the platform's scope.
    ///
    /// Watched against: the filter built from `platform_scope`.
    #[test]
    fn the_filter_admits_the_core_module_in_every_build() {
        let sink = MemorySink::default();
        let subscriber = tracing_subscriber::registry()
            .with(PrintEverything(sink.clone()).with_filter(device_log_filter()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "zec_wallet_core::store", "module path");
            tracing::info!(target: "zec_wallet_core", "wallet.dial");
            tracing::info!(target: "zcash_client_sqlite", "stored");
            tracing::debug!(target: "zec_wallet_core::store", "wallet.debug");
        });
        let text: Vec<String> = sink.lines().into_iter().map(|(_, _, s)| s).collect();
        assert_eq!(text, ["module path", "wallet.dial"]);
    }

    /// Assertion 7 — with both arms present, one event is one platform write
    /// and one host send: the fan-out neither doubles nor skips an arm.
    ///
    /// Watched against: the host arm called from both `write` and `on_event`.
    #[test]
    fn one_event_is_one_write_per_arm() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let platform = MemorySink::default();
        let dispatch = fanned(&platform, Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, a_dial);
        assert_eq!(platform.lines().len(), 1, "one platform write");
        assert_eq!(tap.sends(), 1, "one host send");
        assert_eq!(dials(&platform), 1);
    }

    /// Assertion 8 — Relim's Q4 (a): a second registration REPLACES the first.
    /// The first subscription's sink is dropped — its Dart stream closes — and
    /// receives nothing after it; the second receives the next line.
    ///
    /// Watched against: the slot pushes instead of replacing.
    #[test]
    fn a_second_watch_replaces_the_first_and_closes_it() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let first = HostTap::default();
        let second = HostTap::default();
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        slot.watch(first.sink());
        through(&dispatch, a_dial);
        assert!(
            !first.is_closed(),
            "the control: a live subscription is open"
        );
        slot.watch(second.sink());
        assert!(first.is_closed(), "a re-subscribe closes the first stream");
        through(&dispatch, a_dial);
        assert_eq!(first.sends(), 1, "nothing reaches the first after it");
        assert_eq!(second.sends(), 1, "the second takes the next line");
        assert!(!second.is_closed());
    }

    /// Assertion 9 — a host whose Dart side is gone costs the platform log
    /// nothing, and nothing panics. The slot drops the gone sink, so the next
    /// event does not post to it again.
    ///
    /// Watched against: an early return on `send == false`.
    #[test]
    fn a_gone_host_sink_costs_the_platform_line_nothing() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.gone_sink());
        let platform = MemorySink::default();
        let dispatch = fanned(&platform, Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, || {
            a_dial();
            a_dial();
        });
        assert_eq!(dials(&platform), 2, "every platform line is written");
        assert_eq!(
            tap.sends(),
            1,
            "a sink that answered gone is not posted to again"
        );
        assert!(tap.is_closed(), "the slot dropped the gone sink");
    }

    /// Assertion 10 — within one subscription `seq` is strictly increasing, and
    /// a gap is exactly the lines this subscription did not get: one produced
    /// while no sink was registered, and one the budget dropped. After a quiesce
    /// and a re-arm, the new stream's `seq` restarts from 0.
    ///
    /// Watched against: `seq` taken after the budget check; `seq` not reset by
    /// the quiesce.
    #[test]
    fn a_lost_line_is_a_gap_in_seq_and_a_quiesce_restarts_it() {
        assert_eq!(HOST_LINES_PER_MINUTE, 60, "the priced fixture below is 60");
        let clock = HandClock::new();
        let gate = open_gate();
        let slot = clock.slot();
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        // seq 0: produced while the gate is open and no sink is registered.
        through(&dispatch, || syncs(1));
        clock.advance(Duration::from_secs(60));
        let tap = HostTap::default();
        slot.watch(tap.sink());
        // seq 1..=61: the burst admits 60, the budget drops seq 61.
        through(&dispatch, || syncs(61));
        clock.advance(Duration::from_secs(60));
        // seq 62, after the refill.
        through(&dispatch, || syncs(1));
        let seqs = tap.seqs();
        assert!(
            seqs.windows(2).all(|w| w[0] < w[1]),
            "strictly increasing: {seqs:?}"
        );
        let expected: Vec<u64> = (1..=60).chain([62]).collect();
        assert_eq!(
            seqs, expected,
            "the unregistered line is the gap before 1; the budget's is the gap before the last"
        );

        quiesce(&gate, &slot);
        gate.set(DeviceLogLevel::Detailed);
        let rearmed = HostTap::default();
        slot.watch(rearmed.sink());
        through(&dispatch, || syncs(2));
        assert_eq!(
            rearmed.seqs(),
            [0, 1],
            "a stream after a quiesce starts from 0"
        );
    }

    /// Assertion 11 — the host budget: 61 lines in one minute are 60 host lines
    /// and 61 platform lines. The budget is the host arm's alone; the platform
    /// log keeps every line.
    ///
    /// Watched against: the budget applied before the fan-out.
    #[test]
    fn the_host_budget_drops_to_the_host_only() {
        assert_eq!(HOST_LINES_PER_MINUTE, 60, "the priced fixture below is 60");
        let clock = HandClock::new();
        let gate = open_gate();
        let slot = clock.slot();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let platform = MemorySink::default();
        let dispatch = fanned(&platform, Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, || syncs(60));
        assert_eq!(
            tap.seqs().len(),
            60,
            "at 60 in the minute nothing is dropped"
        );
        through(&dispatch, || syncs(1));
        assert_eq!(
            tap.seqs().len(),
            60,
            "the 61st in the same minute is dropped"
        );
        assert_eq!(
            platform.lines().len(),
            61,
            "the platform log has every line"
        );
        clock.advance(Duration::from_secs(60));
        through(&dispatch, || syncs(1));
        assert_eq!(
            tap.seqs().last(),
            Some(&61),
            "after the refill the next line arrives, and its seq shows the dropped one"
        );
        assert_eq!(platform.lines().len(), 62);
    }

    /// Assertion 12 — a sever closes the stream: the quiesce sets the gate
    /// `Off`, drops the host sink (the Dart stream closes), and resets `seq`.
    /// Nothing re-opens the log on its own: the host's level is `Off` and an
    /// event after it reaches no sink. (The process verb, through `wipe`, is
    /// the bridge's `convert.rs` tests.)
    ///
    /// Watched against: the call moved after `validate_db_dir` (convert.rs);
    /// here, a quiesce that leaves the sink in the slot.
    #[test]
    fn a_sever_closes_the_stream() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let platform = MemorySink::default();
        let dispatch = fanned(&platform, Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, || syncs(3));
        assert_eq!(tap.seqs(), [0, 1, 2], "the control: the stream was live");
        quiesce(&gate, &slot);
        assert_eq!(gate.level(), DeviceLogLevel::Off, "the gate is Off");
        assert!(tap.is_closed(), "the host stream is closed");
        through(&dispatch, || syncs(1));
        assert_eq!(tap.sends(), 3, "no line after the sever");
        assert_eq!(platform.lines().len(), 3, "nor on the platform arm");
        // Only the gate re-opened: the slot is still closed to the old sink.
        gate.set(DeviceLogLevel::Detailed);
        through(&dispatch, || syncs(1));
        assert_eq!(
            tap.sends(),
            3,
            "the host stream stays closed until a new watch"
        );
        let rearmed = HostTap::default();
        slot.watch(rearmed.sink());
        through(&dispatch, || syncs(1));
        assert_eq!(
            rearmed.seqs(),
            [1],
            "seq was reset at the quiesce: 0 went to no sink"
        );
    }

    /// A host sink whose FIRST send parks until the test releases it: the
    /// emitting thread is then inside the host arm's critical section, past its
    /// gate re-read, at its send.
    struct ParkingSink {
        rx: Arc<Received>,
        parked: AtomicBool,
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
        /// Set by the test once the quiesce has RETURNED.
        quiesced: Arc<AtomicBool>,
        /// Lines this sink was handed after that.
        late: Arc<AtomicUsize>,
    }

    impl HostLineSink for ParkingSink {
        fn send(&self, line: DeviceLogLine) -> bool {
            if !self.parked.swap(true, Ordering::SeqCst) {
                let _ = self.entered.send(());
                let _ = self
                    .release
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .recv_timeout(STUCK);
            }
            if self.quiesced.load(Ordering::SeqCst) {
                self.late.fetch_add(1, Ordering::SeqCst);
            }
            self.rx.sends.fetch_add(1, Ordering::SeqCst);
            self.rx
                .lines
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(line);
            true
        }
    }

    impl Drop for ParkingSink {
        fn drop(&mut self) {
            self.rx.closed.store(true, Ordering::SeqCst);
        }
    }

    struct Parked {
        tap: HostTap,
        entered: mpsc::Receiver<()>,
        release: mpsc::Sender<()>,
        quiesced: Arc<AtomicBool>,
        late: Arc<AtomicUsize>,
    }

    fn park_on(slot: &HostSlot) -> Parked {
        let tap = HostTap::default();
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let quiesced = Arc::new(AtomicBool::new(false));
        let late = Arc::new(AtomicUsize::new(0));
        slot.watch(Box::new(ParkingSink {
            rx: tap.0.clone(),
            parked: AtomicBool::new(false),
            entered: entered_tx,
            release: Mutex::new(release_rx),
            quiesced: quiesced.clone(),
            late: late.clone(),
        }));
        Parked {
            tap,
            entered,
            release,
            quiesced,
            late,
        }
    }

    /// Assertion 13 — THE BARRIER, Relim's Q4 (b) made exact for the host
    /// stream: one thread is paused inside the host arm at its send while a
    /// second runs the quiesce. The quiesce does not return until the first
    /// thread leaves the lock; the line the first thread held was enqueued
    /// before the quiesce took the lock, so it goes; a third thread's line,
    /// racing the quiesce for the lock, either went before it or never — and
    /// NO line reaches the host sink after the quiesce has returned.
    ///
    /// Watched against: the send moved outside the lock (clone the sink,
    /// unlock, send: revision 2's mechanism).
    #[test]
    fn no_host_line_is_enqueued_after_the_quiesce_returns() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        let parked = park_on(&slot);

        let first = {
            let dispatch = dispatch.clone();
            std::thread::spawn(move || through(&dispatch, a_dial))
        };
        parked
            .entered
            .recv_timeout(STUCK)
            .expect("the host arm never reached its send");

        let (returned_tx, returned) = mpsc::channel();
        let quiescer = {
            let (gate, slot, quiesced) = (gate.clone(), slot.clone(), parked.quiesced.clone());
            std::thread::spawn(move || {
                quiesce(&gate, &slot);
                quiesced.store(true, Ordering::SeqCst);
                let _ = returned_tx.send(());
            })
        };
        let early = returned.recv_timeout(Duration::from_millis(300));
        let third = {
            let dispatch = dispatch.clone();
            std::thread::spawn(move || through(&dispatch, a_dial))
        };
        let _ = parked.release.send(());
        first.join().expect("the first emitter");
        quiescer.join().expect("the quiescer");
        third.join().expect("the third emitter");

        assert!(
            early.is_err(),
            "the quiesce returned while a host send was in flight: the send is not inside the lock"
        );
        assert!(
            parked.quiesced.load(Ordering::SeqCst),
            "the quiesce returned"
        );
        assert_eq!(
            parked.late.load(Ordering::SeqCst),
            0,
            "a line reached the host sink after the quiesce returned"
        );
        assert_eq!(
            parked.tap.seqs().first(),
            Some(&0),
            "the parked line was enqueued before the quiesce took the lock, and it went"
        );
        assert!(parked.tap.is_closed(), "the quiesce dropped the sink");
        assert_eq!(gate.level(), DeviceLogLevel::Off);
    }

    /// NOT in the contract's list (the test author's plant): the host arm
    /// RE-READS the gate under its lock. A line that passed `on_event`'s gate
    /// read while the gate was open, then waited on the host lock while the
    /// host turned the log Off, is not sent. This is the re-read the quiesce's
    /// "no line after it" rests on; assertion 13 cannot see it alone, because
    /// the quiesce also drops the sink.
    ///
    /// Watched against: the host arm's gate re-read removed (the check at the
    /// top of `on_event` only).
    #[test]
    fn a_line_waiting_on_the_host_lock_rereads_the_gate() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        let parked = park_on(&slot);

        let first = {
            let dispatch = dispatch.clone();
            std::thread::spawn(move || through(&dispatch, a_dial))
        };
        parked
            .entered
            .recv_timeout(STUCK)
            .expect("the host arm never reached its send");
        // The second line's field says when it is FORMATTED. `on_event` formats
        // only after its gate read passed, and the next step is the host arm,
        // whose lock the parked send holds. So once this signal arrives, the
        // line is past `on_event`'s read and can only be stopped by the re-read
        // under the lock. No sleep decides that.
        let (formatted_tx, formatted) = mpsc::channel();
        let waiting = {
            let dispatch = dispatch.clone();
            std::thread::spawn(move || {
                let count = Signalled(Mutex::new(formatted_tx));
                through(&dispatch, || {
                    tracing::info!(target: "zec_wallet_core", count = ?count, "wallet.sync");
                });
            })
        };
        formatted
            .recv_timeout(STUCK)
            .expect("the second line never passed on_event's gate read");
        gate.set(DeviceLogLevel::Off);
        let _ = parked.release.send(());
        first.join().expect("the first emitter");
        waiting.join().expect("the waiting emitter");

        assert_eq!(
            parked.tap.texts(),
            ["wallet.dial dial_arm=host dial_class=sync outcome=connected"],
            "only the line sent before the host turned the log Off"
        );
    }

    /// A field value that signals each time it is formatted.
    struct Signalled(Mutex<mpsc::Sender<()>>);

    impl std::fmt::Debug for Signalled {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let _ = self
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .send(());
            f.write_str("2")
        }
    }

    /// A field value that counts how often it is formatted.
    struct Counted(Arc<AtomicUsize>);

    impl std::fmt::Debug for Counted {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fetch_add(1, Ordering::SeqCst);
            f.write_str("7")
        }
    }

    /// Assertion 14 — Relim's Q4 (c): Off stops lines at the SOURCE. With a host
    /// sink attached and the gate Off, the event's field is never formatted;
    /// once the gate is open it is formatted exactly ONCE for both arms — one
    /// `LineBuilder`, one line.
    ///
    /// Watched against: the gate read moved after `event.record`.
    #[test]
    fn off_formats_nothing() {
        let gate = Gate::default();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let platform = MemorySink::default();
        let dispatch = fanned(&platform, Scope::AllSdkTargets, &slot, &gate);
        let formatted = Arc::new(AtomicUsize::new(0));
        let emit = || {
            let value = Counted(formatted.clone());
            tracing::info!(target: "zec_wallet_core", count = ?value, "wallet.sync");
        };
        through(&dispatch, emit);
        assert_eq!(formatted.load(Ordering::SeqCst), 0, "Off formats nothing");
        assert_eq!(tap.sends(), 0);
        gate.set(DeviceLogLevel::Detailed);
        through(&dispatch, emit);
        assert_eq!(
            formatted.load(Ordering::SeqCst),
            1,
            "the control: open, the field is formatted once for both arms"
        );
        assert_eq!(tap.texts(), ["wallet.sync count=7"]);
        assert_eq!(platform_texts(&platform), ["wallet.sync count=7"]);
    }

    /// Assertion 15 — a re-arm after a quiesce delivers again: nothing
    /// re-opens the log on its own, and the host's two calls (level, watch) on
    /// the same gate and slot bring it back.
    ///
    /// Watched against: a quiesce whose close outlives the re-arm (it empties
    /// the host budget, so the re-watched stream gets nothing).
    #[test]
    fn a_rearm_after_quiesce_delivers_again() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let before = HostTap::default();
        slot.watch(before.sink());
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, a_dial);
        quiesce(&gate, &slot);
        through(&dispatch, a_dial);
        assert_eq!(
            before.sends(),
            1,
            "the control: nothing between the quiesce and the re-arm"
        );

        gate.set(DeviceLogLevel::Detailed);
        let after = HostTap::default();
        slot.watch(after.sink());
        through(&dispatch, a_dial);
        assert_eq!(after.seqs(), [0], "the re-armed stream delivers, from 0");
        assert!(!after.is_closed());
    }

    fn severity_name(severity: &DeviceLogSeverity) -> &'static str {
        match severity {
            DeviceLogSeverity::Error => "error",
            DeviceLogSeverity::Warn => "warn",
            DeviceLogSeverity::Info => "info",
        }
    }

    /// Assertion 16 — each host line carries the EVENT's severity (not the
    /// host's level) and the tag `tag_of` gives its target, as the platform
    /// sink gets it.
    ///
    /// Watched against: severity taken from the host level, not the event.
    #[test]
    fn the_host_line_carries_severity_and_tag() {
        let gate = open_gate();
        let slot = HostSlot::default();
        let tap = HostTap::default();
        slot.watch(tap.sink());
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);
        through(&dispatch, || {
            tracing::error!(target: "zec_wallet_core", "wallet.error");
            tracing::warn!(target: "zec_wallet", "wallet.host_dialer_retired");
            tracing::info!(target: "wallet.consensus", "wallet.consensus");
            tracing::info!(target: "zec_wallet::net_dialer_cabi", "wallet.host_dialer_registered");
        });
        let seen: Vec<(&str, String)> = tap.with_lines(|lines| {
            lines
                .iter()
                .map(|l| (severity_name(&l.severity), l.tag.clone()))
                .collect()
        });
        assert_eq!(
            seen,
            [
                ("error", CORE_TAG.to_owned()),
                ("warn", BRIDGE_TAG.to_owned()),
                ("info", CORE_TAG.to_owned()),
                ("info", BRIDGE_TAG.to_owned()),
            ]
        );
    }

    /// A host sink that panics inside the slot's lock.
    struct PanickingSink(Arc<Received>);

    impl HostLineSink for PanickingSink {
        fn send(&self, _line: DeviceLogLine) -> bool {
            self.0.sends.fetch_add(1, Ordering::SeqCst);
            panic!("a host sink that panics inside the slot's lock");
        }
    }

    impl Drop for PanickingSink {
        fn drop(&mut self) {
            self.0.closed.store(true, Ordering::SeqCst);
        }
    }

    /// Assertion 17 — a panic inside a `send` poisons the slot's mutex, and
    /// neither the fail-closed quiesce nor a later event is disarmed by it: the
    /// quiesce still drops the sink and resets `seq`, and a later event posts
    /// nothing and does not panic.
    ///
    /// Watched against: `.lock().unwrap()` on the quiesce path, or on the emit
    /// path.
    #[test]
    fn a_poisoned_slot_still_quiesces_and_posts_nothing() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let gate = open_gate();
        let slot = HostSlot::default();
        let rx = Arc::new(Received::default());
        slot.watch(Box::new(PanickingSink(rx.clone())));
        let dispatch = fanned(&MemorySink::default(), Scope::AllSdkTargets, &slot, &gate);

        let first = catch_unwind(AssertUnwindSafe(|| through(&dispatch, a_dial)));
        assert!(
            first.is_err(),
            "the precondition: the send panicked under the lock"
        );
        assert_eq!(rx.sends.load(Ordering::SeqCst), 1);

        let quiesced = catch_unwind(AssertUnwindSafe(|| quiesce(&gate, &slot)));
        assert!(
            quiesced.is_ok(),
            "the quiesce must not panic on a poisoned slot"
        );
        assert_eq!(gate.level(), DeviceLogLevel::Off);
        assert!(
            rx.closed.load(Ordering::SeqCst),
            "the quiesce dropped the sink"
        );

        // The host opens the gate again but has not re-watched: nothing posts.
        gate.set(DeviceLogLevel::Detailed);
        let later = catch_unwind(AssertUnwindSafe(|| through(&dispatch, a_dial)));
        assert!(
            later.is_ok(),
            "a later event must not panic on a poisoned slot"
        );
        assert_eq!(rx.sends.load(Ordering::SeqCst), 1, "and it posts nothing");

        let rearmed = HostTap::default();
        slot.watch(rearmed.sink());
        let again = catch_unwind(AssertUnwindSafe(|| through(&dispatch, a_dial)));
        assert!(again.is_ok());
        assert_eq!(
            rearmed.seqs(),
            [1],
            "seq was reset by the quiesce (0 went to no sink): {:?}",
            rearmed.seqs()
        );
    }
}
