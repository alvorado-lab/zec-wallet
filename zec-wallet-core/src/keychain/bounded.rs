//! FR-47 — every production key-store call answers inside a bound.
//!
//! A JNI or Security.framework call cannot be cancelled, so the only bound the
//! SDK can give is to stop WAITING for it. [`BoundedVault`] is the decorator
//! `platform_vault` wraps around every native vault: each port call runs on
//! ONE process-wide worker thread (`zec-keychain`, spawned lazily once) and the
//! caller waits at most its bound, then answers
//! [`WalletError::KeychainTimeout`]. Everything a call can still do after its
//! caller left must be something the next wipe converges from (stage S9 §3.1's
//! late-landing table):
//!
//! - **One native call in flight, never a call that starts after its caller
//!   left.** A job waits `Queued`; the worker claims it `Running`; a caller
//!   whose bound expires claims `Queued → Abandoned` (the job is skipped, never
//!   run) or finds it `Running` and marks the in-flight call abandoned. While an
//!   abandoned call is in flight every new submission fails at once (`busy`)
//!   and is never enqueued — so a wedge holds ONE call and ONE thread, however
//!   many callers retry.
//! - **An abandoned job holds no key.** A job's inputs (a `SealKey` among them)
//!   live in a payload slot the caller takes and drops at the moment it claims
//!   `Queued → Abandoned`, not when the worker eventually reaches the job. When
//!   an abandoned in-flight call returns, the worker drops its result (a loaded
//!   `SealKey` zeroizes) BEFORE it clears the busy flag.
//! - **A late sever is recorded as evidence.** An abandoned `purge_namespace`
//!   that later returns `Ok(n > 0)` puts the namespace the wipe asked it to
//!   purge into a process-wide severed-late set before the busy flag clears;
//!   `store::destroy` skips its zero-sever guard for a namespace in that set.
//!   A zero count proves nothing and records nothing; the evidence is
//!   in-process only (after a restart the host uses `wipe_force`, as before).
//! - **The wipe has one budget** for all its key-store calls: a scoped,
//!   thread-local start instant ([`wipe_scope`]) that the decorator reads; a
//!   call inside it waits at most the smaller of its per-call bound and the
//!   budget's remainder, and a call submitted after the budget is spent fails
//!   at once (`past_deadline`). The duress sever narrows it further to its
//!   caller's deadline ([`wipe_scope_until`]).
//! - **A duress tombstone stops every creating call inside its job** (stage
//!   S16): the process-wide path table records which namespaces a sever has
//!   tombstoned, and a creating call's job reads it just before the inner
//!   call. The FIFO worker orders that read against the sever's own purge.
//! - **A panic inside a vault call** is caught on the worker and answered as
//!   `KeystoreUnavailable` (a failure, not a timeout); the worker survives.
//!
//! Logging: one payload-free `wallet.vault_call` line per call that timed
//! out, failed fast, landed late, ran slow or panicked (`outcome` ∈ `timeout`
//! · `busy` · `past_deadline` · `late` · `slow` · `panicked`), `op` from a
//! closed vocabulary that avoids §5.4's FORBIDDEN `key` token (`store_wrap`,
//! not `store_wrap_key` — the event was `wallet.keychain_call` until the S9
//! adjudication found the device log withheld it), and the whole-millisecond
//! duration (`call_ms`) on every call slower than a second — including a call
//! that lands after its caller left, which only the worker can time. No
//! namespace, no id (§5.4).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::custody::CustodyIndexEntry;
use crate::error::{KeychainTimeoutCause, WalletError};
use crate::seal::SealKey;

use super::{KeychainNamespace, KeychainPort, VaultTier, WrapArtifact};

/// The bound on every key-store call outside a wipe. StrongBox key generation
/// (create's wrap, the index write) is the slowest legitimate call and must not
/// time out on a slow device; the SDK UI's own Dart belt (`_localIoBound`,
/// 30 s) stays above it. Priced from principle, not measured (stage S9 §3.1).
pub(crate) const KEYCHAIN_CALL_BOUND: Duration = Duration::from_secs(8);

/// The wipe's budget for ALL its key-store calls together. Relim's panic wipe
/// allows 10 s for two attempts: the first answers within this plus the file
/// sweep, and a second attempt while the key store is still wedged answers at
/// once (`busy`). Public because it is also the most a duress sever spends
/// (`Wallet::sever_custody`): the bridge clamps a host's deadline to it.
pub const KEYCHAIN_WIPE_BUDGET: Duration = Duration::from_secs(4);

/// A call at least this slow logs its duration.
const SLOW_CALL: Duration = Duration::from_secs(1);

/// Poison-tolerant lock: every guarded structure here stays consistent across
/// a panic (vault panics are caught before they reach a lock holder).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn whole_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

// ── The wipe's budget and purge target (thread-scoped) ──────────────────────

thread_local! {
    /// When the wipe running on this thread started — the one budget every
    /// key-store call inside it shares. `None` outside a wipe.
    static WIPE_STARTED: Cell<Option<Instant>> = const { Cell::new(None) };
    /// A caller's own deadline inside the wipe's budget (the duress sever's,
    /// [`wipe_scope_until`]). `None` = the budget alone. Never back-dates
    /// `WIPE_STARTED`: the vault's own `wipe_budget` stays the decorator's.
    static WIPE_END: Cell<Option<Instant>> = const { Cell::new(None) };
    /// The namespace the caller on this thread is purging — what a late sever
    /// is recorded against (a decorator over a fixed test vault cannot know).
    static SEVER_TARGET: RefCell<Option<KeychainNamespace>> = const { RefCell::new(None) };
}

/// Holds the wipe's budget open on this thread; restores the prior state on
/// drop. A nested scope keeps the OUTER start — one wipe, one budget — and
/// the EARLIER end.
pub(crate) struct WipeScope {
    prev: Option<Instant>,
    prev_end: Option<Instant>,
}

/// Start the wipe's key-store budget on this thread for the life of the
/// returned guard (`Wallet::wipe_resolving`'s blocking section). Sets no end
/// of its own, so a plain wipe is bounded by the budget alone.
pub(crate) fn wipe_scope() -> WipeScope {
    let prev = WIPE_STARTED.with(Cell::get);
    let prev_end = WIPE_END.with(Cell::get);
    WIPE_STARTED.with(|c| c.set(Some(prev.unwrap_or_else(Instant::now))));
    WipeScope { prev, prev_end }
}

