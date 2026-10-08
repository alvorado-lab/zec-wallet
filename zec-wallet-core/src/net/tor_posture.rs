//! The wallet's ONE Tor posture: the sticky fell-back latch and the patience
//! clock that decides WHEN a `Preferred` wallet may stop insisting on the
//! private path (ADR-0552 — the maintainer's minute).
//!
//! Its own module for the reason `mod.rs` already gives for `readiness_gate`:
//! `dialer.rs`'s tail is a test module with cited mutant rows, and clippy
//! forbids items after it. Keeping BOTH facts in one object is also what lets
//! `PolicyDialer`, `with_fallback` and `resolve_dialer` keep their shapes — the
//! latch already travelled to every dialer, and the clock travels with it.
//!
//! Why the clock measures from the last SUCCESS rather than the first failure:
//! a caller whose own RPC budget is shorter than the dial bound cancels its
//! dial before the dialer can observe a failure at all. A first-failure clock
//! could then never start and the wallet would insist forever; a last-success
//! clock advances no matter who cancels. (That race is now also closed at the
//! constants — `DIAL_TIMEOUT_SECS < GRPC_UNARY_TIMEOUT_SECS` — but the
//! reasoning stands on its own and the clock does not depend on the margin.)
//!
//! WHAT COUNTS AS SUCCESS, and why it is not a connect (ADR-0552 stage 1b, the
//! security and crypto angles' shared MEDIUM): a transport that ACCEPTS and
//! then blackholes is the commonest active-censorship shape and a hostile
//! bridge's cheapest move. If a bare connect restarted the window, such a
//! transport would reset it on every attempt and the minute would never
//! elapse — the ruling's promise ("60 s of the private path not working")
//! would be unimplemented for exactly the case it was written for. So a dial
//! moves nothing (the swap class excepted, and it says why); the window moves
//! when an RPC completes over a connection the PRIVATE arm served, which is
//! the first moment there is evidence the circuit carried gRPC.
//!
//! EVIDENCE IS ATTRIBUTED TO THE CONNECTION IT RODE (phase 2 §2b). The arm is
//! decided once, by `PolicyDialer` at dial time, and that verdict ([`DialArm`])
//! travels with the connection to whoever reports on it. It used to be one
//! flag per CLASS, written by the latest dial — but `Broadcast` mints a
//! connection per send and its callers run concurrently, so a sibling's
//! fall-back could relabel a private connection mid-flight (a genuine success
//! dropped) and a sibling's private dial could relabel a clearnet one (a
//! CLEARNET RPC restarting the private window). The posture therefore keeps no
//! idea of which arm a class is "on": it is told, per report.
//!
//! WHY THE WINDOW HAS A SUBJECT (the same review's second MEDIUM): sync rides
//! one long-lived circuit while every broadcast mints a fresh one. A censor
//! that carries the established stream and refuses new circuits would, under
//! one wallet-wide window, let each sync pass reset the window a starved send
//! is measuring — a payment that never leaves, with the UI reading UpToDate
//! and `Active`. Each [`PathClass`] therefore keeps its own window.
//!
//! SCOPE OF THE WINDOW, stated rather than implied (the same review's LOW):
//! it is PER SESSION and it counts device-AWAKE time. Per session because only
//! the server switch carries a posture across — create, open and rescan each
//! take a fresh one — so a relaunch on a hostile network insists again from
//! zero. Awake because the clock does not advance while the DEVICE is
//! suspended.
//!
//! **"Awake" does NOT mean "in the foreground", and the difference is the
//! common case.** `Instant` is `mach_absolute_time` on Apple and
//! `CLOCK_MONOTONIC` on Linux: both keep running while an app is merely
//! BACKGROUNDED on an awake device, so a user who switches apps for two
//! minutes spends the whole window. Only system sleep stops it. An earlier
//! version of this paragraph said "a phone asleep in a pocket spends none of
//! its minute" and left the backgrounded case to be inferred, which read as a
//! stronger guarantee than the code gives.
//!
//! That is also why the window has a SECOND conjunct (see
//! [`ClassWindow::first_failure_ms`]): time alone, however it is counted,
//! cannot distinguish a broken private path from an unused one.
//! Both defaults err towards INSISTING, which is the private
//! direction; they are recorded here and in ADR-0552 so a future change to
//! either is a decision rather than a drift.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tokio::sync::watch;
use tokio::time::Instant;

use crate::constants::{
    BROADCAST_ISOLATION_KEY_PREFIX, TOR_PATIENCE_SECS, WALLET_SYNC_ISOLATION_KEY,
};

/// Which of the wallet's circuit families a dial belongs to — the SUBJECT of a
/// patience window.
///
/// Derived from the isolation key every dial already carries, so this adds no
/// plumbing. It is deliberately a small CLOSED set rather than the key itself:
/// each broadcast mints a fresh key, so keying the window by the key would give
/// every send its own fresh minute — the opposite of the ruling, which insists
/// for a minute of silence and not for a minute per attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathClass {
    /// The long-lived sync circuit (`WALLET_SYNC_ISOLATION_KEY`) — one channel,
    /// one circuit, reused for every sync RPC. The circuit a censor most
    /// cheaply keeps alive.
    Sync,
    /// A broadcast circuit (`BROADCAST_ISOLATION_KEY_PREFIX-*`) — the MONEY
    /// path, a fresh circuit per send. The circuit a censor most cheaply
    /// refuses, which is why it must not depend on the sync circuit's health.
    Broadcast,
    /// The SWAP on-ramp's HTTP dials (`swap/…`, minted in the peer crate) — the
    /// one class where a CONNECT is accepted as evidence, because nothing on it
    /// can ever report an RPC; see [`PathClass::connect_is_the_only_evidence`].
    Swap,
    /// Everything else: the ephemeral-detect poll, the sync-server probe, and
    /// any dial with no isolation key. Short-lived gRPC work that must neither
    /// starve the classes above nor be starved by them. It CONFIRMS like they
    /// do, so it keeps their strict rule (phase 2 §2c: it shared a bucket with
    /// swap until then, and inherited a weakening justified by swap alone — on
    /// the poll that finds stranded TEX funds, whose faults are silent).
    Other,
}

/// The `swap/` stem every key the swap adapter mints carries (`swap/quote`,
/// `swap/tokens`, `swap/id/<token>` — `zec-wallet-swap-near`). Matched here as
/// a literal because core may not depend on the adapter (the pluggability
/// invariant); `each_isolation_class_keeps_its_own_window` pins the adapter's
/// real keys against it.
const SWAP_ISOLATION_KEY_STEM: &str = "swap/";

impl PathClass {
    /// The number of distinct windows a posture keeps. It sizes
    /// [`TorPosture::windows`]; [`Self::slot`]'s exhaustive match is what makes
    /// a new variant a COMPILE error rather than a runtime panic, so the two
    /// must be read together.
    const COUNT: usize = 4;

    /// THE classifier — read by the dialer (which has the key at dial time) and
    /// by the gRPC client (which has it at connect time), never re-derived at a
    /// second site. Both must agree or a dial would fail one window while its
    /// RPC confirmed another.
    pub(crate) fn of(isolation_key: Option<&str>) -> Self {
        match isolation_key {
            // Exact, not a prefix: the sync key is ONE stable label by
            // construction (`WALLET_SYNC_ISOLATION_KEY`'s contract).
            Some(key) if key == WALLET_SYNC_ISOLATION_KEY => Self::Sync,
            Some(key) if key.starts_with(BROADCAST_ISOLATION_KEY_PREFIX) => Self::Broadcast,
            Some(key) if key.starts_with(SWAP_ISOLATION_KEY_STEM) => Self::Swap,
            _ => Self::Other,
        }
    }

    /// Whether a bare CONNECT counts as evidence for this class, because no RPC
    /// on this path can report back.
    ///
    /// True for [`Self::Swap`] ONLY, and it is a deliberate weakening with a
    /// stated cost. The swap on-ramp dials through the same `PolicyDialer`
    /// (`Wallet::swap_dialer`) but speaks HTTP from a PEER CRATE that cannot
    /// reach this posture — the pluggability invariant forbids core from
    /// depending on the adapter — so a swap dial can never be confirmed the way
    /// a gRPC call is. Without this, stage 1b would have introduced a REGRESSION
    /// it was not asked for: swap dials would never move the window, so a swap
    /// dial failure a minute later would reach clearnet where before that chunk
    /// it would not have. Leaking a swap's traffic to fix a sync-path bug is not
    /// a trade worth making.
    ///
    /// What it gives up, stated rather than buried: a transport that accepts and
    /// blackholes holds THIS class insisting forever. That is the exact hole
    /// finding (i) closed for [`Self::Sync`] and [`Self::Broadcast`], and it
    /// stays closed there — those are the paths where "never switches" is a
    /// SILENT stall (a wallet that never syncs, a payment that never leaves,
    /// both under a UI reading fine). A swap is foreground, user-initiated
    /// work: one that never switches FAILS VISIBLY, in front of the person who
    /// started it, which is honest degradation rather than a stall nobody can
    /// see.
    ///
    /// `pub(crate)` for ONE reader: `PolicyDialer` must not treat an accept as
    /// the blackhole shape on the class where the accept IS the evidence.
    pub(crate) fn connect_is_the_only_evidence(self) -> bool {
        matches!(self, Self::Swap)
    }

    /// The two classes that may TRIGGER
    /// [`TorState::Unanswered`](crate::state::TorState) (stage S1 `truth`,
    /// contract §3.2 "SUBJECT"): the ones whose silence is a SILENT stall for
    /// the user — a wallet that never syncs, a payment that never leaves, both
    /// under a UI reading fine. `Swap` is out because a connect is its only
    /// evidence, so it can never be "unanswered" in this sense; `Other` is out
    /// because it is short-lived foreground work whose faults are its own to
    /// report. "Any class" would restate FR-36's question rather than answer
    /// it.
    ///
    /// TRIGGERING only. REFUTING is a wider set and deliberately so — see
    /// [`Self::carriage_is_an_rpc`]: a class whose RPCs cannot raise the state
    /// must still be able to end it, because "nothing has come back over the
    /// private connection" is a fact about the PATH and any completed RPC over
    /// it refutes the sentence whatever class asked for it.
    const CONFIRMABLE: [Self; 2] = [Self::Sync, Self::Broadcast];

    /// Every class, for the readers that take the UNION over all of them.
    ///
    /// **NOT compile-enforced, corrected (code reviewer MAJOR).** This
    /// said a new variant "is a compile error at the array literal rather than
    /// a class silently missing from the union". It is not: `COUNT` and this
    /// array are both hand-written, with no type-level link to the enum's
    /// variant count, so adding a variant and bumping `COUNT` without listing
    /// it here compiles — and the union then silently skips a class, which for
    /// `carried_nothing_for_ms` means silence a class is banking is invisible.
    ///
    /// What IS compile-enforced is [`Self::slot`]'s exhaustive match, which
    /// forces a new variant to get a slot. The test `every_path_class_is_in_all`
    /// ties the two together: an exhaustive match forces the arm to exist, and
    /// the assertion inside it then fails until the variant is listed here. So
    /// the real guarantee is "compile error until you write the arm, then a red
    /// test" — which is enough, and is what this comment now says.
    const ALL: [Self; Self::COUNT] = [Self::Sync, Self::Broadcast, Self::Swap, Self::Other];

