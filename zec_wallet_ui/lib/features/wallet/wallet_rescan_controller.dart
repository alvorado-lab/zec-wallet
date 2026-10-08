import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'onboarding/onboarding_controller.dart';
import 'onboarding/onboarding_state.dart';
import 'onboarding/wallet_provisioner.dart' show RescanTarget;
import 'wallet_providers.dart';
import 'wallet_sync_controller.dart' show walletSyncPassesRunProvider;

/// The rescan-recovery presentation state (ADR-0534 / FR-1b). Distinct from the
/// onboarding state machine (which owns the session swap) and the `SyncStatus`
/// (which reports scan progress): this is purely how the wallet surface presents
/// a user-initiated "scan from earlier / scan all history" recovery.
///
/// WHY a separate controller from [OnboardingController]: the "rebuilding" cue is
/// a sync-PRESENTATION concern, not a provisioning phase, and — crucially — it
/// must SURVIVE the session swap the rescan performs. This controller does NOT
/// watch `walletSessionProvider`, so it is NOT torn down + rebuilt when the
/// active session changes; it only `listen`s to the sync status to learn when the
/// repopulating scan has caught up. One source of truth for the rescan UX.
sealed class WalletRescanState {
  const WalletRescanState();
}

/// No rescan in progress (the default) — the activity list renders normally.
class WalletRescanIdle extends WalletRescanState {
  const WalletRescanIdle();
}

/// The FFI rebuild is running (stops + joins sync, rebuilds the data DB). Brief +
/// local; the confirm sheet shows a spinner. The wallet surface still shows the
/// PRIOR session's data until the swap lands.
class WalletRescanRunning extends WalletRescanState {
  const WalletRescanRunning();
}

/// The rebuild SUCCEEDED and the wallet is re-scanning from the lower birthday.
/// The balance + activity are EMPTY/partial until sync reaches the tip; the
/// surface shows an honest "rebuilding your history" cue — DISTINCT from the
/// "no activity yet" empty card — so the user knows their funds are safe and the
/// history is being recovered (review P1 #4: a user must not watch their whole
/// history vanish unexplained). Cleared on reached-tip.
class WalletRescanRebuilding extends WalletRescanState {
  const WalletRescanRebuilding({required this.target});

  /// WHICH recovery is repopulating, for the cue copy: all-history, a picked
  /// date, or the wallet's own start (#317 — the sealed intent keeps the
  /// banner honest about what the user actually chose).
  final RescanTarget target;
}

/// The SDK refused the rescan because a send is still settling on-chain (the
/// §4.4 witness-inversion fence — rebuilding under it could double-pay). The
/// wallet is fully usable at its prior state; the cue is HOURS-scale ("keep
/// the app online" — resolution needs sync passes), distinct from
/// [WalletRescanFailed]'s "try again in a moment".
class WalletRescanBlockedBySettlingSend extends WalletRescanState {
  const WalletRescanBlockedBySettlingSend();
}

/// The rescan was REFUSED at its commit point because no sync pass will run
/// (#405): either the host's sync policy is off or the start command failed, so
/// the rebuild this wipe depends on cannot execute. Nothing destructive ran —
/// the wallet is untouched at its full prior state, which is why this is a
/// distinct state from [WalletRescanFailed] (whose wallet may have been rebuilt
/// at a lower birthday, #379) and why its copy claims "unchanged" outright.
///
/// CAUSE-AGNOSTIC by contract (see [walletSyncPassesRunProvider]): the notice
/// names the CONDITION, never the remedy — "turn syncing on in settings" is
/// wrong for a failed start, and the badge + start-failed notice above it on
/// the same surface already carry the cause and its way out.
class WalletRescanBlockedBySyncNotRunning extends WalletRescanState {
  const WalletRescanBlockedBySyncNotRunning();
}