/// [`wipe_scope`] with a caller's deadline: every key-store call inside it
/// waits at most until the EARLIER of `end` and the vault's own budget, and a
/// call submitted at or past that instant fails at once (`past_deadline`). A
/// nested scope keeps the earlier of the two ends (`min(prev_end, end)`).
pub(crate) fn wipe_scope_until(end: Instant) -> WipeScope {
    let scope = wipe_scope();
    let end = scope.prev_end.map_or(end, |prev_end| prev_end.min(end));
    WIPE_END.with(|c| c.set(Some(end)));
    scope
}

impl Drop for WipeScope {
    fn drop(&mut self) {
        WIPE_STARTED.with(|c| c.set(self.prev));
        WIPE_END.with(|c| c.set(self.prev_end));
    }
}

struct SeverTargetScope {
    prev: Option<KeychainNamespace>,
}

impl Drop for SeverTargetScope {
    fn drop(&mut self) {
        let prev = self.prev.take();
        SEVER_TARGET.with(|c| *c.borrow_mut() = prev);
    }
}

/// Run `f` (a purge) with `namespace` named as its target, so a bounded purge
/// that lands after its caller left records THAT namespace as severed-late.
pub(crate) fn purging<T>(namespace: &KeychainNamespace, f: impl FnOnce() -> T) -> T {
    let prev = SEVER_TARGET.with(|c| c.borrow_mut().replace(namespace.clone()));
    let _restore = SeverTargetScope { prev };
    f()
}

// ── The severed-late evidence set (process-wide) ────────────────────────────

static SEVERED_LATE: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Did an abandoned purge of `namespace` land in this process with a non-zero
/// count? The evidence `store::destroy` accepts in place of this run's sever.
pub(crate) fn severed_late(namespace: &KeychainNamespace) -> bool {
    lock(&SEVERED_LATE).contains(namespace.as_str())
}

/// The wipe that relied on (or simply covered) `namespace` completed — its
/// late-sever evidence is spent.
pub(crate) fn forget_severed_late(namespace: &KeychainNamespace) {
    lock(&SEVERED_LATE).remove(namespace.as_str());
}

fn record_severed_late(namespace: &KeychainNamespace) {
    lock(&SEVERED_LATE).insert(namespace.as_str().to_owned());
}

/// A fresh process's view: no late sever was ever seen. PROCESS-WIDE: it
/// clears every test's evidence, so a test that calls it must not run beside
/// one that relies on a recorded late sever.
#[cfg(test)]
#[allow(dead_code)] // the stage's evidence test (S9 §3.1 assertion 7a) calls it
pub(crate) fn reset_severed_late() {
    lock(&SEVERED_LATE).clear();
}

// ── The path table (process-wide; stage S16 §3.1 item 4) ────────────────────
//
// ONE table keyed by keychain namespace. A PATH namespace's entry names the
// live instances of the wallet at that `db_dir` in this process, the openers
// holding its lock, and whether a duress sever tombstoned it; an id or legacy
// namespace's entry carries only the tombstone. The tombstone is what the
// key-store worker's job reads before every CREATING call ([`BoundedVault`]).
//
// LOCK ORDER (§3.1 item 8): `PATHS` is innermost, and nothing is done while
// holding it — never a key-store call or job submission, an `.await`, a
// `Lifecycle` or DB lock, file I/O, or a drop whose `Drop` takes `PATHS` (an
// `OpeningToken`, or an `Inner`, whose lock carries one). A section is one
// map lookup plus O(instances) vector work, and every section on a key prunes
// that entry (item 9): dead `Weak`s go, and an entry with no instance, no
// opener and no tombstone is removed. Tombstoned entries stay until a wipe of
// that path completes (or the process exits).

#[derive(Default)]
struct NsState {
    instances: Vec<std::sync::Weak<crate::wallet::Inner>>,
    opening: usize,
    tombstoned: bool,
    /// A proven sever (a non-zero count) of this path wrote its breadcrumb.
    proven: bool,
    /// The id / legacy namespaces this path's sever tombstoned, cleared with it.
    linked: Vec<String>,
}

impl NsState {
    fn idle(&self) -> bool {
        self.instances.is_empty() && self.opening == 0 && !self.tombstoned
    }
}

static PATHS: LazyLock<Mutex<std::collections::HashMap<String, NsState>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// One `PATHS` section on `key`: `f` sees the entry (created empty if absent),
/// then the entry is pruned. Nothing `f` returns may drop a token or an
/// `Inner` (a `Weak` is fine: dropping it runs no `Drop` of `Inner`).
fn paths_section<R>(key: &str, f: impl FnOnce(&mut NsState) -> R) -> R {
    let mut table = lock(&PATHS);
    let entry = table.entry(key.to_owned()).or_default();
    let out = f(entry);
    entry.instances.retain(|w| w.strong_count() > 0);
    if entry.idle() {
        table.remove(key);
    }
    out
}

/// An opener took (or is about to take) the lock of the path `namespace`.
pub(crate) fn opening_bump(namespace: &KeychainNamespace) {
    paths_section(namespace.as_str(), |e| e.opening += 1);
}

/// An opener's token dropped.
pub(crate) fn opening_release(namespace: &KeychainNamespace) {
    paths_section(namespace.as_str(), |e| {
        e.opening = e.opening.saturating_sub(1)
    });
}

/// The birth gate (§3.2 mechanism 2): ONE section that refuses a new instance
/// at a tombstoned path (`false`) or registers it (`true`). The caller holds
/// the only strong reference either way, and drops a refused one off-lock.
pub(crate) fn register_instance(
    namespace: &KeychainNamespace,
    instance: std::sync::Weak<crate::wallet::Inner>,
) -> bool {
    paths_section(namespace.as_str(), |e| {
        if e.tombstoned {
            false
        } else {
            e.instances.push(instance);
            true
        }
    })
}

/// The sever's step (i), in ONE section: tombstone the path, snapshot its live
/// instances, read its openers. The caller upgrades and poisons the snapshot
/// after this returns, off-lock.
pub(crate) fn sever_begin(
    path: &KeychainNamespace,
) -> (Vec<std::sync::Weak<crate::wallet::Inner>>, usize) {
    paths_section(path.as_str(), |e| {
        e.tombstoned = true;
        let live = e
            .instances
            .iter()
            .filter(|w| w.strong_count() > 0)
            .cloned()
            .collect();
        (live, e.opening)
    })
}

