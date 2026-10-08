import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/lifecycle/app_lifecycle_provider.dart';
import 'wallet_providers.dart';
import 'wallet_session.dart';

/// What the host is doing with the background sync LOOP — distinct from the
/// observed `SyncStatus` (which reports how that loop is *progressing*; see
/// [syncStatusProvider]). This enum is the reference app's SYNC POLICY.
///
/// It is INTERNAL POLICY state. [WalletSyncDrive.failed] renders the retry
/// notice, and [WalletSyncDrive.running] is read by the sync banner to
/// disambiguate the SDK's `Idle`: a started-but-not-yet-reporting loop (the
/// silent prep phase — fetching commitment-tree roots + the chain tip) renders
/// "Connecting…" rather than a stale "Not syncing yet". `suspended`/`inactive`
/// are still never shown. The honest live progress is always the `SyncStatus`
/// banner; this state only colours how its `Idle` arm reads. The optimistic
/// `running` set before a (near-instant, loop-spawning) `startSync` settles is
/// safe — at worst it shows "Connecting…" a beat early, never a false sync claim.
///
/// FORWARD-OWED (review S99): ideally the SDK emits a real `SyncStatus::Connecting`
/// during the prep phase so this disambiguation lives Rust-side and `driving` drops
/// out of `_SyncBanner`; today the production controller never emits that arm.
///
/// WHY this exists (maintainer review, spec §3.3): the SDK's `Wallet::open`
/// deliberately does NOT auto-sync — the host owns sync policy (battery,
/// network, foreground/background), so the core exposes idempotent
/// `startSync`/`stopSync` and leaves the *when* to the host. Real wallets have
/// no "Start sync" button; sync just runs. So this controller drives the loop:
///  - run while a deposit-ready wallet exists AND the app is foreground,
///  - stop in the background (battery — the OS would suspend us anyway),
///  - resume on return.
/// It replaces the manual Start/Stop buttons an earlier slice surfaced.
///
/// Money-safety: stopping loses nothing — durable scan progress is kept (the
/// chain is the source of truth, SDK §6.3), so a background stop + foreground
/// restart resumes exactly where it left off.
enum WalletSyncDrive {
  /// No deposit-ready wallet → nothing to drive.
  inactive,

  /// Wallet present + app foreground → the loop is running (or being started).
  running,

  /// App backgrounded → the loop is stopped to save battery; a real resume
  /// restarts it. UNREACHABLE on desktop: `paused` never fires there (the
  /// deepest state is `hidden`), so desktop keeps the loop running while
  /// minimized — which is wanted.
  suspended,

  /// The START command itself failed (rare — e.g. the handle closed underneath
  /// us). Surfaced honestly (no silent failure, invariant 10); a foreground
  /// resume (mobile only — `paused` never fires on desktop) or an explicit
  /// [WalletSyncController.retry] re-attempts. A *stop* failure does NOT land
  /// here — see [WalletSyncController._command].
  failed,

  /// The HOST's sync policy is off (#383 R1 — `walletSyncPolicyProvider`
  /// false): the loop is deliberately not driven. Distinct from [inactive]
  /// (a wallet EXISTS; the host chose not to sync it) so the badge/sheet can
  /// render the honest "sync off — turn it on in settings" story instead of a
  /// stalled/connecting state that never resolves. Reactive: the controller
  /// watches the policy, so a host flip starts/stops the loop on the spot.
  disabledByHost,
}

