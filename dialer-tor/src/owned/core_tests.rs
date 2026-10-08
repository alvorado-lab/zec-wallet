//! The owned client's lifetime rules, driven with probes and no arti
//! (`tor-plugin.md` §12.8). Every test builds its own ledger and runtime.

use std::sync::atomic::AtomicUsize;
use std::sync::mpsc;
use std::time::Duration;

use super::*;

/// Generous for a loaded machine; the overrun tests pass their own.
const BUDGET: Duration = Duration::from_secs(10);
/// An overrun budget, far below the time a test blocks a worker for.
const TIGHT: Duration = Duration::from_millis(100);
/// How long a test blocks a worker or a poll to force an overrun.
const BLOCKED: Duration = Duration::from_millis(1500);

fn owned_runtime(ledger: &Arc<ClientLedger>, spawner: Spawner) -> OwnedRuntime {
    OwnedRuntime::new(ledger, "test-owned", BUDGET, spawner).expect("an owned runtime")
}

/// A host runtime for the tests' own awaits.
fn host() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a host runtime")
}

fn eventually(what: &str, mut ok: impl FnMut() -> bool) {
    let until = Instant::now() + BUDGET;
    while Instant::now() < until {
        if ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for: {what}");
}

/// The client stand-in: records the thread its drop ran on, and whether the
/// shutdown's outcome was already published at that moment.
struct Probe {
    dropped_on: Arc<Mutex<Option<String>>>,
    outcome_seen_at_drop: Arc<Mutex<Option<Option<Shutdown>>>>,
    outcome: Option<watch::Receiver<Option<Shutdown>>>,
    panic_on_drop: bool,
}

impl Probe {
    fn new() -> Self {
        Self {
            dropped_on: Arc::new(Mutex::new(None)),
            outcome_seen_at_drop: Arc::new(Mutex::new(None)),
            outcome: None,
            panic_on_drop: false,
        }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let name = std::thread::current().name().map(str::to_owned);
        *lock(&self.dropped_on) = Some(name.unwrap_or_else(|| "<unnamed>".into()));
        if let Some(rx) = &self.outcome {
            *lock(&self.outcome_seen_at_drop) = Some(*rx.borrow());
        }
        tracing::info!("probe dropped");
        if self.panic_on_drop {
            panic!("a client whose drop panics");
        }
    }
}

/// Held across an `.await` forever, recording the live count when finally
/// dropped — the shape of arti's loops that hold its managers.
struct HeldAcrossAwait {
    ledger: Arc<ClientLedger>,
    seen_at_drop: Arc<AtomicUsize>,
}

impl Drop for HeldAcrossAwait {
    fn drop(&mut self) {
        self.seen_at_drop.store(self.ledger.live(), SeqCst);
    }
}

fn install(owned: &Arc<Owned<Probe>>, probe: Probe) {
    host()
        .block_on(owned.mint(async move { Ok(probe) }))
        .expect("the probe installs");
}

#[test]
fn an_owned_runtime_ends_its_tasks_before_the_ledger_releases() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    assert_eq!(ledger.live(), 1, "counted from the runtime's construction");
    let seen = Arc::new(AtomicUsize::new(usize::MAX));
    let held = HeldAcrossAwait {
        ledger: Arc::clone(&ledger),
        seen_at_drop: Arc::clone(&seen),
    };
    owned.handle.spawn(async move {
        let _held = held;
        std::future::pending::<()>().await;
    });
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    assert_eq!(
        seen.load(SeqCst),
        1,
        "the task's held value dropped while the client was still counted"
    );
    assert_eq!(ledger.live(), 0);
    assert_eq!(ledger.stuck(), 0);
}

#[test]
fn an_overrun_shutdown_stays_counted_and_stuck() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let (entered, entered_rx) = mpsc::channel();
    owned.handle.spawn(async move {
        let _ = entered.send(());
        std::thread::sleep(BLOCKED); // blocks the one worker
    });
    entered_rx.recv().expect("the blocking task started");
    assert_eq!(host().block_on(owned.shutdown(TIGHT)), Shutdown::Overran);
    assert_eq!(ledger.live(), 1, "a stuck client stays counted");
    assert_eq!(ledger.stuck(), 1);
}

static ONE_OUTCOME_SPAWNS: AtomicUsize = AtomicUsize::new(0);

fn counting_spawner(job: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    ONE_OUTCOME_SPAWNS.fetch_add(1, SeqCst);
    thread_spawner(job)
}

#[test]
fn shutdown_through_any_clone_runs_once_and_every_caller_sees_one_outcome() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, counting_spawner));
    let other = Arc::clone(&owned);
    let host = host();
    let (a, b) = host.block_on(async {
        let a = owned.shutdown(BUDGET);
        let b = other.shutdown(BUDGET);
        tokio::join!(a, b)
    });
    let later = host.block_on(owned.shutdown(BUDGET));
    assert_eq!(
        (a, b, later),
        (Shutdown::Stopped, Shutdown::Stopped, Shutdown::Stopped)
    );
    // One shutdown starts two threads: the watchdog and its work thread. A
    // second shutdown would show four.
    assert_eq!(ONE_OUTCOME_SPAWNS.load(SeqCst), 2, "one shutdown ran");
}

#[test]
fn a_dropped_shutdown_future_still_finishes_the_shutdown() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    drop(owned.shutdown(BUDGET)); // never polled
    assert!(owned.is_closed(), "the hand-off happened before the future");
    eventually("the shutdown released the entry", || ledger.live() == 0);
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
}

