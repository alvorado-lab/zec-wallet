import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'wallet_providers.dart';

/// How long a Stalled-posture EDGE must hold before the display adopts it
/// (#399 item 3). FR-21's sub-second dial verdicts made the raw status flap
/// Stalled↔Scanning on every scheduled retry of a dead link: the badge
/// flickered, the liveRegion re-announced the pair each cycle (screen-reader
/// chatter), and the send screen's pinned queue button inserted/removed under
/// the user's thumb. The dwell must comfortably outlast one whole doomed retry
/// pass (dial verdict ≲1s + status churn) so the blip is absorbed, while
/// staying short enough that a REAL posture change still reads prompt.
const walletStallPostureDwell = Duration(milliseconds: 2500);

/// How long the display mirrors the raw status LIVE after a "Try now" tap
/// (#399 item 4; review M2). The tap's feedback must be the truth in
/// motion — behind the dwell, a successful restart would sit under a held
/// "Sync paused" for a full [walletStallPostureDwell] (a dead-feeling tap,
/// the class). The window spans one whole doomed restart cycle (dial
/// verdict ≲1s + status churn) so a FAILED Try now visibly returns to the
/// stall too — honest in both directions — then the flap-absorbing dwell
/// resumes.
const walletTryNowLiveWindow = Duration(milliseconds: 4000);

/// The POSTURE-STABLE view of [syncStatusProvider] for display surfaces (the
/// badge, the sync sheet, the send screen's queue affordance — all three watch
/// THIS so they can never flicker apart). Controllers and logic listeners
/// (rescan tip-edge, synced-tip latch, refresh edges) stay on the RAW provider:
/// the debounce is a presentation policy, never a truth filter. The ONE named
/// exception is the wallet tab's tip follow (`walletTipFollowProvider`): it
/// decides whether the sync BAR shows and whether the coin spins as the bar
/// hides, so it must read what the bar renders (S12; reading raw made a
/// flaky link flicker). Do not move it back to raw.
///
/// The rule (pure — see [nextDisplaySyncStatus]): mirror the raw status
/// exactly, EXCEPT the two edges of the Stalled posture, which must survive
/// [walletStallPostureDwell] before the display follows:
///
///  * non-Stalled → Stalled: hold the previous shown state. A transient
///    hiccup that recovers within the dwell never raises the alarm posture
///    (the sub-second "snap to Sync paused" the review measured).
///  * shown-Stalled → Scanning/Connecting/Idle/Unknown: hold Stalled. A
///    retry pass CLAIMS an attempt, it proves nothing — during a dead-link
///    backoff the raw status blips through Scanning for well under a second
///    on every retry, and following it instantly would re-create the exact
///    flap (plus doubled liveRegion announcements) this layer exists to kill.
///    A pass that survives the dwell is really delivering → adopt it.
///  * shown-Stalled → UpToDate or Offline: INSTANT. UpToDate is PROOF of
///    health (a completed pass — recovery must never lag the truth); the
///    reserved Offline arm is the calmer sibling posture, not a flap risk.
///  * Stalled → Stalled with a DIFFERENT reason: instant. That is new
///    information inside the same posture (e.g. connectivity → storage full,
///    which must promote to RED now), not flicker.
///
/// A user-initiated "Try now" (the sync sheet, #399 item 4) calls
/// [WalletDisplaySyncStatusNotifier.followRawNow]: the user acted, so live
/// truth is the only honest feedback — the pending dwell is dropped, the
/// raw value adopted on the spot, and for [walletTryNowLiveWindow] every
/// raw change mirrors instantly (the user is WATCHING; a held stale posture
/// there reads as a dead tap — review M2). When the window closes the
/// normal dwell edges resume.
final walletDisplaySyncStatusProvider =
    NotifierProvider<WalletDisplaySyncStatusNotifier, AsyncValue<SyncStatus>>(
      WalletDisplaySyncStatusNotifier.new,
    );

/// Which dwell (if any) a raw transition needs before the display follows —
/// pure, so the notifier's timer plumbing and the truth-table tests share one
/// rule that cannot drift.
enum DisplayStatusEdge {
  /// Mirror the raw value immediately.
  adopt,

  /// Hold the shown value; adopt the raw side only if it survives the dwell.
  dwell,
}

/// The #399 posture-dwell truth table. [shown] is the currently displayed
/// status, [raw] the new raw emission. Total over every pair; never throws.
DisplayStatusEdge nextDisplaySyncStatus(SyncStatus shown, SyncStatus raw) {
  final shownStalled = shown is SyncStatus_Stalled;
  final rawStalled = raw is SyncStatus_Stalled;
  if (shownStalled == rawStalled) {
    // Same posture side — including Stalled→Stalled reason changes (new
    // information, not flicker) and every healthy↔healthy move.
    return DisplayStatusEdge.adopt;
  }
  if (rawStalled) {
    // Entering the alarm posture: must survive the dwell (transient-hiccup
    // grace — item 3's "dwell before promoting").
    return DisplayStatusEdge.dwell;
  }
  // Leaving the shown alarm posture: proof (a completed pass) and the calm
  // reserved-Offline sibling flip instantly; attempt CLAIMS (Scanning /
  // Connecting / Idle / Unknown) must survive the dwell — they are exactly
  // the sub-second retry blips the debounce exists to absorb.
  return raw is SyncStatus_UpToDate || raw is SyncStatus_Offline
      ? DisplayStatusEdge.adopt
      : DisplayStatusEdge.dwell;
}