/// Drives the background sync loop from (session-present ∧ app-foreground).
///
/// LIFETIME: a plain (non-autoDispose) [NotifierProvider]. It is built lazily
/// the first time something watches it: the wallet surface when it first
/// renders Active (`_WalletActive` watches it), or — FR-51 — a HOST that
/// listens to [walletSyncDriveProvider] at its root so sync runs whenever a
/// session is live and the app is foreground, wallet screen or not. Built
/// before a session exists it is `inactive` and does nothing; the session
/// arriving rebuilds it and starts the loop. From then on it persists
/// for the life of the root `ProviderContainer` (the same mechanism that keeps
/// `onboardingControllerProvider`/`walletSessionProvider`/`syncStatusProvider`
/// alive across navigation), so sync keeps running while the user is on another
/// screen — incoming funds are still detected with the wallet screen closed —
/// and is suspended only by actual backgrounding. It is re-built (a FRESH
/// instance, fields reset) only when its one watched dependency,
/// `walletSessionProvider`, changes (e.g. null → session on Active). The SDK
/// itself never warms it at boot: WHEN the wallet opens (and so when sync can
/// start) is the host's decision, since opening takes the single-writer lock.
/// A host that opens the wallet at launch and listens to the drive gets sync
/// from launch; one that opens lazily gets it from the first wallet use.
///
/// ORTHOGONAL to [SyncStatusNotifier]: that one OBSERVES the status stream (and
/// pauses/reconnects it); this one CONTROLS the loop. They share the same
/// lifecycle triggers but never call each other — one source of truth each. On
/// a resume both act independently: the observer re-subscribes (replaying the
/// CURRENT status immediately) while this controller re-issues `startSync`.
/// That ordering is unsynchronized and SAFE by construction — `startSync` is
/// idempotent + near-instant (spawns the loop, no network wait), the stream's
/// current-first replay means the banner is always the real status, and this
/// drive state is not rendered except `failed`. So there is no window in which
/// the UI claims a state the core isn't in.
final walletSyncControllerProvider =
    NotifierProvider<WalletSyncController, WalletSyncDrive>(
      WalletSyncController.new,
    );

/// The sync drive, public (FR-51): the SAME provider as the package's
/// internal `walletSyncControllerProvider`, under the name hosts use.
///
/// A host that wants sync whenever the app is in the foreground — not only
/// after the wallet screen has rendered — listens to it once at its root:
///
/// ```dart
/// ref.listen(walletSyncDriveProvider, (_, __) {});
/// ```
///
/// It then runs while `walletSessionProvider` is non-null, the host's sync
/// policy ([walletSyncPolicyProvider]) is on and the app is not paused; it
/// suspends on `paused` and resumes on return, exactly as under the wallet
/// screen. With no session it is `inactive` and costs nothing. It does not
/// OPEN the wallet: a session-only host opens its own.
final walletSyncDriveProvider = walletSyncControllerProvider;

