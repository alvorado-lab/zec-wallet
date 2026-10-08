import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../shared/wallet_notice.dart';
import '../../shared/wallet_sheet.dart';
import 'onboarding/onboarding_providers.dart';
import 'onboarding/onboarding_store.dart';
import 'wallet_providers.dart';
import 'wallet_rescan_controller.dart';
import 'wallet_session.dart';

/// #390 — the "Check older swap addresses" DEEP SCAN: the SDK's own in-app
/// recovery of a seed-only restore's older swap deposits/refunds (the #387
/// residual (a)). It WIDENS the range the wallet watches; the normal sync then
/// surfaces any older swap money into the balance over the next minutes — it does
/// NOT itself find funds, and it moves NO money (no authorizer). Its
/// SINGLE-FLIGHT latch ([swapDeepScanInFlightProvider]) both prevents a second
/// concurrent widen and drives the sheet's disabled/in-progress cue.
final swapDeepScanInFlightProvider =
    NotifierProvider<SwapDeepScanController, bool>(SwapDeepScanController.new);

/// The single-flight owner of the deep scan. State = in-flight.
class SwapDeepScanController extends Notifier<bool> {
  @override
  bool build() => false;

  /// Run ONE deep scan. Returns the counts-only report, or `null` when nothing
  /// ran — a scan is already in flight (that run owns the outcome) or the session
  /// is gone. A typed refusal / fault PROPAGATES (the caller maps the copy); the
  /// latch resets and the coverage read re-pulls on EVERY exit so the sheet
  /// settles.
  Future<SwapAddressCheckReport?> check() async {
    if (state) return null; // single-flight: the running scan owns the outcome
    final session = ref.read(walletSessionProvider);
    if (session == null) return null;
    state = true;
    try {
      // Bound the widen FFI call (rel-MED2): a wedged bridge must degrade to a
      // reset latch + an honest "couldn't start", never a permanent "Checking…"
      // that ALSO pins the core aux mutex the widen serializes on. The widen is
      // a durable local aux write — a timeout after a slow commit self-corrects
      // on the coverage re-pull in the finally below. Same package-wide wedge
      // bound every other local FFI read carries.
      final report = await session.checkOlderSwapAddresses().timeout(
        walletFfiWedgeTimeout,
      );
      // A widen executed — arm the C1 progress cue (survives closing the sheet).
      _armProgress(session);
      return report;
    } on TimeoutException {
      // The FFI call exceeded the wedge bound, but the widen is a durable local
      // aux write that MAY have committed (rel-H1). Arm the cue anyway: the
      // banner only shows if the coverage re-pull below confirms a pending band,
      // so a genuinely-failed widen stays hidden — while a slow-but-committed
      // one keeps its "still surfacing" cue instead of a false "nothing changed".
      _armProgress(session);
      rethrow;
    } finally {
      // Guarded: the controller has no provider dependencies (session is
      // `ref.read`), so only a whole-scope teardown can unmount it mid-scan —
      // and an unmounted-ref throw in `finally` would REPLACE the scan's real
      // outcome. Re-pull the coverage read so the sheet's line advances.
      if (ref.mounted) {
        state = false;
        ref.invalidate(swapAddressCoverageReadProvider);
      }
    }
  }

  /// Arm the C1 progress cue — but ONLY for the identity this scan ran against
  /// (NIT-1: the inline outcome is identity-fenced, so the banner arm must match
  /// or a flip's stale scan could arm the NEW wallet's cue). A whole-scope
  /// teardown trips `ref.mounted`; a same-scope session flip trips `identical`.
  void _armProgress(WalletSession session) {
    if (ref.mounted && identical(ref.read(walletSessionProvider), session)) {
      ref.read(swapDeepScanProgressProvider.notifier).markRan();
    }
  }
}