#[test]
fn every_method_after_shutdown_fails_fast_with_closed() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    let host = host();
    assert_eq!(host.block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    assert!(matches!(owned.with(|_| ()), Err(TorDialError::Closed)));
    let call = host.block_on(owned.call(|_| async { Ok(()) }));
    assert!(matches!(call, Err(TorDialError::Closed)), "{call:?}");
    let mint = host.block_on(owned.mint(async { Ok(Probe::new()) }));
    assert!(matches!(mint, Err(TorDialError::Closed)), "{mint:?}");
}

#[test]
fn a_call_in_flight_at_shutdown_resolves_closed_never_a_panic() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    let host = host();
    let (started, started_rx) = mpsc::channel();
    let in_flight = Arc::clone(&owned);
    let call = host.spawn(async move {
        in_flight
            .call(move |_client| async move {
                let _ = started.send(());
                // A timer of the OWNED runtime, alive across the shutdown.
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(())
            })
            .await
    });
    started_rx
        .recv()
        .expect("the call started on the owned runtime");
    assert_eq!(host.block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    let result = host
        .block_on(call)
        .expect("the caller's task did not panic");
    assert!(matches!(result, Err(TorDialError::Closed)), "{result:?}");
}

#[test]
fn no_client_reference_outlives_stopped() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let mut probe = Probe::new();
    probe.outcome = Some(owned.outcome.subscribe());
    let dropped_on = Arc::clone(&probe.dropped_on);
    let seen_at_drop = Arc::clone(&probe.outcome_seen_at_drop);
    install(&owned, probe);
    let host = host();
    // Half the calls are surely IN FLIGHT at the shutdown, each holding a
    // reference to the client across an `.await` (the case where a reference
    // could outlive the slot's own); the other half race it.
    const HOLDERS: usize = 16;
    let holding = Arc::new(AtomicUsize::new(0));
    let mut racers: Vec<_> = (0..HOLDERS)
        .map(|_| {
            let (owned, holding) = (Arc::clone(&owned), Arc::clone(&holding));
            host.spawn(async move {
                let _ = owned
                    .call(move |client| async move {
                        holding.fetch_add(1, SeqCst);
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        drop(client);
                        Ok(())
                    })
                    .await;
            })
        })
        .collect();
    eventually("every holder is in flight", || {
        holding.load(SeqCst) == HOLDERS
    });
    racers.extend((0..HOLDERS).map(|_| {
        let owned = Arc::clone(&owned);
        host.spawn(async move {
            let _ = owned
                .call(|client| async move {
                    tokio::task::yield_now().await;
                    drop(client);
                    Ok(())
                })
                .await;
        })
    }));
    assert_eq!(host.block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    for racer in racers {
        host.block_on(racer).expect("no racer panicked");
    }
    let thread = lock(&dropped_on)
        .clone()
        .expect("the client was dropped by Stopped");
    assert!(
        thread == "dialer-tor-shutdown" || thread == "test-owned",
        "the client's last reference dropped on {thread:?}, off the owned runtime"
    );
    assert_eq!(
        *lock(&seen_at_drop),
        Some(None),
        "the client was dropped BEFORE the outcome was published"
    );
}

#[test]
fn dropping_a_call_aborts_its_task_and_rotates_nothing() {
    struct FlagOnDrop(Arc<AtomicBool>);
    impl Drop for FlagOnDrop {
        fn drop(&mut self) {
            self.0.store(true, SeqCst);
        }
    }
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    let task_dropped = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&task_dropped);
    // Built inside the host runtime: a timeout needs its timer.
    let gave_up = host().block_on(async {
        tokio::time::timeout(
            Duration::from_millis(50),
            owned.call(move |_client| async move {
                let _flag = FlagOnDrop(flag);
                std::future::pending::<Result<(), TorDialError>>().await
            }),
        )
        .await
    });
    assert!(gave_up.is_err(), "the caller gave up");
    eventually("the abandoned task was aborted", || {
        task_dropped.load(SeqCst)
    });
    assert!(
        !owned.is_closed(),
        "giving up on a call does not close the client"
    );
}

/// A stream whose every poll stays pending, holding a `Sleep` of the OWNED
/// runtime: polling it after that runtime shut down would panic.
struct OwnedTimerStream(Pin<Box<tokio::time::Sleep>>);

impl OwnedTimerStream {
    fn on(handle: &Handle) -> Self {
        let _inside = handle.enter();
        Self(Box::pin(tokio::time::sleep(Duration::from_secs(3600))))
    }
}

impl AsyncRead for OwnedTimerStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.0.as_mut().poll(cx).map(|()| Ok(()))
    }
}

impl AsyncWrite for OwnedTimerStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.0.as_mut().poll(cx).map(|()| Ok(buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn not_connected<T: std::fmt::Debug>(r: io::Result<T>) -> bool {
    matches!(r, Err(e) if e.kind() == io::ErrorKind::NotConnected)
}

#[test]
fn a_stream_after_shutdown_errors_on_read_and_write_and_drops_cleanly() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let mut stream = Gated::new(OwnedTimerStream::on(&owned.handle), owned.gate());
    let host = host();
    assert_eq!(host.block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    let mut buf = [0u8; 8];
    assert!(not_connected(host.block_on(stream.read(&mut buf))));
    assert!(not_connected(host.block_on(stream.write(b"x"))));
    drop(stream);
}

#[test]
fn a_stream_dropped_before_the_shutdown_is_safe() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let stream = Gated::new(OwnedTimerStream::on(&owned.handle), owned.gate());
    drop(stream);
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
}

#[test]
fn a_stream_dropped_during_the_shutdown_is_safe() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let stream = Gated::new(OwnedTimerStream::on(&owned.handle), owned.gate());
    let (entered, entered_rx) = mpsc::channel();
    owned.handle.spawn(async move {
        let _ = entered.send(());
        // Keeps `shutdown_timeout` running while the stream is dropped.
        std::thread::sleep(Duration::from_millis(300));
    });
    entered_rx.recv().expect("the worker is busy");
    let outcome = owned.shutdown(BUDGET);
    std::thread::sleep(Duration::from_millis(100));
    drop(stream);
    assert_eq!(host().block_on(outcome), Shutdown::Stopped);
}

