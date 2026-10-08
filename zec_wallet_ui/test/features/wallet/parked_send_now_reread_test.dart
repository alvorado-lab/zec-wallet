import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

import '../../support/scripted_authorizer.dart';

/// R13 §4.5 row 5 (and row 1b's parked arms) — the parked "Send now" never
/// says "The payment is unchanged. Try again." over a transaction that may
/// already be saved (`docs/plan/r13-a-saved-transaction-is-never-offered-a-retry.md`
/// §4.4).
///
/// `drain_multi` commits the wallet-db write and only then runs
/// `mark_sent_multi` and `read_raw_tx`, so a store error from
/// `authorizeParkedSend` can arrive over a saved transaction. An error the
/// parked list cannot prove came first is answered by RE-READING the parked
/// list: the row present (same `(id, createdAt)`) and not `sending` → it IS
/// unchanged; absent, `sending`, or an unreadable list → "your wallet may
/// already be sending it" ([WalletLocalizations.walletParkedAlreadyInProgress]).
///
/// The re-read is observed through the fake's `parkedSendsResult` /
/// `listParkedSendsThrows`, which the scripted authorizer changes BEFORE the
/// action runs (nothing re-reads the list between that point and the error).
/// A pre-persistence error is pinned as taking NO re-read by making the
/// re-read fail: had it been taken, the answer would be AlreadyInProgress.
Widget _harness(FakeWalletSession session, WalletSendAuthorizer authorizer) =>
    ProviderScope(
      overrides: [
        walletSessionProvider.overrideWithValue(session),
        walletSendAuthorizerProvider.overrideWithValue(authorizer),
      ],
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: const WalletScreen(),
      ),
    );

/// Up to date, funded — the parked surface renders (parked_and_recover_test's
/// `_activeFake` shape).
FakeWalletSession _activeFake(List<ParkedSend> parked) => FakeWalletSession(
  current: const SyncStatus.upToDate(tip: 100),
  snapshotValue: walletStateFixture(
    syncStatus: const SyncStatus.upToDate(tip: 100),
    balance: balanceFixture(
      spendableZat: 1000000,
      totalZat: 1000000,
      transparentZat: 1000000,
    ),
  ),
)..parkedSendsResult = parked;

const _id = 9;
const _createdAt = 1700000456;

ParkedSend _row({bool paused = false, bool sending = false, int? createdAt}) =>
    parkedSendFixture(
      id: _id,
      createdAt: createdAt ?? _createdAt,
      paused: paused,
      sending: sending,
    );

WalletApiError _err(WalletErrorKind kind) =>
    WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

/// Pumps the wallet screen, taps the row's "Send now" and settles; returns
/// the localizations so the row asserts on KEYS, not literals.
Future<WalletLocalizations> _tapSendNow(
  WidgetTester tester,
  FakeWalletSession fake,
  WalletSendAuthorizer authorizer,
) async {
  await tester.pumpWidget(_harness(fake, authorizer));
  await tester.pumpAndSettle();
  final l10n = WalletLocalizations.of(
    tester.element(find.byType(WalletScreen)),
  );
  final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
  await tester.ensureVisible(sendNow);
  await tester.pumpAndSettle();
  await tester.tap(sendNow);
  await tester.pumpAndSettle();
  return l10n;
}

/// Clears the snackbar's timers so the test ends with no pending work.
Future<void> _drain(WidgetTester tester) =>
    tester.pumpAndSettle(const Duration(seconds: 5));

