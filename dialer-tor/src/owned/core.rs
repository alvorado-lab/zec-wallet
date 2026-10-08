//! The lifetime machinery behind [`super::OwnedDialer`] (`tor-plugin.md`
//! §12.2–§12.3): a runtime of the client's own, a ledger of clients that may
//! still write, the poll gate, and the shutdown thread.
//!
//! Generic over the client value `C` held in the slot and the stream `S`
//! behind the gate, and over the shutdown thread's spawner, so the tests drive
//! every rule with probes and no arti.
//!
//! The rule everything here serves: arti writes its guard state in a `Drop`
//! (`tor-circmgr` `CircMgrInner::drop`), on whatever thread drops the last
//! reference. So no reference to the client ever exists off the owned runtime
//! or the shutdown thread: tasks read the slot INSIDE themselves, sync calls
//! run while holding the slot's lock, and the shutdown thread drops the slot's
//! own reference and then shuts the runtime down, inside the budget.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::{Notify, watch};
use tokio::task::{JoinError, JoinHandle};
use tracing::{error, info, warn};

use crate::constants::{
    MAX_SHUTDOWN_BUDGET, OWNED_MAX_BLOCKING_THREADS, OWNED_WORKER_THREADS, SHUTDOWN_WAIT_GRACE,
};
use crate::{ErrorKind, TorDialError};

/// A poisoned lock is recovered, never a panic: a panic in one host call must
/// not turn the shutdown into one (`tor-plugin.md` §12.3, the lock rule).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// How a shutdown ended. Two outcomes, frozen: a shutdown thread that cannot
/// start, or that panics, is folded into [`Shutdown::Overran`] on purpose
/// (fail closed), so a later split of the reasons is a new field, never a
/// third outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    /// Every task the client started has ended and the client has been
    /// dropped: it writes nothing more and opens no connection.
    Stopped,
    /// The runtime did not finish inside the budget, or the shutdown thread
    /// could not start or panicked: the client may still write for the life
    /// of the process, and its ledger entry is never released.
    Overran,
}

/// How many Tor clients may still write, and how many are stuck — one per
/// process per host, held where the host composes its Tor client. A stuck
/// client never leaves the count, and while one exists no new owned client
/// may start (`spawn_owned` answers [`TorDialError::RestartRequired`]).
#[derive(Debug, Default)]
pub struct ClientLedger {
    live: AtomicUsize,
    stuck: AtomicUsize,
    stopping: AtomicUsize,
    /// Notified when `stopping` falls to zero.
    settled: Notify,
    /// The longest a shutdown handed off in this ledger may take to settle
    /// (its running budget plus [`SHUTDOWN_WAIT_GRACE`]), in milliseconds;
    /// only ever raised, so it bounds every shutdown still in flight.
    settle_bound_ms: AtomicU64,
}

impl ClientLedger {
    /// Clients that may still write: counted from the moment their runtime
    /// exists until its shutdown has finished.
    pub fn live(&self) -> usize {
        self.live.load(SeqCst)
    }

    /// Clients whose shutdown overran: they stay counted for the process.
    pub fn stuck(&self) -> usize {
        self.stuck.load(SeqCst)
    }

    /// Shutdowns started and not yet settled (`Stopped` or `Overran`). Each
    /// settles within its budget (its watchdog's clock); the client's entry
    /// is released or stranded BEFORE it leaves this count.
    pub fn stopping(&self) -> usize {
        self.stopping.load(SeqCst)
    }

    /// One shutdown in flight, running with `budget`, counted until the guard
    /// drops at its settlement.
    fn begin_stopping(self: &Arc<Self>, budget: Duration) -> Stopping {
        let bound = budget.saturating_add(SHUTDOWN_WAIT_GRACE);
        let bound_ms = u64::try_from(bound.as_millis()).unwrap_or(u64::MAX);
        self.settle_bound_ms.fetch_max(bound_ms, SeqCst);
        self.stopping.fetch_add(1, SeqCst);
        Stopping(Arc::clone(self))
    }