    /// Whether a failing run on this class can make the path read
    /// `Unanswered` — see [`Self::CONFIRMABLE`].
    fn counts_toward_unanswered(self) -> bool {
        Self::CONFIRMABLE.contains(&self)
    }

    /// Whether this class's success stamp means an RPC CARRIED — and so whether
    /// it may refute "nothing has come back over the private connection".
    ///
    /// Every class but [`Self::Swap`], whose stamp is written by a bare CONNECT
    /// ([`Self::connect_is_the_only_evidence`]). That exclusion is the whole
    /// reason this predicate exists rather than a bare "every class": a
    /// transport that ACCEPTS and blackholes completes connects for ever, so
    /// letting the swap stamp refute the silence would hand the censor's own
    /// cheapest move the power to end the state it exists to raise — the exact
    /// hole the module header rules out for the window.
    ///
    /// `Other` IS in, which [`Self::CONFIRMABLE`] is not: it carries real
    /// production gRPC over the private path (the ephemeral-detect poll, the
    /// sync-server probe), and a full round trip on it five seconds ago refutes
    /// "nothing for a minute" however wedged the sync endpoint is. Holding that
    /// evidence and publishing the sentence anyway is the contract floor's own
    /// forbidden shape ("a wedged server while a SIBLING class confirms is NOT
    /// reported") on the sibling the floor did not enumerate.
    fn carriage_is_an_rpc(self) -> bool {
        !self.connect_is_the_only_evidence()
    }

    /// Whether this class's SILENCE also ends when the sync circuit carries.
    ///
    /// True for [`Self::Other`] only. The class split narrowed every class's
    /// evidence base from the union to its own, which is right for `Broadcast`
    /// (finding (ii): a healthy sync circuit must not starve a send's switch)
    /// and costs `Other` something for nothing: it is short-lived foreground
    /// work that the starvation argument does not reach, and its own stamp
    /// moves only on the rare poll or probe. Inheriting sync's evidence means a
    /// probe does not leave a private path that sync is demonstrably riding.
    /// Its FAILING run stays its own.
    fn inherits_sync_evidence(self) -> bool {
        matches!(self, Self::Other)
    }

    /// An EXHAUSTIVE match, not `self as usize`, and the difference is the whole
    /// point: the discriminant cast compiled fine for a fourth variant and
    /// panicked at `windows[3]` on the first dial of that class — a latent
    /// index-out-of-bounds on a network path, reachable from FFI, while
    /// [`Self::COUNT`]'s doc claimed the compiler would have caught it. Now it
    /// does.
    fn slot(self) -> usize {
        match self {
            Self::Sync => 0,
            Self::Broadcast => 1,
            Self::Swap => 2,
            Self::Other => 3,
        }
    }
}

/// Which ARM served a connection — `PolicyDialer`'s verdict at dial time, and
/// the first field of the `wallet.dial` line. It travels WITH the connection to
/// whoever reports evidence about it (see the module header): the posture never
/// guesses it from the class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DialArm {
    /// The host's transport (a `PolicyDialer`'s primary).
    Private,
    /// The SDK's own `DirectTcpDialer`: `TorPolicy::Off`, or the `Preferred`
    /// fallback after the maintainer's minute.
    Clearnet,
}

/// One class's patience window.
#[derive(Debug, Default)]
struct ClassWindow {
    /// Milliseconds after [`TorPosture::created`] at the last CONFIRMED private
    /// RPC on this class; `0` means the private path has never carried an RPC
    /// for this class in this wallet's life, in which case the window is
    /// measured from `created` — so a wallet that starts on a censored network
    /// still insists for the maintainer's minute before it goes direct (ADR-0552
    /// decision 1).
    ///
    /// A success landing in the FIRST millisecond stores `0` and so reads as
    /// "never": harmless, because the window then runs from `created`, which is
    /// that same instant — the two readings coincide rather than disagree.
    last_private_rpc_ms: AtomicU64,
    /// Milliseconds after [`TorPosture::created`] at the START of the current
    /// run of consecutive dial failures on this class; `0` means this class is
    /// not currently failing.
    ///
    /// THE SECOND HALF OF THE WINDOW, and the repair to stage 1b's HIGH. A clock
    /// that measures only SILENCE cannot tell a broken private path from a
    /// wallet that had nothing to send: `Broadcast`'s silence stamp is written
    /// only by a completed `send_transaction`, so between payments it reads the
    /// whole session age, and the first send after a minute of uptime would have
    /// switched to clearnet on its first transient failure — zero seconds of
    /// insisting, on the one path where per-txid circuit isolation exists to
    /// prevent exactly that correlation.
    ///
    /// Set by the FIRST failure of a run and left alone by the rest of it
    /// (compare-exchange from `0`), because the window must measure from where
    /// the trouble STARTED. A plain store, or a `fetch_max`, would restart the
    /// clock on every failure and the switch would never arrive — the exact
    /// mirror of [`Self::last_private_rpc_ms`], which takes the LATEST because
    /// it measures from where the evidence ENDED. The two opposite choices are
    /// why each has its own named test.
    ///
    /// Cleared by a confirmed private RPC: evidence that the path carries ends
    /// the run. Note a fall-back does NOT clear it — a clearnet dial is not the
    /// private path recovering.
    ///
    /// (Why a first-failure clock is available NOW when the module header once
    /// ruled it out: the header's reason was that a caller whose RPC budget is
    /// shorter than the dial bound cancels before the dialer observes a failure
    /// at all. `DIAL_TIMEOUT_SECS < GRPC_UNARY_TIMEOUT_SECS` is a compile-time
    /// relation since an earlier revision, so the dialer always observes its own outcome. The
    /// reason died in the chunk that wrote it.)
    first_failure_ms: AtomicU64,
    /// The LATEST failure of the current run, in the same one-based
    /// milliseconds; meaningful only while [`Self::first_failure_ms`] is set.
    ///
    /// It exists for ONE reader, [`TorPosture::kept_failing_past_the_window`],
    /// and that reader exists because an ACCEPT became switch-eligible. A dial
    /// that FAILS is its own fresh evidence: by the time the window is read it
    /// has just been recorded. A dial that is ACCEPTED is evidence of nothing,
    /// so the question "has this path kept failing?" can no longer be answered
    /// by the clock alone — one failure followed by ten idle minutes (an app
    /// backgrounded mid-RPC, the commonest way a run is left open) reads as a
    /// ten-minute run, and a healthy private path would be abandoned on the
    /// first dial after resume, by a wallet that has seen no failure since.
    ///
    /// SINCE THE S1 `truth` FOLD that one reader has two callers, and the
    /// second is why the field is load-bearing for what a USER is told:
    /// [`TorPosture::is_unanswered`] reads it too. The state asked the same
    /// question the accept arm asks — has this path kept failing? — and
    /// answered it from the clock alone, so the backgrounded-mid-RPC shape
    /// above published "a ready path has carried nothing for the minute" about
    /// a path the wallet had not touched since one transient failure. One
    /// predicate now answers it for both.
    last_failure_ms: AtomicU64,
    /// Whether the MOST RECENT failure of the current run was a DIAL that
    /// failed ([`TorPosture::note_private_dial_failure`]) rather than an RPC
    /// that failed over a connection the path had accepted; meaningless while
    /// [`Self::first_failure_ms`] is `0`.
    ///
    /// The one bit that lets the fail-closed arm see evidence only the posture
    /// holds (ADR-0554 rule 1: the fail-closed word is a claim about the DIAL).
    /// `SyncStatus` carries the same fact, but only the SYNC class writes it,
    /// so a `Required` wallet whose BROADCAST dial is refused before the sync
    /// loop starts had no way to say `Unavailable` — see
    /// [`TorPosture::path_refused`].
    ///
    /// **THE RUN'S MOST RECENT failure, not its first (review, MAJOR).**
    /// This was written once, by the thread that won the run-starting
    /// compare-exchange, and never updated — so a refusal the path had SINCE
    /// superseded by accepting connections went on answering "the path, not the
    /// server" for as long as the run stayed open. The measured shape: a
    /// `Required` wallet's first dial is refused, opening the run with the flag
    /// `true`; 20 s later the bridge is back and every dial is ACCEPTED, but
    /// the exit blackholes, so each RPC fails over an accepted connection and
    /// leaves the flag alone. At the minute the wallet told the user "not
    /// connected — turn the private path off" about a path its own dialer had
    /// just connected over.
    ///
    /// One layer up, `grpc.rs`'s `transport_stall` resolves the IDENTICAL claim
    /// against the connection's LAST dial, and its doc says why: "an accept
    /// from a minute ago cannot make a refused redial read as the server's
    /// fault." Keyed to the run's FIRST failure, this was that same sentence
    /// pointing the other way.
    ///
    /// Every failing thread writes it now, so a run no longer has a single
    /// writer. Two failures racing leave whichever landed last, and both are
    /// equally recent evidence — the loser's reading is older, not wrong. The
    /// orderings still fail SOFT toward the value that claims less: the start
    /// writes the stamp and THEN this flag (a reader in between sees a run not
    /// attributed to a dial — `Unanswered`, not `Unavailable`), and the clear
    /// clears this flag and THEN the stamp (the same direction). A wrong
    /// reading in either window lasts nanoseconds, and every edge that opens
    /// one also wakes the publisher, which re-derives.
    last_failure_was_a_refused_dial: AtomicBool,
}