/// The render-only coverage read backing the sheet's "checked your first N swaps"
/// line. `autoDispose` — it lives only while a sheet holds it, and re-pulls after
/// a scan (the controller invalidates it). Fail-soft `null` on no session; a
/// bridge fault surfaces as the FutureProvider's error arm (the sheet renders a
/// neutral fallback, never a scary error).
final swapAddressCoverageReadProvider =
    FutureProvider.autoDispose<SwapAddressCoverage?>((ref) async {
      final session = ref.watch(walletSessionProvider);
      if (session == null) return null;
      // Bound the local read too (rel-MED2): a wedged bridge surfaces the sheet's
      // neutral fallback line, never an indefinite spinner on the coverage cue.
      return session.swapAddressCheckCoverage().timeout(walletFfiWedgeTimeout);
    });

/// The main-surface progress cue's state (C1 / review H2): a widen keeps
/// surfacing older swap money for MINUTES after the sheet closes, so — once the
/// user has run a scan this session — a dismissible "still checking" banner
/// reports it while [SwapAddressCoverage.pending] > 0. Gated on `ran` so it is
/// the CONSEQUENCE of the user's explicit action, never noise on every restored
/// wallet's ordinary first-sync backfill (which the #380 catch-up cue owns).
@immutable
class SwapDeepScanProgress {
  const SwapDeepScanProgress({required this.ran, required this.dismissed});

  /// A widen ran THIS session (and may still be surfacing money).
  final bool ran;

  /// The user dismissed the banner — hidden until the next run re-arms it.
  final bool dismissed;
}

/// Session-scoped owner of the progress cue (the cross-process discoverability
/// is the post-restore note's job, not this transient in-session banner).
final swapDeepScanProgressProvider =
    NotifierProvider<SwapDeepScanProgressController, SwapDeepScanProgress>(
      SwapDeepScanProgressController.new,
    );

class SwapDeepScanProgressController extends Notifier<SwapDeepScanProgress> {
  @override
  SwapDeepScanProgress build() {
    // Reset on an identity change — a new wallet never inherits the prior
    // life's "a scan is surfacing" banner. (A same-identity rescan reopen also
    // resets it; benign — the rescan's own rebuilding cue takes over, and the
    // re-registered widen keeps surfacing underneath.)
    ref.watch(walletSessionProvider);
    return const SwapDeepScanProgress(ran: false, dismissed: false);
  }

  /// A widen executed: (re)show the banner — a fresh run clears a prior dismiss.
  void markRan() =>
      state = const SwapDeepScanProgress(ran: true, dismissed: false);

  /// The user dismissed the banner — hide it until the next run.
  void dismiss() =>
      state = SwapDeepScanProgress(ran: state.ran, dismissed: true);
}

/// The one-time POST-RESTORE note's lifecycle (#390 C2 / review H1), read from
/// the onboarding store where the controller writes it definitively at each
/// activation. Rebuilds on an identity change so a new wallet re-reads its OWN
/// provenance. Fail-soft: no store (a session-only host) or a read fault ⇒
/// `notApplicable` (no note — the note is a package-onboarding affordance).
final deepScanRestoreNoteProvider =
    AsyncNotifierProvider<
      DeepScanRestoreNoteController,
      DeepScanRestoreNoteState
    >(DeepScanRestoreNoteController.new);