    /// Wait until no shutdown is in flight; true once none is. Bounded: each
    /// settles within its running budget (its watchdog), and this wait gives
    /// up, answering false, once the longest budget handed off plus
    /// [`SHUTDOWN_WAIT_GRACE`] (a watchdog's thread may start late) has passed
    /// since it began. The bound is re-read on every wake, so a longer
    /// shutdown handed off DURING the wait extends it rather than being cut
    /// short. Takes no timer while nothing is in flight. Cancel-safe: it
    /// holds nothing but its registration, which a drop removes.
    pub(crate) async fn shutdowns_settled(&self) -> bool {
        if self.stopping() == 0 {
            return true;
        }
        let began = tokio::time::Instant::now();
        loop {
            let settled = self.settled.notified();
            tokio::pin!(settled);
            // Registered BEFORE the check, so a fall to zero between the
            // check and the await still wakes it.
            settled.as_mut().enable();
            if self.stopping() == 0 {
                return true;
            }
            let bound = Duration::from_millis(self.settle_bound_ms.load(SeqCst));
            let Some(deadline) = began.checked_add(bound) else {
                settled.await;
                continue;
            };
            if tokio::time::timeout_at(deadline, settled).await.is_err() {
                let bound = Duration::from_millis(self.settle_bound_ms.load(SeqCst));
                let extended = began.checked_add(bound).is_none_or(|d| d > deadline);
                if self.stopping() != 0 && !extended {
                    return false;
                }
            }
        }
    }

    /// One entry, released on drop. [`crate::TorDialer::spawn_owned`] takes
    /// one per client itself; this is public only so a host's own engine
    /// seam (a test double standing in for a client) counts in the SAME
    /// ledger rather than a second counter beside it. A host never takes one
    /// for an owned client.
    pub fn enter(self: &Arc<Self>) -> LedgerEntry {
        self.live.fetch_add(1, SeqCst);
        LedgerEntry(Arc::clone(self))
    }
}

/// One count in a [`ClientLedger`], released on drop.
#[derive(Debug)]
pub struct LedgerEntry(Arc<ClientLedger>);

impl LedgerEntry {
    /// Never release this count: the client may still write. Recorded as
    /// stuck, for the life of the process, and no owned client starts again
    /// in it. The shutdown does this on an overrun; a host strands only an
    /// entry of its own seam whose client it knows may still write.
    pub fn strand(self) {
        self.0.stuck.fetch_add(1, SeqCst);
        std::mem::forget(self);
    }
}

impl Drop for LedgerEntry {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, SeqCst);
    }
}

/// One shutdown in [`ClientLedger::stopping`], released at its settlement.
#[derive(Debug)]
struct Stopping(Arc<ClientLedger>);

impl Drop for Stopping {
    fn drop(&mut self) {
        if self.0.stopping.fetch_sub(1, SeqCst) == 1 {
            // Isolated: a waiter's waker that panics must not unwind through
            // a settlement (or abort one already unwinding).
            let woken = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.0.settled.notify_waiters();
            }));
            if woken.is_err() {
                warn!("a waker of a spawn_owned waiting for a shutdown panicked");
            }
        }
    }
}

/// Starts the shutdown thread. A seam: the tests inject one that fails.
pub(crate) type Spawner = fn(Box<dyn FnOnce() + Send>) -> io::Result<()>;

/// The production spawner: a plain, named std thread.
pub(crate) fn thread_spawner(job: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    std::thread::Builder::new()
        .name("dialer-tor-shutdown".into())
        .spawn(job)
        .map(|_| ())
}

/// The poll gate (`tor-plugin.md` §12.3): `closed` and `in_flight` are a
/// Dekker handshake — a poll increments, then reads the flag; the shutdown
/// sets the flag, then waits for zero — so both are `SeqCst`.
#[derive(Default)]
pub(crate) struct Gate {
    closed: AtomicBool,
    in_flight: AtomicUsize,
    zero_lock: Mutex<()>,
    zero: Condvar,
    /// The latest waker of each pending poll, per stream AND per direction: a
    /// split stream's reader and writer park separately, and neither may
    /// overwrite the other's.
    parked: Mutex<HashMap<(u64, Side), Waker>>,
    next_stream: AtomicU64,
}

/// Which half of a stream a poll belongs to (`poll_flush` and
/// `poll_shutdown` are the write half).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Side {
    Read,
    Write,
}