/// The wallet-level Tor posture, created once per wallet identity (`Inner`) and
/// shared into every dialer the wallet builds.
pub(crate) struct TorPosture {
    /// The WALLET-LEVEL fell-back latch (the INC-2D GATE, inc-2d-3-a) — ONE
    /// atomic every dialer the wallet builds writes through: the long-lived
    /// sync client AND each fresh per-txid BROADCAST client. `tor_state()`
    /// reads it, so a `Preferred` BROADCAST that degrades to clearnet surfaces
    /// as `FellBack` even before the first sync pass dials — it can never read
    /// a stale `Active`. Sticky (`false`→`true`, never reset): once ANY circuit
    /// has leaked over clearnet, the honest state is `FellBack`; only
    /// `Preferred` writes it (`Off`/`Required` never fall back). `Relaxed` — a
    /// payload-free flag with no happens-before to publish.
    ///
    /// Deliberately NOT per class: the latch answers "has this wallet ever gone
    /// clearnet", which is one fact about the wallet however many circuits it
    /// has. Only the WINDOW is per class.
    /// (NB `set_tor_policy` (G3, not yet wired) must RESET this AND every
    /// window below at the policy-epoch boundary, so a leak from a prior policy
    /// era cannot mask a fresh `Active`. Since stage S1 `truth` the windows are
    /// fed under `Required` too — its plan shares this posture (Q2) — so the
    /// reset is load-bearing for `TorState::Unanswered` in every policy, not
    /// only for the `Preferred` switch; and it must wake the state publisher
    /// (`wake`) so a host sees the epoch's fresh `Active`. What a reset must
    /// NOT touch: the dial counters, which live beside this object
    /// (`Inner.dial_counters`) precisely so they can run "since the wallet
    /// opened" across an epoch.)
    fell_back: AtomicBool,
    /// The origin of every patience clock. `tokio::time::Instant` (not
    /// `std::time::Instant`) for two reasons: `dialer.rs` already measures on
    /// it, and `tokio::time::pause()`/`advance()` make every patience test
    /// deterministic with no wall-clock sleep.
    created: Instant,
    /// One window per [`PathClass`], indexed by its discriminant.
    windows: [ClassWindow; PathClass::COUNT],
    /// THE WAKE (stage S1 `truth`, contract §3.2 "the WAKER"): a generation
    /// counter bumped at every edge that can move the derived `TorState` —
    /// the latch, the 0 → non-zero edge of a failing run, the failure that
    /// first carries a run's observed span past the window (the edge the state
    /// actually turns true on, and no timer is waiting for it), the clear a
    /// confirmed RPC makes, and a private success while a confirmable run is
    /// open (it moves the silence stamp the union deadline is measured from).
    /// The wallet's state publisher subscribes ([`Self::changes`]) and
    /// re-derives on each bump; between bumps it sleeps until
    /// [`Self::next_unanswered_deadline`]. Nothing here wakes anybody
    /// otherwise: every stamp is a lock-free CAS that notifies nobody, and a
    /// host that rode existing wakes would learn of the minute up to
    /// `SYNC_BACKOFF_MAX_SECS` late — and for `Broadcast`, never.
    ///
    /// `watch` rather than `Notify`: a permit-less edge is never lost to a
    /// subscriber that is between polls, and a second subscriber (a test)
    /// costs nothing. Created with its receiver dropped — `send_modify` is
    /// fine with zero receivers, and every subscriber comes later.
    wake: watch::Sender<u64>,
    /// TEST-ONLY: forces [`Self::may_switch_to_direct`] to read `true` for
    /// EVERY class, for tests that build a REAL wallet and dial a REAL loopback
    /// and therefore cannot use a paused clock (see
    /// [`Self::with_the_minute_already_spent`] for why). The field does not
    /// EXIST outside `cfg(test)`, so production has no way to set it and no
    /// branch to take.
    #[cfg(test)]
    minute_forced_spent: AtomicBool,
}

impl TorPosture {
    pub(crate) fn new() -> Self {
        Self {
            fell_back: AtomicBool::new(false),
            created: Instant::now(),
            windows: std::array::from_fn(|_| ClassWindow::default()),
            wake: watch::channel(0).0,
            #[cfg(test)]
            minute_forced_spent: AtomicBool::new(false),
        }
    }

    /// Subscribe to the posture's wakes — see the `wake` field. The value is
    /// a generation with no meaning of its own; a subscriber re-derives
    /// whatever it derives on every `changed()`.
    pub(crate) fn changes(&self) -> watch::Receiver<u64> {
        self.wake.subscribe()
    }

    fn wake(&self) {
        self.wake
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }

    /// TEST-ONLY: read every class's window as spent from now on. The sibling
    /// of [`Self::with_the_minute_already_spent`] — and BOTH exist because
    /// `created` has no interior mutability: a posture already built inside a
    /// live `Wallet` cannot be back-dated, only force-flagged, which is what
    /// the four wallet-level tests need.
    ///
    /// WHAT IT FORCES, exactly: [`Self::may_switch_to_direct`] and nothing
    /// else. No stamp moves, so nothing derived from the stamps changes — in
    /// particular NOT [`Self::kept_failing_past_the_window`] and NOT
    /// [`Self::is_unanswered`] (the state a host reads is derived from the
    /// stamps, never from this flag; a test of that state drives a paused
    /// clock) — and no evidence can un-force it: a confirmed RPC still clears
    /// the run underneath and the predicate still reads `true`. So under this
    /// flag a primary that FAILS reachably, or is declared failed, switches on
    /// the spot; a primary that ACCEPTS does not, because the accept arm also
    /// needs failures the flag never recorded. The four wallet tests drive a
    /// primary that fails; they prove the latch is shared across circuits, and
    /// say nothing about timing or about the accepted-and-silent shape — a
    /// paused clock does that.
    #[cfg(test)]
    pub(crate) fn spend_the_minute_for_test(&self) {
        self.minute_forced_spent.store(true, Ordering::Relaxed);
    }

    /// TEST-ONLY: a posture whose patience windows have ALREADY been spent, so
    /// a test can drive the switch without advancing a clock.
    ///
    /// It exists because the two clock styles do not mix: a test that needs a
    /// REAL loopback listener cannot run under `start_paused`, since tokio's
    /// auto-advance expires the dial bound while the accepting task is still
    /// idle, and the fallback dial then times out instead of connecting. Those
    /// tests take a back-dated posture; the tests that pin the TIMING itself
    /// advance a paused clock. A test holding a posture it did not construct
    /// takes [`Self::spend_the_minute_for_test`] instead — see there.
    /// BOTH conjuncts are spent, not only the silence one. Back-dating `created`
    /// alone stopped being enough at phase 2: a class that is silent but not
    /// FAILING may not switch, which is the whole repair. So every class also
    /// starts with a failing run that began at the same back-dated instant —
    /// i.e. this is a posture whose private path has been silent and failing for
    /// just over the maintainer's minute, which is what its name has always
    /// claimed.
    ///
    /// WHAT IT FORCES, exactly: the STAMPS, for every class including swap, and
    /// not the predicate — a run that began just over a minute ago and whose
    /// latest failure is now. Unlike the flag above it is real state and
    /// evidence ends it (one confirmed private RPC, or a swap connect, and that
    /// class insists again). Until then the first switch-eligible primary
    /// outcome on any class switches, an ACCEPT included.
    #[cfg(test)]
    pub(crate) fn with_the_minute_already_spent() -> Self {
        Self {
            fell_back: AtomicBool::new(false),
            created: Instant::now() - std::time::Duration::from_secs(TOR_PATIENCE_SECS + 1),
            windows: std::array::from_fn(|_| ClassWindow {
                last_private_rpc_ms: AtomicU64::new(0),
                // 1 ms after the back-dated creation: a run that has been going
                // for the whole window. `0` would read as "not failing".
                first_failure_ms: AtomicU64::new(1),
                // …and that was still failing a moment ago: its latest failure
                // is NOW, so the run has been SEEN to outlast the window.
                last_failure_ms: AtomicU64::new((TOR_PATIENCE_SECS + 1) * 1_000 + 1),
                // A run of RPCs that carried nothing, NOT a refused dial: this
                // fixture is the accepted-and-silent path, and a fixture that
                // claimed a refusal would read `Unavailable` where its tests
                // expect the either/or value.
                last_failure_was_a_refused_dial: AtomicBool::new(false),
            }),
            wake: watch::channel(0).0,
            minute_forced_spent: AtomicBool::new(false),
        }
    }

    fn window(&self, class: PathClass) -> &ClassWindow {
        &self.windows[class.slot()]
    }

    /// Whether this wallet has EVER degraded to clearnet under `Preferred`.
    pub(crate) fn fell_back(&self) -> bool {
        self.fell_back.load(Ordering::Relaxed)
    }

    /// Latch the degradation. Called at the ONE site that dials the clearnet
    /// fallback, never anywhere else. Wakes the state publisher: this is the
    /// edge FR-34's announcement keys on.
    pub(crate) fn latch_fell_back(&self) {
        self.fell_back.store(true, Ordering::Relaxed);
        self.wake();
    }

    /// The PRIVATE primary just served a dial on this class.
    ///
    /// For every class but one this does NOT move the window — see the module
    /// header. A connect proves only that something accepted, which is what a
    /// blackholing transport also does.
    pub(crate) fn note_private_connect(&self, class: PathClass) {
        // The one class with no RPC that can report back — see
        // `connect_is_the_only_evidence` for why, and for what it costs.
        if class.connect_is_the_only_evidence() {
            self.note_private_success(class, DialArm::Private);
        }
    }

    /// An RPC COMPLETED over a connection of this class that `served_by`
    /// served. Over the private arm this is the clock RESET of ADR-0552
    /// decision 4 — the wallet returns to insisting as soon as the private path
    /// demonstrably works again. Over the clearnet fallback it says nothing
    /// about the private path and is ignored.
    ///
    /// The arm is an ARGUMENT, never a flag read here: it is a fact about the
    /// connection the RPC rode, and only that connection's owner knows it (the
    /// module header says what one flag per class got wrong).
    ///
    /// `fetch_max` keeps the stored value monotone: two RPCs completing
    /// concurrently would otherwise let the later one store the earlier
    /// timestamp and shorten the window.
    pub(crate) fn note_private_success(&self, class: PathClass, served_by: DialArm) {
        if served_by != DialArm::Private {
            return;
        }
        let window = self.window(class);
        let elapsed = self.created.elapsed().as_millis().min(u64::MAX as u128) as u64;
        window
            .last_private_rpc_ms
            .fetch_max(elapsed, Ordering::Relaxed);
        // Evidence that the path carries ENDS the failing run. A later failure
        // starts a fresh one. The dial attribution goes FIRST, so the only
        // window a concurrent reader can see is the soft one — see
        // [`ClassWindow::last_failure_was_a_refused_dial`].
        window
            .last_failure_was_a_refused_dial
            .store(false, Ordering::Relaxed);
        let ended_a_run = window.first_failure_ms.swap(0, Ordering::Relaxed) != 0;
        // The CLEAR is a wake (it may end `Unanswered`, and it cancels the
        // deadline). So is a success while any confirmable run is open: the
        // union silence the deadline measures from just moved, and if that
        // run had reached the minute this success is what ends the state.
        // A success with no run open anywhere changes nothing a host sees.
        if ended_a_run || self.confirmable_run_open() {
            self.wake();
        }
    }

    /// The private path FAILED to carry on this class, over a connection it had
    /// ACCEPTED — an RPC that failed over the private arm
    /// ([`Self::note_rpc_failure`]). Starts the failure clock if no run is in
    /// progress; leaves an existing run's start alone.
    ///
    /// A dial that FAILED goes through [`Self::note_private_dial_failure`]
    /// instead: same clock, one more fact. This entry is the reading that
    /// claims less, and it is the one every test that means "the path carried
    /// nothing" should take.
    ///
    /// **BOTH sources are required, and a dial-only clock would be a defect.**
    /// The blackhole shape that finding (i) exists to catch is a transport that
    /// ACCEPTS and then carries nothing: its dials SUCCEED and only its RPCs
    /// hang. A failure clock fed by dials alone would never start there, the
    /// second conjunct would never be satisfied, and the wallet would insist
    /// forever — reintroducing the exact hole this window was built to close.
    /// So `net/grpc.rs` reports RPC failures here too, through the witness, and
    /// only when that connection is the private one (a clearnet RPC failing says
    /// nothing about the private path).
    ///
    /// `compare_exchange` rather than a store, and the distinction is the
    /// mechanism: the window measures from where the trouble STARTED, so only
    /// the first failure of a run may write. See [`ClassWindow::first_failure_ms`].
    ///
    /// Stored ONE-BASED (`elapsed + 1`) so that `0` can mean "not failing"
    /// without costing precision. Clamping to `1` instead would under-count a
    /// run that begins in the first millisecond, and the boundary tests assert
    /// on exact milliseconds — a window that is one millisecond short at
    /// t=60 s is a test that cannot say what it means.
    pub(crate) fn note_private_failure(&self, class: PathClass) {
        self.note_failure(class, false);
    }