class DeepScanRestoreNoteController
    extends AsyncNotifier<DeepScanRestoreNoteState> {
  @override
  Future<DeepScanRestoreNoteState> build() async {
    // Re-read on an identity change. The controller AWAITS the note write
    // BEFORE it flips the session to OnboardingActive, so by the time
    // that flip rebuilds this provider the store already holds the fresh value
    // — no write/flip race. Belt: the note is only RENDERED after the first
    // catch-up (WalletCatchUpNone in wallet_screen), long after that write, so
    // even a watcher that read earlier would still resolve correctly. A
    // silently-lost write (the setter never throws) reads notApplicable ⇒ no
    // note (fail-safe). The catch-up gate only delays RENDERING, not this read.
    ref.watch(walletSessionProvider);
    final store = ref.watch(onboardingStoreProvider);
    if (store == null) return DeepScanRestoreNoteState.notApplicable;
    try {
      return await store.deepScanRestoreNoteState();
    } catch (_) {
      return DeepScanRestoreNoteState.notApplicable;
    }
  }

  /// The user checked or dismissed the note — record `done` so it never returns
  /// (durably, across relaunch). Optimistically hides it first; a failed write
  /// is non-fatal (the store setter never throws).
  ///
  /// No identity fence is needed here: the optimistic `state = done` is
  /// synchronous and the sole `await` is the write itself, so no session flip
  /// can interleave BEFORE the write — a fence in front of it would be dead
  /// code. Across the only flip path (delete → welcome → restore) the note card
  /// is unmounted, so `acknowledge()` is never live on a stale identity; and the
  /// shared (non-namespaced) key's cross-wallet clobber is structurally
  /// unreachable under single-wallet package onboarding. Were an await ever
  /// added before the write, THAT is where a fence would have to go.
  Future<void> acknowledge() async {
    final store = ref.read(onboardingStoreProvider);
    state = const AsyncData(DeepScanRestoreNoteState.done);
    if (store == null) return;
    await store.setDeepScanRestoreNoteState(DeepScanRestoreNoteState.done);
  }
}

/// Open the "Check older swap addresses" sheet — a modal bottom sheet (a
/// deliberate recovery, adjacent to Rescan in the overflow menu). Exits through
/// its own Close (the scan runs in the background regardless once started).
Future<void> showSwapDeepScanSheet(BuildContext context) {
  return showWalletSheet<void>(
    context,
    builder: (_) => const _SwapDeepScanSheet(),
  );
}

/// The inline outcome of the last scan attempt — shown IN the sheet (N2: a
/// snackbar would be occluded by this modal). `isProblem` colours a refusal /
/// failure distinctly from the neutral "checking" confirmation.
typedef _DeepScanOutcome = ({String message, bool isProblem});

class _SwapDeepScanSheet extends ConsumerStatefulWidget {
  const _SwapDeepScanSheet();

  @override
  ConsumerState<_SwapDeepScanSheet> createState() => _SwapDeepScanSheetState();
}

class _SwapDeepScanSheetState extends ConsumerState<_SwapDeepScanSheet> {
  /// The last attempt's outcome, rendered inline (B1 / review N2).
  _DeepScanOutcome? _outcome;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    final inFlight = ref.watch(swapDeepScanInFlightProvider);
    // Mutually exclusive with a rescan (both re-register + re-poll the transparent
    // set): disable while a rescan is actively RUNNING/REBUILDING, with the reason
    // shown. rel-M2: a terminal FAILURE / blocked notice (Failed / NeedsSpace /
    // BlockedBySettling) is dismissible and re-polls NOTHING, so it must NOT
    // block the deep scan — only the actively-working states do.
    final rescanState = ref.watch(walletRescanControllerProvider);
    final rescanBusy =
        rescanState is WalletRescanRunning ||
        rescanState is WalletRescanRebuilding;
    final coverageAsync = ref.watch(swapAddressCoverageReadProvider);
    final coverage = coverageAsync.value;

    // A band from a prior widen is still surfacing (B1 / review M2): a tap now
    // would just earn a CheckOutstanding refusal, so the primary action is
    // DISABLED and reads "Checking…" — never a live "Check older" under a
    // "still checking" line.
    final outstanding = (coverage?.pending ?? 0) > 0;

    // Degraded-transport hint (elective op). Tor was REQUESTED but fell back to
    // clearnet → recommend deferral. When the snapshot has not loaded yet we
    // simply don't KNOW the transport — surface a softer "can't confirm" nudge
    // rather than staying silent (B5 / security MED): silence reads as "all
    // clear", which we can't promise.
    final tor = ref.watch(walletSnapshotProvider).value?.tor;
    final torDegraded = tor is TorState_FellBack || tor is TorState_Unavailable;
    final torUnknown = tor == null;