/// SSOT for money copy that leans on a future SYNC PASS (#401 R5).
///
/// The core's durable money machinery — the §6.1 `ReBroadcast` arm that re-sends
/// an already-signed-but-unbroadcast transaction, the §6.3 reconcile that
/// re-queues a mid-signature row, the queue drain — is driven EXCLUSIVELY by
/// `after_synced`. No completed pass, no correction. So any string that promises
/// the wallet "will finish this on a later sync" is a claim about THIS provider,
/// and must be qualified when it reads `false`.
///
/// BOTH TERMS (#403 R5 — the #401 version read the DRIVE alone).
///
/// `!= failed` is the term that does the work the policy cannot: a start command
/// that FAILED means the loop never ran and `after_synced` never fires, WHILE the
/// policy still reads `true`. Copy keyed on the policy alone tells exactly that
/// user their payment completes on its own — the wait-forever this qualifier
/// exists to prevent.
///
/// [walletSyncPolicyProvider] is the host's own switch and a plain
/// `Provider<bool>` — it cannot lag and it has no in-flight command behind it.
/// BE PRECISE ABOUT WHAT IT BUYS, because the review that proposed this
/// conjunction credited it with more: today it is REDUNDANT with the drive's
/// `disabledByHost`, since `build` returns that synchronously the moment the
/// policy reads `false`, and every settle path is already policy-aware. It is
/// kept as defence in depth — the host's explicit "off" should be authoritative
/// for a money promise no matter what the drive is doing — not as a fix for a
/// reproduced bug. It does NOT close the background/resume flicker either: on the
/// flickering host the policy reads `true`, so the whole predicate rides on the
/// drive. What closes that is the drive itself now PRESERVING `failed` across a
/// background stop (see the settle in `_command`), which is where the bug was.
///
/// The two remaining drive states are deliberately not consulted:
///  * [WalletSyncDrive.suspended] — the app is backgrounded, so nothing that
///    reads this is on screen, and a resume restarts the loop. Qualifying it
///    would be a pause the user can only ever be told about after it ended.
///  * [WalletSyncDrive.inactive] — no session; every consumer here renders
///    inside a live one.
///
/// TWO ROLES, one predicate (#405 sharpened the original "never a gate" claim,
/// which this provider outgrew the moment a DESTRUCTIVE op was keyed on it):
///
///  1. HONESTY QUALIFIER for money copy — the original role. A `false` hides
///     and disables NOTHING: the affordances a committed payment needs stay
///     available (the #400 R9 rule), the copy just stops promising a schedule
///     it cannot keep.
///  2. READINESS PREDICATE for the two ops that can only COMPLETE through a
///     sync pass — the rescan (wipes the DB and rebuilds as sync scans) and the
///     swap deep scan (marks ranges a later pass covers). Offering those while
///     no pass will run is a destructive dead end, not a copy problem, so they
///     are genuinely DISABLED on `false` and the rescan is refused again at its
///     commit point. This is not an exception to rule 1 — neither op is an
///     affordance a committed payment needs; both are user-initiated recovery.
///
/// The line between the two: never remove a way to FINISH or RESCUE money that
/// is already committed; do refuse to START something that provably cannot end.
///
/// CAUSE-AGNOSTIC AT EVERY CONSUMER (#405). The predicate deliberately does not
/// say WHICH term is false, and no consumer may re-derive it to word its copy:
/// "turn syncing on in settings" is the right instruction for `disabledByHost`
/// and the WRONG one for a failed start, and a family that words itself per
/// cause drifts back into exactly the #401 R5 bug the SSOT replaced. Consumers
/// state the CONDITION ("syncing isn't running"); the sync badge and the
/// start-failed notice — both always on the wallet surface — own the cause and
/// its remedy.
final walletSyncPassesRunProvider = Provider<bool>((ref) {
  return ref.watch(walletSyncPolicyProvider) &&
      ref.watch(walletSyncControllerProvider) != WalletSyncDrive.failed;
});

class WalletSyncController extends Notifier<WalletSyncDrive> {
  /// Touching `ref`/`state` after dispose throws; every async command
  /// continuation re-checks this first. Set in the `onDispose` below. NOTE:
  /// riverpod REUSES this notifier across a dependency-change rebuild (it
  /// does NOT construct a fresh one — the wrong model that produced #330);
  /// build() resets this at the top, so it is never sticky-true.
  bool _disposed = false;

  /// The build-cycle stamp for command continuations (the #330 class):
  /// a session flip re-runs build() on the SAME notifier and resets
  /// `_disposed`, so a command issued in the PRIOR cycle needs its own guard —
  /// without it the stale continuation would start/stop the DEAD session's
  /// handle and write a stale drive state over the new cycle's. Bumped each
  /// build(); captured by [_command] at issue time.
  int _generation = 0;

  /// The lifecycle arrives as `paused → hidden → inactive → resumed`, so at the
  /// `resumed` event the previous state is `inactive`, NOT `paused`. Only a
  /// remembered flag tells a real resume (restart sync) from window-focus /
  /// shade-pull noise (`inactive → resumed`), which must NOT re-issue a command.
  bool _wasPaused = false;

  /// Did the last start command for THIS session fail (#407 R7)?
  ///
  /// A separate memory from the drive state because the drive is overwritten by
  /// the very transitions this needs to survive: a policy flip to `false`
  /// settles it to `disabledByHost`, so the flip back to `true` finds no
  /// `failed` left to preserve. Cleared on a session change (below) and by any
  /// successful start; set by every path that writes [WalletSyncDrive.failed].
  bool _startFailed = false;

  /// The session `_startFailed` describes. A NEW wallet has not failed anything,
  /// so the flag must not ride the swap.
  WalletSession? _lastSession;

