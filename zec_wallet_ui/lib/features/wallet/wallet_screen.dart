import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart'
    show HapticFeedback, SystemUiOverlayStyle;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart' show DateFormat;
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/router/wallet_routes.dart';
import 'in_flight_swaps_section.dart';
import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../core/theme/shapes.dart';
import '../../core/theme/theme.dart' show overlayStyleFor;
import '../../core/theme/typography.dart';
import '../../shared/action_row_layout.dart';
import '../../shared/wallet_loading.dart';
import '../../shared/wallet_notice.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'arrival_cue.dart';
import 'onboarding/onboarding_controller.dart';
import 'onboarding/onboarding_providers.dart';
import 'onboarding/onboarding_state.dart';
import 'onboarding/onboarding_store.dart' show DeepScanRestoreNoteState;
import 'onboarding/onboarding_views.dart';
import 'onboarding/wallet_provisioner.dart'
    show
        RescanAllHistory,
        RescanFromTime,
        RescanFromWalletBirthday,
        RescanTarget;
import 'ephemeral_sweep.dart';
import 'move_to_transparent/move_to_transparent_sheet.dart';
import 'in_flight_sends_section.dart';
import 'hide_balance.dart';
import 'labeled_zat_row.dart';
import 'parked_sends_section.dart';
import 'recover_ephemeral_action.dart';
import 'recoverable_ephemeral.dart';
import 'rescan_sheet.dart';
import 'swap_deep_scan.dart';
import 'shield/shield_sheet.dart';
import 'swap/swap_enabled_provider.dart';
import 'sync_server_sheet.dart';
import 'sync_status_presentation.dart';
import 'sync_status_sheet.dart';
import 'transparent_funds/auto_shield_controller.dart';
import 'transparent_funds/transparent_funds_sheet.dart';
import 'tx_detail_sheet.dart';
import 'wallet_activity_controller.dart';
import 'wallet_coin.dart';
import 'wallet_tip_follow.dart';
import 'wallet_display_sync_status.dart';
import 'wallet_health.dart';
import 'wallet_providers.dart';
import 'wallet_rescan_controller.dart';
import 'reconnect_kick.dart';
import 'wallet_sync_controller.dart';
import 'zat_format.dart';

/// The wallet surface (spec §3.3): a live, honest sync indicator + balance.
/// Rendering layer ONLY — every value comes from the Rust core via
/// [walletSnapshotProvider] (cold) and [syncStatusProvider] (live); nothing
/// is stored here (design invariant 1).
///
/// Honest degradation (invariant 6): with no wallet provisioned this build
/// SAYS so rather than faking an empty wallet — and crucially never invites
/// a deposit before recovery-phrase backup exists (money-safety).
class WalletScreen extends ConsumerWidget {
  const WalletScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    // The deposit gate's single reader: a WalletSession is exposed ONLY in
    // OnboardingActive (walletSessionProvider derives that). When present — the
    // active path, and the path the screen tests inject directly — render the
    // live wallet; otherwise render the matching onboarding phase. A wallet is
    // never shown as deposit-ready before its backup is confirmed-and-persisted.
    final session = ref.watch(walletSessionProvider);
    // Host seam: the Settings entry points (the seam keeps its historical
    // "appearance" name) render only when the host wired a destination (the
    // package has no settings surface of its own).
    final appearanceRoute = ref.watch(walletAppearanceRoutePathProvider);
    // The ACTIVE wallet is a tab root in the refreshed design (FR-49 S12, C1):
    // a large title that scrolls with the content, drawn by _WalletActive's
    // own header — no AppBar. The status-bar icon brightness the AppBar used to
    // set is applied here instead.
    if (session != null) {
      return Scaffold(
        body: AnnotatedRegion<SystemUiOverlayStyle>(
          value: overlayStyleFor(WalletColors.of(context)),
          child: const SafeArea(child: _WalletActive()),
        ),
      );
    }
    // Pre-Active (onboarding; `session` is null here): the plain title and a
    // bare Settings action, so the host's settings stay reachable
    // before a wallet exists (when the wallet is the host's home there is no
    // separate settings surface). The active wallet's header — the watch-only
    // chip, the eye and the overflow menu — is `_WalletHeader` above.
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.walletTitle),
        actions: [
          if (appearanceRoute != null)
            IconButton(
              tooltip: l10n.walletAppearanceMenuItem,
              icon: const WalletIcon(WalletGlyph.appearance),
              onPressed: () => context.push(appearanceRoute),
            ),
        ],
      ),
      body: SafeArea(child: _onboarding(ref, l10n)),
    );
  }

  /// The session-less branch: render the onboarding phase. `walletSessionProvider`
  /// is null in every phase but Active, so this is the create → back-up → confirm
  /// flow (plus the honest not-set-up / failed / busy states). The gate is the
  /// controller state, read here once — no separate router redirect: the wallet
  /// is this example's home surface, so an in-surface state switch (rather than a
  /// whole-app router unlock-gate) is the honest, reactive minimum — it
  /// re-renders the instant the controller advances.
  Widget _onboarding(WidgetRef ref, WalletLocalizations l10n) {
    final state = ref.watch(onboardingControllerProvider);
    return switch (state) {
      // Boot probe + open in flight (wallet on disk? backup confirmed?). Usually
      // brief, but the open can lawfully wait out a transiently-held wallet lock
      // for up to ~27.5 s, so it gets the LABELED
      // busy view — never a mute spinner for a wait that long (UX review).
      OnboardingLoading() => WalletOnboardingBusyView(
        label: l10n.walletOpeningLabel,
      ),
      // No onboarding backend wired (production default): honest not-set-up.
      OnboardingUnavailable() => const _WalletNotSetUp(),
      OnboardingWelcome() => const WalletWelcomeView(),
      OnboardingGenerating() => WalletOnboardingBusyView(
        label: l10n.walletGeneratingLabel,
      ),
      // Both the input and the transient restoring phase render the SAME restore
      // view (the const canonicalizes to ONE element, so the typed phrase in its
      // State survives the restore round-trip — a fixable fault returns here with
      // the phrase intact; on success Active replaces it).
      OnboardingRestoreInput() => const WalletRestoreView(),
      OnboardingRestoring() => const WalletRestoreView(),
      // #397 §3.7 D5: both the watch-only input and the transient creating phase
      // render the SAME view (the const canonicalizes to ONE element, so the
      // pasted key + picked date survive the create round-trip — a fixable fault
      // returns here intact; on success Active replaces it).
      OnboardingWatchOnlyInput() => const WalletWatchOnlyView(),
      OnboardingCreatingWatchOnly() => const WalletWatchOnlyView(),
      // Both the awaiting and the transient confirming phase render the backup
      // view (the const canonicalizes to ONE element, so its State — and the
      // screen-security request — survive the confirm round-trip; on a persist
      // failure it shows the honest cue in place, on success Active replaces it).
      OnboardingAwaitingBackup() => const WalletBackupView(),
      OnboardingConfirming() => const WalletBackupView(),
      // Unreachable here (Active ⇒ session non-null ⇒ the branch above), but the
      // switch must be exhaustive; render the live surface defensively.
      OnboardingActive() => const _WalletActive(),
      OnboardingFailed(:final kind) => WalletOnboardingFailedView(kind: kind),
    };
  }
}

/// The wallet app-bar overflow menu (rendered only when Active). Houses the
/// advanced/recovery actions kept off the main surface — the rescan-recovery
/// entry (FR-1b), the Send expert layer, and the always-available one-time-
/// address check. A [PopupMenuButton] so a future "Settings" / "Reveal
/// recovery phrase" item slots in without re-styling the bar. A
/// [ConsumerWidget] so the one-time check can watch the SHARED sweep latch
/// (disabled while any sweep runs, whichever surface launched it).
class _WalletOverflowMenu extends ConsumerWidget {
  const _WalletOverflowMenu({this.syncStatus});

  /// While the sync bar is HIDDEN (synced and healthy — FR-49 S12, C2), the
  /// sync-status sheet it opens stays reachable from here: the live
  /// headline ("Up to date") and its glyph, the state's own words, so the
  /// entry needs no string of its own. Null while the bar is visible.
  final ({WalletGlyph glyph, String headline})? syncStatus;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final sweepInFlight = ref.watch(ephemeralSweepInFlightProvider);
    // #390 — the "Check older swap addresses" deep scan shares this pattern: its
    // own single-flight latch drives the disabled/"Checking…" cue, and it is
    // mutually exclusive with a rescan (both re-register + re-poll the transparent
    // set), so a running/rebuilding rescan disables it too.
    final deepScanInFlight = ref.watch(swapDeepScanInFlightProvider);
    // rel-M2: only an actively-working rescan (Running/Rebuilding) re-polls the
    // transparent set, so only THAT disables the deep-scan entry — a dismissible
    // rescan FAILURE / blocked notice must not block it.
    final rescanState = ref.watch(walletRescanControllerProvider);
    final rescanBusy =
        rescanState is WalletRescanRunning ||
        rescanState is WalletRescanRebuilding;
    // Whether the PACKAGE manages this wallet's custody (its own onboarding /
    // provisioner). A host that brings its own provisioning and overrides
    // `walletSessionProvider` directly (the session-only configuration) leaves
    // this seam null — the affordances that route through the package's
    // onboarding machine (rescan, the Security screen's custody + delete)
    // would silently no-op there, so they HIDE instead (an affordance that
    // does nothing is a broken promise on a money surface). The session-driven
    // entries (move-to-transparent, the one-time-address check) stay: they run
    // entirely over the live WalletSession.
    final packageCustody = ref.watch(walletProvisionerProvider) != null;
    // #397 §3.7 D3/D5: a WATCH-ONLY wallet cannot spend, shield, swap, rescan,
    // or reclaim — HIDE those recovery/expert entries (the SDK refuses them
    // typed regardless; this keeps the menu honest). The transparent-funds
    // policy sheet, Sync server, Settings, and the Security screen stay.
    final watchOnly = ref.watch(isWatchOnlyProvider);
    // (the R1 gate): rescan and the swap deep scan both complete ONLY
    // via the sync loop — a rescan wipes the DB and rebuilds as sync scans; a
    // widen marks ranges the next sync pass covers. With no pass coming neither
    // can ever finish, so offering them is a destructive dead-end (the rescan
    // case: balance wiped to zero behind a "Rebuilding" promise that cannot
    // execute). Disabled, not hidden — the entries return the moment a pass is
    // possible again.
    //
    // #405: the SSOT, not `disabledByHost` alone. A start command that FAILED
    // is equally unable to run a pass while the POLICY still reads on, so the
    // policy-shaped read left both entries ENABLED on exactly the wallet that
    // could not honour them — and the rescan's commit fence, keyed the same
    // way, let the wipe through. Cause-agnostic by contract: the hint states
    // the condition, the badge and the start-failed notice own the cause.
    final syncNotRunning = !ref.watch(walletSyncPassesRunProvider);
    // Watched in build (not read inside itemBuilder — a menu built from a
    // stale read wouldn't rebuild if a host ever wired this seam reactively);
    // the callbacks below close over this build's value.
    final appearanceRoute = ref.watch(walletAppearanceRoutePathProvider);
    return PopupMenuButton<_WalletMenuAction>(
      // Honest a11y: the bare icon button is invisible to screen readers without
      // a tooltip/label (flutter-patterns § Widget Conventions).
      tooltip: l10n.walletMenuTooltip,
      onSelected: (action) {
        switch (action) {
          case _WalletMenuAction.syncStatus:
            unawaited(showSyncStatusSheet(context));
          case _WalletMenuAction.syncServer:
            // The picker straight from the menu (S15 iPhone walk: behind the
            // sync status row alone, a user did not find it).
            unawaited(showSyncServerSheet(context));
          case _WalletMenuAction.transparentFunds:
            // The transparent-funds POLICY home (§3.2i-3): plain-factual
            // explanation + the expert gate + (gated) the auto-shield switch
            // and the promoted unshield entry. Not a money form.
            unawaited(showTransparentFundsSheet(context));
          case _WalletMenuAction.moveToTransparent:
            // The Send expert layer (§3.2i-1) — a deliberate de-shield to the
            // wallet's OWN t-addr. Fire-and-forget; the sheet drives the outcome.
            unawaited(showMoveToTransparentSheet(context));
          case _WalletMenuAction.rescan:
            // Fire-and-forget — the sheet drives the outcome onto the surface.
            unawaited(showWalletRescanSheet(context));
          case _WalletMenuAction.checkOneTimeAddresses:
            // v-5c finding #3 — the ALWAYS-AVAILABLE entry to the manual
            // one-time-address sweep. The balance-card "Recover now" is gated
            // on the automatic windowed detect reading non-empty, so the case
            // the manual sweep uniquely exists for — a return PAST the detect
            // window, or to an already-used one-time address the windowed
            // detect skips — was unreachable without this. Same shared flow +
            // single-flight latch as Recover-now; fire-and-forget (the dialog
            // + snackbar drive the outcome).
            unawaited(confirmAndSweepEphemeral(context, ref));
          case _WalletMenuAction.checkOlderSwapAddresses:
            // #390 — the user-triggered deep scan recovering a seed-only
            // restore's older swap deposits/refunds. Session-driven (not
            // package-custody gated); fire-and-forget (the sheet + snackbar
            // drive the outcome).
            unawaited(showSwapDeepScanSheet(context));
          case _WalletMenuAction.appearance:
            // The host's settings page ("Settings"). The menu item renders
            // only when the host wired the seam, but check defensively.
            if (appearanceRoute != null) context.push(appearanceRoute);
          case _WalletMenuAction.security:
            // Pushed sub-route (a back button returns here); on a successful
            // delete it navigates to /wallet, which re-renders onboarding.
            context.push(WalletRoutes.security);
        }
      },
      itemBuilder: (context) => [
        if (syncStatus case final s?)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.syncStatus,
            child: Row(
              children: [
                WalletIcon(s.glyph, size: 20),
                const SizedBox(width: 12),
                Flexible(child: Text(s.headline)),
              ],
            ),
          ),
        // The server picker by its own name (S15 iPhone walk). The title
        // string is the picker's own, so no new translation. Ungated: this
        // menu exists only on the active wallet, which always has the
        // session the picker needs.
        PopupMenuItem<_WalletMenuAction>(
          value: _WalletMenuAction.syncServer,
          child: Row(
            children: [
              const WalletIcon(WalletGlyph.connecting, size: 20),
              const SizedBox(width: 12),
              Flexible(child: Text(l10n.walletSyncServerSheetTitle)),
            ],
          ),
        ),
        // The transparent-funds sheet is entirely SPEND-framed (its auto-shield
        // toggle + move-to-transparent both de-shield/shield) — hidden for a
        // watch-only wallet, matching move/rescan/reclaim below. The balance
        // card still shows the transparent split + its public-visibility note,
        // so no holdings information is lost (#397 §3.7 D3/D5, UX-M2).
        if (!watchOnly)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.transparentFunds,
            child: Row(
              children: [
                const WalletIcon(WalletGlyph.transparentFunds, size: 20),
                const SizedBox(width: 12),
                Flexible(child: Text(l10n.walletTransparentFundsMenuItem)),
              ],
            ),
          ),
        // Move-to-transparent is a SPEND (de-shield) — hidden for watch-only.
        if (!watchOnly)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.moveToTransparent,
            child: Row(
              children: [
                const WalletIcon(WalletGlyph.transparent, size: 20),
                const SizedBox(width: 12),
                // Flexible so a longer label (or a wider locale / larger text
                // scale) wraps within the menu instead of overflowing the row.
                Flexible(child: Text(l10n.walletMoveMenuItem)),
              ],
            ),
          ),
        // Package-custody only (see `packageCustody` above): rescan rebuilds
        // the wallet DB through the package provisioner, which a session-only
        // host doesn't wire — its own custody layer owns recovery there.
        // Disabled while a deep scan's widen is in flight — the RECIPROCAL of the
        // deep-scan entry's `!rescanBusy` guard, so the two transparent-set
        // recovery ops are mutually exclusive from EITHER direction (rel-MED3):
        // neither sheet can be opened while the other's op runs. Hidden for
        // watch-only (the SDK refuses a watch-only rescan typed).
        if (packageCustody && !watchOnly)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.rescan,
            enabled: !deepScanInFlight && !syncNotRunning,
            // The sync-off disable SAYS WHY (the one-time-address
            // entry's own precedent: "the label SAYS WHY… not just a grey
            // row"): the explaining badge is occluded behind the open menu,
            // and a screen reader would otherwise hear only "dimmed".
            child: Row(
              children: [
                const WalletIcon(WalletGlyph.restore, size: 20),
                const SizedBox(width: 12),
                Flexible(
                  child: syncNotRunning
                      ? Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l10n.walletRescanMenuItem),
                            Text(
                              l10n.walletMenuSyncNotRunningHint,
                              style: Theme.of(context).textTheme.bodySmall,
                            ),
                          ],
                        )
                      : Text(l10n.walletRescanMenuItem),
                ),
              ],
            ),
          ),
        // #390 — adjacent to Rescan but NOT package-custody gated: the deep scan
        // runs entirely over the live WalletSession (the one-time-address-check
        // class), so a session-only host gets it too. Disabled while its own scan
        // is in flight ("Checking…") or a rescan is running/rebuilding (they both
        // re-poll the transparent set); the rescan sheet cross-points back here.
        // Swap-related deep scan — hidden for watch-only (swap is off).
        if (!watchOnly)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.checkOlderSwapAddresses,
            enabled: !deepScanInFlight && !rescanBusy && !syncNotRunning,
            child: Row(
              children: [
                const WalletIcon(WalletGlyph.rescan, size: 20),
                const SizedBox(width: 12),
                Flexible(
                  // The in-flight label swap keeps precedence; the sync-off
                  // hint explains the grey row when nothing else in
                  // the menu does — same reason-visible rule as the rescan
                  // entry above.
                  child: syncNotRunning && !deepScanInFlight
                      ? Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l10n.walletDeepScanMenuItem),
                            Text(
                              l10n.walletMenuSyncNotRunningHint,
                              style: Theme.of(context).textTheme.bodySmall,
                            ),
                          ],
                        )
                      : Text(
                          deepScanInFlight
                              ? l10n.walletDeepScanChecking
                              : l10n.walletDeepScanMenuItem,
                        ),
                ),
              ],
            ),
          ),
        // Reclaiming one-time-address funds MINTS a self-send (a spend) —
        // hidden for watch-only.
        if (!watchOnly)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.checkOneTimeAddresses,
            // Disabled while ANY sweep runs (the shared single-flight latch) —
            // and the label SAYS WHY ("Recovering…"), so both sighted users and
            // screen readers get the reason, not just a grey row; this is also
            // the only visible in-progress cue when the balance-card button is
            // absent (a failed recoverable read must not hide a running sweep).
            enabled: !sweepInFlight,
            child: Row(
              children: [
                const WalletIcon(WalletGlyph.sweep, size: 20),
                const SizedBox(width: 12),
                Flexible(
                  child: Text(
                    sweepInFlight
                        ? l10n.walletRecoverInProgress
                        : l10n.walletCheckOneTimeMenuItem,
                  ),
                ),
              ],
            ),
          ),
        // Only when the host wired the appearance seam (wallet_routes.dart) —
        // a host whose shell owns display settings gets no dead entry here.
        if (appearanceRoute != null)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.appearance,
            child: Row(
              children: [
                const WalletIcon(WalletGlyph.appearance, size: 20),
                const SizedBox(width: 12),
                // Flexible like all five siblings (#409 R4): a bare Text in a
                // menu Row claims its full intrinsic width, and German
                // "Erscheinungsbild" overflows the popup AT 1.0x.
                Flexible(child: Text(l10n.walletAppearanceMenuItem)),
              ],
            ),
          ),
        // Package-custody only (see `packageCustody` above): the Security
        // screen's custody probe + delete-wallet crypto-shred both run through
        // the package provisioner. A session-only host owns key custody itself
        // (its recovery/delete surfaces live host-side), so the entry hides.
        if (packageCustody)
          PopupMenuItem<_WalletMenuAction>(
            value: _WalletMenuAction.security,
            child: Row(
              children: [
                const WalletIcon(WalletGlyph.shielded, size: 20),
                const SizedBox(width: 12),
                // Flexible — see the appearance entry above (#409 R4).
                Flexible(child: Text(l10n.walletSecurityMenuItem)),
              ],
            ),
          ),
      ],
    );
  }
}