    // The button verb: "check even older" once a range is fully checked (covered
    // AND nothing pending), else "check older". Suppressed while outstanding.
    final deeper =
        !outstanding &&
        coverage != null &&
        coverage.coveredSwaps > 0 &&
        coverage.pending == 0;
    final actionable = !(inFlight || rescanBusy || outstanding);
    final outcome = _outcome;

    return SafeArea(
      child: Padding(
        padding: EdgeInsets.only(
          left: 24,
          right: 24,
          top: 20,
          bottom: 20 + MediaQuery.viewInsetsOf(context).bottom,
        ),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              WalletSheetHeader(
                title: l10n.walletDeepScanTitle,
                closeKey: const Key('deep-scan-close'),
              ),
              const SizedBox(height: 12),
              Text(l10n.walletDeepScanBody, style: textTheme.bodyMedium),
              const SizedBox(height: 16),
              _CoverageLine(coverageAsync: coverageAsync),
              if (torDegraded || torUnknown) ...[
                const SizedBox(height: 12),
                _HintRow(
                  icon: WalletGlyph.privacyNotice,
                  text: torDegraded
                      ? l10n.walletDeepScanTorHint
                      : l10n.walletDeepScanTorUnknownHint,
                ),
              ],
              if (rescanBusy) ...[
                const SizedBox(height: 12),
                Text(
                  l10n.walletDeepScanRescanBusy,
                  style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
                ),
              ],
              if (outcome != null) ...[
                const SizedBox(height: 12),
                _HintRow(
                  icon: outcome.isProblem
                      ? WalletGlyph.info
                      : WalletGlyph.success,
                  text: outcome.message,
                  color: outcome.isProblem ? colors.orange : colors.green,
                ),
              ],
              const SizedBox(height: 24),
              FilledButton(
                onPressed: actionable ? _runCheck : null,
                child: inFlight
                    ? Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          const SizedBox(
                            height: 18,
                            width: 18,
                            child: CircularProgressIndicator.adaptive(
                              strokeWidth: 2,
                            ),
                          ),
                          const SizedBox(width: 12),
                          // Flexible for the same reason as the rescan sheet's
                          // running label (#407 R10a).
                          Flexible(child: Text(l10n.walletDeepScanChecking)),
                        ],
                      )
                    : Text(
                        outstanding
                            ? l10n.walletDeepScanChecking
                            : deeper
                            ? l10n.walletDeepScanCheckDeeperButton
                            : l10n.walletDeepScanCheckButton,
                      ),
              ),
              const SizedBox(height: 8),
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.walletDeepScanClose),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Run the scan and surface the honest outcome INLINE (N2: a snackbar is
  /// occluded by this modal). A refusal maps to its typed reason; a running
  /// scan (`null`) or a session flip reports nothing.
  Future<void> _runCheck() async {
    final l10n = WalletLocalizations.of(context);
    // The identity this outcome belongs to (rel-MED4, the sweep's fence). A
    // session flip mid-scan must never report THIS scan's outcome over the NEW
    // wallet's surface: the widen correctly landed on the captured (now-dead)
    // identity's aux store, but the message would be a lie on the one on screen.
    final session = ref.read(walletSessionProvider);
    _DeepScanOutcome result;
    try {
      final report = await ref
          .read(swapDeepScanInFlightProvider.notifier)
          .check();
      // Nothing ran: another scan is in flight (it owns the outcome) or the
      // session closed — nothing honest to report here.
      if (report == null) return;
      result = (message: l10n.walletDeepScanRan, isProblem: false);
    } on TimeoutException {
      // rel-H1: the widen MAY have committed past the wedge bound — do NOT claim
      // "nothing changed". A neutral "taking longer" message is honest; the
      // coverage re-pull (controller `finally`) flips the sheet to the
      // outstanding "still checking" state + arms the home banner if it did
      // commit, so nothing is lost.
      result = (message: l10n.walletDeepScanSlow, isProblem: false);
    } on WalletApiError catch (e) {
      result = (message: deepScanRefusalMessage(l10n, e), isProblem: true);
    } catch (_) {
      result = (message: l10n.walletDeepScanFailed, isProblem: true);
    }
    // Fence: bail if the sheet is gone OR the wallet identity changed under us
    // while the scan ran (the outcome belongs to the previous identity).
    if (!mounted || !identical(ref.read(walletSessionProvider), session)) {
      return;
    }
    setState(() => _outcome = result);
  }
}

