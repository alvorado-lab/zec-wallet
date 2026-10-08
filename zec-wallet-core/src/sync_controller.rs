//! §3.2 / §3.2g iv-d-3 — the `SyncController`: the resilient background sync
//! loop over the `SyncEnginePort` port (the Tachyon seam).
//!
//! The controller owns the *policy* around a sync pass; the engine owns one
//! pass. Policy:
//!
//! 1. **Infinite retry, capped backoff.** Sync NEVER gives up while started
//!    (§6.2: upstream shipped retry-CAPPED sync, harvested "I had to manually
//!    restart it" bug reports, and reversed). Only the interval is bounded
//!    (`SYNC_BACKOFF_MAX_SECS`), never the count. **The ladder resets on a CLEAN
//!    pass only** (§4u REW-1): an `Ok` pass that rewound climbs it like a fault
//!    and sleeps `max(POLL_INTERVAL, backoff)`, never faster than a clean poll —
//!    so a patient forking endpoint cannot buy a fresh per-pass rewind budget at
//!    the 1 s rung every 20 s (the REQ-1-R fold review's row 1). Consecutive
//!    rewinding passes are counted, and from `MAX_CONSECUTIVE_REWINDING_PASSES`
//!    on the loop publishes `Stalled { EndpointMisbehaving }` in place of the
//!    pass's terminal status until clean passes have decayed the streak below the
//!    threshold (`run_loop`; one per clean pass, never cleared in one).
//! 2. **No-progress stuck-sync watchdog** (`SYNC_STUCK_WATCHDOG_SECS`).
//!    A pass that reports no progress for the whole window is restarted — the
//!    failure the retry loop alone cannot see (a pass that neither errors nor
//!    advances). Each progress report re-arms it, so a pass that is making
//!    progress (however slowly — a first sync runs for hours) is never
//!    restarted. **The watchdog cancels COOPERATIVELY** (it flips the pass's
//!    `CancelToken`); the pass honors it only where it checks — so the watchdog
//!    recovers a pass wedged BETWEEN batches or stuck reporting no progress, but
//!    NOT one blocked inside an un-timed network read. That makes a hard
//!    contract on the engine (see [`SyncEnginePort::run_pass`]): every per-batch I/O
//!    MUST be timeout-bounded (≤ the watchdog window) and progress reported
//!    intra-batch, so cancel is always promptly reachable. The controller cannot
//!    enforce that — the lightd adapter (d-3-b) must, with its own named test.
//! 3. **Single-writer.** At most one pass runs at a time (`once()` never
//!    overlaps the loop); a second writer would double-scan a suggested range
//!    and contend the db lock.
//! 4. **Cooperative stop + `Drop` teardown.** `stop()` is prompt (≤ one batch,
//!    GIVEN the engine's per-batch I/O timeout above) even mid-sync; `Drop`
//!    guarantees no leaked task.
//! 5. **Live `SyncStatus` stream** (`tokio::watch`, latest-wins): a late
//!    subscriber sees the current state, and a stale event can never beat a
//!    fresher one (§3.3 — streams never die, faults surface as data). A wedged
//!    pass degrades HONESTLY (`Stalled`), never a forever-spinning `Scanning`.
//!
//! **The first sync-engine tracing site (§5.4).** (The dialer already emits
//! `wallet.private_path_fell_back`; this is the first on the sync path, and where the
//! capture-layer GUARD lands.) The pass-level `wallet.sync` span
//! (`{batches, reorgs, outcome, backoff_secs}` — all counts / durations / error
//! codes / outcomes, the §5.4 allowlist) and the `wallet.reorg_rewind {depth}`
//! instability event (gate 5 — a recovery that happens silently is a debugging
//! black hole) live here, on `target: "zec_wallet_core"`. The per-range
//! `wallet.sync {from, to, blocks, outcome}` span — with the absolute heights
//! §5.4 keeps loggable (maintainer decision, S37) — lands inside the engine pass
//! (d-3-b), where the precise range is known. The capture-layer guard
//! (`tracing_spans_carry_no_address_amount_or_memo`) pins the allowlist and is
//! span-aware (it inspects span fields too), ready for that d-3-b span.
//!
//! Crate-internal; wired into `Wallet::sync()` with the real lightd engine
//! adapter + the FRB stream surface in d-3-b. Tests drive a deterministic fake
//! engine over the virtual clock (no real network, no wall-clock waits).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::constants::{
    GRPC_UNARY_TIMEOUT_SECS, MAX_CONSECUTIVE_REWINDING_PASSES, POLL_INTERVAL_SECS,
    RESUBMIT_BROADCAST_BUDGET_SECS, SYNC_BACKOFF_INITIAL_SECS, SYNC_BACKOFF_MAX_SECS,
    SYNC_STUCK_WATCHDOG_SECS,
};
use crate::error::WalletError;
use crate::money::BlockHeight;
use crate::state::{PoolService, PoolServiceReport, StallReason, SyncStatus, UnknownBranchGrace};
use crate::sync::{CancelToken, PoolFetch, SUBTREE_ROOT_POOLS, SyncPass, TipStanding};

/// First backoff after a fault; doubles each retry up to [`SYNC_BACKOFF_MAX_SECS`]
/// (both pinned in `constants.rs` under the gate-7 boundary test).
const INITIAL_BACKOFF: Duration = Duration::from_secs(SYNC_BACKOFF_INITIAL_SECS);

/// One scan-progress sample the engine emits after each batch — the live
/// Spend-before-Sync UX (§1.7 / §2.5 `Scanning`). Heights are §5.4-loggable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SyncProgress {
    /// Chain tip the pass is scanning toward.
    pub(crate) tip: BlockHeight,
    /// The MONOTONIC scanned-equivalent height the UI renders as `Scanning.from`, placed so
    /// `tip − scanned_to == round(span · (1 − percent))` (`wallet::scanned_equiv`) — an honest
    /// blocks-left countdown that AGREES with `percent`. NOT the raw per-range download frontier
    /// (which jumps between Spend-before-Sync priority ranges — §2.5 "never range position", the
    /// "1 %, 289 blocks left" footgun). Falls back to the frontier only pre-provision (no floor).
    pub(crate) scanned_to: BlockHeight,
    /// Scan-progress fraction in `[0, 1]`: scanned/total notes across BOTH the recovery
    /// and scan windows — never range position (§2.5: a non-linear scan order would make
    /// a naive `frontier/tip` regress). The authoritative value (§3.2g iv-d-3-b-iii) is
    /// derived from `WalletRead::get_wallet_summary().progress()`, combining `.scan()`
    /// (recovery-height→tip) with `.recovery()` (birthday→recovery-height) so a deep first
    /// sync — whose work is the RECOVERY window — does not read a frozen 0% (`account::
    /// progress_percent`). Read by `sync_once` after each committed batch and carried
    /// across the download ticks (`account::progress_snapshot`). MONOTONIC absent a reorg;
    /// a reorg rewind lowers it to the honest post-rewind truth. NEVER gate a money
    /// decision on this (§2.5 — UX only).
    pub(crate) percent: f32,
    /// Funds usable before 100% (§1.7 Spend-before-Sync). The authoritative signal
    /// (§3.2g iv-d-3-b-iii) — any account with a positive spendable balance under the
    /// ZIP-315 default confirmations policy (`AccountBalance::spendable_value()`).
    pub(crate) spendable_ready: bool,
    /// The pass has rewound at least once (§2.5 `Scanning.rewound`, #317) —
    /// stamped by `sync_once`'s report wrapper from the moment the first
    /// reorg rewind lands, carried on every later sample of the same pass.
    pub(crate) rewound: bool,
}

/// The engine's progress channel back to the controller. `report` is cheap and
/// non-blocking (two `watch` sends): it (1) pushes the live
/// `SyncStatus::Scanning` to subscribers and (2) feeds the stuck-sync watchdog —
/// each report resets its deadline, so a pass that is *making progress* is never
/// force-restarted, no matter how long it legitimately runs.
pub(crate) struct ProgressSink {
    status_tx: watch::Sender<SyncStatus>,
    tick_tx: watch::Sender<()>,
    /// Latches the moment any progress sample carries `rewound` (review
    /// HIGH). The loop's `Err` arm used to infer "did this pass rewind" from the
    /// STALL REASON, so a pass that performed up to `MAX_TOTAL_REORGS_PER_PASS`
    /// truncates and then died of a transport fault counted as non-rewinding: no
    /// streak, no floor, and a clean pass then stepped the ladder back down — the
    /// ~21 s cycle §4u was written to close, reachable through a different door.
    /// A `SyncPass` is lost on `Err`, but the engine stamps `rewound` on EVERY
    /// sample after its first rewind (the Rewind arm reports one immediately), so
    /// the fact is already crossing this seam and was simply being discarded.
    rewound: Arc<AtomicBool>,
}

impl ProgressSink {
    pub(crate) fn report(&self, p: SyncProgress) {
        // Live status — `watch` keeps only the latest, so a burst coalesces and a
        // slow subscriber can never be flooded (mobile-lens: bounded UI churn).
        if p.rewound {
            self.rewound.store(true, Ordering::Relaxed);
        }
        let _ = self.status_tx.send(SyncStatus::Scanning {
            from: p.scanned_to,
            to: p.tip,
            percent: p.percent,
            spendable_ready: p.spendable_ready,
            rewound: p.rewound,
        });
        // Watchdog kick (the value is irrelevant — `watch::send` always notifies).
        let _ = self.tick_tx.send(());
    }

    /// Re-arm the stuck-sync watchdog WITHOUT changing the published status — liveness for
    /// non-scan pass work that legitimately runs long (the §3.3 memo-enhancement drain: up to
    /// `MAX_ENHANCEMENTS_PER_PASS` unary `GetTransaction`s, each bounded by
    /// `GRPC_UNARY_TIMEOUT_SECS`, which on a slow link can cumulatively exceed
    /// `SYNC_STUCK_WATCHDOG_SECS`). Unlike [`report`](Self::report) it does NOT touch `status_tx`:
    /// the scan's last status stays current (the pass is still in progress until it returns, so
    /// re-publishing `Scanning` would be redundant churn), and only the watchdog tick fires — so a
    /// slow-but-ALIVE backlog re-arms the watchdog instead of tripping a false stuck-restart, while
    /// a genuinely wedged fetch is still caught (each fetch is unary-timeout-bounded, so the gap
    /// between ticks can never exceed one timeout).
    pub(crate) fn keepalive(&self) {
        let _ = self.tick_tx.send(());
    }
}

/// The sync-engine seam (the Tachyon seam, §3.2). One production impl — the
/// lightd engine adapter (d-3-b) wrapping `Wallet::sync_once` over a real gRPC
/// client; tests drive a deterministic fake. The controller owns the loop /
/// backoff / watchdog / status policy; the engine owns ONE pass + its progress.
#[async_trait]
pub(crate) trait SyncEnginePort: Send + Sync {
    /// Run ONE sync pass to the recorded tip, or until `cancel` is honored.
    /// Reports live progress via `sink`. Recoverable faults are typed
    /// `WalletError::Sync { stall }`; the controller maps any other error
    /// conservatively and keeps retrying (§6.2 retry-forever).
    ///
    /// **Cancel + watchdog CONTRACT (load-bearing — the d-3-b adapter MUST honor
    /// it; the controller cannot enforce it).** The stuck-sync watchdog and
    /// `stop()` cancel COOPERATIVELY via `cancel`; if the engine only checks it
    /// between long-running batches, a pass blocked inside one un-timed network
    /// read is neither recoverable by the watchdog nor interruptible by `stop()`
    /// (which would then hang on the join). So a conforming engine MUST:
    ///   1. bound EVERY per-batch network I/O with a timeout (≤ the watchdog
    ///      window `SYNC_STUCK_WATCHDOG_SECS`), and check `cancel.is_cancelled()`
    ///      (or race `cancel.cancelled()`) frequently enough that cancel is
    ///      honored PROMPTLY, not only at batch boundaries;
    ///   2. `sink.report(..)` INTRA-batch (per N blocks), so a slow-but-advancing
    ///      download keeps re-arming the watchdog instead of tripping a false
    ///      stuck-restart;
    ///   3. lose at most the in-flight (re-downloadable) work on cancel — never a
    ///      torn/partial commit.
    /// The bare `Wallet::sync_once` polls `cancel` only between batches, so the
    /// d-3-b adapter wraps it accordingly and ships a named test for a batch that
    /// blocks past the watchdog window.
    async fn run_pass(
        &self,
        cancel: &CancelToken,
        sink: &ProgressSink,
    ) -> Result<SyncPass, WalletError>;

    /// Best-known chain tip — the `UpToDate { tip }` terminal status when a pass
    /// finishes with nothing left to scan (a no-op pass emits no progress event).
    /// Cheap, no network (the engine caches the last tip it recorded).
    fn tip(&self) -> Option<BlockHeight>;

    /// Persist the last-synced stamp for `tip` (§2.5 `WalletState.last_synced`,
    /// #317). The controller calls this on every CLEAN pass whose tip the grade
    /// found at or above what this wallet already holds
    /// (`SyncPass::tip_standing == Some(AtOrAboveBundle)` — T0-1c-R2 M2: a
    /// behind pass, or one that made no claim, is not a reached-tip event and
    /// gets no call), immediately BEFORE publishing the terminal status —
    /// write-then-publish, so a host that `snapshot()`s on seeing `UpToDate`
    /// reads a stamp that matches the tip it was told about (minimizes the
    /// unpaired height/time window in a header).
    /// Display-only, fail-open: the impl logs and swallows its own faults (a
    /// stamp write must NEVER fail or delay a sync outcome); the default is a
    /// no-op so test fakes and any stampless engine need not implement it.
    async fn record_synced(&self, _tip: BlockHeight) {}

    /// Post-pass resubmission hook (§6.3 / inc-2d-3-b-ii-B). The controller calls this AFTER a
    /// CLEAN (non-cancelled) pass — under the single-writer pass guard (so it is SCANNER-IDLE),
    /// with the watchdog torn down and the active `cancel` still live (so `stop()`/`Drop`
    /// interrupt it). The production [`LightdSyncEngine`](crate::wallet) drives the §6.3 outbox
    /// here (reconcile in-flight intents + re-propose Queued + re-broadcast); the default is a
    /// NO-OP so the test fakes + any non-outbox engine need not implement it. It returns nothing:
    /// a resubmission fault is the engine's own to log and swallow — it must never fail sync.
    async fn after_synced(&self, _cancel: &CancelToken) {}

    /// Could this build fully interpret every block the pass just scanned?
    /// (`ironwood-nu63-support.md` §6.4.)
    ///
    /// `false` ⇒ the controller publishes [`SyncStatus::UpToDateLimited`]
    /// instead of `UpToDate`, because reporting a clean "Up to date" over a
    /// range we could not fully read is the same silence the Ironwood outage
    /// was made of, moved into the sync surface.
    ///
    /// Defaults to `true` so the test fakes and any engine without a consensus
    /// verdict need not implement it — the production engine reads the
    /// persisted verdict.
    async fn scan_is_fully_interpretable(&self) -> bool {
        true
    }

    /// Is THIS ENDPOINT withholding which network it is on — and if so, where
    /// does the §6.3 grace stand right now? (GRACE-1, §4p Q-G2 / P-G3.)
    ///
    /// `Some(grace)` ⇒ the controller publishes
    /// [`SyncStatus::UpToDateUnverified`] (running: the countdown; ended: the
    /// reason and the next step) instead of a plain `UpToDate` — an `Unknown`
    /// verdict rendered as "Up to date" was the silence this closes. Read
    /// against the device clock at the pass's END, from the durable stamp, so
    /// the reading survives a relaunch.
    ///
    /// Defaults to `None` so the test fakes and any engine without a consensus
    /// verdict need not implement it — the production engine reads the
    /// persisted verdict through `consensus_stamp::observe`.
    async fn unknown_branch_grace(&self) -> Option<UnknownBranchGrace> {
        None
    }
}
// FELL-BACK (the INC-2D GATE, inc-2d-3-a): the "degraded to clearnet" signal is NO
// LONGER an engine concern. It is a single wallet-level posture (`Inner.tor_posture`:
// the latch AND ADR-0552's patience clock) every dialer writes through,
// read DIRECTLY by `Wallet::tor_state()`. Routing it through the engine would have
// observed only the SYNC circuit, so a `Preferred` broadcast that degraded to clearnet
// could read `Active` (the bug §3.2g/S45 deferred here). One predicate, one source.

/// Shared between the public handle and the spawned loop task.
struct Shared {
    engine: Arc<dyn SyncEnginePort>,
    status_tx: watch::Sender<SyncStatus>,
    /// A retained receiver so the status channel NEVER closes — a `watch::send`
    /// with zero receivers is a silent no-op (the stored value would stay stuck at
    /// the initial `Idle`), so the controller must always hold one even when no
    /// host has subscribed. Never read; it exists only to keep the channel open.
    _status_keepalive: watch::Receiver<SyncStatus>,
    /// At most one pass at a time — the single-writer contract (`once()` and the
    /// background loop never overlap a pass).
    pass_guard: tokio::sync::Mutex<()>,
    /// The in-flight pass's cancel token, so `stop()`/`Drop` interrupt it
    /// PROMPTLY (the pass honors it between batches). `None` ⇒ no pass running.
    active_cancel: Mutex<Option<CancelToken>>,
    /// §4u REW-1 / phase-2 P2-5: the rewinding streak — consecutive LOOP passes
    /// that rewound, the loop's judgement about the server (`run_loop`'s doc).
    /// Lives HERE rather than in a loop local so `once()` (a pull-to-refresh)
    /// READS it: built with a zero streak, a once-verdict published "Up to date"
    /// under a standing "switch servers", and the next loop pass put the stall
    /// back — the flicker `emit_synced`'s own doc calls impossible (phase-1
    /// §4u-run row 3). ONE writer, the loop: decayed on a clean pass, raised on
    /// a rewinding one, and moved by NOTHING else — **a `stop()`/`start()` does
    /// not forgive it** (phase-2 P2-6, maintainer decision 6: the loop used
    /// to zero it at its top, and the host's reconnect kick answers every
    /// `Stalled` — this reason included — with exactly that pair, so a
    /// reachability tick or the user's "Try now" handed the endpoint a fresh
    /// rewind budget the moment the badge reported). The count starts at zero
    /// when the controller is CONSTRUCTED (`SyncController::new`), so an app
    /// relaunch still starts fresh: the streak is the controller's memory, not
    /// the loop's and not the DB's. `once()` neither raises nor decays it — a
    /// pull-to-refresh is the user's action, not evidence about the server's
    /// pattern, and the count is over LOOP passes. `Relaxed` suffices: a single
    /// writer, and a reader that observes the previous value publishes the
    /// previous judgement, which is still the loop's. **The residual, stated
    /// here where the next reader is:** between a `stop()` and the next
    /// `start()` the count stands as the dead loop left it, so a `once()` in
    /// that window (the host backgrounded, then a foreground pull-to-refresh)
    /// publishes a judgement nothing has re-measured; since P2-6 that is the
    /// DESIGN, not a gap — only the next loop's clean passes can decay it.
    /// **And "one writer" has a ±1 window** (the security pass's row 4):
    /// `stop()` takes the task handle before the old loop has exited, so a
    /// `start()` racing it spawns the next loop, whose first raise/decay can
    /// interleave with the departing loop's last. Bounded to one step, in either
    /// direction, once per restart — TRUE because the helpers are atomic
    /// read-modify-writes (a load-then-store could lose a whole step); the
    /// reconnect kick issues exactly that pair. Not a data race (an atomic), and
    /// not worth a generation token for one step (see `run_loop`'s doc and
    /// phase-2 P2-6). **Third residual:** the count both a loop pass and a
    /// `once()` publish is this pass's, but which of the two TERMINAL STATUSES
    /// lands last in the latest-wins `watch` is not ordered (`emit_synced`
    /// awaits `record_synced` after the guard drops) — unreachable through the
    /// one production door, `once_within` (FR-40): it refuses while the loop is
    /// started, and `stop()` joins the loop (its last publish included) before a
    /// bounded pass can begin; stated so it is not discovered twice.
    rewinding_streak: AtomicU32,
    /// The no-progress watchdog's window. `new` — the only constructor a
    /// shipped build has — always sets `Some(SYNC_STUCK_WATCHDOG_SECS)`. `None`
    /// runs every pass with no watchdog, and only the test seam
    /// `SyncController::with_stuck_window` can build it (S6 §3.2).
    stuck_window: Option<Duration>,
}

impl Shared {
    /// The streak as the loop last left it (P2-5) — what a `once()` verdict carries.
    fn streak(&self) -> u32 {
        self.rewinding_streak.load(Ordering::Relaxed)
    }

    /// A clean LOOP pass: decay by one, never below zero. Returns the new count.
    /// An atomic read-modify-write, not load-then-store (the code reviewer's row 3):
    /// across the `stop()`/`start()` window the field doc names, a load-then-store
    /// could overwrite the other loop's step with a stale value and lose it whole;
    /// an RMW loses at most the one step the doc promises. (Until P2-6 the loop
    /// also zeroed the count at its top; the two writer paths are now these two
    /// helpers and nothing else — no arm and no lifecycle event touches the atomic.)
    // `try_update`, not `fetch_update` (deprecated from rustc 1.99): the same
    // call under its new name. It builds on the workspace floor (1.96) and on
    // `wallet-linux-check`'s 1.97.1 — measured on 1.95, 1.96.1 and 1.97.1,
    // correcting an earlier comment that said it only exists from 1.99 (the
    // reason a scoped `allow(deprecated)` sat here before).
    fn decay_streak(&self) -> u32 {
        self.rewinding_streak
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(1))
            })
            .expect("the closure always returns Some")
            .saturating_sub(1)
    }

    /// A rewinding LOOP pass: raise by one, saturating. Returns the new count.
    /// Same RMW discipline as `decay_streak`, for the same reason.
    /// (`try_update`, as in `decay_streak`; the closure always returns `Some`,
    /// so the update never fails.)
    fn raise_streak(&self) -> u32 {
        self.rewinding_streak
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_add(1))
            })
            .expect("the closure always returns Some")
            .saturating_add(1)
    }
}