class WalletDisplaySyncStatusNotifier extends Notifier<AsyncValue<SyncStatus>> {
  Timer? _pending;

  /// While true (the [walletTryNowLiveWindow] after a Try-now tap), every raw
  /// emission is adopted verbatim — no dwell. Cleared by [_followTimer].
  bool _followRaw = false;
  Timer? _followTimer;

  /// Post-dispose timer safety (the [SyncStatusNotifier] idiom): riverpod
  /// REUSES the notifier across a rebuild and fires onDispose first, so the
  /// flag is reset at the top of every build.
  bool _disposed = false;

  void _resetTimers() {
    _pending?.cancel();
    _pending = null;
    _stopFollow();
  }

  void _stopFollow() {
    _followTimer?.cancel();
    _followTimer = null;
    _followRaw = false;
  }

  @override
  AsyncValue<SyncStatus> build() {
    _disposed = false;
    _resetTimers();
    ref.onDispose(() {
      _disposed = true;
      _resetTimers();
    });
    ref.listen<AsyncValue<SyncStatus>>(syncStatusProvider, (_, next) {
      _onRaw(next);
    });
    // Seed from the current raw value verbatim: the dwell exists for EDGES —
    // a cold start straight into a stalled wallet shows the stall right away
    // (honest), it never waits out a dwell that has no flicker to absorb.
    return ref.read(syncStatusProvider);
  }

  void _onRaw(AsyncValue<SyncStatus> next) {
    if (_disposed) return;
    final shown = state.value;
    final raw = next.value;
    // A value-less envelope is the raw provider REBUILDING (the session-flip
    // loading emit) or a stream fault: any post-Try-now live window belongs
    // to the OLD context — a pre-flip tap must not bypass the NEW identity's
    // dwell discipline (audit D). Mirror the envelope verbatim.
    if (raw == null) {
      _stopFollow();
      _adopt(next);
      return;
    }
    // The pre-first-value window mirrors verbatim (the debounce is only ever
    // between two DATA values — the flap it kills is a value↔value flap),
    // and so does the post-Try-now live window — the user is watching their
    // own action play out (see [walletTryNowLiveWindow]).
    if (shown == null || _followRaw) {
      _adopt(next);
      return;
    }
    switch (nextDisplaySyncStatus(shown, raw)) {
      case DisplayStatusEdge.adopt:
        _adopt(next);
      case DisplayStatusEdge.dwell:
        // An armed dwell KEEPS its clock — never restarted by later
        // dwell-side emissions. The clock measures CONTINUOUS time on the
        // opposite posture side: while it runs, [shown] cannot change (only
        // [_adopt] changes it, and _adopt cancels this timer), so every
        // later emission classifies against the same shown — a bounce-back
        // to shown's side lands on the ADOPT arm and cancels; anything else
        // is still the same posture edge, just a newer sample of it. This
        // is LOAD-BEARING (review H1): a delivering recovery pass
        // emits Scanning with ADVANCING from/to/percent every few blocks —
        // far faster than the dwell — so re-arming on value change starved
        // adoption for the entire catch-up and pinned "Sync paused" over a
        // healthy scan. At fire time the current raw is adopted VERBATIM
        // (the freshest sample of the side that survived).
        if (_pending != null) return;
        _pending = Timer(walletStallPostureDwell, () {
          if (_disposed) return;
          _pending = null;
          _adopt(ref.read(syncStatusProvider));
        });
    }
  }

  void _adopt(AsyncValue<SyncStatus> next) {
    _pending?.cancel();
    _pending = null;
    state = next;
  }

  /// Drop any pending dwell, mirror the raw provider RIGHT NOW, and stay
  /// LIVE for [walletTryNowLiveWindow] — the user-initiated escape (the sync
  /// sheet's "Try now", #399 item 4): after an explicit action the user is
  /// watching, so live truth is the only honest feedback in BOTH directions
  /// (a successful restart shows its first pass immediately; a failed one
  /// visibly returns to the stall) — a dwell-held stale posture there reads
  /// as a dead tap (the lesson; review M2). A repeat tap simply
  /// restarts the window.
  void followRawNow() {
    if (_disposed) return;
    _followRaw = true;
    _followTimer?.cancel();
    _followTimer = Timer(walletTryNowLiveWindow, () {
      if (_disposed) return;
      _followTimer = null;
      _followRaw = false;
    });
    _adopt(ref.read(syncStatusProvider));
  }

  /// End the post-Try-now live window NOW (idempotent). Called when the sync
  /// sheet CLOSES (audit D): the window's whole rationale is "the user
  /// is watching their own tap play out", and once the sheet is dismissed
  /// nobody is — letting the window run on would spill up to 4s of raw flap
  /// onto the badge's now-unblocked liveRegion (a full stall label re-read
  /// per blip) and the send screen's pinned queue affordance. Closing it
  /// cannot re-create the dead tap the window fixes: the dwell resumes only
  /// once the watched surface is gone.
  void stopFollowingRaw() {
    if (_disposed) return;
    _stopFollow();
  }
}