impl Gate {
    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(SeqCst)
    }

    /// Close the gate; true for the FIRST caller only. Only the flag: the
    /// parked polls are woken by [`Self::wake_parked`] once the shutdown is
    /// handed off, so no host waker runs before it.
    fn close(&self) -> bool {
        !self.closed.swap(true, SeqCst)
    }

    /// Wake every parked poll, so a reader waiting on arti re-polls and meets
    /// the closed gate. Each waker is isolated: one that panics neither stops
    /// the others nor reaches the caller.
    fn wake_parked(&self) {
        let parked: Vec<Waker> = lock(&self.parked).drain().map(|(_, w)| w).collect();
        for waker in parked {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| waker.wake())).is_err() {
                warn!("a waker of a parked stream poll panicked");
            }
        }
    }

    /// Enter for one poll, or `None` once closed. The guard decrements on
    /// drop, so a poll that panics cannot pin the count.
    fn enter(&self) -> Option<PollGuard<'_>> {
        self.in_flight.fetch_add(1, SeqCst);
        let guard = PollGuard(self);
        if self.closed.load(SeqCst) {
            return None;
        }
        Some(guard)
    }

    fn stream_id(&self) -> u64 {
        self.next_stream.fetch_add(1, SeqCst)
    }

    /// Keep a pending poll's waker, so the shutdown can wake it.
    fn park(&self, stream: u64, side: Side, waker: &Waker) {
        lock(&self.parked).insert((stream, side), waker.clone());
        // Closed between the inner poll and the insert: wake it ourselves —
        // after the lock is released, so a waker that polls inline cannot
        // re-enter `park` under it.
        if self.closed.load(SeqCst) {
            let woken = lock(&self.parked).remove(&(stream, side));
            if let Some(w) = woken {
                w.wake();
            }
        }
    }

    fn unpark(&self, stream: u64) {
        let mut parked = lock(&self.parked);
        parked.remove(&(stream, Side::Read));
        parked.remove(&(stream, Side::Write));
    }

    /// Wait until no poll is inside the stream, or the deadline passes.
    /// A `Condvar` wait, notified at zero: no spin.
    fn wait_quiet(&self, deadline: Instant) -> bool {
        let mut held = lock(&self.zero_lock);
        loop {
            if self.in_flight.load(SeqCst) == 0 {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            held = self
                .zero
                .wait_timeout(held, deadline - now)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
    }
}

struct PollGuard<'a>(&'a Gate);

impl Drop for PollGuard<'_> {
    fn drop(&mut self) {
        let gate = self.0;
        if gate.in_flight.fetch_sub(1, SeqCst) == 1 && gate.closed.load(SeqCst) {
            let _held = lock(&gate.zero_lock);
            gate.zero.notify_all();
        }
    }
}

/// Everything a shutdown consumes. Its `Drop` LEAKS a runtime or a client
/// still inside it — that only happens on a failed thread start or an unwind,
/// and dropping a runtime on an async thread panics while dropping the client
/// there could write state off the budget — and settles `Overran` if nothing
/// settled yet, so no path leaves the outcome open.
struct Payload {
    rt: Option<Runtime>,
    client: Option<Box<dyn Any + Send>>,
    gate: Option<Arc<Gate>>,
    budget: Duration,
    terminal: Arc<Terminal>,
}

impl Drop for Payload {
    fn drop(&mut self) {
        std::mem::forget(self.rt.take());
        std::mem::forget(self.client.take());
        self.terminal.settle(Shutdown::Overran);
    }
}

/// The ONE terminal outcome of a shutdown, settled exactly once by whoever
/// gets there first: the shutdown's worker when it finishes, its watchdog at
/// the budget, or an awaiting caller past its own bound. It exists from the
/// moment the client starts, so every caller of `shutdown` holds it, however
/// it races the first. Settling COMMITS under one lock: the ledger entry is
/// released or stranded and the outcome recorded together, and `settle`
/// returns the committed outcome, the winner's or its own. So no caller
/// answers `Overran` while `stuck()` is zero, and no two callers answer
/// differently.
pub(crate) struct Terminal(Mutex<Settle>);

enum Settle {
    Open(Finish),
    Settled(Shutdown),
}

impl Terminal {
    fn new(finish: Finish) -> Arc<Self> {
        Arc::new(Self(Mutex::new(Settle::Open(finish))))
    }

