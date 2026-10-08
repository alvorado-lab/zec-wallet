import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart' show RenderParagraph;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/rescan_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_presentation.dart'
    show compactBlockCount;
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The rescan-recovery confirm sheet (FR-1b, re-cut #317). Tests the range
/// selection + the [RescanTarget] threaded into the controller — the
/// money-recovery contract a user drives (the wallet's own start by DEFAULT,
/// vs a picked date, vs scan-all). The actual rescan is a spy: this pins the
/// SHEET, not the session swap (controller-tested elsewhere).
class _SpyRescan extends WalletRescanController {
  _SpyRescan({this.gate});

  bool called = false;
  RescanTarget? lastTarget;

  /// When set, [rescan] blocks on this — so a test can observe the submitting
  /// (spinner + disabled controls) window before the sheet pops.
  final Completer<void>? gate;

  @override
  WalletRescanState build() => const WalletRescanIdle();

  @override
  Future<void> rescan(RescanTarget target) async {
    called = true;
    lastTarget = target;
    if (gate != null) await gate!.future;
  }
}

void main() {
  // The DEFAULT is the wallet's own floor for EVERY readable-floor wallet
  // (rescan is destructive lower-only, so a start above the floor hides
  // funds — the floor is the highest safe start). These fixtures pin BOTH a
  // young floor (near the tip) and an old floor (far below) to prove neither
  // ever defaults above its own floor.
  const fixtureTip = 8100000;
  const fixtureFloor = 8000000; // young: floor near the tip
  const oldWalletFloor = 1000000; // old: floor far below the tip

  /// A host with a button that opens the sheet, wired with the spy controller
  /// + the session/provisioner seams the sheet reads (#317: the floor default
  /// via `birthdayHeight`, the size cue via the estimator + snapshot tip).
  Widget host(
    _SpyRescan spy, {
    required FakeWalletSession session,
    FakeWalletProvisioner? provisioner,
  }) {
    return ProviderScope(
      overrides: [
        walletRescanControllerProvider.overrideWith(() => spy),
        walletSessionProvider.overrideWithValue(session),
        walletProvisionerProvider.overrideWithValue(
          provisioner ?? FakeWalletProvisioner(exists: true),
        ),
      ],
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                onPressed: () => showWalletRescanSheet(context),
                child: const Text('open'),
              ),
            ),
          ),
        ),
      ),
    );
  }

  FakeWalletSession sessionWithFloor(int? floor) => FakeWalletSession(
    current: const SyncStatus.upToDate(tip: fixtureTip),
    snapshotValue: walletStateFixture(
      syncStatus: const SyncStatus.upToDate(tip: fixtureTip),
      tip: fixtureTip,
    ),
  )..birthdayHeightResult = floor;

  FakeWalletSession sessionWithTip() => sessionWithFloor(fixtureFloor);

  Future<WalletLocalizations> openSheet(
    WidgetTester tester,
    _SpyRescan spy, {
    FakeWalletSession? session,
  }) async {
    await tester.pumpWidget(host(spy, session: session ?? sessionWithTip()));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle(); // includes the async floor read
    return WalletLocalizations.of(tester.element(find.byType(ElevatedButton)));
  }

  /// Scroll the sheet to Confirm, then tap it — the way a user would. The
  /// sheet scrolls (SingleChildScrollView), and at the refreshed type scale
  /// (FR-49 S10) Confirm sits at the fold of the 800×600 test surface.
  Future<void> tapConfirm(WidgetTester tester, WalletLocalizations l10n) async {
    final confirm = find.text(l10n.walletRescanConfirm);
    await tester.ensureVisible(confirm);
    await tester.pumpAndSettle();
    await tester.tap(confirm);
  }

  testWidgets('renders the title, body, range control, warning, and actions', (
    tester,
  ) async {
    final l10n = await openSheet(tester, _SpyRescan());

    expect(find.text(l10n.walletRescanTitle), findsOneWidget);
    expect(find.text(l10n.walletRescanBody), findsOneWidget);
    expect(find.text(l10n.walletRescanRangeTitle), findsOneWidget);
    expect(find.text(l10n.walletRescanWarning), findsOneWidget);
    expect(find.text(l10n.walletRescanConfirm), findsOneWidget);
    expect(find.text(l10n.walletRescanCancel), findsOneWidget);
  });

  testWidgets(
    "#317: DEFAULTS to the wallet's own start — the floor description, the "
    'size cue from tip − floor, and Pick + Scan-all affordances',
    (tester) async {
      final l10n = await openSheet(tester, _SpyRescan());

      expect(find.text(l10n.walletRescanRangeDefault), findsOneWidget);
      // The size cue: (tip − floor) compact — the honest magnitude.
      expect(
        find.text(
          l10n.walletRescanEstimate(
            compactBlockCount(fixtureTip - fixtureFloor, l10n.localeName),
          ),
        ),
        findsOneWidget,
      );
      expect(find.text(l10n.walletRescanPick), findsOneWidget);
      expect(find.text(l10n.walletRescanScanAll), findsOneWidget);
      expect(find.text(l10n.walletRescanChange), findsNothing);
    },
  );

  testWidgets(
    "#317: a YOUNG wallet's default threads the wallet's FLOOR height — never "
    'a height above it (an above-floor rebuild hides funds), never a dead '
    'pre-birthday scan',
    (tester) async {
      final spy = _SpyRescan();
      final l10n = await openSheet(tester, spy);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();

      expect(spy.called, isTrue);
      final target = spy.lastTarget;
      expect((target as RescanFromWalletBirthday?)?.floorHeight, fixtureFloor);
      // The sheet pops after the rescan resolves.
      expect(find.text(l10n.walletRescanTitle), findsNothing);
    },
  );

  testWidgets(
    '#317 S157 BLOCKER GUARD: an OLDER wallet ALSO defaults to its OWN floor — '
    'NEVER a height above it. A default above the floor would rebuild from '
    'there and HIDE the funds below it (rescan is destructive lower-only)',
    (tester) async {
      final spy = _SpyRescan();
      final l10n = await openSheet(
        tester,
        spy,
        session: sessionWithFloor(oldWalletFloor),
      );

      // Same walletStart default as a young wallet — not a dated arm above it.
      expect(find.text(l10n.walletRescanRangeDefault), findsOneWidget);
      expect(find.text(l10n.walletRescanChange), findsNothing);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();

      expect(spy.called, isTrue);
      expect(
        (spy.lastTarget as RescanFromWalletBirthday?)?.floorHeight,
        oldWalletFloor,
        reason: 'the default is the floor VERBATIM, never a higher start',
      );
    },
  );

  testWidgets(
    '#317 reliability: a NULL floor (account not provisioned) degrades to '
    'SCAN-ALL — the only money-safe default when the floor is unknown — with '
    'Start ENABLED (never a permanently-disabled Start)',
    (tester) async {
      final spy = _SpyRescan();
      final l10n = await openSheet(
        tester,
        spy,
        session: sessionWithFloor(null),
      );

      // A blind dated default could sit above the true floor and hide funds;
      // Scan-all (floors to activation) cannot. Start is live.
      expect(find.text(l10n.walletRescanRangeAll), findsOneWidget);
      final start = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletRescanConfirm),
      );
      expect(start.onPressed, isNotNull);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();
      expect(spy.called, isTrue);
      expect(spy.lastTarget, isA<RescanAllHistory>());
    },
  );

  testWidgets(
    '#317 S157: a FAULTED floor read degrades to SCAN-ALL (unknown floor ⇒ the '
    'safe default), never a blind dated start that could hide funds',
    (tester) async {
      final spy = _SpyRescan();
      final session = sessionWithTip()
        ..birthdayHeightThrows = StateError('wedged bridge');
      final l10n = await openSheet(tester, spy, session: session);

      expect(find.text(l10n.walletRescanRangeAll), findsOneWidget);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();

      expect(spy.called, isTrue);
      expect(spy.lastTarget, isA<RescanAllHistory>());
    },
  );

  testWidgets(
    '#317 S157 reliability HIGH: a Scan-all tapped DURING the floor read is '
    'PRESERVED — the landing floor must not silently revert the user\'s choice',
    (tester) async {
      final gate = Completer<void>();
      final spy = _SpyRescan();
      final session = sessionWithTip()..birthdayHeightGate = gate;
      await tester.pumpWidget(host(spy, session: session));
      await tester.tap(find.text('open'));
      await tester.pump(); // start the sheet's entrance animation
      await tester.pump(const Duration(milliseconds: 350)); // settle it (< 15s)
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ElevatedButton)),
      );

      // The user chooses Scan-all while the floor is still loading.
      await tester.tap(find.text(l10n.walletRescanScanAll));
      await tester.pump();
      expect(find.text(l10n.walletRescanRangeAll), findsOneWidget);

      // The floor lands — it must NOT clobber the user's Scan-all back to the
      // wallet-start default.
      gate.complete();
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRescanRangeAll), findsOneWidget);
      expect(find.text(l10n.walletRescanRangeDefault), findsNothing);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();
      expect(spy.lastTarget, isA<RescanAllHistory>());
    },
  );

  testWidgets(
    '#317 S157 reliability HIGH: a deep date PICKED during the floor read is '
    'PRESERVED (a below-floor recovery pick is never reverted to the default)',
    (tester) async {
      final gate = Completer<void>();
      final spy = _SpyRescan();
      final session = sessionWithTip()..birthdayHeightGate = gate;
      await tester.pumpWidget(host(spy, session: session));
      await tester.tap(find.text('open'));
      await tester.pump(); // start the entrance animation
      await tester.pump(const Duration(milliseconds: 350)); // settle it (< 15s)
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ElevatedButton)),
      );

      await tester.tap(find.text(l10n.walletRescanPick));
      await tester.pumpAndSettle();
      await tester.tap(find.text('OK')); // the picker's initial ~1yr-ago date
      await tester.pump();
      expect(find.text(l10n.walletRescanChange), findsOneWidget); // a pick

      gate.complete();
      await tester.pumpAndSettle();
      // Still the user's dated pick, not the wallet-start default.
      expect(find.text(l10n.walletRescanChange), findsOneWidget);
      expect(find.text(l10n.walletRescanRangeDefault), findsNothing);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();
      expect(
        (spy.lastTarget as RescanFromTime?)?.earliestTime.year,
        DateTime.now().year - 1,
      );
    },
  );

  testWidgets(
    '#317 S157: cancelling the sheet WHILE the floor read is in flight throws '
    'no setState-after-dispose when the read later resolves',
    (tester) async {
      final gate = Completer<void>();
      final spy = _SpyRescan();
      final session = sessionWithTip()..birthdayHeightGate = gate;
      await tester.pumpWidget(host(spy, session: session));
      await tester.tap(find.text('open'));
      // Let the sheet finish sliding in while the floor read is still parked
      // on the gate. One frame is not enough: the sheet is still below the
      // screen, the Cancel tap misses, and the row passes without ever
      // cancelling (found). pumpAndSettle would never settle here — the
      // resolving spinner animates until the gate opens. Two pumps: the first
      // frame STARTS the entrance animation's clock, the second advances it.
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 600));
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ElevatedButton)),
      );

      final cancel = find.text(l10n.walletRescanCancel);
      await tester.ensureVisible(cancel);
      await tester.pump(const Duration(milliseconds: 100));
      await tester.tap(cancel);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 600));
      // The sheet is gone while the read is still pending — the precondition
      // the row exists for.
      expect(find.text(l10n.walletRescanCancel), findsNothing);
      expect(gate.isCompleted, isFalse);

      gate.complete(); // the read resolves onto the disposed state
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(spy.called, isFalse);
    },
  );

  testWidgets(
    '#317 S157: the size cue is SUPPRESSED at a sub-2-block span (floor at the '
    'tip) — "About 0 blocks to scan" would read as broken',
    (tester) async {
      final spy = _SpyRescan();
      // Floor == tip ⇒ zero blocks to scan.
      final session = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: fixtureTip),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: fixtureTip),
          tip: fixtureTip,
        ),
      )..birthdayHeightResult = fixtureTip;
      final l10n = await openSheet(tester, spy, session: session);

      expect(find.text(l10n.walletRescanRangeDefault), findsOneWidget);
      expect(
        find.textContaining(l10n.walletRescanEstimate('0')),
        findsNothing,
        reason: 'no "About 0 blocks" cue at a floor-at-tip span',
      );
    },
  );

  testWidgets(
    '"Scan all history" → the all-history intent threaded (recover all)',
    (tester) async {
      final spy = _SpyRescan();
      final l10n = await openSheet(tester, spy);

      await tester.tap(find.text(l10n.walletRescanScanAll));
      await tester.pumpAndSettle();
      // Now the range reads as a full scan; "Pick a date" is still offered,
      // and the Scan-all button hides (already there).
      expect(find.text(l10n.walletRescanRangeAll), findsOneWidget);
      expect(find.text(l10n.walletRescanPick), findsOneWidget);
      expect(find.text(l10n.walletRescanScanAll), findsNothing);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();

      expect(spy.called, isTrue);
      expect(spy.lastTarget, isA<RescanAllHistory>());
    },
  );

  testWidgets(
    '#317: a PICKED date threads as a date — NEVER clamped to the floor in '
    'either direction (an earlier pick is the post-restore recovery)',
    (tester) async {
      final spy = _SpyRescan();
      final l10n = await openSheet(tester, spy);

      await tester.tap(find.text(l10n.walletRescanPick));
      await tester.pumpAndSettle();
      expect(find.byType(DatePickerDialog), findsOneWidget);
      // Accept the picker's initial date (~1 year ago — BELOW the fixture
      // floor's implied age, i.e. the deeper-recovery direction).
      await tester.tap(find.text('OK'));
      await tester.pumpAndSettle();

      // The chosen-date description + Change affordance render.
      expect(find.text(l10n.walletRescanChange), findsOneWidget);

      await tapConfirm(tester, l10n);
      await tester.pumpAndSettle();

      expect(spy.called, isTrue);
      final target = spy.lastTarget;
      expect(
        (target as RescanFromTime?)?.earliestTime.year,
        DateTime.now().year - 1,
        reason: 'the pick passes through as a DATE, not a clamped height',
      );
    },
  );

  testWidgets(
    '#317: while the scan-floor read is in flight, Start is DISABLED and the '
    'range shows the resolving cue; Pick + Scan-all stay live (never wedged)',
    (tester) async {
      final gate = Completer<void>();
      final spy = _SpyRescan();
      final session = sessionWithTip()..birthdayHeightGate = gate;
      await tester.pumpWidget(host(spy, session: session));
      await tester.tap(find.text('open'));
      await tester.pump(); // open the sheet; the floor read is parked
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ElevatedButton)),
      );

      // Resolving: the recommended-range cue shows, Start is disabled...
      expect(find.text(l10n.walletRescanRangeResolving), findsOneWidget);
      final start = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletRescanConfirm),
      );
      expect(start.onPressed, isNull);
      // ...but the user is never stuck: Pick and Scan-all remain live.
      final pick = tester.widget<OutlinedButton>(
        find.widgetWithText(OutlinedButton, l10n.walletRescanPick),
      );
      expect(pick.onPressed, isNotNull);

      // The floor lands → the default resolves and Start enables.
      gate.complete();
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRescanRangeResolving), findsNothing);
      final startAfter = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletRescanConfirm),
      );
      expect(startAfter.onPressed, isNotNull);
    },
  );

  testWidgets('Cancel dismisses WITHOUT rescanning', (tester) async {
    final spy = _SpyRescan();
    final l10n = await openSheet(tester, spy);

    // Cancel is the last control in the scrollable sheet; on the small test
    // viewport the #390 swap cross-pointer pushes it below the fold, so scroll
    // it into view before tapping (production caps the sheet + scrolls it).
    await tester.ensureVisible(find.text(l10n.walletRescanCancel));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletRescanCancel));
    await tester.pumpAndSettle();

    expect(spy.called, isFalse);
    expect(find.text(l10n.walletRescanTitle), findsNothing);
  });

  testWidgets(
    'S11 C4: the sheet stays LOCKED — a tap on the scrim above it and '
    'a drag down both leave it open (a §9.4 must-preserve)',
    (tester) async {
      // A tall window, so the sheet does not fill it and a scrim shows above.
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final l10n = await openSheet(tester, _SpyRescan());
      // Precondition: the scrim is really there above the sheet, so the tap
      // below hits the barrier, not the sheet (else the row passes vacuously).
      final sheetTop = tester.getTopLeft(find.byType(BottomSheet)).dy;
      expect(sheetTop, greaterThan(20));

      await tester.tapAt(const Offset(400, 8));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRescanTitle), findsOneWidget);

      await tester.drag(
        find.text(l10n.walletRescanTitle),
        const Offset(0, 500),
      );
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRescanTitle), findsOneWidget);
    },
  );

  testWidgets('the date picker opens from the range control', (tester) async {
    final l10n = await openSheet(tester, _SpyRescan());

    await tester.tap(find.text(l10n.walletRescanPick));
    await tester.pumpAndSettle();

    // Flutter's Material date picker dialog is up (its OK/Cancel actions show).
    expect(find.byType(DatePickerDialog), findsOneWidget);
  });

  testWidgets('while submitting, the spinner shows and every control disables', (
    tester,
  ) async {
    final gate = Completer<void>();
    final spy = _SpyRescan(gate: gate);
    final l10n = await openSheet(tester, spy);

    await tapConfirm(tester, l10n);
    await tester
        .pump(); // enter the submitting state (rescan is gated, in flight)

    // The Start button now shows the running label + a spinner...
    expect(find.text(l10n.walletRescanRunning), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    // ...Cancel is disabled (a started rescan is never abandoned from the sheet)...
    final cancel = tester.widget<TextButton>(
      find.widgetWithText(TextButton, l10n.walletRescanCancel),
    );
    expect(cancel.onPressed, isNull);
    // ...and the date-range control is disabled (no mid-rebuild date change).
    final pickBtn = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, l10n.walletRescanPick),
    );
    expect(pickBtn.onPressed, isNull);

    // Release the rescan → the sheet pops.
    gate.complete();
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletRescanTitle), findsNothing);
  });

  testWidgets(
    'with a send still settling, the sheet shows the fence advisory BEFORE '
    'Start — and Start stays ENABLED (engine-authoritative, #364 M3)',
    (tester) async {
      final session = sessionWithTip()
        ..inFlightSendsResult = [inFlightSendFixture()];
      final l10n = await openSheet(tester, _SpyRescan(), session: session);

      expect(find.text(l10n.walletRescanSettlingAdvisory), findsOneWidget);
      // Advisory only: the in-flight read is a cautionary view — a send can
      // settle between this frame and the confirm, so the engine's fence
      // stays the decider and Start must not be pre-blocked.
      final start = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletRescanConfirm),
      );
      expect(start.onPressed, isNotNull);
    },
  );

  testWidgets(
    'the settling advisory is a LIVE REGION — a late in-flight read resolving '
    'after the sheet opened must be heard, not silently inserted (S219-b U5)',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final session = sessionWithTip()
        ..inFlightSendsResult = [inFlightSendFixture()];
      final l10n = await openSheet(tester, _SpyRescan(), session: session);

      expect(
        tester
            .getSemantics(find.text(l10n.walletRescanSettlingAdvisory))
            .flagsCollection
            .isLiveRegion,
        isTrue,
      );
      semantics.dispose();
    },
  );

  testWidgets('no settling advisory when nothing is in flight (#364 M3)', (
    tester,
  ) async {
    final l10n = await openSheet(tester, _SpyRescan());
    expect(find.text(l10n.walletRescanSettlingAdvisory), findsNothing);
  });

  // The twin of the restore screen's birthday row: an ellipsized "Scan all
  // history" once kept this row on one line; the walk showed the restore
  // twin cut to "Scan all hi…" on a 332dp phone at 1.2x. The row wraps now.
  // 280dp at 3.0x: a label wider than the whole row wraps inside its button.
  for (final (width, scale) in const [
    (320.0, 1.4),
    (332.0, 1.2),
    (280.0, 3.0),
  ]) {
    testWidgets(
      'the range buttons render whole, without overflow or a cut label, at '
      '${width.toInt()}dp and ${scale}x text',
      (tester) async {
        tester.view.physicalSize = Size(width, 1400);
        tester.view.devicePixelRatio = 1.0;
        tester.platformDispatcher.textScaleFactorTestValue = scale;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);

        final l10n = await openSheet(tester, _SpyRescan());

        // The default range is the wallet's start, so BOTH buttons show.
        for (final label in [l10n.walletRescanPick, l10n.walletRescanScanAll]) {
          expect(find.text(label), findsOneWidget);
          expect(
            tester
                .renderObject<RenderParagraph>(find.text(label))
                .didExceedMaxLines,
            isFalse,
            reason: '"$label" must render whole, never ellipsized',
          );
        }
        expect(tester.takeException(), isNull);
      },
    );
  }
}