void main() {
  // A phone-tall viewport, as parked_and_recover_test: the parked section sits
  // below the fold of the default 800x600 surface.
  setUp(() {
    final view =
        TestWidgetsFlutterBinding.instance.platformDispatcher.views.first;
    view.physicalSize = const Size(800, 1200) * view.devicePixelRatio;
  });
  tearDown(() {
    TestWidgetsFlutterBinding.instance.platformDispatcher.views.first
        .resetPhysicalSize();
  });

  group(
    'R13 row 5 — a store error from authorizeParkedSend re-reads the row',
    () {
      testWidgets('StoreBusy, re-read row SENDING → AlreadyInProgress', (
        tester,
      ) async {
        final fake = _activeFake([_row()])
          ..authorizeParkedSendThrows = _err(const WalletErrorKind.storeBusy());
        final auth = ScriptedAuthorizer(
          beforeAction: () => fake.parkedSendsResult = [_row(sending: true)],
        );
        final l10n = await _tapSendNow(tester, fake, auth);
        expect(fake.authorizeParkedSendCount, 1);
        expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
        expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
        await _drain(tester);
      });

      testWidgets('StoreBusy, re-read row ABSENT → AlreadyInProgress', (
        tester,
      ) async {
        final fake = _activeFake([_row()])
          ..authorizeParkedSendThrows = _err(const WalletErrorKind.storeBusy());
        final auth = ScriptedAuthorizer(
          beforeAction: () => fake.parkedSendsResult = const [],
        );
        final l10n = await _tapSendNow(tester, fake, auth);
        expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
        expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
        await _drain(tester);
      });

      testWidgets('StoreBusy, re-read row PRESENT and idle → "unchanged, try '
          'again" (it IS unchanged)', (tester) async {
        final fake = _activeFake([_row()])
          ..authorizeParkedSendThrows = _err(const WalletErrorKind.storeBusy());
        final l10n = await _tapSendNow(tester, fake, ScriptedAuthorizer());
        expect(find.text(l10n.walletParkedAuthorizeFailed), findsOneWidget);
        expect(find.text(l10n.walletParkedAlreadyInProgress), findsNothing);
        await _drain(tester);
      });

      testWidgets('StoreBusy, the re-read itself FAILS → AlreadyInProgress', (
        tester,
      ) async {
        final fake = _activeFake([_row()])
          ..authorizeParkedSendThrows = _err(const WalletErrorKind.storeBusy());
        final auth = ScriptedAuthorizer(
          beforeAction: () =>
              fake.listParkedSendsThrows = StateError('re-read failed'),
        );
        final l10n = await _tapSendNow(tester, fake, auth);
        expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
        expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
        await _drain(tester);
      });

      testWidgets('a row with the same id but another createdAt counts as '
          'ABSENT (rowids are reused) → AlreadyInProgress', (tester) async {
        final fake = _activeFake([_row()])
          ..authorizeParkedSendThrows = _err(const WalletErrorKind.storeBusy());
        final auth = ScriptedAuthorizer(
          beforeAction: () =>
              fake.parkedSendsResult = [_row(createdAt: _createdAt + 1)],
        );
        final l10n = await _tapSendNow(tester, fake, auth);
        expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
        expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
        await _drain(tester);
      });

      for (final (name, kind) in <(String, WalletErrorKind)>[
        ('StoreCorrupt', const WalletErrorKind.storeCorrupt()),
        ('DiskFull', const WalletErrorKind.diskFull()),
        ('Io', const WalletErrorKind.io()),
      ]) {
        testWidgets('$name with the row gone → AlreadyInProgress (every store '
            'kind may follow the commit)', (tester) async {
          final fake = _activeFake([_row()])
            ..authorizeParkedSendThrows = _err(kind);
          final auth = ScriptedAuthorizer(
            beforeAction: () => fake.parkedSendsResult = const [],
          );
          final l10n = await _tapSendNow(tester, fake, auth);
          expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
          await _drain(tester);
        });
      }

      testWidgets(
        'an untyped throw from authorizeParkedSend with the row gone → '
        'AlreadyInProgress',
        (tester) async {
          final fake = _activeFake([_row()])
            ..authorizeParkedSendThrows = StateError('untyped');
          final auth = ScriptedAuthorizer(
            beforeAction: () => fake.parkedSendsResult = const [],
          );
          final l10n = await _tapSendNow(tester, fake, auth);
          expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
          await _drain(tester);
        },
      );
    },
  );

  group('R13 row 5 — errors that provably precede signing keep "unchanged" '
      'WITHOUT a re-read', () {
    testWidgets('SeedMismatch → "unchanged, try again"; a re-read (rigged to '
        'fail) is never taken', (tester) async {
      final fake = _activeFake(
        [_row()],
      )..authorizeParkedSendThrows = _err(const WalletErrorKind.seedMismatch());
      final auth = ScriptedAuthorizer(
        beforeAction: () =>
            fake.listParkedSendsThrows = StateError('a re-read was taken'),
      );
      final l10n = await _tapSendNow(tester, fake, auth);
      expect(find.text(l10n.walletParkedAuthorizeFailed), findsOneWidget);
      expect(find.text(l10n.walletParkedAlreadyInProgress), findsNothing);
      await _drain(tester);
    });

    testWidgets('a re-arm throw BEFORE any signing call → "unchanged", no '
        'authorize, no re-read (even for a store kind)', (tester) async {
      final fake = _activeFake([_row(paused: true)])
        ..retryParkedSendThrows = _err(const WalletErrorKind.storeBusy());
      final auth = ScriptedAuthorizer(
        beforeAction: () =>
            fake.listParkedSendsThrows = StateError('a re-read was taken'),
      );
      final l10n = await _tapSendNow(tester, fake, auth);
      expect(fake.retryParkedSendCount, 1);
      expect(fake.authorizeParkedSendCount, 0);
      expect(find.text(l10n.walletParkedAuthorizeFailed), findsOneWidget);
      expect(find.text(l10n.walletParkedAlreadyInProgress), findsNothing);
      await _drain(tester);
    });
  });

  group('R13 row 5 — a successful re-arm, then a pre-signing failure, gets the '
      're-armed wording (#400 R5), not "unchanged"', () {
    testWidgets('re-armed, then SeedMismatch → walletParkedAuthorizeRearmed', (
      tester,
    ) async {
      final fake = _activeFake([_row(paused: true)])
        ..retryParkedSendResult = true
        ..authorizeParkedSendThrows = _err(
          const WalletErrorKind.seedMismatch(),
        );
      final l10n = await _tapSendNow(tester, fake, ScriptedAuthorizer());
      expect(fake.retryParkedSendCount, 1);
      expect(find.text(l10n.walletParkedAuthorizeRearmed), findsOneWidget);
      expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
      await _drain(tester);
    });

    testWidgets('re-armed, then StoreBusy with the row present and idle → '
        'walletParkedAuthorizeRearmed', (tester) async {
      final fake = _activeFake([_row(paused: true)])
        ..retryParkedSendResult = true
        ..authorizeParkedSendThrows = _err(const WalletErrorKind.storeBusy());
      final auth = ScriptedAuthorizer(
        // The re-arm un-paused it; still waiting, not sending.
        beforeAction: () => fake.parkedSendsResult = [_row()],
      );
      final l10n = await _tapSendNow(tester, fake, auth);
      expect(find.text(l10n.walletParkedAuthorizeRearmed), findsOneWidget);
      expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
      await _drain(tester);
    });
  });

  group('R13 row 1b — parked: the host acts AFTER authorizeParkedSend ran', () {
    for (final (name, thenThrow) in [
      ('declines', hostDeclineAfter),
      ('throws SessionChanged', hostSessionChangedAfter),
      ('throws its own fault', hostFaultAfter),
    ]) {
      testWidgets(
        'the host $name after the spend, row gone → AlreadyInProgress, '
        'never silence or "unchanged"',
        (tester) async {
          final fake = _activeFake([_row()]);
          final auth = ScriptedAuthorizer(
            beforeAction: () => fake.parkedSendsResult = const [],
            afterAction: thenThrow,
          );
          final l10n = await _tapSendNow(tester, fake, auth);
          expect(fake.authorizeParkedSendCount, 1, reason: 'the spend ran');
          expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
          expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
          await _drain(tester);
        },
      );
    }

    testWidgets(
      'a decline BEFORE the spend → silence, no bridge call (today)',
      (tester) async {
        final fake = _activeFake([_row()]);
        final auth = ScriptedAuthorizer(
          beforeAction: () => throw const WalletSpendAuthorizationDenied(),
        );
        await _tapSendNow(tester, fake, auth);
        expect(fake.authorizeParkedSendCount, 0);
        expect(find.byType(SnackBar), findsNothing);
        await _drain(tester);
      },
    );
  });
}
