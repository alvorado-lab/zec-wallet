import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_controller.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_activity_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

import '../../../support/scripted_authorizer.dart';

/// R13 §4.5 rows 1b, 3 and 4 for MOVE-TO-TRANSPARENT
/// (`docs/plan/r13-a-saved-transaction-is-never-offered-a-retry.md`).
///
/// The money row: a move whose answer was lost may already be saved and will
/// broadcast on the next pass. Today's "Couldn't complete this move" + Retry
/// runs `start()` — a fresh propose over OTHER notes — so a retry deshields
/// twice (double transparent exposure, a second fee). After
/// [MoveOutcomeUnknown] nothing reachable from the sheet may start a propose.
void main() {
  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  MoveToTransparentController ctl(ProviderContainer c) =>
      c.read(moveToTransparentControllerProvider.notifier);
  MoveToTransparentState st(ProviderContainer c) =>
      c.read(moveToTransparentControllerProvider);

  /// A container at [MoveReady] over a fake whose `send` is scripted by
  /// [arrange], with the balance and activity readers kept alive.
  Future<({ProviderContainer c, FakeWalletSession fake})> ready({
    void Function(FakeWalletSession fake)? arrange,
    WalletSendAuthorizer? auth,
  }) async {
    final fake = FakeWalletSession()
      ..currentTransparentAddressResult = 't1myownTaddr';
    arrange?.call(fake);
    final c = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        if (auth != null) walletSendAuthorizerProvider.overrideWithValue(auth),
      ],
    );
    addTearDown(c.dispose);
    c.listen(walletSnapshotProvider, (_, _) {});
    c.listen(walletActivityProvider, (_, _) {});
    await ctl(c).start();
    await ctl(c).prepare('1');
    expect(st(c), isA<MoveReady>());
    return (c: c, fake: fake);
  }

  group('R13 row 3 — move: errors that may follow persistence → '
      'MoveOutcomeUnknown', () {
    for (final (name, thrown) in <(String, Object)>[
      ('StoreCorrupt', err(const WalletErrorKind.storeCorrupt())),
      (
        'InvalidState{Wiped}',
        err(const WalletErrorKind.invalidState(phase: LifecyclePhase.wiped)),
      ),
      ('an untyped throw', StateError('not a WalletApiError')),
    ]) {
      test('$name from send → MoveOutcomeUnknown, never "Couldn\'t complete" '
          '+ Retry', () async {
        final h = await ready(arrange: (f) => f.sendThrows = thrown);
        await ctl(h.c).confirm();
        expect(st(h.c), isA<MoveOutcomeUnknown>());
        expect(h.fake.sendCount, 1);
      });
    }

    test(
      'landing on unknown refreshes the balance AND the activity list',
      () async {
        final h = await ready(
          arrange: (f) =>
              f.sendThrows = err(const WalletErrorKind.storeCorrupt()),
        );
        await pumpEventQueue();
        final snapshots = h.fake.snapshotCount;
        final activity = h.fake.transactionsCount;
        await ctl(h.c).confirm();
        await pumpEventQueue();
        expect(st(h.c), isA<MoveOutcomeUnknown>());
        expect(h.fake.snapshotCount, greaterThan(snapshots), reason: 'balance');
        expect(
          h.fake.transactionsCount,
          greaterThan(activity),
          reason: 'activity',
        );
      },
    );
  });

  group('R13 row 3 — move: errors that provably precede persistence keep '
      'today\'s routes', () {
    for (final (name, kind, reason)
        in <(String, WalletErrorKind, SendFaultReason)>[
          (
            'DiskFull',
            const WalletErrorKind.diskFull(),
            SendFaultReason.storageFull,
          ),
          (
            'StoreBusy',
            const WalletErrorKind.storeBusy(),
            SendFaultReason.walletBusy,
          ),
          (
            'InvalidState{Closing} (row 1b)',
            const WalletErrorKind.invalidState(phase: LifecyclePhase.closing),
            SendFaultReason.walletBusy,
          ),
          (
            'ProposalStale',
            const WalletErrorKind.proposalStale(),
            SendFaultReason.amountsExpired,
          ),
        ]) {
      test('$name from send → MoveAmountEntry(${reason.name})', () async {
        final h = await ready(arrange: (f) => f.sendThrows = err(kind));
        await ctl(h.c).confirm();
        final s = st(h.c);
        expect(s, isA<MoveAmountEntry>());
        expect(
          ((s as MoveAmountEntry).fault as SendCategoricalFault).reason,
          reason,
        );
      });
    }

    test('SignFailed from send → MoveSent(SendSignFailed) (control)', () async {
      final h = await ready(
        arrange: (f) => f.sendThrows = err(const WalletErrorKind.signFailed()),
      );
      await ctl(h.c).confirm();
      expect((st(h.c) as MoveSent).outcome, isA<SendSignFailed>());
    });

    test('ProposalAlreadyUsed from send → MoveSent(SendAlreadySubmitted) '
        '(control)', () async {
      final h = await ready(
        arrange: (f) =>
            f.sendThrows = err(const WalletErrorKind.proposalAlreadyUsed()),
      );
      await ctl(h.c).confirm();
      expect((st(h.c) as MoveSent).outcome, isA<SendAlreadySubmitted>());
    });
  });

  group('R13 rows 1b + 3 — move: the host acts AFTER running the spend', () {
    for (final (name, thenThrow) in [
      ('throws', hostFaultAfter),
      ('declines', hostDeclineAfter),
      ('throws SessionChanged', hostSessionChangedAfter),
    ]) {
      test('the host runs the spend then $name → MoveOutcomeUnknown', () async {
        final auth = ScriptedAuthorizer(afterAction: thenThrow);
        final h = await ready(
          arrange: (f) =>
              f.sendThrows = err(const WalletErrorKind.signFailed()),
          auth: auth,
        );
        await ctl(h.c).confirm();
        expect(st(h.c), isA<MoveOutcomeUnknown>());
        expect(h.fake.sendCount, 1, reason: 'the spend really ran');
      });
    }

    test('a decline BEFORE the spend → back to the review (today)', () async {
      final h = await ready(
        auth: ScriptedAuthorizer(
          beforeAction: () => throw const WalletSpendAuthorizationDenied(),
        ),
      );
      final review = st(h.c);
      await ctl(h.c).confirm();
      expect(st(h.c), same(review));
      expect(h.fake.sendCount, 0);
    });

    test('SessionChanged BEFORE the spend → no bridge call and NOT unknown '
        '(today)', () async {
      final h = await ready(
        auth: ScriptedAuthorizer(
          beforeAction: () => throw const WalletSpendSessionChanged(),
        ),
      );
      await ctl(h.c).confirm();
      expect(h.fake.sendCount, 0);
      expect(st(h.c), isNot(isA<MoveOutcomeUnknown>()));
    });
  });

  group('R13 row 1b — move: a throw after `send` RETURNED its results shows '
      'the landed outcome, never unknown', () {
    for (final (name, thenThrow) in [
      ('an untyped host fault', hostFaultAfter),
      ('a decline', hostDeclineAfter),
      ('SessionChanged', hostSessionChangedAfter),
    ]) {
      test(
        '$name after a landed broadcast → MoveSent(SendSucceeded)',
        () async {
          final h = await ready(
            arrange: (f) =>
                f.sendResults = const [TxSubmitResult.success(txidHex: 'aa')],
            auth: ScriptedAuthorizer(afterAction: thenThrow),
          );
          await ctl(h.c).confirm();
          final s = st(h.c);
          expect(s, isA<MoveSent>());
          expect((s as MoveSent).outcome, isA<SendSucceeded>());
        },
      );
    }

    test('an untyped throw from the delivery read after a landed, unbroadcast '
        'move → MoveSent(SendKept)', () async {
      final h = await ready(
        arrange: (f) => f
          ..sendResults = const [TxSubmitResult.grpcFailure(txidHex: 'bb')]
          ..deliveryStateThrows = StateError('untyped delivery read'),
      );
      await ctl(h.c).confirm();
      final s = st(h.c);
      expect(s, isA<MoveSent>());
      expect((s as MoveSent).outcome, isA<SendKept>());
    });
  });

  group('R13 row 3 — the money row (controller): from MoveOutcomeUnknown no '
      'action proposes or sends again', () {
    test('prepare / confirm / retryLoad / backToForm are all no-ops', () async {
      final h = await ready(
        arrange: (f) =>
            f.sendThrows = err(const WalletErrorKind.storeCorrupt()),
      );
      await ctl(h.c).confirm();
      final unknown = st(h.c);
      expect(unknown, isA<MoveOutcomeUnknown>());
      final proposes = h.fake.proposeCount;
      final sends = h.fake.sendCount;

      await ctl(h.c).prepare('1');
      await ctl(h.c).confirm();
      await ctl(h.c).retryLoad();
      ctl(h.c).backToForm();

      expect(st(h.c), same(unknown));
      expect(h.fake.proposeCount, proposes, reason: 'no fresh propose');
      expect(h.fake.sendCount, sends, reason: 'no second send');
    });
  });

  group('R13 rows 3 + 4 — the move sheet\'s unknown view', () {
    Widget harness(FakeWalletSession session) => ProviderScope(
      overrides: [walletSessionProvider.overrideWithValue(session)],
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: TextButton(
                onPressed: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) =>
                        const Scaffold(body: MoveToTransparentSheet()),
                  ),
                ),
                child: const Text('open-move'),
              ),
            ),
          ),
        ),
      ),
    );

    /// Funded and synced so the sheet shows its amount form (the
    /// move_to_transparent_sheet_test `_funded` shape), `send` corrupting.
    FakeWalletSession corrupting() =>
        FakeWalletSession(current: const SyncStatus.upToDate(tip: 1))
          ..currentTransparentAddressResult = 't1myownTaddr'
          ..setSnapshot(
            walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 1),
              balance: balanceFixture(spendableZat: 1000000),
            ),
          )
          ..sendThrows = err(const WalletErrorKind.storeCorrupt());

    Future<WalletLocalizations> landUnknown(
      WidgetTester tester,
      FakeWalletSession session,
    ) async {
      await tester.pumpWidget(harness(session));
      await tester.tap(find.text('open-move'));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(MoveToTransparentSheet)),
      );
      await tester.enterText(find.byType(TextField), '0.01');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletMoveConfirmButton));
      await tester.pumpAndSettle();
      return l10n;
    }

    testWidgets('renders the move unknown title and body', (tester) async {
      final l10n = await landUnknown(tester, corrupting());
      expect(find.text(l10n.walletMoveUnknownTitle), findsOneWidget);
      expect(find.text(l10n.walletMoveUnknownBody), findsOneWidget);
    });

    testWidgets('offers no Retry and no "couldn\'t complete this move"', (
      tester,
    ) async {
      final l10n = await landUnknown(tester, corrupting());
      expect(find.text(l10n.walletMoveRetry), findsNothing);
      expect(find.text(l10n.walletMoveFailedTitle), findsNothing);
    });

    testWidgets('THE MONEY ROW: its only action closes the sheet, and no '
        'fresh propose starts', (tester) async {
      final session = corrupting();
      await landUnknown(tester, session);
      final proposes = session.proposeCount;
      final actions = find.byWidgetPredicate((w) => w is ButtonStyleButton);
      expect(actions, findsOneWidget, reason: 'Close only');
      await tester.tap(actions);
      await tester.pumpAndSettle();
      expect(find.byType(MoveToTransparentSheet), findsNothing);
      expect(session.proposeCount, proposes, reason: 'no fresh propose');
      expect(session.sendCount, 1, reason: 'no second send');
    });
  });
}