    /// The private path's DIAL failed on this class — refused, unreachable,
    /// bounded out at the dial, or a transport its own host declared failed.
    ///
    /// [`Self::note_private_failure`] with the one extra fact a dial carries
    /// and an RPC over an accepted connection does not: the path would not take
    /// the connection at all. That is evidence the fail-closed arm may act on
    /// (ADR-0554 rule 1), and it is recorded here because the DIALER is the
    /// only caller that knows it — see [`Self::path_refused`] for what reads it
    /// and for the two conjuncts that stop one stale refusal from claiming the
    /// path is down.
    ///
    /// ONE PRODUCTION CALLER, `PolicyDialer::dial_attributed`'s failed-dial arm.
    /// Every other failure — the witness's, and every test that means "the path
    /// carried nothing" — goes through the plain entry, which is the reading
    /// that claims less.
    pub(crate) fn note_private_dial_failure(&self, class: PathClass) {
        self.note_failure(class, true);
    }

    fn note_failure(&self, class: PathClass, the_dial_failed: bool) {
        let elapsed = self.created.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let stamp = elapsed.saturating_add(1);
        let window = self.window(class);
        // The ORIGIN of the run this failure belongs to, and whether this call
        // is what opened it. Two separate facts, deliberately: a second failure
        // in the same millisecond as the first carries the same stamp, so
        // deriving "did I start it" by comparing the two would call every such
        // failure an edge — and under a paused clock that is every failure.
        let started_a_run = match window.first_failure_ms.compare_exchange(
            0,
            stamp,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => true,
            Err(_already_running) => false,
        };
        let run_began_at = window.first_failure_ms.load(Ordering::Relaxed);
        // EVERY failure writes it, not only the one that opened the run, and
        // after the stamp — see the field's doc. Keyed to the run's first
        // failure, a refusal the path had since superseded by accepting
        // connections kept answering "the path, not the server" for the life of
        // the run; `grpc.rs::transport_stall` resolves the same claim against
        // the LAST dial one layer up.
        window
            .last_failure_was_a_refused_dial
            .store(the_dial_failed, Ordering::Relaxed);
        // The LATEST, for the mirror-image reason: see `last_failure_ms`.
        let previous_last = window.last_failure_ms.fetch_max(stamp, Ordering::Relaxed);
        // TWO edges wake the publisher, and the second is the one the predicate
        // moved under. The 0 → non-zero edge brings a deadline into existence
        // and is the whole of what a refused dial needs to reach the state. The
        // other is the failure that first carries this run's OBSERVED span past
        // the window — because `is_unanswered` turns true on a FAILURE SEEN
        // past the minute, not on the clock reaching it, so that failure is a
        // state transition and no timer is waiting for it. Every other failure
        // of a run moves no deadline and no state.
        let window_ms = TOR_PATIENCE_SECS * 1_000;
        let crossed_the_window = previous_last.saturating_sub(run_began_at) < window_ms
            && stamp.saturating_sub(run_began_at) >= window_ms;
        if (started_a_run || crossed_the_window) && class.counts_toward_unanswered() {
            self.wake();
        }
    }

    /// Whether any confirmable class currently has a failing run open — the
    /// precondition for `Unanswered` ever being reached, and so for a success
    /// being able to change what a host sees.
    fn confirmable_run_open(&self) -> bool {
        PathClass::CONFIRMABLE
            .iter()
            .any(|class| self.window(*class).first_failure_ms.load(Ordering::Relaxed) != 0)
    }

    /// How long the private path has carried NOTHING, across every class whose
    /// success stamp means an RPC completed: since the latest such RPC, or
    /// since creation if the path has never carried one. The UNION, not one
    /// class's stamp: "nothing has come back over the private connection" is a
    /// fact about the PATH, and a class confirming an RPC ten seconds ago
    /// refutes it whatever another class's run says (contract §3.2: a wedged
    /// server while a sibling class confirms is NOT reported).
    ///
    /// The union is WIDER than [`PathClass::CONFIRMABLE`] and the asymmetry is
    /// the point — see [`PathClass::carriage_is_an_rpc`]. Refuting takes every
    /// class that can carry; triggering takes only the two whose silence is a
    /// silent stall. A class whose RPCs cannot raise the state must still be
    /// able to end it, or the SDK publishes a sentence its own evidence
    /// contradicts; `Swap` is out of both, because its stamp is a bare connect
    /// and a blackhole completes those for ever.
    fn carried_nothing_for_ms(&self) -> u64 {
        let now = self.created.elapsed().as_millis().min(u64::MAX as u128) as u64;
        now.saturating_sub(self.latest_carried_rpc_ms())
    }

    fn latest_carried_rpc_ms(&self) -> u64 {
        PathClass::ALL
            .iter()
            .filter(|class| class.carriage_is_an_rpc())
            .map(|class| {
                self.window(*class)
                    .last_private_rpc_ms
                    .load(Ordering::Relaxed)
            })
            .max()
            .unwrap_or(0)
    }

    /// The derivation's input for `TorState::Unanswered` (stage S1 `truth`,
    /// contract §3.2a): whether the private path is ready-and-silent — the
    /// wallet has been TRYING and NOTHING has come back for the maintainer's
    /// minute. Two conjuncts, like [`Self::may_switch_to_direct`]'s, and with
    /// the same reason each: silence alone is what an idle wallet banks,
    /// failing alone is what a flaky-but-carrying path shows.
    ///
    /// - SILENT: no class that can carry has carried an RPC over the private
    ///   arm for the window ([`Self::carried_nothing_for_ms`], the union).
    /// - FAILING: at least one confirmable class has been SEEN to go on failing
    ///   past the window ([`Self::kept_failing_past_the_window`]) — the class
    ///   that "moved it".
    ///
    /// **THE SECOND CONJUNCT IS AN OBSERVATION, NOT A CLOCK READ, and that is
    /// this predicate's own repair.** It used to ask `failing_for_ms >=
    /// window`, which is `now − first_failure` and is bounded by nothing the
    /// wallet did since: one transient failure on the last RPC before the app
    /// was backgrounded, five idle minutes in which no dial was attempted, and
    /// the predicate read `true` — a WARN and a sentence about a path the
    /// wallet had not touched, published before the session's first RPC. The
    /// doc above it claimed the wallet "has been TRYING"; nothing checked it.
    /// Reusing the accept arm's predicate fixes the meaning and buys the
    /// switch and the state ONE definition of "kept failing" instead of two —
    /// [`Self::kept_failing_past_the_window`]'s doc is the argument, and it was
    /// already written for this exact shape.
    ///
    /// It is also what keeps the predicate MONOTONE, which both its consumers
    /// encode: `last_failure − first_failure` cannot shrink while a run is
    /// open and does not move with the clock, so once true only EVIDENCE — a
    /// confirmed RPC, which clears the run and wakes — can end it. A recency
    /// phrasing ("a failure seen within the last window") would flip the
    /// predicate false again a window later with no new evidence, while
    /// [`Self::next_unanswered_deadline`] had returned `None` and the waker was
    /// asleep on it.
    ///
    /// NOT [`Self::may_switch_to_direct`], deliberately: that predicate
    /// carries the test-only force flag, which must never move the state a
    /// host reads (§3.1's fixture trap), and it is asked per class by the
    /// dialer, while this is asked once per derivation about the path. It is
    /// blind to the policy: `Required` reads it too, which is the whole point —
    /// a fail-closed wallet has no switch to make the silence visible.
    pub(crate) fn is_unanswered(&self) -> bool {
        self.carried_nothing_for_ms() >= TOR_PATIENCE_SECS * 1_000
            && PathClass::CONFIRMABLE
                .iter()
                .any(|class| self.kept_failing_past_the_window(*class))
    }

    /// Whether the private path REFUSED to take a connection and nothing has
    /// come back over it since — the fail-closed arm's posture-sourced input
    /// (ADR-0554 rules 1 and 3: a refused dial's EVIDENCE outranks a timeout's
    /// non-evidence).
    ///
    /// It is [`Self::is_unanswered`] REFINED, never a second trigger: the same
    /// two conjuncts, with the failing class additionally required to have most
    /// recently failed on a DIAL rather than on an RPC over a connection the
    /// path had accepted. So it is true only where the state would otherwise
    /// read `Unanswered`, and it answers the one question that value refuses to
    /// guess at — the path or the server? — in the single case where the SDK
    /// does hold the evidence.
    ///
    /// **MOST RECENTLY, not first.** The conjunct read the
    /// failure that OPENED the run, which meant a refusal survived every accept
    /// that followed it: a path that refused once and then took every
    /// connection while the exit blackholed still read `Unavailable` — "turn
    /// the private path off" — at the minute. The bit is now the run's latest
    /// failure ([`ClassWindow::last_failure_was_a_refused_dial`]), which is the
    /// rule `grpc.rs::transport_stall` already applied one layer up.
    ///
    /// **Why it is not prompt, when a refused dial is fresh evidence the
    /// instant it is recorded.** Both conjuncts are load-bearing against a
    /// shape the sync class never had. A refusal on one class does not clear
    /// when another class carries, so without the union-silence conjunct one
    /// refused broadcast dial would hold `Unavailable` for hours over a path
    /// sync is demonstrably riding; and dropping the observed-failure conjunct
    /// instead would let a single stale refusal claim it after an idle gap,
    /// which is the very defect the predicate above repairs. The prompt
    /// reading stays where it already was and is untouched: the SYNC class
    /// reaches `Unavailable` through `SyncStatus::Stalled { TorUnavailable }`
    /// at the first refused dial. This closes the gap where NO sync pass is
    /// running — a send before the loop starts — and there it changes the WORD
    /// at the minute, not the clock.
    ///
    /// Blind to the policy, like everything else the derivation reads.
    pub(crate) fn path_refused(&self) -> bool {
        self.carried_nothing_for_ms() >= TOR_PATIENCE_SECS * 1_000
            && PathClass::CONFIRMABLE.iter().any(|class| {
                self.kept_failing_past_the_window(*class)
                    && self
                        .window(*class)
                        .last_failure_was_a_refused_dial
                        .load(Ordering::Relaxed)
            })
    }

    /// When [`Self::is_unanswered`] will next turn true IF NOTHING ELSE
    /// HAPPENS — the instant the state publisher sleeps until, and `None` for
    /// "sleep until woken".
    ///
    /// Only ONE of the predicate's conjuncts advances with the clock: the union
    /// silence. So there is a deadline exactly when the other one is already
    /// satisfied — some confirmable class has been SEEN to fail past the window
    /// — and it is then `latest carried RPC + window`, the instant the silence
    /// crosses. The union stamp is what moves it out when a class carried
    /// something after the run began, which is exactly when the state would
    /// have lied.
    ///
    /// `None` in the other three cases, and each is "no timer can make it
    /// true": it is already true (only evidence, which wakes, can end it); no
    /// confirmable run is open; or a run is open but has not yet been seen to
    /// outlast the window, in which case the only thing that can satisfy that
    /// conjunct is a FAILURE — and that failure wakes
    /// ([`Self::note_private_failure`]'s second edge). A deadline armed there
    /// would fire on a predicate still reading `false` and re-arm at the same
    /// past instant: a busy loop, not an event.
    ///
    /// Derived from the same stamps as the predicate, so the instant it names
    /// is strictly in the future while the predicate is false, and at it the
    /// predicate reads `true` — the publisher re-derives on waking and never
    /// guesses. Re-read after every wake: a success or a fresh failure moves it.
    pub(crate) fn next_unanswered_deadline(&self) -> Option<Instant> {
        if self.is_unanswered() {
            return None;
        }
        if !PathClass::CONFIRMABLE
            .iter()
            .any(|class| self.kept_failing_past_the_window(*class))
        {
            return None;
        }
        let due_ms = self
            .latest_carried_rpc_ms()
            .saturating_add(TOR_PATIENCE_SECS * 1_000);
        Some(self.created + std::time::Duration::from_millis(due_ms))
    }