/// A muted (or coloured) icon + wrapped text row — the sheet's inline hint /
/// outcome shape.
class _HintRow extends StatelessWidget {
  const _HintRow({required this.icon, required this.text, this.color});

  final WalletGlyph icon;
  final String text;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final c = color ?? colors.textMuted;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        WalletIcon(icon, size: 16, color: c),
        const SizedBox(width: 6),
        Expanded(
          child: Text(text, style: textTheme.bodySmall?.copyWith(color: c)),
        ),
      ],
    );
  }
}

/// The coverage line — an EMPHASISED status block (B6): a "still checking" cue
/// while a band is registering, a "checked so far, go deeper" line once a range
/// is fully covered, or a neutral fallback when nothing has been checked yet or
/// the read is loading/errored (never a scary error over a recovery sheet, and
/// never a false swap COUNT — the index is not a swap tally; B2).
class _CoverageLine extends ConsumerWidget {
  const _CoverageLine({required this.coverageAsync});

  final AsyncValue<SwapAddressCoverage?> coverageAsync;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);

    // (text, icon, tone) per state — pending is "working", covered is "done",
    // everything else is the neutral explainer.
    final (String text, WalletGlyph icon, WalletNoticeTone tone) = coverageAsync
        .when(
          data: (c) {
            if (c != null && c.pending > 0) {
              return (
                l10n.walletDeepScanCoveragePending,
                WalletGlyph.syncing,
                WalletNoticeTone.positive,
              );
            }
            if (c != null && c.coveredSwaps > 0) {
              return (
                l10n.walletDeepScanCoverage,
                WalletGlyph.success,
                WalletNoticeTone.positive,
              );
            }
            // c == null, or nothing checked yet (covered == 0) → the neutral
            // explainer, NEVER "your first 0 swaps".
            return (
              l10n.walletDeepScanCoverageUnknown,
              WalletGlyph.search,
              WalletNoticeTone.info,
            );
          },
          loading: () => (
            l10n.walletDeepScanCoverageUnknown,
            WalletGlyph.search,
            WalletNoticeTone.info,
          ),
          error: (_, _) => (
            l10n.walletDeepScanCoverageUnknown,
            WalletGlyph.search,
            WalletNoticeTone.info,
          ),
        );

    return WalletNotice(tone: tone, glyph: icon, message: text);
  }
}

/// Map a typed [WalletApiError] from the deep scan to its honest copy: the two
/// refusal reasons get distinct guidance; anything else is the neutral
/// "couldn't start" (nothing changed).
String deepScanRefusalMessage(WalletLocalizations l10n, WalletApiError e) {
  final kind = e.kind;
  if (kind is WalletErrorKind_SwapAddressCheckRefused) {
    return switch (kind.reason) {
      SwapAddressCheckRefusal.swapDisabled =>
        l10n.walletDeepScanRefusedDisabled,
      SwapAddressCheckRefusal.checkOutstanding =>
        l10n.walletDeepScanRefusedOutstanding,
      SwapAddressCheckRefusal.unknown => l10n.walletDeepScanFailed,
    };
  }
  return l10n.walletDeepScanFailed;
}
