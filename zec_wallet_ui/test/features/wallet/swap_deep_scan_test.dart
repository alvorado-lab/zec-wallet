import 'dart:async';

import 'package:fake_async/fake_async.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart'
    show onboardingControllerProvider;
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart'
    show walletProvisionerProvider, onboardingStoreProvider;
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart'
    show OnboardingActive;
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_store.dart'
    show DeepScanRestoreNoteState, OnboardingStore;
import 'package:zec_wallet_ui/features/wallet/swap_deep_scan.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Resolve l10n from a live element so finders couple to KEYS, not literals.
WalletLocalizations _l10nAt(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(WalletScreen)));

Widget _harness(WalletSession session) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const WalletScreen(),
    ),
  );
}

FakeWalletSession _activeFake() {
  return FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 100),
    snapshotValue: walletStateFixture(
      syncStatus: const SyncStatus.upToDate(tip: 100),
      balance: balanceFixture(
        spendableZat: 1000000,
        totalZat: 1000000,
        transparentZat: 0,
      ),
    ),
  );
}

/// Harness that also wires the onboarding store — for the C2 post-restore note,
/// which reads its provenance/lifecycle from [onboardingStoreProvider].
Widget _harnessWithStore(WalletSession session, OnboardingStore store) {
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      onboardingStoreProvider.overrideWithValue(store),
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const WalletScreen(),
    ),
  );
}

Future<void> _openOverflowMenu(WidgetTester tester) async {
  final l10n = _l10nAt(tester);
  await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
  await tester.pumpAndSettle();
}

WalletApiError _refusal(SwapAddressCheckRefusal reason) => WalletApiError(
  code: 'RW-LIFE-007',
  message: 'swap address check refused',
  kind: WalletErrorKind.swapAddressCheckRefused(reason: reason),
);

/// Coverage with nothing outstanding — makes the sheet's primary button
/// ACTIONABLE (B1 disables it while a band is still surfacing).
const _actionableCoverage = SwapAddressCoverage(coveredSwaps: 0, pending: 0);

/// A session whose deep-scan check is PARKED at a gate — so a test can flip the
/// wallet identity WHILE the scan is in flight (the exact window the A2 fence
/// covers).
class _GatedCheckSession extends FakeWalletSession {
  _GatedCheckSession({required this.gate, super.current, super.snapshotValue}) {
    swapAddressCheckCoverageResult = _actionableCoverage;
  }

  final Completer<void> gate;

  @override
  Future<SwapAddressCheckReport> checkOlderSwapAddresses() async {
    checkOlderSwapAddressesCount++;
    await gate.future;
    return checkOlderSwapAddressesResult;
  }
}

/// Models the widen's before/after: coverage reports nothing pending UNTIL a
/// scan runs, then a band is outstanding — the exact transition that arms the
/// home "still checking" banner and disables the sheet's button.
class _WidenModelSession extends FakeWalletSession {
  _WidenModelSession({super.current, super.snapshotValue}) {
    swapAddressCheckCoverageResult = _actionableCoverage;
  }

  @override
  Future<SwapAddressCheckReport> checkOlderSwapAddresses() async {
    final r = await super.checkOlderSwapAddresses();
    swapAddressCheckCoverageResult = const SwapAddressCoverage(
      coveredSwaps: 1024,
      pending: 1024,
    );
    return r;
  }
}

/// Models rel-H1: the widen COMMITS (coverage flips to pending) but the FFI call
/// HANGS past the wedge timeout — so the check times out over a band that landed.
class _SlowCommitSession extends FakeWalletSession {
  _SlowCommitSession({super.current, super.snapshotValue}) {
    swapAddressCheckCoverageResult = _actionableCoverage;
  }

  @override
  Future<SwapAddressCheckReport> checkOlderSwapAddresses() {
    swapAddressCheckCoverageResult = const SwapAddressCoverage(
      coveredSwaps: 1024,
      pending: 1024,
    );
    return Completer<SwapAddressCheckReport>().future; // hangs past the timeout
  }
}

/// A wallet still CATCHING UP (never-synced, below tip, not UpToDate) — the C2
/// note must stay hidden here (never beside a still-filling balance).
FakeWalletSession _catchingUpFake() {
  const scanning = SyncStatus.scanning(
    from: 1000,
    to: 3000,
    percent: 0.5,
    spendableReady: false,
    rewound: false,
  );
  return FakeWalletSession(
    current: scanning,
    snapshotValue: walletStateFixture(syncStatus: scanning, tip: 3000),
  );
}