/// The sever's step (iv): tombstone the id / legacy namespaces it resolved for
/// `path`, BEFORE their purge is submitted — a creating call that runs after
/// the purge job is then refused inside its own job.
pub(crate) fn tombstone_linked(path: &KeychainNamespace, linked: &[KeychainNamespace]) {
    let mut table = lock(&PATHS);
    for ns in linked {
        table.entry(ns.as_str().to_owned()).or_default().tombstoned = true;
    }
    let entry = table.entry(path.as_str().to_owned()).or_default();
    for ns in linked {
        if !entry.linked.iter().any(|l| l == ns.as_str()) {
            entry.linked.push(ns.as_str().to_owned());
        }
    }
}

/// Tombstone the path `path` (the blind delete's own step, on both branches:
/// a free-lock sever reaches it with no step (i) before it). Idempotent.
pub(crate) fn tombstone_path(path: &KeychainNamespace) {
    paths_section(path.as_str(), |e| e.tombstoned = true);
}

/// Is `namespace` tombstoned? The worker job's check (and the open path's).
pub(crate) fn is_tombstoned(namespace: &KeychainNamespace) -> bool {
    paths_section(namespace.as_str(), |e| e.tombstoned)
}

/// A proven sever of `path` is about to write its breadcrumb (§3.1 item 6).
pub(crate) fn mark_proven(path: &KeychainNamespace) {
    paths_section(path.as_str(), |e| e.proven = true);
}

/// Did a proven sever of `path` write a breadcrumb that an open's sweep must
/// put back?
pub(crate) fn is_proven(path: &KeychainNamespace) -> bool {
    paths_section(path.as_str(), |e| e.proven)
}

/// A wipe of `path` completed: its tombstone, its proven mark and every
/// namespace its sever linked are cleared (§3.1 item 6).
pub(crate) fn clear_severed_path(path: &KeychainNamespace) {
    let mut table = lock(&PATHS);
    let linked = match table.get_mut(path.as_str()) {
        Some(entry) => {
            entry.tombstoned = false;
            entry.proven = false;
            entry.instances.retain(|w| w.strong_count() > 0);
            std::mem::take(&mut entry.linked)
        }
        None => return,
    };
    for key in linked.iter().map(String::as_str).chain([path.as_str()]) {
        if let Some(entry) = table.get_mut(key) {
            entry.tombstoned = false;
            if entry.idle() {
                table.remove(key);
            }
        }
    }
}

/// A fresh process's view of the table. PROCESS-WIDE, like
/// [`reset_severed_late`]: a test that calls it must not run beside one that
/// relies on a tombstone or a registration.
#[cfg(test)]
#[allow(dead_code)] // the stage's convergence rows call it
pub(crate) fn reset_paths() {
    lock(&PATHS).clear();
}

/// What the table holds for `namespace`: `(live instances, openers,
/// tombstoned)`, or `None` when it holds no entry (the growth rows).
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn path_entry(namespace: &KeychainNamespace) -> Option<(usize, usize, bool)> {
    lock(&PATHS).get(namespace.as_str()).map(|e| {
        (
            e.instances.iter().filter(|w| w.strong_count() > 0).count(),
            e.opening,
            e.tombstoned,
        )
    })
}

// ── The worker ──────────────────────────────────────────────────────────────

type Work = Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>;

enum JobState {
    Queued,
    Running,
    Finished(Box<dyn Any + Send>),
    Abandoned,
    Taken,
}

struct Job {
    op: &'static str,
    /// Set on a purge submitted inside [`purging`].
    sever_target: Option<KeychainNamespace>,
    state: Mutex<JobState>,
    finished: Condvar,
    /// The call with its inputs. Taken by the worker when it claims the job,
    /// or by the caller — and dropped on the spot — when it abandons a queued
    /// one. Always locked INSIDE `state`.
    payload: Mutex<Option<Work>>,
}

struct Queue {
    jobs: VecDeque<Arc<Job>>,
    /// An abandoned call is inside the key store.
    busy: bool,
    spawned: bool,
    closed: bool,
}

struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
    spawns: AtomicUsize,
    payloads_dropped_at_abandonment: AtomicUsize,
}

/// One key-store worker: its queue, its busy flag, its (lazily spawned)
/// thread. Production has exactly one ([`PROCESS_WORKER`]), and so does the
/// contract's test seam `BoundedVault::with_bounds`; only this module's
/// unit tests build a private one (`with_own_worker`).
struct Worker {
    shared: Arc<Shared>,
}

static PROCESS_WORKER: LazyLock<Arc<Worker>> = LazyLock::new(|| Arc::new(Worker::new()));

impl Worker {
    fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                queue: Mutex::new(Queue {
                    jobs: VecDeque::new(),
                    busy: false,
                    spawned: false,
                    closed: false,
                }),
                wake: Condvar::new(),
                spawns: AtomicUsize::new(0),
                payloads_dropped_at_abandonment: AtomicUsize::new(0),
            }),
        }
    }

    /// Enqueue `job`, spawning the thread on first use. Refused at once — and
    /// never enqueued — while an abandoned call is in flight.
    fn submit(&self, job: Arc<Job>) -> Result<(), WalletError> {
        let mut q = lock(&self.shared.queue);
        if q.busy {
            return Err(WalletError::KeychainTimeout {
                cause: KeychainTimeoutCause::Busy,
            });
        }
        if !q.spawned {
            let shared = Arc::clone(&self.shared);
            std::thread::Builder::new()
                .name("zec-keychain".to_owned())
                .spawn(move || worker_loop(&shared))
                .map_err(|_| WalletError::KeystoreUnavailable)?;
            q.spawned = true;
            self.shared.spawns.fetch_add(1, Ordering::SeqCst);
        }
        q.jobs.push_back(job);
        drop(q);
        self.shared.wake.notify_one();
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // Only a test worker ever drops; its thread exits once idle.
        lock(&self.shared.queue).closed = true;
        self.shared.wake.notify_all();
    }
}