#[test]
fn a_reader_parked_before_the_shutdown_wakes_with_an_error() {
    use tokio::io::AsyncReadExt;
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    // Pending with NOTHING registered on the owned runtime (a timer there is
    // woken by the runtime's own shutdown, which would hide the gate's wake):
    // only the gate can wake this reader.
    let mut stream = Gated::new(NeverReady, owned.gate());
    let host = host();
    let reader = host.spawn(async move {
        let mut buf = [0u8; 8];
        stream.read(&mut buf).await
    });
    let gate = Arc::clone(owned.gate());
    eventually("the reader is parked", || !lock(&gate.parked).is_empty());
    owned.start_shutdown(BUDGET);
    let read = host
        .block_on(async { tokio::time::timeout(Duration::from_secs(2), reader).await })
        .expect("the parked reader was woken, not left to a host timeout")
        .expect("the reader did not panic");
    assert!(not_connected(read));
}

/// The reviews of the built diff: a SPLIT stream parks its reader and its
/// writer separately, and the shutdown wakes BOTH (one slot per stream lost
/// the reader's waker to the writer's).
#[test]
fn a_split_streams_reader_and_writer_are_both_woken() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let stream = Gated::new(NeverReady, owned.gate());
    let (mut read_half, mut write_half) = tokio::io::split(stream);
    let host = host();
    let reader = host.spawn(async move {
        let mut buf = [0u8; 8];
        read_half.read(&mut buf).await
    });
    let gate = Arc::clone(owned.gate());
    eventually("the reader is parked", || lock(&gate.parked).len() == 1);
    let writer = host.spawn(async move { write_half.write(b"x").await });
    eventually("the writer is parked too", || lock(&gate.parked).len() == 2);
    owned.start_shutdown(BUDGET);
    let (read, write) = host.block_on(async {
        tokio::time::timeout(Duration::from_secs(2), async {
            (reader.await, writer.await)
        })
        .await
        .expect("both halves were woken")
    });
    assert!(not_connected(read.expect("the reader did not panic")));
    assert!(not_connected(write.expect("the writer did not panic")));
}

/// The reviews of the built diff: awaiting a shutdown is bounded even when
/// the client's drop blocks (arti's drop-time write stalled in the
/// filesystem): past the budget and the grace it reads `Overran`.
#[test]
fn awaiting_a_shutdown_whose_drop_blocks_reads_overran_in_bounded_time() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<BlockedDrop>::start(owned_runtime(&ledger, thread_spawner));
    host()
        .block_on(owned.mint(async { Ok(BlockedDrop) }))
        .expect("installed");
    let began = Instant::now();
    assert_eq!(host().block_on(owned.shutdown(TIGHT)), Shutdown::Overran);
    // The drop blocks for GRACE + BLOCKED; the caller must give up a whole
    // BLOCKED earlier than that, give or take a scheduling margin.
    assert!(
        began.elapsed() < TIGHT + SHUTDOWN_WAIT_GRACE + BLOCKED / 3,
        "the caller waited for the blocked drop"
    );
    // The `Overran` a caller reads is the ONE outcome: the ledger already
    // holds the client stuck (so no new client may start), and a later reader
    // agrees, even once the blocked drop has finished.
    assert_eq!(
        (ledger.live(), ledger.stuck()),
        (1, 1),
        "Overran was answered while the ledger still allowed a new client"
    );
    std::thread::sleep(SHUTDOWN_WAIT_GRACE + BLOCKED);
    assert_eq!(host().block_on(owned.shutdown(TIGHT)), Shutdown::Overran);
    assert_eq!((ledger.live(), ledger.stuck()), (1, 1), "stuck stays stuck");
}

/// A client whose drop blocks (arti's drop-time write stalled in the
/// filesystem) for far longer than `TIGHT` plus the grace, and well inside
/// `BUDGET`.
struct BlockedDrop;

impl Drop for BlockedDrop {
    fn drop(&mut self) {
        std::thread::sleep(SHUTDOWN_WAIT_GRACE + BLOCKED);
    }
}

/// The shutdown's own watchdog settles `Overran` at the budget, with NO
/// caller awaiting (`start_shutdown` only, as a synchronous host does): the
/// ledger holds the client stuck long before the blocked drop ends.
#[test]
fn a_blocked_shutdown_is_settled_overran_at_its_budget_with_no_waiter() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<BlockedDrop>::start(owned_runtime(&ledger, thread_spawner));
    host()
        .block_on(owned.mint(async { Ok(BlockedDrop) }))
        .expect("installed");
    let began = Instant::now();
    owned.start_shutdown(TIGHT);
    eventually("the watchdog stranded the client", || ledger.stuck() == 1);
    assert!(
        began.elapsed() < SHUTDOWN_WAIT_GRACE,
        "settled by the drop's end, not by the budget"
    );
    // Published just after the ledger commit, outside the lock.
    eventually("the outcome was published", || {
        owned.outcome.borrow().is_some()
    });
    assert_eq!(*owned.outcome.borrow(), Some(Shutdown::Overran));
}