    /// Settle `outcome` unless something settled first; either way, return
    /// the outcome that is committed.
    fn settle(&self, outcome: Shutdown) -> Shutdown {
        let mut state = lock(&self.0);
        if let Settle::Settled(committed) = *state {
            return committed;
        }
        let Settle::Open(mut finish) = std::mem::replace(&mut *state, Settle::Settled(outcome))
        else {
            unreachable!("checked open under the same lock");
        };
        finish.commit(outcome);
        let stopping = finish.stopping.take();
        drop(state);
        // Published, then released from `stopping`, both after the lock, so
        // a waker that runs inline cannot re-enter it; whoever sees either
        // sees the ledger entry already released or stranded. Publishing
        // first: nothing after the publish can make it disagree with the
        // ledger, and a new client waits for the release a moment longer.
        finish.publish(outcome);
        drop(stopping);
        outcome
    }

    /// Count this shutdown in [`ClientLedger::stopping`] until it settles.
    fn hold(&self, stopping: Stopping) {
        let mut state = lock(&self.0);
        if let Settle::Open(finish) = &mut *state {
            finish.stopping = Some(stopping);
            return;
        }
        drop(state);
        drop(stopping);
    }
}

/// Releases or strands the ledger entry and publishes the outcome — exactly
/// once. Dropped unsettled (a panic), it strands and publishes `Overran`, so
/// no caller waits forever.
struct Finish {
    entry: Option<LedgerEntry>,
    publish: Option<Arc<watch::Sender<Option<Shutdown>>>>,
    /// Held from the hand-off to the settlement.
    stopping: Option<Stopping>,
}

impl Finish {
    /// The ledger half, made while the [`Terminal`]'s lock is held.
    fn commit(&mut self, outcome: Shutdown) {
        if let Some(entry) = self.entry.take() {
            match outcome {
                Shutdown::Stopped => drop(entry),
                Shutdown::Overran => entry.strand(),
            }
        }
    }

    fn publish(&mut self, outcome: Shutdown) {
        if let Some(tx) = self.publish.take() {
            // `send_replace`: succeeds with no receiver yet, so a caller that
            // subscribes later still reads the outcome.
            tx.send_replace(Some(outcome));
        }
        // Logged after the publish: a log subscriber that panics cannot hold
        // the outcome back from a waiting caller.
        match outcome {
            Shutdown::Overran => warn!("a Tor client did not finish shutting down in time"),
            // A clean stop is logged too: a host's evidence that Tor stopped.
            Shutdown::Stopped => info!("a Tor client shut down"),
        }
    }
}

impl Drop for Finish {
    fn drop(&mut self) {
        if self.entry.is_some() || self.publish.is_some() || self.stopping.is_some() {
            self.commit(Shutdown::Overran);
            self.publish(Shutdown::Overran);
            drop(self.stopping.take());
        }
    }
}

/// The shutdown's work, on its own thread: wait for the gate, drop the client
/// (arti's drop-time state write, which has no bound of its own), then shut
/// the runtime down with what is left of the budget. Reports whether it
/// overran; its `Drop` leaks what an unwind left inside it.
struct Work {
    rt: Option<Runtime>,
    client: Option<Box<dyn Any + Send>>,
    gate: Option<Arc<Gate>>,
    begun: Instant,
    budget: Duration,
}

impl Drop for Work {
    fn drop(&mut self) {
        std::mem::forget(self.rt.take());
        std::mem::forget(self.client.take());
    }
}

impl Work {
    fn run(mut self) -> bool {
        let deadline = self.begun + self.budget;
        let mut overran = false;
        if let Some(gate) = self.gate.take()
            && !gate.wait_quiet(deadline)
        {
            // A poll still inside arti at the budget: the shutdown goes on
            // regardless (the overrun's trade-off). That poll may then fail,
            // or panic on the host's own polling thread, when the runtime
            // under it goes; waiting longer would hold the shutdown on a host
            // poll that may never return.
            overran = true;
        }
        drop(self.client.take());
        if let Some(rt) = self.rt.take() {
            // Returns once every worker has exited, i.e. dropped every task;
            // on an overrun it returns at the bound, silently.
            rt.shutdown_timeout(deadline.saturating_duration_since(Instant::now()));
        }
        // `shutdown_timeout` reports nothing, so elapsed time is the only
        // overrun signal; `>=` on purpose, so the doubtful case fails closed.
        overran || self.begun.elapsed() >= self.budget
    }
}