/// The rescan failed because the disk is FULL (the SDK's typed `DiskFull` —
/// the rebuild + WAL fold need headroom routine commits don't, so the sync
/// badge stays HEALTHY while a retry deterministically re-fails; #375). The
/// wallet is fully usable — at its prior state on the common pre-rename
/// faults, or at the REBUILT lower-birthday state if the DiskFull hit after
/// the rebuild's atomic rename (#379: balance/history repopulate via sync,
/// which is why the cue claims funds-safety, never "unchanged"); the cue is
/// ACTIONABLE ("free up space"), distinct from [WalletRescanFailed]'s "try
/// again in a moment".
class WalletRescanFailedNeedsSpace extends WalletRescanState {
  const WalletRescanFailedNeedsSpace();
}

/// The rescan FAILED but the wallet was recovered — re-opened at its prior
/// birthday on the common pre-rename faults, or at the REBUILT lower birthday
/// if the fault hit after the atomic rename (#379; NO funds lost either way —
/// see [RescanOutcome.failedRecovered] for the two-arm story). An honest,
/// dismissable cue; the wallet stays fully usable. (A failure that ALSO
/// couldn't re-open routes the whole surface to OnboardingFailed, so this
/// state is never used for that case.)
class WalletRescanFailed extends WalletRescanState {
  const WalletRescanFailed();
}

/// Drives the rescan-recovery flow (FR-1b). Root-scoped (a plain, non-autoDispose
/// [NotifierProvider]) so the "rebuilding" cue persists across the session swap a
/// rescan performs — it is built lazily when the wallet surface first watches it
/// and lives for the container's life thereafter.
final walletRescanControllerProvider =
    NotifierProvider<WalletRescanController, WalletRescanState>(
      WalletRescanController.new,
    );

class WalletRescanController extends Notifier<WalletRescanState> {
  /// Touching `ref`/`state` after dispose throws; the async [rescan]
  /// continuation re-checks this. Reset on every build (Riverpod may reuse the
  /// instance), so it is never sticky-true.
  bool _disposed = false;

  @override
  WalletRescanState build() {
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    // Clear the rebuilding cue once sync reaches the tip — the history has fully
    // repopulated. `listen` (not `watch`): this notifier must NOT rebuild on sync
    // status (that would reset the cue on every scan tick); it only reacts to the
    // reached-tip EDGE. `syncStatusProvider` is itself rebuilt across the session
    // swap, but this listen re-registers against the live one, so the post-rescan
    // session's UpToDate is what clears the cue.
    ref.listen<AsyncValue<SyncStatus>>(syncStatusProvider, (_, next) {
      if (state is WalletRescanRebuilding &&
          next.value is SyncStatus_UpToDate) {
        state = const WalletRescanIdle();
      }
    });
    // Reset when the WALLET ITSELF dies (#380 — the #377 identity leak):
    // this presentation describes ONE wallet's recovery, but the controller
    // deliberately outlives the session swap, so without this a failure notice
    // would outlive a delete, and a Rebuilding banner would render over the
    // NEXT wallet's surface until ITS first reached-tip. The no-wallet-on-disk
    // phases (Welcome / the restore form, and their in-flight successors) are
    // exactly "the wallet this state described is gone". A rescan's own swap
    // never leaves Active, so the survive-the-swap guarantee is untouched —
    // as is a delete/rescan FAULT recovery (it re-probes back to a
    // wallet-holding state without passing through these).
    ref.listen<OnboardingState>(onboardingControllerProvider, (_, next) {
      final walletGone =
          next is OnboardingWelcome ||
          next is OnboardingRestoreInput ||
          next is OnboardingGenerating ||
          next is OnboardingRestoring;
      if (walletGone && state is! WalletRescanIdle) {
        state = const WalletRescanIdle();
      }
    });
    return const WalletRescanIdle();
  }