/// A caller whose budget is shorter than the one the shutdown runs with
/// waits for the RUNNING budget: a host that starts the shutdown with one
/// budget and awaits it elsewhere with a shorter one must not strand a client
/// that stops inside the first (a false `RestartRequired` for the process).
#[test]
fn a_later_caller_with_a_shorter_budget_waits_for_the_running_one() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<BlockedDrop>::start(owned_runtime(&ledger, thread_spawner));
    host()
        .block_on(owned.mint(async { Ok(BlockedDrop) }))
        .expect("installed");
    owned.start_shutdown(BUDGET);
    assert_eq!(
        host().block_on(owned.shutdown(TIGHT)),
        Shutdown::Stopped,
        "the shorter caller gave up on a shutdown that finished inside its budget"
    );
    assert_eq!((ledger.live(), ledger.stuck()), (0, 0));
}

/// A caller that finds the shutdown already begun — the gate closed, the
/// hand-off not yet made — still holds the ONE outcome, so its own bound
/// settles it: `Overran` with the client stranded, never an `Overran` of its
/// own that leaves a new client allowed.
#[test]
fn a_caller_that_finds_the_shutdown_begun_settles_the_one_outcome() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    // The first caller's first step only, held there.
    assert!(owned.gate.close());
    assert_eq!(host().block_on(owned.shutdown(TIGHT)), Shutdown::Overran);
    assert_eq!(
        (ledger.live(), ledger.stuck()),
        (1, 1),
        "the second caller answered Overran with the ledger allowing a new client"
    );
    assert_eq!(*owned.outcome.borrow(), Some(Shutdown::Overran));
}

fn terminal_over(
    ledger: &Arc<ClientLedger>,
) -> (Arc<Terminal>, Arc<watch::Sender<Option<Shutdown>>>) {
    let publish = Arc::new(watch::Sender::new(None));
    let terminal = Terminal::new(Finish {
        entry: Some(ledger.enter()),
        publish: Some(Arc::clone(&publish)),
        stopping: None,
    });
    (terminal, publish)
}

fn ledger_after(outcome: Shutdown) -> (usize, usize) {
    match outcome {
        Shutdown::Stopped => (0, 0),
        Shutdown::Overran => (1, 1),
    }
}

/// A settler that loses answers the winner's committed outcome, never one of
/// its own — with the ledger already matching it.
#[test]
fn a_later_settler_answers_the_committed_outcome() {
    for (first, later) in [
        (Shutdown::Stopped, Shutdown::Overran),
        (Shutdown::Overran, Shutdown::Stopped),
    ] {
        let ledger = Arc::new(ClientLedger::default());
        let (terminal, publish) = terminal_over(&ledger);
        assert_eq!(terminal.settle(first), first);
        assert_eq!(
            terminal.settle(later),
            first,
            "a later settler answered its own outcome"
        );
        assert_eq!((ledger.live(), ledger.stuck()), ledger_after(first));
        assert_eq!(*publish.borrow(), Some(first));
    }
}

/// Settlers racing on threads all answer one outcome, and the ledger already
/// holds it when each of them answers.
#[test]
fn racing_settlers_agree_with_each_other_and_the_ledger() {
    const SETTLERS: usize = 8;
    for _ in 0..50 {
        let ledger = Arc::new(ClientLedger::default());
        let (terminal, _publish) = terminal_over(&ledger);
        let start = Arc::new(std::sync::Barrier::new(SETTLERS));
        let settlers: Vec<_> = (0..SETTLERS)
            .map(|i| {
                let terminal = Arc::clone(&terminal);
                let ledger = Arc::clone(&ledger);
                let start = Arc::clone(&start);
                std::thread::spawn(move || {
                    let mine = if i % 2 == 0 {
                        Shutdown::Stopped
                    } else {
                        Shutdown::Overran
                    };
                    start.wait();
                    let answered = terminal.settle(mine);
                    (answered, (ledger.live(), ledger.stuck()))
                })
            })
            .collect();
        let answers: Vec<_> = settlers
            .into_iter()
            .map(|t| t.join().expect("a settler"))
            .collect();
        let (one, _) = answers[0];
        for (answered, seen) in answers {
            assert_eq!(answered, one, "two settlers answered differently");
            assert_eq!(
                seen,
                ledger_after(one),
                "answered before the ledger held it"
            );
        }
    }
}

/// A stream that is never ready and keeps no waker: a reader waiting on a
/// channel nothing will ever close.
struct NeverReady;