impl Payload {
    /// On the shutdown thread, which is the WATCHDOG: start the work on a
    /// thread of its own (under the construction dispatcher), then settle
    /// whichever comes first — the work's own outcome, or `Overran` at the
    /// budget. A work thread that cannot start or that panics settles
    /// `Overran` too.
    fn run(mut self, dispatch: tracing::Dispatch, spawner: Spawner) {
        let begun = Instant::now();
        let work = Work {
            rt: self.rt.take(),
            client: self.client.take(),
            gate: self.gate.take(),
            begun,
            budget: self.budget,
        };
        let (done, outcome) = std::sync::mpsc::channel::<bool>();
        let job: Box<dyn FnOnce() + Send> = Box::new(move || {
            let _log = tracing::dispatcher::set_default(&dispatch);
            let overran = work.run();
            let _ = done.send(overran);
        });
        if spawner(job).is_ok() {
            let left = (begun + self.budget).saturating_duration_since(Instant::now());
            let settled = match outcome.recv_timeout(left) {
                Ok(false) => Shutdown::Stopped,
                Ok(true) | Err(_) => Shutdown::Overran,
            };
            self.terminal.settle(settled);
        }
        // A failed start dropped the job, and the work's runtime and client
        // with it, here (leaked by `Work::drop`); dropping `self` settles
        // `Overran` if nothing settled.
    }
}

/// Hand a payload to the shutdown thread, under `dispatch`. If the thread
/// cannot start, the payload is taken back here and abandoned: runtime and
/// client leaked, entry stranded, `Overran` published.
fn hand_off(payload: Payload, dispatch: tracing::Dispatch, spawner: Spawner) {
    let shared = Arc::new(Mutex::new(Some(payload)));
    let for_thread = Arc::clone(&shared);
    let job: Box<dyn FnOnce() + Send> = Box::new(move || {
        let _log = tracing::dispatcher::set_default(&dispatch);
        let taken = lock(&for_thread).take();
        if let Some(payload) = taken {
            payload.run(dispatch.clone(), spawner);
        }
    });
    if spawner(job).is_err() {
        // Dropping the payload leaks what it holds and settles `Overran`.
        let payload = lock(&shared).take();
        drop(payload);
    }
}

/// The owned runtime as a value that cannot be lost: from the moment the
/// runtime exists it holds its ledger entry, and its `Drop` (a mint cancelled
/// half-way, a value dropped without a shutdown) takes the shutdown-thread
/// path, so a runtime is never dropped on an async thread and never left
/// running unreachable.
pub(crate) struct OwnedRuntime {
    rt: Option<Runtime>,
    handle: Handle,
    entry: Option<LedgerEntry>,
    /// Where the shutdown is counted while in flight.
    ledger: Arc<ClientLedger>,
    drop_budget: Duration,
    /// The log dispatcher of the thread that built it, never the dropping
    /// thread's: a drop on a host thread must not send the client's logs to
    /// that host's global subscriber.
    dispatch: tracing::Dispatch,
    spawner: Spawner,
}

impl OwnedRuntime {
    pub(crate) fn new(
        ledger: &Arc<ClientLedger>,
        thread_name: &'static str,
        drop_budget: Duration,
        spawner: Spawner,
    ) -> io::Result<Self> {
        let dispatch = tracing::dispatcher::get_default(Clone::clone);
        let for_threads = dispatch.clone();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(OWNED_WORKER_THREADS)
            .max_blocking_threads(OWNED_MAX_BLOCKING_THREADS)
            .thread_name(thread_name)
            .enable_all()
            .on_thread_start(move || {
                // Held for the thread's life: the guard is leaked on purpose.
                std::mem::forget(tracing::dispatcher::set_default(&for_threads));
            })
            .build()?;
        let handle = rt.handle().clone();
        Ok(Self {
            rt: Some(rt),
            handle,
            entry: Some(ledger.enter()),
            ledger: Arc::clone(ledger),
            drop_budget,
            dispatch,
            spawner,
        })
    }

    /// The [`Terminal`] of an owned client, holding this runtime's ledger
    /// entry from here on; made when the client starts, before any caller can
    /// see a shutdown begin.
    fn terminal(&mut self, publish: Arc<watch::Sender<Option<Shutdown>>>) -> Arc<Terminal> {
        Terminal::new(Finish {
            entry: self.entry.take(),
            publish: Some(publish),
            stopping: None,
        })
    }