  /// Serializes start/stop so a rapid background→foreground flap can't run a
  /// stop and a start concurrently (the SDK calls are async). Each new command
  /// chains after the previous one settles; the chain never rejects (errors are
  /// caught inside), so it can be safely re-`then`-ed forever.
  ///
  /// This does NOT grow unbounded: `_inFlight` only ever holds the TAIL, so once
  /// a link settles nothing references it and the GC reclaims it — the live
  /// chain is at most the count of not-yet-settled commands. We deliberately do
  /// NOT re-anchor it to `Future.value()` after each settle: writing the field
  /// from inside the settled callback would race a concurrently-arriving
  /// `_command` (a TOCTOU on the tail) and could drop a serialization edge.
  /// Leaving the tail is correct.
  ///
  /// #407 R8 — "≤2 in practice" WAS the bound and no longer is. It held while
  /// only taps and lifecycle events appended; #404's reconnect kick appends
  /// AUTOMATICALLY, up to once per cooldown while a link flaps. So EVERY link in
  /// this chain must settle on its own: an unbounded `startSync` that wedges
  /// would stall the chain permanently with every later command's closure
  /// retained behind it. The dispose stop was already bounded for the same
  /// reason; now the start legs are too. The bound is
  /// [walletFfiWedgeTimeout] and it is a LIVENESS device, not a correctness one
  /// — a timed-out start reports the honest `failed`, which is what the user
  /// would see anyway if the FFI never answered.
  Future<void> _inFlight = Future<void>.value();