impl AsyncRead for NeverReady {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

impl AsyncWrite for NeverReady {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Pending
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Pending
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

/// A stream whose poll blocks its thread until released: a poll "inside
/// arti" while the shutdown starts.
struct HeldPoll {
    entered: mpsc::Sender<()>,
    release: Arc<(Mutex<bool>, Condvar)>,
}

impl AsyncRead for HeldPoll {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let _ = self.entered.send(());
        let (lock_, cvar) = &*self.release;
        let mut released = lock(lock_);
        while !*released {
            released = cvar.wait(released).unwrap_or_else(|p| p.into_inner());
        }
        Poll::Ready(Ok(()))
    }
}

fn poll_once_on_a_thread(mut stream: Gated<HeldPoll>) -> std::thread::JoinHandle<Gated<HeldPoll>> {
    std::thread::spawn(move || {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        let mut storage = [0u8; 1];
        let mut buf = ReadBuf::new(&mut storage);
        let _ = Pin::new(&mut stream).poll_read(&mut cx, &mut buf);
        stream
    })
}

fn release(flag: &Arc<(Mutex<bool>, Condvar)>) {
    *lock(&flag.0) = true;
    flag.1.notify_all();
}

#[test]
fn a_poll_in_progress_holds_the_runtime_until_it_returns() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let (entered, entered_rx) = mpsc::channel();
    let gate_release = Arc::new((Mutex::new(false), Condvar::new()));
    let stream = Gated::new(
        HeldPoll {
            entered,
            release: Arc::clone(&gate_release),
        },
        owned.gate(),
    );
    let poller = poll_once_on_a_thread(stream);
    entered_rx.recv().expect("the poll is inside the stream");
    let outcome = owned.shutdown(BUDGET);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        ledger.live(),
        1,
        "the runtime waits for the poll in progress"
    );
    assert!(
        owned.outcome.borrow().is_none(),
        "no outcome while a poll is inside"
    );
    release(&gate_release);
    assert_eq!(host().block_on(outcome), Shutdown::Stopped);
    drop(poller.join().expect("the poller"));
}

#[test]
fn a_gate_wait_past_the_budget_is_overran() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let (entered, entered_rx) = mpsc::channel();
    let gate_release = Arc::new((Mutex::new(false), Condvar::new()));
    let stream = Gated::new(
        HeldPoll {
            entered,
            release: Arc::clone(&gate_release),
        },
        owned.gate(),
    );
    let poller = poll_once_on_a_thread(stream);
    entered_rx.recv().expect("the poll is inside the stream");
    assert_eq!(host().block_on(owned.shutdown(TIGHT)), Shutdown::Overran);
    assert_eq!(ledger.stuck(), 1);
    release(&gate_release);
    drop(poller.join().expect("the poller"));
}

/// A stream whose poll panics.
struct PanicPoll;

impl AsyncRead for PanicPoll {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        panic!("a poll that panics inside the stream");
    }
}

#[test]
fn a_panicking_poll_does_not_pin_the_gate() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let mut stream = Gated::new(PanicPoll, owned.gate());
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut cx = Context::from_waker(Waker::noop());
        let mut storage = [0u8; 1];
        let mut buf = ReadBuf::new(&mut storage);
        let _ = Pin::new(&mut stream).poll_read(&mut cx, &mut buf);
    }));
    assert!(panicked.is_err());
    assert_eq!(
        host().block_on(owned.shutdown(BUDGET)),
        Shutdown::Stopped,
        "the panicked poll released the gate"
    );
}

#[test]
fn the_last_drop_without_shutdown_takes_the_same_path() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, host_handle) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    drop(owned);
    // Dropped on an ASYNC thread: a runtime dropped there would panic.
    host().block_on(async move { drop(host_handle) });
    eventually("the unawaited shutdown released the entry", || {
        ledger.live() == 0
    });
    assert_eq!(ledger.stuck(), 0);
}

#[test]
fn cancelling_spawn_owned_mid_mint_counts_until_the_runtime_ended() {
    let ledger = Arc::new(ClientLedger::default());
    let seen = Arc::new(AtomicUsize::new(usize::MAX));
    let runtime = owned_runtime(&ledger, thread_spawner);
    let held = HeldAcrossAwait {
        ledger: Arc::clone(&ledger),
        seen_at_drop: Arc::clone(&seen),
    };
    let bring_up = async move {
        let (owned, host) = Owned::<Probe>::start(runtime);
        owned
            .mint(async move {
                let _held = held;
                std::future::pending::<Result<Probe, TorDialError>>().await
            })
            .await?;
        Ok::<_, TorDialError>(host)
    };
    let aborted =
        host().block_on(async { tokio::time::timeout(Duration::from_millis(100), bring_up).await });
    assert!(aborted.is_err(), "the bring-up was aborted mid-mint");
    eventually("the half-built client's runtime ended", || {
        ledger.live() == 0
    });
    assert_eq!(
        seen.load(SeqCst),
        1,
        "the mint's held value dropped while the client was still counted"
    );
}

/// The minted client lives in the slot, never with the caller: when the
/// caller lets go (on its own thread), the client is dropped elsewhere.
#[test]
fn the_minted_client_is_never_dropped_on_the_callers_thread() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, host_handle) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let probe = Probe::new();
    let dropped_on = Arc::clone(&probe.dropped_on);
    install(&owned, probe);
    drop(owned);
    drop(host_handle); // on this, the test's thread
    eventually("the client was dropped", || lock(&dropped_on).is_some());
    let thread = lock(&dropped_on).clone().expect("dropped");
    assert_ne!(
        Some(thread.as_str()),
        std::thread::current().name(),
        "the client dropped on the caller's thread"
    );
}

#[test]
fn a_failed_mint_stays_counted_until_its_runtime_ended() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, host_handle) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let seen = Arc::new(AtomicUsize::new(usize::MAX));
    let held = HeldAcrossAwait {
        ledger: Arc::clone(&ledger),
        seen_at_drop: Arc::clone(&seen),
    };
    let failed = host().block_on(owned.mint(async move {
        // Work the half-built client started before it failed.
        tokio::spawn(async move {
            let _held = held;
            std::future::pending::<()>().await;
        });
        Err::<Probe, _>(TorDialError::Setup {
            kind: ErrorKind::Other,
        })
    }));
    assert!(matches!(failed, Err(TorDialError::Setup { .. })));
    assert_eq!(ledger.live(), 1, "still counted: its work has not ended");
    drop(owned);
    drop(host_handle);
    eventually("the failed client's runtime ended", || ledger.live() == 0);
    assert_eq!(seen.load(SeqCst), 1);
}