/// Whether the wallet tab shows its sync bar (FR-49 S12, C2). HIDDEN only
/// when every one of these holds, so nothing a user must know hides with it:
/// - the status is a plain, fresh `UpToDate` at badge level `ok` (every
///   qualified reached-tip variant — limited, degraded, unverified, endpoint
///   behind — is `caution` and keeps the bar, as do a stale snapshot, a failed
///   start and a host-off policy, via [walletBadgeLevel]);
/// - the sync loop is running;
/// - the transport claim cannot go stale while the tab is open (rev.2 R6):
///   `neutral` (Tor off / a direct connection — static for the session), or
///   `protected` ONLY when the HOST declared it ([hostDeclaredTransport]).
///   An SDK-derived `protected` comes from the cold snapshot, which is not
///   re-read on a mid-session Tor fallback, so it keeps the bar (with its
///   filled shield) until the UI can watch Tor state live. `caution`,
///   `danger` and `progress` always keep the bar. Relim accepted hiding on
///   `neutral` over "show whenever not protected", which would never hide
///   for a host without Tor.
///
/// A `Scanning` sample with [followingTip] (the [walletTipFollowProvider]
/// rule: the wallet catching up to `kWalletTipFollowBlocks` new blocks after a
/// healthy `UpToDate`, usually one) counts as that `UpToDate`, judged at the
/// tip it scans to — else
/// the bar would slide in and out on every block (the S12 security review's
/// MEDIUM).
bool walletSyncBarVisible({
  required SyncStatus status,
  required bool stale,
  required bool driving,
  required bool startFailed,
  required bool syncDisabled,
  required TransportTone transportTone,
  required bool hostDeclaredTransport,
  bool followingTip = false,
}) {
  final steadyTransport =
      transportTone == TransportTone.neutral ||
      (transportTone == TransportTone.protected && hostDeclaredTransport);
  final settled = switch (status) {
    SyncStatus_UpToDate() => status,
    SyncStatus_Scanning(:final to) when followingTip => SyncStatus.upToDate(
      tip: to,
    ),
    _ => null,
  };
  final healthy =
      settled != null &&
      driving &&
      walletBadgeLevel(
            status: settled,
            stale: stale,
            startFailed: startFailed,
            syncDisabled: syncDisabled,
          ) ==
          WalletBadgeLevel.ok &&
      steadyTransport;
  return !healthy;
}

/// The active wallet's large-title header (FR-49 S12, C1): *Wallet* in the
/// tab-root title style, the watch-only chip after it, then the hide-balance
/// eye and the overflow menu on the trailing side. It scrolls with the
/// content.
class _WalletHeader extends ConsumerWidget {
  const _WalletHeader({this.syncStatus});

  /// Passed through to the overflow menu while the sync bar is hidden.
  final ({WalletGlyph glyph, String headline})? syncStatus;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final hidden = ref.watch(walletBalanceHiddenProvider);
    final watchOnly = ref.watch(isWatchOnlyProvider);
    // Past the shared large-text threshold the watch-only chip moves to its
    // own full-width line under the title, so the chip — the information
    // carrier — stays whole beside the eye and the menu on a narrow screen
    // (UX-M6; at 3× on 320 dp it no longer fits beside the title).
    final chipBelow = watchOnly && walletTextScaleForcesStack(context);
    // A heading for screen readers, as the AppBar title it replaces was.
    final title = Semantics(
      header: true,
      child: Text(
        l10n.walletTitle,
        style: textTheme.headlineLarge,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
    );
    final row = Row(
      children: [
        Expanded(
          child: Row(
            children: [
              // Flexible + ellipsis: at a large text scale / a long-locale
              // title the row must shrink the TITLE, never overflow — the
              // watch-only chip is the information carrier (UX-M6).
              Flexible(child: title),
              if (watchOnly && !chipBelow) ...[
                const SizedBox(width: 10),
                const _WatchOnlyBadge(),
              ],
            ],
          ),
        ),
        IconButton(
          key: const ValueKey('wallet-hide-balance'),
          // The label names what a press WILL do (FD-6), and so does the
          // glyph: hidden → "show balance", shown → "hide" (S13 §1.7, the
          // reveal glyph keeps its one meaning).
          tooltip: hidden ? l10n.walletShowBalance : l10n.walletHideBalance,
          icon: WalletIcon(
            hidden ? WalletGlyph.showBalance : WalletGlyph.hidden,
          ),
          onPressed: () =>
              ref.read(walletBalanceHiddenProvider.notifier).toggle(),
        ),
        _WalletOverflowMenu(syncStatus: syncStatus),
      ],
    );
    if (!chipBelow) return row;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [row, const SizedBox(height: 4), const _WatchOnlyBadge()],
    );
  }
}

enum _WalletMenuAction {
  syncStatus,
  syncServer,
  transparentFunds,
  moveToTransparent,
  rescan,
  checkOlderSwapAddresses,
  checkOneTimeAddresses,
  appearance,
  security,
}

class _WalletNotSetUp extends StatelessWidget {
  const _WalletNotSetUp();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(WalletGlyph.wallet, size: 48, color: colors.textMuted),
            const SizedBox(height: 16),
            Text(
              l10n.walletNotSetUpTitle,
              style: textTheme.headlineSmall,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 12),
            Text(
              l10n.walletNotSetUpBody,
              style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}

class _WalletActive extends ConsumerWidget {
  const _WalletActive();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final snapshot = ref.watch(walletSnapshotProvider);
    // The live stream overrides the cold status once it emits; before that
    // the snapshot's own status carries the screen (no spinner flash).
    // `select` narrows the watch to the status VALUE: the
    // notifier wraps every emit in a fresh AsyncValue, so without it each
    // loading/error-flag flip and every value-equal replay (re-subscribe
    // after resume, coalesced duplicate samples) rebuilt this whole surface
    // for nothing. SyncStatus has value equality, so the select only fires
    // when the rendered status actually changed.
    //
    // The posture-stable DISPLAY view (#399 item 3), never the raw stream:
    // this value renders the badge (color + headline + the liveRegion a11y
    // announcement) and the disabled-Send reason — under FR-21's sub-second
    // dial verdicts the raw status flaps Stalled↔Scanning on every doomed
    // retry, which flashed the badge and re-announced the pair to screen
    // readers each cycle. Logic listeners (the refresh-edges listen below,
    // the rescan tip-edge, the synced-tip latch) stay on the RAW provider —
    // the dwell is presentation policy, never a truth filter.
    final liveStatus = ref.watch(
      walletDisplaySyncStatusProvider.select((s) => s.value),
    );
    // Watching this BUILDS the sync-drive controller on first Active render,
    // unless a host already built it at its root (walletSyncDriveProvider,
    // FR-51); non-autoDispose, it then persists for the container's life, driving
    // the loop by lifecycle (auto-start on Active, suspend on background, resume
    // on foreground) even after the user leaves this screen — no manual button.
    // The value is read only to surface a rare start failure honestly below.
    final syncDrive = ref.watch(walletSyncControllerProvider);
    // WILL a sync pass run? (#405 — the SSOT.) The badge keeps reading the
    // drive directly: it is the one surface whose whole job is naming the
    // CAUSE ("Sync off" vs "couldn't start"). Everything else on this screen
    // that suppresses an active-progress claim or gates a
    // completes-only-via-sync op asks this instead, so a failed start and a
    // host-off policy — indistinguishable in their consequence — stop being
    // told apart by surfaces that then word themselves for only one of them.
    final syncPassesRun = ref.watch(walletSyncPassesRunProvider);
    // Builds the reconnect kick on first Active render (#404) — the same
    // lifetime argument as the controller above: it must stay subscribed while
    // the WALLET is alive, not only while this screen is on top. It turns a
    // network-available tick into the ladder reset the sync sheet's "Try now"
    // does by hand, for the FOREGROUND reconnect case where no lifecycle event
    // fires and the backoff would otherwise run to 256s+ after the network is
    // demonstrably back. Its value is never rendered.
    //
    // `.notifier`, NOT the value (#407 R10c): watching the value rebuilds ALL of
    // `_WalletActive` on every kick, because that value is an int that
    // increments — a whole-surface rebuild per reconnect for a number nothing
    // displays. Watching the notifier still BUILDS the provider (which is the
    // only thing wanted here — the subscription's lifetime) without subscribing
    // to its state.
    ref.watch(walletReconnectKickProvider.notifier);
    // Builds the auto-shield policy loop on first Active render (§3.2i-3 (a)).
    // Non-autoDispose, so — like the sync controller above — it then persists
    // for the container's life and keeps evaluating on every refreshed
    // snapshot even while the user is on another screen. Its status is
    // rendered by the balance card's honesty cue, not here.
    ref.watch(walletAutoShieldControllerProvider);
    // Anchors the two receive-address views for the container's life (#386,
    // the E2E-2 fold — the auto-shield precedent above): unanchored,
    // leaving the Receive screen PAUSED the view, which paused its listen on
    // the identity-keyed reader, so the first derive's honest conclusion
    // (value or timeout) was deferred and the deferred rebuild made every
    // re-entry appear to restart a 25–45 s derive from zero. Anchored, each
    // identity's derive runs to its conclusion exactly ONCE regardless of
    // navigation — and starts at first Active render, so the address is
    // usually ready BEFORE Receive is first opened. The #381 identity fence
    // is preserved: this watches the stable VIEWS, so an identity flip
    // re-keys them and the superseded reader element disposes (the values
    // are unused here — the Receive screen renders them). Known cost (
    // review): the address elements now exist for EVERY wallet life, so the
    // documented heap-lingering caveat (a flip with no money surface
    // mounted parks the OLD identity's last values until the next mount
    // re-keys) applies to addresses too — never renderable, memory-hygiene
    // only, same class as the snapshot view.
    ref.watch(walletReceiveAddressProvider);
    ref.watch(walletTransparentAddressProvider);

    // Balance freshness while the user WATCHES sync (ADR-0533, "see balance
    // asap"): the cold snapshot is otherwise re-read only on resume, so a
    // foreground sync would leave the balance frozen at its first value even as
    // scanning finds notes. Refresh it on the two edges that change the
    // user-visible balance — funds first becoming spendable (spend-before-sync)
    // and reaching the tip. Edge-triggered (not per-batch), so it never
    // reintroduces the ADR-0532 O(n^2) per-batch summary cost; scoped to this
    // screen, so the carefully-counted resume path in SyncStatusNotifier (the
    // lifecycle tests' "no wasteful re-fetch" invariant) is left untouched.
    ref.listen<AsyncValue<SyncStatus>>(syncStatusProvider, (prev, next) {
      // Only TRANSITIONS matter. On the very first emission `prev` is null, and
      // the mount already kicked off a fresh `walletSnapshotProvider` read — so
      // skip it (else an already-synced wallet would needlessly cancel + redo
      // that in-flight cold read on open; review HARDENING).
      if (prev == null) return;
      final p = prev.value;
      final n = next.value;
      if (n == null) return;
      // Not while FOLLOWING the tip (a routine one-block pass on a synced
      // wallet): that pass's re-read would land mid-pass and flip the
      // headline to the total until the reached-tip re-read seconds later
      // (S12 rev.3 diff review). The edge exists for a catch-up's
      // spend-before-sync, which is never a follow.
      final becameSpendable =
          n is SyncStatus_Scanning &&
          n.spendableReady &&
          !(p is SyncStatus_Scanning && p.spendableReady) &&
          !ref.read(walletTipFollowProvider).following;
      // §4r U-4 (§4m #18): the edge into ANY reached-tip variant from a
      // different state — not into `UpToDate` alone. A pass that completed
      // QUALIFIED (this build's reach, this server's pools, its height, its
      // network claim) changes the user-visible balance exactly as a plain
      // completion does, and `Scanning → UpToDateDegraded` used to leave the
      // balance frozen at its first value. A same-variant repeat is not an
      // edge (replay and coalescing would otherwise re-read on every pass); a
      // change BETWEEN two reached-tip variants is one (a pool newly served,
      // a server switch) — the snapshot may differ.
      final reached = _reachedTipVariant(n);
      final reachedTip = reached != null && reached != _reachedTipVariant(p);
      if (becameSpendable || reachedTip) {
        ref.invalidate(walletSnapshotReadProvider);
        // Re-pull the recoverable one-time-address subset on the SAME edges (a
        // newly-buried strand/return flips `isFinal` or appears) so the balance
        // note tracks the snapshot it annotates and never goes stale (2e-2b-iv).
        ref.invalidate(walletRecoverableEphemeralFundsReadProvider);
        // Re-pull the parked "saved & pending" surface on the SAME edges so a
        // newly queued/drained TEX send appears/clears without a manual pull
        // (2e-2b-v-4b — the parked surface tracks the same balance edges).
        ref.invalidate(walletParkedSendsReadProvider);
        // Re-pull the in-flight two-step cue on the SAME edges (#309): a drained
        // TEX moves parked → in-flight, and a completed/stranded chain clears it.
        ref.invalidate(walletInFlightSendsReadProvider);
        // Re-pull the in-flight SWAPS home on the SAME edges (W-swap-5, #366): a
        // deposit draining/mining changes what "tap to check" will show, and a
        // record dismissed by another surface's terminal observation clears here.
        ref.invalidate(walletInFlightSwapsReadProvider);
        // Re-pull the deep-scan coverage on the SAME edges (C1 / review H2): a
        // widen's backfill marker advances as the wallet syncs, so `pending`
        // drops on these edges — the "still checking older swap addresses"
        // banner tracks it and self-clears when it reaches 0. Harmless when no
        // scan ran (the autoDispose read is unwatched then).
        ref.invalidate(swapAddressCoverageReadProvider);
        // Refresh the activity list on the SAME edges, so a newly arrived or
        // newly confirmed tx appears without a manual pull (FR-1). `refresh()`
        // (not invalidate) so the current rows stay visible until the fresh page
        // lands — a transient busy-DB fault never blanks a populated history.
        ref.read(walletActivityProvider.notifier).refresh();
      }
    });

