import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/ephemeral_sweep.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
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
        transparentZat: 1000000,
      ),
    ),
  );
}

Future<void> _openOverflowMenu(WidgetTester tester) async {
  final l10n = _l10nAt(tester);
  await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
  await tester.pumpAndSettle();
}

void main() {
  group('the always-available one-time check (v-5c finding #3)', () {
    testWidgets(
      'the menu entry is available with NO recoverable funds detected '
      '(the past-window return Recover-now can never surface)',
      (tester) async {
        // The automatic windowed detect reads EMPTY — so the gated Recover-now
        // button is absent — yet the check stays reachable.
        final fake = _activeFake()..recoverableEphemeralFundsResult = const [];
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        expect(
          find.text(l10n.walletRecoverNow),
          findsNothing,
          reason: 'the gated button has nothing to show',
        );
        await _openOverflowMenu(tester);
        expect(find.text(l10n.walletCheckOneTimeMenuItem), findsOneWidget);

        // Tap → the SAME confirm dialog as Recover-now → confirm → one sweep.
        await tester.tap(find.text(l10n.walletCheckOneTimeMenuItem));
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletRecoverConfirmTitle), findsOneWidget);
        await tester.tap(find.text(l10n.walletRecoverConfirmAction));
        await tester.pumpAndSettle();

        expect(fake.sweepCount, 1);
        // Default fake summary sweeps nothing — the honest, common answer.
        expect(find.text(l10n.walletRecoverNothing), findsOneWidget);
        await tester.pumpAndSettle(
          const Duration(seconds: 5),
        ); // drain snackbar
      },
    );

    testWidgets('a funded check reports the provisional recovered amount', (
      tester,
    ) async {
      final fake = _activeFake()
        ..recoverableEphemeralFundsResult = const []
        ..sweepResult = ephemeralSweepSummaryFixture(recoveredZat: 250000);
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await _openOverflowMenu(tester);
      await tester.tap(find.text(l10n.walletCheckOneTimeMenuItem));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRecoverConfirmAction));
      await tester.pumpAndSettle();

      expect(
        find.text(l10n.walletRecoverDone(l10n.walletAmount(formatZec(250000)))),
        findsOneWidget,
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets(
      'a truncated-only check reports an INCOMPLETE check, never "some funds"',
      (tester) async {
        // truncated means addresses UNCHECKED (the per-invocation cap), not
        // funds found — from the always-available check entry, the old
        // "Some funds need another try" copy invented money the run never
        // saw (UX review F5).
        final fake = _activeFake()
          ..recoverableEphemeralFundsResult = const []
          ..sweepResult = ephemeralSweepSummaryFixture(swept: 0, truncated: 2);
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletCheckOneTimeMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletRecoverConfirmAction));
        await tester.pumpAndSettle();

        expect(find.text(l10n.walletRecoverTruncated), findsOneWidget);
        expect(
          find.text(l10n.walletRecoverRetry),
          findsNothing,
          reason: 'no per-address fault ⇒ no "funds need another try" claim',
        );
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );

    testWidgets('cancelling the confirm runs NO sweep', (tester) async {
      final fake = _activeFake()..recoverableEphemeralFundsResult = const [];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await _openOverflowMenu(tester);
      await tester.tap(find.text(l10n.walletCheckOneTimeMenuItem));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRecoverConfirmCancel));
      await tester.pumpAndSettle();

      expect(fake.sweepCount, 0);
    });

    testWidgets(
      'the single-flight latch is SHARED: a menu-launched sweep disables '
      'Recover-now (spinner) AND the menu entry',
      (tester) async {
        final fake = _activeFake()
          // Recoverable funds so the gated Recover-now button renders too.
          ..recoverableEphemeralFundsResult = const [
            RecoverableEphemeralFunds(recoverableZat: 250000, isFinal: true),
          ]
          ..sweepNeverCompletes = true; // hold the sweep in flight
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        // Launch from the MENU entry.
        await _openOverflowMenu(tester);
        await tester.tap(find.text(l10n.walletCheckOneTimeMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletRecoverConfirmAction));
        await tester.pump(); // the sweep never completes — no settle
        expect(fake.sweepCount, 1);

        // The balance-card button reflects the SHARED latch: in-progress + disabled.
        expect(find.text(l10n.walletRecoverInProgress), findsOneWidget);
        final button = tester.widget<OutlinedButton>(
          find.ancestor(
            of: find.text(l10n.walletRecoverInProgress),
            matching: find.byType(OutlinedButton),
          ),
        );
        expect(
          button.onPressed,
          isNull,
          reason: 'the other surface is disabled while any sweep runs',
        );

        // The menu entry itself is disabled AND its label says why (UX
        // review F2: "Recovering…" replaces the action label, so the reason is
        // visible/announced — and it doubles as the only in-progress cue when
        // the balance-card button is absent). A re-launch attempt is inert.
        // (Bounded pumps, never pumpAndSettle: the in-flight spinner animates
        // forever, so "settle" can never be reached while the sweep runs.)
        await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
        await tester.pump(const Duration(milliseconds: 300)); // menu opens
        expect(
          find.text(l10n.walletCheckOneTimeMenuItem),
          findsNothing,
          reason: 'the latched entry renames to the in-progress label',
        );
        // Two "Recovering…" now: the balance-card button + the menu entry.
        expect(find.text(l10n.walletRecoverInProgress), findsNWidgets(2));
        await tester.tap(
          find.text(l10n.walletRecoverInProgress).last,
          warnIfMissed: false,
        );
        await tester.pump(const Duration(milliseconds: 300));
        expect(
          find.text(l10n.walletRecoverConfirmTitle),
          findsNothing,
          reason: 'a disabled entry opens no second confirm',
        );
        expect(fake.sweepCount, 1, reason: 'never a second concurrent sweep');
      },
    );
  });

  group('EphemeralSweepController (the shared single-flight owner)', () {
    test(
      'a second sweep() while one runs returns null and never re-enters',
      () async {
        final fake = _activeFake()..sweepNeverCompletes = true;
        final container = ProviderContainer(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
        );
        addTearDown(container.dispose);
        final controller = container.read(
          ephemeralSweepInFlightProvider.notifier,
        );

        final first = controller.sweep(); // in flight forever
        await Future<void>.delayed(Duration.zero);
        expect(container.read(ephemeralSweepInFlightProvider), isTrue);

        final second = await controller.sweep();
        expect(second, isNull, reason: 'single-flight: the first run owns it');
        expect(fake.sweepCount, 1);
        expect(first, isA<Future<EphemeralSweepSummary?>>());
      },
    );

    test('no session ⇒ null, latch untouched', () async {
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(null)],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        ephemeralSweepInFlightProvider.notifier,
      );
      expect(await controller.sweep(), isNull);
      expect(container.read(ephemeralSweepInFlightProvider), isFalse);
    });

    test('a sweep fault resets the latch and propagates', () async {
      final fake = _activeFake()..sweepThrows = StateError('boom');
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        ephemeralSweepInFlightProvider.notifier,
      );
      await expectLater(controller.sweep(), throwsStateError);
      expect(
        container.read(ephemeralSweepInFlightProvider),
        isFalse,
        reason: 'the latch must never stick shut after a fault',
      );
    });
  });

  group('the HOST send authorizer (#327 seam)', () {
    test('a sweep routes through the authorizer EXACTLY once as a SWEEP '
        'intent with NO amount (the total is unknown up front)', () async {
      final fake = _activeFake();
      final auth = FakeSendAuthorizer();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);
      await container.read(ephemeralSweepInFlightProvider.notifier).sweep();
      expect(auth.intents, hasLength(1));
      expect(auth.intents.single.kind, WalletSpendKind.sweep);
      expect(auth.intents.single.amountZat, isNull);
      expect(
        auth.intents.single.recipientIsSelf,
        isTrue,
        reason: '#383 R3: the sweep RETURNS funds to this wallet',
      );
      expect(fake.sweepCount, 1);
    });

    test('a sweep intent carries NO spend-binding token — the sweep pulls '
        'unbound (FR-17 #396)', () async {
      final fake = _activeFake();
      final auth = FakeSendAuthorizer();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);
      await container.read(ephemeralSweepInFlightProvider.notifier).sweep();
      expect(
        auth.intents.single.bindingToken,
        isNull,
        reason:
            'FR-17: the shield-to-self sweep signs UNBOUND — a '
            'host-custody supplier must not expect a nonce here',
      );
    });

    test(
      'a DENIED sweep makes ZERO bridge calls and resets the latch',
      () async {
        final fake = _activeFake();
        final auth = FakeSendAuthorizer(denyAll: true);
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSendAuthorizerProvider.overrideWithValue(auth),
          ],
        );
        addTearDown(container.dispose);
        final controller = container.read(
          ephemeralSweepInFlightProvider.notifier,
        );
        await expectLater(
          controller.sweep(),
          throwsA(isA<WalletSpendAuthorizationDenied>()),
        );
        expect(fake.sweepCount, 0);
        expect(
          container.read(ephemeralSweepInFlightProvider),
          isFalse,
          reason: 'a denial must never stick the shared latch shut',
        );
      },
    );

    test(
      'an approval landing after a mid-prompt session flip is FENCED — '
      'the sweep runs on NEITHER session and the latch resets (S153)',
      () async {
        final gate = Completer<void>();
        final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
        final fakeA = _activeFake();
        final fakeB = _activeFake();
        final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWith(
              (ref) => ref.watch(sessionSwitch),
            ),
            walletSendAuthorizerProvider.overrideWithValue(auth),
          ],
        );
        addTearDown(container.dispose);
        final controller = container.read(
          ephemeralSweepInFlightProvider.notifier,
        );
        final pending = controller.sweep(); // parks at the host prompt
        expect(container.read(ephemeralSweepInFlightProvider), isTrue);

        container.read(sessionSwitch.notifier).state = fakeB;
        await Future<void>.delayed(Duration.zero);

        gate.complete(); // the approval lands AFTER the identity switch
        await expectLater(pending, throwsA(isA<WalletSpendSessionChanged>()));
        expect(
          fakeA.sweepCount,
          0,
          reason: 'the fence refused the dead identity\'s sweep',
        );
        expect(fakeB.sweepCount, 0);
        expect(
          container.read(ephemeralSweepInFlightProvider),
          isFalse,
          reason: 'the latch resets like any other sweep exit',
        );
      },
    );

    testWidgets(
      'a DENIED sweep from the always-available entry shows NO snackbar — '
      'the host\'s own prompt was the communication, and "recovery failed" '
      'would be a lie',
      (tester) async {
        final fake = _activeFake()..recoverableEphemeralFundsResult = const [];
        final auth = FakeSendAuthorizer(denyAll: true);
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              walletSessionProvider.overrideWithValue(fake),
              walletSendAuthorizerProvider.overrideWithValue(auth),
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
        await tester.tap(find.text(l10n.walletCheckOneTimeMenuItem));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletRecoverConfirmAction));
        await tester.pumpAndSettle();

        expect(fake.sweepCount, 0);
        expect(find.byType(SnackBar), findsNothing);
      },
    );
  });
}