/// A rescan controller pinned to a terminal FAILURE — rel-M2: this must NOT
/// block the deep scan (only actively Running/Rebuilding does).
class _StubRescanFailed extends WalletRescanController {
  @override
  WalletRescanState build() => const WalletRescanFailed();
}

void main() {
  group('SwapDeepScanController (single-flight owner)', () {
    test(
      'a second check() while one runs returns null and never re-enters',
      () async {
        final fake = _activeFake()
          ..checkOlderSwapAddressesNeverCompletes = true;
        final container = ProviderContainer(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
        );
        addTearDown(container.dispose);
        final controller = container.read(
          swapDeepScanInFlightProvider.notifier,
        );

        final first = controller.check(); // in flight forever
        await Future<void>.delayed(Duration.zero);
        expect(container.read(swapDeepScanInFlightProvider), isTrue);

        final second = await controller.check();
        expect(second, isNull, reason: 'single-flight: the first run owns it');
        expect(fake.checkOlderSwapAddressesCount, 1);
        expect(first, isA<Future<SwapAddressCheckReport?>>());

        // `first` is wedged on purpose and this test is done with it — but it
        // is NOT inert. `check()` bounds the FFI call with `walletFfiWedgeTimeout`
        // (15 s), so 15 s from now that future REJECTS, and an unhandled
        // rejection after the test has completed is charged to the test:
        //
        //   swap_deep_scan_test.dart 182:34
        //   TimeoutException after 0:00:15.000000: Future not completed
        //   This test failed after it had already completed.
        //
        // It only lands when this file's own process is still alive 15 s in, so
        // it is invisible on a dev box (the file runs in ~2 s) and it took the
        // launchd nightly `-1` twice — 2026-09-06 04:35 and 06:30, both runs
        // otherwise green (P0-4, docs/plan/production-readiness-phase-0.md §4i).
        // `ignore()` marks the rejection handled; the assertion above has
        // already used the future, so nothing else is given up.
        //
        // The sibling shape does NOT need this and it was checked rather than
        // assumed: `ephemeral_sweep_test.dart` leaves the same kind of future
        // dangling, but `EphemeralSweepController.sweep()` carries no
        // `.timeout()`, so that future never rejects and there is nothing to
        // handle.
        first.ignore();
      },
    );

    test('no session ⇒ null, latch untouched', () async {
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(null)],
      );
      addTearDown(container.dispose);
      final controller = container.read(swapDeepScanInFlightProvider.notifier);
      expect(await controller.check(), isNull);
      expect(container.read(swapDeepScanInFlightProvider), isFalse);
    });

    test('a check fault resets the latch and propagates', () async {
      final fake = _activeFake()
        ..checkOlderSwapAddressesThrows = _refusal(
          SwapAddressCheckRefusal.checkOutstanding,
        );
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      final controller = container.read(swapDeepScanInFlightProvider.notifier);
      await expectLater(controller.check(), throwsA(isA<WalletApiError>()));
      expect(
        container.read(swapDeepScanInFlightProvider),
        isFalse,
        reason: 'the latch must never stick shut after a fault',
      );
    });
  });

  group('the deep-scan overflow entry + sheet', () {
    testWidgets(
      'the entry is session-visible (no package custody) and opens the sheet',
      (tester) async {
        // No walletProvisionerProvider override ⇒ session-only host ⇒ Rescan
        // hides, but the deep-scan entry (not custody-gated) stays reachable.
        final fake = _activeFake();
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        expect(find.text(l10n.walletRescanMenuItem), findsNothing);
        await _openOverflowMenu(tester);
        expect(find.text(l10n.walletDeepScanMenuItem), findsOneWidget);

        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletDeepScanTitle), findsOneWidget);
        // Default fake coverage is a mid-check restored wallet (pending > 0).
        expect(find.text(l10n.walletDeepScanCoveragePending), findsOneWidget);
      },
    );

    testWidgets(
      'a check runs once and reports the honest outcome INLINE (N2) — still in '
      'the open sheet, never an occluded snackbar',
      (tester) async {
        final fake = _activeFake()
          ..swapAddressCheckCoverageResult = _actionableCoverage;
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckButton));
        await tester.pumpAndSettle();

        expect(fake.checkOlderSwapAddressesCount, 1);
        // The outcome renders INLINE while the sheet is STILL open (title
        // present) — proving it's not a snackbar occluded behind the modal.
        expect(find.text(l10n.walletDeepScanTitle), findsOneWidget);
        expect(find.text(l10n.walletDeepScanRan), findsOneWidget);
      },
    );

    testWidgets(
      'a checkOutstanding refusal shows the "still checking the last range" copy',
      (tester) async {
        final fake = _activeFake()
          ..swapAddressCheckCoverageResult = _actionableCoverage
          ..checkOlderSwapAddressesThrows = _refusal(
            SwapAddressCheckRefusal.checkOutstanding,
          );
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckButton));
        await tester.pumpAndSettle();

        // Inline in the still-open sheet (N2), not an occluded snackbar.
        expect(find.text(l10n.walletDeepScanTitle), findsOneWidget);
        expect(
          find.text(l10n.walletDeepScanRefusedOutstanding),
          findsOneWidget,
        );
      },
    );

    testWidgets('a swapDisabled refusal shows the "swap is off" copy', (
      tester,
    ) async {
      final fake = _activeFake()
        ..swapAddressCheckCoverageResult = _actionableCoverage
        ..checkOlderSwapAddressesThrows = _refusal(
          SwapAddressCheckRefusal.swapDisabled,
        );
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await _openOverflowMenu(tester);
      await tester.tap(find.text(l10n.walletDeepScanMenuItem));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletDeepScanCheckButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletDeepScanRefusedDisabled), findsOneWidget);
    });

    testWidgets(
      'while a scan is in flight the sheet button shows "Checking…", is disabled, '
      'and a second tap is inert (single-flight)',
      (tester) async {
        final fake = _activeFake()
          ..swapAddressCheckCoverageResult = _actionableCoverage
          ..checkOlderSwapAddressesNeverCompletes = true;
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckButton));
        await tester.pump(); // the scan never completes — no settle
        expect(fake.checkOlderSwapAddressesCount, 1);

        // The sheet button reflects the in-flight latch: spinner + label + disabled.
        expect(find.text(l10n.walletDeepScanChecking), findsOneWidget);
        final button = tester.widget<FilledButton>(
          find.ancestor(
            of: find.text(l10n.walletDeepScanChecking),
            matching: find.byType(FilledButton),
          ),
        );
        expect(
          button.onPressed,
          isNull,
          reason: 'disabled while its own scan runs',
        );

        // A second tap is inert (the disabled button + the controller latch).
        await tester.tap(find.byType(FilledButton), warnIfMissed: false);
        await tester.pump(const Duration(milliseconds: 300));
        expect(
          fake.checkOlderSwapAddressesCount,
          1,
          reason: 'never a second scan',
        );

        // A1 now wraps the scan in a wedge-timeout; drain the timer it
        // scheduled (and the resulting "couldn't start" snackbar) so the test
        // leaves the fake clock clean.
        await tester.pumpAndSettle(const Duration(seconds: 20));
      },
    );
  });

  // ── Batch A: reliability-safety (money-adjacent, re-reviewed) ──────────────
  group('Batch A — reliability safety', () {
    test('A1: a wedged check times out at the FFI bound, resets the latch, and '
        'surfaces a TimeoutException — never a permanent "Checking…"', () {
      fakeAsync((async) {
        final fake = _activeFake()
          ..checkOlderSwapAddressesNeverCompletes = true;
        final container = ProviderContainer(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
        );
        addTearDown(container.dispose);
        final controller = container.read(
          swapDeepScanInFlightProvider.notifier,
        );

        Object? caught;
        controller.check().then(
          (_) {},
          onError: (Object e) {
            caught = e;
          },
        );
        async.flushMicrotasks();
        expect(
          container.read(swapDeepScanInFlightProvider),
          isTrue,
          reason: 'in flight before the bound',
        );

        // Past the wedge bound: the timeout fires.
        async.elapse(walletFfiWedgeTimeout + const Duration(seconds: 1));
        async.flushMicrotasks();

        expect(caught, isA<TimeoutException>());
        expect(
          container.read(swapDeepScanInFlightProvider),
          isFalse,
          reason: 'the wedge bound must RESET the latch, never stick shut',
        );
      });
    });

    testWidgets(
      'A2: a session flip mid-scan fences the outcome — no snackbar reports the '
      'previous identity\'s scan over the new wallet (S153)',
      (tester) async {
        final gate = Completer<void>();
        final sessionA = _GatedCheckSession(
          gate: gate,
          current: const SyncStatus.upToDate(tip: 100),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 100),
            balance: balanceFixture(
              spendableZat: 1000000,
              totalZat: 1000000,
              transparentZat: 0,
            ),
          ),
        );
        final sessionB = _activeFake();
        final sessionSwitch = StateProvider<WalletSession?>((ref) => sessionA);

        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              walletSessionProvider.overrideWith(
                (ref) => ref.watch(sessionSwitch),
              ),
            ],
            child: MaterialApp(
              localizationsDelegates:
                  WalletLocalizations.localizationsDelegates,
              supportedLocales: WalletLocalizations.supportedLocales,
              theme: lightTheme,
              home: const WalletScreen(),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckButton));
        await tester.pump(); // the scan is now in flight, parked at the gate
        expect(sessionA.checkOlderSwapAddressesCount, 1);

        // Flip the wallet identity WHILE the scan runs (the screen stays
        // mounted — only the fence's `identical` check can catch this).
        final container = ProviderScope.containerOf(
          tester.element(find.byType(WalletScreen)),
        );
        container.read(sessionSwitch.notifier).state = sessionB;
        await tester.pump();

        // Release: the now-previous identity's scan completes.
        gate.complete();
        await tester.pumpAndSettle();

        expect(
          find.text(l10n.walletDeepScanRan),
          findsNothing,
          reason:
              'the previous identity\'s scan must not report over the new '
              'wallet — the S153 fence bails silently',
        );
        // the SAME fence must also refuse to arm the NEW wallet's home
        // "still checking" banner. sessionB has pending > 0, so a fence-less
        // `_armProgress(sessionA)` would markRan for B and light the banner —
        // the `identical(session)` check keeps `ran` false for B.
        expect(
          find.text(l10n.walletDeepScanBannerChecking),
          findsNothing,
          reason:
              'the stale scan must not arm the new wallet\'s progress banner '
              '(the _armProgress identity fence, not just the inline outcome)',
        );
      },
    );

    testWidgets(
      'A3: the rescan menu entry is DISABLED while a deep scan\'s widen is in '
      'flight — the reciprocal mutual-exclusion (rel-MED3)',
      (tester) async {
        final gate = Completer<void>();
        final fake = _GatedCheckSession(
          gate: gate,
          current: const SyncStatus.upToDate(tip: 100),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 100),
            balance: balanceFixture(
              spendableZat: 1000000,
              totalZat: 1000000,
              transparentZat: 0,
            ),
          ),
        );
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              walletSessionProvider.overrideWithValue(fake),
              // Rescan renders only under PACKAGE custody (a wired provisioner).
              walletProvisionerProvider.overrideWithValue(
                FakeWalletProvisioner(),
              ),
            ],
            child: MaterialApp(
              localizationsDelegates:
                  WalletLocalizations.localizationsDelegates,
              supportedLocales: WalletLocalizations.supportedLocales,
              theme: lightTheme,
              home: const WalletScreen(),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        // Hold a deep-scan widen in flight (parked at the gate — the ~1s window).
        final container = ProviderScope.containerOf(
          tester.element(find.byType(WalletScreen)),
        );
        unawaited(
          container.read(swapDeepScanInFlightProvider.notifier).check(),
        );
        await tester.pump();
        expect(container.read(swapDeepScanInFlightProvider), isTrue);

        await _openOverflowMenu(tester);
        // The rescan entry still RENDERS (package custody) but is disabled.
        final rescanItem =
            tester.widget(
                  find.ancestor(
                    of: find.text(l10n.walletRescanMenuItem),
                    matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
                  ),
                )
                as PopupMenuItem;
        expect(
          rescanItem.enabled,
          isFalse,
          reason:
              'rescan is mutually exclusive with a running deep-scan widen '
              '(the reciprocal of the deep-scan entry\'s !rescanBusy guard)',
        );

        // Release the scan so the test leaves no in-flight future.
        gate.complete();
        await tester.pumpAndSettle();
      },
    );
  });

  // ── C1: the progress cue that survives closing the sheet (review H2) ───────
  group('C1 — deep-scan progress banner', () {
    test(
      'markRan arms the banner, dismiss hides it, an identity change resets both',
      () async {
        final sessionSwitch = StateProvider<WalletSession?>(
          (ref) => _activeFake(),
        );
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWith(
              (ref) => ref.watch(sessionSwitch),
            ),
          ],
        );
        addTearDown(container.dispose);
        container.listen(swapDeepScanProgressProvider, (_, _) {});

        expect(container.read(swapDeepScanProgressProvider).ran, isFalse);

        container.read(swapDeepScanProgressProvider.notifier).markRan();
        expect(container.read(swapDeepScanProgressProvider).ran, isTrue);
        expect(container.read(swapDeepScanProgressProvider).dismissed, isFalse);

        container.read(swapDeepScanProgressProvider.notifier).dismiss();
        expect(container.read(swapDeepScanProgressProvider).dismissed, isTrue);
        expect(
          container.read(swapDeepScanProgressProvider).ran,
          isTrue,
          reason: 'dismiss hides the banner but does not un-run the scan',
        );

        // An identity change resets — a new wallet never inherits the cue.
        container.read(sessionSwitch.notifier).state = _activeFake();
        await pumpEventQueue();
        expect(
          container.read(swapDeepScanProgressProvider).ran,
          isFalse,
          reason: 'a new identity starts with a clean progress cue',
        );
      },
    );

    testWidgets(
      'after a scan the home shows the "still checking" banner (survives '
      'closing the sheet); Dismiss hides it',
      (tester) async {
        // Models the real transition: actionable (pending 0) before the scan,
        // a band outstanding (pending > 0) after the widen.
        final fake = _WidenModelSession(
          current: const SyncStatus.upToDate(tip: 100),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 100),
            balance: balanceFixture(
              spendableZat: 1000000,
              totalZat: 1000000,
              transparentZat: 0,
            ),
          ),
        );
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        // No banner before any scan (ran == false).
        expect(find.text(l10n.walletDeepScanBannerChecking), findsNothing);

        // Run a scan via the sheet, then close it.
        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckButton));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanClose));
        await tester.pumpAndSettle();

        // The home banner is now the progress cue.
        expect(find.text(l10n.walletDeepScanBannerChecking), findsOneWidget);

        // Dismiss → gone (a session-scoped flag; the widen keeps working).
        await tester.tap(find.byTooltip(l10n.walletDeepScanRestoreNoteDismiss));
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletDeepScanBannerChecking), findsNothing);

        // Drain the post-scan snackbar timer.
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );

    testWidgets(
      'a fully-checked wallet (pending == 0) shows NO banner even after a scan '
      '— the cue self-clears when the widen finishes surfacing',
      (tester) async {
        final fake = _activeFake()
          ..swapAddressCheckCoverageResult = const SwapAddressCoverage(
            coveredSwaps: 128,
            pending: 0,
          );
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckDeeperButton));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanClose));
        await tester.pumpAndSettle();

        expect(
          find.text(l10n.walletDeepScanBannerChecking),
          findsNothing,
          reason: 'pending == 0 → nothing is surfacing → no banner',
        );
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );
  });

  // ── Batch B: interaction + honesty ─────────────────────────────────────────
  group('Batch B — interaction + honesty', () {
    testWidgets(
      'B1/M2: a band still surfacing (pending > 0) DISABLES the primary button '
      'and reads "Checking…" — never a live "Check older" that only earns a refusal',
      (tester) async {
        final fake = _activeFake(); // default coverage: pending 63 > 0
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();

        // No actionable label; the disabled button reads "Checking…".
        expect(find.text(l10n.walletDeepScanCheckButton), findsNothing);
        expect(find.text(l10n.walletDeepScanCheckDeeperButton), findsNothing);
        final button = tester.widget<FilledButton>(
          find.ancestor(
            of: find.text(l10n.walletDeepScanChecking),
            matching: find.byType(FilledButton),
          ),
        );
        expect(
          button.onPressed,
          isNull,
          reason: 'a re-tap would only earn a CheckOutstanding refusal',
        );

        // A tap is inert — no scan.
        await tester.tap(find.byType(FilledButton), warnIfMissed: false);
        await tester.pump();
        expect(fake.checkOlderSwapAddressesCount, 0);
      },
    );

    testWidgets('B5: a known-off transport shows NO Tor hint (no over-nudge)', (
      tester,
    ) async {
      final fake = _activeFake()
        ..swapAddressCheckCoverageResult = _actionableCoverage;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await _openOverflowMenu(tester);
      await tester.tap(find.text(l10n.walletDeepScanMenuItem));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletDeepScanTorHint), findsNothing);
      expect(find.text(l10n.walletDeepScanTorUnknownHint), findsNothing);
    });

    testWidgets('B5: a CONFIRMED Tor fallback shows the strong deferral hint', (
      tester,
    ) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
          tor: const TorState.fellBack(),
        ),
      )..swapAddressCheckCoverageResult = _actionableCoverage;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await _openOverflowMenu(tester);
      await tester.tap(find.text(l10n.walletDeepScanMenuItem));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletDeepScanTorHint), findsOneWidget);
    });
  });

  // ── C2: the one-time post-restore note (review H1) ─────────────────────────
  group('C2 — post-restore note', () {
    testWidgets(
      'shows on a RESTORED (pending), caught-up wallet; Dismiss records done and '
      'hides it',
      (tester) async {
        final store = FakeOnboardingStore()
          ..deepScanNote = DeepScanRestoreNoteState.pending;
        await tester.pumpWidget(_harnessWithStore(_activeFake(), store));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        expect(find.text(l10n.walletDeepScanRestoreNoteTitle), findsOneWidget);

        await tester.tap(
          find.widgetWithText(
            TextButton,
            l10n.walletDeepScanRestoreNoteDismiss,
          ),
        );
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletDeepScanRestoreNoteTitle), findsNothing);
        expect(
          store.deepScanNote,
          DeepScanRestoreNoteState.done,
          reason: 'dismiss records done durably so it never returns',
        );
      },
    );

    testWidgets('Check now records done and opens the deep-scan sheet', (
      tester,
    ) async {
      final store = FakeOnboardingStore()
        ..deepScanNote = DeepScanRestoreNoteState.pending;
      await tester.pumpWidget(_harnessWithStore(_activeFake(), store));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await tester.tap(
        find.widgetWithText(FilledButton, l10n.walletDeepScanRestoreNoteCheck),
      );
      await tester.pumpAndSettle();

      expect(store.deepScanNote, DeepScanRestoreNoteState.done);
      expect(
        find.text(l10n.walletDeepScanTitle),
        findsOneWidget,
        reason: 'Check now opens the deep-scan sheet',
      );
    });

    testWidgets('a CREATED wallet (notApplicable) shows NO note', (
      tester,
    ) async {
      final store = FakeOnboardingStore()
        ..deepScanNote = DeepScanRestoreNoteState.notApplicable;
      await tester.pumpWidget(_harnessWithStore(_activeFake(), store));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletDeepScanRestoreNoteTitle), findsNothing);
    });

    test(
      'provenance: create-activation records notApplicable, restore-activation '
      'records pending',
      () async {
        final store = FakeOnboardingStore(confirmed: true);
        final p = FakeWalletProvisioner(exists: false);
        final container = ProviderContainer(
          overrides: [
            walletProvisionerProvider.overrideWithValue(p),
            onboardingStoreProvider.overrideWithValue(store),
          ],
        );
        addTearDown(container.dispose);
        container.listen(onboardingControllerProvider, (_, _) {});
        await pumpEventQueue();
        final c = container.read(onboardingControllerProvider.notifier);

        // Create → confirm: a created wallet gets notApplicable (no note).
        await c.startCreate();
        await c.confirmBackup();
        await pumpEventQueue();
        expect(store.deepScanNote, DeepScanRestoreNoteState.notApplicable);

        // Delete → restore: a restored wallet arms the note (pending).
        await c.deleteWallet();
        c.beginRestore();
        await c.startRestore(p.recoveryWords);
        await pumpEventQueue();
        expect(store.deepScanNote, DeepScanRestoreNoteState.pending);
      },
    );

    test(
      'MED-1: deleteWallet CLEARS the note key so a next wallet cannot inherit '
      'a prior restore\'s pending',
      () async {
        final store = FakeOnboardingStore(confirmed: true)
          ..deepScanNote = DeepScanRestoreNoteState.pending;
        final p = FakeWalletProvisioner(exists: true);
        final container = ProviderContainer(
          overrides: [
            walletProvisionerProvider.overrideWithValue(p),
            onboardingStoreProvider.overrideWithValue(store),
          ],
        );
        addTearDown(container.dispose);
        container.listen(onboardingControllerProvider, (_, _) {});
        await pumpEventQueue();

        await container
            .read(onboardingControllerProvider.notifier)
            .deleteWallet();
        await pumpEventQueue();
        expect(
          store.deepScanNote,
          DeepScanRestoreNoteState.notApplicable,
          reason: 'the shred clears the note residue',
        );
      },
    );

    test(
      'S204: confirmBackup AWAITS the note write BEFORE flipping to Active '
      '(write-before-flip — the note provider never races a stale read)',
      () async {
        final store = FakeOnboardingStore(confirmed: true)
          ..holdNoteSet = Completer<void>();
        final p = FakeWalletProvisioner(exists: false);
        final container = ProviderContainer(
          overrides: [
            walletProvisionerProvider.overrideWithValue(p),
            onboardingStoreProvider.overrideWithValue(store),
          ],
        );
        addTearDown(container.dispose);
        container.listen(onboardingControllerProvider, (_, _) {});
        await pumpEventQueue();
        final c = container.read(onboardingControllerProvider.notifier);

        await c.startCreate();
        final confirm = c.confirmBackup(); // parks at the gated note write
        await pumpEventQueue();

        // The note write is still gated ⇒ the session must NOT have flipped.
        // The OLD flip-then-write ordering would already read OnboardingActive.
        expect(
          container.read(onboardingControllerProvider),
          isNot(isA<OnboardingActive>()),
          reason: 'the note write is AWAITED before the Active flip (S204)',
        );
        expect(store.noteSetCount, 1, reason: 'the note write was dispatched');

        store.holdNoteSet!.complete();
        await confirm;
        await pumpEventQueue();
        expect(
          container.read(onboardingControllerProvider),
          isA<OnboardingActive>(),
        );
        expect(store.deepScanNote, DeepScanRestoreNoteState.notApplicable);
      },
    );

    test('S204: startRestore AWAITS the pending note write BEFORE flipping to '
        'Active (the same write-before-flip ordering, restore arm)', () async {
      final store = FakeOnboardingStore(confirmed: true)
        ..holdNoteSet = Completer<void>();
      final p = FakeWalletProvisioner(exists: false);
      final container = ProviderContainer(
        overrides: [
          walletProvisionerProvider.overrideWithValue(p),
          onboardingStoreProvider.overrideWithValue(store),
        ],
      );
      addTearDown(container.dispose);
      container.listen(onboardingControllerProvider, (_, _) {});
      await pumpEventQueue();
      final c = container.read(onboardingControllerProvider.notifier);

      c.beginRestore();
      final restore = c.startRestore(
        p.recoveryWords,
      ); // parks at the gated note write
      await pumpEventQueue();

      expect(
        container.read(onboardingControllerProvider),
        isNot(isA<OnboardingActive>()),
        reason: 'the pending note write is AWAITED before the Active flip',
      );

      store.holdNoteSet!.complete();
      await restore;
      await pumpEventQueue();
      expect(
        container.read(onboardingControllerProvider),
        isA<OnboardingActive>(),
      );
      expect(store.deepScanNote, DeepScanRestoreNoteState.pending);
    });
  });

  // ── Review fold: the security + reliability findings ─────────────────
  group('review fold', () {
    testWidgets(
      'rel-H1: a check that TIMES OUT over a committed widen shows the honest '
      '"taking longer" copy (never "nothing changed") and arms the home banner',
      (tester) async {
        final fake = _SlowCommitSession(
          current: const SyncStatus.upToDate(tip: 100),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 100),
            balance: balanceFixture(
              spendableZat: 1000000,
              totalZat: 1000000,
              transparentZat: 0,
            ),
          ),
        );
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckButton));
        await tester.pump(); // dispatched; the FFI call hangs
        // Advance past the FFI wedge bound → TimeoutException.
        await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
        await tester.pumpAndSettle();

        expect(find.text(l10n.walletDeepScanSlow), findsOneWidget);
        expect(
          find.text(l10n.walletDeepScanFailed),
          findsNothing,
          reason: 'the widen may have committed — never "nothing changed"',
        );

        // The banner armed on timeout (a committed widen keeps its cue).
        await tester.tap(find.text(l10n.walletDeepScanClose));
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletDeepScanBannerChecking), findsOneWidget);
      },
    );

    testWidgets(
      'rel-M2: a rescan FAILURE notice does NOT block the deep scan (only an '
      'actively running/rebuilding rescan does)',
      (tester) async {
        final fake = _activeFake()
          ..swapAddressCheckCoverageResult = _actionableCoverage;
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              walletSessionProvider.overrideWithValue(fake),
              walletProvisionerProvider.overrideWithValue(
                FakeWalletProvisioner(),
              ),
              walletRescanControllerProvider.overrideWith(
                _StubRescanFailed.new,
              ),
            ],
            child: MaterialApp(
              localizationsDelegates:
                  WalletLocalizations.localizationsDelegates,
              supportedLocales: WalletLocalizations.supportedLocales,
              theme: lightTheme,
              home: const WalletScreen(),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        // The menu entry is ENABLED despite the rescan failure.
        await _openOverflowMenu(tester);
        final entry =
            tester.widget(
                  find.ancestor(
                    of: find.text(l10n.walletDeepScanMenuItem),
                    matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
                  ),
                )
                as PopupMenuItem;
        expect(entry.enabled, isTrue);

        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        // The sheet's button is actionable + shows no rescan-busy note.
        expect(find.text(l10n.walletDeepScanRescanBusy), findsNothing);
        final button = tester.widget<FilledButton>(
          find.ancestor(
            of: find.text(l10n.walletDeepScanCheckButton),
            matching: find.byType(FilledButton),
          ),
        );
        expect(button.onPressed, isNotNull);
      },
    );

    testWidgets(
      'C2: the note stays HIDDEN while the wallet is still catching up (never '
      'beside a still-filling balance)',
      (tester) async {
        final store = FakeOnboardingStore()
          ..deepScanNote = DeepScanRestoreNoteState.pending;
        await tester.pumpWidget(_harnessWithStore(_catchingUpFake(), store));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);
        expect(find.text(l10n.walletDeepScanRestoreNoteTitle), findsNothing);
      },
    );

    testWidgets(
      'C1: an already-shown banner SELF-CLEARS when a sync edge re-pulls '
      'coverage to pending == 0',
      (tester) async {
        final fake = _WidenModelSession(
          current: const SyncStatus.upToDate(tip: 100),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 100),
            balance: balanceFixture(
              spendableZat: 1000000,
              totalZat: 1000000,
              transparentZat: 0,
            ),
          ),
        );
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        // Run a scan → the banner arms (coverage flips to pending 1024).
        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletDeepScanMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanCheckButton));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletDeepScanClose));
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletDeepScanBannerChecking), findsOneWidget);

        // The backfill finished surfacing: coverage now reports pending 0, and a
        // sync edge re-pulls it — the banner self-clears.
        fake.swapAddressCheckCoverageResult = const SwapAddressCoverage(
          coveredSwaps: 1024,
          pending: 0,
        );
        final container = ProviderScope.containerOf(
          tester.element(find.byType(WalletScreen)),
        );
        container.invalidate(swapAddressCoverageReadProvider);
        await tester.pumpAndSettle();
        expect(
          find.text(l10n.walletDeepScanBannerChecking),
          findsNothing,
          reason: 'pending → 0 clears the still-checking banner',
        );
      },
    );

    testWidgets('S204/F2: a transient coverage-read ERROR does NOT blank the '
        '"still checking" banner — .value retains the last pending '
        '(copyWithPrevious), matching the snapshot-retention idiom', (
      tester,
    ) async {
      final fake = _WidenModelSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
          balance: balanceFixture(
            spendableZat: 1000000,
            totalZat: 1000000,
            transparentZat: 0,
          ),
        ),
      );
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      // Arm the banner (the widen flips coverage to pending 1024).
      await _openOverflowMenu(tester);
      await tester.tap(find.text(l10n.walletDeepScanMenuItem));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletDeepScanCheckButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletDeepScanClose));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletDeepScanBannerChecking), findsOneWidget);

      // A transient busy-DB fault on the NEXT coverage re-pull (a sync edge
      // invalidates it). Riverpod retains the prior value through the error
      // (copyWithPrevious), so `.value?.pending` stays 1024 > 0 and the honest
      // cue must NOT vanish — this is the retention wallet_screen relies on.
      fake.swapAddressCheckCoverageThrows = _refusal(
        SwapAddressCheckRefusal.checkOutstanding,
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(WalletScreen)),
      );
      container.invalidate(swapAddressCoverageReadProvider);
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletDeepScanBannerChecking),
        findsOneWidget,
        reason:
            'a transient coverage error retains the last pending — the '
            'banner must not flicker off (F2 investigated, found safe)',
      );
    });
  });
}