fn worker_loop(shared: &Shared) {
    loop {
        let job = {
            let mut q = lock(&shared.queue);
            loop {
                if let Some(job) = q.jobs.pop_front() {
                    break job;
                }
                if q.closed {
                    return;
                }
                q = shared.wake.wait(q).unwrap_or_else(PoisonError::into_inner);
            }
        };
        let work = {
            let mut st = lock(&job.state);
            if !matches!(*st, JobState::Queued) {
                // Its caller left before we reached it: never run.
                continue;
            }
            *st = JobState::Running;
            lock(&job.payload).take()
        };
        let Some(work) = work else { continue };
        let started = Instant::now();
        let result = work();
        let took = started.elapsed();
        let mut st = lock(&job.state);
        if matches!(*st, JobState::Running) {
            *st = JobState::Finished(result);
            drop(st);
            job.finished.notify_all();
            continue;
        }
        // Abandoned while in flight: the caller was told it timed out.
        drop(st);
        tracing::warn!(
            target: "zec_wallet_core",
            outcome = "late",
            op = job.op,
            call_ms = whole_ms(took),
            "wallet.vault_call",
        );
        if let Some(ns) = &job.sever_target
            && matches!(
                result.downcast_ref::<Result<usize, WalletError>>(),
                Some(Ok(n)) if *n > 0
            )
        {
            record_severed_late(ns);
        }
        // The late result (a loaded `SealKey` included) is gone BEFORE any
        // later call can run.
        drop(result);
        lock(&shared.queue).busy = false;
    }
}

// ── The decorator ───────────────────────────────────────────────────────────

/// The bounded key store (FR-47): every [`KeychainPort`] call of the vault it
/// wraps runs on the key-store worker and answers inside its bound. The ONLY
/// production constructor is `platform_vault`'s; the native vaults are
/// untouched.
pub(crate) struct BoundedVault {
    inner: Arc<dyn KeychainPort>,
    worker: Arc<Worker>,
    call_bound: Duration,
    wipe_budget: Duration,
    /// The namespace the wrapped vault custodies under — what a creating call's
    /// job checks for a duress tombstone (stage S16 §3.1 item 4). `None` for a
    /// vault no sever can name (the selftest's own namespaces).
    namespace: Option<KeychainNamespace>,
}

impl BoundedVault {
    /// Production: the process's one worker, the SDK's bounds.
    pub(crate) fn new(inner: Arc<dyn KeychainPort>, namespace: Option<KeychainNamespace>) -> Self {
        Self {
            inner,
            worker: Arc::clone(&PROCESS_WORKER),
            call_bound: KEYCHAIN_CALL_BOUND,
            wipe_budget: KEYCHAIN_WIPE_BUDGET,
            namespace,
        }
    }

    /// Test seam (contract §3.1 "The seam"): injected bounds over the
    /// process's ONE worker, exactly as production shares it — so a second
    /// decorator, a second namespace or a second wipe sees the first one's
    /// wedge (items 1 and 3). Tests that wedge it serialise among themselves.
    /// `namespace` is what the tombstone check reads (S16), as
    /// `platform_vault` sets it in production.
    #[cfg(test)]
    pub(crate) fn with_bounds(
        inner: Arc<dyn KeychainPort>,
        call_bound: Duration,
        wipe_budget: Duration,
        namespace: Option<KeychainNamespace>,
    ) -> Self {
        Self {
            inner,
            worker: Arc::clone(&PROCESS_WORKER),
            call_bound,
            wipe_budget,
            namespace,
        }
    }

    /// This module's unit tests only: a worker of the decorator's own (a
    /// fresh "process" whose spawn count starts at zero), so a unit test can
    /// wedge it without touching the process worker the s9 rows share.
    #[cfg(test)]
    fn with_own_worker(
        inner: Arc<dyn KeychainPort>,
        call_bound: Duration,
        wipe_budget: Duration,
    ) -> Self {
        Self {
            inner,
            worker: Arc::new(Worker::new()),
            call_bound,
            wipe_budget,
            namespace: None,
        }
    }

    /// How many times the PROCESS worker's thread was spawned (assertion 8).
    #[cfg(test)]
    pub(crate) fn worker_spawns() -> usize {
        PROCESS_WORKER.shared.spawns.load(Ordering::SeqCst)
    }

    /// How many jobs queued on the PROCESS worker still hold their inputs
    /// (assertion 12's drop-probe: an abandoned job's slot is emptied at
    /// abandonment, so it stops counting while the wedge is still held).
    #[cfg(test)]
    pub(crate) fn payloads_held() -> usize {
        lock(&PROCESS_WORKER.shared.queue)
            .jobs
            .iter()
            .filter(|job| lock(&job.payload).is_some())
            .count()
    }

    /// How many worker threads this decorator's worker has ever spawned.
    #[cfg(test)]
    pub(crate) fn spawn_count(&self) -> usize {
        self.worker.shared.spawns.load(Ordering::SeqCst)
    }

    /// How many queued jobs had their payload (inputs, keys included) dropped
    /// by the caller at the moment it abandoned them.
    #[cfg(test)]
    pub(crate) fn payloads_dropped_at_abandonment(&self) -> usize {
        self.worker
            .shared
            .payloads_dropped_at_abandonment
            .load(Ordering::SeqCst)
    }

    /// How long a call submitted `now` may wait, or why it must not be
    /// submitted at all.
    fn bound_at(&self, now: Instant) -> Result<Duration, KeychainTimeoutCause> {
        let Some(started) = WIPE_STARTED.with(Cell::get) else {
            return Ok(self.call_bound);
        };
        // The earlier of this vault's budget and the caller's own end
        // (`wipe_scope_until`); a budget too large to add is no bound at all.
        let end = match (
            started.checked_add(self.wipe_budget),
            WIPE_END.with(Cell::get),
        ) {
            (Some(budget_end), Some(caller_end)) => Some(budget_end.min(caller_end)),
            (budget_end, caller_end) => budget_end.or(caller_end),
        };
        let Some(end) = end else {
            return Ok(self.call_bound);
        };
        let left = end.saturating_duration_since(now);
        if left.is_zero() {
            return Err(KeychainTimeoutCause::PastDeadline);
        }
        Ok(self.call_bound.min(left))
    }