    /// Hand the shutdown off; it settles `terminal`, which an awaiting caller
    /// settles too if it gives up first. The caller has already counted it
    /// in `stopping` (`Owned::start_shutdown`, before the gate closes).
    fn shut_down(
        mut self,
        budget: Duration,
        client: Option<Box<dyn Any + Send>>,
        gate: Option<Arc<Gate>>,
        terminal: Arc<Terminal>,
    ) {
        let payload = Payload {
            rt: self.rt.take(),
            client,
            gate,
            budget,
            terminal,
        };
        hand_off(payload, self.dispatch.clone(), self.spawner);
    }
}

impl Drop for OwnedRuntime {
    fn drop(&mut self) {
        if self.rt.is_some() {
            let budget = self.drop_budget.min(MAX_SHUTDOWN_BUDGET);
            let payload = Payload {
                rt: self.rt.take(),
                client: None,
                gate: None,
                budget,
                terminal: Terminal::new(Finish {
                    entry: self.entry.take(),
                    publish: None,
                    stopping: Some(self.ledger.begin_stopping(budget)),
                }),
            };
            hand_off(payload, self.dispatch.clone(), self.spawner);
        }
    }
}

/// A client on its owned runtime. Tasks hold this (`Arc<Owned<C>>`); the host
/// holds a [`Host`], whose last drop starts the shutdown — so a task that never
/// ends (a pending bootstrap) cannot keep the client alive.
pub(crate) struct Owned<C: Send + Sync + 'static> {
    /// The client. NEVER cloned off the owned runtime (the module's rule).
    client: Mutex<Option<Arc<C>>>,
    handle: Handle,
    runtime: Mutex<Option<OwnedRuntime>>,
    gate: Arc<Gate>,
    outcome: Arc<watch::Sender<Option<Shutdown>>>,
    /// The shutdown's one outcome, from the client's start: an awaiting
    /// caller that gives up settles it here, never on its own.
    terminal: Arc<Terminal>,
    /// The budget the shutdown runs with: the first caller's, fixed BEFORE
    /// the gate closes. Every caller's wait is bounded by it, never by its
    /// own budget, so a later caller with a shorter one cannot strand a
    /// shutdown that finishes inside the budget it runs with.
    running_budget: OnceLock<Duration>,
    /// Where the shutdown is counted in `stopping`, from BEFORE the gate
    /// closes until it settles.
    ledger: Arc<ClientLedger>,
    drop_budget: Duration,
}