#[test]
fn a_panicking_shutdown_thread_publishes_overran() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    let mut probe = Probe::new();
    probe.panic_on_drop = true;
    install(&owned, probe);
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Overran);
    assert_eq!(ledger.stuck(), 1);
}

fn failing_spawner(_job: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    Err(io::Error::other("no thread for you"))
}

#[test]
fn a_shutdown_thread_that_cannot_start_strands_and_answers_overran() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, failing_spawner));
    install(&owned, Probe::new());
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Overran);
    assert_eq!((ledger.live(), ledger.stuck()), (1, 1));
}

/// Single-shot for the process: ONE test uses `second_spawn_fails`, so its
/// second spawn is the work thread of that test's one shutdown.
static SECOND_SPAWN_FAILS: AtomicUsize = AtomicUsize::new(0);

/// The watchdog's thread starts; the work's thread, the second spawn, cannot.
fn second_spawn_fails(job: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    if SECOND_SPAWN_FAILS.fetch_add(1, SeqCst) == 1 {
        return Err(io::Error::other("no second thread"));
    }
    thread_spawner(job)
}

/// The watchdog runs but its work cannot start: the runtime and client are
/// leaked on the watchdog's thread, never dropped, and the shutdown settles
/// `Overran` with the client stranded.
#[test]
fn a_work_thread_that_cannot_start_strands_and_answers_overran() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, second_spawn_fails));
    let probe = Probe::new();
    let dropped_on = Arc::clone(&probe.dropped_on);
    install(&owned, probe);
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Overran);
    assert_eq!(
        (ledger.live(), ledger.stuck(), ledger.stopping()),
        (1, 1, 0)
    );
    assert!(
        lock(&dropped_on).is_none(),
        "the client was dropped; it must be leaked, never dropped"
    );
}

/// A shutdown in flight is counted in `stopping()` from the hand-off until it
/// settles, and leaves the count only after its entry is stranded.
#[test]
fn a_shutdown_in_flight_is_counted_until_it_settles() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<BlockedDrop>::start(owned_runtime(&ledger, thread_spawner));
    host()
        .block_on(owned.mint(async { Ok(BlockedDrop) }))
        .expect("installed");
    assert_eq!(ledger.stopping(), 0);
    owned.start_shutdown(TIGHT);
    assert_eq!(ledger.stopping(), 1, "counted from the hand-off");
    assert!(
        host().block_on(ledger.shutdowns_settled()),
        "the watchdog settled it within its bound"
    );
    assert_eq!(
        (ledger.stopping(), ledger.stuck()),
        (0, 1),
        "it left the count before its entry was stranded"
    );
}

fn offline_config(dirs: &tempfile::TempDir) -> crate::TorClientConfig {
    let mut builder = arti_client::config::TorClientConfigBuilder::from_directories(
        dirs.path().join("state"),
        dirs.path().join("cache"),
    );
    builder.storage().permissions().dangerously_trust_everyone();
    builder.build().expect("the test config must build")
}

/// Far past `TIGHT` plus the grace: a watchdog thread that starts this late.
const LATE: Duration = Duration::from_secs(6);

/// Every thread starts `LATE`: a watchdog the OS has not scheduled.
fn late_spawner(job: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    std::thread::Builder::new()
        .name("test-late".into())
        .spawn(move || {
            std::thread::sleep(LATE);
            job();
        })
        .map(|_| ())
}

/// `spawn_owned`'s wait for a shutdown in flight is bounded by that
/// shutdown's budget plus the grace, even when its watchdog never ran: it
/// then refuses (fail closed), never waits on.
#[test]
fn a_spawn_owned_waiting_past_the_bound_is_refused() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, late_spawner));
    install(&owned, Probe::new());
    owned.start_shutdown(TIGHT);
    let dirs = tempfile::TempDir::new().expect("tempdir");
    let began = Instant::now();
    let refused = host().block_on(super::super::spawn_owned_with(
        offline_config(&dirs),
        &ledger,
        super::super::OwnedOptions::default(),
        thread_spawner,
    ));
    assert!(
        matches!(refused, Err(TorDialError::RestartRequired)),
        "{refused:?}"
    );
    assert!(
        began.elapsed() < LATE,
        "the wait outlasted its bound ({:?})",
        began.elapsed()
    );
    assert_eq!(ledger.live(), 1, "nothing new was counted");
}

/// A caller racing the first one, which has closed the gate but not yet
/// handed off, returns with the shutdown already counted in `stopping`: a
/// `spawn_owned` it starts next waits for that shutdown instead of starting a
/// client beside it.
#[test]
fn a_caller_racing_the_first_returns_with_the_shutdown_counted() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    // The first caller stalls on the client slot, after it closed the gate.
    let held = lock(&owned.client);
    let first = {
        let owned = Arc::clone(&owned);
        std::thread::spawn(move || owned.start_shutdown(BUDGET))
    };
    eventually("the first caller closed the gate", || owned.is_closed());
    owned.start_shutdown(BUDGET);
    assert_eq!(
        ledger.stopping(),
        1,
        "a racing caller returned before the shutdown was counted"
    );
    drop(held);
    first.join().expect("the first caller");
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    assert_eq!((ledger.live(), ledger.stopping()), (0, 0));
}