    /// Whether this class's failing run has been SEEN to outlast the window: a
    /// failure observed a full `TOR_PATIENCE_SECS` or more after the run began.
    ///
    /// What an ACCEPTED private dial must also satisfy before it may be left
    /// for clearnet. [`Self::may_switch_to_direct`] reads the clock, which is
    /// right for a dial that FAILED (the failure is the fresh evidence,
    /// recorded a moment before the read) and is what a host is shown. An
    /// accept carries no evidence of its own, so it may be left only on
    /// evidence already in hand — otherwise one failure and a long idle gap
    /// would walk a wallet off a private path that is working. Where the clock
    /// is spent and this is not, the accepted connection is USED: if the path
    /// is dead its first RPC says so, this turns true, and the redial that
    /// failure forces is the one that leaves. One RPC later, never never.
    ///
    /// THE SECOND CALLER, since the S1 `truth` fold: [`Self::is_unanswered`]
    /// (and through it [`Self::path_refused`]). The state asked the same
    /// question — has this path kept failing? — from the clock alone, and so
    /// told a user a ready path had carried nothing for the minute when the
    /// wallet had not touched it since one transient failure. The argument
    /// above was written for the accept arm and covers the state word for word;
    /// one predicate now answers it for both, which is also why the switch and
    /// the state can no longer drift apart on what "kept failing" means.
    pub(crate) fn kept_failing_past_the_window(&self, class: PathClass) -> bool {
        let window = self.window(class);
        let first = window.first_failure_ms.load(Ordering::Relaxed);
        if first == 0 {
            return false;
        }
        let last = window.last_failure_ms.load(Ordering::Relaxed);
        last.saturating_sub(first) >= TOR_PATIENCE_SECS * 1_000
    }

    /// An RPC FAILED over a connection of this class that `served_by` served:
    /// [`Self::note_private_failure`], but only for the PRIVATE arm.
    ///
    /// The RPC layer's entry point. A dial failure is unconditionally evidence
    /// about the private path — the primary is what failed — but an RPC failure
    /// is only evidence when the connection under it was private. A bad
    /// clearnet link must not spend the window that decides whether traffic
    /// leaves in the clear, whatever a sibling connection of the same class is
    /// riding at that moment.
    pub(crate) fn note_rpc_failure(&self, class: PathClass, served_by: DialArm) {
        if served_by == DialArm::Private {
            self.note_private_failure(class);
        }
    }

    /// How long this class has been failing WITHOUT INTERRUPTION; `0` when it is
    /// not currently failing.
    fn failing_for_ms(&self, class: PathClass) -> u64 {
        let stored = self.window(class).first_failure_ms.load(Ordering::Relaxed);
        if stored == 0 {
            return 0;
        }
        let now = self.created.elapsed().as_millis().min(u64::MAX as u128) as u64;
        now.saturating_sub(stored - 1)
    }

    /// How long the private path has been silent for this class: since its last
    /// confirmed RPC, or since this posture was created if it has never carried
    /// one. `Other` also counts the sync circuit's — see
    /// [`PathClass::inherits_sync_evidence`].
    fn private_silent_for_ms(&self, class: PathClass) -> u64 {
        let now = self.created.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let mut last = self
            .window(class)
            .last_private_rpc_ms
            .load(Ordering::Relaxed);
        if class.inherits_sync_evidence() {
            last = last.max(
                self.window(PathClass::Sync)
                    .last_private_rpc_ms
                    .load(Ordering::Relaxed),
            );
        }
        now.saturating_sub(last)
    }