  /// Trigger a rescan from [target] (all-history / a picked date / the
  /// wallet's own start — see [RescanTarget]). Idempotent against a
  /// double-tap: a second call while the FFI rebuild is [running] no-ops (the
  /// OnboardingController also single-flights — defence in depth). Never
  /// throws: the resulting [WalletRescanState] IS the surface.
  Future<void> rescan(RescanTarget target) async {
    if (state is WalletRescanRunning) return; // re-entrancy guard (double-tap)
    // COMMIT-point fence (security MED): the overflow-menu disable is
    // ENTRY-only, and the drive/policy behind it is reactive — a sheet opened
    // while sync was healthy can straddle a host flip OR a failed start and
    // confirm here. A rescan wipes the DB and can only rebuild via a sync pass:
    // confirming would strand a zeroed wallet whose only explanation is a badge
    // occluded by this very sheet. Refuse VISIBLY (a silent dead confirm is a
    // broken promise; the auto-shield pre-prompt fence precedent). Deliberately
    // checked at the COMMIT, before any destructive call. (The swap deep-scan's
    // widen is NOT fenced the same way on purpose: a widen is a non-destructive
    // durable marker that simply completes when sync resumes.)
    //
    // #405: reads the SSOT, NOT `walletSyncPolicyProvider`. A start command
    // that FAILED leaves the policy reading `true` while the loop never ran and
    // never will without a retry — so the policy-only fence PASSED exactly the
    // confirm it exists to stop, and the destructive wipe went ahead behind a
    // "Rebuilding" promise nothing could keep. It is refused on its OWN state
    // (never the shared `WalletRescanFailed`): nothing ran, so the wallet is
    // genuinely unchanged and the copy may say so.
    if (!ref.read(walletSyncPassesRunProvider)) {
      state = const WalletRescanBlockedBySyncNotRunning();
      return;
    }
    state = const WalletRescanRunning();
    final outcome = await ref
        .read(onboardingControllerProvider.notifier)
        .rescanActiveWallet(target);
    if (_disposed) return;
    state = switch (outcome) {
      RescanOutcome.success => WalletRescanRebuilding(target: target),
      RescanOutcome.failedRecovered => const WalletRescanFailed(),
      RescanOutcome.failedNeedsSpace => const WalletRescanFailedNeedsSpace(),
      RescanOutcome.blockedBySettlingSend =>
        const WalletRescanBlockedBySettlingSend(),
      // The whole surface is now OnboardingFailed (its own retry) — render nothing
      // here. notActive is a raced/stray call — likewise nothing to present.
      RescanOutcome.failedClosed => const WalletRescanIdle(),
      RescanOutcome.notActive => const WalletRescanIdle(),
    };
  }

  /// Dismiss the failed/blocked cue (back to idle) after the user has read it.
  /// Dismissing over a still-catching-up wallet does NOT erase the explanation:
  /// [walletCatchUpCueProvider]'s durable arm re-derives it (#380 (b)) — and
  /// does so exactly when the wallet's balance/history genuinely are still
  /// filling in: always on the post-rename arms (the rebuild cleared the
  /// stamp), and on an intact pre-rename/fence arm only if that wallet had
  /// never finished a sync pass anyway (a fence hit mid-FIRST-sync — the cue
  /// is equally honest there). A previously-synced intact wallet derives no
  /// cue. A controller-side "dismiss → Rebuilding" transition could not tell
  /// these arms apart, which is why the derivation owns the handover.
  void dismissFailure() {
    if (state is WalletRescanFailed ||
        state is WalletRescanFailedNeedsSpace ||
        state is WalletRescanBlockedBySettlingSend ||
        state is WalletRescanBlockedBySyncNotRunning) {
      state = const WalletRescanIdle();
    }
  }
}