    // The coin's first spin event (FR-49 S12, C3 rev.2 R2): the status moving
    // INTO a HEALTHY `UpToDate` from a state this mount OBSERVED that was not
    // one — the moment the sync bar slides away. Read through the tip follow
    // (the DISPLAYED status the bar renders), so a routine one-block pass is not an
    // entry: the bar never showed for it, and a spin per block would read as
    // money arriving. A first sample (the open, a resume into an
    // already-synced wallet) does not spin. Only plain `UpToDate` with the
    // loop running and a fresh snapshot — the badge's `ok` — earns it: a
    // limited, degraded, unverified or behind-server "reached tip" is a
    // caution state, and a celebration over it would contradict the bar.
    ref.listen<WalletTipFollow>(walletTipFollowProvider, (prev, next) {
      final p = prev?.status;
      if (p == null) return;
      if (next.status is SyncStatus_UpToDate &&
          p is! SyncStatus_UpToDate &&
          !prev!.following &&
          ref.read(walletSyncControllerProvider) == WalletSyncDrive.running &&
          !ref.read(walletSnapshotProvider).hasError) {
        ref.read(walletCoinSpinsProvider.notifier).bump();
      }
    });

    // The incoming-funds ARRIVAL edge (ADR-0536 #392): a `live` detection, a
    // catch-up `replay` that found arrivals, or a `memoRefresh` (attribution
    // data improved for already-listed rows) refreshes the activity list AT
    // ONCE — a received payment appears while the pass is still scanning,
    // instead of waiting for the coarse reachedTip edge above. ONLY the
    // activity refresh rides this edge: it is one bounded, generation-
    // serialized keyset page read, so a restore's per-batch replays stay
    // cheap — the O(scanned-set) snapshot summary stays on the two coarse
    // edges above (the ADR-0532 discipline). The count-0 `replay` baseline
    // and coalesced repeats are filtered by the value-equality of the edge.
    ref.listen<AsyncValue<IncomingFundsEvent>>(incomingFundsEventsProvider, (
      prev,
      next,
    ) {
      final event = next.value;
      if (event == null || event == prev?.value) return;
      // A REAL arrival (count > 0) bypasses the deep-page guard: a new payment
      // is worth collapsing pagination — the alternative is a deep-paged list
      // that never shows it at all (U1). The advisory edges (conservative
      // fire, memoRefresh, unknown) keep the guarded refresh: they are
      // "pull to confirm" nudges, not confirmed arrivals.
      if (event.newTxCount > 0) {
        ref.read(walletActivityProvider.notifier).collapseToFirstPage();
        // The MINIMAL arrival cue: a transient
        // SnackBar — visible even when the list sits below the fold, and
        // announced by screen readers (closing the a11y gap: arrivals were
        // the one money edge with no cue, against the liveRegion convention).
        // Amount-free BY DESIGN (the event carries none — ADR-0536); Live
        // arrivals only, never the replay catch-up (old news) or advisory
        // nudges. The count is the detect diff's exact count at emit time
        // (advisory only under coalescing — acceptable for a cue whose call
        // to action is "look at the list").
        if (event.kind == IncomingFundsEventKind.live) {
          // The coin's second spin event (C3 rev.2 R3): a live arrival of
          // funds NEWER than the last synced tip. The SDK delivers a restore's,
          // rescan's or first sync's replayed history as `live` events too
          // (its IncomingFundsEvent doc: a host MUST gate on its own
          // watermark), and old money must not be celebrated as arriving.
          final latch = ref.read(walletSyncedTipProvider);
          final span = event.spanToHeight;
          if (ref.read(walletCatchUpCueProvider) is WalletCatchUpNone &&
              latch is WalletSyncedTipLatched &&
              span != null &&
              span > latch.tip) {
            ref.read(walletCoinSpinsProvider.notifier).bumpArrival(span);
          }
          // S13 §1.6 / M3: once per arrival — a re-delivered event (a resume)
          // does not announce the same money twice. Its own watermark, not
          // the coin's (the coin's gate hides catch-up arrivals).
          if (!ref.read(walletArrivalCueProvider.notifier).admit(span)) {
            return;
          }
          // The arrival is felt as well as seen (S13 §1.7), once per
          // admitted arrival — the same gate as the snackbar.
          unawaited(HapticFeedback.lightImpact());
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                WalletLocalizations.of(
                  context,
                ).walletPaymentReceived(event.newTxCount),
              ),
            ),
          );
        }
        return;
      }
      final advisory =
          // The conservative-fire degradation (a detect read fault): span set,
          // count 0 — "pull to confirm", so pull.
          (event.kind == IncomingFundsEventKind.live &&
              event.spanToHeight != null) ||
          event.kind == IncomingFundsEventKind.memoRefresh ||
          // Forward-compat: the DTO contract says treat an unknown kind as a
          // generic "pull now" nudge — the reference host models its own rule.
          event.kind == IncomingFundsEventKind.unknown;
      if (advisory) {
        ref.read(walletActivityProvider.notifier).refresh();
      }
    });

    // The rescan-recovery presentation (FR-1b): a "rebuilding your history" cue
    // (so the user understands WHY the balance/activity briefly emptied — funds
    // are safe) or an honest "couldn't rescan, funds safe" notice (#379). Read
    // BEFORE the cold-loading guard below so the reassurance is shown even during
    // the post-rescan snapshot re-read (the session swap re-reads the now-empty
    // DB — without this the banner would be invisible behind a bare spinner and
    // the user would watch their balance vanish unexplained; review P1 #4).
    // Survives the session swap (the controller does not key off walletSession).
    final rescan = ref.watch(walletRescanControllerProvider);
    // The DURABLE catch-up truth (#380): re-derived from lastSynced +
    // below-tip, so the explanation survives a process death mid-catch-up
    // (app update / LMK kill) and a failure-notice dismiss over a rebuilt
    // wallet — the states where the controller alone has forgotten and the
    // user would watch a 0-balance, "No activity yet" wallet unexplained.
    final catchUp = ref.watch(walletCatchUpCueProvider);

    // The deep-scan progress cue (C1 / review H2): once the user has run a
    // widen THIS session, keep a dismissible "still checking older swap
    // addresses" banner up while the backfill is still surfacing (pending > 0).
    // Coverage is pulled ONLY when a scan ran (`ran`) — never for an ordinary
    // wallet, so this never duplicates the #380 first-sync catch-up cue.
    final deepScanProgress = ref.watch(swapDeepScanProgressProvider);
    final deepScanPending = deepScanProgress.ran
        ? (ref.watch(swapAddressCoverageReadProvider).value?.pending ?? 0)
        : 0;
    // …and not under sync-off: the widened ranges are scanned by the
    // normal sync loop, so "still checking older swap addresses" would read
    // "working" forever while nothing runs. The cue state survives (providers
    // are untouched) — the banner returns the moment the policy flips back on.
    final showDeepScanBanner =
        deepScanProgress.ran &&
        !deepScanProgress.dismissed &&
        deepScanPending > 0 &&
        syncDrive != WalletSyncDrive.disabledByHost;

    // #390 C2 / review H1: the one-time POST-RESTORE note — a proactive nudge
    // for a RESTORED wallet whose oldest swaps may sit past the restore sweep's
    // ceiling. Gated to AFTER the first catch-up (WalletCatchUpNone) so it never
    // appears beside a still-filling balance; `pending` only for a restored,
    // un-acknowledged wallet (a created wallet / session-only host reads
    // notApplicable → nothing).
    final showRestoreNote =
        catchUp is WalletCatchUpNone &&
        ref.watch(deepScanRestoreNoteProvider).value ==
            DeepScanRestoreNoteState.pending;

    // Keep the LAST-KNOWN state across a transient reload failure (a busy-DB
    // snapshot throw on resume must NOT blank a live wallet — `.value`
    // retains the prior data through an error). Only a cold first-load with
    // no prior value falls through to loading / the honest error card.
    final state = snapshot.value;
    if (state == null) {
      if (snapshot.isLoading) {
        // During a post-rescan rebuild the empty-DB snapshot is re-reading; show
        // the reassuring banner above the spinner, not a bare (alarming) spinner.
        // (The controller fast path only: the durable arm needs the snapshot,
        // which is exactly what has not loaded yet on this branch.)
        // …not under sync-off (the R1 gate the Rebuilding arms
        // missed): "Rebuilding your history" claims active recovery, but the
        // rebuild only fills via the sync loop the host turned off — the
        // banner would stand forever over a wiped balance while the badge
        // says "Sync off". The badge is the one honest surface then.
        // Every early state keeps the header (S12 diff review MEDIUM): the
        // title and, above all, the overflow menu — Rescan, Security, the
        // sync sheet — are the recovery routes a wallet that cannot load
        // needs most. The bar's state is unknown here, so the menu carries
        // no sync-status entry.
        if (catchUp is WalletCatchUpRebuilding &&
            syncDrive != WalletSyncDrive.disabledByHost) {
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
            children: [
              const _WalletHeader(),
              const SizedBox(height: 12),
              _RescanRebuildingBanner(
                target: catchUp.target,
                resumedRescan: catchUp.target == null,
              ),
              const SizedBox(height: 24),
              const Center(
                child: WalletLoadingIndicator(key: Key('wallet-loading')),
              ),
            ],
          );
        }
        return const _EarlyState(
          child: WalletLoadingIndicator(key: Key('wallet-loading')),
        );
      }
      return const _EarlyState(child: _SnapshotError());
    }

    // We're rendering the last-known state THROUGH a refetch error (a
    // persistent busy/locked DB). Keeping the wallet usable is right, but
    // staying silent about a possibly-stale balance is not (no silent
    // failures, invariant 10) — show an honest "couldn't refresh" notice.
    // (The header's as-of stamp is NOT this notice's substitute: it dates the
    // last good SYNC, while this dates a failed snapshot RE-READ.)
    final stale = snapshot.hasError;

    // The live status drives both the status badge and the Send gate; the cold
    // snapshot's own status carries the very first frame before the stream emits.
    final status = liveStatus ?? state.syncStatus;

    // The recoverable one-time-address subset (2e-2b-iv) — a SUBSET of the
    // transparent balance shown as a locational note, NEVER added on top. Purely
    // additive INFORMATION, so it never blocks the balance: take the last-known
    // value (`.value`, default empty) and reduce. LIVE since gate-removal
    // (2e-2b-v-5a); empty only on a wallet that has stranded nothing.
    final recoverable = summarizeRecoverable(
      ref.watch(walletRecoverableEphemeralFundsProvider).value ??
          const <RecoverableEphemeralFunds>[],
    );

    // The sync bar HIDES when the wallet is synced and healthy (FR-49 S12,
    // C2); its sheet then stays one tap away in the overflow menu.
    final l10n = WalletLocalizations.of(context);
    final driving = syncDrive == WalletSyncDrive.running;
    final startFailed = syncDrive == WalletSyncDrive.failed;
    final syncDisabled = syncDrive == WalletSyncDrive.disabledByHost;
    final hostTransport = ref.watch(walletHostTransportProvider);
    final transport = transportPresentation(
      l10n,
      tor: state.tor,
      host: hostTransport,
    );
    final followingTip = ref.watch(
      walletTipFollowProvider.select((f) => f.following),
    );
    final showSyncBar = walletSyncBarVisible(
      status: status,
      stale: stale,
      driving: driving,
      startFailed: startFailed,
      syncDisabled: syncDisabled,
      transportTone: transport.tone,
      hostDeclaredTransport: hostTransport != null,
      followingTip: followingTip,
    );
    final presentation = syncStatusPresentation(
      l10n,
      status,
      driving: driving,
      startFailed: startFailed,
      syncDisabled: syncDisabled,
    );
    final reduceMotion = MediaQuery.maybeDisableAnimationsOf(context) ?? false;

    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
      children: [
        _WalletHeader(
          syncStatus: showSyncBar
              ? null
              : (glyph: presentation.icon, headline: presentation.headline),
        ),
        const SizedBox(height: 12),
        // …not while no pass will run (#405 widened it from the policy
        // to the SSOT): same gate as the Syncing arm below — the rebuild only
        // progresses via a sync pass, so the banner's active-recovery claim
        // would never resolve. A FAILED start suppresses it for exactly the
        // same reason a host-off policy does.
        if (catchUp is WalletCatchUpRebuilding && syncPassesRun) ...[
          _RescanRebuildingBanner(
            target: catchUp.target,
            resumedRescan: catchUp.target == null,
          ),
          const SizedBox(height: 12),
        ],
        // The durable arm (#380): a never-synced wallet below the tip with no
        // intra-session rescan state — post-relaunch, post-dismiss, or a
        // restore/create's first catch-up. Gated to controller-Idle so it
        // never STACKS on a failure notice (pre-dismiss, the notice is the
        // explaining surface; on dismiss this takes over seamlessly).
        // …and never while no pass will run (#383 R1; #405 → the SSOT): the
        // banner claims active catching-up, but nothing is running — the badge
        // (or the start-failed notice) is the one honest surface then.
        if (catchUp is WalletCatchUpSyncing &&
            rescan is WalletRescanIdle &&
            syncPassesRun) ...[
          const _RescanRebuildingBanner(target: null),
          const SizedBox(height: 12),
        ],
        if (rescan is WalletRescanFailed) ...[
          const _RescanFailedNotice(),
          const SizedBox(height: 12),
        ],
        if (rescan is WalletRescanFailedNeedsSpace) ...[
          const _RescanNeedsSpaceNotice(),
          const SizedBox(height: 12),
        ],
        if (rescan is WalletRescanBlockedBySettlingSend) ...[
          const _RescanBlockedSettlingNotice(),
          const SizedBox(height: 12),
        ],
        // #405 — the commit-point fence refused the wipe because no sync pass
        // will run. Its OWN notice, not the shared failed one: nothing
        // destructive happened, so this is the one refusal arm entitled to say
        // the wallet is unchanged outright (#379 softened the others because
        // a post-rename fault leaves a REBUILT wallet behind).
        if (rescan is WalletRescanBlockedBySyncNotRunning) ...[
          const _RescanBlockedSyncNotRunningNotice(),
          const SizedBox(height: 12),
        ],
        // #390 C2: the one-time post-restore nudge (only after catch-up).
        if (showRestoreNote) ...[
          const _DeepScanRestoreNote(),
          const SizedBox(height: 12),
        ],
        // #390 C1: the "still checking older swap addresses" progress cue —
        // survives closing the sheet, self-clears when the widen finishes
        // surfacing (pending → 0), user-dismissible.
        if (showDeepScanBanner) ...[
          const _DeepScanCheckingBanner(),
          const SizedBox(height: 12),
        ],
        if (stale) ...[const _StaleNotice(), const SizedBox(height: 12)],
        // The bar collapses and returns in 250 ms so its space never jumps
        // under a reading eye; under reduced motion it changes at once, WITHOUT
        // an AnimatedSize. A zero-duration one, in this screen, raised "A
        // RenderAnimatedSize was mutated in its own performLayout" when the bar
        // hid (S12 test run; a bare two-pump probe did not reproduce it, so the
        // trigger is the in-list relayout). A reduce-motion toggle mid-life
        // remounts the bar's subtree, which holds no local state.
        _SyncBarSlot(
          animate: !reduceMotion,
          child: showSyncBar
              ? Padding(
                  padding: const EdgeInsets.only(bottom: 16),
                  child: _SyncBadge(
                    status: status,
                    stale: stale,
                    // running ⇒ the loop is up; an Idle status then means
                    // "connecting", not "stopped". A failed/suspended drive is
                    // genuinely not running.
                    driving: driving,
                    // failed ⇒ the Idle arm's copy must say the start failed,
                    // never "starts automatically" (#356-F8) — the notice
                    // below carries retry.
                    startFailed: startFailed,
                    // #383 R1: the host's sync policy is off — the badge owns
                    // the honest "Sync off" story over any retained status.
                    syncDisabled: syncDisabled,
                    // The transport indicator rides the SAME fixed row
                    // the sheet one tap away carries its full
                    // story.
                    tor: state.tor,
                  ),
                )
              : const SizedBox(width: double.infinity),
        ),
        _BalanceCard(
          balance: state.balance,
          recoverable: recoverable,
          // S12 rev.3 N1/N2: only a synced wallet splits "arriving" out of
          // the headline — while catching up, the core's pending-incoming
          // also holds money the user already had. Decided from the
          // SNAPSHOT'S OWN provenance (the rev.3 diff review), never the live
          // status: the figures are those of the moment the snapshot was
          // read, and a read taken mid-pass (a new tip recorded, its blocks
          // not yet scanned) holds recent money in pending-incoming. Only a
          // fresh read taken at a completed pass, at that pass's own tip,
          // splits; anything else shows the total, which never mislabels.
          synced:
              !stale &&
              switch (state.syncStatus) {
                SyncStatus_UpToDate(:final tip) => tip == state.tip,
                _ => false,
              },
          // "(as of block N)" folds INTO the Balance header —
          // the old standalone _BalanceAge row floated below the buttons.
          // The session latch keeps the height through re-scan windows.
          asOf: balanceAsOf(
            status: status,
            latch: ref.watch(walletSyncedTipProvider),
            lastSynced: state.lastSynced,
          ),
        ),
        // The "recover now" affordance for one-time-address funds (2e-2b-v-4b) —
        // self-hides unless a SUCCESSFUL recoverable read shows funds.
        const RecoverEphemeralAction(),
        const SizedBox(height: 12),
        _WalletActions(
          status: status,
          spendableZat: state.balance.spendableZat,
          // #405: the SSOT, not the policy-shaped drive read. Under a FAILED
          // start the old read fell through to the status arms and told the
          // user their balance was "catching up" — an active-progress claim
          // over a loop that never started.
          syncNotRunning: !syncPassesRun,
        ),
        // Sync runs on its own (auto-start + lifecycle) — no manual control.
        // Only the rare command-failure surfaces, with a contextual retry.
        if (syncDrive == WalletSyncDrive.failed) ...[
          const SizedBox(height: 24),
          const _SyncStartFailedNotice(),
        ],
        // ZODL-style activity history in the lower portion of the surface
        // (maintainer: "history activity in the down side"). FR-1: the paginated
        // transaction list, newest first.
        const SizedBox(height: 24),
        // The durable SWAP HOME (W-swap-5, #366) — every in-flight swap, listed
        // from the encrypted store so it survives process death; "View swap"
        // re-attaches live tracking. Self-hides when empty; honest error line
        // on a failed read (the only wallet-side witness of a mid-flight swap).
        const InFlightSwapsSection(),
        // The durable in-flight two-step cue (#309) — "on its way through a
        // one-time address; don't send it again", surviving the dismissible
        // result screen across the forwarding window. Self-hides when empty.
        const InFlightSendsSection(),
        // The parked "saved & pending" surface (2e-2b-v-3/v-4b, widened #331) —
        // EVERY queued send with no on-chain tx yet (one-time-address two-steps
        // AND plainly-queued offline sends), INVISIBLE in the activity list.
        // Self-hides when empty; each row carries its own cancel affordance.
        // Sits ABOVE the activity list.
        const ParkedSendsSection(),
        const _ActivityHistory(),
      ],
    );
  }
}