/// The background sync loop + its controls (spec §3.1 `sync()` →
/// `start()`/`stop()`/`once()`). Created STOPPED; `start()` spawns the loop,
/// `stop()` ends it, and a stopped controller can `start()` again.
pub(crate) struct SyncController {
    shared: Arc<Shared>,
    /// The running loop's stop signal (`true` ⇒ stop; latest-wins). Recreated per
    /// `start()` so restart is clean. `None` ⇒ not started.
    stop_tx: Mutex<Option<watch::Sender<bool>>>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl SyncController {
    pub(crate) fn new(engine: Arc<dyn SyncEnginePort>) -> Self {
        Self::build(engine, Some(Duration::from_secs(SYNC_STUCK_WATCHDOG_SECS)))
    }

    /// `new` with the watchdog window chosen by the caller; `None` runs every
    /// pass with no watchdog. Test-only (S6 §3.2): `Wallet::controller_over`
    /// passes `None` because its rows assert the status a pass publishes, not
    /// the watchdog — whose own rows in this file (and
    /// `wallet::tests::stream_carries_the_watchdog_stalled_then_recovery_without_dying`)
    /// build through `new` on the paused clock. Over the real engine a
    /// real-time 600 s window cancelled passes that report nothing by design
    /// on a loaded nightly (2026-09-28: four `degraded_pool_proof` rows red).
    #[cfg(test)]
    pub(crate) fn with_stuck_window(
        engine: Arc<dyn SyncEnginePort>,
        stuck_window: Option<Duration>,
    ) -> Self {
        Self::build(engine, stuck_window)
    }

    fn build(engine: Arc<dyn SyncEnginePort>, stuck_window: Option<Duration>) -> Self {
        let (status_tx, status_rx) = watch::channel(SyncStatus::Idle);
        Self {
            shared: Arc::new(Shared {
                engine,
                status_tx,
                _status_keepalive: status_rx,
                pass_guard: tokio::sync::Mutex::new(()),
                active_cancel: Mutex::new(None),
                rewinding_streak: AtomicU32::new(0),
                stuck_window,
            }),
            stop_tx: Mutex::new(None),
            task: Mutex::new(None),
        }
    }

    /// Subscribe to live `SyncStatus` (the FRB stream source, d-3-b). The
    /// receiver starts at the current status — `watch` never drops the initial.
    pub(crate) fn subscribe(&self) -> watch::Receiver<SyncStatus> {
        self.shared.status_tx.subscribe()
    }

    /// The current status without subscribing (the §3.1 cold snapshot read).
    pub(crate) fn status(&self) -> SyncStatus {
        self.shared.status_tx.borrow().clone()
    }

    /// Start the background loop (idempotent — a second `start()` while running is
    /// a no-op, never a second loop).
    pub(crate) fn start(&self) {
        // Hold the `task` guard ONLY for the running-check; release it before taking
        // `stop_tx` so the two locks are never nested (start/stop stay lock-order
        // independent — no latent deadlock for a future editor to trip).
        {
            let task = self.task.lock().expect("sync task mutex poisoned");
            if task.as_ref().is_some_and(|t| !t.is_finished()) {
                return;
            }
        }
        let (stop_tx, stop_rx) = watch::channel(false);
        *self.stop_tx.lock().expect("stop mutex poisoned") = Some(stop_tx);
        let shared = Arc::clone(&self.shared);
        // Loop lifecycle is host-driven (pause/resume, rescan, retry) and each
        // fresh loop deliberately restarts the backoff ladder at 1 s — WITHOUT
        // this event a field log shows "backoff reset with no success between",
        // indistinguishable from a controller bug (exactly that false
        // alarm, chased live). Counts-only, §5.4-clean.
        tracing::info!(target: "zec_wallet_core", outcome = "loop_start", "wallet.sync");
        *self.task.lock().expect("sync task mutex poisoned") =
            Some(tokio::spawn(run_loop(shared, stop_rx)));
    }

    /// Signal the loop to exit AND interrupt an in-flight pass, WITHOUT awaiting the
    /// join (the synchronous half of `stop`). Idempotent. `Drop for Wallet` calls
    /// this — a synchronous context that cannot `.await` — so a handle dropped without
    /// `close()` still tears the loop down: the cancel is honored at the pass's next
    /// cooperative checkpoint (the engine races it, so PROMPTLY in every phase), the
    /// pass returns, the transient `Inner` ref releases, and the single-writer lock
    /// frees — rather than the lock wedging for a whole (possibly hours-long) pass.
    pub(crate) fn request_stop(&self) {
        if let Some(tx) = self.stop_tx.lock().expect("stop mutex poisoned").take() {
            let _ = tx.send(true);
            // The twin of `loop_start` (only on a LIVE loop — the take() makes
            // repeat stops silent): brackets the loop's life in field logs so a
            // backoff restart reads as the host's pause/resume, not a bug.
            tracing::info!(target: "zec_wallet_core", outcome = "loop_stop", "wallet.sync");
        }
        if let Some(c) = self
            .shared
            .active_cancel
            .lock()
            .expect("active-cancel mutex poisoned")
            .clone()
        {
            c.cancel();
        }
    }

    /// Stop the background loop and AWAIT clean teardown (idempotent). Signals the
    /// loop to exit AND interrupts an in-flight pass (prompt — the engine races the
    /// cancel), then joins. Durable progress is preserved (the next `start()`
    /// resumes). The chain is the source of truth — a stop loses nothing (§6.3).
    pub(crate) async fn stop(&self) {
        self.request_stop();
        // Take the handle OUT before awaiting — never hold a std mutex across await.
        let handle = self.task.lock().expect("sync task mutex poisoned").take();
        if let Some(h) = handle {
            let _ = h.await;
        }
        // A bounded pass (`once_within`, the host's `syncFor`) runs on its
        // CALLER's task, not `self.task`, and a host can overlap it with this
        // stop. `request_stop` fired its token above; wait until it has unwound
        // and released the pass guard, so this `Idle` is published after its
        // last word, never before. (A pass that took the guard but had not yet
        // armed its token when the stop fired runs on to its own deadline.)
        drop(self.shared.pass_guard.lock().await);
        let _ = self.shared.status_tx.send(SyncStatus::Idle);
    }

    /// Run exactly ONE pass now and return its summary (the §3.1 `once()`), with no
    /// deadline. Serialized against the background loop by the single-writer pass
    /// guard — it WAITS for a running pass — so it never overlaps one. The host's
    /// door is [`Self::once_within`] (FR-40), which refuses instead of waiting and
    /// carries a deadline; this unbounded form is the tests' driver.
    #[cfg(test)]
    pub(crate) async fn once(&self) -> Result<SyncPass, WalletError> {
        let guard = self.shared.pass_guard.lock().await;
        self.run_once(guard, None).await.map(|b| b.pass)
    }

    /// The worst case of the post-pass resubmission (FR-40 §3.5b): one
    /// `RESUBMIT_BROADCAST_BUDGET_SECS` window plus the one TEX group started just
    /// under its deadline — `constants.rs`'s const-asserted shape. A bounded pass
    /// drives the outbox only when at least this much of its budget is left.
    pub(crate) const RESUBMIT_RESERVE: Duration =
        Duration::from_secs(RESUBMIT_BROADCAST_BUDGET_SECS + 2 * GRPC_UNARY_TIMEOUT_SECS);

    /// FR-40 — run ONE pass for at most `budget` and report how far it got (the
    /// host's `syncFor`). The deadline fires the pass's OWN cancel token, the one
    /// `request_stop` fires, so the pass stops exactly the way `stop_sync` stops
    /// it: cooperatively, durable progress kept, nothing half-written. The future
    /// is never dropped under a timeout — that would skip `set_active(None)` and
    /// the outcome publish — it is awaited to its return after the fire.
    ///
    /// **The bound:** the call returns within `budget + GRPC_UNARY_TIMEOUT_SECS`
    /// (an in-flight unary RPC is not preempted; the engine races the token
    /// everywhere else). The post-pass resubmission runs only when the budget
    /// left after the pass covers [`Self::RESUBMIT_RESERVE`]; otherwise it is
    /// skipped and `resubmitted: false` says so — the next `start_sync` pass
    /// carries the obligation. The budget is always the caller's.
    ///
    /// Refused with [`WalletError::SyncRunning`] while the background loop is
    /// started or another pass holds the pass guard — refused, never queued: a
    /// wait for the guard would spend the caller's budget on someone else's
    /// pass. A pass the deadline cut publishes `Idle` (nothing is scanning once
    /// the call returns; a stale `Scanning` would claim otherwise). The rewinding
    /// streak is read, never moved (the `once()` rule, `Shared::rewinding_streak`).
    pub(crate) async fn once_within(&self, budget: Duration) -> Result<BoundedPass, WalletError> {
        let deadline = tokio::time::Instant::now() + budget;
        if self.loop_started() {
            return Err(WalletError::SyncRunning);
        }
        let Ok(guard) = self.shared.pass_guard.try_lock() else {
            return Err(WalletError::SyncRunning);
        };
        self.run_once(guard, Some(deadline)).await
    }

    /// Is the background loop started (spawned and not yet exited)? The same
    /// reading `start()` uses for its idempotence.
    fn loop_started(&self) -> bool {
        self.task
            .lock()
            .expect("sync task mutex poisoned")
            .as_ref()
            .is_some_and(|t| !t.is_finished())
    }

    /// One pass under the held pass guard: the active token, the optional
    /// deadline that fires it, the resubmission gate, the outcome publish.
    async fn run_once(
        &self,
        _guard: tokio::sync::MutexGuard<'_, ()>,
        deadline: Option<tokio::time::Instant>,
    ) -> Result<BoundedPass, WalletError> {
        let cancel = CancelToken::new();
        set_active(&self.shared, Some(cancel.clone()));
        // Resubmission rides the SAME single-writer guard as the pass (scanner idle) — a manual
        // pass drives the outbox too, not just the background loop.
        let rewound = Arc::new(AtomicBool::new(false));
        let work = run_pass_then_resubmit(&self.shared, &cancel, &rewound, deadline);
        tokio::pin!(work);
        let (res, resubmitted) = match deadline {
            None => work.await,
            Some(deadline) => tokio::select! {
                out = &mut work => out,
                () = tokio::time::sleep_until(deadline) => {
                    // Fire the pass's own token, then let the pass return through its
                    // cooperative checkpoints like a `stop_sync` would.
                    cancel.cancel();
                    work.await
                }
            },
        };
        set_active(&self.shared, None);
        match &res {
            Ok(p) if p.cancelled && deadline.is_some() => {
                let _ = self.shared.status_tx.send(SyncStatus::Idle);
            }
            _ => emit_outcome(&self.shared, &res).await,
        }
        res.map(|pass| BoundedPass { pass, resubmitted })
    }
}

/// What a bounded pass hands back: the pass itself, and whether the post-pass
/// resubmission ran to its end (`false` when the reserve did not fit, the pass
/// did not finish, or the token fired while it ran).
#[derive(Clone, Copy, Debug)]
pub(crate) struct BoundedPass {
    pub(crate) pass: SyncPass,
    pub(crate) resubmitted: bool,
}

impl Drop for SyncController {
    fn drop(&mut self) {
        // Guarantee no leaked background task when the handle drops (the controller
        // lives inside the Wallet `Inner`; close/drop must tear it down). Signal
        // stop, interrupt the in-flight pass, and abort the task as a backstop —
        // each `run_blocking` batch section completes atomically, so aborting
        // between awaits is safe (durable progress).
        if let Some(tx) = self.stop_tx.lock().ok().and_then(|mut g| g.take()) {
            let _ = tx.send(true);
        }
        if let Some(c) = self
            .shared
            .active_cancel
            .lock()
            .ok()
            .and_then(|g| g.clone())
        {
            c.cancel();
        }
        if let Some(h) = self.task.lock().ok().and_then(|mut g| g.take()) {
            h.abort();
        }
    }
}

/// The background loop: run a pass, react, sleep, repeat — until stopped.
///
/// **§4u REW-1 — the ladder rule and the streak, as built (with the fold review's
/// row 1 applied).** Two loop-local counters. A CLEAN pass (`Ok` with
/// `reorgs == 0`) steps the ladder DOWN one rung (`prev_backoff`, floored at
/// `INITIAL_BACKOFF`) and DECAYS the streak by one; every other finished pass
/// sleeps and then climbs one (`next_backoff`) — a non-rewinding fault sleeps the
/// rung itself, while a rewinding pass (an `Ok` with `reorgs > 0`, or a
/// `ChainReorg` stall) sleeps `max(POLL_INTERVAL, rung)` so it never polls faster
/// than a clean one (`rewinding_sleep`).
///
/// **Why a decay and not a reset.** The reset was the round's own evasion: a clean
/// pass costs a hostile endpoint nothing — hold the tip still, `next_batch` finds
/// no non-empty range and returns `None`, and the pass ends `Ok` with no batches
/// and no rewinds, which IS clean — so "fork for five passes, freeze for one,
/// repeat" moved neither counter and the badge never changed. Under the decay an
/// endpoint must produce about as many clean passes as rewinding ones to hold the
/// ladder down and the badge off. **The residual, stated rather than smoothed**: a
/// strict 1:1 alternation still nets zero on the streak, so it never reaches the
/// report — what it costs is the rate, which the rewinding floor halves (both
/// halves of the cycle now sleep at least the poll interval, where the stall used
/// to sleep the bare 1 s rung).
///
/// The rewinding streak counts consecutive passes that rewound (`Ok` with
/// `reorgs > 0`, or a `ChainReorg` stall — the cap, the storm bound or the exit
/// belt); a clean pass decays it and NOTHING else moves it down: a
/// cancelled pass and a non-rewind fault leave it standing (neither is evidence
/// about the server's chain, and a server must not be able to dodge the report
/// with a timeout every few passes). From `MAX_CONSECUTIVE_REWINDING_PASSES` on,
/// every rewinding pass publishes `Stalled { EndpointMisbehaving }` in place of its
/// own verdict (`emit_synced`, where the build's and the grace's claims rank
/// ABOVE it since P2-6; the `Err` arm) — it cannot flicker, because a clean
/// pass carries its DECAYED streak into `emit_synced` and so returns the terminal
/// status only once the streak has fallen below the threshold: one clean pass no
/// longer takes the badge down, which is the point of the decay. Where the streak
/// lives — in `Shared`, beside the loop (phase-2 P2-5; it was a loop local, and a
/// `once()` then carried a ZERO streak into `emit_synced`, so a pull-to-refresh
/// under a standing report published "Up to date" and the next loop pass put the
/// stall back — the flicker), not in the aux DB: bought, no schema, no bridge
/// field, and `once()` (a pull-to-refresh) READS it so its verdict is the loop's
/// judgement while it can neither raise nor decay it; lost, an app relaunch
/// forgives it (the count is born zero with the controller), so an endpoint gets
/// a fresh streak budget per LAUNCH — or per SERVER SWITCH, which builds a new
/// controller for the new server (P3-13 D3: a streak is a judgement about ONE
/// server) — not per loop start. **A `stop()`/`start()`
/// does NOT forgive it** (phase-2 P2-6, maintainer decision 6). It did until
/// P2-6 — this function zeroed the count at its top, beside the ladder it still
/// restarts — and the Batch C security pass's row 3 priced that: the host's
/// `WalletReconnectKick` answers ANY `Stalled`, this reason included, with a
/// `stop()` + `start()` (pause/resume and a rescan issue the same pair, and
/// `loop_start` marks each in the log), so a reachability tick (Android fires one
/// per network change; a flapping link fires dozens) or the user's "Try now"
/// handed the endpoint a fresh `MAX_CONSECUTIVE_REWINDING_PASSES` budget the
/// moment the badge reported, and the report never stood on a phone that
/// changed networks. Now only clean LOOP passes decay it: the kick and "Try
/// now" restart the ladder (a fresh 1 s rung is the right answer to "reconnect")
/// and leave the judgement about the server where the loop left it. The count
/// MOVES under the pass guard (inside the `let (pass, …) = { … }` block below,
/// before the guard drops), so a `once()` that takes the guard next reads this
/// pass's count and never the pre-pass one. The `wallet.sync` pass event carries
/// `backoff_secs` = the sleep the loop takes next on EVERY loop-driven pass
/// (`emit_synced`); the stall event as before.
async fn run_loop(shared: Arc<Shared>, mut stop_rx: watch::Receiver<bool>) {
    let mut backoff = INITIAL_BACKOFF;
    // NOT `rewinding_streak.store(0)` here — P2-6: a loop start restarts the ladder
    // and nothing else; the streak is the controller's memory (the doc above).
    // R10 §4.2: has a LOOP pass faulted locally (`is_local_stall`) since the last
    // pass that COMPLETED? Only a completed `Ok` pass clears it — not a network fault
    // or a watchdog cancel between two local faults, which would otherwise hide a
    // corrupt store behind a flaky link forever (the R10 security review). Loop-local
    // on purpose: a restart starts fresh, and `once()` neither reads nor moves it.
    let mut prev_local = false;
    loop {
        if *stop_rx.borrow() {
            break;
        }

        // Did THIS pass rewind? Latched by the progress sink, because an `Err`
        // pass returns no `SyncPass` and the stall reason is not the same
        // question (review HIGH — the ladder's floor was bypassable by
        // rewinding 29 times and then exiting through a transport fault).
        let pass_rewound = Arc::new(AtomicBool::new(false));
        let (pass, rewinding_streak, rewound) = {
            // Single-writer: hold the pass guard across the whole pass.
            let _guard = shared.pass_guard.lock().await;
            if *stop_rx.borrow() {
                break; // stop arrived while we waited for the guard
            }
            let cancel = CancelToken::new();
            set_active(&shared, Some(cancel.clone()));
            // `stop()` sets the flag THEN cancels the active token; re-check here so a
            // stop racing in between `set_active` and the run is honored — otherwise a
            // wedged pass could start un-cancellable and `stop()` would hang on the
            // join. (If stop already cancelled `cancel`, the pass returns cancelled
            // immediately anyway; this just avoids starting it at all.)
            if *stop_rx.borrow() {
                set_active(&shared, None);
                break;
            }
            // The post-pass resubmission (inc-2d-3-b-ii-B) runs HERE — still under the pass guard
            // (single-writer, scanner idle), with the active cancel — before `set_active(None)`
            // releases it. A `stop()`/`Drop` racing in cancels the in-flight broadcast promptly.
            let (res, _) = run_pass_then_resubmit(&shared, &cancel, &pass_rewound, None).await;
            // §4u / P2-5 (the Batch C security pass's row 5): the streak MOVES HERE,
            // while this task still holds the pass guard — so a `once()` that takes
            // the guard next reads the count THIS pass produced, never the pre-pass
            // one (it used to move in the arms below, after the guard dropped, and a
            // pull-to-refresh on the multi-thread runtime could read the old count
            // and publish "Up to date" after this pass published the stall). The
            // arms read `rewinding_streak` and never touch the atomic. Did THIS
            // pass rewind? For an `Ok` pass its own `reorgs`; for a fault the pass's
            // own work (the progress sink's latch), not an inference from how it
            // exited — a pass that truncated and THEN met a transport fault rewound
            // just as much as one the cap stopped, and a `ChainReorg`
            // stall counts on its own since it may end the pass before any progress
            // sample. A cancelled pass and a non-rewind fault leave the count
            // standing (the doc above).
            // ONE match decides both (the code reviewer's row 10: two matches over one
            // scrutinee let a later edit to "rewound" desynchronise the count).
            let (rewound, streak) = match &res {
                Ok(p) if p.cancelled => (false, shared.streak()),
                Ok(p) if p.reorgs == 0 => (false, shared.decay_streak()),
                Ok(_) => (true, shared.raise_streak()),
                Err(e) => {
                    let rewound = stall_for(e) == StallReason::ChainReorg
                        || pass_rewound.load(Ordering::Relaxed);
                    let streak = if rewound {
                        shared.raise_streak()
                    } else {
                        shared.streak()
                    };
                    (rewound, streak)
                }
            };
            set_active(&shared, None);
            (res, streak, rewound)
        };

        match pass {
            // Cancelled — by `stop()`/`Drop` (exit) or the watchdog (restart).
            Ok(p) if p.cancelled => {
                if *stop_rx.borrow() {
                    break;
                }
                // Watchdog fired: a wedged pass, not a fault. Surface a typed Stalled
                // BEFORE restarting (honest degradation §2.5/§3.3 — a persistently
                // wedged endpoint must NOT show a forever-moving "Scanning"; a
                // no-progress wedge reads as the endpoint not delivering).
                let _ = shared.status_tx.send(SyncStatus::Stalled {
                    reason: StallReason::EndpointUnreachable,
                });
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = "watchdog_restart",
                    batches = p.batches,
                    // SCAN-1 (review, item 10): a wedged pass still holds the
                    // sums of the batches it did commit — say where they went too.
                    anchor_ms = p.anchor_ms,
                    dl_ms = p.dl_ms,
                    scan_ms = p.scan_ms,
                    snap_ms = p.snap_ms,
                    chain_outputs = p.chain_outputs,
                    "wallet.sync"
                );
                if p.batches == 0 {
                    // The wedge committed NOTHING — back off before re-running so a
                    // persistently-wedged endpoint can't busy-loop re-downloading the
                    // same range (defense-in-depth — the d-3-b-i derived report cadence
                    // already keeps a slow-but-ALIVE link from tripping the watchdog;
                    // this bounds the residual case + a future timeout-constant change).
                    interruptible_sleep(backoff, &mut stop_rx).await;
                    backoff = next_backoff(backoff);
                }
                // A wedge that DID commit batches restarts promptly (real progress was
                // made; backoff is for "no forward motion at all").
            }
            Ok(p) if p.reorgs == 0 => {
                // A CLEAN pass — the ONE event that DECAYS the streak and steps
                // the ladder down a rung (§4u; the REW-1 fold review's row 1, the
                // maintainer's decision). It used to CLEAR both, and that was free:
                // a clean pass costs a hostile endpoint nothing (hold the tip
                // still — `next_batch` finds no non-empty range, the pass ends
                // `Ok` with no batches and no rewinds, which is clean), so fork
                // for five passes, freeze for one, repeat, and neither counter
                // ever moved. Decay by one instead: an endpoint has to behave
                // about as often as it forks, and what it clawed back on one
                // clean pass is one rung, not the whole ladder. (The decay itself
                // ran under the pass guard above; `rewinding_streak` is its result.)
                prev_local = false; // R10 §4.2: a completed pass clears the memory
                let sleep = Duration::from_secs(POLL_INTERVAL_SECS);
                let verdict = LoopVerdict {
                    next_sleep: Some(sleep),
                    rewinding_streak,
                };
                emit_synced(&shared, &p, verdict).await;
                backoff = prev_backoff(backoff);
                interruptible_sleep(sleep, &mut stop_rx).await;
            }
            Ok(p) => {
                // An `Ok` pass that REWOUND: it reached the tip it recorded, so its
                // facts stand (the pass event, the stamp when earned) — but it is no
                // evidence the server is well behaved, so it climbs the ladder like
                // a fault and never polls faster than a clean pass (§4u; the REQ-1-R
                // fold review's row 1: the reset that stood here handed a patient
                // fork-advance-fork endpoint a fresh 30-rewind budget every 20 s at
                // the 1 s rung, forever). Same order as the `Err` arm: sleep the
                // current rung (floored at the poll interval), then climb. (The
                // raise ran under the pass guard above; `rewinding_streak` is its
                // result.)
                prev_local = false; // R10 §4.2: a completed pass clears the memory
                let sleep = rewinding_sleep(backoff);
                let verdict = LoopVerdict {
                    next_sleep: Some(sleep),
                    rewinding_streak,
                };
                emit_synced(&shared, &p, verdict).await;
                interruptible_sleep(sleep, &mut stop_rx).await;
                backoff = next_backoff(backoff);
            }
            Err(e) => {
                let reason = stall_for(&e);
                // §4u: a `ChainReorg` stall is a rewinding pass (the cap, the storm
                // bound, or the exit belt's DB-below-tip read); any other fault
                // leaves the streak as it stands. `rewound` and `rewinding_streak`
                // were decided under the pass guard above, from the pass's own
                // work (the progress sink's latch) and the stall reason together.
                // What the surface shows: the pass's own reason — unless this pass
                // rewound and the streak has reached the report, when the loop's
                // judgement replaces it. A non-rewind fault under a standing streak
                // keeps its own copy ("check your connection" is that pass's truth).
                let streak_reported = rewound && streak_is_reported(rewinding_streak);
                let published = if streak_reported {
                    StallReason::EndpointMisbehaving
                } else {
                    reason
                };
                // §4u, the REW-1 fold review's row 1: a REWINDING stall takes the
                // same floor as a rewinding `Ok` pass. Without it the arch angle's
                // 1:1 variant — alternate a cap-hit stall with one clean pass —
                // always slept the bare 1 s rung, a full 30-rewind budget every
                // ~21 s, the pre-fix defect's own rate. A NON-rewinding fault keeps
                // the bare rung: a dropped connection deserves a prompt retry, and
                // for it the alternation is closed by the clean pass's decay above,
                // not by a floor here.
                let sleep = if rewound {
                    rewinding_sleep(backoff)
                } else {
                    backoff
                };
                // R10 §4.2: the FIRST local fault since a completed pass publishes
                // nothing — a one-pass WAL race clears on the next pass, 1 s later,
                // and must not flash "restore". A later local fault before a pass
                // completes publishes its own reason, even with network faults or a
                // watchdog cancel in between. Keyed on `published`, so a streak
                // report (never local) always publishes; the memory is keyed on the
                // pass's own `reason`, so a local fault under a report still counts.
                let after_local = prev_local;
                prev_local = after_local || is_local_stall(reason);
                if !is_local_stall(published) || after_local {
                    let _ = shared
                        .status_tx
                        .send(SyncStatus::Stalled { reason: published });
                }
                tracing::warn!(
                    target: "zec_wallet_core",
                    outcome = stall_code(reason),
                    // R10 §4.3: which fault it was (`RW-` code) — the outcome alone
                    // cannot tell a busy store from a corrupt one.
                    code = e.code(),
                    // The sleep the loop actually takes, which is the rung only when
                    // the rung is above the floor — `LoopVerdict::next_sleep`'s
                    // meaning on the `Ok` arms, now honest here too.
                    backoff_secs = sleep.as_secs(),
                    "wallet.sync"
                );
                if streak_reported {
                    log_rewinding_streak(Some(sleep));
                }
                interruptible_sleep(sleep, &mut stop_rx).await;
                backoff = next_backoff(backoff);
            }
        }
    }
}

fn set_active(shared: &Shared, c: Option<CancelToken>) {
    *shared
        .active_cancel
        .lock()
        .expect("active-cancel mutex poisoned") = c;
}

/// What the loop knows about a finished pass that the pass itself cannot carry
/// (§4u): the sleep it takes next — `wallet.sync`'s `backoff_secs`, honest on the
/// `Ok` paths too — and the rewinding streak after this pass, which decides
/// whether `emit_synced` publishes the pass's own verdict or the loop's.
#[derive(Clone, Copy)]
struct LoopVerdict {
    /// `None` from `once()`: no loop sleep follows, and the field is not recorded
    /// (an `Option` tracing value records only when `Some`) — never a made-up zero.
    next_sleep: Option<Duration>,
    /// Consecutive rewinding LOOP passes, the count after this pass on a loop
    /// pass; from `once()`, the loop's STANDING count (phase-2 P2-5 — a
    /// pull-to-refresh carries the loop's judgement, so it can never take a
    /// reported "switch servers" down; it keeps no streak of its own).
    rewinding_streak: u32,
}

impl LoopVerdict {
    /// The `once()` path: outside the loop — no sleep, and the streak as the loop
    /// last left it (`Shared::streak`), never a zero of its own (P2-5).
    fn once(rewinding_streak: u32) -> Self {
        Self {
            next_sleep: None,
            rewinding_streak,
        }
    }
}

/// Map a finished pass to status (the `once()` path; the loop does this inline,
/// plus its backoff/tracing).
async fn emit_outcome(shared: &Shared, res: &Result<SyncPass, WalletError>) {
    match res {
        Ok(p) if p.cancelled => {} // cancelled once(): leave the last status as-is
        // Outside the loop: no sleep follows, so the pass event carries no
        // `backoff_secs` (§4u) — but the streak it carries is the LOOP's standing
        // count, read here (P2-5): a zero of its own published "Up to date" under a
        // reported "switch servers", and the next loop pass put the stall back.
        Ok(p) => emit_synced(shared, p, LoopVerdict::once(shared.streak())).await,
        Err(e) => {
            let _ = shared.status_tx.send(SyncStatus::Stalled {
                reason: stall_for(e),
            });
        }
    }
}