  @override
  WalletSyncDrive build() {
    // Riverpod REUSES this notifier instance across a dependency-change rebuild
    // (it re-runs build() rather than constructing a fresh object), and the
    // prior build's onDispose fires first — which would leave `_disposed` stuck
    // true and `_wasPaused` stale on the reused instance. Reset both at the top
    // so the controller is correct on every (re)build, fresh instance or not.
    // (Since FR-51 production DOES rebuild null→session: a host that listens
    // to walletSyncDriveProvider at its root builds the drive before the
    // wallet opens. This reset is what makes that path correct.)
    _disposed = false;
    _wasPaused = false;
    _generation++;

    final session = ref.watch(walletSessionProvider);
    if (!identical(session, _lastSession)) {
      _lastSession = session;
      _startFailed = false; // a new wallet has not failed anything yet
    }
    ref.onDispose(() {
      _disposed = true;
      // Best-effort stop on teardown — idempotent and loses no progress.
      // Dispose can't await, but the stop MUST ride the [_inFlight] serializer
      // (fold, the one command that didn't): since #383 made build()
      // watch the sync POLICY, a policy flip re-runs build() on the SAME
      // session — the first same-handle stop/start neighborhood. Dispatched
      // fire-and-forget (the pre-#383 shape), this stop and an in-flight or
      // just-issued start are two unordered FFI calls racing on one Rust
      // controller: orderings ending in the start leave the loop RUNNING under
      // a "Sync off" badge; the mirror flip leaves a stopped loop under
      // `running` — which desktop (no `paused` lifecycle event) never heals.
      // Chained, it provably runs after any in-flight start and before the
      // next build's `_command(start)` (the notifier and `_inFlight` survive
      // the rebuild). On a session SWITCH this also orders the dead handle's
      // stop before the new session's start — a LIVENESS COUPLING, not free
      // tidiness: a wedged dead-handle `stopSync` would
      // otherwise starve the NEW wallet's first start forever behind
      // "Connecting…" (desktop never gets the lifecycle re-drive, and
      // retry() chains behind the same tail). So this one link — best-effort
      // by contract — is BOUNDED by the package FFI wedge timeout: on
      // timeout the wedged stop is abandoned and the chain proceeds. The
      // bounded window deliberately re-opens a sliver of the stop/start race
      // it serializes, only in the already-pathological wedged-FFI case.
      // `.catchError` is REQUIRED: the chain must never reject (it is
      // re-`then`-ed forever). On an abrupt process kill the stop may never
      // reach the SDK at all, but that is safe: scan progress is durable
      // (the chain is the source of truth), so the next open resumes where
      // it left off.
      if (session != null) {
        _inFlight = _inFlight
            .then(
              (_) => session.stopSync().timeout(
                walletFfiWedgeTimeout,
                onTimeout: () {},
              ),
            )
            .catchError((Object _) {});
      }
    });
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      _onLifecycle(next);
    });

    if (session == null) return WalletSyncDrive.inactive;

    // The host's sync policy (#383 R1) — WATCHED, so a host flip re-runs this
    // build: policy→false rebuilds through the prior cycle's onDispose (which
    // best-effort stops the loop) and settles here without a start;
    // policy→true falls through to the normal start below. Checked BEFORE the
    // lifecycle arm so a born-paused disabled host reads "disabled", not
    // "suspended" (the honest reason wins).
    if (!ref.watch(walletSyncPolicyProvider)) {
      return WalletSyncDrive.disabledByHost;
    }

    // Born-backgrounded cold start (Android push-trampoline): do NOT start the
    // loop while paused — the first real resume starts it (no background scan).
    //
    // #407 R7 — PRESERVE A FAILED START HERE TOO. #403 R5 taught the stop-settle
    // to keep `failed` across a background cycle, but this arm returned
    // `suspended` unconditionally, and `suspended` is a PASSES-RUN state. A host
    // whose sync policy flips while backgrounded (battery saver, metered, an org
    // policy) re-runs build() through this line, so the unqualified "your wallet
    // will send this on a later sync" promise came back — and it is still there
    // on the first frames after the user foregrounds, because the resume's
    // `_command(start)` writes no optimistic state and only corrects to `failed`
    // once it settles. That flash is exactly what #403 R5 removed.
    //
    // IT READS `_startFailed`, NOT THE CURRENT STATE — measured, after a first
    // attempt at the latter was dead code. The only path that re-runs build()
    // while paused is a POLICY flip, and a flip to `false` legitimately settles
    // the state to `disabledByHost` on the way; by the time the flip back to
    // `true` reaches this arm, the `failed` it was supposed to preserve is gone.
    // Probe, with the state-reading version in place:
    //   C policy-off-while-bg  drive=disabledByHost passesRun=false
    //   D policy-on-while-bg   drive=suspended      passesRun=true   <-- still wrong
    // The remembered flag survives that detour, which is the whole point.
    if (ref.read(appLifecycleProvider) == AppLifecycleState.paused) {
      _wasPaused = true;
      return _startFailed ? WalletSyncDrive.failed : WalletSyncDrive.suspended;
    }

    _command(session, start: true);
    // Optimistic; _command corrects to `failed` if the start command throws.
    // Safe to show because this state is internal (not rendered) — see the
    // [WalletSyncDrive] doc.
    return WalletSyncDrive.running;
  }

  void _onLifecycle(AppLifecycleState next) {
    // Read the session fresh (not a build-time capture) so a lifecycle event
    // always acts on the current wallet — matches SyncStatusNotifier's pattern.
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    switch (next) {
      case AppLifecycleState.paused:
        _wasPaused = true;
        _command(session, start: false); // stop the loop (battery)
      case AppLifecycleState.resumed:
        // Only a REAL paused→resumed restarts (flutter-patterns § Stream
        // Lifecycle): `inactive/hidden → resumed` is focus/shade noise and
        // would re-issue a needless FFI command every time. NEVER while the
        // host's sync policy is off (#383 R1) — a resume must not sneak the
        // loop past the policy gate; the policy flip itself (watched in
        // build()) is the only way back to running.
        if (_wasPaused) {
          _wasPaused = false;
          if (!ref.read(walletSyncPolicyProvider)) {
            // Belt for the stop-settle rewrite above: whatever transient the
            // pause left behind, a resumed-but-policy-off wallet renders the
            // honest "Sync off", never a retained healthy story.
            state = WalletSyncDrive.disabledByHost;
            return;
          }
          _command(session, start: true);
        }
      // Never act on inactive/hidden/detached. `hidden` is desktop's deepest
      // state (and Android's pre-`paused` step) — stopping there would kill
      // desktop sync while minimized, which we WANT to keep running.
      case AppLifecycleState.inactive:
      case AppLifecycleState.hidden:
      case AppLifecycleState.detached:
        break;
    }
  }

  /// Re-attempt after a [WalletSyncDrive.failed] — the only state with a manual
  /// affordance, and a contextual recovery action (honest next step, invariant
  /// 6), not the old always-on Start/Stop button.
  void retry() {
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    // #383 R1: retry is the failed-start escape, never a policy override — a
    // disabled host stays disabled (the retry notice isn't rendered there,
    // this is the belt for a stale tap racing a policy flip).
    if (!ref.read(walletSyncPolicyProvider)) return;
    // Optimistic `running` (the same treatment build() uses — safe because
    // this state is internal except the `failed` notice): the retry notice
    // clears ON TAP instead of sitting enabled through the serialized command
    // settle (a dead-feeling tap invites double-taps — reliability LOW);
    // _command re-asserts `failed` if the start actually fails again.
    state = WalletSyncDrive.running;
    _command(session, start: true);
  }

  /// Retry NOW from a live [SyncStatus_Stalled] (#399 item 4) — the sheet's
  /// "Try now" affordance. The sync loop's retry backoff caps at 600s and
  /// never resets on connectivity return (no connectivity listener exists —
  /// the residual), so after a long outage a user who just fixed their
  /// Wi-Fi could stare at "retries automatically" for up to 10 minutes. A
  /// stop+start resets the ladder by design (durable scan progress is kept —
  /// stopping loses nothing), giving them an immediate attempt instead.
  ///
  /// One serialized chain LINK for both commands (never two [_command]s): the
  /// pair must be atomic against a concurrently-arriving lifecycle stop/start,
  /// and the intermediate stop must not write its `suspended` settle state
  /// over a surface that never stopped wanting to run. Stop failure is
  /// swallowed (best-effort — the START is the point; a wedged stop is
  /// bounded by [walletFfiWedgeTimeout] like the dispose link); a START
  /// failure lands on the honest `failed` (the retry notice renders).
  /// Returns whether the restart pair was actually ENQUEUED — a gate
  /// rejection returns false so the caller doesn't open a post-tap live
  /// window over a restart that never happened (audit H).
  bool tryNow() {
    final session = ref.read(walletSessionProvider);
    if (session == null) return false;
    // Policy gate (the retry() belt): never a way past a deliberate host off.
    if (!ref.read(walletSyncPolicyProvider)) return false;
    // Only from a LIVE-or-UNCERTAIN loop. The belt this gate exists for is a
    // stale tap racing a background transition (`suspended`): stop+start there
    // would restart sync in the BACKGROUND, past the battery policy, with no
    // later lifecycle stop coming.
    //
    // #409 R2 ADDS `failed`, and this is the whole of that ticket's real harm.
    // A start that merely TIMED OUT lands on `failed` while the Rust loop is
    // most likely running and reporting `Stalled` — and the kick fires exactly
    // on `Stalled`. Gating on `running` alone therefore let a timeout foreclose
    // the one automatic recovery path in the package, permanently, on the very
    // wallet that needed it; desktop never fires `paused`, so nothing cleared
    // it there at all. Admitting `failed` is not a new power: `retry()` already
    // performs this same stop+start from this same state, and a start that
    // succeeds writes `running` and clears `_startFailed` on the way.
    if (state != WalletSyncDrive.running && state != WalletSyncDrive.failed) {
      return false;
    }
    // No optimistic state write (unlike retry()): the gate above just proved
    // `running`, and the drive stays internal here — the tap's visible
    // feedback is the DISPLAY status following the restarted loop (the
    // sheet pairs this call with `followRawNow`).
    final gen = _generation;
    _inFlight = _inFlight
        .then((_) async {
          // The gate above closes only once the pause's stop SETTLES (the
          // pause path writes no optimistic state), so a tap already in the
          // event queue when `paused` lands can pass it (review M1).
          // `_wasPaused` is the pause INTENT flag, set synchronously at the
          // event — re-checked here, inside the serialized link, so a
          // backgrounded restart is foreclosed at execution time too.
          if (_disposed || gen != _generation || _wasPaused) return;
          try {
            await session.stopSync().timeout(
              walletFfiWedgeTimeout,
              onTimeout: () {},
            );
          } catch (_) {
            // Best-effort: an un-stopped loop just keeps its old ladder; the
            // start below is still idempotent-safe.
          }
          if (_disposed || gen != _generation || _wasPaused) return;
          try {
            // Bounded like the stop above (#407 R8): the kick appends to this
            // chain automatically, so a wedged start must not park every later
            // command behind it. A timeout lands on `failed` via the catch —
            // the CONSERVATIVE reading, and #409 R2 keeps it that way; what the
            // bound must not do is DISCARD the eventual answer, which is why the
            // call is also watched to completion by [_watchLateStartVerdict].
            await _watchLateStartVerdict(
              session.startSync(),
              gen,
            ).timeout(walletFfiWedgeTimeout);
            _startFailed = false;
            if (!_disposed && gen == _generation) {
              state = WalletSyncDrive.running;
            }
          } catch (_) {
            _startFailed = true;
            if (_disposed || gen != _generation) return;
            state = WalletSyncDrive.failed;
          }
        })
        .catchError((Object _) {});
    return true;
  }

  /// Monotonic id for START commands, so a LATE verdict can tell whether it is
  /// still the newest word on the subject (#409 R2).
  int _startSeq = 0;

  /// Watch `call` to completion and apply its verdict WHENEVER it lands, then
  /// return a future the caller can bound with [walletFfiWedgeTimeout].
  ///
  /// THE DEFECT THIS CLOSES (#409 R2, security review, proven by probe).
  /// `Future.timeout` registers its `onError` behind `if (timer.isActive)`, so
  /// once the bound fires that handler is a no-op — AND, because a handler was
  /// attached at all, the late error counts as HANDLED and never reaches the
  /// zone. So the abandoned call's answer is not merely late, it is DISCARDED.
  /// A `startSync` that replies at t=20 s with `WalletBusy`/`InvalidState` — the
  /// SDK saying NO, the very case that deserves `failed` — changed no state at
  /// all. #407 R8 got away with it only because its timeout had already written
  /// `failed`; the first cut of #409 R2 wrote `running` instead and so removed
  /// the one thing making that case honest.
  ///
  /// The timeout keeps its conservative reading (`failed`, which is the state
  /// that RENDERS a retry affordance — `running` renders none), and this makes
  /// that reading self-correcting: a late SUCCESS clears it, a late REFUSAL
  /// confirms it. Uncertainty is therefore expensive for nobody: the user sees
  /// an honest notice with a way out, money copy stays qualified, and the
  /// destructive-rescan fence stays shut until the loop is actually known to run.
  ///
  /// The sequence guard is what makes a late write safe: `_generation` only
  /// bumps on a rebuild, so it cannot tell one command from the next within a
  /// generation. Without `_startSeq` a verdict from an abandoned start could
  /// land on top of a newer command's state.
  Future<void> _watchLateStartVerdict(Future<void> call, int gen) {
    final seq = ++_startSeq;
    unawaited(
      call.then(
        (_) {
          if (_disposed || gen != _generation || seq != _startSeq) return;
          _startFailed = false;
          state = WalletSyncDrive.running;
        },
        onError: (Object _) {
          if (_disposed || gen != _generation || seq != _startSeq) return;
          _startFailed = true;
          state = WalletSyncDrive.failed;
        },
      ),
    );
    return call;
  }

  /// Issue an idempotent start/stop, serialized behind any in-flight command.
  /// The optimistic build()/lifecycle state is confirmed here once the command
  /// settles — or corrected on error.
  void _command(WalletSession session, {required bool start}) {
    // Captured at ISSUE time: a continuation from a prior build cycle must
    // neither drive the dead session's handle nor write over the new cycle's
    // state (see [_generation]).
    final gen = _generation;
    _inFlight = _inFlight
        .then((_) async {
          if (_disposed || gen != _generation) return;
          try {
            if (start) {
              // Both legs bounded (#407 R8) — see [_inFlight]. A wedged start
              // times out onto the conservative `failed`; a wedged stop onto the
              // catch's stop-arm. Either way the chain proceeds — and the
              // abandoned start is still watched to completion (#409 R2), so a
              // late answer corrects the guess instead of vanishing.
              await _watchLateStartVerdict(
                session.startSync(),
                gen,
              ).timeout(walletFfiWedgeTimeout);
              _startFailed = false;
              if (!_disposed && gen == _generation) {
                state = WalletSyncDrive.running;
              }
            } else {
              await session.stopSync().timeout(walletFfiWedgeTimeout);
              if (!_disposed && gen == _generation) {
                // #383 R1: a stop that settles while the host's policy is OFF must
                // land on the honest `disabledByHost`, not `suspended` — otherwise
                // a policy-off pause→resume leaves `suspended` standing (the
                // resume start is policy-gated and writes nothing), and the badge
                // would fall through to a retained healthy story instead of
                // "Sync off".
                //
                // #403 R5 extends the SAME rule to a FAILED start, for the same
                // reason. `stopSync` on a loop that never started is a no-op and
                // reports Ok, so this used to overwrite `failed` with `suspended`
                // — and `suspended` is a passes-run state, so every background /
                // resume cycle on a persistently failing start flashed the
                // UNQUALIFIED money promise ("your wallet will send this on a
                // later sync") until the resume's own start failed again. The
                // conjunction in `walletSyncPassesRunProvider` does NOT close that
                // on its own: it only holds when the HOST policy is off, and here
                // the policy reads `true`. The honest reason wins over the
                // lifecycle one — nothing renders while backgrounded, and a resume
                // re-issues the start, so preserving `failed` costs nothing and a
                // successful retry clears it immediately.
                state = !ref.read(walletSyncPolicyProvider)
                    ? WalletSyncDrive.disabledByHost
                    : state == WalletSyncDrive.failed
                    ? WalletSyncDrive.failed
                    : WalletSyncDrive.suspended;
              }
            }
          } catch (_) {
            if (_disposed || gen != _generation) return;
            // A START failure is the honest `failed` (the loop is NOT running). A
            // STOP failure leaves the loop most likely STILL running, so the honest
            // state is `running` — never `failed` (that renders a "couldn't start"
            // notice and would mislead). `stopSync` is a no-op-safe SDK call, so
            // this branch is rare. Either way: caught broadly (any failure → one
            // honest, recoverable state; never a raw code, invariant 6).
            // the stop-failure arm mirrors the settle ternary above — a
            // policy-off pause whose stop throws must not overwrite
            // `disabledByHost` with `running` (the policy story wins over
            // transients, the :210 resume-belt precedent; under policy-off the
            // loop never ran, so `running` would also just be false).
            // #409 R2 (review MINOR): the STOP arms must preserve `failed` the
            // same way the success settle above does (#403 R5 / #407 R7).
            // Without it a wedged or throwing background stop PROMOTES a drive
            // that legitimately failed to start into `running`, flipping
            // `walletSyncPassesRunProvider` back to true — the unqualified
            // "your wallet will send this on a later sync" promise, live again
            // over a loop that never started, with no start attempt in between.
            if (start) _startFailed = true;
            state = start
                ? WalletSyncDrive.failed
                : (state == WalletSyncDrive.failed
                      ? WalletSyncDrive.failed
                      : ref.read(walletSyncPolicyProvider)
                      ? WalletSyncDrive.running
                      : WalletSyncDrive.disabledByHost);
          }
          // The trailing belt makes "the chain never rejects" STRUCTURAL
          // the catch arm above reads a HOST-overridable
          // provider — a throwing override would otherwise rethrow out of the
          // catch, reject this link, and silently skip every queued command
          // until a dispose link healed the chain.
        })
        .catchError((Object _) {});
  }
}