/// The transaction-history section (FR-1; maintainer's ZODL "history in the down
/// side"). Watches [walletTransactionsProvider] and renders the newest-first list:
/// an honest empty state, a bounded loading line, an honest error line, or the rows.
/// Last-known rows are kept THROUGH a transient refetch error (a busy-DB throw must
/// not blank a populated history); only a cold first-load falls through to loading /
/// error. Each row is display-only (§5.4: txids/amounts are shown, never logged).
class _ActivityHistory extends ConsumerWidget {
  const _ActivityHistory();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    final activity = ref.watch(walletActivityProvider);
    final txs = activity.rows;
    final isEmpty = txs.isEmpty;
    // While the history repopulates, an empty/loading list is EXPECTED — show
    // the distinct catch-up cue instead of "No activity yet" / a bare spinner,
    // so the user never reads a temporarily-empty history as lost (review P1
    // #4). Derived from the DURABLE cue (#380), so it holds through a process
    // death mid-catch-up and stays up even while a failure notice renders
    // above (the pre-dismiss "No activity yet" contradiction). The full
    // reassurance (incl. the 0 balance) is the top banner; this is the short
    // in-section cue. Once rows repopulate they render normally.
    final catchUp = ref.watch(walletCatchUpCueProvider);
    // (the R1 gate, applied here like the top banners): both catch-up
    // notes claim ACTIVE progress ("Rebuilding…", "Still catching up… will
    // show up here"), which with no pass coming never happens — and with the
    // top banner suppressed this note would be the only, and false, progress
    // claim on the surface. Fall through to the honest sync-not-running note
    // instead; the badge (or the start-failed notice) is the explaining
    // surface. #405: the SSOT, so a FAILED start is covered too — under the
    // old policy-shaped read that wallet showed "Still catching up".
    final syncNotRunning = !ref.watch(walletSyncPassesRunProvider);

    final Widget body;
    if (isEmpty && !syncNotRunning && catchUp is WalletCatchUpRebuilding) {
      body = _activityNote(l10n.walletActivityRebuilding, textTheme, colors);
    } else if (isEmpty && !syncNotRunning && catchUp is WalletCatchUpSyncing) {
      // The durable arm cannot name the recovery (the choice did not survive
      // the relaunch — or this is a restore/create's first sync): generic copy.
      body = _activityNote(l10n.walletActivityCatchingUp, textTheme, colors);
    } else if (isEmpty && syncNotRunning && catchUp is! WalletCatchUpNone) {
      // (UX HIGH): a wiped/never-synced history with no pass coming
      // must not read "No activity yet" — a flat lie over history that exists
      // but cannot repopulate until sync runs. This arm is the honest
      // replacement for the two suppressed catch-up notes above: it states the
      // pending fill and its condition, and leaves the CAUSE to the badge
      // (#405 — naming a remedy here would name the wrong one half the time).
      body = _activityNote(
        l10n.walletActivitySyncNotRunning,
        textTheme,
        colors,
      );
    } else if (isEmpty && activity.phase == WalletActivityPhase.loading) {
      // Cold first load with no rows yet: a bounded spinner (the controller caps
      // the FFI read with a timeout → the error phase below, never an endless spin).
      body = const Padding(
        padding: EdgeInsets.symmetric(vertical: 24),
        child: Center(
          child: WalletLoadingIndicator(key: Key('wallet-activity-loading')),
        ),
      );
    } else if (isEmpty && activity.phase == WalletActivityPhase.error) {
      body = _activityNote(l10n.walletActivityError, textTheme, colors);
    } else if (isEmpty) {
      body = _activityNote(l10n.walletActivityEmpty, textTheme, colors);
    } else {
      final hidden = ref.watch(walletBalanceHiddenProvider);
      body = Column(
        children: [
          // ONE grouped container (FR-49 S12, C5): `bgCard` at the `group`
          // radius, rows split by 1 dp hairlines inset to the text start.
          ClipRRect(
            key: const ValueKey('wallet-activity-group'),
            borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
            child: ColoredBox(
              color: colors.bgCard,
              child: Column(
                children: [
                  for (var i = 0; i < txs.length; i++) ...[
                    if (i > 0)
                      Divider(
                        height: 1,
                        thickness: 1,
                        indent: _ActivityRow.textInset,
                        color: colors.border,
                      ),
                    _ActivityRow(
                      key: ValueKey(txs[i].txidHex),
                      tx: txs[i],
                      hidden: hidden,
                    ),
                  ],
                ],
              ),
            ),
          ),
          // Keyset pagination (FR-1): a "load more" affordance when more rows
          // remain, so a history past the first page is reachable (no silently-
          // unreachable 51st tx). A trailing spinner while the next page loads.
          if (activity.phase == WalletActivityPhase.loadingMore)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 16),
              child: Center(
                child: SizedBox(
                  height: 20,
                  width: 20,
                  child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                ),
              ),
            )
          else if (activity.canLoadMore)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: TextButton(
                onPressed: () =>
                    ref.read(walletActivityProvider.notifier).loadMore(),
                child: Text(l10n.walletActivityLoadMore),
              ),
            ),
        ],
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsetsDirectional.only(start: 4),
          child: Text(l10n.walletActivityTitle, style: textTheme.titleLarge),
        ),
        const SizedBox(height: 12),
        body,
      ],
    );
  }

  static Widget _activityNote(
    String text,
    TextTheme textTheme,
    WalletColors colors,
  ) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 16),
    child: Text(
      text,
      style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
    ),
  );
}

/// One transaction-history row (FR-1): a direction glyph, Received/Sent + status +
/// time, and the SIGNED amount (incoming tinted, with a leading `+`; outgoing keeps
/// its `-`). A memo glyph rides a row that carries one.
///
/// (maintainer): TAPPABLE — each row opens the transaction-detail sheet
/// (#320). A cancelled row (Expired/Failed — the tx moved NO funds) strikes
/// through and mutes its amount, the "didn't happen" visual every ledger uses,
/// and appends the funds-kept reassurance to its a11y label (a strikethrough
/// is invisible to screen readers); the sheet carries the full explanation.
class _ActivityRow extends StatelessWidget {
  const _ActivityRow({required this.tx, this.hidden = false, super.key});

  final TxSummary tx;

  /// Hide balance (FR-49 W-7): the amount reads as dots, on screen AND in
  /// the row's screen-reader label.
  final bool hidden;

  /// Where a row's text starts: the 16 padding, the 40 disc, the 12 gap —
  /// the group's hairlines are inset to it.
  static const double textInset = 68;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    // STRICTLY positive is incoming; a zero-net tx (self-transfer / fee-only) is
    // NOT "Received" — it falls into the neutral outgoing arm (no `+`, neutral color).
    final incoming = tx.netAmountZat > 0;
    final title = incoming
        ? l10n.walletActivityReceived
        : l10n.walletActivitySent;
    // Signed, in the SDK's own number format and unit (FR-49 S12, C5): a `+`
    // for incoming, and for outgoing the true minus sign U+2212 in place of
    // the ASCII hyphen `formatZec` emits (a list of signed figures reads
    // unevenly with a hyphen). The unit is localized via `walletAmount`.
    final figure = formatZec(tx.netAmountZat);
    final signed = incoming
        ? '+$figure'
        : figure.startsWith('-')
        ? '−${figure.substring(1)}'
        : figure;
    final realAmount = l10n.walletAmount(signed);
    final amountText = displayedAmount(realAmount, hidden: hidden);
    // What a screen reader hears for the amount: never the dots, never the
    // digits while hidden.
    final spokenAmount = hidden ? l10n.walletBalanceHiddenAmount : realAmount;
    final cancelled = txIsCancelled(tx);
    final amountColor = cancelled
        ? colors.textMuted
        : (incoming ? colors.accent : colors.text);
    // Refined by the wallet's delivery reading (stage S8 `obligation`): an
    // unmined send the wallet still owes reads "Retrying", never "Pending".
    final status = txRowStatusLabel(l10n, tx);
    final subtitle = tx.timestamp != null
        ? '$status · ${_formatTime(tx.timestamp!, l10n.localeName)}'
        : status;
    final stackAmount = walletTextScaleForcesStack(context);
    // A money figure is NEVER clipped (the card's and LabeledZatRow's rule):
    // past the space it is given it scales down with every digit intact.
    Widget amountFigure(AlignmentGeometry alignment) => FittedBox(
      fit: BoxFit.scaleDown,
      alignment: alignment,
      child: Text(
        amountText,
        textAlign: TextAlign.end,
        maxLines: 1,
        // The list amount is an identifier-like figure, set in the host's mono
        // face (C5).
        style: WalletTypography.of(context).monoOn(
          (textTheme.bodyMedium ?? const TextStyle()).copyWith(
            color: amountColor,
            fontWeight: FontWeight.w500,
            // Struck through = "didn't happen": an expired/failed tx moved no
            // funds (the a11y label says it in words).
            decoration: cancelled ? TextDecoration.lineThrough : null,
          ),
        ),
      ),
    );