/// A `spawn_owned` waiting for a shutdown on an executor whose waker panics
/// cannot split the outcome: when the shutdown settles `Stopped` and wakes
/// that waiter, every `shutdown` caller still reads `Stopped`, as the ledger
/// says.
#[test]
fn a_panicking_waiter_cannot_split_the_outcome_from_the_ledger() {
    struct Panics;
    impl std::task::Wake for Panics {
        fn wake(self: Arc<Self>) {
            panic!("a waiting host's waker that panics");
        }
    }
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    let rt = host();
    // Hold the gate shut for a moment so the waiter registers before the
    // shutdown can settle.
    let held = lock(&owned.client);
    let starter = {
        let owned = Arc::clone(&owned);
        std::thread::spawn(move || owned.start_shutdown(BUDGET))
    };
    eventually("the shutdown is counted", || ledger.stopping() == 1);
    let waiter = {
        let _inside = rt.enter();
        let mut waiter = Box::pin(ledger.shutdowns_settled());
        let waker = Waker::from(Arc::new(Panics));
        let mut cx = Context::from_waker(&waker);
        assert!(waiter.as_mut().poll(&mut cx).is_pending());
        waiter
    };
    drop(held);
    starter.join().expect("the shutdown started");
    assert_eq!(rt.block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    assert_eq!((ledger.live(), ledger.stuck()), (0, 0));
    eventually("the shutdown left the count", || ledger.stopping() == 0);
    drop(waiter);
}

/// A client whose drop takes a set time: a shutdown that stops cleanly, but
/// not at once.
struct DropTakes(Duration);

impl Drop for DropTakes {
    fn drop(&mut self) {
        std::thread::sleep(self.0);
    }
}

/// A longer shutdown handed off WHILE `spawn_owned` waits extends the wait:
/// the wait is not cut short at the bound it read when it began, which would
/// answer `RestartRequired` with nothing stuck.
#[test]
fn a_longer_shutdown_started_during_the_wait_extends_it() {
    const SHORT: Duration = Duration::from_millis(300);
    // The first stops inside a budget four times its drop: no overrun on a
    // loaded machine. Its bound when the wait begins: this plus the grace.
    const FIRST_BUDGET: Duration = Duration::from_millis(1200);
    let ledger = Arc::new(ClientLedger::default());
    let rt = host();
    let first = Owned::<DropTakes>::start(owned_runtime(&ledger, thread_spawner));
    let second = Owned::<DropTakes>::start(owned_runtime(&ledger, thread_spawner));
    rt.block_on(first.0.mint(async move { Ok(DropTakes(SHORT)) }))
        .expect("installed");
    // Stops cleanly inside its own budget, but well after the first bound.
    let slow = FIRST_BUDGET + SHUTDOWN_WAIT_GRACE + BLOCKED;
    rt.block_on(second.0.mint(async move { Ok(DropTakes(slow)) }))
        .expect("installed");
    first.0.start_shutdown(FIRST_BUDGET);
    let dirs = tempfile::TempDir::new().expect("tempdir");
    let config = offline_config(&dirs);
    let (entering, entered) = mpsc::channel();
    let waiting = {
        let ledger = Arc::clone(&ledger);
        rt.spawn(async move {
            // The wait reads its bound in this same first poll, before it
            // yields: once this is received, it has read the SHORT bound.
            let _ = entering.send(());
            super::super::spawn_owned_with(
                config,
                &ledger,
                super::super::OwnedOptions::default(),
                thread_spawner,
            )
            .await
        })
    };
    entered.recv().expect("the wait began");
    std::thread::sleep(SHORT / 6);
    assert_eq!(
        ledger.stopping(),
        1,
        "the first shutdown is still in flight"
    );
    second.0.start_shutdown(BUDGET);
    let started = rt.block_on(waiting).expect("the waiting task");
    let dialer = started.expect("the wait outlasted the first bound and the client started");
    assert_eq!(ledger.stuck(), 0);
    assert_eq!(rt.block_on(dialer.shutdown(BUDGET)), Shutdown::Stopped);
}

/// A `spawn_owned` dropped while it waits for a shutdown in flight holds
/// nothing: no entry counted, no waiter left behind.
#[test]
fn a_spawn_owned_dropped_while_it_waits_holds_nothing() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<BlockedDrop>::start(owned_runtime(&ledger, thread_spawner));
    host()
        .block_on(owned.mint(async { Ok(BlockedDrop) }))
        .expect("installed");
    owned.start_shutdown(TIGHT);
    let dirs = tempfile::TempDir::new().expect("tempdir");
    let gave_up = host().block_on(async {
        tokio::time::timeout(
            TIGHT / 4,
            super::super::spawn_owned_with(
                offline_config(&dirs),
                &ledger,
                super::super::OwnedOptions::default(),
                thread_spawner,
            ),
        )
        .await
    });
    assert!(gave_up.is_err(), "it was still waiting when dropped");
    assert_eq!(
        (ledger.live(), ledger.stopping()),
        (1, 1),
        "nothing new counted"
    );
    assert!(host().block_on(ledger.shutdowns_settled()));
    assert_eq!(
        (ledger.live(), ledger.stuck(), ledger.stopping()),
        (1, 1, 0)
    );
}