/// A clean pass finished: emit the §5.4 pass span (+ the reorg instability event
/// if any rewinds happened), persist the last-synced stamp — ONLY when the pass
/// reached a tip the grade found at or above what this wallet already holds —
/// then the terminal status (write-then-publish — see `record_synced`).
///
/// **T0-1c-R2 (§4n Q-G2, M2): the durable stamp waits for a current tip.** The
/// stamp used to be written before the ranking, for every clean pass, so a
/// BEHIND pass durably set `ever_synced` (write-once — "this wallet has reached
/// chain tip at least once", which it had not) and stamped the behind height;
/// after a relaunch the header rendered "as of block `<behind>`" with no
/// qualification and the first-run catch-up framing was suppressed for the
/// wallet's life. The shape chosen, of the three the contract priced: gate the
/// WHOLE `record_synced` on `Some(TipStanding::AtOrAboveBundle)` — the flag,
/// the stamp and the rescan-rebuild breadcrumb clear are one "reached tip"
/// event in `LightdSyncEngine::record_synced`, and a behind pass is not that
/// event for any of the three. Property bought: nothing durable ever carries a
/// height the wallet knows is behind, no new state, no new bridge surface, no
/// Dart arm; a relaunch after a behind pass renders the last CURRENT stamp (or
/// none, with the first-run framing, for a wallet that has never had one —
/// honest: it has never been current). Property lost: a behind pass that DID
/// scan (a wallet far behind a server that is itself behind the row) leaves the
/// stamp staler than the balance until a current pass — the stamp under-claims
/// freshness, the safe direction for a display-only row (`sync_stamp` module
/// doc) — and the rescan-rebuild cue stays up through behind passes. The
/// alternatives: gating only `ever_synced` would still rewrite the stamp to the
/// behind height (the wrap's render, unfixed); persisting the standing beside
/// the stamp buys a qualified render at the price of a schema column, a bridge
/// field and a Dart arm for a state the surface already carries live. `None`
/// (a pass that made no claim about its tip — the fakes, a future engine) is
/// "no claim", never "at or above": no stamp, the same reading `root_outcomes`
/// gets. `UpToDateLimited` on an at-or-above pass still stamps (it reached the
/// tip; the BUILD is what cannot read it), as before.
///
/// **§4u REW-1: the loop's verdict rides in beside the pass's.** `verdict` is
/// what the pass cannot carry: the sleep the loop takes next — logged as the pass
/// event's `backoff_secs`, the poll interval after a clean pass and the ladder's
/// rung (never below the poll) after a rewinding one; absent from a `once()`,
/// which sleeps nothing — and the rewinding streak after this pass. From the
/// report threshold on, the status published is `Stalled { EndpointMisbehaving }`
/// INSTEAD of the terminal status, chosen over "both in order" because a `watch`
/// keeps only the latest value: both in order would leave the terminal status on
/// glass for every subscriber that missed the first send, and a badge that
/// alternated between "up to date" and "switch servers" is exactly the flicker
/// the streak exists to prevent. The stamp is STILL written when the pass earned
/// it (`record_synced` above the ranking): the stamp is a fact about the DB —
/// this wallet did reach that tip — while the stall is a judgement about the
/// server, and a relaunch should render the honest "as of block N" beneath
/// whatever the next loop concludes. **Where the stall RANKS (phase-2 P2-6,
/// maintainer decision 5):** below `UpToDateLimited` and below
/// `UpToDateUnverified` — running OR ended — and above everything else. It
/// used to replace the whole ranking, and under a streak the user lost the
/// grace's countdown and `Ended { by: Clock }`'s one actionable remedy (check
/// the device's date and time; "switch servers" fixes nothing there), on the
/// loop path and — since P2-5's once-verdict carries the loop's count — on a
/// pull-to-refresh too. A build that cannot read the chain cannot act on
/// "switch servers" until it is updated, and a grace's consequence reaches
/// SIGNING: both are told first. Under either, the report line still logs
/// (the judgement stands; only the glass shows the higher claim) and the badge
/// says "switch servers" the moment the higher claim clears with the streak
/// still at the report. The `Err` arm's substitution is unchanged — a stall
/// has no grace to hide. It cannot flicker: only clean LOOP passes decay the streak, a clean pass
/// IS the evidence that the server behaved, a `once()` carries the loop's
/// standing count rather than a zero of its own (phase-2 P2-5 — before it, a
/// pull-to-refresh under the report published "Up to date" for one loop
/// interval), and the loop moves the count while it still holds the pass guard,
/// so the `once()` that takes the guard next reads this pass's count, never the
/// pre-pass one (the Batch C security pass's row 5: with the move in the arms,
/// after the guard dropped, a `once()` on the multi-thread runtime could still
/// read the old count and publish "Up to date" after the loop published the
/// stall). What remains, stated exactly (the code reviewer's row 5): the COUNT
/// both publishes carry is this pass's, but which pass's TERMINAL STATUS lands
/// last is not ordered — this function awaits `record_synced` after the guard
/// has dropped, so a `once()` can take the guard, finish and publish inside that
/// await, and the loop then publishes its older pass's verdict into a
/// latest-wins `watch`. Unreachable today (the production door, `once_within`, refuses
/// while the loop is started); the field doc lists it beside the other residuals. Its REACH, stated: the substitution
/// sits inside the `engine.tip()` arm, so a pass with no known tip publishes
/// nothing under the streak just as it publishes no terminal status without one —
/// there is nothing to replace. Unreachable on the shipped path (`LightdSyncEngine`
/// caches the last tip a pass carried, a pass that rewound recorded one to re-queue
/// against, and the cache is sticky across passes), and a `ChainReorg` fault needs
/// no tip to publish its stall — the `Err` arm's own substitution covers it.
async fn emit_synced(shared: &Shared, pass: &SyncPass, verdict: LoopVerdict) {
    if pass.reorgs > 0 {
        // Recovery is observable (gate 5). `depth` is the blocks the pass's rewinds
        // actually un-scanned — `SyncPass::rewound_blocks`, the DB's pre-rewind
        // frontier minus the landed height, summed in the engine's `Rewind` arm
        // (§4q-R P-RR4) — never `reorgs × REWIND_DISTANCE_BLOCKS`, a constant the
        // REQ-1 fold measured 92 blocks wrong on one rewind. §5.4: a count.
        let depth = pass.rewound_blocks;
        tracing::info!(target: "zec_wallet_core", depth, "wallet.reorg_rewind");
    }
    tracing::info!(
        target: "zec_wallet_core",
        batches = pass.batches,
        reorgs = pass.reorgs,
        // SCAN-1 (§4o S3): where the pass went — the batch spans' phase timings
        // summed, the whole pass's wall-clock, and the shielded outputs downloaded.
        // Durations and one chain count (`tracing_guard::ALLOWLIST` says why each
        // narrows nothing); a fake engine's zeros read "not measured".
        anchor_ms = pass.anchor_ms,
        dl_ms = pass.dl_ms,
        scan_ms = pass.scan_ms,
        snap_ms = pass.snap_ms,
        // S15-F1 phase A: the two phases before the first batch.
        tip_ms = pass.tip_ms,
        roots_ms = pass.roots_ms,
        wall_ms = pass.wall_ms,
        chain_outputs = pass.chain_outputs,
        // §4u: the sleep the loop takes next — honest on the `Ok` paths too (the
        // ladder's rung after a rewinding pass); a `once()` records nothing here.
        backoff_secs = verdict.next_sleep.map(|d| d.as_secs()),
        outcome = "ok",
        "wallet.sync"
    );
    if let Some(tip) = shared.engine.tip() {
        // The stamp gate (M2, above): a reached-tip event only when the grade
        // found the tip at or above what this wallet already holds. Written
        // BEFORE the status is published (write-then-publish, `record_synced`).
        if pass.tip_standing == Some(TipStanding::AtOrAboveBundle) {
            shared.engine.record_synced(tip).await;
        }
        // Four claims, in precedence order, and only the last is "Up to date":
        // §6.4 "read everything we scanned" (a stale build cannot make it — and it
        // outranks everything below, because a build that cannot read the chain
        // cannot act on "switch servers" until it is updated); T0-1c "this
        // endpoint's tip is at or above the row this binary shipped with" (read
        // from the pass — `SyncPass::tip_standing`, the second carrier beside
        // `root_outcomes`: no port method, so no fake and no future engine can
        // default its way to "at or above"), which carries the pools report WITH
        // it so the next claim is never hidden by the ranking; T0-1b "this
        // endpoint served every pool" (read from the pass itself); then the plain
        // tip. What each lower status would hide if it won (§4k-R decision 3):
        // `UpToDate` over the behind claim — a stale height reading as current,
        // INC-023 verbatim; `UpToDateDegraded` over it — the same stale height
        // under a pool badge (the right remedy, a false freshness); and
        // `EndpointBehind` over `UpToDateLimited` — "update the app", which the
        // user must do before any server switch can help.
        // GRACE-1 (§4p): a FIFTH claim, between the build's and the server's
        // height — "this server will not say which network it is on" — read from
        // the durable stamp against the device clock (a port method with a
        // default, like `scan_is_fully_interpretable`). It outranks the behind
        // and pool claims because its consequence reaches SIGNING (a countdown
        // on sending, or a refusal), and it carries the `pools` report with it
        // so the ranking drops no pool fact; `UpToDateLimited` cannot co-occur
        // (one verdict). What it hides when it wins over `EndpointBehind` is
        // that claim's `newest_known` — same remedy, "switch servers".
        let pools = pool_report(pass.root_outcomes);
        // §4u: the loop's judgement stands whatever the glass shows — the report
        // line says why the badge and the `outcome = "ok"` pass event disagree
        // (gate 5), and, under a higher claim, why the badge will say "switch
        // servers" the moment that claim clears. Logged BEFORE the ranking so a
        // build or grace claim that outranks the stall (P2-6) does not also
        // silence the field log.
        let streak_reported = streak_is_reported(verdict.rewinding_streak);
        if streak_reported {
            log_rewinding_streak(verdict.next_sleep);
        }
        let status = if !shared.engine.scan_is_fully_interpretable().await {
            SyncStatus::UpToDateLimited { tip }
        } else if let Some(grace) = shared.engine.unknown_branch_grace().await {
            // P3-12 (maintainer, Q2 option 2): the grace still outranks the
            // streak's report (P2-6), but it now CARRIES the report, so the glass
            // can drop "your balance is current" without hiding the countdown.
            SyncStatus::UpToDateUnverified {
                tip,
                grace,
                pools,
                streak_reported,
            }
        } else if streak_reported {
            // §4u / P2-6: the loop's judgement in place of the pass's own, below
            // the build's claim and the grace's (the doc above says why INSTEAD,
            // why the stamp stands, why it ranks here, and why it cannot flicker).
            SyncStatus::Stalled {
                reason: StallReason::EndpointMisbehaving,
            }
        } else if let Some(TipStanding::BehindBundle { newest_known }) = pass.tip_standing {
            SyncStatus::EndpointBehind {
                tip,
                newest_known: BlockHeight::new(newest_known),
                pools,
            }
        } else if let Some(pools) = pools.filter(report_is_degraded) {
            SyncStatus::UpToDateDegraded { tip, pools }
        } else {
            SyncStatus::UpToDate { tip }
        };
        let _ = shared.status_tx.send(status);
    }
}

/// T0-1b D1 — the ONE place an engine's per-pool fetch outcome becomes a surface
/// value. EXHAUSTIVE and wildcard-free on purpose: a [`PoolFetch`] variant added
/// later is a compile error HERE, so it can never again be computed, typed and
/// rendered nowhere (the finding this item exists to close).
fn pool_service(outcome: PoolFetch) -> PoolService {
    match outcome {
        // `usize → u32` cannot truncate in practice (the client-side per-pool cap is
        // 2^16); saturate rather than panic if that ever changes.
        PoolFetch::Served { roots } => PoolService::Served {
            roots: u32::try_from(roots).unwrap_or(u32::MAX),
        },
        PoolFetch::Unsupported => PoolService::Unsupported,
        // The refusal code stays in the log (§5.4 — `report_pool_outcome`); D11:
        // nothing crosses the surface.
        PoolFetch::HeightViolation { code: _ } => PoolService::HeightViolation,
        // The bundle's proof is a count from the signed binary; saturate like the
        // root count above.
        PoolFetch::Withheld { proven } => PoolService::Withheld {
            proven: u32::try_from(proven).unwrap_or(u32::MAX),
        },
    }
}

/// Is this pool's service a DEGRADATION — refused, lied about, or withheld?
/// Exhaustive for the same reason [`pool_service`] is; a served pool is fine at
/// ANY count, because the serve the wallet can grade — zero or short of the
/// bundle's proof (T0-1d) — has already become `Withheld` in
/// `sync::apply_height_bind`, and the count it cannot grade is the honest one
/// (see `SyncStatus::UpToDateDegraded`).
fn pool_is_degraded(service: PoolService) -> bool {
    match service {
        PoolService::Served { .. } => false,
        PoolService::Unsupported | PoolService::HeightViolation | PoolService::Withheld { .. } => {
            true
        }
    }
}

/// The surface's per-pool report — `Some` iff the pass made a pool claim at all.
/// `emit_synced` carries it whole on [`SyncStatus::EndpointBehind`] (so a
/// degraded pool is visible beside the behind claim) and publishes
/// [`SyncStatus::UpToDateDegraded`] with it exactly when [`report_is_degraded`],
/// a plain `UpToDate` otherwise.
///
/// `None` outcomes — a pass that made no pool claim: the fake engines, or a future
/// engine that skips root ingestion — render as "nothing to say", the same reading
/// `LightdSyncEngine::scan_is_fully_interpretable` gives an absent verdict ("no
/// positive reason to qualify the claim"). The production engine cannot reach
/// `emit_synced` with `None` on a clean pass: `Wallet::sync_once` fills the field
/// before its first batch, and `wallet::tests::
/// controller_over_the_production_engine_renders_a_refused_ironwood_pool` pins
/// that through the shipped body, not a fake.
fn pool_report(
    outcomes: Option<[PoolFetch; SUBTREE_ROOT_POOLS.len()]>,
) -> Option<PoolServiceReport> {
    let outcomes = outcomes?;
    // Positional → named. The slot order is compile-time asserted in `sync.rs`
    // (`SUBTREE_ROOT_POOLS[0..3]` are Sapling, Orchard, Ironwood — the wire values
    // 0, 1, 2), and this irrefutable length-3 pattern fails to compile the day a
    // fourth pool is added, which is the point: it must get a field, not a
    // silent drop.
    let [sapling, orchard, ironwood] = outcomes.map(pool_service);
    Some(PoolServiceReport {
        sapling,
        orchard,
        ironwood,
    })
}

/// Is at least one pool in the report refused, lied about, or withheld — the
/// condition under which the report earns its own status
/// ([`SyncStatus::UpToDateDegraded`]) rather than riding a healthier one.
fn report_is_degraded(report: &PoolServiceReport) -> bool {
    pool_is_degraded(report.sapling)
        || pool_is_degraded(report.orchard)
        || pool_is_degraded(report.ironwood)
}

/// Run one watchdogged pass, THEN — only if it finished CLEANLY (Ok + not cancelled) — drive the
/// post-pass resubmission hook (§6.3 / inc-2d-3-b-ii-B). The hook runs under the SAME single-writer
/// pass guard the caller holds (scanner idle), AFTER the watchdog is torn down (so a bounded
/// best-effort resubmission cannot trip a false stuck-restart), with the still-active `cancel` so
/// `stop()`/`Drop` interrupt it promptly. A cancelled or faulted pass skips it (no point
/// resubmitting when the scan did not complete — the next clean pass will). The single seam both
/// `once()` and the background loop call, so the post-pass policy lives in ONE place.
///
/// FR-40: a bounded pass passes its `deadline`, and the hook runs only when the
/// time left covers [`SyncController::RESUBMIT_RESERVE`] (`None` — the loop and
/// the unbounded `once()` — always runs it). The flag returned says the hook ran
/// AND the token had not fired by its end; the loop ignores it.
async fn run_pass_then_resubmit(
    shared: &Arc<Shared>,
    cancel: &CancelToken,
    rewound: &Arc<AtomicBool>,
    deadline: Option<tokio::time::Instant>,
) -> (Result<SyncPass, WalletError>, bool) {
    let res = run_pass_watchdogged(shared, cancel, rewound).await;
    let reserve_fits = deadline.is_none_or(|d| {
        d.saturating_duration_since(tokio::time::Instant::now()) >= SyncController::RESUBMIT_RESERVE
    });
    let mut resubmitted = false;
    if matches!(&res, Ok(p) if !p.cancelled) && reserve_fits {
        shared.engine.after_synced(cancel).await;
        resubmitted = !cancel.is_cancelled();
    }
    (res, resubmitted)
}

/// Run one pass under the no-progress watchdog: a concurrent timer cancels the
/// pass if `SYNC_STUCK_WATCHDOG_SECS` elapse with no progress report. Each
/// progress report re-arms the timer (via `timeout` over the tick channel), so a
/// pass that is making progress is never force-restarted — only a WEDGED pass is.
/// The window is `Shared::stuck_window`; with `None` (test seam only) no watchdog
/// is spawned and the sink is built and dropped exactly as with one.
async fn run_pass_watchdogged(
    shared: &Arc<Shared>,
    cancel: &CancelToken,
    rewound: &Arc<AtomicBool>,
) -> Result<SyncPass, WalletError> {
    let (tick_tx, mut tick_rx) = watch::channel(());
    let sink = ProgressSink {
        status_tx: shared.status_tx.clone(),
        tick_tx,
        rewound: Arc::clone(rewound),
    };
    let wd_cancel = cancel.clone();
    let watchdog: Option<JoinHandle<()>> = shared.stuck_window.map(|window| {
        tokio::spawn(async move {
            loop {
                match tokio::time::timeout(window, tick_rx.changed()).await {
                    Ok(Ok(())) => {}      // progress — re-arm the window
                    Ok(Err(_)) => return, // pass finished — sink (tick sender) dropped
                    Err(_elapsed) => {
                        // No progress in the whole window ⇒ wedged. Cancel + step aside.
                        wd_cancel.cancel();
                        return;
                    }
                }
            }
        })
    });

    let res = shared.engine.run_pass(cancel, &sink).await;
    // Drop the sink so the watchdog's `changed()` errors (`Ok(Err(_))`) and it exits;
    // abort is a backstop if it is mid-window. This implicit self-termination is also
    // what makes `Drop`'s `h.abort()` safe: aborting THIS future mid-await detaches
    // the spawned watchdog, but dropping `sink` (its tick sender) here closes the
    // channel, so the detached watchdog ends promptly instead of lingering a full
    // window. A refactor that moves `sink` out of this future MUST keep that property.
    drop(sink);
    if let Some(watchdog) = watchdog {
        watchdog.abort();
    }
    res
}

/// Sleep `d`, but wake immediately if stop is requested — so `stop()` never waits
/// out a poll interval or a backoff. (`timeout` over the stop channel: `Elapsed`
/// ⇒ the full `d` slept; otherwise stop was signalled / the sender dropped.)
async fn interruptible_sleep(d: Duration, stop_rx: &mut watch::Receiver<bool>) {
    let _ = tokio::time::timeout(d, stop_rx.changed()).await;
}

/// Capped exponential backoff: double, clamp at [`SYNC_BACKOFF_MAX_SECS`]. The
/// loop retries FOREVER (§6.2) — only the interval is bounded, never the count.
/// Climbed by every finished pass that was not clean — a fault, or an `Ok` pass
/// that rewound (§4u) — and reset only by a clean one.
fn next_backoff(prev: Duration) -> Duration {
    (prev * 2).min(Duration::from_secs(SYNC_BACKOFF_MAX_SECS))
}

/// One rung DOWN: halve, floored at [`INITIAL_BACKOFF`]. The clean pass's step
/// since the REW-1 fold review's row 1 (§4u; the maintainer's decision).
///
/// It replaces a RESET, and the reset was the round's own evasion: a clean pass
/// costs a hostile endpoint nothing — hold the tip still, `next_batch` finds no
/// non-empty range and returns `None`, and the pass ends `Ok` with no batches and
/// no rewinds, which IS clean — so one free pass handed back the whole ladder and
/// zeroed the streak with it. Giving back one rung at a time means an endpoint has
/// to behave about as often as it forks to stay on the low rungs. Pure —
/// boundary-tested.
fn prev_backoff(prev: Duration) -> Duration {
    (prev / 2).max(INITIAL_BACKOFF)
}

/// The sleep after an `Ok` pass that rewound (§4u): the ladder's current rung,
/// floored at the poll interval — a rewinding pass must never poll FASTER than a
/// clean one, and at the rungs below `POLL_INTERVAL_SECS` the ladder climbs unseen
/// (1 → 2 → 4 → 8 → 16 s all sleep the 20 s poll) until it passes the poll. Pure —
/// boundary-tested.
fn rewinding_sleep(backoff: Duration) -> Duration {
    backoff.max(Duration::from_secs(POLL_INTERVAL_SECS))
}

/// Has the rewinding streak reached the report (§4u)? `true` from
/// [`MAX_CONSECUTIVE_REWINDING_PASSES`] consecutive rewinding passes on — the
/// threshold pass itself and every rewinding pass after it, until enough clean
/// passes have DECAYED the count back below the threshold. One clean pass takes
/// one off (`2a6c87e2`, the maintainer's reset-vs-decay decision); this sentence said
/// "clears" until an earlier revision and had been false since that commit. Pure —
/// boundary-tested.
fn streak_is_reported(rewinding_streak: u32) -> bool {
    rewinding_streak >= MAX_CONSECUTIVE_REWINDING_PASSES
}

/// The streak report's log line (§4u; gate 5 — a badge that says "switch servers"
/// over a pass event that says `ok` needs the line that explains it): a code
/// naming the check, and the sleep the loop takes next. `outcome =
/// "rewinding_streak"` is the one vocabulary value this round minted
/// (`tracing_guard::ALLOWLIST` says why it adds no name).
fn log_rewinding_streak(next_sleep: Option<Duration>) {
    tracing::warn!(
        target: "zec_wallet_core",
        outcome = "rewinding_streak",
        backoff_secs = next_sleep.map(|d| d.as_secs()),
        "wallet.sync"
    );
}

/// Map a pass error to a renderable stall reason (§2.5 honest degradation). Net
/// faults already arrive typed `Sync { stall }`; a local `DiskFull` is `StorageFull`;
/// a busy store OR a non-ENOSPC filesystem IO fault (permission / EIO / read-only)
/// is `StorageUnavailable` (transient, retried — R10); a corrupt store is `Internal`
/// (a LOCAL fault — repair/restore, not a server switch); anything else
/// falls back to a transient endpoint issue so the loop keeps retrying (a persistent
/// unknown fault then shows a steady Stalled).
///
/// (d-3-b-iii-B-2 resolved the prior KNOWN DEBT: local faults now render honestly as
/// `Internal` — the user is told the problem is local, not the network — instead of
/// the misleading `EndpointUnreachable`. Whether the controller should STOP retrying
/// on `Internal` — a corrupt store won't heal by retrying — is a separate BEHAVIORAL
/// change tracked for the send/repair increment (inc-2d / repair path), NOT this read
/// slice; today it surfaces the truthful steady `Stalled { Internal }`, which never
/// tells the user funds are lost.)
///
/// T0-1c (§4j row 2): a `NetworkMismatch` is the server answering for the WRONG
/// CHAIN — a testnet lightwalletd under a mainnet wallet. The link works and the
/// data is useless, which is the content tier's own definition, so it renders
/// `EndpointMisbehaving` ("switch servers", the one remedy) and never the
/// catch-all's "check your connection", which it fell into before — telling the
/// user to fix a connection that worked, every pass, forever. The producers of
/// that variant reachable from `run_pass`, listed so the mapping's honesty is
/// checkable (P5): exactly ONE, `provision::endpoint_identity`, reached every pass
/// through `Wallet::evaluate_consensus` and on the first pass through
/// `provision::resolve_birthday`. (The STATUS `Stalled { EndpointMisbehaving }`
/// has had a second producer since §4u that is no error at all — the loop's
/// rewinding-streak report, `run_loop` / `emit_synced`, a judgement over
/// consecutive passes; it never passes through this mapping.) The crate's other
/// producers (`memo`,
/// `derivation`, `payment_uri` — address, key and URI parsing on the send and
/// import paths; `store` and `Wallet::open` — the manifest check at open) are not
/// on the pass, and a manifest mismatch is a LOCAL misconfig this mapping would
/// misdescribe if one ever were. A consensus `Unsupported` / foreign-branch
/// verdict is NOT this class: that is `SyncStatus::UpToDateLimited`, a build
/// problem ("update the app"), published by `emit_synced` and never a stall.
fn stall_for(err: &WalletError) -> StallReason {
    match err {
        WalletError::Sync { stall } => *stall,
        WalletError::DiskFull => StallReason::StorageFull,
        WalletError::NetworkMismatch => StallReason::EndpointMisbehaving,
        // T0-1c-R2 (§4m #13, §4k-run owed 2): a configured birthday above the
        // chain as this server reports it, on the provisioning pass. Its own
        // reading — the `_ =>` fallback rendered it "check your connection" for a
        // link that worked, and PREEMPTED the behind report on exactly T0-1c's
        // population (provisioning runs before `sync_once`, so the grade never
        // ran). Two producers since T0-1c-R3: `provision::resolve_birthday` (a
        // configured birthday above `max(tip, row)` on the provisioning pass)
        // and `Wallet::sync_once`'s guard (`sync::birthday_beyond_tip` — the
        // account's birthday above `tip + 1` with the tip at or above the grade's
        // reference; unreachable from any shipped resolver today, an arm the
        // contract names). Below the reference neither reaches here: M3 clamps a
        // provisioning tip up to the row, and the guard turns the pass into
        // `EndpointBehind` with nothing scanned instead of upstream's assert
        // (INC-024). What does reach here is a birthday above a height no oracle
        // disputes — a host typo or a behind server, and the wallet cannot say
        // which, so the variant's copy names both remedies
        // (`StallReason::BirthdayInFuture`).
        WalletError::BirthdayInFuture => StallReason::BirthdayInFuture,
        // Local, device-side faults — never "switch servers". R10 splits them by
        // remedy. `StoreBusy` (#371: a mid-scan write that lost its WAL race to a
        // concurrent commit) and `Io` (a filesystem fault that is not ENOSPC — EIO, a
        // locked iOS device's Data Protection) are the classes #371 documents as
        // transient and non-corrupt: `StorageUnavailable`, "retrying", because the
        // store is intact and the next pass usually clears it. Only `StoreCorrupt` can
        // justify the restore remedy, so only it keeps `Internal` (a device walk: one
        // local fault after a send told the user to restore for ~60 s). Both are held
        // back by `run_loop` until a second local fault before a pass completes (§4.2).
        WalletError::StoreBusy | WalletError::Io(_) => StallReason::StorageUnavailable,
        WalletError::StoreCorrupt => StallReason::Internal,
        _ => StallReason::EndpointUnreachable,
    }
}

/// Stable, secret-free code for the `outcome` tracing field (§5.4 allowlist:
/// error codes only — never an address / amount / memo). Exhaustive on purpose
/// (in-crate): a new `StallReason` variant fails to compile until it has a code.
fn stall_code(reason: StallReason) -> &'static str {
    match reason {
        StallReason::EndpointUnreachable => "endpoint_unreachable",
        StallReason::TorUnavailable => "tor_unavailable",
        StallReason::StorageFull => "storage_full",
        StallReason::ChainReorg => "chain_reorg",
        StallReason::Internal => "internal",
        StallReason::EndpointMisbehaving => "endpoint_misbehaving",
        StallReason::BirthdayInFuture => "birthday_in_future",
        StallReason::StorageUnavailable => "storage_unavailable",
    }
}