    return Semantics(
      container: true,
      excludeSemantics: true,
      button: true,
      // excludeSemantics drops the InkWell's action — the a11y tap action must
      // live on this node (same rule as the sync badge above).
      onTap: () => showTxDetailSheet(context, tx),
      onTapHint: l10n.walletActivityRowHint,
      // The transparency badge rides the label in words (the icon below is
      // excluded with the rest of the subtree) — a public payment must be as
      // distinguishable to a screen reader as to the eye (§3.2i-3 (c)).
      label: [
        cancelled
            ? '$title $spokenAmount, $subtitle. ${l10n.walletTxFundsKept}'
            : '$title $spokenAmount, $subtitle',
        if (tx.hasTransparentOutput) l10n.walletActivityPublicBadge,
      ].join(', '),
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          onTap: () => showTxDetailSheet(context, tx),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 64),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              // The amount's cap is measured against THIS row's width, not the
              // screen's: a host may mount the wallet in a pane narrower than
              // the window (fold review MEDIUM; LabeledZatRow's rule).
              child: LayoutBuilder(
                builder: (context, rowBox) => Row(
                  children: [
                    // A 40 recessed disc holding the direction (C5).
                    Container(
                      width: 40,
                      height: 40,
                      alignment: Alignment.center,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: colors.bgHover,
                      ),
                      child: WalletIcon(
                        incoming ? WalletGlyph.incoming : WalletGlyph.outgoing,
                        size: 20,
                        color: incoming ? colors.accent : colors.text,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Flexible(
                                child: Text(
                                  title,
                                  style: textTheme.titleSmall?.copyWith(
                                    fontSize: 15,
                                    fontWeight: FontWeight.w500,
                                  ),
                                ),
                              ),
                              if (tx.hasMemo) ...[
                                const SizedBox(width: 6),
                                WalletIcon(
                                  WalletGlyph.memo,
                                  size: 14,
                                  color: colors.textMuted,
                                ),
                              ],
                              // §3.2i-3 (c): a non-change transparent output —
                              // this payment (or arrival) is publicly visible
                              // on-chain, so history marks it apart from shielded
                              // rows (the hasMemo-glyph pattern; the a11y words
                              // ride the row label above).
                              if (tx.hasTransparentOutput) ...[
                                const SizedBox(width: 6),
                                WalletIcon(
                                  WalletGlyph.transparent,
                                  size: 14,
                                  color: colors.orange,
                                ),
                              ],
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(
                            subtitle,
                            // One line always (row heights stay uniform); the sheet
                            // holds the full status + date, so nothing is lost.
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: textTheme.bodySmall?.copyWith(
                              color: colors.textMuted,
                            ),
                          ),
                          // At a large text scale the amount takes its own line
                          // at the user's size: squeezed into 40 % of the row
                          // it would scale down past what they chose, the
                          // setting LabeledZatRow's stack exists to honour (S12
                          // security review LOW).
                          if (stackAmount) ...[
                            const SizedBox(height: 4),
                            amountFigure(AlignmentDirectional.centerStart),
                          ],
                        ],
                      ),
                    ),
                    // The amount sits at the row's trailing edge (C5) and takes only
                    // the width it needs, up to 40 % of the row — the title and
                    // sub-line keep the rest.
                    if (!stackAmount) ...[
                      const SizedBox(width: 12),
                      ConstrainedBox(
                        constraints: BoxConstraints(
                          maxWidth: rowBox.maxWidth * 0.4,
                        ),
                        child: amountFigure(AlignmentDirectional.centerEnd),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  // A compact, locale-aware date+time (e.g. "Jun 24, 3:45 PM") via the shared
  // per-locale-memoized factory (#317 — the old static froze to
  // `Intl.defaultLocale`). Relative-time ("2h ago") is a follow-up — it needs
  // localized relative strings, and a locale-aware absolute time is honest
  // and non-misleading in the meantime.
  static String _formatTime(int unixSecs, String locale) =>
      walletCompactTimeFormat(
        locale,
      ).format(DateTime.fromMillisecondsSinceEpoch(unixSecs * 1000).toLocal());
}

/// The entry into the send flow (inc-2d-ui). Only rendered on the active wallet
/// surface — i.e. behind the money-safety gate (a backed-up wallet) — so a send
/// is structurally reachable only from a confirmed wallet. Pushes the send screen
/// (a back button returns here).
/// The ZODL-style action row (maintainer: "buttons in the same row, looking
/// nice"): Send / Receive / Swap as equal-width, circular icon-over-label
/// buttons. Send is gated on spendable funds (spend-before-sync), its honest
/// reason RESERVED below the row so enabling/disabling never shifts the layout.
/// Swap appears only when the host enabled it (§3.5; a host that didn't turn
/// swap on shows no swap surface at all). All three sit behind the active-wallet
/// money-safety gate, so they're structurally reachable only from a confirmed,
/// backed-up wallet.
class _WalletActions extends ConsumerWidget {
  const _WalletActions({
    required this.status,
    required this.spendableZat,
    this.syncNotRunning = false,
  });

  /// The live sync status — chooses the honest reason for a disabled Send.
  final SyncStatus status;

  /// Confirmed, spendable-now balance (the §2.5 SSOT). Send is enabled only when
  /// there is something to send; the SDK re-validates the exact amount on the
  /// form (§1.7 — this is the honest pre-gate, never the binding check).
  final int spendableZat;

  /// No sync pass will run — host policy off OR a failed start (#405; the
  /// R1 gate, widened from the policy to the SSOT). The disabled-Send
  /// reason must not claim "Still syncing" two widgets under a badge saying
  /// otherwise; this arm owns the reason over any retained status. Worded
  /// cause-agnostically, because the badge above it is what names the cause.
  final bool syncNotRunning;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final canSend = spendableZat > 0;
    // #397 §3.7 D3/D5: a WATCH-ONLY wallet cannot send, shield, or swap — HIDE
    // those affordances (never a greyed dead-button that only errors), leaving
    // Receive (a watch-only wallet still derives receive addresses from its
    // UFVK). The SDK refuses the money paths typed regardless, so this chrome
    // is honesty, not the gate.
    final watchOnly = ref.watch(isWatchOnlyProvider);
    // The third-slot tri-state (#356-F3): `live` renders the Swap button,
    // `reserved` holds an EMPTY equal-width slot while the activation outcome
    // is pending — so Send/Receive never shift under a finger mid-tap when the
    // enable resolves — and `absent` collapses to the two-button row (host
    // never enabled swap, or the enable resolved off). Watch-only forces it
    // absent — swap is a spend surface.
    final swapSlot = watchOnly ? SwapSlot.absent : ref.watch(swapSlotProvider);

    // Honest reason for a disabled Send (invariant 6/10 — never a silent dead
    // button). A stalled/offline wallet won't progress until connectivity / the
    // fault clears; a fully-synced wallet simply has nothing spendable;
    // otherwise sync is still catching up to the user's funds.
    final String reason;
    if (syncNotRunning) {
      // The drive truth wins over any retained status arm: with no
      // pass coming, nothing is syncing, stalling, or catching up.
      reason = l10n.walletSendSyncNotRunning;
    } else if (status is SyncStatus_Stalled || status is SyncStatus_Offline) {
      reason = l10n.walletSendSyncUnavailable;
    } else if (status is SyncStatus_UpToDate) {
      reason = l10n.walletSendNoSpendableYet;
    } else {
      reason = l10n.walletSendWaitingForFunds;
    }

    // The 3-up tile grid (FR-49 S12, C4): Send tonal, Receive primary, Swap
    // tonal, 10 apart.
    final tiles = <Widget>[
      if (!watchOnly)
        _ActionButton(
          actionKey: const ValueKey('wallet-action-send'),
          icon: WalletGlyph.send,
          label: l10n.walletSendButton,
          // Spend-before-sync gate: nothing spendable ⇒ no dead-end into a
          // form that can only fail. NOT gated on sync % — funds are usable
          // the moment a near-tip note clears, well before 100% (§1.7).
          enabled: canSend,
          onTap: () => context.push(WalletRoutes.send),
        ),
      _ActionButton(
        actionKey: const ValueKey('wallet-action-receive'),
        icon: WalletGlyph.receive,
        label: l10n.walletReceive,
        enabled: true,
        primary: true,
        onTap: () => context.push(WalletRoutes.receive),
      ),
      if (swapSlot == SwapSlot.live)
        _ActionButton(
          actionKey: const ValueKey('wallet-action-swap'),
          icon: WalletGlyph.swap,
          label: l10n.walletSwapButton,
          enabled: true,
          onTap: () => context.push(WalletRoutes.swap),
        )
      // Reserved: an empty equal-width slot, NOT a greyed button — a disabled
      // Swap would be an optimistic surface (and a silent dead button) for a
      // feature that may resolve off.
      else if (swapSlot == SwapSlot.reserved)
        const SizedBox.shrink(),
    ];

    return Column(
      children: [
        // IntrinsicHeight + stretch: every tile takes the tallest one's
        // height, so a label that wraps at a large text scale never leaves
        // the three tiles uneven.
        IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (var i = 0; i < tiles.length; i++) ...[
                if (i > 0) const SizedBox(width: 10),
                Expanded(child: tiles[i]),
              ],
            ],
          ),
        ),
        // The reason occupies its space whether shown or hidden, so Send
        // toggling enabled↔disabled (funds arriving mid-sync) never shifts the
        // content below (maintainer: no layout shift on hint toggle). Hidden ⇒ also
        // dropped from semantics, so a screen reader doesn't read a reason for an
        // enabled button. Absent entirely for watch-only — there is no Send
        // button to explain.
        if (!watchOnly)
          Visibility(
            visible: !canSend,
            maintainSize: true,
            maintainAnimation: true,
            maintainState: true,
            child: Padding(
              padding: const EdgeInsets.only(top: 10),
              child: Text(
                reason,
                style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
                textAlign: TextAlign.center,
              ),
            ),
          ),
      ],
    );
  }
}

/// The #397 watch-only header chip — a small, non-alarming "Watch-only" pill
/// beside the wallet title, marking a wallet that holds no spending keys. One
/// merged Semantics node so a screen reader reads "Watch-only" once.
class _WatchOnlyBadge extends StatelessWidget {
  const _WatchOnlyBadge();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Semantics(
      container: true,
      // excludeSemantics: without it the child Text contributes its OWN node
      // beside this label and a screen reader says "Watch-only" twice
      // (device-confirmed). The label carries the one announcement; the
      // icon + pill text are presentation.
      excludeSemantics: true,
      label: l10n.walletWatchOnlyBadge,
      child: Container(
        key: const ValueKey('wallet-watch-only-badge'),
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
        decoration: BoxDecoration(
          color: scheme.secondaryContainer,
          borderRadius: BorderRadius.circular(WalletShapes.of(context).chip),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(
              WalletGlyph.watchOnly,
              size: 14,
              color: scheme.onSecondaryContainer,
            ),
            const SizedBox(width: 4),
            // Flexible so at an extreme text scale the label wraps INSIDE the
            // chip instead of overflowing its line (the chip stays whole).
            Flexible(
              child: Text(
                l10n.walletWatchOnlyBadge,
                style: textTheme.labelSmall?.copyWith(
                  color: scheme.onSecondaryContainer,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// One circular icon-over-label action in the [_WalletActions] row. A disabled
/// action is greyed (never hidden) — the reason lives below the row. One merged
/// semantics node (iOS merge rule, flutter-patterns) carrying the label + the
/// activate action, so VoiceOver reads + fires it as a single button.
class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.actionKey,
    required this.icon,
    required this.label,
    required this.enabled,
    required this.onTap,
    this.primary = false,
  });

  final Key actionKey;
  final WalletGlyph icon;
  final String label;
  final bool enabled;
  final VoidCallback onTap;

  /// The one primary tile (Receive): `accent` fill. The others are tonal
  /// (`accentSoft` / `accentText`), as the maintainer's reference shows.
  final bool primary;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final fill = primary ? colors.accent : colors.accentSoft;
    final ink = primary ? colors.onAccent : colors.accentText;
    return Semantics(
      container: true,
      button: true,
      enabled: enabled,
      excludeSemantics: true,
      label: label,
      onTap: enabled ? onTap : null,
      // The refreshed action tile (FR-49 S12, C4): 88 tall, the `tile` radius
      // (22, stage S11 C1), a 22 glyph over the label. A disabled tile keeps
      // its colour role at 38 % opacity — it never turns into a
      // different-looking control.
      child: Opacity(
        opacity: enabled ? 1 : 0.38,
        child: Material(
          key: actionKey,
          color: fill,
          borderRadius: BorderRadius.circular(WalletShapes.of(context).tile),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            onTap: enabled ? onTap : null,
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 88),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 8,
                  vertical: 14,
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    WalletIcon(icon, size: 22, color: ink),
                    const SizedBox(height: 8),
                    Text(
                      label,
                      textAlign: TextAlign.center,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelLarge?.copyWith(
                        color: ink,
                        fontSize: 14,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Honest, recoverable cue when the background sync loop's start command itself
/// failed (rare — e.g. the handle closed). Never a silent failure (invariant
/// 10); offers the one sensible next step (invariant 6) — try again. The loop
/// also self-restarts on the next foreground resume, so this is belt-and-braces.
class _SyncStartFailedNotice extends ConsumerWidget {
  const _SyncStartFailedNotice();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    return _ActionNotice.prominent(
      icon: WalletGlyph.syncProblem,
      message: l10n.walletSyncStartFailed,
      actionLabel: l10n.walletSyncRetry,
      onAction: () => ref.read(walletSyncControllerProvider.notifier).retry(),
    );
  }
}

/// ONE shell for the wallet's five ACTION notices (#409 R3): a notice
/// carrying a warning glyph, one honest sentence, and exactly one action —
/// Try again, or Dismiss. Five hand-written copies of the same Row had drifted
/// only in decoration, and every one of them starved its own message.
/// ([_DeepScanCheckingBanner] is deliberately NOT one of them: its trailing
/// affordance is an icon-only dismiss whose width does not text-scale, so it
/// cannot starve its message and it needs no stacking rule.)
///
/// **The defect this class exists to remove.** A `Row` lays its non-flexible
/// children out FIRST, at their intrinsic width, and gives what is left to the
/// `Expanded` message. The button's label therefore takes its width off the
/// top, and the message gets the remainder however small that is. Measured on
/// THIS screen (320dp viewport ⇒ a 288dp card inside the surface's 16dp
/// padding) at the DEFAULT 1.0x, no text scaling involved:
///
/// * `en` — the sync-start message keeps **59dp of 288**, card 243dp tall.
/// * `de` — the same widget keeps **0.0dp**, and the card runs **1250dp**
///   tall. English is the mild case; the long-compound locales are the ones
///   that motivate the fix.
/// * the rescan-failed message keeps 80dp at 1.3x.
///
/// That is the state photographed, where the user who just confirmed a
/// DESTRUCTIVE rescan cannot read why it was refused and the ribbon pushes the
/// sync badge and the Try-again out of the viewport.
///
/// **None of those throws a RenderFlex overflow** (the notice's first throw is
/// at 360dp/3.0x, by which point the message is 0.0dp wide), so the pins in
/// `notice_layout_test.dart` measure the message's WIDTH. A `takeException`
/// assertion would have called the 1.0x phone healthy.
///
/// So past [walletRowStacksAction] the action moves onto its own line and the
/// message keeps the full width. The rule is shared with the parked-send and
/// in-flight-swap rows — one threshold, not a fourth copy of it.
class _ActionNotice extends StatelessWidget {
  /// The key on the card container, so a pin can measure the message against
  /// the card it actually sits in. Without it a test resolves "the card" by
  /// nearest-`Container` ancestor, and any future styling wrapper around the
  /// message silently redefines the denominator — measured: a bare `Container`
  /// around the message makes a 59dp message read as 100% of "the card" and
  /// every width pin in the file passes against the un-fixed layout.
  static const cardKey = ValueKey('wallet-action-notice-card');

  /// The top-of-surface treatment: the card-notice (a 20 glyph, the action a
  /// button under the message). Used by the start-failed notice, which sits
  /// above the balance.
  const _ActionNotice.prominent({
    required this.icon,
    required this.message,
    required this.actionLabel,
    required this.onAction,
  }) : _prominent = true;

  /// The in-line treatment: the notice line (a 16 glyph, an inline text
  /// action). Used by the four rescan-outcome notices, which sit inside the
  /// surface's flow.
  const _ActionNotice.compact({
    required this.icon,
    required this.message,
    required this.actionLabel,
    required this.onAction,
  }) : _prominent = false;

  final WalletGlyph icon;

  /// LOCALIZED COPY ONLY — never a wallet value (§5.4 render hygiene). This
  /// shell renders whatever it is handed, verbatim, on a money surface; when
  /// each notice was its own hand-written widget that rule was enforced by the
  /// call site being the only place a string could come from, and it is now
  /// one hop further away. Every call site passes an `l10n.*` getter.
  final String message;

  /// Localized copy only — see [message].
  final String actionLabel;

  final VoidCallback onAction;
  final bool _prominent;

  @override
  Widget build(BuildContext context) {
    // Stage S11 C2: the prominent form is a card-notice (no title: there is no
    // title string, and S11 adds none), its action a §6.7 button under the
    // message; the compact form is the notice line, whose inline text action
    // moves under the message past [walletRowStacksAction] — the rule above,
    // now owned by `WalletNotice`.
    if (_prominent) {
      return WalletNotice.card(
        key: cardKey,
        tone: WalletNoticeTone.warning,
        glyph: icon,
        message: message,
        actions: [
          OutlinedButton(onPressed: onAction, child: Text(actionLabel)),
        ],
      );
    }
    return WalletNotice(
      key: cardKey,
      tone: WalletNoticeTone.warning,
      glyph: icon,
      message: message,
      actions: [TextButton(onPressed: onAction, child: Text(actionLabel))],
    );
  }
}

/// Honest staleness cue when the last-known balance is shown through a failed
/// refresh (the cold read keeps erroring) — keeps the wallet usable without
/// pretending the balance is fresh.
class _StaleNotice extends StatelessWidget {
  const _StaleNotice();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    return Row(
      children: [
        WalletIcon(WalletGlyph.behind, size: 16, color: colors.orange),
        const SizedBox(width: 6),
        Expanded(
          child: Text(
            l10n.walletBalanceStale,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: colors.orange),
          ),
        ),
      ],
    );
  }
}

/// The "rebuilding your history" banner (FR-1b / ADR-0534). Shown across the top
/// of the active surface while a rescan repopulates: it explains, up front and
/// reassuringly, WHY the balance + activity briefly emptied (a deliberate rescan,
/// not lost funds). A positive/neutral treatment with a sync glyph — never a
/// warning. Cleared on reached-tip by [WalletRescanController] — or, on the
/// durable arms, by the same edge flipping [walletCatchUpCueProvider] off.
class _RescanRebuildingBanner extends StatelessWidget {
  const _RescanRebuildingBanner({
    required this.target,
    this.resumedRescan = false,
  });

  /// WHICH recovery is repopulating — the copy names what the user chose.
  /// `null` with [resumedRescan] is the durable rescan arm (#377 s357b-2):
  /// the breadcrumb proves a rescan rebuild, but the range choice did not
  /// survive the relaunch. `null` without it is the durable-signal arm
  /// (#380): the wallet is provably still catching up (never-synced + below
  /// tip) with no rescan evidence — a restore/create's first sync — so the
  /// copy is the generic catch-up reassurance.
  final RescanTarget? target;

  /// See [WalletCatchUpRebuilding.target] — `true` iff the durable
  /// rescan-rebuilding breadcrumb (not the first-run arm) put us here, so the
  /// copy can name the rescan (the user's own action) without its lost range.
  final bool resumedRescan;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final message = switch (target) {
      null =>
        resumedRescan
            ? l10n.walletCatchUpRescanBanner
            : l10n.walletCatchUpBanner,
      RescanAllHistory() => l10n.walletRescanRebuildingAll,
      RescanFromWalletBirthday() => l10n.walletRescanRebuildingDefault,
      RescanFromTime(:final earliestTime) => l10n.walletRescanRebuildingFrom(
        MaterialLocalizations.of(context).formatMonthYear(earliestTime),
      ),
    };
    // #377 s357b-1: one merged, LIVE a11y node — the banner's appearance (and
    // a copy change, e.g. rescan-start mid-session) is announced to screen
    // readers without manual navigation, per the liveRegion convention on
    // meaningful money-surface state (the `_SyncBadge` / arrival-SnackBar
    // precedent). The glyph is decorative; the Text is the whole story.
    //
    // A static sync glyph (NOT a spinner): the live progress is the
    // `_SyncBadge` above; a second perpetual animation here would only add
    // motion noise (and hang `pumpAndSettle`). Positive, not a warning.
    return WalletNotice(
      key: const Key('wallet-catchup-banner'),
      tone: WalletNoticeTone.positive,
      glyph: WalletGlyph.syncing,
      message: message,
      liveRegion: true,
    );
  }
}

/// #390 C1 — the deep-scan progress cue. A widen keeps surfacing older swap
/// money for minutes after the sheet closes; this dismissible banner is the
/// only place that says so once the sheet is gone (review H2). Sync-neutral
/// copy ("as it's found", not "as your wallet syncs") so it stays honest while
/// offline. Positive brand-green, a static glyph (the live progress is the
/// `_SyncBadge`), and a Dismiss affordance (the cue is informational, not an
/// error — the user can clear it and the widen keeps working underneath).
class _DeepScanCheckingBanner extends ConsumerWidget {
  const _DeepScanCheckingBanner();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    return WalletNotice(
      tone: WalletNoticeTone.positive,
      glyph: WalletGlyph.rescan,
      message: l10n.walletDeepScanBannerChecking,
      // Dismiss — the cue is informational; clearing it does NOT stop the
      // widen (a session-scoped flag hides the banner until the next run).
      dismissLabel: l10n.walletDeepScanRestoreNoteDismiss,
      onDismiss: () =>
          ref.read(swapDeepScanProgressProvider.notifier).dismiss(),
    );
  }
}

/// #390 C2 — the one-time post-restore note (review H1). A proactive, dismissible
/// card shown ONCE (until acknowledged, durably) on a restored wallet after its
/// first catch-up: its oldest swaps may hold refunds/deposits past the restore
/// sweep's ceiling (#387), which the deep scan recovers. "Check now" opens the
/// sheet; both actions record `done` so it never returns.
class _DeepScanRestoreNote extends ConsumerWidget {
  const _DeepScanRestoreNote();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    // The line has no title, so the heading is its message and the detail
    // rides below it in `child`. Its two actions stack under the text at large
    // text or a narrow width (the line's `walletRowStacksAction` rule, which
    // is directional for ar/he).
    return WalletNotice(
      tone: WalletNoticeTone.info,
      glyph: WalletGlyph.rescan,
      message: l10n.walletDeepScanRestoreNoteTitle,
      actions: [
        TextButton(
          onPressed: () =>
              ref.read(deepScanRestoreNoteProvider.notifier).acknowledge(),
          child: Text(l10n.walletDeepScanRestoreNoteDismiss),
        ),
        FilledButton(
          onPressed: () {
            // Acknowledge (hide + durably record done), THEN open the
            // deep-scan sheet.
            unawaited(
              ref.read(deepScanRestoreNoteProvider.notifier).acknowledge(),
            );
            unawaited(showSwapDeepScanSheet(context));
          },
          child: Text(l10n.walletDeepScanRestoreNoteCheck),
        ),
      ],
      child: Text(
        l10n.walletDeepScanRestoreNoteBody,
        style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
      ),
    );
  }
}

/// The rescan fence notice (§4.4 W-swap-4-a-3): the SDK refused to rebuild
/// while a send is still settling on-chain (rebuilding under it could pay the
/// same recipient twice). Honest, HOURS-scale copy — distinct from
/// [_RescanFailedNotice]'s "try again in a moment", which would invite retry
/// churn against a refusal that holds until the send settles. Same dismiss.
class _RescanBlockedSettlingNotice extends ConsumerWidget {
  const _RescanBlockedSettlingNotice();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    return _ActionNotice.compact(
      icon: WalletGlyph.waiting,
      message: l10n.walletRescanBlockedSettlingNotice,
      actionLabel: l10n.walletRescanFailedDismiss,
      onAction: () =>
          ref.read(walletRescanControllerProvider.notifier).dismissFailure(),
    );
  }
}

/// The no-sync-pass rescan refusal (#405): the commit-point fence turned the
/// confirm away because neither the host's policy nor a failed start leaves a
/// sync pass to rebuild through. Nothing destructive ran — the copy says
/// "unchanged" outright, which [_RescanFailedNotice] deliberately cannot (#379).
/// CAUSE-AGNOSTIC by contract (see `walletSyncPassesRunProvider`): it names the
/// condition and stops; the badge two widgets up, and the start-failed notice
/// with its own Try-again, carry the cause and the way out. Same dismiss.
class _RescanBlockedSyncNotRunningNotice extends ConsumerWidget {
  const _RescanBlockedSyncNotRunningNotice();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    return _ActionNotice.compact(
      icon: WalletGlyph.syncOff,
      message: l10n.walletRescanBlockedSyncNotRunningNotice,
      actionLabel: l10n.walletRescanFailedDismiss,
      onAction: () =>
          ref.read(walletRescanControllerProvider.notifier).dismissFailure(),
    );
  }
}

/// The disk-full rescan notice (#375, UX HIGH F1): the SDK's typed
/// `DiskFull` from the rebuild + WAL fold — a deterministic refusal only the
/// user can clear, on a device whose sync badge stays HEALTHY (routine small
/// commits still fit). Actionable "free up space" copy, never
/// [_RescanFailedNotice]'s "try again in a moment" (which would invite a
/// retry lie-loop against a full disk — the same refusals-that-hold
/// principle as the settling-send fence). Same dismiss.
class _RescanNeedsSpaceNotice extends ConsumerWidget {
  const _RescanNeedsSpaceNotice();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    return _ActionNotice.compact(
      icon: WalletGlyph.storageFull,
      message: l10n.walletRescanNeedsSpaceNotice,
      actionLabel: l10n.walletRescanFailedDismiss,
      onAction: () =>
          ref.read(walletRescanControllerProvider.notifier).dismissFailure(),
    );
  }
}