impl<C: Send + Sync + 'static> Owned<C> {
    /// An owned client with an EMPTY slot, and the host's handle on it. The
    /// handle exists from here on, so a mint cancelled before it finished
    /// still drops it and so starts the shutdown.
    pub(crate) fn start(mut rt: OwnedRuntime) -> (Arc<Self>, Host<C>) {
        let drop_budget = rt.drop_budget;
        let outcome = Arc::new(watch::Sender::new(None));
        let terminal = rt.terminal(Arc::clone(&outcome));
        let ledger = Arc::clone(&rt.ledger);
        let owned = Arc::new(Self {
            client: Mutex::new(None),
            handle: rt.handle.clone(),
            runtime: Mutex::new(Some(rt)),
            gate: Arc::new(Gate::default()),
            outcome,
            terminal,
            running_budget: OnceLock::new(),
            ledger,
            drop_budget,
        });
        let host = Host(Arc::clone(&owned));
        (owned, host)
    }

    pub(crate) fn gate(&self) -> &Arc<Gate> {
        &self.gate
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.gate.is_closed()
    }

    /// Build the client ON the owned runtime and INSTALL it there: the client
    /// never crosses to the caller, not even as a task's output a cancelled
    /// caller would drop on its own thread.
    pub(crate) async fn mint<F>(self: &Arc<Self>, build: F) -> Result<(), TorDialError>
    where
        F: Future<Output = Result<C, TorDialError>> + Send + 'static,
    {
        let owned = Arc::clone(self);
        let task = self.handle.spawn(async move {
            let client = build.await?;
            let mut slot = lock(&owned.client);
            if owned.gate.is_closed() {
                // Shut down while we built: dropped here, on the worker, and
                // after the slot's lock is released, so a `start_shutdown`
                // waiting for that lock is not held behind arti's drop-time
                // state write.
                drop(slot);
                drop(client);
                return Err(TorDialError::Closed);
            }
            *slot = Some(Arc::new(client));
            Ok(())
        });
        joined(AbortOnDrop(task).await)
    }

    /// Run `call` ON the owned runtime with the client read inside the task.
    /// Dropping the returned future aborts the task (a caller that gives up
    /// leaves no work behind).
    pub(crate) async fn call<T, F, Fut>(self: &Arc<Self>, call: F) -> Result<T, TorDialError>
    where
        F: FnOnce(Arc<C>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, TorDialError>> + Send + 'static,
        T: Send + 'static,
    {
        if self.gate.is_closed() {
            return Err(TorDialError::Closed);
        }
        let owned = Arc::clone(self);
        let task = self.handle.spawn(async move {
            let client = lock(&owned.client).clone();
            drop(owned);
            match client {
                Some(client) => call(client).await,
                None => Err(TorDialError::Closed),
            }
        });
        joined(AbortOnDrop(task).await)
    }

    /// A synchronous call, made while holding the slot's lock.
    pub(crate) fn with<R>(&self, call: impl FnOnce(&C) -> R) -> Result<R, TorDialError> {
        let slot = lock(&self.client);
        match slot.as_deref() {
            Some(client) => Ok(call(client)),
            None => Err(TorDialError::Closed),
        }
    }

    /// The whole hand-off, synchronous, so no cancellation can strand it.
    /// Idempotent: only the first call does anything, and the budget the
    /// shutdown runs with is the first one recorded. The shutdown is counted
    /// in `stopping` BEFORE the gate closes, so every caller that sees the
    /// gate closed, the first or a later one racing it, returns with the
    /// shutdown already counted: a `spawn_owned` it starts next waits for it.
    pub(crate) fn start_shutdown(&self, budget: Duration) {
        if self.gate.is_closed() {
            return;
        }
        let budget = *self
            .running_budget
            .get_or_init(|| budget.min(MAX_SHUTDOWN_BUDGET));
        let stopping = self.ledger.begin_stopping(budget);
        if !self.gate.close() {
            // Another caller closed it first, and counted it first.
            drop(stopping);
            return;
        }
        self.terminal.hold(stopping);
        // Lock order: the client slot, then the runtime slot — everywhere.
        let client = lock(&self.client).take();
        let runtime = lock(&self.runtime).take();
        match runtime {
            Some(rt) => rt.shut_down(
                budget,
                client.map(|c| Box::new(c) as Box<dyn Any + Send>),
                Some(Arc::clone(&self.gate)),
                Arc::clone(&self.terminal),
            ),
            None => {
                // Unreachable by construction; never leave a waiter hanging.
                std::mem::forget(client);
                self.terminal.settle(Shutdown::Overran);
            }
        }
        // Only now that the shutdown is handed off can a host waker run.
        self.gate.wake_parked();
    }

    /// Start the shutdown NOW, then return a future of its one outcome. The
    /// hand-off happens before this returns, so a future that is dropped, or
    /// never polled, loses nothing. The shutdown's watchdog settles
    /// `Overran` at the budget itself; this wait is bounded too, by the
    /// budget the shutdown RUNS with (the first caller's, whatever `budget`
    /// this caller passed) plus [`SHUTDOWN_WAIT_GRACE`] (the watchdog's clock
    /// starts only once its thread runs). A caller past that bound settles
    /// `Overran` through the ONE [`Terminal`] — stranding the entry first —
    /// and answers what it committed, so it never disagrees with the ledger
    /// or another caller. Needs a tokio timer in the caller's context.
    ///
    /// Called from a task on the client's OWN runtime (a misuse: the shutdown
    /// cancels its caller), it still starts the shutdown, logs an error, and
    /// answers `Overran` at once, the same in every build.
    pub(crate) fn shutdown(
        &self,
        budget: Duration,
    ) -> impl Future<Output = Shutdown> + Send + 'static {
        let on_own_runtime =
            Handle::try_current().is_ok_and(|current| current.id() == self.handle.id());
        self.start_shutdown(budget);
        if on_own_runtime {
            error!("a Tor client's shutdown was awaited on its own runtime; answered Overran");
        }
        let running = self
            .running_budget
            .get()
            .copied()
            .unwrap_or(budget.min(MAX_SHUTDOWN_BUDGET));
        let mut outcome = self.outcome.subscribe();
        // Give up through the ONE outcome, never alone: settle `Overran`, and
        // answer what `settle` committed — the winner's outcome if something
        // settled first, even one still being published — so this caller, the
        // ledger and every other caller agree.
        let terminal = Arc::clone(&self.terminal);
        let give_up = move || terminal.settle(Shutdown::Overran);
        async move {
            if on_own_runtime {
                // Fail closed: it cannot wait for its own cancellation.
                return give_up();
            }
            // The wait's borrow of `outcome` ends inside this block.
            let seen = {
                let waited = tokio::time::timeout(
                    running + SHUTDOWN_WAIT_GRACE,
                    outcome.wait_for(Option::is_some),
                )
                .await;
                match waited {
                    Ok(Ok(seen)) => *seen,
                    Ok(Err(_)) | Err(_) => None,
                }
            };
            match seen {
                Some(settled) => settled,
                None => give_up(),
            }
        }
    }
}