/// A new client started while an earlier one's shutdown is still in flight
/// waits for that shutdown's verdict: here it overruns, so the new one is
/// refused, and a stuck client never runs beside a live one.
#[test]
fn spawn_owned_waits_for_a_shutdown_in_flight_and_refuses_if_it_overran() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<BlockedDrop>::start(owned_runtime(&ledger, thread_spawner));
    host()
        .block_on(owned.mint(async { Ok(BlockedDrop) }))
        .expect("installed");
    owned.start_shutdown(TIGHT);
    assert_eq!(
        ledger.stuck(),
        0,
        "not yet settled when the new client asks"
    );
    let dirs = tempfile::TempDir::new().expect("tempdir");
    let refused = host().block_on(super::super::spawn_owned_with(
        offline_config(&dirs),
        &ledger,
        super::super::OwnedOptions::default(),
        thread_spawner,
    ));
    assert!(
        matches!(refused, Err(TorDialError::RestartRequired)),
        "a new client started beside a shutdown that then overran: {refused:?}"
    );
    assert_eq!((ledger.live(), ledger.stuck()), (1, 1));
}

/// A waker that panics when the shutdown wakes the parked polls neither
/// stops the shutdown nor the other wakers, and does not reach the caller.
#[test]
fn a_panicking_waker_does_not_stop_the_shutdown_or_the_other_wakers() {
    struct Panics;
    impl std::task::Wake for Panics {
        fn wake(self: Arc<Self>) {
            panic!("a host waker that panics");
        }
    }
    struct Counts(AtomicUsize);
    impl std::task::Wake for Counts {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, SeqCst);
        }
    }
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    let counted = Arc::new(Counts(AtomicUsize::new(0)));
    owned
        .gate
        .park(1, Side::Read, &Waker::from(Arc::new(Panics)));
    owned
        .gate
        .park(2, Side::Read, &Waker::from(Arc::clone(&counted)));
    let started = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        owned.start_shutdown(BUDGET);
    }));
    assert!(
        started.is_ok(),
        "a parked poll's panicking waker reached the caller"
    );
    assert_eq!(counted.0.load(SeqCst), 1, "the other parked poll was woken");
    assert_eq!(host().block_on(owned.shutdown(BUDGET)), Shutdown::Stopped);
    assert_eq!((ledger.live(), ledger.stuck()), (0, 0));
}

/// Awaiting the shutdown from a task on the client's OWN runtime starts it
/// and answers `Overran` at once, in every build, with the client stranded.
#[test]
fn a_shutdown_awaited_on_its_own_runtime_starts_it_and_answers_overran() {
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner));
    install(&owned, Probe::new());
    let (answer, answered) = mpsc::channel();
    let inner = Arc::clone(&owned);
    owned.handle.spawn(async move {
        // Polled once, with no await: the task is not cancelled half-way.
        let future = std::pin::pin!(inner.shutdown(BUDGET));
        let mut cx = Context::from_waker(Waker::noop());
        let _ = answer.send((future.poll(&mut cx), inner.is_closed()));
    });
    let answer = answered.recv();
    assert!(owned.is_closed(), "the misuse stopped nothing");
    let (polled, closed) = answer.expect("the task answered");
    assert_eq!(polled, Poll::Ready(Shutdown::Overran));
    assert!(closed, "the shutdown was started");
    assert_eq!(ledger.stuck(), 1);
}

/// A poll that returns `Pending` just as the gate closes, after the shutdown
/// has woken every parked poll, wakes itself: it is never left parked.
#[test]
fn a_poll_that_meets_the_gate_closing_wakes_itself() {
    struct ClosesTheGate(Arc<Gate>);
    impl AsyncRead for ClosesTheGate {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            self.0.close();
            Poll::Pending
        }
    }
    struct Counts(AtomicUsize);
    impl std::task::Wake for Counts {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, SeqCst);
        }
    }
    let gate = Arc::new(Gate::default());
    let mut stream = Gated::new(ClosesTheGate(Arc::clone(&gate)), &gate);
    let counted = Arc::new(Counts(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&counted));
    let mut cx = Context::from_waker(&waker);
    let mut bytes = [0u8; 8];
    let mut buf = ReadBuf::new(&mut bytes);
    assert!(
        Pin::new(&mut stream)
            .poll_read(&mut cx, &mut buf)
            .is_pending()
    );
    assert_eq!(counted.0.load(SeqCst), 1, "the poll was left parked");
    assert!(lock(&gate.parked).is_empty());
}

#[test]
fn the_shutdown_thread_logs_through_the_construction_dispatcher() {
    use tracing_subscriber::layer::{Context as LayerContext, Layer, SubscriberExt};

    struct Count(Arc<AtomicUsize>);
    impl<S: tracing::Subscriber> Layer<S> for Count {
        fn on_event(&self, _event: &tracing::Event<'_>, _ctx: LayerContext<'_, S>) {
            self.0.fetch_add(1, SeqCst);
        }
    }
    let built_under = Arc::new(AtomicUsize::new(0));
    let dropped_under = Arc::new(AtomicUsize::new(0));
    let construction = tracing::Dispatch::new(
        tracing_subscriber::registry().with(Count(Arc::clone(&built_under))),
    );
    let dropping = tracing::Dispatch::new(
        tracing_subscriber::registry().with(Count(Arc::clone(&dropped_under))),
    );
    let ledger = Arc::new(ClientLedger::default());
    let (owned, _host) = tracing::dispatcher::with_default(&construction, || {
        Owned::<Probe>::start(owned_runtime(&ledger, thread_spawner))
    });
    install(&owned, Probe::new());
    let outcome =
        tracing::dispatcher::with_default(&dropping, || host().block_on(owned.shutdown(BUDGET)));
    assert_eq!(outcome, Shutdown::Stopped);
    assert!(
        built_under.load(SeqCst) >= 1,
        "the client's drop-time log went through the construction dispatcher"
    );
    assert_eq!(
        dropped_under.load(SeqCst),
        0,
        "the dropping thread's dispatcher saw the client's logs"
    );
}