/// The honest "couldn't rescan" notice (FR-1b). Shown when a rescan FAILED but
/// the wallet was recovered — at its prior birthday on pre-rename faults, at
/// the REBUILT lower birthday on the post-rename arms (funds safe either way;
/// the copy claims only funds-safety, never "unchanged" — #379). A dismissable
/// warning treatment; the dismiss returns the rescan controller to idle.
class _RescanFailedNotice extends ConsumerWidget {
  const _RescanFailedNotice();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    return _ActionNotice.compact(
      icon: WalletGlyph.error,
      message: l10n.walletRescanFailedNotice,
      actionLabel: l10n.walletRescanFailedDismiss,
      onAction: () =>
          ref.read(walletRescanControllerProvider.notifier).dismissFailure(),
    );
  }
}

class _SnapshotError extends StatelessWidget {
  const _SnapshotError();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          l10n.walletSnapshotUnavailable,
          textAlign: TextAlign.center,
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: colors.textMuted),
        ),
      ),
    );
  }
}

/// The persistent, compact, color-coded sync badge (spec §3.3, ADR-0533). One
/// glance answers "can I trust the balance and transact right now?" via a
/// traffic light — GREEN usable+fresh, YELLOW limited/stale, RED can't-sync.
/// Since S12 (FR-49 C2, ADR-0564, the maintainer's adoption of the refreshed
/// design) it HIDES while the wallet is synced and healthy
/// ([walletSyncBarVisible]); the earlier persistent-badge ask is superseded,
/// and the live status stays one tap away in the overflow menu. A routine
/// one-block pass does not bring it back ([walletTipFollowProvider]).
///
/// (maintainer): ONE FIXED-HEIGHT ROW across every state — "Up to date" →
/// "Scanning"/"Sync paused" must never change the badge's size and shove the
/// balance card (the layout-shift report). The scan bar rides INSIDE the row,
/// the detail lines moved to the tap-through sheet (#319), and a trailing ⓘ
/// signals the affordance: tapping anywhere opens the live "what's going on"
/// sheet. The a11y label still carries the detail lines, so screen readers
/// lose nothing to the fixed height. The compact blocks-left count keeps
/// riding next to the percent (maintainer: "keep block counts near percentage");
/// the giant exact number stays demoted to the sheet.
class _SyncBadge extends ConsumerWidget {
  const _SyncBadge({
    required this.status,
    required this.stale,
    this.driving = false,
    this.startFailed = false,
    this.syncDisabled = false,
    required this.tor,
  });

  final SyncStatus status;

  /// The last cold refresh failed — the on-screen figures may be out of date.
  /// Pulls an otherwise-GREEN badge down to YELLOW (honest staleness; the
  /// dedicated `_StaleNotice` above carries the detail).
  final bool stale;

  /// Whether the host is actively driving the sync loop ([WalletSyncDrive.running]).
  /// Disambiguates the SDK's `Idle`: the loop starts a pass with a silent prep
  /// phase (commitment-tree roots + chain-tip fetch) that emits no status, so a
  /// freshly-started loop sits at `Idle` until its first batch reports. With the
  /// loop driving, that window is honestly "Connecting…", not "Not syncing yet".
  final bool driving;

  /// Whether the start command itself FAILED ([WalletSyncDrive.failed]) — the
  /// Idle arm then reads the honest start-failure detail (#356-F8), never the
  /// default "starts automatically" (which the a11y label would otherwise
  /// speak right beside the retry notice contradicting it).
  final bool startFailed;

  /// Whether the HOST's sync policy is off (#383 R1,
  /// [WalletSyncDrive.disabledByHost]) — the badge then reads the honest
  /// "Sync off" story over ANY retained status, and the level caps at
  /// caution (a green "fresh" claim under a deliberately-idle loop would lie).
  final bool syncDisabled;

  /// The SDK's Tor state from the cold snapshot — the transport indicator's
  /// input when the host wired no transport claim of its own
  /// ([walletHostTransportProvider]).
  ///
  /// ORTHOGONAL TO SYNC HEALTH (review — the Tor/RPC-endpoint nuance):
  /// the transport tone attests the Tor PATH, not that the lightwalletd RPC
  /// endpoint is reachable — the two axes are independent (mirrors the core's
  /// `live_tor_state` note). So a PROTECTED (green) transport shield can sit
  /// beside a Stalled/Offline sync badge without contradiction: the onion
  /// path is up while the endpoint is the problem (`EndpointUnreachable`).
  /// Only a failure to reach Tor ITSELF pulls this to `fellBack`/`unavailable`.
  /// The badge shows both signals because a user needs both truths at once.
  ///
  /// STALENESS WINDOW (review H4, inherited from the old Tor chip —
  /// same data source): the snapshot refreshes on resume and the
  /// balance-changing sync edges, so a mid-scan Tor transition (e.g.
  /// `active → fellBack`) can keep the previous tone until the next edge.
  /// A live Tor-state stream / snapshot invalidation on Tor edges is the
  /// SDK-side fix (filed in the #318 basket).
  final TorState tor;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // The transport indicator: ICON-ONLY in this fixed row —
    // the overflow lesson (a text chip broke 320dp at accessibility
    // scales); the label rides the a11y string and the sheet's Connection
    // section. The host's own transport claim wins when wired.
    final transport = transportPresentation(
      l10n,
      tor: tor,
      host: ref.watch(walletHostTransportProvider),
    );
    final transportTint = switch (transport.tone) {
      TransportTone.protected => colors.green,
      TransportTone.neutral => colors.textMuted,
      TransportTone.progress => colors.cyan,
      TransportTone.caution => colors.orange,
      TransportTone.danger => colors.red,
    };

    // The traffic-light color from the pure health derivation — the single new
    // load-bearing rule, gate-8 truth-tabled in wallet_health_test. The icon
    // stays per-arm (it carries the *kind* of state); the COLOR carries health.
    final level = walletBadgeLevel(
      status: status,
      stale: stale,
      startFailed: startFailed,
      syncDisabled: syncDisabled,
    );
    final tint = switch (level) {
      WalletBadgeLevel.ok => colors.green,
      WalletBadgeLevel.caution => colors.orange,
      WalletBadgeLevel.error => colors.red,
    };

    // The per-arm words/icon/bar come from the ONE shared mapping
    // (sync_status_presentation.dart) so this row and the detail sheet it
    // opens can never disagree on the state they describe.
    final p = syncStatusPresentation(
      l10n,
      status,
      driving: driving,
      startFailed: startFailed,
      syncDisabled: syncDisabled,
    );