impl<C: Send + Sync + 'static> Drop for Owned<C> {
    /// A backstop: the host's handle has already started the shutdown.
    fn drop(&mut self) {
        self.start_shutdown(self.drop_budget);
    }
}

/// The host's handle. Its LAST drop without a shutdown takes the same path,
/// unawaited; the outcome reaches only the ledger.
pub(crate) struct Host<C: Send + Sync + 'static>(Arc<Owned<C>>);

impl<C: Send + Sync + 'static> Host<C> {
    pub(crate) fn owned(&self) -> &Arc<Owned<C>> {
        &self.0
    }
}

impl<C: Send + Sync + 'static> Drop for Host<C> {
    fn drop(&mut self) {
        self.0.start_shutdown(self.0.drop_budget);
    }
}

/// A `JoinHandle` that aborts its task when dropped: the task is then
/// dropped on its worker, as a future polled in place would have been.
struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl<T> Future for AbortOnDrop<T> {
    type Output = Result<T, JoinError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

/// A task's end, as the caller sees it: cancelled by the shutdown is
/// `Closed`; a panic inside the task is `Setup { Internal }`, logged.
fn joined<T>(result: Result<Result<T, TorDialError>, JoinError>) -> Result<T, TorDialError> {
    match result {
        Ok(result) => result,
        Err(e) if e.is_cancelled() => Err(TorDialError::Closed),
        Err(_) => {
            warn!("a task on the Tor client's own runtime panicked");
            Err(TorDialError::Setup {
                kind: ErrorKind::Internal,
            })
        }
    }
}

/// A stream behind the poll gate: once the gate is closed nothing polls the
/// inner stream again, and a poll parked before that is woken to fail.
pub(crate) struct Gated<S> {
    inner: S,
    gate: Arc<Gate>,
    id: u64,
}

impl<S> Gated<S> {
    pub(crate) fn new(inner: S, gate: &Arc<Gate>) -> Self {
        Self {
            inner,
            id: gate.stream_id(),
            gate: Arc::clone(gate),
        }
    }

    /// Poll `inner` through the gate, for one half of the stream.
    fn gated<T>(
        &mut self,
        side: Side,
        cx: &mut Context<'_>,
        poll: impl FnOnce(Pin<&mut S>, &mut Context<'_>) -> Poll<io::Result<T>>,
    ) -> Poll<io::Result<T>>
    where
        S: Unpin,
    {
        let Some(_inside) = self.gate.enter() else {
            return Poll::Ready(Err(closed_stream()));
        };
        let result = poll(Pin::new(&mut self.inner), cx);
        if result.is_pending() {
            self.gate.park(self.id, side, cx.waker());
        }
        result
    }
}

fn closed_stream() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "the Tor client was shut down")
}

impl<S> Drop for Gated<S> {
    fn drop(&mut self) {
        self.gate.unpark(self.id);
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Gated<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.get_mut()
            .gated(Side::Read, cx, |s, cx| s.poll_read(cx, buf))
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Gated<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.get_mut()
            .gated(Side::Write, cx, |s, cx| s.poll_write(cx, buf))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut()
            .gated(Side::Write, cx, |s, cx| s.poll_flush(cx))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut()
            .gated(Side::Write, cx, |s, cx| s.poll_shutdown(cx))
    }
}

#[cfg(test)]
#[path = "core_tests.rs"]
mod tests;