/// R10 §4.2: is this a LOCAL stall — a fault on this device, which `run_loop`
/// publishes only when the previous loop pass faulted locally too? Exhaustive on
/// purpose, like [`stall_code`]: a new variant must decide which side it is on.
fn is_local_stall(reason: StallReason) -> bool {
    match reason {
        StallReason::Internal | StallReason::StorageUnavailable => true,
        StallReason::EndpointUnreachable
        | StallReason::TorUnavailable
        | StallReason::StorageFull
        | StallReason::ChainReorg
        | StallReason::EndpointMisbehaving
        | StallReason::BirthdayInFuture => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicUsize;

    use tracing_subscriber::prelude::*;

    fn h(n: u32) -> BlockHeight {
        BlockHeight::new(n)
    }

    fn progress(tip: u32, scanned: u32, pct: f32) -> SyncProgress {
        SyncProgress {
            tip: h(tip),
            scanned_to: h(scanned),
            percent: pct,
            spendable_ready: false,
            rewound: false,
        }
    }

    /// A healthy completed pass: no pool claim (the fakes make none) and a tip
    /// the grade found at or above what the wallet holds — the standing every
    /// stamp row needs, since T0-1c-R2 gates `record_synced` on it. A pass with
    /// NO standing is `no_claim_pass` below.
    fn ok_pass(batches: u32) -> SyncPass {
        SyncPass {
            batches,
            reorgs: 0,
            cancelled: false,
            tip: None,
            root_outcomes: None,
            tip_standing: Some(TipStanding::AtOrAboveBundle),
            // the SCAN-1 timings: a fake's zeros are "not measured"
            ..SyncPass::default()
        }
    }

    /// A completed pass that made no claim about its tip's standing (a fake or
    /// a future engine that skipped the grade) — "no claim", never "at or above".
    fn no_claim_pass(batches: u32) -> SyncPass {
        SyncPass {
            tip_standing: None,
            ..ok_pass(batches)
        }
    }

    fn reorg_pass(batches: u32, reorgs: u32) -> SyncPass {
        SyncPass {
            batches,
            reorgs,
            cancelled: false,
            tip: None,
            root_outcomes: None,
            tip_standing: Some(TipStanding::AtOrAboveBundle),
            ..SyncPass::default()
        }
    }

    #[test]
    fn keepalive_kicks_the_watchdog_tick_without_touching_status() {
        // Fix A: the §3.3 enhancement drain re-arms the stuck-sync watchdog via `keepalive` so a
        // slow backlog doesn't trip a false `Stalled` — but it must NOT republish `Scanning` (the
        // scan's last status stays current until the pass returns). So `keepalive` kicks `tick_tx`
        // (the watchdog re-arm — the same channel the watchdog `timeout`s over) and leaves
        // `status_tx` untouched, unlike `report`.
        let (status_tx, mut status_rx) = watch::channel(SyncStatus::Idle);
        let (tick_tx, mut tick_rx) = watch::channel(());
        let sink = ProgressSink {
            status_tx,
            tick_tx,
            rewound: Arc::new(AtomicBool::new(false)),
        };
        // Mark both as seen so `has_changed` reflects ONLY what the next call does.
        let _ = status_rx.borrow_and_update();
        let _ = tick_rx.borrow_and_update();

        sink.keepalive();
        assert!(
            tick_rx.has_changed().expect("tick sender live"),
            "keepalive re-arms the watchdog (kicks tick_tx)",
        );
        assert!(
            !status_rx.has_changed().expect("status sender live"),
            "keepalive does NOT republish status — the scan's last status stays current",
        );

        // Contrast: `report` DOES change the status (and also kicks the tick).
        sink.report(progress(100, 100, 1.0));
        assert!(
            status_rx.has_changed().expect("status sender live"),
            "report (unlike keepalive) publishes a Scanning status update",
        );
    }

    /// One scripted pass outcome for the deterministic engine.
    enum Plan {
        /// Report each step, then return this pass.
        Ok {
            steps: Vec<SyncProgress>,
            pass: SyncPass,
        },
        /// Return this fault.
        Err(WalletError),
        /// Report each step, THEN return this fault — a pass that did work and
        /// then died. The shape the review's HIGH runs on: a pass may
        /// truncate up to `MAX_TOTAL_REORGS_PER_PASS` times and then meet a
        /// transport fault, and the loop must price the rewinds it performed
        /// rather than the reason it exited. `Plan::Err` alone cannot express
        /// it (no steps ⇒ nothing to report ⇒ the sink's rewound latch never
        /// fires), which is why the arm was untestable and therefore untested.
        ErrAfter {
            steps: Vec<SyncProgress>,
            err: WalletError,
        },
        /// Report steps, then BLOCK until cancelled (a WEDGED pass) — returns a
        /// cancelled pass when the watchdog or `stop()` fires.
        Stuck { steps: Vec<SyncProgress> },
        /// Report each step with a `gap` sleep BEFORE it (a slow-but-ADVANCING pass on
        /// the virtual clock), then return this pass. Models the d-3-b-i contract: a
        /// download that progresses slower than wall-clock but re-arms the watchdog
        /// each report — it must NOT be force-restarted even if the total span exceeds
        /// the watchdog window.
        Slow {
            steps: Vec<SyncProgress>,
            gap: Duration,
            pass: SyncPass,
        },
        /// Emit NO scan progress (the chain is already at tip — a deep-restore memo backlog
        /// draining after the scan caught up), then fire `keepalives` watchdog kicks each
        /// preceded by a `gap` sleep on the virtual clock. Models the §3.3 enhancement drain
        /// re-arming the watchdog via `ProgressSink::keepalive` (NOT `report`): the silent-but-
        /// alive phase must NOT be force-restarted even though its total span exceeds the watchdog
        /// window and the published status never changes (Fix A).
        KeepaliveDrain {
            keepalives: usize,
            gap: Duration,
            pass: SyncPass,
        },
    }

    /// A deterministic `SyncEnginePort`: pops a scripted outcome per pass; once the
    /// script is exhausted it behaves as "already up to date" (a no-op pass), so a
    /// loop that out-runs the script just idles at `UpToDate` rather than panic.
    /// Phase-3 P3-5: park the engine INSIDE `record_synced` on its `on_call`-th
    /// call — the controller's write-then-publish await, i.e. the loop task
    /// between its guard drop and its terminal publish. `parked` fires when the
    /// loop is there; `release` lets it continue.
    struct RecordSyncedPark {
        on_call: usize,
        parked: Option<tokio::sync::oneshot::Sender<()>>,
        release: Arc<tokio::sync::Notify>,
    }

    struct FakeEngine {
        plan: Mutex<VecDeque<Plan>>,
        tip: BlockHeight,
        /// P3-5: an optional park inside `record_synced` (see `RecordSyncedPark`).
        record_synced_park: Mutex<Option<RecordSyncedPark>>,
        calls: AtomicUsize,
        /// The (virtual-clock) start instant of each pass — lets a test assert the
        /// inter-pass spacing (e.g. the watchdog window + the backoff fold).
        call_times: Mutex<Vec<tokio::time::Instant>>,
        /// How many times the controller invoked [`SyncEnginePort::after_synced`] — the
        /// post-pass resubmission hook (inc-2d-3-b-ii-B). Asserts it fires ONLY after a clean pass.
        after_synced_calls: AtomicUsize,
        /// Every tip the controller asked to stamp via [`SyncEnginePort::record_synced`]
        /// (#317) — asserts the stamp fires per CLEAN pass with the published tip, and
        /// NEVER on a cancelled/faulted one.
        record_synced_tips: Mutex<Vec<BlockHeight>>,
        /// Wired by the write-then-publish pin (#317 wrap review F1): a subscriber
        /// `record_synced` peeks at — if it already observes `UpToDate`, the
        /// write-then-publish order was inverted.
        status_probe: Mutex<Option<watch::Receiver<SyncStatus>>>,
        stamped_after_publish: AtomicBool,
        /// §4u (REW-1): the `reorgs` of every pass this engine RETURNED, in call
        /// order — `Some(reorgs)` for a returned pass (a scripted `Ok`, or the
        /// exhausted script's no-op), `None` for a scripted fault. The ladder
        /// rows assert this as their PRECONDITION, so a row whose plan was
        /// shortened fails on "these were not the scripted passes" and never on
        /// the clause it exists to red. Recorded on the three arms those rows
        /// script (`Ok`, `Err`, the exhausted script); the wedge, slow and drain
        /// arms are not recorded, and no ladder row uses them.
        returned_reorgs: Mutex<Vec<Option<u32>>>,
    }

    impl FakeEngine {
        fn new(tip: u32, plan: Vec<Plan>) -> Arc<Self> {
            Arc::new(Self {
                plan: Mutex::new(plan.into()),
                tip: h(tip),
                record_synced_park: Mutex::new(None),
                calls: AtomicUsize::new(0),
                call_times: Mutex::new(Vec::new()),
                after_synced_calls: AtomicUsize::new(0),
                record_synced_tips: Mutex::new(Vec::new()),
                status_probe: Mutex::new(None),
                stamped_after_publish: AtomicBool::new(false),
                returned_reorgs: Mutex::new(Vec::new()),
            })
        }
        /// P3-5: arm a park on the `on_call`-th `record_synced` — returns the
        /// "parked" signal and the release handle.
        fn park_record_synced(
            &self,
            on_call: usize,
        ) -> (tokio::sync::oneshot::Receiver<()>, Arc<tokio::sync::Notify>) {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let release = Arc::new(tokio::sync::Notify::new());
            *self.record_synced_park.lock().expect("park mutex") = Some(RecordSyncedPark {
                on_call,
                parked: Some(tx),
                release: Arc::clone(&release),
            });
            (rx, release)
        }
        fn call_count(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }
        fn call_times(&self) -> Vec<tokio::time::Instant> {
            self.call_times.lock().expect("call-times mutex").clone()
        }
        fn after_synced_count(&self) -> usize {
            self.after_synced_calls.load(Ordering::Relaxed)
        }
        fn recorded_synced_tips(&self) -> Vec<BlockHeight> {
            self.record_synced_tips
                .lock()
                .expect("record-synced mutex")
                .clone()
        }
        fn returned_reorgs(&self) -> Vec<Option<u32>> {
            self.returned_reorgs
                .lock()
                .expect("returned-reorgs mutex")
                .clone()
        }
        fn note_returned(&self, reorgs: Option<u32>) {
            self.returned_reorgs
                .lock()
                .expect("returned-reorgs mutex")
                .push(reorgs);
        }
    }

    #[async_trait]
    impl SyncEnginePort for FakeEngine {
        async fn run_pass(
            &self,
            cancel: &CancelToken,
            sink: &ProgressSink,
        ) -> Result<SyncPass, WalletError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.call_times
                .lock()
                .expect("call-times mutex")
                .push(tokio::time::Instant::now());
            let next = self.plan.lock().expect("plan mutex poisoned").pop_front();
            match next {
                Some(Plan::Ok { steps, pass }) => {
                    for s in steps {
                        sink.report(s);
                    }
                    self.note_returned(Some(pass.reorgs));
                    Ok(pass)
                }
                Some(Plan::Err(e)) => {
                    self.note_returned(None);
                    Err(e)
                }
                Some(Plan::ErrAfter { steps, err }) => {
                    for s in steps {
                        sink.report(s);
                    }
                    self.note_returned(None);
                    Err(err)
                }
                Some(Plan::Stuck { steps }) => {
                    for s in steps {
                        sink.report(s);
                    }
                    cancel.cancelled().await; // wedged until the watchdog / stop cancels
                    Ok(SyncPass {
                        cancelled: true,
                        ..SyncPass::default()
                    })
                }
                Some(Plan::Slow { steps, gap, pass }) => {
                    for s in steps {
                        // Slow-but-alive: sleep (virtual clock) then report — each
                        // report re-arms the watchdog. If cancelled mid-wait (the
                        // watchdog DID fire — a defect this test asserts against), bail.
                        if tokio::time::timeout(gap, cancel.cancelled()).await.is_ok() {
                            return Ok(SyncPass {
                                cancelled: true,
                                ..SyncPass::default()
                            });
                        }
                        sink.report(s);
                    }
                    Ok(pass)
                }
                Some(Plan::KeepaliveDrain {
                    keepalives,
                    gap,
                    pass,
                }) => {
                    for _ in 0..keepalives {
                        // Like Slow, but re-arm via `keepalive` (tick only, NO status) — the §3.3
                        // drain. If the watchdog cancels mid-wait (the bug this asserts against),
                        // bail cancelled.
                        if tokio::time::timeout(gap, cancel.cancelled()).await.is_ok() {
                            return Ok(SyncPass {
                                cancelled: true,
                                ..SyncPass::default()
                            });
                        }
                        sink.keepalive();
                    }
                    Ok(pass)
                }
                None => {
                    self.note_returned(Some(0));
                    Ok(SyncPass::default())
                }
            }
        }
        fn tip(&self) -> Option<BlockHeight> {
            Some(self.tip)
        }
        async fn record_synced(&self, tip: BlockHeight) {
            if let Some(rx) = self.status_probe.lock().expect("probe mutex").as_mut()
                && matches!(&*rx.borrow(), SyncStatus::UpToDate { .. })
            {
                self.stamped_after_publish.store(true, Ordering::Relaxed);
            }
            let call = {
                let mut tips = self.record_synced_tips.lock().expect("record-synced mutex");
                tips.push(tip);
                tips.len()
            };
            // P3-5: the armed park, if this is its call — signal, then wait for the
            // release. The std guard is dropped before the await.
            let release = {
                let mut park = self.record_synced_park.lock().expect("park mutex");
                match park.as_mut() {
                    Some(p) if p.on_call == call => {
                        if let Some(tx) = p.parked.take() {
                            let _ = tx.send(());
                        }
                        Some(Arc::clone(&p.release))
                    }
                    _ => None,
                }
            };
            if let Some(release) = release {
                release.notified().await;
            }
        }
        async fn after_synced(&self, _cancel: &CancelToken) {
            self.after_synced_calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn wait_for_uptodate(sub: &mut watch::Receiver<SyncStatus>) {
        loop {
            if matches!(&*sub.borrow_and_update(), SyncStatus::UpToDate { .. }) {
                return;
            }
            sub.changed().await.expect("status stream alive");
        }
    }

    // ── the orchestration behaviours ────────────────────────────────────────

    #[tokio::test]
    async fn controller_once_reports_uptodate_on_a_clean_pass() {
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![progress(2_000_000, 1_999_000, 0.5)],
                pass: ok_pass(1),
            }],
        );
        let ctl = SyncController::new(engine.clone());
        let pass = ctl.once().await.expect("pass ok");
        assert_eq!(pass.batches, 1);
        assert_eq!(ctl.status(), SyncStatus::UpToDate { tip: h(2_000_000) });
        assert_eq!(engine.call_count(), 1);
    }

    // ── T0-1b: the degraded-pool surface, at the renderer ───────────────────────
    //
    // These drive `emit_synced` through a FAKE engine — they pin the RENDERING
    // (which pass value publishes which status), not the shipped path. The shipped
    // path (the production engine's own pass body, injected client, no subscriber)
    // is pinned in `wallet::tests::controller_over_the_production_engine_*`, and
    // the contract's named proofs live with the test author.

    fn pass_with_pools(outcomes: [PoolFetch; SUBTREE_ROOT_POOLS.len()]) -> SyncPass {
        SyncPass {
            root_outcomes: Some(outcomes),
            ..ok_pass(0)
        }
    }

    fn served(roots: usize) -> PoolFetch {
        PoolFetch::Served { roots }
    }

    async fn status_after_one_pass(pass: SyncPass) -> SyncStatus {
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![],
                pass,
            }],
        );
        let ctl = SyncController::new(engine);
        ctl.once().await.expect("pass ok");
        ctl.status()
    }

    #[tokio::test]
    async fn a_refused_pool_publishes_up_to_date_degraded_with_the_per_pool_report() {
        // D1/D5: the pass SUCCEEDED (two pools served, one refused), so it is not a
        // stall — but it is not `UpToDate` either, and the report names which pool.
        let status = status_after_one_pass(pass_with_pools([
            served(3),
            served(2),
            PoolFetch::Unsupported,
        ]))
        .await;
        assert_eq!(
            status,
            SyncStatus::UpToDateDegraded {
                tip: h(2_000_000),
                pools: PoolServiceReport {
                    sapling: PoolService::Served { roots: 3 },
                    orchard: PoolService::Served { roots: 2 },
                    ironwood: PoolService::Unsupported,
                },
            }
        );
    }

    #[tokio::test]
    async fn every_pool_served_publishes_up_to_date_at_any_count_including_zero() {
        // D4 "a healthy sync does not raise it" — and zero roots IS the honest state
        // before a pool's first completed subtree (see `UpToDateDegraded`'s doc for
        // why raising on it would be the false alarm).
        for outcomes in [
            [served(5), served(1), served(0)],
            [served(0), served(0), served(0)],
            [served(12), served(7), served(4)],
        ] {
            assert_eq!(
                status_after_one_pass(pass_with_pools(outcomes)).await,
                SyncStatus::UpToDate { tip: h(2_000_000) },
                "{outcomes:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_height_violation_is_a_different_surface_value_from_a_refusal() {
        // D1: "the server lied about heights" and "the server does not know this
        // pool" are two sentences; the code stays in the log (D11).
        let lied = status_after_one_pass(pass_with_pools([
            served(1),
            served(1),
            PoolFetch::HeightViolation {
                code: "completion_gap",
            },
        ]))
        .await;
        let refused = status_after_one_pass(pass_with_pools([
            served(1),
            served(1),
            PoolFetch::Unsupported,
        ]))
        .await;
        assert_eq!(
            lied,
            SyncStatus::UpToDateDegraded {
                tip: h(2_000_000),
                pools: PoolServiceReport {
                    sapling: PoolService::Served { roots: 1 },
                    orchard: PoolService::Served { roots: 1 },
                    ironwood: PoolService::HeightViolation,
                },
            }
        );
        assert_ne!(lied, refused);
    }

    #[tokio::test]
    async fn a_degraded_pool_served_on_the_next_pass_clears_the_variant() {
        // D4: not a latch. The status is a property of the pass that produced it.
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Ok {
                    steps: vec![],
                    pass: pass_with_pools([served(1), served(1), PoolFetch::Unsupported]),
                },
                Plan::Ok {
                    steps: vec![],
                    pass: pass_with_pools([served(1), served(1), served(0)]),
                },
            ],
        );
        let ctl = SyncController::new(engine);
        ctl.once().await.expect("pass ok");
        assert!(matches!(ctl.status(), SyncStatus::UpToDateDegraded { .. }));
        ctl.once().await.expect("pass ok");
        assert_eq!(ctl.status(), SyncStatus::UpToDate { tip: h(2_000_000) });
    }

    #[tokio::test]
    async fn a_pass_that_makes_no_pool_claim_publishes_up_to_date() {
        // The documented reading of `root_outcomes: None` (a fake, or an engine that
        // skipped root ingestion): nothing to say, the same as an absent consensus
        // verdict. NOT the production engine's reading — it always makes a claim
        // (`wallet::tests::controller_over_the_production_engine_*`).
        assert_eq!(
            status_after_one_pass(ok_pass(0)).await,
            SyncStatus::UpToDate { tip: h(2_000_000) }
        );
    }

    /// An engine whose build cannot read the chain — wraps a scripted fake and
    /// answers `scan_is_fully_interpretable` with `false`.
    struct StaleBuild(Arc<FakeEngine>);
    #[async_trait]
    impl SyncEnginePort for StaleBuild {
        async fn run_pass(
            &self,
            cancel: &CancelToken,
            sink: &ProgressSink,
        ) -> Result<SyncPass, WalletError> {
            self.0.run_pass(cancel, sink).await
        }
        fn tip(&self) -> Option<BlockHeight> {
            self.0.tip()
        }
        async fn scan_is_fully_interpretable(&self) -> bool {
            false
        }
    }

    /// An engine that answers the two ranking questions ABOVE the streak (P2-6):
    /// the build's claim (`scan_is_fully_interpretable`) and the grace's
    /// (`unknown_branch_grace`), scripted per test, over a scripted fake for the
    /// passes themselves.
    struct Claiming {
        inner: Arc<FakeEngine>,
        interpretable: bool,
        grace: Option<UnknownBranchGrace>,
    }
    #[async_trait]
    impl SyncEnginePort for Claiming {
        async fn run_pass(
            &self,
            cancel: &CancelToken,
            sink: &ProgressSink,
        ) -> Result<SyncPass, WalletError> {
            self.inner.run_pass(cancel, sink).await
        }
        fn tip(&self) -> Option<BlockHeight> {
            self.inner.tip()
        }
        async fn scan_is_fully_interpretable(&self) -> bool {
            self.interpretable
        }
        async fn unknown_branch_grace(&self) -> Option<UnknownBranchGrace> {
            self.grace
        }
    }

    #[tokio::test]
    async fn a_stale_build_outranks_a_degraded_pool() {
        // Both claims true at once: the build cannot read the chain AND the endpoint
        // refused a pool. `UpToDateLimited` wins — "update the app" comes before
        // "switch servers" can help, and its balance-is-a-floor already covers this.
        let inner = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![],
                pass: pass_with_pools([served(1), served(1), PoolFetch::Unsupported]),
            }],
        );
        let ctl = SyncController::new(Arc::new(StaleBuild(inner)));
        ctl.once().await.expect("pass ok");
        assert_eq!(
            ctl.status(),
            SyncStatus::UpToDateLimited { tip: h(2_000_000) }
        );
    }

    // ── T0-1c (§4k-R): the behind standing's rendering, through the fake engine ──
    //
    // The shipped path — `fetch_tip`'s grade riding `SyncPass::tip_standing` out of
    // the production pass body — is pinned in `wallet::tests::
    // controller_over_the_production_engine_*` (every one of which now runs BELOW
    // the testnet row); the contract's named proofs are the test author's. These
    // pin the RENDERING: which carrier value publishes which status.

    /// The reference the fake engine's grade "fired against" — any height above
    /// its tip (the bundle's row, or its own scanned height less the margin: the
    /// rendering does not care which).
    const NEWEST: u32 = 3_000_000;

    fn behind() -> TipStanding {
        TipStanding::BehindBundle {
            newest_known: NEWEST,
        }
    }

    fn pass_with_standing(
        standing: TipStanding,
        outcomes: Option<[PoolFetch; SUBTREE_ROOT_POOLS.len()]>,
    ) -> SyncPass {
        SyncPass {
            root_outcomes: outcomes,
            tip_standing: Some(standing),
            ..ok_pass(0)
        }
    }

    fn report(
        sapling: PoolService,
        orchard: PoolService,
        ironwood: PoolService,
    ) -> PoolServiceReport {
        PoolServiceReport {
            sapling,
            orchard,
            ironwood,
        }
    }

    #[tokio::test]
    async fn a_behind_tip_publishes_endpoint_behind_with_the_pools_report_beside_it() {
        // Q-E1's surface: a completed pass whose tip the grade found below the row
        // is NOT `UpToDate` (INC-023's silence), NOT a stall (the pass succeeded,
        // the pools were served) and NOT `UpToDateDegraded` (nothing is
        // under-served) — it is its own sentence, naming the row it is behind and
        // carrying what every pool did.
        let status = status_after_one_pass(pass_with_standing(
            behind(),
            Some([served(3), served(2), served(1)]),
        ))
        .await;
        assert_eq!(
            status,
            SyncStatus::EndpointBehind {
                tip: h(2_000_000),
                newest_known: h(NEWEST),
                pools: Some(report(
                    PoolService::Served { roots: 3 },
                    PoolService::Served { roots: 2 },
                    PoolService::Served { roots: 1 },
                )),
            }
        );
    }

    #[tokio::test]
    async fn a_behind_tip_and_a_degraded_pool_are_both_carried_on_one_pass() {
        // RE4: both facts at once — the server is behind the row AND it withheld a
        // pool. One status carries both (D1: never a conflation, never a drop):
        // the behind variant wins the ranking and the pools report rides with
        // it, `Withheld` intact. A mutant that drops the report from the behind
        // arm (`pools: None`) reds the first assertion; one that ranks
        // `UpToDateDegraded` above the behind claim reds it too.
        let both = status_after_one_pass(pass_with_standing(
            behind(),
            Some([served(1), served(1), PoolFetch::Withheld { proven: 1 }]),
        ))
        .await;
        assert_eq!(
            both,
            SyncStatus::EndpointBehind {
                tip: h(2_000_000),
                newest_known: h(NEWEST),
                pools: Some(report(
                    PoolService::Served { roots: 1 },
                    PoolService::Served { roots: 1 },
                    PoolService::Withheld { proven: 1 },
                )),
            }
        );
        // And the pool fact is what distinguishes it from a behind server that
        // serves every pool: two sentences, not one.
        let behind_only = status_after_one_pass(pass_with_standing(
            behind(),
            Some([served(1), served(1), served(1)]),
        ))
        .await;
        assert_ne!(
            both, behind_only,
            "behind-and-withheld and behind are two sentences"
        );
        // The same pass with no behind claim is the T0-1b sentence, unchanged.
        assert_eq!(
            status_after_one_pass(pass_with_pools([
                served(1),
                served(1),
                PoolFetch::Withheld { proven: 1 }
            ]))
            .await,
            SyncStatus::UpToDateDegraded {
                tip: h(2_000_000),
                pools: report(
                    PoolService::Served { roots: 1 },
                    PoolService::Served { roots: 1 },
                    PoolService::Withheld { proven: 1 },
                ),
            }
        );
    }

    #[tokio::test]
    async fn a_behind_tip_without_a_pool_claim_still_says_behind() {
        // D8, one level over: an engine that graded the tip but made no pool
        // claim (a fake, or a future engine that skips root ingestion) must not
        // have its behind claim dropped because the OTHER carrier is absent. The
        // report is "nothing to say" (`None`), the behind claim stands.
        assert_eq!(
            status_after_one_pass(pass_with_standing(behind(), None)).await,
            SyncStatus::EndpointBehind {
                tip: h(2_000_000),
                newest_known: h(NEWEST),
                pools: None,
            }
        );
    }

    #[tokio::test]
    async fn a_pass_that_makes_no_tip_claim_is_not_behind() {
        // The documented reading of `tip_standing: None`: "no claim made", never
        // "at or above" — and, like `root_outcomes: None`, nothing to say, so the
        // OTHER claims decide. NOT the production engine's reading — it always
        // grades the tip (`wallet::tests::controller_over_the_production_engine_*`).
        assert_eq!(
            status_after_one_pass(pass_with_pools([served(1), served(1), served(0)])).await,
            SyncStatus::UpToDate { tip: h(2_000_000) }
        );
        assert!(matches!(
            status_after_one_pass(pass_with_pools([
                served(1),
                served(1),
                PoolFetch::Unsupported
            ]))
            .await,
            SyncStatus::UpToDateDegraded { .. }
        ));
    }

    #[tokio::test]
    async fn a_tip_at_or_above_the_bundle_is_not_behind() {
        // The positive grade changes nothing: the T0-1b rendering decides.
        assert_eq!(
            status_after_one_pass(pass_with_standing(
                TipStanding::AtOrAboveBundle,
                Some([served(1), served(1), served(0)]),
            ))
            .await,
            SyncStatus::UpToDate { tip: h(2_000_000) }
        );
        assert!(matches!(
            status_after_one_pass(pass_with_standing(
                TipStanding::AtOrAboveBundle,
                Some([served(1), served(1), PoolFetch::Unsupported]),
            ))
            .await,
            SyncStatus::UpToDateDegraded { .. }
        ));
    }

    #[tokio::test]
    async fn a_stale_build_outranks_a_behind_endpoint() {
        // Precedence (§4k-R decision 3): the build cannot read the chain AND the
        // server is behind. "Update the app" wins — a build that cannot read the
        // chain cannot act on "switch servers" until it is updated, and its
        // balance-is-a-floor claim subsumes the behind one. What this hides for
        // one pass: the behind fact and the pools report, both moot until the
        // update.
        let inner = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![],
                pass: pass_with_standing(
                    behind(),
                    Some([served(1), served(1), PoolFetch::Unsupported]),
                ),
            }],
        );
        let ctl = SyncController::new(Arc::new(StaleBuild(inner)));
        ctl.once().await.expect("pass ok");
        assert_eq!(
            ctl.status(),
            SyncStatus::UpToDateLimited { tip: h(2_000_000) }
        );
    }

    #[tokio::test]
    async fn a_behind_endpoint_that_is_at_or_above_the_row_on_the_next_pass_clears_the_variant() {
        // E6's class, rendered: not a latch. The status is a property of the pass
        // that produced it — the pass on which the grade reads at-or-above
        // publishes a plain `UpToDate` again, nothing sticky (RE8: no new state).
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Ok {
                    steps: vec![],
                    pass: pass_with_standing(behind(), Some([served(1), served(1), served(1)])),
                },
                Plan::Ok {
                    steps: vec![],
                    pass: pass_with_standing(
                        TipStanding::AtOrAboveBundle,
                        Some([served(1), served(1), served(1)]),
                    ),
                },
            ],
        );
        let ctl = SyncController::new(engine);
        ctl.once().await.expect("pass ok");
        assert!(matches!(ctl.status(), SyncStatus::EndpointBehind { .. }));
        ctl.once().await.expect("pass ok");
        assert_eq!(ctl.status(), SyncStatus::UpToDate { tip: h(2_000_000) });
    }

    #[test]
    fn pool_service_is_total_and_keeps_the_count_but_not_the_code() {
        assert_eq!(pool_service(served(0)), PoolService::Served { roots: 0 });
        assert_eq!(
            pool_service(served(65_536)),
            PoolService::Served { roots: 65_536 }
        );
        assert_eq!(
            pool_service(PoolFetch::Unsupported),
            PoolService::Unsupported
        );
        assert_eq!(
            pool_service(PoolFetch::HeightViolation {
                code: "recorded_height"
            }),
            PoolService::HeightViolation
        );
        // The fifth outcome: the bundle's proof crosses as a count, and it degrades.
        assert_eq!(
            pool_service(PoolFetch::Withheld { proven: 3 }),
            PoolService::Withheld { proven: 3 }
        );
        assert!(!pool_is_degraded(PoolService::Served { roots: 0 }));
        assert!(pool_is_degraded(PoolService::Unsupported));
        assert!(pool_is_degraded(PoolService::HeightViolation));
        assert!(pool_is_degraded(PoolService::Withheld { proven: 1 }));
    }

    #[tokio::test]
    async fn a_withheld_pool_publishes_up_to_date_degraded_and_a_proofless_zero_does_not() {
        // Adjudication (b): the zero the wallet can grade (the bind turned it into
        // `Withheld`) is degraded; the zero it cannot grade stays `Served { 0 }` and
        // publishes `UpToDate`. Which is which is decided in `sync::apply_height_bind`
        // against the bundled treestates; here only the rendering is pinned.
        let withheld = status_after_one_pass(pass_with_pools([
            served(1),
            served(1),
            PoolFetch::Withheld { proven: 1 },
        ]))
        .await;
        assert_eq!(
            withheld,
            SyncStatus::UpToDateDegraded {
                tip: h(2_000_000),
                pools: PoolServiceReport {
                    sapling: PoolService::Served { roots: 1 },
                    orchard: PoolService::Served { roots: 1 },
                    ironwood: PoolService::Withheld { proven: 1 },
                },
            }
        );
        let proofless =
            status_after_one_pass(pass_with_pools([served(1), served(1), served(0)])).await;
        assert_eq!(proofless, SyncStatus::UpToDate { tip: h(2_000_000) });
        assert_ne!(
            withheld,
            status_after_one_pass(pass_with_pools([
                served(1),
                served(1),
                PoolFetch::Unsupported
            ]))
            .await,
            "withheld and refused are two sentences"
        );
    }

    // ── inc-2d-3-b-ii-B: the post-pass resubmission hook wiring ──────────────────

    #[tokio::test]
    async fn resubmission_runs_after_a_clean_pass() {
        // §6.3: a CLEAN pass drives the outbox exactly once — under the same single-writer guard,
        // scanner idle. (`once()` is the deterministic single-pass seam; the loop wires the SAME
        // `run_pass_then_resubmit`.)
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                pass: ok_pass(1),
            }],
        );
        let ctl = SyncController::new(engine.clone());
        ctl.once().await.expect("pass ok");
        assert_eq!(
            engine.after_synced_count(),
            1,
            "a clean pass drives resubmission exactly once",
        );
    }

    // ── #317: the last-synced stamp hook (`record_synced`) ──────────────────────

    #[tokio::test]
    async fn a_clean_pass_records_the_stamp_with_the_published_tip() {
        // §2.5 `last_synced`: every clean pass stamps EXACTLY the tip it publishes
        // as `UpToDate` — write-then-publish, one stamp per pass.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                pass: ok_pass(1),
            }],
        );
        let ctl = SyncController::new(engine.clone());
        ctl.once().await.expect("pass ok");
        assert_eq!(ctl.status(), SyncStatus::UpToDate { tip: h(2_000_000) });
        assert_eq!(
            engine.recorded_synced_tips(),
            vec![h(2_000_000)],
            "one stamp, carrying the SAME tip UpToDate published",
        );
    }

    #[tokio::test]
    async fn the_stamp_is_recorded_before_uptodate_is_published() {
        // #317 write-then-publish: a host that `snapshot()`s on seeing UpToDate
        // must read a stamp matching the tip it was told about. `emit_synced`
        // awaits record_synced on the SAME task before sending, so the probe
        // (peeking at the status channel from INSIDE record_synced) is
        // deterministic — a reordered emit flips the flag every run.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![],
                pass: ok_pass(1),
            }],
        );
        let ctl = SyncController::new(engine.clone());
        *engine.status_probe.lock().expect("probe mutex") = Some(ctl.subscribe());
        ctl.once().await.expect("pass ok");
        assert_eq!(engine.recorded_synced_tips(), vec![h(2_000_000)]);
        assert!(
            !engine.stamped_after_publish.load(Ordering::Relaxed),
            "record_synced observed UpToDate already published — write-then-publish inverted",
        );
    }

    #[tokio::test]
    async fn faulted_and_cancelled_passes_record_no_stamp() {
        // A faulted pass did not complete; a cancelled pass may have stopped anywhere.
        // Neither is a truthful "synced to tip at this time" — no stamp for either.
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Err(WalletError::Sync {
                    stall: StallReason::EndpointUnreachable,
                }),
                Plan::Ok {
                    steps: vec![],
                    pass: SyncPass {
                        cancelled: true,
                        ..SyncPass::default()
                    },
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        let _ = ctl.once().await; // faulted
        let _ = ctl.once().await; // cancelled
        assert_eq!(
            engine.recorded_synced_tips(),
            Vec::<BlockHeight>::new(),
            "no stamp without a completed pass",
        );
    }

    /// T0-1c-R2 (§4n G2, M2) — the stamp gate, at the rendering: a completed
    /// pass whose tip the grade found BEHIND publishes `EndpointBehind` and
    /// records NO stamp; the next pass at or above stamps. At the base the
    /// behind pass stamped its tip before the ranking. Mutants: the gate
    /// removed (the first assertion red: one stamp too many); `record_synced`
    /// moved back above the ranking, ungated (the same); the gate inverted
    /// (the second assertion red: the at-or-above pass records nothing).
    #[tokio::test]
    async fn a_behind_pass_records_no_stamp_and_the_next_at_or_above_pass_does() {
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Ok {
                    steps: vec![],
                    pass: pass_with_standing(behind(), Some([served(1), served(1), served(1)])),
                },
                Plan::Ok {
                    steps: vec![],
                    pass: ok_pass(0),
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        ctl.once().await.expect("the behind pass completes");
        assert!(
            matches!(ctl.status(), SyncStatus::EndpointBehind { .. }),
            "{:?}",
            ctl.status()
        );
        assert_eq!(
            engine.recorded_synced_tips(),
            Vec::<BlockHeight>::new(),
            "a behind pass is not a reached-tip event: no stamp",
        );
        ctl.once().await.expect("the current pass completes");
        assert_eq!(ctl.status(), SyncStatus::UpToDate { tip: h(2_000_000) });
        assert_eq!(
            engine.recorded_synced_tips(),
            vec![h(2_000_000)],
            "the at-or-above pass stamps, once, with the tip it publishes",
        );
    }

    /// T0-1c-R2 (M2) — a completed pass that made NO claim about its tip's
    /// standing (`tip_standing: None`: the fakes, a future engine that skipped
    /// the grade) is "no claim", never "at or above": no stamp, and the status
    /// still renders (the pools/tip claims are independent). Mutant: the gate
    /// written as `!= Some(BehindBundle)` (the stamp assertion red).
    #[tokio::test]
    async fn a_pass_with_no_standing_claim_records_no_stamp() {
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![],
                pass: no_claim_pass(1),
            }],
        );
        let ctl = SyncController::new(engine.clone());
        ctl.once().await.expect("pass ok");
        assert_eq!(ctl.status(), SyncStatus::UpToDate { tip: h(2_000_000) });
        assert_eq!(
            engine.recorded_synced_tips(),
            Vec::<BlockHeight>::new(),
            "no claim about the tip's standing ⇒ no reached-tip event ⇒ no stamp",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn scanning_status_carries_the_rewound_flag() {
        // §2.5 `Scanning.rewound` (#317): a progress sample stamped rewound by the
        // engine reaches subscribers as an explicit first-class field — a host latch
        // reacts to THIS, not to inferring a reorg from a shrinking `to`.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Stuck {
                steps: vec![SyncProgress {
                    rewound: true,
                    ..progress(2_000_000, 1_900_000, 0.4)
                }],
            }],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        loop {
            if let SyncStatus::Scanning { rewound, .. } = &*sub.borrow_and_update() {
                assert!(*rewound, "the engine's rewound stamp must survive the hop");
                break;
            }
            sub.changed().await.expect("status channel open");
        }
        ctl.stop().await;
    }

    #[tokio::test]
    async fn resubmission_is_skipped_after_a_faulted_pass() {
        // A faulted pass (dropped link) did NOT complete the scan — no point resubmitting against a
        // half-synced view. The hook must NOT fire; the next clean pass will.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Err(WalletError::Sync {
                stall: StallReason::EndpointUnreachable,
            })],
        );
        let ctl = SyncController::new(engine.clone());
        let _ = ctl.once().await; // returns the fault
        assert_eq!(
            engine.after_synced_count(),
            0,
            "a faulted pass never drives resubmission",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn resubmission_is_skipped_after_a_watchdog_cancelled_pass() {
        // A WEDGED pass that the watchdog cancels is `cancelled`, not clean — resubmission must NOT
        // fire on it (the broadcast would race a torn-down scan view). The virtual clock advances to
        // the watchdog window; the Stuck pass returns cancelled.
        let engine = FakeEngine::new(2_000_000, vec![Plan::Stuck { steps: vec![] }]);
        let ctl = SyncController::new(engine.clone());
        let pass = ctl.once().await.expect("a cancelled pass is still Ok");
        assert!(pass.cancelled, "the watchdog cancelled the wedge");
        assert_eq!(
            engine.after_synced_count(),
            0,
            "a watchdog-cancelled pass never drives resubmission",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn controller_keepalive_only_drain_is_not_falsely_restarted() {
        // Fix A end-to-end through the REAL controller watchdog: a deep-restore memo drain with the
        // chain ALREADY at tip emits NO scan progress, so its ONLY watchdog re-arm is the per-tx
        // `keepalive` (tick-only, no status change). Each kick is spaced half the window; the total
        // span is 2× the window — far past it — yet the watchdog must NOT fire, because each
        // keepalive re-arms it. This is the assembled controller↔sink wire the sink-unit test
        // (`keepalive_kicks_the_watchdog_tick_without_touching_status`, which builds its own
        // channels) cannot reach. Pre-Fix-A (no keepalive) this exact shape tripped a false Stalled.
        let gap = Duration::from_secs(SYNC_STUCK_WATCHDOG_SECS / 2);
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::KeepaliveDrain {
                keepalives: 4, // 4 × (window/2) = 2× the window total
                gap,
                pass: ok_pass(0), // a no-op scan (already at tip); the drain is the only work
            }],
        );
        let ctl = SyncController::new(engine.clone());
        let pass = ctl
            .once()
            .await
            .expect("the keepalive-armed pass completes");
        assert!(
            !pass.cancelled,
            "keepalive re-armed the watchdog every half-window ⇒ the slow drain is NOT restarted",
        );
        assert_eq!(
            engine.call_count(),
            1,
            "no false watchdog restart — a single pass"
        );
        assert_eq!(
            engine.after_synced_count(),
            1,
            "a clean (non-cancelled) pass drives resubmission — the money path Fix A unblocks",
        );
    }

    #[tokio::test]
    async fn controller_emits_live_scanning_status_during_a_pass() {
        // A wedged pass that first reports one progress step: the live status must
        // be `Scanning` WHILE the pass runs (the honest-degradation stream moves,
        // not a single opaque state). `start()` so the pass runs in the background.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Stuck {
                steps: vec![progress(2_000_000, 1_950_000, 0.8)],
            }],
        );
        let ctl = SyncController::new(engine);
        let mut sub = ctl.subscribe();
        ctl.start();
        // Wait for the Scanning event (auto-advance drives the spawned pass).
        loop {
            if let SyncStatus::Scanning {
                from,
                to,
                percent,
                spendable_ready,
                rewound: _,
            } = &*sub.borrow_and_update()
            {
                assert_eq!(*from, h(1_950_000));
                assert_eq!(*to, h(2_000_000));
                assert!((*percent - 0.8).abs() < 1e-6);
                assert!(!spendable_ready);
                break;
            }
            sub.changed().await.expect("status stream alive");
        }
        ctl.stop().await;
        assert_eq!(ctl.status(), SyncStatus::Idle);
    }

    #[tokio::test(start_paused = true)]
    async fn controller_recovers_from_a_transient_fault_with_backoff() {
        // First pass errors (a dropped link), then a clean pass: the loop surfaces
        // Stalled, waits out the backoff (virtual clock), and converges to UpToDate
        // WITHOUT manual intervention — the unstable-network resume property.
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Err(WalletError::Sync {
                    stall: StallReason::EndpointUnreachable,
                }),
                Plan::Ok {
                    steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                    pass: ok_pass(2),
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        // Both the fault pass and the recovery pass ran (≥ 2 — the idle no-op passes
        // after may add more before stop lands).
        assert!(engine.call_count() >= 2);
    }

    #[tokio::test(start_paused = true)]
    async fn controller_retries_forever_until_success() {
        // The Zashi "manually restart it" bug, inverted: five faults then success.
        // Reaching UpToDate proves the loop NEVER gives up (defaultRetries = ∞).
        let mut plan: Vec<Plan> = (0..5)
            .map(|_| {
                Plan::Err(WalletError::Sync {
                    stall: StallReason::EndpointUnreachable,
                })
            })
            .collect();
        plan.push(Plan::Ok {
            steps: vec![progress(2_000_000, 2_000_000, 1.0)],
            pass: ok_pass(1),
        });
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        assert!(engine.call_count() >= 6);
    }

    #[tokio::test(start_paused = true)]
    async fn controller_watchdog_restarts_a_wedged_pass() {
        // First pass reports one step then WEDGES (no error, no further progress) —
        // the failure the retry loop alone can't see. The no-progress watchdog must
        // cancel it after the window and the loop restarts; the second pass is clean.
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Stuck {
                    steps: vec![progress(2_000_000, 1_900_000, 0.7)],
                },
                Plan::Ok {
                    steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                    pass: ok_pass(1),
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        // The wedged pass + the recovery pass both ran (the watchdog forced #2).
        assert!(engine.call_count() >= 2);
    }

    #[tokio::test(start_paused = true)]
    async fn controller_persistently_wedged_sync_degrades_to_stalled() {
        // A pass that wedges with NO progress (the failure the retry loop can't see):
        // the watchdog must surface a typed Stalled (honest degradation §2.5/§3.3),
        // not leave a forever-moving "Scanning". Two wedges so the Stalled persists
        // across a restart and is deterministically observable.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Stuck { steps: vec![] }, Plan::Stuck { steps: vec![] }],
        );
        let ctl = SyncController::new(engine);
        let mut sub = ctl.subscribe();
        ctl.start();
        loop {
            if matches!(&*sub.borrow_and_update(), SyncStatus::Stalled { .. }) {
                break;
            }
            sub.changed().await.expect("status stream alive");
        }
        ctl.stop().await;
        assert_eq!(ctl.status(), SyncStatus::Idle);
    }

    #[tokio::test(start_paused = true)]
    async fn controller_slow_but_progressing_pass_is_not_falsely_restarted() {
        // §8 (the NAMED d-3-b obligation — a batch whose work spans LONGER than the
        // watchdog window but keeps REPORTING progress must NOT be force-restarted).
        // A `Slow` pass reports 10 steps with a SYNC_STUCK_WATCHDOG_SECS/2 gap each:
        // the total span (5× the window) far exceeds the watchdog window, but every
        // gap is < the window so each report re-arms it. The pass runs to completion
        // exactly ONCE (call_count == 1) and converges — proving the d-3-b-i
        // intra-batch report cadence defeats a false stuck-restart on a slow-but-alive
        // link. (This is the controller-level discharge of the contract; the real
        // engine honors it because `download_range` reports every PROGRESS_REPORT_BLOCKS
        // blocks and each transport read is idle-timeout-bounded < the window.)
        let gap = Duration::from_secs(SYNC_STUCK_WATCHDOG_SECS / 2);
        let steps: Vec<_> = (1..=10)
            .map(|i| progress(2_000_000, 1_900_000 + i * 10_000, i as f32 / 10.0))
            .collect();
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Slow {
                steps,
                gap,
                pass: ok_pass(3),
            }],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        assert_eq!(
            engine.call_count(),
            1,
            "a slow-but-progressing pass ran ONCE (never watchdog-restarted), got {}",
            engine.call_count()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn controller_backs_off_a_zero_progress_wedge_before_restart() {
        // §8 (the d-3-b backoff fold): a wedge that commits NOTHING (0 batches) must
        // back off before the loop re-runs it — so a persistently-wedged endpoint can't
        // busy-loop re-downloading the same range. A `Stuck` pass (no steps ⇒ 0
        // batches) is watchdog-cancelled at the window; the NEXT pass must start
        // STRICTLY LATER than the bare window (window + backoff), proving the backoff
        // sleep was inserted (without the fold the restart is immediate at the window).
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Stuck { steps: vec![] }, // 0-progress wedge ⇒ watchdog cancel, 0 batches
                Plan::Ok {
                    steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                    pass: ok_pass(1),
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        let times = engine.call_times();
        assert!(times.len() >= 2, "both passes ran, got {}", times.len());
        let between = times[1] - times[0];
        assert!(
            between > Duration::from_secs(SYNC_STUCK_WATCHDOG_SECS),
            "the 0-progress wedge backed off (window + backoff) before restart: {between:?}"
        );
    }

    // ── §4u REW-1: the ladder never resets on a rewinding pass; the streak ──────
    //
    // The implementer's own rows (the RW rows are the test author's). They run the
    // CONSUMER — the loop over `FakeEngine` on the virtual clock — and read the
    // inter-pass spacing off `call_times`, the published statuses off a collector
    // that wakes on every `watch` send, and the stamp off `recorded_synced_tips`.

    /// Every status the loop publishes, in order. Spawned BEFORE `start()`; on the
    /// current-thread test runtime the loop yields at each sleep, so a subscriber
    /// that wakes on `changed()` sees every terminal status (none coalesce).
    fn collect_statuses(mut sub: watch::Receiver<SyncStatus>) -> Arc<Mutex<Vec<SyncStatus>>> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let out = Arc::clone(&seen);
        tokio::spawn(async move {
            while sub.changed().await.is_ok() {
                let status = sub.borrow().clone();
                out.lock().expect("status collector").push(status);
            }
        });
        seen
    }

    /// Run the loop until the engine has been asked for `n` passes (the virtual
    /// clock auto-advances through the loop's sleeps), then stop it.
    async fn run_until_calls(ctl: &SyncController, engine: &FakeEngine, n: usize) {
        ctl.start();
        while engine.call_count() < n {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        ctl.stop().await;
    }

    /// The statuses the loop published for its passes: everything but the `Idle`
    /// that `stop()` sends last.
    fn pass_statuses(seen: &Arc<Mutex<Vec<SyncStatus>>>) -> Vec<SyncStatus> {
        seen.lock()
            .expect("status collector")
            .iter()
            .filter(|s| !matches!(s, SyncStatus::Idle))
            .cloned()
            .collect()
    }

    #[test]
    fn rewinding_sleep_floors_at_the_poll_and_the_streak_reports_at_the_threshold() {
        let poll = Duration::from_secs(POLL_INTERVAL_SECS);
        let max = Duration::from_secs(SYNC_BACKOFF_MAX_SECS);
        assert_eq!(rewinding_sleep(INITIAL_BACKOFF), poll);
        assert_eq!(rewinding_sleep(poll), poll);
        assert_eq!(
            rewinding_sleep(poll + Duration::from_secs(1)),
            poll + Duration::from_secs(1)
        );
        assert_eq!(rewinding_sleep(max), max);
        assert!(!streak_is_reported(0));
        assert!(!streak_is_reported(MAX_CONSECUTIVE_REWINDING_PASSES - 1));
        assert!(streak_is_reported(MAX_CONSECUTIVE_REWINDING_PASSES));
        assert!(streak_is_reported(MAX_CONSECUTIVE_REWINDING_PASSES + 1));

        // §4u, the REW-1 fold review's row 1: the clean pass's step DOWN. One rung,
        // floored at the initial one — never a reset, and never below the bottom.
        assert_eq!(prev_backoff(INITIAL_BACKOFF), INITIAL_BACKOFF);
        assert_eq!(prev_backoff(next_backoff(INITIAL_BACKOFF)), INITIAL_BACKOFF);
        let two_up = next_backoff(next_backoff(INITIAL_BACKOFF));
        assert_eq!(prev_backoff(two_up), next_backoff(INITIAL_BACKOFF));
        assert_eq!(prev_backoff(max), max / 2);
        // The round trip, at the boundary that matters: climbing then stepping down
        // returns the same rung, so the ladder is a ladder and not a slide. (At the
        // cap the climb saturates, so the identity is stated where it holds.)
        let mut rung = INITIAL_BACKOFF;
        while next_backoff(rung) < max {
            assert_eq!(
                prev_backoff(next_backoff(rung)),
                rung,
                "one up then one down is the identity below the cap; rung {rung:?}"
            );
            rung = next_backoff(rung);
        }
        // And the decay's own boundary, which is the property the review asked for:
        // one clean pass gives back ONE rung, not the ladder.
        assert_ne!(
            prev_backoff(max),
            INITIAL_BACKOFF,
            "a clean pass at the cap must not return to the initial rung — that is the reset \
             the REW-1 fold review's row 1 found evadable at about a sixth of the cost"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_rewinding_ok_pass_never_polls_faster_than_a_clean_one() {
        // §4u, the implementer's pin (a): `[Ok { reorgs: 1 }, Ok { reorgs: 0 }]`. The
        // rewinding pass sleeps `max(POLL_INTERVAL, backoff)` — at the base rung that
        // is the poll interval, never below it (the `max` dropped → 1 s, red); the
        // clean pass sleeps the poll interval and resets the ladder. One honest reorg
        // is forgiven: `UpToDate` after both, never a stall, and the stamp written for
        // both (the stamp is a fact about the DB).
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Ok {
                    steps: vec![],
                    pass: reorg_pass(2, 1),
                },
                Plan::Ok {
                    steps: vec![],
                    pass: ok_pass(1),
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        let seen = collect_statuses(ctl.subscribe());
        run_until_calls(&ctl, &engine, 3).await;

        let t = engine.call_times();
        assert!(t.len() >= 3, "three passes ran, got {}", t.len());
        let poll = Duration::from_secs(POLL_INTERVAL_SECS);
        let after_rewinding = t[1] - t[0];
        let after_clean = t[2] - t[1];
        assert!(
            after_rewinding >= poll,
            "a rewinding Ok pass must not poll faster than a clean one: slept \
             {after_rewinding:?}, poll {poll:?}"
        );
        assert_eq!(
            after_clean, poll,
            "the clean pass sleeps exactly the poll interval"
        );
        let statuses = pass_statuses(&seen);
        assert!(
            statuses
                .iter()
                .all(|s| matches!(s, SyncStatus::UpToDate { .. })),
            "one honest reorg is forgiven — UpToDate after both passes, never a stall: \
             {statuses:?}"
        );
        assert_eq!(
            engine.recorded_synced_tips(),
            vec![h(2_000_000), h(2_000_000)],
            "both passes reached the tip and both stamped it"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn rewinding_ok_passes_climb_the_ladder_and_a_streak_is_reported_until_a_clean_pass() {
        // §4u, the implementer's pin (b) — a DEVIATION from the brief's `× 3`: the
        // rungs 1, 2, 4, 8, 16 s all sit under the 20 s poll, so by the rule itself the
        // first five rewinding spacings ARE the poll interval and `× 3` cannot show a
        // climb; `× 8` can (the rung passes the poll on the sixth spacing).
        // `[Ok { reorgs: 30 }] × 8`: every spacing ≥ the poll interval, the spacings
        // non-decreasing, and the last > the first — the ladder climbed and no
        // rewinding `Ok` reset it (the reset moved back to every `Ok` → all 20 s, red).
        // The streak: `UpToDate` after each pass below MAX_CONSECUTIVE_REWINDING_PASSES,
        // `Stalled { EndpointMisbehaving }` from the threshold pass on (the counter
        // removed → red), the stamp written for every one of the eight (each reached
        // its tip), and the clean no-op passes after the plan: each DECAYS the streak
        // by one (the REW-1 fold review's row 1), so the badge stands until the count
        // falls below the report and only then does `UpToDate` return (the decay
        // turned back into a reset → the first clean pass clears it → red; the decay
        // removed entirely → the badge never clears → red).
        const PASSES: usize = 8;
        let threshold = MAX_CONSECUTIVE_REWINDING_PASSES as usize;
        assert!(threshold < PASSES, "the row drives past the threshold");
        let plan = (0..PASSES)
            .map(|_| Plan::Ok {
                steps: vec![],
                pass: reorg_pass(4, 30),
            })
            .collect();
        // A clean pass DECAYS the streak by one since the REW-1 fold review's
        // row 1, so the badge needs this many clean passes to fall below the
        // report — derived from the two constants, never typed.
        let clean_to_clear = PASSES - threshold + 1;
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        let seen = collect_statuses(ctl.subscribe());
        run_until_calls(&ctl, &engine, PASSES + clean_to_clear + 1).await;

        let t = engine.call_times();
        assert!(
            t.len() > PASSES + clean_to_clear,
            "the eight rewinding passes and the {clean_to_clear} clean ones after ran, got {}",
            t.len()
        );
        let spacings: Vec<Duration> = t.windows(2).take(PASSES).map(|w| w[1] - w[0]).collect();
        let poll = Duration::from_secs(POLL_INTERVAL_SECS);
        assert!(
            spacings.iter().all(|s| *s >= poll),
            "a rewinding pass never polls faster than a clean one: {spacings:?}"
        );
        assert!(
            spacings.windows(2).all(|w| w[1] >= w[0]),
            "the spacings never fall while rewinds continue: {spacings:?}"
        );
        assert!(
            spacings[PASSES - 1] > spacings[0],
            "the ladder climbed across rewinding Ok passes: {spacings:?}"
        );

        let statuses = pass_statuses(&seen);
        assert!(statuses.len() > PASSES, "a status per pass: {statuses:?}");
        for (i, status) in statuses.iter().take(PASSES).enumerate() {
            let pass_no = i + 1;
            if pass_no < threshold {
                assert!(
                    matches!(status, SyncStatus::UpToDate { .. }),
                    "pass {pass_no}, below the threshold, publishes its own terminal \
                     status: {status:?}"
                );
            } else {
                assert_eq!(
                    *status,
                    SyncStatus::Stalled {
                        reason: StallReason::EndpointMisbehaving
                    },
                    "pass {pass_no}, at or past the threshold, publishes the streak's judgement"
                );
            }
        }
        for k in 0..clean_to_clear - 1 {
            assert!(
                matches!(
                    statuses[PASSES + k],
                    SyncStatus::Stalled {
                        reason: StallReason::EndpointMisbehaving
                    }
                ),
                "clean pass {} of {clean_to_clear} only DECAYS the streak (to {}), so the badge \
                 still stands — the REW-1 fold review's row 1: a clean pass is free, so clearing \
                 on one handed it back for nothing. Got {:?}; all {statuses:?}",
                k + 1,
                PASSES - k - 1,
                statuses[PASSES + k]
            );
        }
        assert!(
            matches!(
                statuses[PASSES + clean_to_clear - 1],
                SyncStatus::UpToDate { .. }
            ),
            "the {clean_to_clear}th clean pass decays the streak below the report, so the \
             terminal status returns: {:?}",
            statuses[PASSES + clean_to_clear - 1]
        );
        assert_eq!(
            engine.recorded_synced_tips().len(),
            PASSES,
            "every rewinding Ok pass reached its tip and stamped it — the stamp is a fact \
             about the DB, the stall a judgement about the server"
        );
    }

    /// **Phase-2 P2-5 (phase-1 §4u-run row 3) — the flicker's absence.** This row
    /// used to be `…_and_cannot_report_a_streak`, and it PINNED the defect: it
    /// asserted `UpToDate` after seven rewinding `once()` calls on the ground that
    /// `once()` "keeps no streak" — which is exactly how a pull-to-refresh under a
    /// reported "switch servers" published "Up to date" and the next loop pass put
    /// the stall back. Three clauses now:
    ///
    /// 1. `once()` cannot RAISE the streak (still true — the count is over LOOP
    ///    passes): one more rewinding `once()` than the threshold, alone, ends on
    ///    the pass's own terminal and nothing reports a streak;
    /// 2. `once()` CARRIES the loop's standing count (the fix): the loop drives the
    ///    streak to the report, a CLEAN `once()` runs under it, and the stall stays
    ///    on glass — a clean once() is not a loop pass, so it neither decays the
    ///    streak nor takes the badge down — with the report line logged for it;
    /// 3. a `once()` pass event carries NO `backoff_secs` at all (an `Option`
    ///    tracing value records only when `Some`), never a made-up zero.
    ///
    /// Watched against: `LoopVerdict::once(shared.streak())` → `LoopVerdict::once(0)`
    /// in `emit_outcome` (the pre-P2-5 `ONCE`; clause 2 reds on the status), and
    /// `next_sleep: Some(Duration::ZERO)` in `LoopVerdict::once` (clause 3 reds).
    #[tokio::test(start_paused = true)]
    async fn a_once_pass_records_no_next_sleep_and_carries_the_loops_streak() {
        use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);
        let has_streak_report = |records: &[Vec<(String, String)>]| {
            records.iter().any(|r| {
                r.iter()
                    .any(|(n, v)| n == "outcome" && v == "rewinding_streak")
            })
        };
        let assert_no_next_sleep = |records: &[Vec<(String, String)>]| {
            let ok: Vec<&Vec<(String, String)>> = records
                .iter()
                .filter(|r| r.iter().any(|(n, v)| n == "outcome" && v == "ok"))
                .collect();
            assert!(
                !ok.is_empty(),
                "a once() pass emits its pass event: {records:?}"
            );
            for event in ok {
                assert!(
                    !event.iter().any(|(n, _)| n == "backoff_secs"),
                    "a once() pass sleeps nothing, so the pass event records no next \
                     sleep — never a made-up zero: {event:?}"
                );
            }
        };

        // Clause 1 — `once()` alone: one more rewinding pass than the threshold, past
        // it if `once()` kept a streak of its own.
        let calls = MAX_CONSECUTIVE_REWINDING_PASSES + 1;
        let plan = (0..calls).map(|_| rewinding(30)).collect();
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        for _ in 0..calls {
            let _ = ctl.once().await.expect("pass ok");
        }
        assert_eq!(
            engine.call_count(),
            calls as usize,
            "every once() call ran a scripted rewinding pass"
        );
        assert_eq!(
            ctl.status(),
            SyncStatus::UpToDate { tip: h(2_000_000) },
            "`once()` cannot RAISE the streak: with no loop pass behind it, a rewinding \
             pull-to-refresh publishes the pass's own terminal status"
        );
        let alone = sink.records_of("wallet.sync");
        assert!(
            !has_streak_report(&alone),
            "no loop pass ran, so nothing reported a streak: {alone:?}"
        );
        assert_no_next_sleep(&alone); // clause 3, on the once()-only events

        // Clause 2 — the loop drives the streak to the report, then a CLEAN `once()`
        // runs under it while the loop sleeps its rung.
        let streak = EXPECTED_REWINDING_STREAK;
        let mut plan: Vec<Plan> = (0..streak).map(|_| rewinding(1)).collect();
        plan.push(clean()); // consumed by the once() below, not by the loop
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_pass_starts(&engine, &mut sub, streak).await;
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        assert!(
            is_misbehaving(&ctl.status()),
            "precondition: after {streak} rewinding loop passes the report stands; the loop \
             left {:?}",
            ctl.status()
        );
        let before = sink.records_of("wallet.sync").len();
        let pass = ctl.once().await.expect("the clean once() completes");
        assert_eq!(
            pass.reorgs, 0,
            "harness: once() consumed the scripted CLEAN pass"
        );
        let mut want = vec![Some(1); streak];
        want.push(Some(0));
        assert_returned(&engine, &want);
        assert!(
            is_misbehaving(&ctl.status()),
            "P2-5: a pull-to-refresh under a reported streak must leave \
             Stalled {{ EndpointMisbehaving }} on glass — a clean once() is not a loop pass, \
             so it neither decays the streak nor takes the badge down (before the fix it \
             published UpToDate here, and the next loop pass put the stall back: the \
             flicker); the controller left {:?}",
            ctl.status()
        );
        let during_once: Vec<Vec<(String, String)>> =
            sink.records_of("wallet.sync")[before..].to_vec();
        assert!(
            has_streak_report(&during_once),
            "the once() pass logs the report line that says why the badge and its own \
             `outcome = \"ok\"` event disagree (gate 5): {during_once:?}"
        );
        assert_no_next_sleep(&during_once); // clause 3, on the once() under the loop
        ctl.stop().await;
    }

    #[tokio::test(start_paused = true)]
    async fn controller_stop_during_idle_poll_is_prompt() {
        // Real-world unstable-network/mobile case: the app backgrounds (or the host
        // calls stop) while the wallet is SYNCED and the loop is parked in the
        // POLL_INTERVAL sleep between polls. stop() must wake the interruptible sleep
        // PROMPTLY (an event, not the timer), never wait out the poll interval — the
        // same `interruptible_sleep` guards the post-fault backoff sleep. Asserted via
        // the virtual clock: ~no time elapses across stop().
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                pass: ok_pass(1),
            }],
        );
        let ctl = SyncController::new(engine);
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await; // loop now enters the POLL_INTERVAL sleep
        let before = tokio::time::Instant::now();
        ctl.stop().await;
        let elapsed = before.elapsed();
        assert!(
            elapsed < Duration::from_secs(POLL_INTERVAL_SECS),
            "stop() waited out the poll interval ({elapsed:?}) instead of waking promptly"
        );
        assert_eq!(ctl.status(), SyncStatus::Idle);
    }

    #[tokio::test]
    async fn controller_noop_pass_when_already_synced_reports_uptodate() {
        // Money/steady-state: a poll that finds nothing new (0 batches, no progress
        // report — the normal state of a synced wallet) must STILL surface UpToDate at
        // the tip, never leave the UI stuck on a stale status. The terminal status
        // comes from engine.tip(), not from a progress event.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![],
                pass: ok_pass(0),
            }],
        );
        let ctl = SyncController::new(engine);
        let pass = ctl.once().await.expect("pass ok");
        assert_eq!(pass.batches, 0);
        assert_eq!(ctl.status(), SyncStatus::UpToDate { tip: h(2_000_000) });
    }

    #[tokio::test]
    async fn controller_stop_is_prompt_and_idempotent() {
        // Stop must interrupt a WEDGED pass promptly (via the active-cancel), not
        // hang waiting for it, and be safe to call twice.
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Stuck {
                steps: vec![progress(2_000_000, 1_900_000, 0.7)],
            }],
        );
        let ctl = SyncController::new(engine);
        ctl.start();
        ctl.stop().await; // must return promptly even though the pass is wedged
        ctl.stop().await; // idempotent
        assert_eq!(ctl.status(), SyncStatus::Idle);
    }

    #[tokio::test]
    async fn controller_request_stop_alone_cancels_a_wedged_pass_and_ends_the_loop() {
        // `request_stop()` is the SYNCHRONOUS half of `stop()` that `Drop for Wallet`
        // calls (a Drop cannot `.await`). It must cancel an in-flight wedged pass +
        // signal the loop to exit WITHOUT a join — so a handle dropped without `close()`
        // does not wedge the single-writer lock. Proven over a `[Stuck, Ok]` script:
        // request_stop cancels the wedge and breaks the loop, so the Ok pass NEVER runs
        // (call_count stays 1). The later `stop().await` only joins the exiting task.
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Stuck {
                    steps: vec![progress(2_000_000, 1_900_000, 0.7)],
                },
                Plan::Ok {
                    steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                    pass: ok_pass(1),
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        // Wait until the wedged pass is demonstrably running (its Scanning report).
        loop {
            if matches!(&*sub.borrow_and_update(), SyncStatus::Scanning { .. }) {
                break;
            }
            sub.changed().await.expect("status stream alive");
        }
        ctl.request_stop(); // the sync half — NO await, NO join (the Drop path)
        ctl.stop().await; // join the already-exiting task
        assert_eq!(
            engine.call_count(),
            1,
            "request_stop cancelled the wedge + ended the loop before the Ok pass; got {}",
            engine.call_count()
        );
        assert_eq!(ctl.status(), SyncStatus::Idle);
    }

    #[tokio::test(start_paused = true)]
    async fn controller_double_start_is_idempotent() {
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Stuck {
                steps: vec![progress(2_000_000, 1_900_000, 0.7)],
            }],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        ctl.start(); // the running-check makes this a no-op — no second loop spawned
        // Wait until the wedged pass is demonstrably RUNNING (its Scanning report),
        // so the count below is deterministic rather than racing the loop's first poll.
        loop {
            if matches!(&*sub.borrow_and_update(), SyncStatus::Scanning { .. }) {
                break;
            }
            sub.changed().await.expect("status stream alive");
        }
        // Exactly ONE pass is in flight (the user-visible single-writer guarantee):
        // the single pass guard would serialize even a stray second loop, and the
        // `is_finished` running-check prevents that second loop existing in the first
        // place. Double-start neither double-scans nor panics.
        assert_eq!(engine.call_count(), 1);
        ctl.stop().await;
        assert_eq!(ctl.status(), SyncStatus::Idle);
    }

    #[tokio::test(start_paused = true)]
    async fn controller_restarts_cleanly_after_stop() {
        // stop() awaits the join and `take()`s `stop_tx`/`task` to None, so a SECOND
        // start() spawns a fresh loop that must converge again — the restart path the
        // `start()`/`stop()` doc promises.
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Ok {
                    steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                    pass: ok_pass(1),
                },
                Plan::Ok {
                    steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                    pass: ok_pass(1),
                },
            ],
        );
        let ctl = SyncController::new(engine.clone());
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        assert_eq!(ctl.status(), SyncStatus::Idle);
        // Restart — must converge again (the fields were left clean by stop()).
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        assert_eq!(ctl.status(), SyncStatus::Idle);
    }

    // ── §4u REW-1: rewinds never reset the backoff ladder, and a rewinding
    //    streak is published as the endpoint's ──────────────────────────────────
    //
    // The REQ-1-R fold review's row 1: `Ok(p) => { backoff = INITIAL_BACKOFF;
    // sleep(POLL_INTERVAL) }` hands a patient endpoint — one that stops under
    // the per-pass cap and lets the pass complete, or alternates a capped stall
    // with a completed rewinding pass — a fresh rewind budget every 20 s from
    // the 1 s rung, forever, and nothing counts rewinds across passes. Every row
    // here drives `run_loop` (the CONSUMER of the rule) over the `FakeEngine` on
    // the virtual clock: a scripted pass is instant there, so the gap between
    // two pass starts (`call_times`) IS the sleep the loop chose after the
    // first, and the streak is read off the status stream. `rewinding(n)`
    // scripts a pass that rewound `n` times and completed; `reorg_stall()` a
    // pass the cap or the storm bound ended; `clean()` a pass that rewound
    // nothing.
    //
    // ── RW-7: which row each of §4u's named mutants is expected to red ────────
    //
    // Recorded here at authoring time, BLIND to the implementer's half, so the
    // join can run each one and check the prediction rather than discover it.
    // One mutant per run (a batch of mutants credits the wrong killer —
    // the path-changing one stalls every pass and its siblings inherit the red).
    //
    // | §4u mutant                        | expected red                        |
    // |-----------------------------------|-------------------------------------|
    // | the reset moved back to every     | RW-1 clause 3 (`last >= floor`) AND |
    // | `Ok`                              | RW-2's climb clause; RW-4 clause 2  |
    // |                                   | follows through the real engine     |
    // | `max(POLL, backoff)` dropped —    | RW-1 clause 1 (`every spacing >=    |
    // | the rewinding pass sleeps the     | POLL_INTERVAL_SECS`); RW-5's        |
    // | bare rung                         | controller half also reds (its      |
    // |                                   | first spacing stops being 20 s)     |
    // | the streak counter removed        | RW-3 clause 2 (first seen == None)  |
    // |                                   | and RW-4 clause 4                   |
    // | the streak not cleared by a clean | RW-3's second half — the LAST        |
    // | pass                              | assert (the clean pass must restore |
    // |                                   | the terminal); RW-4 clause 5        |
    //
    // RW-5 is the CONTROL in both halves: no mutant above may red it except the
    // `max(POLL, backoff)` one, which reds it for the same reason it reds RW-1
    // clause 1. A mutant that reds RW-5's `EndpointMisbehaving` clause has made
    // one honest reorg look like a misbehaving endpoint — a false positive on
    // the surface, not a passing build.

    /// §4u's streak length, READ FROM THE SHIPPED CONSTANT. It was spelled `6`
    /// locally while the halves were blind (`constants.rs` did not carry it at
    /// the split base); **the join reconciled it** — here and in its twin in
    /// `wallet::tests` — so a maintainer decision on the number moves these rows
    /// with the code instead of leaving them asserting the old formula. A local
    /// copy that is never reconciled is a second source of truth: the
    /// REQ-1-R fold review's row 11, applied at the join this time rather than
    /// found by a reviewer after it.
    /// **Phase-3 P3-5 — a `once()` in the window the P2-5 fix named.** P2-5 moved
    /// the streak's raise UNDER the pass guard so a pull-to-refresh that takes the
    /// guard next reads the count the loop's pass produced; its own pin
    /// (`a_once_pass_records_no_next_sleep_and_carries_the_loops_streak`) runs the
    /// loop to rest first, so the guard-drop → publish window the fix was written
    /// for is never open there. Here it is open: the fake engine parks the LOOP
    /// task inside `record_synced` — the write-then-publish await, after its
    /// guard dropped and before its terminal is on glass — and a clean `once()`
    /// runs in that window on the multi-thread runtime. The report the loop's
    /// pass just earned must be what the once() publishes, and the loop's own
    /// publish, released afterwards, must carry the same judgement — the
    /// ordering residual `Shared::rewinding_streak`'s doc names resolves to two
    /// equal terminals here, never a stale downgrade.
    ///
    /// The precondition sets the controller's memory to one short of the report
    /// directly (P2-6: the streak is the controller's, not the loop's): the
    /// multi-thread runtime has no paused clock, and five real rungs are not a
    /// unit test. **What this does NOT discriminate, stated:** a raise moved out
    /// of the guard but placed BEFORE `emit_synced` — between the unlock and such
    /// a raise there is no await to park on, so a once() and the loop's arm would
    /// race, and a race is not a test (the multi-thread runtime makes the race
    /// real, not deterministic). It does discriminate a once() reading anything
    /// but the count the loop's pass produced: watched against
    /// `LoopVerdict::once(shared.streak())` → `…(shared.streak().saturating_sub(1))`
    /// in `emit_outcome` (a one-behind read — the pre-P2-5 symptom in this window).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_once_between_the_loops_guard_drop_and_its_publish_carries_the_count_that_pass_produced()
     {
        let engine = FakeEngine::new(2_000_000, vec![rewinding(1), clean()]);
        let (parked, release) = engine.park_record_synced(1);
        let ctl = SyncController::new(engine.clone());
        ctl.shared
            .rewinding_streak
            .store(MAX_CONSECUTIVE_REWINDING_PASSES - 1, Ordering::Relaxed);
        let mut sub = ctl.subscribe();
        ctl.start();
        tokio::time::timeout(Duration::from_secs(10), parked)
            .await
            .expect("the loop's first pass reached record_synced within 10 s")
            .expect("the park signal was sent");
        // The loop is between its guard drop and its publish: nothing on glass yet.
        assert!(
            !is_misbehaving(&ctl.status()),
            "precondition: the loop has not published its verdict yet (it is parked in \
             record_synced); the controller shows {:?}",
            ctl.status()
        );
        let pass = ctl
            .once()
            .await
            .expect("the clean once() completes while the loop is parked");
        assert_eq!(
            pass.reorgs, 0,
            "harness: once() consumed the scripted CLEAN pass"
        );
        assert!(
            is_misbehaving(&ctl.status()),
            "P3-5: a once() that took the guard right after the loop's pass dropped it must \
             carry the count THAT pass produced — the report — never the pre-pass count; \
             the controller left {:?}",
            ctl.status()
        );
        // Mark the once()'s publish seen, so the next `changed()` is the loop's own.
        let _ = sub.borrow_and_update();
        release.notify_one();
        tokio::time::timeout(Duration::from_secs(10), sub.changed())
            .await
            .expect("the loop published within 10 s of its release")
            .expect("status stream alive");
        assert!(
            is_misbehaving(&sub.borrow_and_update()),
            "the loop's own terminal, published after the once(), carries the same report — \
             the ordering residual resolves to two equal terminals, never a stale downgrade"
        );
        ctl.stop().await;
        assert_returned(&engine, &[Some(1), Some(0)]);
    }

    const EXPECTED_REWINDING_STREAK: usize = MAX_CONSECUTIVE_REWINDING_PASSES as usize;

    /// The rung the ladder stands on after `climbs` climbs from
    /// [`INITIAL_BACKOFF`] — through `next_backoff` itself, so the cap is the
    /// shipped one and never a literal here.
    fn ladder_after(climbs: usize) -> Duration {
        (0..climbs).fold(INITIAL_BACKOFF, |rung, _| next_backoff(rung))
    }

    /// How many climbs from the initial rung it takes for the ladder to reach
    /// `target` — the rows' plan lengths are derived from this, never typed.
    fn climbs_to_reach(target: Duration) -> usize {
        let mut climbs = 0;
        while ladder_after(climbs) < target {
            climbs += 1;
            assert!(
                climbs < 64,
                "the ladder never reaches {target:?}: the constants changed shape"
            );
        }
        climbs
    }

    /// A pass that rewound `reorgs` times and completed — no progress steps, so
    /// the only status the loop publishes for it is its terminal.
    fn rewinding(reorgs: u32) -> Plan {
        Plan::Ok {
            steps: vec![],
            pass: reorg_pass(1, reorgs),
        }
    }

    /// A pass that rewound nothing and completed.
    fn clean() -> Plan {
        Plan::Ok {
            steps: vec![],
            pass: ok_pass(1),
        }
    }

    /// A pass the per-pass cap or the storm bound ended.
    fn reorg_stall() -> Plan {
        Plan::Err(WalletError::Sync {
            stall: StallReason::ChainReorg,
        })
    }

    /// A pass that REWOUND (its progress samples say so) and then died of a
    /// TRANSPORT fault — the endpoint dropped the stream after taking its
    /// truncates. The stall reason is `EndpointUnreachable`, so a loop that
    /// decides "did this pass rewind" from the reason alone sees nothing.
    fn rewound_then_dropped() -> Plan {
        Plan::ErrAfter {
            steps: vec![SyncProgress {
                tip: h(2_000_000),
                scanned_to: h(1_999_000),
                percent: 0.5,
                spendable_ready: false,
                rewound: true,
            }],
            err: WalletError::Sync {
                stall: StallReason::EndpointUnreachable,
            },
        }
    }

    fn is_misbehaving(status: &SyncStatus) -> bool {
        matches!(
            status,
            SyncStatus::Stalled {
                reason: StallReason::EndpointMisbehaving
            }
        )
    }

    /// Wait until the engine has been asked for `passes` passes — the
    /// `passes`-th has STARTED, so its start instant is in `call_times`. Wakes
    /// on each status the loop publishes (every scripted pass publishes at
    /// least its terminal), and fails rather than hangs if the loop goes silent
    /// for a day of virtual time: a dead loop task idles the runtime, and the
    /// paused clock then jumps straight to this bound.
    async fn wait_for_pass_starts(
        engine: &FakeEngine,
        sub: &mut watch::Receiver<SyncStatus>,
        passes: usize,
    ) {
        loop {
            let _ = sub.borrow_and_update();
            if engine.call_count() >= passes {
                return;
            }
            tokio::time::timeout(Duration::from_secs(24 * 3600), sub.changed())
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "the loop went silent after {} of {passes} passes",
                        engine.call_count()
                    )
                })
                .expect("status stream alive");
        }
    }

    /// Run the loop until `passes` passes have started, then stop it.
    async fn run_passes(ctl: &SyncController, engine: &FakeEngine, passes: usize) {
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_pass_starts(engine, &mut sub, passes).await;
        ctl.stop().await;
    }

    /// Run the loop for `passes` passes and return the status the loop left
    /// standing after EACH of them, in order — its last word on that pass,
    /// whichever way an implementation orders a stall behind a terminal: once
    /// the pass has started this task yields a few times so the loop finishes
    /// the pass (on the paused clock the NEXT pass cannot start until this task
    /// is idle, and a yielded task is not idle), then reads what stands.
    async fn statuses_after_each_pass(
        ctl: &SyncController,
        engine: &FakeEngine,
        passes: usize,
    ) -> Vec<SyncStatus> {
        let mut sub = ctl.subscribe();
        let mut out = Vec::with_capacity(passes);
        ctl.start();
        while out.len() < passes {
            wait_for_pass_starts(engine, &mut sub, out.len() + 1).await;
            for _ in 0..8 {
                tokio::task::yield_now().await;
            }
            out.push(sub.borrow_and_update().clone());
        }
        ctl.stop().await;
        out
    }

    /// The gaps between consecutive pass starts, in whole seconds — spacing `i`
    /// (0-based) is the sleep the loop chose after pass `i + 1`.
    fn spacings_secs(times: &[tokio::time::Instant]) -> Vec<u64> {
        times.windows(2).map(|w| (w[1] - w[0]).as_secs()).collect()
    }

    /// The rows' PRECONDITION: the first passes the engine returned were the
    /// scripted ones (`want`), not the exhausted script's no-op — a plan
    /// shortened by an editor fails HERE, on the harness, and never on the
    /// clause the row exists to red (§4u anti-vacuity).
    fn assert_returned(engine: &FakeEngine, want: &[Option<u32>]) {
        let got = engine.returned_reorgs();
        assert!(
            got.len() >= want.len() && got[..want.len()] == *want,
            "precondition: the engine must have returned the scripted passes {want:?} first \
             (Some(reorgs) = a completed pass, None = a stall); it returned {got:?}"
        );
    }

    /// **§4u RW-1 — DEFECT row, RED at the base.** A pass that rewound and
    /// completed is not a clean pass: it must CLIMB the ladder, never reset it.
    ///
    /// The plan is `MAX_TOTAL_REORGS_PER_PASS` rewinds per pass — a pass that
    /// spent its whole budget and still completed — for as many passes as the
    /// ladder needs to climb from the poll interval to its cap and be seen there
    /// (`climbs_to_reach(cap) + 2`: under the `Err` arm's own order, sleep then
    /// climb, the sleep after pass `j` reads the rung after `j − 1` climbs, and
    /// the last spacing this row can see is the one after the second-to-last
    /// pass). Asserted on the spacings between consecutive pass starts:
    /// 1. every spacing ≥ `POLL_INTERVAL_SECS` — a rewinding pass never polls
    ///    FASTER than a clean one (the `max(POLL, backoff)` clause; the mutant
    ///    that sleeps the bare backoff reds here);
    /// 2. the spacings are non-decreasing — the ladder never comes back down
    ///    while the rewinds continue;
    /// 3. the last spacing ≥ the smaller of `SYNC_BACKOFF_MAX_SECS` and the
    ///    rung after `passes − 2` climbs — the ladder actually climbed (the
    ///    defect clause; the reset moved back to every `Ok` reds here).
    ///
    /// At the base every spacing is `POLL_INTERVAL_SECS`: clauses 1 and 2 hold
    /// vacuously and clause 3 prints eleven 20 s gaps.
    #[tokio::test(start_paused = true)]
    async fn a_rewinding_pass_never_resets_the_ladder() {
        let cap_rewinds = crate::constants::MAX_TOTAL_REORGS_PER_PASS;
        let cap = Duration::from_secs(SYNC_BACKOFF_MAX_SECS);
        let passes = climbs_to_reach(cap) + 2;
        let plan: Vec<Plan> = (0..passes).map(|_| rewinding(cap_rewinds)).collect();
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        run_passes(&ctl, &engine, passes).await;

        assert_returned(&engine, &vec![Some(cap_rewinds); passes]);
        let times = engine.call_times();
        assert!(
            times.len() >= passes,
            "harness: {passes} passes must have started; got {}",
            times.len()
        );
        let spacings = spacings_secs(&times[..passes]);
        assert!(
            spacings.iter().all(|&s| s >= POLL_INTERVAL_SECS),
            "§4u: a rewinding pass must never poll FASTER than a clean one — the sleep after it \
             is max(POLL_INTERVAL, backoff); spacings {spacings:?} s"
        );
        assert!(
            spacings.windows(2).all(|w| w[1] >= w[0]),
            "§4u: the ladder never comes back down while the rewinds continue; spacings \
             {spacings:?} s"
        );
        let floor = ladder_after(passes - 2).min(cap).as_secs();
        let last = *spacings.last().expect("passes − 1 spacings");
        assert!(
            last >= floor,
            "§4u RW-1: {passes} consecutive passes each rewound {cap_rewinds} times (the whole \
             per-pass budget) and completed, and the loop's inter-pass spacing never left the \
             poll interval — spacings {spacings:?} s — because `Ok` resets the ladder to the \
             initial rung whatever the pass rewound, so a patient endpoint is handed a fresh \
             budget every {POLL_INTERVAL_SECS} s forever. Expected the last spacing ≥ {floor} s \
             (the rung after {} climbs, capped at SYNC_BACKOFF_MAX_SECS = {SYNC_BACKOFF_MAX_SECS})",
            passes - 2
        );
    }

    /// **§4u RW-2 — DEFECT row, RED at the base.** An endpoint that alternates
    /// a capped stall with a completed rewinding pass cannot use the completed
    /// pass to reset the ladder; and a CLEAN pass steps it down ONE RUNG, never
    /// back to the initial one.
    ///
    /// `[Err(Sync { ChainReorg }), Ok { reorgs: cap }] × 4`: the sleep after
    /// each stall (read as the spacing to the next pass start) never decreases
    /// across the cycles and the last one is ABOVE the poll interval — the
    /// ladder climbed and nothing reset it. It cannot be asserted as a STRICT
    /// climb any more: since the REW-1 fold review's row 1 a rewinding STALL
    /// takes the same `max(POLL_INTERVAL, rung)` floor a rewinding `Ok` takes,
    /// so the rungs at or below the poll (1, 2, 4, 8, 16 s) all read 20 s and
    /// the climb only becomes visible once the ladder passes the poll.
    ///
    /// Then the CONTROL inside the row, on its own engine, and it is the half
    /// the review's row 1 inverted: `[Err × climbs, Ok { reorgs: 0 }, Err]`,
    /// with `climbs` chosen so the ladder stands ABOVE the poll interval when
    /// the clean pass arrives. The stall after the clean pass must sleep ONE
    /// RUNG DOWN — which is the same rung the stall before it slept — and must
    /// NOT sleep the initial rung. This row used to assert the opposite
    /// (`spacings[2] == spacings[0] == SYNC_BACKOFF_INITIAL_SECS`): it pinned
    /// the free reset as intended behaviour, and a clean pass is free — hold
    /// the tip still and the pass ends `Ok` with no batches and no rewinds.
    ///
    /// At the base (the reset) the rewinding `Ok` resets the ladder every cycle,
    /// so every post-stall sleep is the initial rung and the first half prints
    /// `[1, 1, 1, 1]`; the control's post-clean stall sleeps the initial rung
    /// instead of one rung down.
    #[tokio::test(start_paused = true)]
    async fn an_alternating_endpoint_cannot_reset_the_ladder_with_a_rewinding_ok_pass() {
        const CYCLES: usize = 4;
        let cap_rewinds = crate::constants::MAX_TOTAL_REORGS_PER_PASS;
        let plan: Vec<Plan> = (0..CYCLES)
            .flat_map(|_| [reorg_stall(), rewinding(cap_rewinds)])
            .collect();
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        run_passes(&ctl, &engine, 2 * CYCLES).await;

        let want: Vec<Option<u32>> = (0..CYCLES)
            .flat_map(|_| [None, Some(cap_rewinds)])
            .collect();
        assert_returned(&engine, &want);
        let times = engine.call_times();
        assert!(
            times.len() >= 2 * CYCLES,
            "harness: {} passes must have started; got {}",
            2 * CYCLES,
            times.len()
        );
        let spacings = spacings_secs(&times[..2 * CYCLES]);
        let after_stall: Vec<u64> = (0..CYCLES).map(|k| spacings[2 * k]).collect();
        let after_rewinding_ok: Vec<u64> = (0..CYCLES - 1).map(|k| spacings[2 * k + 1]).collect();
        assert!(
            after_rewinding_ok.iter().all(|&s| s >= POLL_INTERVAL_SECS),
            "§4u: the sleep after a completed rewinding pass is never below the poll interval; \
             after each rewinding Ok: {after_rewinding_ok:?} s (all spacings {spacings:?})"
        );
        assert!(
            after_stall.windows(2).all(|w| w[1] >= w[0]),
            "§4u RW-2: an endpoint alternating a capped stall with a completed rewinding pass — \
             the backoff sleep after each stall must never come back DOWN across the cycles, and \
             it read {after_stall:?} s (all spacings {spacings:?}): the rewinding Ok pass between \
             two stalls reset the ladder to the initial rung, so the endpoint is back on the \
             bottom rung every other pass, forever"
        );
        let last_stall = *after_stall.last().expect("one spacing per cycle");
        assert!(
            last_stall > POLL_INTERVAL_SECS,
            "§4u RW-2: after {CYCLES} cycles the ladder must have climbed PAST the poll interval \
             — a rewinding pass that never resets it reaches {} s by then. It read \
             {after_stall:?} s (all spacings {spacings:?}), which is the ladder standing still: \
             the rewinding Ok pass reset it every cycle",
            ladder_after(2 * CYCLES - 1).as_secs()
        );

        // The control, on its own engine, and the half the REW-1 fold review's
        // row 1 INVERTED: a CLEAN pass between two stalls steps the ladder down
        // ONE RUNG — it does not reset it. Climb first, because every rung at or
        // below the poll interval reads as the poll and a down-step among them
        // is invisible.
        let climbs = climbs_to_reach(Duration::from_secs(POLL_INTERVAL_SECS)) + 2;
        let rung_before_clean = ladder_after(climbs - 1);
        assert!(
            rung_before_clean > Duration::from_secs(POLL_INTERVAL_SECS),
            "harness: the control needs the ladder ABOVE the poll interval when the clean pass \
             arrives, else a one-rung step down cannot be seen; it stands at \
             {rung_before_clean:?}"
        );
        let mut plan: Vec<Plan> = (0..climbs).map(|_| reorg_stall()).collect();
        plan.push(clean());
        plan.push(reorg_stall());
        let passes = plan.len();
        let control = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(control.clone());
        run_passes(&ctl, &control, passes + 1).await;
        let mut want: Vec<Option<u32>> = vec![None; climbs];
        want.push(Some(0));
        want.push(None);
        assert_returned(&control, &want);
        let spacings = spacings_secs(&control.call_times()[..passes + 1]);
        assert_eq!(
            spacings[climbs], POLL_INTERVAL_SECS,
            "the control: a clean pass polls, whatever the ladder stands on; spacings \
             {spacings:?} s"
        );
        assert_eq!(
            spacings[climbs + 1],
            rung_before_clean.as_secs(),
            "§4u RW-2, the control: the clean pass steps the ladder DOWN ONE RUNG, so the stall \
             after it sleeps what the stall BEFORE it slept ({} s) — not the initial rung. A \
             clean pass is free (hold the tip still and the pass ends Ok with no batches and no \
             rewinds), so a reset here hands the whole ladder back for nothing; spacings \
             {spacings:?} s",
            rung_before_clean.as_secs()
        );
        assert_ne!(
            spacings[climbs + 1],
            SYNC_BACKOFF_INITIAL_SECS,
            "…and specifically NOT the initial rung, which is what the reset gave; spacings \
             {spacings:?} s"
        );
    }

    /// **§4u RW-8 — DEFECT row, RED at the base (the REW-1 fold review's row 1,
    /// arch half).** An endpoint that alternates a cap-hit stall with one clean
    /// pass never gets a pass faster than the poll interval.
    ///
    /// `[Err(Sync { ChainReorg }), Ok { reorgs: 0 }] × 4`, and every spacing —
    /// the stall's as much as the clean pass's — must be at least
    /// `POLL_INTERVAL_SECS`. At the base the stall arm sleeps the BARE rung and
    /// the clean pass hands the rung back, so the spacings read `[1, 20, 1, 20,
    /// …]`: a full `MAX_TOTAL_REORGS_PER_PASS` rewind budget every ~21 s, which
    /// is the pre-fix defect's own rate. The killer for the `rewinding_sleep`
    /// call on the `Err` arm — RW-1 drives only completed passes and RW-2's
    /// control is blind to it, because both of its stalls sleep the same rung
    /// whether or not the floor is there.
    ///
    /// **The residual this row does NOT close, stated and deliberately not
    /// pinned**: a 1:1 alternation nets zero on the streak (one rewinding pass
    /// up, one clean pass down), so it never reaches
    /// `MAX_CONSECUTIVE_REWINDING_PASSES` and the badge never changes. What the
    /// decay and this floor buy is the RATE — from ~21 s per budget to ~40 s,
    /// and the badge does fire on anything that rewinds more often than it
    /// behaves (fork five, freeze one: +4 per six passes). Pinning the evasion
    /// as expected behaviour is what RW-2's control used to do, so it is
    /// recorded here instead of asserted.
    #[tokio::test(start_paused = true)]
    async fn an_alternating_stall_and_clean_pass_never_polls_faster_than_the_poll() {
        const CYCLES: usize = 4;
        let plan: Vec<Plan> = (0..CYCLES).flat_map(|_| [reorg_stall(), clean()]).collect();
        let passes = plan.len();
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        run_passes(&ctl, &engine, passes).await;

        let want: Vec<Option<u32>> = (0..CYCLES).flat_map(|_| [None, Some(0)]).collect();
        assert_returned(&engine, &want);
        let spacings = spacings_secs(&engine.call_times()[..passes]);
        assert!(
            spacings.iter().all(|&s| s >= POLL_INTERVAL_SECS),
            "§4u RW-8: a cap-hit stall alternated with one clean pass must never hand the \
             endpoint a pass faster than the poll interval — the stall arm sleeps \
             max(POLL_INTERVAL, rung) like a rewinding Ok pass does. It read {spacings:?} s: the \
             stall slept the bare rung and the clean pass gave the rung back, so the whole \
             {}-rewind budget is available again every {} s — the defect's own rate",
            crate::constants::MAX_TOTAL_REORGS_PER_PASS,
            spacings.first().copied().unwrap_or(0) + POLL_INTERVAL_SECS
        );
    }

    /// **§4u RW-9 — DEFECT row, RED before the review repair.** A pass that
    /// rewound and then died of a TRANSPORT fault is a rewinding pass.
    ///
    /// The security angle's HIGH, in its own words: the `Err` arm decided "did
    /// this pass rewind" from the STALL REASON, not from work actually done. So
    /// an endpoint could take up to `MAX_TOTAL_REORGS_PER_PASS` truncates and
    /// then drop the stream — `EndpointUnreachable`, not `ChainReorg` — and the
    /// pass counted as non-rewinding: no streak increment, and no
    /// `rewinding_sleep` floor, so it slept the BARE rung. One free clean pass
    /// then stepped the ladder back down (`prev_backoff`) and the cycle repeats
    /// at ~21 s — the pre-fix defect's own rate, reached through a door RW-8 does
    /// not drive (RW-8 scripts `reorg_stall()`, whose reason IS `ChainReorg`).
    ///
    /// `[rewound-then-dropped, clean] × 4`, and every spacing must be at least
    /// the poll interval. At the base the fault arm sleeps `[1, 20, 1, 20, …]`.
    /// The streak clause is the other half: four rewinding passes against four
    /// clean ones nets zero, so the badge is not expected — what is expected is
    /// that the count MOVED, which the ladder proves by climbing.
    #[tokio::test(start_paused = true)]
    async fn a_pass_that_rewound_then_faulted_is_a_rewinding_pass() {
        const CYCLES: usize = 4;
        let plan: Vec<Plan> = (0..CYCLES)
            .flat_map(|_| [rewound_then_dropped(), clean()])
            .collect();
        let passes = plan.len();
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        run_passes(&ctl, &engine, passes).await;

        let spacings = spacings_secs(&engine.call_times()[..passes]);
        assert!(
            spacings.iter().all(|&s| s >= POLL_INTERVAL_SECS),
            "§4u RW-9: a pass that rewound and THEN faulted must take the rewinding floor like \
             one the cap stopped — the ladder's question is what the pass DID, not how it \
             exited. It read {spacings:?} s: the fault arm slept the bare rung, so an endpoint \
             takes its whole truncate budget and is met again a second later"
        );
    }

    /// **§4u RW-3 — DEFECT row, RED at the base.** A rewinding STREAK is the
    /// endpoint's, and the surface says so until the endpoint has behaved as
    /// often as it misbehaved above the threshold.
    ///
    /// `EXPECTED_REWINDING_STREAK` passes that each rewound once and completed,
    /// one more of the same, then TWO clean passes. Asserted on the status the
    /// loop leaves standing after each pass:
    /// 1. every pass before the streak is reached ends on its terminal —
    ///    `UpToDate` — never on the stall early;
    /// 2. `Stalled { EndpointMisbehaving }` is FIRST left standing after exactly
    ///    the `EXPECTED_REWINDING_STREAK`-th rewinding pass — the printed value
    ///    is the pass it was first seen after, so a streak constant moved by
    ///    one moves the line;
    /// 3. one more rewinding pass keeps it standing — a pass that rewound does
    ///    not clear the streak;
    /// 4. the FIRST clean pass does NOT clear it — it decays the count by one
    ///    and the count is still at the report (the REW-1 fold review's row 1:
    ///    a clean pass is free, so clearing on one handed the badge back for
    ///    nothing);
    /// 5. the SECOND clean pass decays it below the report: the terminal again.
    ///
    /// At the base the loop publishes `UpToDate` after every one of the passes —
    /// nothing counts rewinds across passes — and clause 2 prints "first seen
    /// after None".
    #[tokio::test(start_paused = true)]
    async fn a_rewinding_streak_is_published_as_endpoint_misbehaving_until_a_clean_pass() {
        let streak = EXPECTED_REWINDING_STREAK;
        let mut plan: Vec<Plan> = (0..=streak).map(|_| rewinding(1)).collect();
        // TWO clean passes, because a clean pass DECAYS the streak by one since
        // the REW-1 fold review's row 1. The count after `streak + 1` rewinding
        // passes is `streak + 1`; it must fall below `streak` to clear, so it
        // takes exactly two — and that is two whatever the constant is.
        plan.push(clean());
        plan.push(clean());
        let passes = plan.len();
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        let statuses = statuses_after_each_pass(&ctl, &engine, passes).await;

        let mut want: Vec<Option<u32>> = vec![Some(1); streak + 1];
        want.push(Some(0));
        want.push(Some(0));
        assert_returned(&engine, &want);
        assert_eq!(statuses.len(), passes, "harness: one status per pass");
        for (i, status) in statuses[..streak - 1].iter().enumerate() {
            assert!(
                matches!(status, SyncStatus::UpToDate { .. }),
                "before the streak is reached a rewinding pass ends on its terminal; after pass \
                 {} the loop left {status:?} standing. All: {statuses:?}",
                i + 1
            );
        }
        let first_seen = statuses.iter().position(is_misbehaving).map(|i| i + 1);
        assert_eq!(
            first_seen,
            Some(streak),
            "§4u RW-3: after {streak} consecutive rewinding passes the published status must be \
             Stalled {{ EndpointMisbehaving }} — the streak is the endpoint's, and the copy says \
             switch servers — and the loop first left it standing after pass {first_seen:?} \
             (None = never: nothing counts rewinds across passes, and every rewinding pass ends \
             `UpToDate`). Statuses after each pass: {statuses:?}"
        );
        assert!(
            is_misbehaving(&statuses[streak]),
            "one more rewinding pass keeps the stall standing; after pass {} the loop left {:?}",
            streak + 1,
            statuses[streak]
        );
        assert!(
            is_misbehaving(&statuses[streak + 1]),
            "§4u RW-3, the REW-1 fold review's row 1: ONE clean pass does NOT clear the streak — \
             it decays it by one, and the count stands at {} which is still at or above the \
             report. A clean pass is free (hold the tip still and the pass ends Ok with no \
             batches and no rewinds), so clearing on one handed a forking endpoint the badge \
             back for nothing. After pass {} the loop left {:?}. All: {statuses:?}",
            streak,
            streak + 2,
            statuses[streak + 1]
        );
        assert!(
            matches!(statuses[streak + 2], SyncStatus::UpToDate { .. }),
            "…and the SECOND clean pass decays it below the report, so the terminal returns; \
             after pass {} the loop left {:?}. All: {statuses:?}",
            streak + 3,
            statuses[streak + 2]
        );
    }

    /// **Phase-2 P2-6, maintainer decision 5 — the reported streak ranks BELOW
    /// the build's claim and the grace's, running or clock-ended.** Under a
    /// streak the user used to lose `UpToDateUnverified { grace }`'s countdown
    /// and `Ended { by: Clock }`'s one actionable remedy (the device clock), and
    /// a stale build was told "switch servers" before "update the app". Four
    /// cases over the SAME plan (`streak + 1` rewinding passes): the control
    /// (nothing above the streak) publishes `Stalled { EndpointMisbehaving }` —
    /// the precondition that this plan reaches the report, so the other three
    /// cannot pass vacuously; a stale build publishes `UpToDateLimited`; a
    /// running grace and a clock-ended grace each publish `UpToDateUnverified`
    /// carrying THAT grace. Under the grace the report line
    /// (`outcome = "rewinding_streak"`) is still logged: the judgement stands,
    /// only the glass shows the higher claim.
    ///
    /// Mutant (registry row): the arm order restored — the streak arm first in
    /// `emit_synced`'s ranking — reds the three non-control cases; the control
    /// stays green under it, which is why it is the control.
    #[tokio::test(start_paused = true)]
    async fn a_reported_streak_ranks_below_a_stale_build_and_a_grace_running_or_clock_ended() {
        use crate::state::GraceExpiry;
        use crate::tracing_guard::{CaptureLayer, CapturedEvents, force_wallet_callsites_enabled};
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);
        let report_lines = |records: &[Vec<(String, String)>]| {
            records
                .iter()
                .filter(|r| {
                    r.iter()
                        .any(|(n, v)| n == "outcome" && v == "rewinding_streak")
                })
                .count()
        };

        let streak = EXPECTED_REWINDING_STREAK;
        let running = UnknownBranchGrace::Running {
            blocks_left: 40,
            secs_left: Some(3_000),
        };
        let clock_ended = UnknownBranchGrace::Ended {
            by: GraceExpiry::Clock,
            blocks_since_last_current: Some(12),
        };
        // (the build can read the chain, the grace the engine reports, the case's name)
        let cases: [(bool, Option<UnknownBranchGrace>, &str); 4] = [
            (true, None, "control: nothing outranks the streak"),
            (false, None, "a stale build"),
            (true, Some(running), "a running grace"),
            (true, Some(clock_ended), "a clock-ended grace"),
        ];
        for (interpretable, grace, name) in cases {
            let plan: Vec<Plan> = (0..=streak).map(|_| rewinding(1)).collect();
            let inner = FakeEngine::new(2_000_000, plan);
            let ctl = SyncController::new(Arc::new(Claiming {
                inner: inner.clone(),
                interpretable,
                grace,
            }));
            let before = report_lines(&sink.records_of("wallet.sync"));
            let statuses = statuses_after_each_pass(&ctl, &inner, streak + 1).await;
            let want: Vec<Option<u32>> = vec![Some(1); streak + 1];
            assert_returned(&inner, &want);
            let last = statuses
                .last()
                .expect("harness: one status per pass")
                .clone();
            let reported_after = report_lines(&sink.records_of("wallet.sync")) - before;
            assert!(
                reported_after >= 2,
                "{name}: passes {streak} and {} both reach the report threshold, so the loop \
                 logs the report line at least twice whatever the glass shows (P2-6: the \
                 judgement stands under a higher claim); logged {reported_after} time(s)",
                streak + 1
            );
            match (interpretable, grace) {
                (true, None) => assert!(
                    is_misbehaving(&last),
                    "{name}: after {} rewinding passes the streak is at the report and \
                     nothing outranks it — the precondition for the other three cases; the \
                     loop left {last:?}",
                    streak + 1
                ),
                (false, None) => assert_eq!(
                    last,
                    SyncStatus::UpToDateLimited { tip: h(2_000_000) },
                    "{name}: P2-6 decision 5 — a build that cannot read the chain cannot \
                     act on \"switch servers\" until it is updated, so `UpToDateLimited` \
                     outranks the reported streak; the loop left {last:?}"
                ),
                (true, Some(g)) => match last {
                    SyncStatus::UpToDateUnverified { grace, tip, .. } => {
                        assert_eq!(grace, g, "{name}: the grace published is the engine's");
                        assert_eq!(tip, h(2_000_000), "{name}: the pass's tip rides with it");
                    }
                    other => panic!(
                        "{name}: P2-6 decision 5 — the grace's countdown (running) and the \
                         clock remedy (`Ended {{ by: Clock }}`) outrank the reported streak, \
                         so `UpToDateUnverified` is published over it; the loop left {other:?}"
                    ),
                },
                (false, Some(_)) => unreachable!("no such case"),
            }
        }
    }

    /// **Phase-3 P3-12, maintainer (Q2 option 2) — the grace CARRIES the
    /// streak's report.** P2-6 ranked the grace above `Stalled {
    /// EndpointMisbehaving }` so the countdown is never hidden; the cost was
    /// that the glass then said "your balance is current" over a server the
    /// loop had judged misbehaving. Two cases over the SAME running grace: the
    /// control (one clean pass, no streak) publishes `UpToDateUnverified` with
    /// `streak_reported: false`; `streak + 1` rewinding passes publish it with
    /// `streak_reported: true` — the same claim, now telling the glass what it
    /// outranked. That this plan reaches the report is pinned by the P2-6 row
    /// above (its control case, on the same plan).
    ///
    /// Mutant (registry row): `streak_reported: false` written as a constant in
    /// `emit_synced`'s grace arm — the streak case reds on the flag; the
    /// control stays green, which is why it is the control.
    #[tokio::test(start_paused = true)]
    async fn a_grace_over_a_reported_streak_carries_the_report() {
        let streak = EXPECTED_REWINDING_STREAK;
        let running = UnknownBranchGrace::Running {
            blocks_left: 40,
            secs_left: Some(3_000),
        };
        let cases: [(Vec<Plan>, bool, &str); 2] = [
            (vec![clean()], false, "control: one clean pass, no streak"),
            (
                (0..=streak).map(|_| rewinding(1)).collect(),
                true,
                "a reported streak under a running grace",
            ),
        ];
        for (plan, want_reported, name) in cases {
            let passes = plan.len();
            let inner = FakeEngine::new(2_000_000, plan);
            let ctl = SyncController::new(Arc::new(Claiming {
                inner: inner.clone(),
                interpretable: true,
                grace: Some(running),
            }));
            let statuses = statuses_after_each_pass(&ctl, &inner, passes).await;
            let last = statuses
                .last()
                .expect("harness: one status per pass")
                .clone();
            match last {
                SyncStatus::UpToDateUnverified {
                    grace,
                    streak_reported,
                    ..
                } => {
                    assert_eq!(
                        grace, running,
                        "{name}: the grace published is the engine's"
                    );
                    assert_eq!(
                        streak_reported, want_reported,
                        "{name}: P3-12 — the grace claim carries whether the loop's streak \
                         has reached its report (the `Stalled {{ EndpointMisbehaving }}` it \
                         outranks); got {streak_reported}, want {want_reported}"
                    );
                }
                other => panic!(
                    "{name}: a running grace outranks everything but the build's claim; \
                     the loop left {other:?}"
                ),
            }
        }
    }

    /// **Phase-2 P2-6, maintainer decision 6 — the streak SURVIVES a
    /// `stop()`/`start()`; only clean LOOP passes decay it.** The host's reconnect
    /// kick answers every `Stalled` — `EndpointMisbehaving` included — with
    /// exactly that pair, so with the loop zeroing the count at its top (the
    /// P2-5 shape) a reachability tick or "Try now" forgave the whole report.
    /// Plan: `streak` rewinding passes on the first loop (the report stands), a
    /// `stop()` (Idle on glass), a `start()`, ONE more rewinding pass — the
    /// report must still stand — then two clean passes: the first decays the
    /// count to exactly the threshold (still reported), the second takes it
    /// below (the terminal returns). The engine's call counter is cumulative
    /// across the restart, so `assert_returned` pins that every pass the loops
    /// consumed was the scripted one, in order.
    ///
    /// Mutant (registry row): `shared.rewinding_streak.store(0, ..)` restored at
    /// the top of `run_loop` — after the restart the count is 1 and the badge
    /// reads `UpToDate`; the first assertion after `start()` reds.
    #[tokio::test(start_paused = true)]
    async fn the_streak_survives_a_stop_and_start_and_only_clean_loop_passes_decay_it() {
        let streak = EXPECTED_REWINDING_STREAK;
        let mut plan: Vec<Plan> = (0..streak).map(|_| rewinding(1)).collect();
        plan.push(rewinding(1)); // the first pass of the SECOND loop
        plan.push(clean());
        plan.push(clean());
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());

        // Loop 1: drive the streak to the report, then `stop()` (the harness stops).
        let first = statuses_after_each_pass(&ctl, &engine, streak).await;
        assert!(
            first.last().is_some_and(is_misbehaving),
            "precondition: after {streak} rewinding passes the report stands; the first \
             loop left {:?}",
            first.last()
        );
        assert_eq!(
            ctl.status(),
            SyncStatus::Idle,
            "harness: `stop()` published Idle, so the next status is the second loop's"
        );

        // Loop 2: the reconnect kick / "Try now" shape — one more rewinding pass.
        let sub = ctl.subscribe();
        ctl.start();
        let settle = |n: usize| {
            let engine = engine.clone();
            let mut sub = sub.clone();
            async move {
                wait_for_pass_starts(&engine, &mut sub, n).await;
                for _ in 0..8 {
                    tokio::task::yield_now().await;
                }
            }
        };
        settle(streak + 1).await;
        let mut want = vec![Some(1); streak + 1];
        assert_returned(&engine, &want);
        assert!(
            is_misbehaving(&ctl.status()),
            "P2-6 decision 6: the streak survives `stop()`/`start()` — a restart the stall \
             itself triggered (the reconnect kick, \"Try now\") must not hand the endpoint \
             a fresh rewind budget; with the loop zeroing the count at its top the count is \
             1 after the restart and the badge reads `UpToDate`. The second loop left {:?}",
            ctl.status()
        );

        // Only clean LOOP passes decay it: one takes the count to the threshold
        // (still reported), the second below it.
        settle(streak + 2).await;
        want.push(Some(0));
        assert_returned(&engine, &want);
        assert!(
            is_misbehaving(&ctl.status()),
            "one clean pass after the restart decays the count from {} to {streak}, which is \
             still at the report; the loop left {:?}",
            streak + 1,
            ctl.status()
        );
        settle(streak + 3).await;
        want.push(Some(0));
        assert_returned(&engine, &want);
        assert!(
            matches!(ctl.status(), SyncStatus::UpToDate { .. }),
            "…and the second clean pass takes it below the report, so the terminal returns \
             — the decay, not the restart, is what forgives; the loop left {:?}",
            ctl.status()
        );
        ctl.stop().await;
    }

    /// **§4u RW-5 — CONTROL, green at the base and under the rule.** One honest
    /// reorg is forgiven by the next clean pass: `[Ok { reorgs: 1 }, Ok { 0 },
    /// Ok { 0 }]` — every sleep is exactly the poll interval (the one rewinding
    /// pass sleeps `max(POLL, rung)` with the rung far below the poll interval,
    /// and the clean passes reset whatever it climbed), and no pass is ever
    /// published as `EndpointMisbehaving`. The engine half of this row — the
    /// production pass body under a `Fork` — is
    /// `wallet::tests::one_honest_reorg_is_forgiven_by_the_next_clean_pass`.
    #[tokio::test(start_paused = true)]
    async fn one_honest_reorg_is_forgiven_by_the_next_clean_pass() {
        let engine = FakeEngine::new(2_000_000, vec![rewinding(1), clean(), clean()]);
        let ctl = SyncController::new(engine.clone());
        let statuses = statuses_after_each_pass(&ctl, &engine, 4).await;

        assert_returned(&engine, &[Some(1), Some(0), Some(0)]);
        let spacings = spacings_secs(&engine.call_times()[..4]);
        assert_eq!(
            spacings,
            vec![POLL_INTERVAL_SECS; 3],
            "one honest reorg then clean passes: every sleep is the poll interval — the third \
             spacing in particular, the ladder having been reset by the first clean pass; got \
             {spacings:?} s"
        );
        assert!(
            statuses.iter().all(|s| !is_misbehaving(s)),
            "one honest reorg is never `EndpointMisbehaving`; statuses after each pass: \
             {statuses:?}"
        );
        assert!(
            statuses
                .iter()
                .all(|s| matches!(s, SyncStatus::UpToDate { .. })),
            "every pass ends on its terminal; statuses after each pass: {statuses:?}"
        );
    }

    // ── the pure decision helpers (boundary-tested, fixture-free) ────────────

    #[test]
    fn next_backoff_doubles_then_caps_at_max() {
        assert_eq!(next_backoff(Duration::from_secs(1)), Duration::from_secs(2));
        assert_eq!(next_backoff(Duration::from_secs(2)), Duration::from_secs(4));
        assert_eq!(
            next_backoff(Duration::from_secs(256)),
            Duration::from_secs(512)
        );
        // 512 → 1024 clamps to the cap; the cap is a fixed point.
        assert_eq!(
            next_backoff(Duration::from_secs(256 * 2)),
            Duration::from_secs(SYNC_BACKOFF_MAX_SECS)
        );
        assert_eq!(
            next_backoff(Duration::from_secs(SYNC_BACKOFF_MAX_SECS)),
            Duration::from_secs(SYNC_BACKOFF_MAX_SECS)
        );
    }

    #[test]
    fn stall_for_maps_errors_to_renderable_reasons() {
        assert_eq!(
            stall_for(&WalletError::Sync {
                stall: StallReason::TorUnavailable
            }),
            StallReason::TorUnavailable
        );
        assert_eq!(stall_for(&WalletError::DiskFull), StallReason::StorageFull);
        // LOCAL faults, split by remedy (R10 §4.1). Only a CORRUPT store justifies the
        // restore-from-seed copy, so only `StoreCorrupt` is `Internal`; never the
        // misleading `EndpointUnreachable` ("switch servers", §2.5).
        assert_eq!(stall_for(&WalletError::StoreCorrupt), StallReason::Internal);
        // A non-ENOSPC filesystem IO fault and a mid-scan StoreBusy (#371: a write that
        // lost its WAL race) are the transient, non-corrupt local classes: "sync paused
        // on this device, retrying" — never restore, never switch servers.
        assert_eq!(
            stall_for(&WalletError::Io(std::io::Error::from(
                std::io::ErrorKind::PermissionDenied
            ))),
            StallReason::StorageUnavailable
        );
        assert_eq!(
            stall_for(&WalletError::StoreBusy),
            StallReason::StorageUnavailable
        );
        assert_ne!(
            stall_for(&WalletError::StoreBusy),
            StallReason::Internal,
            "R10: a busy store is not corruption; the restore remedy is destructive"
        );
        // An error with no explicit local/network mapping falls through to the
        // lowest-claim transient reason (the loop keeps retrying; never "funds lost").
        assert_eq!(
            stall_for(&WalletError::SeedRequired),
            StallReason::EndpointUnreachable
        );
    }

    /// T0-1c (§4j row 2): a wrong-chain server is "switch servers", never "check
    /// your connection" and never "the device is broken". `NetworkMismatch` reaches
    /// this mapper from exactly one producer inside a pass —
    /// `provision::endpoint_identity`, every pass — so this pin is the whole of the
    /// class as it stands.
    #[test]
    fn stall_for_renders_a_wrong_chain_server_as_switch_servers() {
        let reason = stall_for(&WalletError::NetworkMismatch);
        assert_eq!(reason, StallReason::EndpointMisbehaving);
        assert_ne!(
            reason,
            StallReason::EndpointUnreachable,
            "the link is fine; a reconnect fixes nothing"
        );
        assert_ne!(
            reason,
            StallReason::Internal,
            "the device is fine; a restore fixes nothing"
        );
    }

    /// T0-1c-R2 (§4n G6; §4m #13, §4k-run owed 2): a configured birthday above
    /// the chain this server reports is its own reason — never "check your
    /// connection" (the `_ =>` fallback it used to fall into: the server
    /// answered) and never "the device is broken". Mutant: the arm removed
    /// (the first assertion red on `EndpointUnreachable`).
    #[test]
    fn stall_for_renders_a_birthday_above_the_servers_chain_as_its_own_reason() {
        let reason = stall_for(&WalletError::BirthdayInFuture);
        assert_eq!(reason, StallReason::BirthdayInFuture);
        assert_ne!(
            reason,
            StallReason::EndpointUnreachable,
            "the server answered; a reconnect fixes nothing"
        );
        assert_ne!(
            reason,
            StallReason::Internal,
            "the device is fine; a restore would destroy the wallet for a config value"
        );
    }

    #[test]
    fn stall_code_is_secret_free_and_total() {
        for r in [
            StallReason::EndpointUnreachable,
            StallReason::TorUnavailable,
            StallReason::StorageFull,
            StallReason::ChainReorg,
            StallReason::Internal,
            StallReason::EndpointMisbehaving,
            StallReason::BirthdayInFuture,
            StallReason::StorageUnavailable,
        ] {
            let code = stall_code(r);
            assert!(!code.is_empty());
            assert!(code.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        }
        // R10 §4.1: the contract names this code; the `wallet.sync` outcome reads it.
        assert_eq!(
            stall_code(StallReason::StorageUnavailable),
            "storage_unavailable"
        );
    }

    // ── R10 §4.2: the local-fault gate ───────────────────────────────────────
    //
    // A LOCAL stall (`Internal`, `StorageUnavailable`) publishes only when the
    // previous LOOP pass also ended in a local fault; any `Ok` pass or non-local
    // fault clears that memory; non-local reasons publish at once. Each row
    // asserts on the published status sequence, stamped with the virtual clock so
    // "after the second pass" is measured against the engine's own pass starts,
    // never inferred from ordering alone.

    /// Every status the loop publishes, with the virtual instant the collector saw
    /// it. The fakes' passes take no virtual time and the loop sleeps between
    /// passes, so a status published by pass `k` carries pass `k`'s start instant.
    fn collect_timed_statuses(mut sub: watch::Receiver<SyncStatus>) -> TimedStatuses {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let out = Arc::clone(&seen);
        tokio::spawn(async move {
            while sub.changed().await.is_ok() {
                let status = sub.borrow().clone();
                out.lock()
                    .expect("timed status collector")
                    .push((tokio::time::Instant::now(), status));
            }
        });
        seen
    }

    type TimedStatuses = Arc<Mutex<Vec<(tokio::time::Instant, SyncStatus)>>>;
    type TimedStalls = Vec<(tokio::time::Instant, StallReason)>;
    type PassStarts = Vec<tokio::time::Instant>;

    /// The published `Stalled` reasons, each with the instant it was published.
    fn timed_stalls(seen: &TimedStatuses) -> TimedStalls {
        seen.lock()
            .expect("timed status collector")
            .iter()
            .filter_map(|(t, s)| match s {
                SyncStatus::Stalled { reason } => Some((*t, *reason)),
                _ => None,
            })
            .collect()
    }

    /// Run the loop over `plan` until the engine has started `calls` passes, and
    /// return what it published and when each pass started.
    async fn run_gate_rows(plan: Vec<Plan>, calls: usize) -> (TimedStalls, PassStarts) {
        let engine = FakeEngine::new(2_000_000, plan);
        let ctl = SyncController::new(engine.clone());
        let seen = collect_timed_statuses(ctl.subscribe());
        run_until_calls(&ctl, &engine, calls).await;
        let starts = engine.call_times();
        assert!(
            starts.len() >= calls,
            "harness: {calls} passes ran, got {}",
            starts.len()
        );
        (timed_stalls(&seen), starts)
    }

    fn clean_pass() -> Plan {
        Plan::Ok {
            steps: vec![],
            pass: ok_pass(1),
        }
    }

    fn unreachable() -> WalletError {
        WalletError::Sync {
            stall: StallReason::EndpointUnreachable,
        }
    }

    fn io_fault() -> WalletError {
        WalletError::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    }

    /// (a) One busy pass between two clean passes is a WAL race the next pass
    /// clears: nothing is published for it.
    #[tokio::test(start_paused = true)]
    async fn a_single_busy_pass_between_clean_passes_publishes_no_stall() {
        let (stalls, _) = run_gate_rows(
            vec![
                clean_pass(),
                Plan::Err(WalletError::StoreBusy),
                clean_pass(),
            ],
            4,
        )
        .await;
        assert!(
            stalls.is_empty(),
            "Ok, Err(StoreBusy), Ok must publish no Stalled at all; published {stalls:?}"
        );
    }

    /// (b) A corrupt store that repeats is shown, once, and only after the second
    /// pass — the restore copy never flashes for a single pass.
    #[tokio::test(start_paused = true)]
    async fn two_corrupt_passes_publish_internal_once_and_only_after_the_second() {
        let (stalls, starts) = run_gate_rows(
            vec![
                Plan::Err(WalletError::StoreCorrupt),
                Plan::Err(WalletError::StoreCorrupt),
            ],
            3,
        )
        .await;
        assert_eq!(
            stalls.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
            vec![StallReason::Internal],
            "two consecutive StoreCorrupt passes publish Stalled{{Internal}} exactly once"
        );
        let (at, _) = stalls[0];
        assert!(
            at >= starts[1] && at < starts[2],
            "the Internal stall is published by the SECOND pass (started {:?}), not the \
             first (started {:?}); published at {at:?}",
            starts[1],
            starts[0]
        );
    }

    /// (c) Two transient local faults in a row, of different classes, publish the
    /// transient local reason after the second — never the restore copy.
    #[tokio::test(start_paused = true)]
    async fn busy_then_io_publishes_storage_unavailable_after_the_second() {
        let (stalls, starts) = run_gate_rows(
            vec![Plan::Err(WalletError::StoreBusy), Plan::Err(io_fault())],
            3,
        )
        .await;
        assert_eq!(
            stalls.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
            vec![StallReason::StorageUnavailable],
            "Err(StoreBusy), Err(Io) publish Stalled{{StorageUnavailable}} once"
        );
        let (at, _) = stalls[0];
        assert!(
            at >= starts[1] && at < starts[2],
            "published by the second pass (started {:?}); published at {at:?}",
            starts[1]
        );
    }

    /// (d) The gate is for LOCAL reasons only: a network stall on the very first
    /// pass publishes at once, as before R10.
    #[tokio::test(start_paused = true)]
    async fn a_network_stall_on_the_first_pass_publishes_at_once() {
        let (stalls, starts) = run_gate_rows(vec![Plan::Err(unreachable())], 2).await;
        let first = stalls.first().copied();
        assert!(
            matches!(first, Some((_, StallReason::EndpointUnreachable))),
            "the first pass's EndpointUnreachable is published; got {stalls:?}"
        );
        let (at, _) = first.expect("checked above");
        assert!(
            at < starts[1],
            "published by the FIRST pass (started {:?}), before the second started \
             ({:?}); published at {at:?}",
            starts[0],
            starts[1]
        );
    }

    /// (e) The memory clears on `Ok`: alternating busy and clean passes never
    /// accumulate into a stall.
    #[tokio::test(start_paused = true)]
    async fn alternating_busy_and_clean_passes_never_publish_a_stall() {
        let (stalls, _) = run_gate_rows(
            vec![
                Plan::Err(WalletError::StoreBusy),
                clean_pass(),
                Plan::Err(WalletError::StoreBusy),
                clean_pass(),
            ],
            5,
        )
        .await;
        assert!(
            stalls.is_empty(),
            "Err(StoreBusy), Ok, Err(StoreBusy), Ok must publish no Stalled; published \
             {stalls:?}"
        );
    }

    /// (f) Local faults of different remedies each publish their OWN reason: a
    /// busy pass then a corrupt one publishes `Internal` after the second, and the
    /// held first pass's `StorageUnavailable` is never published.
    #[tokio::test(start_paused = true)]
    async fn busy_then_corrupt_publishes_internal_after_the_second() {
        let (stalls, starts) = run_gate_rows(
            vec![
                Plan::Err(WalletError::StoreBusy),
                Plan::Err(WalletError::StoreCorrupt),
            ],
            3,
        )
        .await;
        assert_eq!(
            stalls.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
            vec![StallReason::Internal],
            "Err(StoreBusy), Err(StoreCorrupt) publish Stalled{{Internal}} once, and the \
             held StoreBusy never surfaces"
        );
        let (at, _) = stalls[0];
        assert!(
            at >= starts[1] && at < starts[2],
            "published by the second pass (started {:?}); published at {at:?}",
            starts[1]
        );
    }

    /// Beyond §4.2's floor (the R10 security review): a NON-local fault does not
    /// clear the memory — only a completed pass does — or a corrupt store would hide
    /// forever behind a flaky link. Busy, network, busy: the network stall
    /// publishes at once, and the second busy pass publishes its own reason.
    #[tokio::test(start_paused = true)]
    async fn a_network_stall_between_local_faults_keeps_the_local_memory() {
        let (stalls, starts) = run_gate_rows(
            vec![
                Plan::Err(WalletError::StoreBusy),
                Plan::Err(unreachable()),
                Plan::Err(WalletError::StoreBusy),
            ],
            4,
        )
        .await;
        assert_eq!(
            stalls.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
            vec![
                StallReason::EndpointUnreachable,
                StallReason::StorageUnavailable
            ],
            "the network stall publishes, then the second local fault since a completed \
             pass publishes its own reason"
        );
        let (at, _) = stalls[1];
        assert!(
            at >= starts[2] && at < starts[3],
            "the local stall is published by the third pass; published at {at:?}"
        );
    }

    /// The same rule across a watchdog cancel: a wedged pass is not a completed
    /// one, so corrupt, wedge, corrupt still reaches `Internal` on the third pass.
    #[tokio::test(start_paused = true)]
    async fn a_watchdog_cancel_between_corrupt_passes_keeps_the_local_memory() {
        let (stalls, _) = run_gate_rows(
            vec![
                Plan::Err(WalletError::StoreCorrupt),
                Plan::Stuck { steps: vec![] },
                Plan::Err(WalletError::StoreCorrupt),
            ],
            4,
        )
        .await;
        assert_eq!(
            stalls.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
            vec![StallReason::EndpointUnreachable, StallReason::Internal],
            "the watchdog's stall, then the corrupt store after the cancelled pass"
        );
    }

    /// R10 §4.3: the loop's fault warn names the fault's `RW-` code, so a field log
    /// says WHICH local fault stalled the pass. Driven on a first-pass `StoreBusy`,
    /// which the §4.2 gate holds back from the status: the warn is logged anyway.
    #[tokio::test(start_paused = true)]
    async fn the_fault_warn_carries_the_errors_code_on_a_busy_pass() {
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        let engine = FakeEngine::new(2_000_000, vec![Plan::Err(WalletError::StoreBusy)]);
        let ctl = SyncController::new(engine.clone());
        run_until_calls(&ctl, &engine, 2).await;

        let want = WalletError::StoreBusy.code();
        assert!(want.starts_with("RW-"), "harness: {want} is an RW- code");
        let records = sink.records_of("wallet.sync");
        let fault: Vec<&Vec<(String, String)>> = records
            .iter()
            .filter(|r| {
                r.iter()
                    .any(|(n, v)| n == "outcome" && v == "storage_unavailable")
            })
            .collect();
        assert_eq!(
            fault.len(),
            1,
            "one warn for the one busy pass: {records:?}"
        );
        assert!(
            fault[0].iter().any(|(n, v)| n == "code" && v == want),
            "the wallet.sync fault warn carries code = {want}: {:?}",
            fault[0]
        );
        assert_5_4_clean(&sink.fields());
    }

    // ── the §5.4 capture-layer guard ─────────────────────────────────────────
    //
    // The harness + the never-log/allowlist policy live in `crate::tracing_guard`
    // (the ONE source of truth, shared with the engine pass's `wallet.sync` span
    // guard in `wallet`). This test drives the CONTROLLER's event surface through
    // it; `wallet::…tracing_span_is_5_4_clean` drives the engine span.

    #[tokio::test(start_paused = true)]
    async fn tracing_spans_carry_no_address_amount_or_memo() {
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };

        // Keep `zec_wallet_core` callsites enabled so this capture test is deterministic
        // under parallel execution (a thread-local `set_default` subscriber is invisible
        // to tracing's global interest cache — see the helper's doc).
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        // Drive EVERY controller tracing path: a WEDGE (the watchdog_restart
        // outcome), then a fault (outcome + backoff_secs), then a clean pass WITH
        // reorgs (the wallet.sync event + the reorg event).
        let engine = FakeEngine::new(
            2_000_000,
            vec![
                Plan::Stuck { steps: vec![] },
                Plan::Err(WalletError::Sync {
                    stall: StallReason::TorUnavailable,
                }),
                Plan::Ok {
                    steps: vec![progress(2_000_000, 1_999_000, 0.9)],
                    // §4q-R P-RR4: the depth is the pass's measured `rewound_blocks`,
                    // never `reorgs × REWIND_DISTANCE_BLOCKS` — 92 is the REQ-1 fold's
                    // one rewind (target 280,291, landed 280,199), a number no product
                    // of 2 and 10 reaches, so the product reverted cannot pass this.
                    pass: SyncPass {
                        rewound_blocks: 92,
                        ..reorg_pass(3, 2)
                    },
                },
            ],
        );
        let ctl = SyncController::new(engine);
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;

        let fields = sink.fields();
        assert!(
            !fields.is_empty(),
            "expected captured zec_wallet_core tracing events"
        );
        // §5.4: no forbidden token in any name/value; every name allowlisted.
        assert_5_4_clean(&fields);

        // Positive coverage: the reorg instability event actually fired with the
        // TRUE depth (gate 5 — recovery is observable; §4q-R P-RR4), proving the
        // guard ran over a populated, representative event set rather than an
        // empty one.
        let depth_seen = fields
            .iter()
            .any(|(n, v)| n == "depth" && v.as_str() == "92");
        assert!(
            depth_seen,
            "expected a wallet.reorg_rewind depth event carrying the pass's rewound_blocks \
             (92), never reorgs × REWIND_DISTANCE_BLOCKS"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn loop_start_and_stop_bracket_the_ladder_and_repeat_stops_stay_silent() {
        // (chased live as a false alarm): each host-driven loop restart
        // (pause/resume, rescan, retry) deliberately resets the backoff ladder,
        // and WITHOUT lifecycle events a field log shows "backoff reset with no
        // success between" — indistinguishable from a controller bug. Pin: one
        // loop_start per start(), one loop_stop per LIVE stop, and an idempotent
        // second stop emits nothing.
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                pass: ok_pass(1),
            }],
        );
        let ctl = SyncController::new(engine);
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;
        ctl.stop().await; // idempotent — must NOT emit a second loop_stop

        let fields = sink.fields();
        let count = |v: &str| {
            fields
                .iter()
                .filter(|(n, val)| n == "outcome" && val == v)
                .count()
        };
        assert_eq!(count("loop_start"), 1, "one start ⇒ one loop_start");
        assert_eq!(
            count("loop_stop"),
            1,
            "one live stop ⇒ one loop_stop; repeats silent"
        );
        assert_5_4_clean(&fields);
    }

    #[tokio::test(start_paused = true)]
    async fn the_pass_event_says_where_the_pass_went() {
        // SCAN-1 (§4o S3 — the capture row): the clean-pass `wallet.sync` event
        // carries the pass's phase sums, its wall-clock and its outputs beside
        // `batches`/`reorgs`, with the VALUES the engine handed over — one line per
        // pass that says where the pass went. The sink groups one event's fields
        // together (`records_of`), so the values are read off THE `outcome=ok`
        // record, not fished out of a flat list.
        //
        // Mutants this row was watched against (S6): the sums zeroed in
        // `emit_synced` (every value is asserted, not its presence); any one of
        // `anchor_ms`/`dl_ms`/`scan_ms`/`snap_ms`/`wall_ms`/`outputs` (and, S15,
        // `tip_ms`/`roots_ms`) dropped from the event (the exact-name set).
        use crate::tracing_guard::{
            CaptureLayer, CapturedEvents, assert_5_4_clean, force_wallet_callsites_enabled,
        };
        use std::collections::BTreeSet;
        force_wallet_callsites_enabled();
        let sink = CapturedEvents::default();
        let subscriber = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone()));
        let _guard = tracing::subscriber::set_default(subscriber);

        let pass = SyncPass {
            anchor_ms: 38,
            dl_ms: 310,
            scan_ms: 330,
            snap_ms: 12,
            tip_ms: 61,
            roots_ms: 520,
            wall_ms: 1_234,
            chain_outputs: 2_048,
            ..ok_pass(3)
        };
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Ok {
                steps: vec![progress(2_000_000, 2_000_000, 1.0)],
                pass,
            }],
        );
        let ctl = SyncController::new(engine);
        let mut sub = ctl.subscribe();
        ctl.start();
        wait_for_uptodate(&mut sub).await;
        ctl.stop().await;

        let records = sink.records_of("wallet.sync");
        let ok: Vec<&Vec<(String, String)>> = records
            .iter()
            .filter(|r| r.iter().any(|(n, v)| n == "outcome" && v == "ok"))
            .collect();
        assert_eq!(
            ok.len(),
            1,
            "one clean pass ⇒ one `outcome=ok` pass event; wallet.sync records: {records:?}"
        );
        let event = ok[0];
        let value = |name: &str| {
            event
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
                .unwrap_or_else(|| panic!("the pass event is missing {name:?}: {event:?}"))
        };
        for (name, want) in [
            ("batches", "3"),
            ("reorgs", "0"),
            ("anchor_ms", "38"),
            ("dl_ms", "310"),
            ("scan_ms", "330"),
            ("snap_ms", "12"),
            ("tip_ms", "61"),
            ("roots_ms", "520"),
            ("wall_ms", "1234"),
            ("chain_outputs", "2048"),
            // §4u: the sleep the loop takes next — the poll interval after a clean pass
            // (= POLL_INTERVAL_SECS; the loop-driven path, so the field is present).
            ("backoff_secs", "20"),
        ] {
            assert_eq!(value(name), want, "pass event field {name}");
        }
        let names: BTreeSet<&str> = event.iter().map(|(n, _)| n.as_str()).collect();
        let want: BTreeSet<&str> = [
            "message",
            "batches",
            "reorgs",
            "anchor_ms",
            "dl_ms",
            "scan_ms",
            "snap_ms",
            "tip_ms",
            "roots_ms",
            "wall_ms",
            "chain_outputs",
            "backoff_secs",
            "outcome",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            names, want,
            "the pass event's field-name set is exactly counts + sums + wall + the next sleep \
             + outcome"
        );
        assert_5_4_clean(&sink.fields());
    }

    // ── FR-40: the bounded pass (`once_within`, stage S2 §3.5b) ─────────────

    /// A wedged pass under a budget: the deadline fires the pass's OWN token (the
    /// one `request_stop` fires), the pass returns cancelled, and the call returns
    /// AT its budget on the virtual clock — without the fire it would sit until the
    /// watchdog's window. Not a `timeout` around the future: the active token is
    /// cleared by the call itself, and the cut pass leaves `Idle` rather than the
    /// last `Scanning` on the stream (nothing is scanning once it returns).
    #[tokio::test(start_paused = true)]
    async fn a_bounded_pass_on_a_wedged_engine_returns_at_its_deadline() {
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Stuck {
                steps: vec![progress(2_000_000, 1_999_000, 0.5)],
            }],
        );
        let ctl = SyncController::new(engine.clone());
        let budget = Duration::from_secs(45);
        let t0 = tokio::time::Instant::now();
        let out = ctl
            .once_within(budget)
            .await
            .expect("a pass cut by its deadline is not an error");
        assert_eq!(
            t0.elapsed(),
            budget,
            "the deadline ended the pass at the budget"
        );
        assert!(out.pass.cancelled, "the pass honoured its own token");
        assert!(!out.resubmitted, "a cut pass drives no resubmission");
        assert_eq!(engine.after_synced_count(), 0);
        assert_eq!(
            ctl.status(),
            SyncStatus::Idle,
            "a cut pass leaves Idle, never a stale Scanning"
        );
        assert!(
            ctl.shared
                .active_cancel
                .lock()
                .expect("active-cancel mutex")
                .is_none(),
            "the call cleared its active token before returning"
        );
    }

    /// The resubmission reserve at its boundary: a budget whose remainder after
    /// the (instant) pass is exactly the reserve runs the hook; one millisecond
    /// less skips it and says so.
    #[tokio::test(start_paused = true)]
    async fn a_bounded_pass_resubmits_only_when_the_rest_of_its_budget_covers_the_reserve() {
        let engine = FakeEngine::new(2_000_000, vec![]);
        let ctl = SyncController::new(engine.clone());

        let at = ctl
            .once_within(SyncController::RESUBMIT_RESERVE)
            .await
            .expect("pass ok");
        assert!(!at.pass.cancelled);
        assert!(at.resubmitted, "the reserve fits exactly: the hook runs");
        assert_eq!(engine.after_synced_count(), 1);

        let short = ctl
            .once_within(SyncController::RESUBMIT_RESERVE - Duration::from_millis(1))
            .await
            .expect("pass ok");
        assert!(!short.pass.cancelled);
        assert!(
            !short.resubmitted,
            "a millisecond short of the reserve: skipped, and said so"
        );
        assert_eq!(
            engine.after_synced_count(),
            1,
            "the skipped hook was not called"
        );
    }

    /// The S2 diff review's race: `stop_sync` while a `syncFor` pass is running
    /// (both are shared-lock calls on the handle, so a host can overlap them). The
    /// stop must not return — and publish its `Idle` — while the bounded pass is
    /// still unwinding: a host told "stopped" would otherwise see that pass
    /// finish its in-flight work and publish after it. The loop's pass is joined
    /// through its task handle; the bounded pass has none, so the stop waits out
    /// the pass guard it holds.
    #[tokio::test(start_paused = true)]
    async fn stop_waits_out_a_running_bounded_pass() {
        let engine = FakeEngine::new(
            2_000_000,
            vec![Plan::Stuck {
                steps: vec![progress(2_000_000, 1_999_000, 0.5)],
            }],
        );
        let ctl = SyncController::new(engine.clone());
        let active = |ctl: &SyncController| {
            ctl.shared
                .active_cancel
                .lock()
                .expect("active-cancel mutex")
                .is_some()
        };
        let (pass, ()) = tokio::join!(ctl.once_within(Duration::from_secs(600)), async {
            while !active(&ctl) {
                tokio::task::yield_now().await;
            }
            ctl.stop().await;
            assert!(
                !active(&ctl),
                "stop returned while the bounded pass still held its token"
            );
            assert_eq!(ctl.status(), SyncStatus::Idle);
        });
        let pass = pass.expect("a stopped pass is not an error");
        assert!(pass.pass.cancelled, "the stop cancelled the bounded pass");
        assert_eq!(
            ctl.status(),
            SyncStatus::Idle,
            "the stop's Idle is the last word"
        );
    }
}