    fn refuse<T>(
        op: &'static str,
        cause: KeychainTimeoutCause,
        submitted: Instant,
    ) -> Result<T, WalletError> {
        let waited = submitted.elapsed();
        if waited >= SLOW_CALL {
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = cause.as_str(),
                op,
                call_ms = whole_ms(waited),
                "wallet.vault_call",
            );
        } else {
            tracing::warn!(
                target: "zec_wallet_core",
                outcome = cause.as_str(),
                op,
                "wallet.vault_call",
            );
        }
        Err(WalletError::KeychainTimeout { cause })
    }

    /// Run `call` against the wrapped vault on the worker, waiting at most
    /// this call's bound.
    fn call<T: Send + 'static>(
        &self,
        op: &'static str,
        call: impl FnOnce(&dyn KeychainPort) -> Result<T, WalletError> + Send + 'static,
    ) -> Result<T, WalletError> {
        self.call_severing(op, None, call)
    }

    /// [`Self::call`] for a CREATING call (stage S16 §3.1 item 4): inside the
    /// job, on the worker, just before the inner call, read whether this
    /// vault's namespace is tombstoned — `PATHS` taken and released within the
    /// read — and refuse `InvalidState { Wiped }` without touching the key
    /// store if it is. The worker runs jobs one at a time in FIFO order, so a
    /// sever's purge job and this check are ordered: a creating call that runs
    /// before the purge is purged by it, one that runs after is refused here.
    fn call_gated<T: Send + 'static>(
        &self,
        op: &'static str,
        call: impl FnOnce(&dyn KeychainPort) -> Result<T, WalletError> + Send + 'static,
    ) -> Result<T, WalletError> {
        let namespace = self.namespace.clone();
        self.call(op, move |v| {
            if namespace.as_ref().is_some_and(is_tombstoned) {
                return Err(WalletError::InvalidState {
                    phase: crate::lifecycle::LifecyclePhase::Wiped,
                });
            }
            call(v)
        })
    }

    /// [`Self::call`], naming the namespace a late non-zero result of this
    /// call is recorded against (a purge only).
    fn call_severing<T: Send + 'static>(
        &self,
        op: &'static str,
        sever_target: Option<KeychainNamespace>,
        call: impl FnOnce(&dyn KeychainPort) -> Result<T, WalletError> + Send + 'static,
    ) -> Result<T, WalletError> {
        let submitted = Instant::now();
        let bound = match self.bound_at(submitted) {
            Ok(bound) => bound,
            Err(cause) => return Self::refuse(op, cause, submitted),
        };
        let inner = Arc::clone(&self.inner);
        let work: Work = Box::new(move || {
            let out =
                catch_unwind(AssertUnwindSafe(|| call(inner.as_ref()))).unwrap_or_else(|_| {
                    tracing::warn!(
                        target: "zec_wallet_core",
                        outcome = "panicked",
                        op,
                        "wallet.vault_call",
                    );
                    Err(WalletError::KeystoreUnavailable)
                });
            Box::new(out) as Box<dyn Any + Send>
        });
        let job = Arc::new(Job {
            op,
            sever_target,
            state: Mutex::new(JobState::Queued),
            finished: Condvar::new(),
            payload: Mutex::new(Some(work)),
        });
        match self.worker.submit(Arc::clone(&job)) {
            Ok(()) => {}
            Err(WalletError::KeychainTimeout { cause }) => {
                // Never enqueued: the inputs go with the job, here.
                return Self::refuse(op, cause, submitted);
            }
            Err(e) => return Err(e),
        }

        let deadline = submitted + bound;
        let mut st = lock(&job.state);
        loop {
            if matches!(*st, JobState::Finished(_)) {
                let JobState::Finished(out) = std::mem::replace(&mut *st, JobState::Taken) else {
                    unreachable!("matched Finished above");
                };
                drop(st);
                let took = submitted.elapsed();
                if took >= SLOW_CALL {
                    tracing::info!(
                        target: "zec_wallet_core",
                        outcome = "slow",
                        op,
                        call_ms = whole_ms(took),
                        "wallet.vault_call",
                    );
                }
                return match out.downcast::<Result<T, WalletError>>() {
                    Ok(result) => *result,
                    // Unreachable by construction (the job returns its own T);
                    // a failure, never a panic, if it ever were not.
                    Err(_) => Err(WalletError::KeystoreUnavailable),
                };
            }
            let now = Instant::now();
            if now >= deadline {
                match *st {
                    JobState::Queued => {
                        *st = JobState::Abandoned;
                        // The inputs die HERE, not whenever the worker gets
                        // past the wedge in front of this job.
                        if let Some(payload) = lock(&job.payload).take() {
                            drop(payload);
                            self.worker
                                .shared
                                .payloads_dropped_at_abandonment
                                .fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    JobState::Running => {
                        *st = JobState::Abandoned;
                        // Set while the job lock is held, so the worker —
                        // which clears it only after seeing `Abandoned` —
                        // cannot clear it first.
                        lock(&self.worker.shared.queue).busy = true;
                    }
                    _ => {}
                }
                drop(st);
                return Self::refuse(op, KeychainTimeoutCause::Timeout, submitted);
            }
            st = job
                .finished
                .wait_timeout(st, deadline - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// Every CREATING method (`store_wrap_key`, `rotate_wrap_key`, `store_index`)
/// runs through [`BoundedVault::call_gated`], so a duress tombstone on this
/// vault's namespace refuses it inside its job. Deletes (`purge_namespace`,
/// `delete_index`, `delete_wrap_key`, `finish_rotation`) and reads are exempt:
/// they only remove or read custody, and the sever's own purge and index
/// delete must pass. A new trait method must be `Fake`d as a new `Method` in S16's row
/// `a_tombstone_stops_creates_…`, whose exhaustive matches fail an ungated create (not a default).
impl KeychainPort for BoundedVault {
    fn probe(&self) -> Result<(), WalletError> {
        self.call("probe", |v| v.probe())
    }

    fn tier(&self) -> Result<VaultTier, WalletError> {
        self.call("tier", |v| v.tier())
    }

    fn store_wrap_key(
        &self,
        key: SealKey,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        let blob = sealed_blob.to_vec();
        self.call_gated("store_wrap", move |v| v.store_wrap_key(key, &blob))
    }

    fn load_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<SealKey, WalletError> {
        let artifact = artifact.clone();
        let blob = sealed_blob.to_vec();
        self.call("load_wrap", move |v| v.load_wrap_key(&artifact, &blob))
    }

    fn rotate_wrap_key(
        &self,
        artifact: &WrapArtifact,
        sealed_blob: &[u8],
    ) -> Result<WrapArtifact, WalletError> {
        let artifact = artifact.clone();
        let blob = sealed_blob.to_vec();
        self.call_gated("rotate_wrap", move |v| v.rotate_wrap_key(&artifact, &blob))
    }

    fn finish_rotation(&self, old: &WrapArtifact, new: &WrapArtifact) -> Result<(), WalletError> {
        let (old, new) = (old.clone(), new.clone());
        self.call("finish_rotation", move |v| v.finish_rotation(&old, &new))
    }

    fn delete_wrap_key(&self, artifact: &WrapArtifact) -> Result<(), WalletError> {
        let artifact = artifact.clone();
        self.call("delete_wrap", move |v| v.delete_wrap_key(&artifact))
    }

    fn purge_namespace(&self) -> Result<usize, WalletError> {
        let target = SEVER_TARGET.with(|c| c.borrow().clone());
        self.call_severing("purge_namespace", target, |v| v.purge_namespace())
    }

    fn store_index(&self, entry: &CustodyIndexEntry) -> Result<(), WalletError> {
        let entry = entry.clone();
        self.call_gated("store_index", move |v| v.store_index(&entry))
    }

    fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
        self.call("load_index", |v| v.load_index())
    }

    fn delete_index(&self) -> Result<(), WalletError> {
        self.call("delete_index", |v| v.delete_index())
    }
}

#[cfg(test)]
mod tests {
    //! The decorator's own mechanics, over a gated fake. The stage's named
    //! tests (the wipe, create and open paths through it) live with the item.

    use std::sync::mpsc;

    use super::*;
    use crate::keychain::testvault::TestVault;

    /// A vault whose purge blocks until the test sends the count it returns,
    /// whose `tier` panics, and whose every other call answers at once.
    struct GateVault {
        release: Mutex<mpsc::Receiver<usize>>,
        entered: Mutex<mpsc::Sender<()>>,
        purges: AtomicUsize,
        stores: AtomicUsize,
    }

    impl KeychainPort for GateVault {
        fn probe(&self) -> Result<(), WalletError> {
            Ok(())
        }
        fn tier(&self) -> Result<VaultTier, WalletError> {
            panic!("the gate vault's tier panics");
        }
        fn store_wrap_key(&self, _: SealKey, _: &[u8]) -> Result<WrapArtifact, WalletError> {
            self.stores.fetch_add(1, Ordering::SeqCst);
            Ok(WrapArtifact::from_freshly_wrapped(vec![1]))
        }
        fn load_wrap_key(&self, _: &WrapArtifact, _: &[u8]) -> Result<SealKey, WalletError> {
            Ok(SealKey::generate())
        }
        fn rotate_wrap_key(&self, a: &WrapArtifact, _: &[u8]) -> Result<WrapArtifact, WalletError> {
            Ok(a.clone())
        }
        fn finish_rotation(&self, _: &WrapArtifact, _: &WrapArtifact) -> Result<(), WalletError> {
            Ok(())
        }
        fn delete_wrap_key(&self, _: &WrapArtifact) -> Result<(), WalletError> {
            Ok(())
        }
        fn purge_namespace(&self) -> Result<usize, WalletError> {
            self.purges.fetch_add(1, Ordering::SeqCst);
            let _ = lock(&self.entered).send(());
            Ok(lock(&self.release).recv().unwrap_or(0))
        }
        fn store_index(&self, _: &CustodyIndexEntry) -> Result<(), WalletError> {
            Ok(())
        }
        fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
            Ok(None)
        }
        fn delete_index(&self) -> Result<(), WalletError> {
            Ok(())
        }
    }

    struct Gate {
        vault: Arc<GateVault>,
        release: mpsc::Sender<usize>,
        entered: mpsc::Receiver<()>,
    }

    fn gate() -> Gate {
        let (release, release_rx) = mpsc::channel();
        let (entered_tx, entered) = mpsc::channel();
        Gate {
            vault: Arc::new(GateVault {
                release: Mutex::new(release_rx),
                entered: Mutex::new(entered_tx),
                purges: AtomicUsize::new(0),
                stores: AtomicUsize::new(0),
            }),
            release,
            entered,
        }
    }

    fn ns(byte: &str) -> KeychainNamespace {
        KeychainNamespace::new(byte.repeat(16)).expect("32 lowercase hex")
    }

    /// Wait (bounded) until the worker has cleared its busy flag.
    fn wait_idle(v: &BoundedVault) {
        let until = Instant::now() + Duration::from_secs(20);
        while lock(&v.worker.shared.queue).busy {
            assert!(Instant::now() < until, "the worker never went idle");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn is_timeout(r: &Result<impl Sized, WalletError>, want: KeychainTimeoutCause) -> bool {
        matches!(r, Err(WalletError::KeychainTimeout { cause }) if *cause == want)
    }

    #[test]
    fn bounded_passes_results_through_on_one_lazily_spawned_worker() {
        let v = BoundedVault::with_own_worker(
            Arc::new(TestVault::new(VaultTier::Tee)),
            Duration::from_secs(20),
            Duration::from_secs(20),
        );
        assert_eq!(v.spawn_count(), 0, "no thread before the first call");
        assert!(matches!(v.tier(), Ok(VaultTier::Tee)));
        v.probe().expect("probe");
        let artifact = v
            .store_wrap_key(SealKey::generate(), b"blob")
            .expect("store");
        v.load_wrap_key(&artifact, b"blob").expect("load");
        assert_eq!(v.spawn_count(), 1);
    }

    #[test]
    fn bounded_wedge_times_out_then_refuses_busy_and_records_a_late_sever() {
        let g = gate();
        let v = BoundedVault::with_own_worker(
            g.vault.clone(),
            Duration::from_millis(300),
            Duration::MAX,
        );
        v.probe().expect("warm the worker");
        let target = ns("a1");
        let first = purging(&target, || v.purge_namespace());
        assert!(is_timeout(&first, KeychainTimeoutCause::Timeout));
        let second = v.purge_namespace();
        assert!(is_timeout(&second, KeychainTimeoutCause::Busy));
        assert!(is_timeout(&v.probe(), KeychainTimeoutCause::Busy));
        assert_eq!(
            g.vault.purges.load(Ordering::SeqCst),
            1,
            "one call in flight"
        );
        assert!(!severed_late(&target), "nothing recorded before it lands");

        g.release.send(2).expect("release");
        wait_idle(&v);
        assert!(severed_late(&target), "a late Ok(n > 0) is evidence");
        forget_severed_late(&target);
        v.probe().expect("the worker answers again");
        assert_eq!(v.spawn_count(), 1);
    }

    #[test]
    fn bounded_late_zero_sever_records_nothing() {
        let g = gate();
        let v = BoundedVault::with_own_worker(
            g.vault.clone(),
            Duration::from_millis(300),
            Duration::MAX,
        );
        v.probe().expect("warm the worker");
        let target = ns("b2");
        assert!(is_timeout(
            &purging(&target, || v.purge_namespace()),
            KeychainTimeoutCause::Timeout
        ));
        g.release.send(0).expect("release");
        wait_idle(&v);
        assert!(!severed_late(&target));
    }

    #[test]
    fn bounded_queued_job_left_by_its_caller_drops_its_payload_and_never_runs() {
        let g = gate();
        let v = Arc::new(BoundedVault::with_own_worker(
            g.vault.clone(),
            Duration::from_millis(400),
            Duration::MAX,
        ));
        v.probe().expect("warm the worker");
        let wedged = {
            let v = Arc::clone(&v);
            std::thread::spawn(move || v.purge_namespace())
        };
        g.entered.recv().expect("the purge is inside the vault");
        // Queued behind the wedge, submitted before the wedge's caller left.
        let queued = v.store_wrap_key(SealKey::generate(), b"blob");
        assert!(is_timeout(&queued, KeychainTimeoutCause::Timeout));
        assert_eq!(v.payloads_dropped_at_abandonment(), 1);
        assert!(is_timeout(
            &wedged.join().expect("join"),
            KeychainTimeoutCause::Timeout
        ));
        g.release.send(1).expect("release");
        wait_idle(&v);
        v.probe().expect("after the wedge");
        assert_eq!(g.vault.stores.load(Ordering::SeqCst), 0, "never run");
    }

    #[test]
    fn bounded_panicking_call_is_a_failure_and_the_worker_survives() {
        let g = gate();
        let v =
            BoundedVault::with_own_worker(g.vault.clone(), Duration::from_secs(20), Duration::MAX);
        assert!(matches!(v.tier(), Err(WalletError::KeystoreUnavailable)));
        v.probe().expect("the next call runs");
        assert_eq!(v.spawn_count(), 1);
    }

    #[test]
    fn bounded_call_after_the_wipe_budget_fails_at_once() {
        let v = BoundedVault::with_own_worker(
            Arc::new(TestVault::new(VaultTier::Tee)),
            Duration::from_secs(20),
            Duration::ZERO,
        );
        {
            let _scope = wipe_scope();
            assert!(is_timeout(&v.probe(), KeychainTimeoutCause::PastDeadline));
        }
        assert_eq!(
            v.spawn_count(),
            0,
            "a refused call never reached the worker"
        );
        v.probe()
            .expect("outside a wipe the per-call bound applies");
    }

    // ── Random schedules (the diff review's MAJOR) ─────────────────────
    //
    // Every test above drives ONE hand-sequenced schedule. This one drives
    // many: caller threads racing, calls whose length falls on both sides of
    // the bound, and a rare call wedged until the end. The TIMING varies by
    // design — that is the point — but every assertion is one that must hold
    // under ANY interleaving, so what the test checks is deterministic. The
    // fake's sleeps simulate a key store's latency; nothing waits on a clock
    // to synchronise.

    const QUICK: u8 = 0;
    const JITTER: u8 = 1;
    const SLOW: u8 = 2;
    const WEDGE: u8 = 3;

    /// A vault whose `store_wrap_key` reads a call id and a behaviour from the
    /// blob, records that the call ENTERED, and returns the id as its artifact
    /// — so a caller can tell its own result from anyone else's.
    struct StressVault {
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        entered: Mutex<Vec<u64>>,
        released: Mutex<bool>,
        release: Condvar,
    }

    impl StressVault {
        fn new() -> Self {
            Self {
                in_flight: AtomicUsize::new(0),
                max_in_flight: AtomicUsize::new(0),
                entered: Mutex::new(Vec::new()),
                released: Mutex::new(false),
                release: Condvar::new(),
            }
        }

        fn release_wedges(&self) {
            *lock(&self.released) = true;
            self.release.notify_all();
        }
    }

    impl KeychainPort for StressVault {
        fn probe(&self) -> Result<(), WalletError> {
            Ok(())
        }
        fn tier(&self) -> Result<VaultTier, WalletError> {
            Ok(VaultTier::Tee)
        }
        fn store_wrap_key(&self, _: SealKey, blob: &[u8]) -> Result<WrapArtifact, WalletError> {
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_in_flight.fetch_max(now, Ordering::SeqCst);
            let id = u64::from_le_bytes(blob[..8].try_into().expect("8-byte id"));
            lock(&self.entered).push(id);
            match blob[8] {
                JITTER => std::thread::sleep(Duration::from_micros(u64::from(blob[9]) * 20)),
                SLOW => std::thread::sleep(Duration::from_millis(3 + u64::from(blob[9] % 6))),
                WEDGE => {
                    let mut released = lock(&self.released);
                    while !*released {
                        released = self
                            .release
                            .wait(released)
                            .unwrap_or_else(PoisonError::into_inner);
                    }
                }
                _ => {}
            }
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(WrapArtifact::from_freshly_wrapped(blob[..8].to_vec()))
        }
        fn load_wrap_key(&self, _: &WrapArtifact, _: &[u8]) -> Result<SealKey, WalletError> {
            Ok(SealKey::generate())
        }
        fn rotate_wrap_key(&self, a: &WrapArtifact, _: &[u8]) -> Result<WrapArtifact, WalletError> {
            Ok(a.clone())
        }
        fn finish_rotation(&self, _: &WrapArtifact, _: &WrapArtifact) -> Result<(), WalletError> {
            Ok(())
        }
        fn delete_wrap_key(&self, _: &WrapArtifact) -> Result<(), WalletError> {
            Ok(())
        }
        fn purge_namespace(&self) -> Result<usize, WalletError> {
            Ok(0)
        }
        fn store_index(&self, _: &CustodyIndexEntry) -> Result<(), WalletError> {
            Ok(())
        }
        fn load_index(&self) -> Result<Option<CustodyIndexEntry>, WalletError> {
            Ok(None)
        }
        fn delete_index(&self) -> Result<(), WalletError> {
            Ok(())
        }
    }

    /// What one caller was told.
    #[derive(Debug)]
    enum Told {
        Mine,
        SomeoneElses(Vec<u8>),
        Timeout,
        Refused,
        Other(String),
    }

    /// What one schedule exercised — the non-vacuity evidence.
    #[derive(Default)]
    struct Seen {
        ok: usize,
        timed_out_then_ran: usize,
        timed_out_never_ran: usize,
        refused: usize,
    }

    /// One random schedule from `seed`. xorshift64 per thread: reproducible
    /// choices, whatever the interleaving does with them.
    fn one_schedule(seed: u64) -> Seen {
        const THREADS: u64 = 4;
        const OPS: u64 = 20;
        let vault = Arc::new(StressVault::new());
        let v = Arc::new(BoundedVault::with_own_worker(
            vault.clone(),
            Duration::from_millis(3),
            Duration::MAX,
        ));
        let next_id = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let callers: Vec<_> = (0..THREADS)
            .map(|t| {
                let (v, next_id) = (Arc::clone(&v), Arc::clone(&next_id));
                std::thread::spawn(move || {
                    let mut x = seed ^ (t + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
                    let mut told = Vec::new();
                    for _ in 0..OPS {
                        x ^= x << 13;
                        x ^= x >> 7;
                        x ^= x << 17;
                        // A wedge ends a schedule's exploration (every later
                        // call is refused `busy`), so it is RARE; slow calls
                        // (3-8 ms against a 3 ms bound) drive the timeouts.
                        let mode = match x % 200 {
                            0 => WEDGE,
                            1..=60 => SLOW,
                            61..=130 => JITTER,
                            _ => QUICK,
                        };
                        let id = next_id.fetch_add(1, Ordering::SeqCst);
                        let mut blob = id.to_le_bytes().to_vec();
                        blob.extend_from_slice(&[mode, (x >> 8) as u8]);
                        let answer = match v.store_wrap_key(SealKey::generate(), &blob) {
                            Ok(a) if a.as_bytes() == &blob[..8] => Told::Mine,
                            Ok(a) => Told::SomeoneElses(a.as_bytes().to_vec()),
                            Err(WalletError::KeychainTimeout {
                                cause: KeychainTimeoutCause::Timeout,
                            }) => Told::Timeout,
                            Err(WalletError::KeychainTimeout { .. }) => Told::Refused,
                            Err(e) => Told::Other(format!("{e:?}")),
                        };
                        told.push((id, answer));
                    }
                    told
                })
            })
            .collect();
        let told: Vec<(u64, Told)> = callers
            .into_iter()
            .flat_map(|c| c.join().expect("a caller thread panicked"))
            .collect();

        // Quiesce: release every wedge, then wait (bounded) until the worker
        // is idle AND has drained its queue (skipped jobs included).
        vault.release_wedges();
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            let q = lock(&v.worker.shared.queue);
            if !q.busy && q.jobs.is_empty() {
                break;
            }
            drop(q);
            assert!(
                Instant::now() < until,
                "seed {seed}: the worker never drained"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        // A job the worker popped may still be mid-call; one more answered
        // call proves the worker is past it (calls are serial). The probe runs
        // under the schedule's own 3 ms bound, so on a loaded machine a quick
        // call can still time out (the public CI's Linux leg, 2026-10-07, with
        // a build running beside it): retry inside the same drain window. A
        // worker that never answers still fails here, and any other error at once.
        loop {
            match v.probe() {
                Ok(()) => break,
                // Only the plain timeout: a `Busy` refusal (an abandoned call
                // still inside the key store) is a real finding, reported at once.
                Err(WalletError::KeychainTimeout {
                    cause: KeychainTimeoutCause::Timeout,
                }) if Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("seed {seed}: the worker answers after the schedule: {e:?}"),
            }
        }

        let entered = lock(&vault.entered).clone();
        let count = |id: u64| entered.iter().filter(|e| **e == id).count();
        let mut seen = Seen::default();
        // 1. Never two calls inside the vault at once.
        assert_eq!(
            vault.max_in_flight.load(Ordering::SeqCst),
            1,
            "seed {seed}: two native calls were in flight at once"
        );
        for (id, answer) in &told {
            let n = count(*id);
            match answer {
                // 3. An Ok is the caller's OWN call, which entered exactly once.
                Told::Mine => {
                    assert_eq!(n, 1, "seed {seed}: call {id} answered Ok but entered {n}×");
                    seen.ok += 1;
                }
                Told::SomeoneElses(got) => {
                    panic!("seed {seed}: call {id} was handed another call's result {got:?}")
                }
                // 2. A refused call never reached the vault; a timed-out one
                //    reached it at most once (Running) or never (Queued).
                Told::Refused => {
                    assert_eq!(n, 0, "seed {seed}: refused call {id} still ran");
                    seen.refused += 1;
                }
                Told::Timeout => {
                    assert!(n <= 1, "seed {seed}: timed-out call {id} entered {n}×");
                    if n == 1 {
                        seen.timed_out_then_ran += 1;
                    } else {
                        seen.timed_out_never_ran += 1;
                    }
                }
                Told::Other(e) => panic!("seed {seed}: call {id}: unexpected {e}"),
            }
        }
        // Everything that entered belongs to a call that was answered.
        assert_eq!(
            entered.len(),
            told.iter().map(|(id, _)| count(*id)).sum::<usize>(),
            "seed {seed}: a call entered the vault that no caller submitted"
        );
        // 4. One thread, however the schedule went.
        assert_eq!(
            v.spawn_count(),
            1,
            "seed {seed}: more than one worker thread"
        );
        seen
    }

    /// 24 FIXED seeds (a failure names its seed and reproduces its choices),
    /// then NON-VACUITY: across them, every path the invariants speak about
    /// must have happened — a schedule set in which nothing timed out, nothing
    /// ran late or nothing was refused would pass the invariants vacuously.
    #[test]
    fn bounded_worker_invariants_hold_under_random_schedules() {
        let mut total = Seen::default();
        for i in 0..24u64 {
            let s = one_schedule(0xD1CE_5EED ^ i.wrapping_mul(0x2545_F491_4F6C_DD1D));
            total.ok += s.ok;
            total.timed_out_then_ran += s.timed_out_then_ran;
            total.timed_out_never_ran += s.timed_out_never_ran;
            total.refused += s.refused;
        }
        assert!(total.ok > 0, "no call ever succeeded");
        assert!(
            total.timed_out_then_ran > 0,
            "no call ever timed out while running"
        );
        assert!(
            total.timed_out_never_ran > 0,
            "no queued call was ever abandoned"
        );
        assert!(total.refused > 0, "no call was ever refused busy");
    }
}