    /// Whether the maintainer's minute has run out FOR THIS CLASS — i.e. whether a
    /// `Preferred` wallet may now switch this class of circuit to a direct
    /// connection (ADR-0552). `false` means KEEP INSISTING: the caller returns
    /// the failure typed and emits no clearnet packet.
    /// TWO CONJUNCTS, and neither alone is the maintainer's minute.
    ///
    /// SILENCE alone was stage 1b, and it is what a class accumulates simply by
    /// not being used: `Broadcast` is silent between payments, `Swap` between
    /// swaps, every class while the app is backgrounded. A wallet that has been
    /// open an hour and sends for the first time would switch on its first
    /// transient failure — the HIGH this conjunction repairs.
    ///
    /// FAILURE alone would be worse: a single dial that fails, succeeds, and
    /// fails again would accumulate nothing, and a path that is merely REFUSING
    /// (never attempted, never failed) would never switch at all.
    ///
    /// Together they say what the ruling says — the private path has been given
    /// a minute in which it both carried nothing AND was continuously failing.
    /// A slow-but-working link fails nothing. An idle wallet fails nothing. A
    /// censored one does both.
    ///
    /// ONE CONSUMER: `PolicyDialer::dial_attributed`, once per dial, after the
    /// primary has answered — whatever it answered. It was read on a FAILED
    /// dial only until phase 2, and the blackhole never fails a dial; the gRPC
    /// client's part is to RETIRE a private connection that failed an RPC, so
    /// that a dial happens at all (`net/grpc.rs`). A second reader of this
    /// predicate is the defect that repair removed, not a way to extend it.
    pub(crate) fn may_switch_to_direct(&self, class: PathClass) -> bool {
        #[cfg(test)]
        if self.minute_forced_spent.load(Ordering::Relaxed) {
            return true;
        }
        let window_ms = TOR_PATIENCE_SECS * 1_000;
        self.private_silent_for_ms(class) >= window_ms && self.failing_for_ms(class) >= window_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A connect marks the class live; the helper keeps the two-step shape out
    /// of the assertions below where the SUBJECT of the test is the clock.
    fn confirmed_success(posture: &TorPosture, class: PathClass) {
        posture.note_private_connect(class);
        posture.note_private_success(class, DialArm::Private);
    }

    /// The BLACKHOLE, which is what the window is really for: the transport
    /// keeps ACCEPTING (so no dial fails) and carries nothing, so every RPC
    /// over it fails. One call is one such attempt.
    ///
    /// It goes through `note_rpc_failure`, the same entry `grpc.rs` uses, rather
    /// than the dialer's unconditional one — because a dial-fed clock alone
    /// would never start on this shape, which is the hole finding (i) exists to
    /// close.
    fn blackholed_attempt(posture: &TorPosture, class: PathClass) {
        posture.note_private_connect(class);
        posture.note_rpc_failure(class, DialArm::Private);
    }

    /// The clock starts at the posture's creation, so a wallet whose private
    /// path has NEVER worked still gets the whole minute before it may switch
    /// (ADR-0552 decision 1's second half).
    #[tokio::test(start_paused = true)]
    async fn a_posture_whose_private_path_never_worked_waits_the_whole_minute() {
        let posture = TorPosture::new();
        // Failing from the start — the wallet is TRYING and getting nowhere,
        // which is what "the private path has never worked" means. Without this
        // the class is merely idle, and an idle class never switches (phase 2).
        posture.note_private_failure(PathClass::Sync);
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "fresh: must insist"
        );

        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "one second short of the minute: still insisting"
        );

        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "the minute has run out: switching is now allowed"
        );
    }

    /// A confirmed success RESETS the window, so the wallet insists for another
    /// minute (ADR-0552 decision 4).
    #[tokio::test(start_paused = true)]
    async fn a_private_success_restarts_the_minute() {
        let posture = TorPosture::new();
        posture.note_private_failure(PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        confirmed_success(&posture, PathClass::Sync);

        // The success cleared the failing run as well as moving the silence
        // stamp, so the path has to start failing again for the window to run
        // at all — which is the point: a path that recovered is not failing.
        posture.note_private_failure(PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "the window restarted at the success, so this is still inside it"
        );

        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "a full minute after the last success: switching is allowed again"
        );
    }

    /// A confirmed success must advance the SILENCE stamp — asserted on the
    /// stamp itself, not through `may_switch_to_direct`.
    ///
    /// **Why this test exists, and it is the phase-2 design biting back.** The
    /// switch predicate is a CONJUNCTION, so a test that drives it end to end
    /// cannot tell which half did the work: freeze the stamp and the failure
    /// clock still gates the assertion; stop clearing the failing run and the
    /// silence clock still gates it. Both mutations SURVIVED
    /// `a_private_success_restarts_the_minute`, which is the test that was
    /// supposed to pin exactly this. A conjunction masks its own operands, so
    /// each operand needs a test that can see only it.
    #[tokio::test(start_paused = true)]
    async fn a_confirmed_success_advances_the_silence_stamp() {
        let posture = TorPosture::new();
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        assert_eq!(
            posture.private_silent_for_ms(PathClass::Sync),
            30_000,
            "nothing has carried, so silence runs from creation"
        );

        confirmed_success(&posture, PathClass::Sync);
        assert_eq!(
            posture.private_silent_for_ms(PathClass::Sync),
            0,
            "a confirmed RPC moves the stamp to NOW — a frozen stamp would leave the \
             wallet measuring silence from creation for ever"
        );
    }

    /// A confirmed success must CLEAR the failing run — the other operand, for
    /// the reason given above. Without this, a wallet whose private path
    /// recovers keeps counting the failure run that preceded the recovery, and
    /// the switch arrives on a path that is working.
    #[tokio::test(start_paused = true)]
    async fn a_confirmed_success_clears_the_failing_run() {
        let posture = TorPosture::new();
        posture.note_private_failure(PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        assert_eq!(
            posture.failing_for_ms(PathClass::Sync),
            30_000,
            "the run has been going since the first failure"
        );

        confirmed_success(&posture, PathClass::Sync);
        assert_eq!(
            posture.failing_for_ms(PathClass::Sync),
            0,
            "evidence that the path carries ENDS the run; a run that outlives its own \
             recovery would switch a working wallet to clearnet"
        );
    }

    /// STAGE 1b, finding (i): a transport that ACCEPTS and then never answers
    /// must still reach the switch at the minute. The connect alone is not
    /// evidence — if it were, this posture would insist forever and the
    /// maintainer's ruling would be unimplemented for the commonest censorship
    /// shape there is.
    #[tokio::test(start_paused = true)]
    async fn a_connect_that_never_carries_an_rpc_still_switches_at_the_minute() {
        let posture = TorPosture::new();
        // The blackhole: it accepts, over and over, and answers nothing. 29 × 2 s
        // keeps every assertion strictly INSIDE the minute; the 30th advance
        // below is the one that reaches it.
        for _ in 0..29 {
            blackholed_attempt(&posture, PathClass::Sync);
            tokio::time::advance(std::time::Duration::from_secs(2)).await;
            assert!(
                !posture.may_switch_to_direct(PathClass::Sync),
                "inside the minute the wallet insists, however many times it connected"
            );
        }
        blackholed_attempt(&posture, PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(2)).await;
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "60 s of connects that carried no RPC IS the private path not working — \
             a bare connect must never restart the window"
        );
    }

    /// STAGE 1b, finding (i)'s attribution half: once a connection has fallen
    /// back it is a CLEARNET one, and an RPC over it says nothing about the
    /// private path. Without the arm check the wallet would restart its private
    /// window on clearnet traffic and insist again on a path it has no evidence
    /// for.
    #[tokio::test(start_paused = true)]
    async fn an_rpc_over_the_clearnet_fallback_never_restarts_the_private_window() {
        let posture = TorPosture::new();
        posture.note_private_failure(PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS)).await;
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "the minute is up"
        );

        // The fallback serves the dial, then RPCs succeed over it.
        for _ in 0..10 {
            posture.note_private_success(PathClass::Sync, DialArm::Clearnet);
        }
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "clearnet RPCs are not evidence about the private path: the window stays spent"
        );

        // …and the private path working again DOES restart it.
        confirmed_success(&posture, PathClass::Sync);
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "a confirmed PRIVATE success restarts the window (decision 4)"
        );
    }

    /// PHASE 2's reason for existing: a healthy wallet must not go clearnet on
    /// its FIRST send failure.
    ///
    /// Stage 1b gave each class its own window and moved it only on a confirmed
    /// RPC. `Broadcast`'s only writer is a completed `send_transaction`, so
    /// between payments its silence stamp reads the whole session age — and a
    /// wallet that had synced happily over Tor for ten minutes would switch on
    /// the first transient circuit-build failure of its first send. Zero
    /// seconds of insisting, on the one path where per-txid isolation exists to
    /// stop exactly that correlation, and strictly worse than the pre-stage-1b
    /// tree where any private success kept the window alive.
    ///
    /// The failure conjunct is what makes the maintainer's minute mean a minute of
    /// NOT WORKING rather than a minute of not being used.
    #[tokio::test(start_paused = true)]
    async fn a_healthy_wallet_does_not_go_clearnet_on_its_first_send_failure() {
        let posture = TorPosture::new();

        // Ten minutes of healthy sync over the private path.
        for _ in 0..300 {
            confirmed_success(&posture, PathClass::Sync);
            tokio::time::advance(std::time::Duration::from_secs(2)).await;
        }

        // The broadcast class has carried nothing in all that time, so its
        // SILENCE alone is long spent — this is the stage 1b defect's premise.
        assert!(
            posture.private_silent_for_ms(PathClass::Broadcast) >= TOR_PATIENCE_SECS * 1_000,
            "the premise: an unused class banks silence simply by existing"
        );

        // The user taps Send and the fresh per-txid circuit fails once.
        posture.note_private_failure(PathClass::Broadcast);
        assert!(
            !posture.may_switch_to_direct(PathClass::Broadcast),
            "ONE transient failure on a demonstrably working private path must NOT \
             send this payment in the clear — the founder's minute is a minute of \
             failing, not a minute of idleness"
        );

        // It keeps failing for the whole minute — now the switch is earned.
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS)).await;
        assert!(
            posture.may_switch_to_direct(PathClass::Broadcast),
            "a full minute of continuous failure IS the private path not working"
        );
    }

    /// STAGE 1b, finding (ii): each class keeps its own window, so a healthy
    /// sync circuit cannot reset the window a starved broadcast is measuring.
    #[tokio::test(start_paused = true)]
    async fn a_healthy_sync_circuit_does_not_starve_the_broadcast_switch() {
        let posture = TorPosture::new();
        // The starved-send shape: the censor CARRIES the established sync
        // stream and REFUSES every fresh per-txid circuit, so sync confirms
        // throughout while broadcast fails throughout.
        posture.note_private_failure(PathClass::Broadcast);
        for _ in 0..30 {
            confirmed_success(&posture, PathClass::Sync);
            tokio::time::advance(std::time::Duration::from_secs(2)).await;
        }
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "sync is healthy, so sync keeps insisting — correct"
        );
        assert!(
            posture.may_switch_to_direct(PathClass::Broadcast),
            "the broadcast path has carried nothing for a minute and MUST be allowed to \
             switch — a send that never leaves, under a UI reading UpToDate, is the \
             failure this window exists to end"
        );
    }

    /// `PathClass::ALL` really does hold every class (code reviewer
    /// MAJOR). `ALL` and `COUNT` are hand-written literals with no type-level
    /// link to the enum, so nothing in the compiler stops a new variant being
    /// added, given a `slot()` arm, counted in `COUNT`, and left out of `ALL` —
    /// after which every union reader silently skips it and the silence that
    /// class is banking becomes invisible to `carried_nothing_for_ms`.
    ///
    /// The match below is what makes this bite: it is EXHAUSTIVE, so a new
    /// variant cannot compile until it gets an arm here, and the arm then fails
    /// until the variant is in `ALL` and inside `COUNT`. Compile error first,
    /// red test second — which is the guarantee `ALL`'s doc now claims, having
    /// claimed a bare compile error before.
    #[test]
    fn every_path_class_is_in_all() {
        for class in [
            PathClass::Sync,
            PathClass::Broadcast,
            PathClass::Swap,
            PathClass::Other,
        ] {
            // Exhaustiveness lives here: adding a variant reds this match.
            let _: usize = match class {
                PathClass::Sync => 0,
                PathClass::Broadcast => 1,
                PathClass::Swap => 2,
                PathClass::Other => 3,
            };
            assert!(
                PathClass::ALL.contains(&class),
                "{class:?} has a slot but is missing from PathClass::ALL — every union \
                 reader skips it, and a class banking silence becomes invisible"
            );
            assert!(
                class.slot() < PathClass::COUNT,
                "{class:?}'s slot is outside COUNT — it indexes past the window array"
            );
        }
        assert_eq!(
            PathClass::ALL.len(),
            PathClass::COUNT,
            "ALL and COUNT disagree about how many classes there are"
        );
        let mut slots: Vec<usize> = PathClass::ALL.iter().map(|c| c.slot()).collect();
        slots.sort_unstable();
        slots.dedup();
        assert_eq!(
            slots.len(),
            PathClass::COUNT,
            "two classes share a window slot — their failures would pool into one run"
        );
    }

    /// The classifier itself: four families, four slots, and a broadcast key
    /// is never read as the sync one (they share a `wallet-` stem).
    #[test]
    fn each_isolation_class_keeps_its_own_window() {
        assert_eq!(
            PathClass::of(Some(WALLET_SYNC_ISOLATION_KEY)),
            PathClass::Sync
        );
        assert_eq!(
            PathClass::of(Some(&format!("{BROADCAST_ISOLATION_KEY_PREFIX}-0123abcd"))),
            PathClass::Broadcast
        );
        assert_eq!(
            PathClass::of(Some(
                crate::constants::EPHEMERAL_DETECT_ISOLATION_KEY_PREFIX
            )),
            PathClass::Other
        );
        assert_eq!(
            PathClass::of(Some(
                crate::constants::SYNC_SERVER_PROBE_ISOLATION_KEY_PREFIX
            )),
            PathClass::Other
        );
        assert_eq!(PathClass::of(None), PathClass::Other);
        // The swap on-ramp's own keys (`swap/quote`, `swap/tokens`,
        // `swap/id/<token>` — minted in the peer crate, never reachable from
        // here, so spelled out) are their own class since phase 2 §2c.
        assert_eq!(PathClass::of(Some("swap/quote")), PathClass::Swap);
        assert_eq!(PathClass::of(Some("swap/tokens")), PathClass::Swap);
        assert_eq!(
            PathClass::of(Some("swap/id/0123456789abcdef0123456789abcdef")),
            PathClass::Swap
        );
        // Distinct slots — the property the per-class windows rest on.
        let slots = [
            PathClass::Sync.slot(),
            PathClass::Broadcast.slot(),
            PathClass::Swap.slot(),
            PathClass::Other.slot(),
        ];
        assert_eq!(
            slots.iter().collect::<std::collections::HashSet<_>>().len(),
            PathClass::COUNT,
            "every class indexes its own window"
        );
    }

    /// STAGE 1b's ONE deliberate weakening, pinned here so it cannot spread to
    /// the classes that must not have it.
    ///
    /// `Swap` — the on-ramp's HTTP dials, from a peer crate that cannot reach
    /// this posture — accepts a bare connect as evidence, because nothing on
    /// that path can confirm an RPC. Without it this chunk would have made a
    /// swap dial failure reach clearnet where it previously would not: a
    /// privacy regression on the money path, introduced while fixing the sync
    /// path. `Sync` and `Broadcast` keep the strict rule, which is the whole of
    /// finding (i).
    #[tokio::test(start_paused = true)]
    async fn the_swap_class_accepts_a_connect_because_no_rpc_can_confirm_it() {
        let posture = TorPosture::new();
        // Every class failing from the start, so the ONLY thing that can
        // distinguish them below is the evidence rule under test.
        for class in [PathClass::Swap, PathClass::Sync, PathClass::Broadcast] {
            posture.note_private_failure(class);
        }
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        for class in [PathClass::Swap, PathClass::Sync, PathClass::Broadcast] {
            posture.note_private_connect(class);
        }
        tokio::time::advance(std::time::Duration::from_secs(1)).await;

        assert!(
            !posture.may_switch_to_direct(PathClass::Swap),
            "a swap's connect IS its only evidence: the window restarted, so a swap dial \
             failure does not reach clearnet — the behaviour before this chunk"
        );
        assert!(
            posture.may_switch_to_direct(PathClass::Sync),
            "sync keeps the strict rule: a connect that carried no RPC restarts nothing"
        );
        assert!(
            posture.may_switch_to_direct(PathClass::Broadcast),
            "and so does the money path — this is finding (i), and the weakening above \
             must never reach it"
        );
    }

    /// The latch is sticky and independent of the clock: a success does not
    /// un-say a leak that already happened.
    #[tokio::test(start_paused = true)]
    async fn the_latch_is_sticky_across_a_later_private_success() {
        let posture = TorPosture::new();
        assert!(!posture.fell_back());
        posture.latch_fell_back();
        confirmed_success(&posture, PathClass::Sync);
        assert!(
            posture.fell_back(),
            "once a circuit has leaked, the honest state stays FellBack"
        );
    }

    /// STAGE 1b's LOW: the window's SCOPE, stated as a test rather than left to
    /// the reader. Per session — a fresh posture insists from zero, whatever a
    /// previous one had spent — and measured on a clock that does not advance
    /// while the device sleeps. Both err towards insisting, which is the
    /// private direction; this test is what makes changing either a decision.
    #[tokio::test(start_paused = true)]
    async fn the_patience_window_is_per_session_and_counts_awake_time() {
        let spent = TorPosture::new();
        spent.note_private_failure(PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS)).await;
        assert!(
            spent.may_switch_to_direct(PathClass::Sync),
            "this session's minute is spent"
        );

        // A relaunch (create/open/rescan) builds a NEW posture: the spent
        // minute does not carry, and the wallet insists again from zero.
        let relaunched = TorPosture::new();
        relaunched.note_private_failure(PathClass::Sync);
        assert!(
            !relaunched.may_switch_to_direct(PathClass::Sync),
            "a fresh session insists from zero — the window is per session"
        );

        // AWAKE time, and the emphasis matters: the clock stops only while the
        // DEVICE is suspended, not while the app is merely backgrounded. A user
        // who switches apps for two minutes on an awake phone spends the whole
        // window. (An earlier version of this comment said "a phone asleep in a
        // pocket spends none of its minute" and left the backgrounded case to
        // be inferred, which is the commoner one by far.)
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        assert!(
            !relaunched.may_switch_to_direct(PathClass::Sync),
            "still one second short: nothing but elapsed awake time moves this"
        );
        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        assert!(relaunched.may_switch_to_direct(PathClass::Sync));
    }

    /// the project's Rust rules: a thread-safe type carrying atomics
    /// gets a concurrent test (N threads, barrier start, assert no corruption).
    ///
    /// Two phases, and the SECOND is the one that bites — the first version of
    /// this test had only the storm and a monotone-read check, and a mutant
    /// that froze the stamp at zero passed it, because a frozen stamp is still
    /// monotone and a fresh posture is inside its window either way. So:
    /// phase 1 is the convention's no-corruption check under 8 writers, and
    /// phase 2 pins the RESET on a posture whose window has already expired,
    /// where a stamp that does not advance is immediately visible.
    ///
    /// What phase 1 does NOT prove, stated rather than implied: `fetch_max`'s
    /// monotonicity under a genuine race is not deterministically observable
    /// from outside, so the reader's assertion can only fail by luck. The
    /// ordering property it protects — an older reading must never SHORTEN the
    /// window, which would switch to clearnet before the maintainer's minute — is
    /// guarded deterministically by `a_private_success_restarts_the_minute`
    /// and by phase 2 here.
    #[test]
    fn concurrent_private_successes_corrupt_nothing_and_still_reset_the_window() {
        use std::sync::{Arc, Barrier};

        const WRITERS: usize = 8;
        const PER_WRITER: usize = 500;

        // Phase 1 — the storm.
        let posture = Arc::new(TorPosture::new());
        posture.note_private_connect(PathClass::Sync);
        let barrier = Arc::new(Barrier::new(WRITERS + 1));
        let writers: Vec<_> = (0..WRITERS)
            .map(|_| {
                let posture = Arc::clone(&posture);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..PER_WRITER {
                        posture.note_private_success(PathClass::Sync, DialArm::Private);
                    }
                })
            })
            .collect();

        barrier.wait();
        let mut highest = 0_u64;
        for _ in 0..5_000 {
            // A child module of `tor_posture`, so the raw stamp is readable
            // here rather than inferred through a predicate.
            let seen = posture
                .window(PathClass::Sync)
                .last_private_rpc_ms
                .load(Ordering::Relaxed);
            assert!(
                seen >= highest,
                "the success stamp went BACKWARDS ({highest} then {seen}) — an older \
                 reading must never shorten the window"
            );
            highest = seen;
        }
        for w in writers {
            w.join().expect("writer thread");
        }
        assert!(
            !posture.may_switch_to_direct(PathClass::Sync),
            "4000 successes inside a fresh window leave the wallet insisting"
        );

        // Phase 2 — the reset, where a stamp that never advances is visible.
        let expired = TorPosture::with_the_minute_already_spent();
        assert!(
            expired.may_switch_to_direct(PathClass::Sync),
            "the back-dated posture starts past its window"
        );
        confirmed_success(&expired, PathClass::Sync);
        assert!(
            !expired.may_switch_to_direct(PathClass::Sync),
            "a success RESTARTS the window even when it had already expired — a stamp \
             that does not advance leaves the wallet switching to clearnet forever"
        );
    }

    // ── stage S1 `truth`: the not-carrying input (§3.2a `is_unanswered`) ────

    /// §3.2a: the value is true iff a CONFIRMABLE class — `Sync` or
    /// `Broadcast` — has been both silent and failing for the maintainer's
    /// minute: the blackhole shape, on the class it holds. Pinned at the
    /// boundary, per class, and cleared by the confirmed RPC that ends the
    /// run (the same evidence that restarts the switch's minute).
    #[tokio::test(start_paused = true)]
    async fn a_confirmable_class_silent_and_failing_for_the_minute_reads_unanswered() {
        for class in [PathClass::Sync, PathClass::Broadcast] {
            let posture = TorPosture::new();
            assert!(
                !posture.is_unanswered(),
                "{class:?}: a fresh posture has nothing to report"
            );
            blackholed_attempt(&posture, class);
            tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
            blackholed_attempt(&posture, class);
            assert!(
                !posture.is_unanswered(),
                "{class:?}: one second short of the minute — still inside it"
            );
            tokio::time::advance(std::time::Duration::from_secs(1)).await;
            blackholed_attempt(&posture, class);
            assert!(
                posture.is_unanswered(),
                "{class:?}: a minute of accepts that carried no RPC, failing throughout — \
                 nothing has come back over the private path, and the state must say so"
            );
            confirmed_success(&posture, class);
            assert!(
                !posture.is_unanswered(),
                "{class:?}: a confirmed private RPC IS something coming back — the value \
                 clears with the run"
            );
        }
    }

    /// THE RUN THE WALLET ABANDONED — the fold's first finding, and the one
    /// shape no test drove. One transient failure on the last RPC before the
    /// app is backgrounded, then idle: `Instant` keeps running on a
    /// backgrounded device (the module header says so), so the OLD failing
    /// conjunct — `now − first_failure` — passed the minute on time alone and
    /// the wallet published "a ready path has carried nothing for the minute",
    /// with its WARN, about a path it had not touched since. The wallet was
    /// never trying, which is exactly what the predicate's own doc claimed it
    /// checked.
    ///
    /// The boundary is a TABLE, because the repair is a threshold and a
    /// threshold has two sides: a run whose latest failure is a millisecond
    /// SHORT of the window is not it, and one a millisecond PAST it is —
    /// measured on the run, never on the clock, which here has run ten times
    /// the window either way.
    #[tokio::test(start_paused = true)]
    async fn an_abandoned_run_is_not_unanswered_however_long_the_clock_then_runs() {
        let window_ms = TOR_PATIENCE_SECS * 1_000;
        // (the span the wallet was SEEN failing for, whether it reads)
        for (seen_failing_ms, reads) in [(window_ms - 1, false), (window_ms + 1, true)] {
            let posture = TorPosture::new();
            blackholed_attempt(&posture, PathClass::Sync);
            tokio::time::advance(std::time::Duration::from_millis(seen_failing_ms)).await;
            blackholed_attempt(&posture, PathClass::Sync);
            // …and then the app goes into the background for ten minutes: no
            // dial is attempted, no evidence arrives, and the clock runs.
            tokio::time::advance(std::time::Duration::from_secs(10 * TOR_PATIENCE_SECS)).await;
            assert_eq!(
                posture.is_unanswered(),
                reads,
                "seen failing for {seen_failing_ms} ms and then idle for ten minutes: the \
                 value is the OBSERVED span of the run, not the clock since it opened — \
                 ten idle minutes are not a wallet trying"
            );
        }

        // The same fact stated where it bites: ONE failure, abandoned. This is
        // the resumed-app reading — `snapshot()` on the first frame of a new
        // session, before the session's first RPC.
        let abandoned = TorPosture::new();
        blackholed_attempt(&abandoned, PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(5 * TOR_PATIENCE_SECS)).await;
        assert!(
            !abandoned.is_unanswered(),
            "one transient failure and five idle minutes: a path nobody touched is not a \
             path that carried nothing while the wallet tried"
        );
        assert!(
            abandoned.next_unanswered_deadline().is_none(),
            "and no timer is waiting to say otherwise: the only thing that can satisfy the \
             failing conjunct is a FAILURE, and a failure wakes"
        );
        // Resume, and the first RPC fails: NOW the run has been seen to
        // outlast the window, and the state turns true on that failure — the
        // edge, not a deadline.
        let mut wakes = abandoned.changes();
        wakes.borrow_and_update();
        blackholed_attempt(&abandoned, PathClass::Sync);
        assert!(
            abandoned.is_unanswered(),
            "the wallet tried again and the path still carried nothing: five minutes of \
             silence and a run seen across it"
        );
        assert!(
            wakes.has_changed().expect("the posture outlives this"),
            "and the failure that turned it woke the publisher — no deadline was armed for \
             this transition, so nothing else would have"
        );
    }

    /// The fold's third finding: `Other` carries REAL production gRPC over the
    /// private path — the ephemeral-detect poll, the sync-server probe — so a
    /// round trip on it refutes "nothing has come back over the private
    /// connection", even though its silence can never TRIGGER the value. The
    /// two sets are deliberately different, and `Swap` is in neither: its
    /// stamp is a bare CONNECT, which a blackhole completes for ever.
    #[tokio::test(start_paused = true)]
    async fn a_probes_round_trip_refutes_the_silence_it_can_never_trigger() {
        // A wedged sync endpoint, failing throughout, while the host's probe
        // keeps completing over the same private path.
        let carrying = TorPosture::new();
        blackholed_attempt(&carrying, PathClass::Sync);
        for _ in 0..12 {
            tokio::time::advance(std::time::Duration::from_secs(10)).await;
            blackholed_attempt(&carrying, PathClass::Sync);
            confirmed_success(&carrying, PathClass::Other);
        }
        assert!(
            carrying.kept_failing_past_the_window(PathClass::Sync),
            "fixture: the sync run HAS been seen to outlast the window"
        );
        assert!(
            !carrying.is_unanswered(),
            "a `GetLightdInfo` round trip came back over this path five seconds ago: the \
             path carries and the SERVER is the problem — the SDK holds the evidence, and \
             publishing 'nothing has come back for a minute' would contradict it"
        );

        // A SWAP connect is not that evidence, and must not be: accepting is
        // what the blackhole does.
        let accepting = TorPosture::new();
        blackholed_attempt(&accepting, PathClass::Sync);
        for _ in 0..12 {
            tokio::time::advance(std::time::Duration::from_secs(10)).await;
            blackholed_attempt(&accepting, PathClass::Sync);
            accepting.note_private_connect(PathClass::Swap);
        }
        assert!(
            accepting.is_unanswered(),
            "a transport that keeps ACCEPTING swap dials and carries no RPC is the \
             blackhole itself — its own accepts may not end the state they raise"
        );
    }

    /// The fold's fourth finding at the posture: which runs carry the
    /// fail-closed word. A run OPENED by a dial the path refused reads
    /// `path_refused`; one opened by an RPC over a connection the path
    /// ACCEPTED does not, however long it fails — that is the either/or case,
    /// and the SDK has nothing that separates path from server there.
    ///
    /// It is `is_unanswered` REFINED, so both conjuncts are pinned here too:
    /// a refusal nothing has outlived, over a path nothing has carried.
    #[tokio::test(start_paused = true)]
    async fn only_a_run_a_refused_dial_opened_reads_path_refused() {
        let spent = std::time::Duration::from_secs(TOR_PATIENCE_SECS);

        // (a) the accepted-and-silent path: the either/or value, no more.
        let accepted = TorPosture::new();
        blackholed_attempt(&accepted, PathClass::Sync);
        tokio::time::advance(spent).await;
        blackholed_attempt(&accepted, PathClass::Sync);
        assert!(accepted.is_unanswered(), "fixture: the minute is up");
        assert!(
            !accepted.path_refused(),
            "the path took the connection and then carried nothing: the SDK cannot say \
             whether that is the path or the server, and must not"
        );

        // (b) a REFUSED dial — the one thing the evidence can separate. The
        // broadcast class, because that is the gap: no `SyncStatus` speaks for
        // it, so before this the reading was the either/or value.
        let refused = TorPosture::new();
        refused.note_private_dial_failure(PathClass::Broadcast);
        tokio::time::advance(spent).await;
        refused.note_private_dial_failure(PathClass::Broadcast);
        assert!(refused.is_unanswered(), "fixture: the minute is up");
        assert!(
            refused.path_refused(),
            "the path would not take the connection at all: that is evidence the path is \
             down, and it outranks a sentence that offers the user two causes"
        );

        // …and it is not a latch. One confirmed RPC ends the run and the
        // attribution with it.
        confirmed_success(&refused, PathClass::Broadcast);
        assert!(
            !refused.path_refused() && !refused.is_unanswered(),
            "a confirmed private RPC ends the run, and a refusal it outlived is not a \
             claim about now"
        );

        // (c) the two conjuncts, each alone. A refusal a SIBLING class has
        // carried past is not the path being down — without the union-silence
        // conjunct one refused broadcast dial would hold `Unavailable` for
        // hours over a path sync is demonstrably riding.
        let carrying = TorPosture::new();
        carrying.note_private_dial_failure(PathClass::Broadcast);
        tokio::time::advance(spent).await;
        carrying.note_private_dial_failure(PathClass::Broadcast);
        confirmed_success(&carrying, PathClass::Sync);
        assert!(
            !carrying.path_refused(),
            "the path is carrying sync RPCs: a refused broadcast circuit is not a path \
             that is down"
        );
        // And a refusal the wallet never went back to is not one either — the
        // same abandoned-run rule, on the same clock.
        let stale = TorPosture::new();
        stale.note_private_dial_failure(PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(10 * TOR_PATIENCE_SECS)).await;
        assert!(
            !stale.path_refused(),
            "one refusal ten minutes ago, nothing attempted since: `Unavailable` is a \
             claim about now"
        );

        // (d) A REFUSAL THE PATH HAS SINCE SUPERSEDED (review, MAJOR).
        // The bridge is down, so the first dial is refused and opens the run.
        // Then it comes back: every dial is ACCEPTED, and the exit blackholes,
        // so each RPC fails over a connection the path took. The run never
        // closes — no RPC is ever confirmed — so with the bit keyed to the
        // failure that OPENED the run, the refusal outlived every accept that
        // followed it and the wallet said "not connected — turn the private
        // path off" about a path its own dialer had just connected over.
        //
        // The state is still `Unanswered`: nothing here tells the SDK whether
        // the blackhole is the path or the server. That is precisely the
        // either/or, and the refusal is no longer evidence against it.
        let superseded = TorPosture::new();
        superseded.note_private_dial_failure(PathClass::Sync);
        assert!(
            superseded
                .window(PathClass::Sync)
                .last_failure_was_a_refused_dial
                .load(Ordering::Relaxed),
            "fixture: the refused dial opened the run"
        );
        for _ in 0..12 {
            tokio::time::advance(std::time::Duration::from_secs(10)).await;
            blackholed_attempt(&superseded, PathClass::Sync);
        }
        assert!(
            superseded.is_unanswered(),
            "fixture: a minute of failing attempts over an accepting path"
        );
        assert!(
            !superseded.path_refused(),
            "the path refused once and has ACCEPTED every connection since: the wallet \
             may not still be calling that the path's fault, and must not tell the user \
             to turn their private path off over it"
        );

        // The mirror, which the same one-writer bug got wrong in the other
        // direction: a run OPENED by a blackholed RPC and then refused at the
        // dial reads the refusal, because that is the freshest evidence.
        let then_refused = TorPosture::new();
        blackholed_attempt(&then_refused, PathClass::Sync);
        tokio::time::advance(spent).await;
        then_refused.note_private_dial_failure(PathClass::Sync);
        assert!(
            then_refused.path_refused(),
            "the last thing the path did was refuse a connection: the evidence is the \
             dial's, whatever opened the run"
        );
    }

    /// Silence alone is not the value and a failing run alone is not either
    /// — the switch's two conjuncts, read for the same reason: an idle class
    /// banks silence by existing, and a path that was confirmed inside the
    /// minute is carrying, whatever it did before.
    #[tokio::test(start_paused = true)]
    async fn unanswered_needs_both_silence_and_a_failing_run() {
        let idle = TorPosture::new();
        tokio::time::advance(std::time::Duration::from_secs(10 * TOR_PATIENCE_SECS)).await;
        assert!(
            !idle.is_unanswered(),
            "ten silent minutes with nothing tried: idle is not unanswered"
        );

        let carrying = TorPosture::new();
        blackholed_attempt(&carrying, PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(50)).await;
        confirmed_success(&carrying, PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        blackholed_attempt(&carrying, PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(29)).await;
        assert!(
            !carrying.is_unanswered(),
            "confirmed a minute ago and failing for 29 s: the private path carried inside \
             the window it is measured on"
        );
    }

    /// The SUBJECT is the two classes whose RPCs can confirm (§3.2). `Swap`
    /// (a bare connect is its only evidence) and `Other` (short-lived
    /// foreground work that inherits sync's evidence) never move it, however
    /// long they fail — and a class wedged beside a SIBLING that confirms is
    /// not the private path not carrying: the sibling's RPCs are coming back
    /// over it, which is the one thing the value denies.
    #[tokio::test(start_paused = true)]
    async fn only_the_confirmable_classes_move_unanswered() {
        let posture = TorPosture::new();
        for _ in 0..12 {
            blackholed_attempt(&posture, PathClass::Other);
            posture.note_private_failure(PathClass::Swap);
            confirmed_success(&posture, PathClass::Sync);
            tokio::time::advance(std::time::Duration::from_secs(10)).await;
        }
        assert!(
            posture.may_switch_to_direct(PathClass::Swap),
            "fixture: the swap class has been silent and failing for two minutes"
        );
        assert!(
            !posture.is_unanswered(),
            "a probe wedged beside a sync circuit that confirms every ten seconds, and a \
             swap class failing for two minutes: neither is the private path not carrying"
        );
    }

    /// §3.1's fixture trap, restated for the new value: the test-only forced
    /// minute short-circuits `may_switch_to_direct` and NOTHING else, so a
    /// value derived from the switch predicate would turn true here on no
    /// evidence at all — while the STAMPED spent posture, whose evidence is
    /// real, must read it.
    #[tokio::test(start_paused = true)]
    async fn the_forced_minute_does_not_move_unanswered_but_real_stamps_do() {
        let forced = TorPosture::new();
        forced.spend_the_minute_for_test();
        assert!(
            forced.may_switch_to_direct(PathClass::Sync)
                && forced.may_switch_to_direct(PathClass::Broadcast),
            "fixture: the switch predicate reads forced"
        );
        assert!(
            !forced.is_unanswered(),
            "no evidence was recorded, and the forced flag is not evidence"
        );

        let spent = TorPosture::with_the_minute_already_spent();
        assert!(
            spent.is_unanswered(),
            "stamps that say silent and failing for just over the minute ARE the evidence"
        );
    }

    /// Evidence rides its connection's arm, for the value as for the window:
    /// an RPC over the clearnet fallback neither clears the value (nothing
    /// came back over the PRIVATE path) nor, failing, starts it (a bad
    /// clearnet link says nothing about the private path).
    #[tokio::test(start_paused = true)]
    async fn a_clearnet_rpc_neither_clears_nor_starts_unanswered() {
        let posture = TorPosture::new();
        blackholed_attempt(&posture, PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS)).await;
        blackholed_attempt(&posture, PathClass::Sync);
        assert!(posture.is_unanswered(), "fixture: the minute is spent");
        for _ in 0..10 {
            posture.note_private_success(PathClass::Sync, DialArm::Clearnet);
        }
        assert!(
            posture.is_unanswered(),
            "ten RPCs over the clearnet fallback: still nothing has come back over the \
             private path"
        );

        let fresh = TorPosture::new();
        for _ in 0..7 {
            fresh.note_rpc_failure(PathClass::Sync, DialArm::Clearnet);
            tokio::time::advance(std::time::Duration::from_secs(10)).await;
        }
        assert!(
            !fresh.is_unanswered(),
            "seventy seconds of clearnet failures started no private run"
        );
    }

    // ── planted after the adjudication (IT-1 +A; ruling §4's survivors) ─────

    /// The union ruling (`docs/adjudication/s1-truth/ruling.md` VERDICT row
    /// 5; §4 M01b/M04): silence is the UNION over the confirmable classes and
    /// failing is per class — and the silence conjunct's THRESHOLD is the
    /// minute, not "a moment". A sync run that has failed for two minutes
    /// reads the value until a BROADCAST RPC comes back over the private
    /// path; from that instant the path is carrying, and it stays so for the
    /// whole minute the sibling's stamp is measured on: at 59 s the sync run
    /// is still failing and the value is still off. (The wedged-sibling rows
    /// read at the instant OF the sibling's success, where even a 1-ms
    /// threshold agrees.)
    #[tokio::test(start_paused = true)]
    async fn a_sibling_class_confirmed_inside_the_minute_refutes_a_run_failing_past_it() {
        let posture = TorPosture::new();
        blackholed_attempt(&posture, PathClass::Sync);
        tokio::time::advance(std::time::Duration::from_secs(2 * TOR_PATIENCE_SECS)).await;
        blackholed_attempt(&posture, PathClass::Sync);
        assert!(
            posture.is_unanswered(),
            "fixture: two minutes of sync failing, nothing back on either class"
        );

        confirmed_success(&posture, PathClass::Broadcast);
        tokio::time::advance(std::time::Duration::from_secs(TOR_PATIENCE_SECS - 1)).await;
        blackholed_attempt(&posture, PathClass::Sync);
        assert!(
            !posture.is_unanswered(),
            "a broadcast RPC came back over the private path 59 s ago while the sync run \
             failed on: the path carried inside the minute it is measured on — silence is \
             the union, and its threshold is the minute"
        );
    }

    /// The CONFIRMABLE set from the VALUE side (§4 M15): a class outside it
    /// failing ALONE — nothing confirmed on any class, the union silence long
    /// spent — never reads the value however long it fails. `Swap` (a bare
    /// connect is its only evidence) and `Other` (short-lived foreground
    /// work) are not the user's silent stall; "any class" would restate FR-36
    /// instead of answering it. (`only_the_confirmable_classes_move_unanswered`
    /// reads them beside a sync circuit that confirms, where the union
    /// silence answers first.)
    #[tokio::test(start_paused = true)]
    async fn a_non_confirmable_class_failing_alone_for_two_minutes_never_reads_unanswered() {
        for class in [PathClass::Swap, PathClass::Other] {
            let posture = TorPosture::new();
            for _ in 0..12 {
                posture.note_private_failure(class);
                tokio::time::advance(std::time::Duration::from_secs(10)).await;
            }
            assert!(
                posture.may_switch_to_direct(class),
                "{class:?}: fixture — silent and failing for two minutes by the switch's \
                 own reading"
            );
            assert!(
                !posture.is_unanswered(),
                "{class:?}: two minutes failing alone, nothing confirmed anywhere — not the \
                 private path not carrying: the value is derived from the classes whose RPCs \
                 can confirm, and this is not one"
            );
        }
    }
}