    return Semantics(
      container: true,
      // One coherent node (iOS merges sibling Text otherwise — flutter-
      // patterns § iOS Semantics merging): read the headline + the trailing
      // count + every detail once, not each child twice. The detail lines are
      // no longer VISIBLE here (fixed height) but stay in the label — screen
      // readers keep the full story without opening the sheet.
      excludeSemantics: true,
      // Announce status changes (sync stalled, funds became spendable) to screen
      // readers without manual navigation — meaningful on a money surface.
      liveRegion: true,
      button: true,
      // excludeSemantics drops the InkWell's own action, so the a11y tap
      // action must live on THIS node (the mnemonic-reveal lesson: claims and
      // affordances the child owns don't survive exclusion).
      onTap: () => showSyncStatusSheet(context),
      onTapHint: l10n.walletSyncBadgeHint,
      // The transport label rides here (the row shows only its icon) so a
      // screen reader hears the privacy state without opening the sheet.
      label: [
        p.headline,
        transport.label,
        ?p.trailing,
        ...p.details,
      ].join('. '),
      child: Material(
        // The refreshed sync bar (FR-49 S12, C2): a recessed `bgHover` card at
        // the `bar` radius, no border. Color on the Material so the ripple
        // renders; the ripple reads the same radius as its container.
        color: colors.bgHover,
        borderRadius: BorderRadius.circular(WalletShapes.of(context).bar),
        child: InkWell(
          borderRadius: BorderRadius.circular(WalletShapes.of(context).bar),
          onTap: () => showSyncStatusSheet(context),
          child: Padding(
            // ONE line of content at constant padding in EVERY state — the
            // whole no-layout-shift guarantee lives here: nothing below may
            // add or remove a vertical element per state. Vertical 14 keeps
            // the tap target above Android's 48dp Material minimum.
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
            child: Row(
              children: [
                WalletIcon(p.icon, color: tint, size: 20),
                const SizedBox(width: 10),
                // The middle is ONE tight flex group so any leftover from the
                // headline's loose allotment stays INSIDE it (UX review:
                // RenderFlex does NOT redistribute a loose child's unused
                // allotment — unnested it became dead space AFTER the trailing
                // icon and unpinned the ⓘ by up to ~250px per state).
                Expanded(
                  child: Row(
                    // spaceBetween parks the bar against the group's right
                    // edge (adjacent to the ⓘ) and turns the leftover into
                    // the natural gap between text and bar.
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      // Flexible+ellipsis: a long localized headline under a
                      // large text scale cedes space instead of overflowing.
                      // The headline reads in the body colour; the glyph
                      // before it carries the tone (C2).
                      Flexible(
                        child: Text(
                          p.headline,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: textTheme.bodyMedium?.copyWith(
                            color: colors.text,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ),
                      // The scan bar rides INSIDE the row (maintainer: "progress
                      // bar should stay in the same row"). The compact
                      // blocks-left chip is NOT rendered here anymore — it
                      // overflowed narrow screens at accessibility scales
                      // (UX review); it still rides the a11y label, and
                      // the sheet one tap away carries the EXACT count.
                      if (p.scanning)
                        Flexible(
                          child: Padding(
                            // DIRECTIONAL (#401 R8b): the 12dp gap separates the
                            // bar from the status text BEFORE it, which is on the
                            // right in ar/he.
                            padding: const EdgeInsetsDirectional.only(
                              start: 12,
                            ),
                            child: ClipRRect(
                              borderRadius: BorderRadius.circular(
                                WalletShapes.of(context).progress,
                              ),
                              // The theme owns the bar's colours and height
                              // (stage S11 C6); the sync STATE rides the
                              // tinted glyph at the row's start.
                              child: LinearProgressIndicator(
                                // null ⇒ indeterminate (opaque early phase).
                                value: p.progress,
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                // The transport privacy indicator — filled
                // shield ONLY when protection is positively verified (§3.3);
                // everything else renders the outline in its honesty tone.
                WalletIcon(
                  transport.tone == TransportTone.protected
                      ? WalletGlyph.protected
                      : WalletGlyph.shielded,
                  size: 16,
                  color: transportTint,
                ),
                const SizedBox(width: 8),
                // The visible cue that details exist one tap away (maintainer:
                // "i icon on the right hand side explaining each state").
                WalletIcon(WalletGlyph.info, size: 18, color: colors.textMuted),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// The five reached-tip variants as a comparable KIND (§4r U-4) — a completed
/// pass on a current server (`upToDate`, and the three that qualify it:
/// `upToDateLimited`, `upToDateDegraded`, `upToDateUnverified`) or on a
/// behind one (`endpointBehind`). Null for every other state and for a
/// missing sample, so "the edge into a reached tip" is `kind(next) != null &&
/// kind(next) != kind(prev)`. Wildcard-free on purpose: a `SyncStatus`
/// variant added later is a compile error HERE, never a sixth site the sweep
/// forgot (§4m #18's own shape).
enum _ReachedTip { upToDate, limited, degraded, behind, unverified }

_ReachedTip? _reachedTipVariant(SyncStatus? s) => switch (s) {
  SyncStatus_UpToDate() => _ReachedTip.upToDate,
  SyncStatus_UpToDateLimited() => _ReachedTip.limited,
  SyncStatus_UpToDateDegraded() => _ReachedTip.degraded,
  SyncStatus_EndpointBehind() => _ReachedTip.behind,
  SyncStatus_UpToDateUnverified() => _ReachedTip.unverified,
  SyncStatus_Idle() ||
  SyncStatus_Connecting() ||
  SyncStatus_Scanning() ||
  SyncStatus_Stalled() ||
  SyncStatus_Offline() ||
  SyncStatus_Unknown() => null,
  null => null,
};

/// What the Balance header's "(as of block N)" shows (the as-of
/// height belongs IN the header, never a floating row). Pure so the
/// height/time PAIRING rule is unit-testable: the timestamp renders ONLY when
/// it belongs to the displayed height — pairing the live tip with an older
/// stamp's clock time would lie about when that tip landed.
///
/// [latch] is the session's tri-state verdict ([walletSyncedTipProvider]).
/// Height precedence: the LIVE up-to-date tip > the latched tip > (only when
/// the session has NO verdict) the persisted stamp. An INVALIDATED verdict
/// suppresses both fallbacks — a reorg rewind just declared that height
/// unsafe, and falling back to the stamp would resurrect it (the wrap
/// review's HIGH finding); the header honestly shows plain "Balance" until
/// the next up-to-date.
///
/// Time precedence at the displayed height: the durable stamp (the SDK's
/// authoritative record, refreshed on every clean pass) > the latch's own
/// observation moment (the gap-filler while the stamp's async re-read is in
/// flight — without it the header pulsed AsOfAt→AsOf→AsOfAt every sync
/// cycle, a measured 64px card shift at ru/2.0×/320dp).
///
/// Since S12 the card's caption shows only the TIME,
/// "Balance · 11:38 PM" (see [balanceCaptionTime]); the height still decides
/// WHETHER a time shows, so every rule above holds.
({int? height, DateTime? time}) balanceAsOf({
  required SyncStatus status,
  required WalletSyncedTip latch,
  required SyncStamp? lastSynced,
}) {
  final int? height = switch (status) {
    SyncStatus_UpToDate(:final tip) => tip,
    _ => switch (latch) {
      WalletSyncedTipLatched(:final tip) => tip,
      WalletSyncedTipUnset() => lastSynced?.height,
      WalletSyncedTipInvalidated() => null,
    },
  };
  final stamp = lastSynced;
  final DateTime? time = (stamp != null && stamp.height == height)
      ? DateTime.fromMillisecondsSinceEpoch(stamp.at * 1000)
      : (latch is WalletSyncedTipLatched && latch.tip == height)
      ? latch.at
      : null;
  return (height: height, time: time);
}

/// The clock [balanceCaptionTime] reads "today" from. A test pins it, so a run
/// that crosses midnight cannot turn "today" into "yesterday" mid-test.
@visibleForTesting
DateTime Function() balanceCaptionNow = DateTime.now;

/// The balance caption's time (S12, "Balance · 11:38 PM"): the
/// time alone when it is today, date and time otherwise.
String balanceCaptionTime(DateTime t, String locale) =>
    DateUtils.isSameDay(t, balanceCaptionNow())
    ? DateFormat.jm(locale).format(t)
    : walletCompactTimeFormat(locale).format(t);

class _BalanceCard extends ConsumerWidget {
  const _BalanceCard({
    required this.balance,
    this.recoverable = const (totalZat: 0, allFinal: true),
    this.asOf = const (height: null, time: null),
    this.synced = false,
  });

  final BalanceSnapshot balance;

  /// The wallet is at the tip (plain `UpToDate`, or following it). Only then
  /// is the core's pending-incoming NEW money, so only then does the headline
  /// show what the user has now with "Arriving" apart (S12 rev.3, maintainer).
  final bool synced;

  /// The header's as-of height + timestamp (see [balanceAsOf]).
  final ({int? height, DateTime? time}) asOf;

  // As-of timestamp — the activity rows' `MMMd + jm` idiom via the shared
  // locale-aware factory (#317).

  /// The recoverable one-time-address subset (2e-2b-iv) — rendered as a note
  /// UNDER the transparent line (it is a SUBSET of `transparentZat`), never a
  /// peer row that would make the breakdown over-sum. Defaults to zero (no row).
  final RecoverableSummary recoverable;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    // The auto-shield honesty cue (§3.2i-3 (a)): shown only when the LOOP
    // could not complete (failed attempt, or the host authorizer denied the
    // automatic spend) — a deliberately-OFF switch shows no cue (holding
    // transparent is then the user's explicit choice, and the standing
    // transparent note above still explains the visibility).
    final autoShield = ref.watch(walletAutoShieldControllerProvider);
    final autoShieldIncomplete =
        autoShield == AutoShieldStatus.failed ||
        autoShield == AutoShieldStatus.denied;
    // #397 §3.7 D3/D5 money-honesty: a watch-only wallet holds no spending keys,
    // so it can never spend or shield. The card must not FRAME its funds as
    // spendable — the "Spendable now" line is hidden, the transparent note drops
    // its "shield to spend" clause (keeping the public-visibility fact), and the
    // pool line + Shield button stop being gateways to the spend sheet. The
    // holdings themselves (total, shielded/transparent split, incoming) stay —
    // watching is the whole point.
    final watchOnly = ref.watch(isWatchOnlyProvider);
    // Defensive subset invariant (2e-2b-iv operational round; the why is on
    // [clampRecoverableToTransparent]): the note can NEVER claim more on a
    // one-time address than the transparent line above it. For this note an
    // under-report during a skew is the safe direction; it self-heals on the
    // next refresh.
    final recoverableZat = clampRecoverableToTransparent(
      recoverable.totalZat,
      balance.transparentZat,
    );
    // Hide balance (FR-49 W-7): every figure on the card, the recoverable
    // note's included, reads as dots while hidden.
    final hidden = ref.watch(walletBalanceHiddenProvider);
    final recoverableFigure = displayedAmount(
      formatZec(recoverableZat),
      hidden: hidden,
    );
    // The recoverable-funds note around a given figure. #397 D3: the
    // "(recoverable)" claim promises an action a watch-only wallet can't take
    // (recovery MINTS a self-send — a spend; the reclaim affordance is
    // hidden), so it keeps the locational fact and drops the claim. The
    // still-confirming variant is neutral, so it needs no watch-only twin.
    String recoverableNote(String figure) => recoverable.allFinal
        ? (watchOnly
              ? l10n.walletRecoverableEphemeralNoteWatchOnly(figure)
              : l10n.walletRecoverableEphemeralNote(figure))
        : l10n.walletRecoverableEphemeralConfirmingNote(figure);
    // The headline (S12 rev.3, maintainer: "show the actual amount that we have
    // now and pending is a separate part"). Synced, the core's pending-incoming
    // is new money not yet confirmed: it leaves the headline and shows as
    // "Arriving". Not synced, it also holds money the user already had
    // (witnesses not yet buildable), so the headline stays the total and the
    // sync bar explains the "Not spendable yet" row. One figure, read by every
    // row below, so the rows always account for what the headline says.
    final arrivingZat = synced ? balance.pendingIncomingZat : 0;
    final headZat = balance.totalZat - arrivingZat;
    return Container(
      // Keyed so tests can pin the NO-FLAP rule: the header is one
      // Text, and through a ROUTINE re-scan the tri-state latch holds the
      // paired height+time so the card height does not flap cycle-to-cycle
      // (the 64px AsOfAt↔AsOf pulse fix). NOT a claim of a constant height
      // across EVERY transition: a reorg that INVALIDATES the latch honestly
      // returns the header to the bare "Balance" — a rare, one-time height
      // change of the same class as the pre-first-sync → first-synced
      // transition the design already accepts (honesty over resurrecting an
      // orphaned height). A plain full-width Text wraps at large scales — no
      // Row, no overflow trap.
      key: const ValueKey('wallet-balance-card'),
      width: double.infinity,
      // The brand surface (FR-49 S12, C3): `deep` at the `hero` radius, min
      // 176 tall. Clipped so the decorative rings stay inside the corner.
      constraints: const BoxConstraints(minHeight: 176),
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: colors.deep,
        borderRadius: BorderRadius.circular(WalletShapes.of(context).hero),
      ),
      child: Stack(
        children: [
          // Two faint rings at the top-trailing corner: decoration only.
          PositionedDirectional(
            top: -60,
            end: -60,
            child: _Ring(diameter: 220, color: colors.onDeep, opacity: 0.08),
          ),
          PositionedDirectional(
            top: -20,
            end: -20,
            child: _Ring(diameter: 140, color: colors.onDeep, opacity: 0.10),
          ),
          PositionedDirectional(
            top: 46,
            end: 26,
            child: WalletCoin(spins: ref.watch(walletCoinSpinsProvider)),
          ),
          Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // "(as of block N[, time])" rides IN the header as
                // ONE Text — never a second line within a state, so the card height
                // does not flap during a routine re-scan (the old
                // "age unknown" sub-line appeared/disappeared per cycle AND was
                // dishonest on a wallet that had already synced — the badge above
                // carries the sync story; pre-first-sync, and after a reorg
                // invalidates the latch, the header is simply "Balance").
                // S12 (maintainer, on the OnePlus capture): "Balance · 11:38 PM"
                // — the time the figure is as of, the date added when it is
                // not today, on ONE line on a small phone. The block height
                // moved to the sync sheet one tap away (its tip rows); with no
                // paired time the caption is plain "Balance".
                Text(
                  switch ((asOf.height, asOf.time)) {
                    (final int _, final DateTime t) =>
                      l10n.walletBalanceHeaderAt(
                        balanceCaptionTime(t, l10n.localeName),
                      ),
                    _ => l10n.walletBalanceLabel,
                  },
                  style: textTheme.labelMedium?.copyWith(
                    color: colors.deepMuted,
                  ),
                ),
                // The total sits low on the card, clear of the coin at its
                // trailing side: it scales down to fit and never truncates (a
                // clipped figure on a money surface is a comprehension hazard).
                const SizedBox(height: 56),
                Padding(
                  padding: const EdgeInsetsDirectional.only(end: 104),
                  child: FittedBox(
                    fit: BoxFit.scaleDown,
                    alignment: AlignmentDirectional.centerStart,
                    child: Text(
                      displayedAmount(
                        l10n.walletAmount(formatZec(headZat)),
                        hidden: hidden,
                      ),
                      key: const ValueKey('wallet-balance-total'),
                      semanticsLabel: hidden
                          ? l10n.walletBalanceHiddenAmount
                          : null,
                      style: textTheme.displaySmall?.copyWith(
                        color: colors.onDeep,
                      ),
                    ),
                  ),
                ),
                // The breakdown appears only once there is a balance to break down.
                // At zero (a fresh wallet still scanning toward its funds) a
                // "Spendable now: 0" line next to a disabled Send reads as a
                // contradiction (maintainer report) — so the headline "0 ZEC" stands
                // alone, and the disabled-Send reason carries the why.
                // The EVERYDAY card (FR-49 S12, C3; Relim §9.4 rev.2) is the header,
                // the total and the coin — nothing else. Each row below appears only
                // when it says something the total does not.
                if (headZat > 0) ...[
                  // Pool clarity (#389): how much is SHIELDED vs TRANSPARENT. Shown
                  // when some of the balance is transparent — when all of it is
                  // shielded the split adds nothing the headline does not say. For
                  // a WATCH-ONLY wallet (which never shows Spendable) also whenever
                  // a pending row INSIDE the headline renders, so the rows it does
                  // show are always accounted for against it (rev.2 R4; rev.3 N3).
                  // shielded == headline − transparent: the two figures sum
                  // EXACTLY to the headline and can never disagree with it.
                  if (balance.transparentZat > 0 ||
                      (watchOnly &&
                          ((!synced && balance.pendingIncomingZat > 0) ||
                              balance.pendingChangeZat > 0))) ...[
                    const SizedBox(height: 12),
                    _PoolLine(
                      shieldedZat: headZat - balance.transparentZat,
                      transparentZat: balance.transparentZat,
                      hidden: hidden,
                      // The pool line opens the transparent-funds sheet (auto-shield
                      // + move-to-transparent — both SPENDS). Informational-only for
                      // a watch-only wallet: the split still renders, but there is no
                      // tap into a sheet it can't act on.
                      onTap: watchOnly
                          ? null
                          : () => showTransparentFundsSheet(context),
                    ),
                  ],
                  // "Spendable now" only when it DIFFERS from the headline: equal,
                  // it repeats it; different, the gap is exactly what the user
                  // must see (change in flight, transparent funds to shield,
                  // funds the catch-up has not yet made spendable). Meaningless
                  // for a watch-only wallet (no keys — nothing is spendable).
                  if (!watchOnly && balance.spendableZat != headZat) ...[
                    const SizedBox(height: 12),
                    _Line(
                      label: l10n.walletSpendableLabel,
                      amountZat: balance.spendableZat,
                      hidden: hidden,
                    ),
                  ],
                ],
                // Not synced: pending-incoming is inside the headline (it may be
                // money the user already had, held until its witnesses exist),
                // so it is a "Not spendable yet" breakdown row here — never
                // "Arriving", which reads as new money (maintainer ruling).
                // Synced, it is "Arriving" at the foot of the card instead.
                if (!synced && balance.pendingIncomingZat > 0)
                  _Line(
                    label: l10n.walletNotSpendableYetLabel,
                    amountZat: balance.pendingIncomingZat,
                    hidden: hidden,
                  ),
                // Our own change in flight (the normal state right after a send).
                // Shown when nonzero so the visible lines reconcile to the total —
                // total = spendable + pendingIncoming + pendingChange + transparent;
                // hiding it would make the breakdown silently not sum to the
                // headline (the inverse of the transparent-funds honesty rule).
                if (balance.pendingChangeZat > 0)
                  _Line(
                    label: l10n.walletPendingChangeLabel,
                    amountZat: balance.pendingChangeZat,
                    hidden: hidden,
                  ),
                // Privacy-relevant state (SDK marks transparentZat load-bearing):
                // unshielded funds are publicly linkable on-chain, so a nonzero
                // value MUST stay visible (e.g. after a failed auto-shield) —
                // never silently folded into the total.
                if (balance.transparentZat > 0) ...[
                  const SizedBox(height: 8),
                  _Line(
                    label: l10n.walletTransparentLabel,
                    amountZat: balance.transparentZat,
                    tint: colors.warningOnDeep,
                    hidden: hidden,
                  ),
                  const SizedBox(height: 2),
                  Text(
                    // The base note frames these as "shield to spend"; for watch-only
                    // that is unfollowable, so the variant keeps ONLY the honest
                    // public-visibility fact (still worth surfacing — it is a privacy
                    // truth about funds a viewer is watching).
                    watchOnly
                        ? l10n.walletTransparentNoteWatchOnly
                        : l10n.walletTransparentNote,
                    style: textTheme.bodySmall?.copyWith(
                      color: colors.warningOnDeep,
                    ),
                  ),
                  // §3.2i-3 (a) failure honesty: automatic shielding did not
                  // complete, so these funds stay public — said in words, next to
                  // the figure it explains, with the manual Shield button right
                  // below as the recovery affordance. Gated inside the
                  // transparent>0 block: no funds, no cue.
                  // #397 D3 (two-reviewer-confirmed): NEVER on a watch-only wallet — the
                  // cue says "you can shield them now" beneath a card with no Shield
                  // button. Belt over the auto-shield loop's own watch-only gate. In
                  // the session-only-host async window BOTH this belt and the loop
                  // read isWatchOnly==false, so this does NOT suppress the window
                  // itself: that window is money-safe on its own —
                  // proposeShield throws WatchOnly before any spend (no keys, nothing
                  // moves) and the failed cue self-heals the instant the bounded async
                  // read resolves true. This belt's real job is that resolved steady
                  // state (a lingering failed status stays hidden).
                  if (autoShieldIncomplete && !watchOnly) ...[
                    const SizedBox(height: 6),
                    Text(
                      l10n.walletAutoShieldIncomplete,
                      key: const ValueKey('wallet-auto-shield-cue'),
                      style: textTheme.bodySmall?.copyWith(
                        color: colors.warningOnDeep,
                      ),
                    ),
                  ],
                  // The recoverable one-time-address subset (2e-2b-iv): part of the
                  // unshielded funds above is sitting on a wallet-controlled single-use
                  // address (an expired TEX forward OR an exchange return). Shown as a
                  // locational note — "X OF your balance is on a one-time address" — so
                  // the user sees WHERE the money is, never "+X" on top of the balance
                  // (it is already counted in the transparent line). Cause-agnostic copy
                  // (never "stranded"/"bounced"). `isFinal` distinguishes recoverable-now
                  // from still-confirming (conservative — relative to the synced tip).
                  // The gap is wider than the privacy note's own (2px) so the two distinct
                  // messages stay visually separate even when each wraps at large scale.
                  if (recoverable.totalZat > 0) ...[
                    const SizedBox(height: 6),
                    Text(
                      // #397 D3: the "(recoverable)" claim promises an action a
                      // watch-only wallet can't take (recovery MINTS a self-send — a
                      // spend; the reclaim affordance is hidden). Keep the locational
                      // fact, drop the recoverability claim. The still-confirming
                      // variant is a neutral status, so it needs no watch-only twin.
                      recoverableNote(recoverableFigure),
                      // Hidden: a screen reader hears the sentence with
                      // "Balance hidden" in the figure's place, not the dots.
                      semanticsLabel: hidden
                          ? recoverableNote(l10n.walletBalanceHiddenAmount)
                          : null,
                      style: textTheme.bodySmall?.copyWith(
                        color: colors.warningOnDeep,
                      ),
                    ),
                  ],
                  const SizedBox(height: 10),
                  // The privacy-positive "move these public funds into the shielded pool"
                  // action (Recv-3). Shown alongside any transparent balance; the sheet
                  // is the authoritative gate (proposeShield ⇒ null below the shieldable
                  // minimum, rendered honestly). The sheet is self-contained (reads the
                  // session provider internally), so this only needs `context`.
                  // #397 §3.7 D3/D5: HIDDEN for a watch-only wallet — shielding SPENDS
                  // (transparent → shielded), which it cannot do. The transparent
                  // balance + its public-funds note above stay (honest information).
                  if (!watchOnly)
                    Align(
                      // DIRECTIONAL (#401 R8b): the Shield action is one of the money
                      // affordances the surrounding copy locates by position, and a hard
                      // `centerLeft` puts it on the trailing edge in ar/he.
                      alignment: AlignmentDirectional.centerStart,
                      child: Semantics(
                        container: true,
                        button: true,
                        label: l10n.walletShieldButton,
                        // ExcludeSemantics (below) drops the button's own tap action, so
                        // the a11y action must live on THIS node or the relabelled button
                        // is inert for screen readers (#389 review, same dead-button bug
                        // as the sync badge).
                        onTap: () => showShieldSheet(context),
                        child: ExcludeSemantics(
                          child: OutlinedButton.icon(
                            key: const ValueKey('wallet-shield-button'),
                            // Colour only: the theme owns the size and shape.
                            style: OutlinedButton.styleFrom(
                              foregroundColor: colors.warningOnDeep,
                              side: BorderSide(color: colors.warningOnDeep),
                            ),
                            onPressed: () => showShieldSheet(context),
                            icon: const WalletIcon(
                              WalletGlyph.shielded,
                              size: 18,
                            ),
                            label: Text(l10n.walletShieldButton),
                          ),
                        ),
                      ),
                    ),
                ],
                // "Arriving" (S12 rev.3): new money not yet confirmed, shown only
                // while synced, OUTSIDE the headline — last, with a `+`, in the
                // card's muted colours (never the accent of a completed receipt:
                // it has not arrived yet).
                if (arrivingZat > 0) ...[
                  const SizedBox(height: 12),
                  _Line(
                    key: const ValueKey('wallet-balance-arriving'),
                    label: l10n.walletArrivingLabel,
                    amountZat: arrivingZat,
                    sign: '+',
                    tint: colors.deepMuted,
                    hidden: hidden,
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// An early wallet state (the cold load, a snapshot that cannot be read)
/// under the tab's header, so the title and the overflow menu's recovery
/// routes stay reachable (S12 diff review MEDIUM).
class _EarlyState extends StatelessWidget {
  const _EarlyState({required this.child});

  final Widget child;

  // Scrollable, like the rebuilding branch: on a short screen at a large text
  // scale the header alone can be taller than the view, and a rigid Column
  // would overflow (fold review MEDIUM). The state stays centred in the space
  // left when there is some.
  @override
  Widget build(BuildContext context) => CustomScrollView(
    slivers: [
      const SliverPadding(
        padding: EdgeInsets.fromLTRB(16, 8, 16, 0),
        sliver: SliverToBoxAdapter(child: _WalletHeader()),
      ),
      SliverFillRemaining(hasScrollBody: false, child: Center(child: child)),
    ],
  );
}

/// The sync bar's slot (C2): its appearance and disappearance animate the
/// space over 250 ms, or — under reduced motion — change it at once with no
/// animation wrapper at all.
class _SyncBarSlot extends StatelessWidget {
  const _SyncBarSlot({required this.animate, required this.child});

  final bool animate;
  final Widget child;

  @override
  Widget build(BuildContext context) => animate
      ? AnimatedSize(
          duration: const Duration(milliseconds: 250),
          alignment: Alignment.topCenter,
          child: child,
        )
      : child;
}

/// One decorative ring at the balance card's corner (C3).
class _Ring extends StatelessWidget {
  const _Ring({
    required this.diameter,
    required this.color,
    required this.opacity,
  });

  final double diameter;
  final Color color;
  final double opacity;

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: ExcludeSemantics(
      child: Container(
        width: diameter,
        height: diameter,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          border: Border.all(
            color: color.withValues(alpha: opacity),
            width: 1.5,
          ),
        ),
      ),
    ),
  );
}

/// A dense balance-breakdown row: the money-row SSOT ([LabeledZatRow]) plus the
/// 2px vertical packing the balance card's tight cluster relies on. Delegating
/// keeps the responsive stack-at-large-scale layout, the never-clip-the-amount
/// rule, and the merged label+amount semantics in ONE place.
class _Line extends StatelessWidget {
  const _Line({
    super.key,
    required this.label,
    required this.amountZat,
    this.tint,
    this.hidden = false,
    this.sign = '',
  });

  final String label;
  final int amountZat;

  /// See [LabeledZatRow.sign].
  final String sign;

  /// Optional emphasis color for both label and value (e.g. the privacy
  /// tint on unshielded funds). Defaults to the card's `deepMuted` label over
  /// an `onDeep` value.
  final Color? tint;

  /// Hide balance: the amount reads as dots.
  final bool hidden;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: LabeledZatRow(
        label: label,
        amountZat: amountZat,
        // The card is the `deep` surface (FR-49 S12, C3).
        tint: tint ?? colors.onDeep,
        labelColor: tint ?? colors.deepMuted,
        hidden: hidden,
        sign: sign,
      ),
    );
  }
}

/// The always-on pool-clarity line (#389): a compact, glanceable summary of how
/// much of the balance is SHIELDED (private) versus TRANSPARENT (publicly
/// visible on-chain), sitting directly under the total. It answers the maintainer's
/// own question — "how much of my ZEC is private?" — on its own axis, distinct
/// from the spendable/pending breakdown rows below.
///
/// Honesty invariant: `shieldedZat == total − transparent`, so the two figures
/// sum EXACTLY to the headline above; the bare (unit-less) numbers can never
/// disagree with the amount they break down. The public pool wears the same
/// privacy-orange the rest of the card uses for transparent funds. Tapping opens
/// the Transparent funds sheet (the shield / auto-shield / expert surface).
///
/// A money figure is NEVER clipped: the whole line scales down inside a
/// [FittedBox] with every digit intact at extreme text scale / long amounts —
/// the same never-clip rule [LabeledZatRow] applies to its amount.
class _PoolLine extends StatelessWidget {
  const _PoolLine({
    required this.shieldedZat,
    required this.transparentZat,
    required this.onTap,
    this.hidden = false,
  });

  final int shieldedZat;
  final int transparentZat;

  /// Hide balance: both figures read as dots.
  final bool hidden;

  /// Opens the transparent-funds sheet. `null` for a watch-only wallet — the
  /// line then renders as pure INFORMATION (no button role, no ripple, no tap
  /// hint), since the sheet it would open is entirely spend-framed (#397 D5).
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    // On the card's `deep` surface (FR-49 S12, C3).
    final baseStyle = textTheme.bodyMedium?.copyWith(color: colors.onDeep);
    final orangeStyle = baseStyle?.copyWith(color: colors.warningOnDeep);
    final allShielded = transparentZat == 0;
    String figure(int zat) => displayedAmount(formatZec(zat), hidden: hidden);

    // Mirror LabeledZatRow's rule: once the OS text scale passes ~1.4×, a
    // low-vision user's chosen size must WIN. A single scale-down FittedBox
    // would shrink the compact two-figure line straight back down and quietly
    // defeat that setting, so past the threshold the segments STACK onto their
    // own full-width lines where each keeps the user's size (only an extreme
    // figure then scales down, every digit intact).
    final stacked = walletTextScaleForcesStack(context);

    final icon = WalletIcon(
      WalletGlyph.shielded,
      size: 16,
      color: colors.deepMuted,
    );

    final Widget visual;
    final String semanticLabel;
    if (allShielded) {
      // Reached only by a watch-only wallet with pending funds (rev.2 R4):
      // the FIGURE accounts for the balance. The old figure-less "All
      // shielded · private" affirmation is gone with C3.
      final text = l10n.walletPoolShielded(figure(shieldedZat));
      // Hidden: the same line with "Balance hidden" in the figure's place.
      semanticLabel = hidden
          ? l10n.walletPoolShielded(l10n.walletBalanceHiddenAmount)
          : text;
      visual = _scaleRow([
        icon,
        const SizedBox(width: 6),
        Text(text, style: baseStyle),
      ]);
    } else {
      final shielded = l10n.walletPoolShielded(figure(shieldedZat));
      final transparent = l10n.walletPoolTransparent(figure(transparentZat));
      // Screen readers hear the plain, comma-joined breakdown — the middot and
      // the per-segment colour carry no meaning read aloud. Hidden, they hear
      // both segments with "Balance hidden" in each figure's place — never the
      // dots — so the fact that part of the balance is transparent survives,
      // as it does for a sighted user.
      final hiddenWord = l10n.walletBalanceHiddenAmount;
      semanticLabel = hidden
          ? '${l10n.walletPoolShielded(hiddenWord)}, '
                '${l10n.walletPoolTransparent(hiddenWord)}'
          : '$shielded, $transparent';
      if (stacked) {
        visual = Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _scaleRow([
              icon,
              const SizedBox(width: 6),
              Text(shielded, style: baseStyle),
            ]),
            const SizedBox(height: 2),
            // The public figure keeps its privacy-orange on its own line.
            _scaleRow([Text(transparent, style: orangeStyle)]),
          ],
        );
      } else {
        // Compact one line: "🛡 Shielded X · Transparent Y", two-tone.
        visual = _scaleRow([
          icon,
          const SizedBox(width: 6),
          Text.rich(
            TextSpan(
              style: baseStyle,
              children: [
                TextSpan(text: shielded),
                TextSpan(
                  text: '  ·  ',
                  style: TextStyle(color: colors.deepMuted),
                ),
                // The public pool wears the SAME privacy-orange the rest of the
                // card uses for transparent funds — one learned colour for
                // "your money that is visible on-chain".
                TextSpan(text: transparent, style: orangeStyle),
              ],
            ),
          ),
        ]);
      }
    }

    return Semantics(
      container: true,
      // Button role ONLY when tappable (a watch-only pool line is pure
      // information — announcing "button" for an inert node is the dead-button
      // a11y bug).
      button: onTap != null,
      // excludeSemantics drops the InkWell's own tap action, so the a11y action
      // must live on THIS node — otherwise the relabelled button is inert for
      // screen readers (the sync-badge lesson).
      excludeSemantics: true,
      onTap: onTap,
      onTapHint: onTap == null ? null : l10n.walletPoolTapHint,
      label: semanticLabel,
      child: Material(
        // A transparency layer so the ink ripple renders ON the card fill (the
        // nearest ancestor Material sits behind it); the tap itself works
        // without this, the ripple is the feedback.
        type: MaterialType.transparency,
        child: InkWell(
          key: const ValueKey('wallet-pool-line'),
          onTap: onTap,
          child: Padding(
            // ~32px tall — a comfortable tap target that keeps the summary
            // tight under the headline.
            padding: const EdgeInsets.symmetric(vertical: 6),
            child: visual,
          ),
        ),
      ),
    );
  }

  /// A left-aligned, never-clip line: the content lays out on one line and scales
  /// down (every digit intact) only if it would exceed the card's bounded width —
  /// the same protection [LabeledZatRow] gives its amount.
  static Widget _scaleRow(List<Widget> children) => Align(
    alignment: AlignmentDirectional.centerStart,
    child: FittedBox(
      fit: BoxFit.scaleDown,
      alignment: AlignmentDirectional.centerStart,
      child: Row(mainAxisSize: MainAxisSize.min, children: children),
    ),
  );
}