/// The catch-up presentation truth (#380) — SHOULD the wallet surface explain
/// an empty/partial balance + history as "still filling in", and with which
/// copy. The banner, the activity empty-cue, and the swap "Available" line all
/// derive from this ONE provider.
///
/// WHY a derived provider over the controller state alone: the controller is
/// process-lifetime memory, but the misread it exists to prevent (review P1
/// #4 — a user watching their history "vanish" unexplained through an
/// hours-scale catch-up) survives a process death (app update / LMK kill
/// mid-catch-up) and a failure-notice dismiss. So the cue re-derives from the
/// two DURABLE signals the wallet itself carries (the stamp is the same one
/// the boot probe's prior-activity check trusts):
///
///  * `lastSynced == null` — this wallet's data DB has never completed a sync
///    pass. The rescan rebuild clears the stamp by design and it stays null
///    until the first reached-tip, so it is precisely "balance/history not
///    yet whole". A restore's first catch-up and a brand-new wallet's brief
///    first scan honestly qualify too (their balance/history are equally
///    still filling in), so they deliberately share the cue. RESOLVED #357
///    (post-ship fix): the converse over-explanation ("synced-before yet
///    `lastSynced == null`") is carried by the durable `everSynced` flag, which
///    the core sets at reached-tip and CLEARS on a rescan (its aux row resets like
///    `sync_stamp` — everSynced is the DURABLE proved-tip latch, and that latch
///    resets on a rescan). So `everSynced == true` unambiguously means "reached
///    tip since the last rescan ⇒ balance complete": suppress the cue (fixes the
///    swallowed-stamp / pre-#317 / offline-relaunch over-explanation). A rescan
///    REBUILD clears everSynced, so it shows the cue for its whole catch-up —
///    including offline after a process death, the "did I lose funds?" panic
///    window a first-cut `everSynced && !scanning` gate wrongly suppressed.
///    `everSynced == false` (never synced, or post-rescan rebuilding) always
///    shows the cue below tip.
///  * the sync status is below the tip (anything but [SyncStatus_UpToDate]:
///    scanning, connecting, offline, stalled all keep the explanation up —
///    an offline relaunch mid-catch-up still shows an emptied wallet).
///
/// Reached-tip clears BOTH signals on the same edge that clears the
/// controller (the live status flips to up-to-date; the snapshot re-read
/// stamps `lastSynced`). The [walletSyncedTipProvider] latch backstops the
/// gap between those two: the snapshot re-read is owned
/// by widget/resume listeners, so a busy-DB re-read fault — or a host that
/// mounts only the swap surface — can retain a stale null stamp past the
/// tip, and the next routine scan/offline sample would resurrect the cue
/// over a FINAL balance. A tip this session already PROVED stays proved
/// through routine re-scan windows, so the cue never flaps back. The
/// controller state stays the intra-session FAST PATH because it carries the
/// [RescanTarget] (the banner names what the user chose); the durable arm
/// necessarily renders generic copy — the choice does not survive a relaunch.
sealed class WalletCatchUpCue {
  const WalletCatchUpCue();
}

/// Nothing to explain — the balance/history render normally.
class WalletCatchUpNone extends WalletCatchUpCue {
  const WalletCatchUpNone();
}

/// A rescan rebuild is repopulating. Two arms share this cue:
///
/// - the intra-session fast path ([WalletRescanRebuilding] is live), where
///   [target] names the recovery the user chose; and
/// - the DURABLE arm (#377 s357b-2): the core's rescan-rebuilding breadcrumb
///   says a rebuild is still catching up but the in-session choice did not
///   survive the relaunch — [target] is `null` and the banner renders the
///   generic-but-rescan-naming copy (`walletCatchUpRescanBanner`), the
///   stronger reassurance than [WalletCatchUpSyncing]'s first-run framing.
///
/// NO value equality on purpose: [RescanTarget] carries
/// none, so an `==` here would be identity-on-target and mislead. The other
/// two cue values are const-canonical (identical ⇒ no downstream notify);
/// this one is freshly allocated per derive — harmless today because the
/// fast path's early return prunes the per-tick sync dependency, so it only
/// re-derives on controller transitions (and the durable arm's allocation is
/// `const`). A future `select`/equality consumer must add real equality down
/// the [RescanTarget] hierarchy first.
class WalletCatchUpRebuilding extends WalletCatchUpCue {
  const WalletCatchUpRebuilding({required this.target});

  /// See [WalletRescanRebuilding.target]; `null` on the durable arm (the
  /// choice does not survive a relaunch — only the breadcrumb fact does).
  final RescanTarget? target;
}

/// The durable-signal arm: a never-synced wallet below the tip with no
/// intra-session rescan state (a relaunch mid-catch-up, a dismissed failure
/// notice over a rebuilt wallet, a restore/create's first sync) — generic
/// catch-up copy.
class WalletCatchUpSyncing extends WalletCatchUpCue {
  const WalletCatchUpSyncing();
}

/// See [WalletCatchUpCue]. A plain derived [Provider]: every input is already
/// live state, so this re-evaluates exactly when they change.
final walletCatchUpCueProvider = Provider<WalletCatchUpCue>((ref) {
  final rescan = ref.watch(walletRescanControllerProvider);
  if (rescan is WalletRescanRebuilding) {
    return WalletCatchUpRebuilding(target: rescan.target);
  }
  final session = ref.watch(walletSessionProvider);
  if (session == null) return const WalletCatchUpNone();
  // Last-known snapshot (`.value` — same retention idiom as the screen): a
  // transient re-read failure must not flap the cue. No snapshot yet ⇒ no
  // verdict ⇒ no cue (the screen is still on its cold-load spinner anyway).
  final snapshot = ref.watch(walletSnapshotProvider).value;
  if (snapshot == null || snapshot.lastSynced != null) {
    return const WalletCatchUpNone();
  }
  // The cold snapshot's own status carries the first frame before the live
  // stream emits (the screen's `liveStatus ?? state.syncStatus` idiom).
  final status = ref.watch(syncStatusProvider).value ?? snapshot.syncStatus;
  if (status is SyncStatus_UpToDate) return const WalletCatchUpNone();
  // #357 (post-ship reliability/UX HIGH fix): the durable everSynced flag is now
  // CLEARED on a rescan by the core (its aux row is reset like `sync_stamp`), so
  // `everSynced == true` UNAMBIGUOUSLY means "reached tip since the last rescan ⇒
  // the balance is complete" — suppress the over-explanation of a settled balance
  // behind a stale-null stamp (a swallowed stamp-write fault / pre-#317 wallet /
  // offline relaunch), with NO scanning-or-balance heuristic. A rescan REBUILD has
  // everSynced CLEARED, so it falls through and correctly shows the cue for its
  // whole catch-up — INCLUDING offline after a process death (LMK kill), the panic
  // window an earlier `status is Scanning` proxy wrongly suppressed. A never-synced
  // wallet (`everSynced == false`) likewise falls through to the first-run cue.
  if (snapshot.everSynced) return const WalletCatchUpNone();
  // A tip this SESSION already proved stays proved: the
  // stamp's visibility here rides an async snapshot re-read that can fault
  // (busy DB, `.value` retention) or simply not be wired (a host mounting
  // only the swap surface) — without this, a routine post-tip scan or an
  // offline drop would resurrect "catching up" over a FINAL balance. The
  // latch is session-keyed (resets on the rescan's own swap) and held
  // through routine re-scan windows by design, which is exactly the
  // no-flap semantic the cue needs. An INVALIDATED latch (deep-reorg
  // rewind) deliberately falls through: over a never-stamped wallet a
  // rewound re-scan genuinely is a catch-up.
  if (ref.watch(walletSyncedTipProvider) is WalletSyncedTipLatched) {
    return const WalletCatchUpNone();
  }
  // #377 s357b-2: the durable rescan-rebuilding breadcrumb — a rescan swapped
  // in and has not reached tip since (the core clears it at the first
  // post-rescan clean pass). The in-session fast path above already handled a
  // LIVE rescan with its named target; reaching here with the breadcrumb set
  // means the choice was lost (process death mid-rebuild — the LMK-kill
  // window), so name the rescan generically rather than falling through to
  // the first-run framing. Checked AFTER everSynced and the session latch:
  // both prove the rebuild is over, so a stale breadcrumb (a swallowed
  // clear fault at tip) can never pin this banner over a settled balance.
  if (snapshot.rescanRebuilding) {
    return const WalletCatchUpRebuilding(target: null);
  }
  return const WalletCatchUpSyncing();
});
