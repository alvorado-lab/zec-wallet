import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_controller.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_activity_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

import '../../../support/scripted_authorizer.dart';

/// R13 §4.5 rows 1b, 2 and 4 for SHIELD
/// (`docs/plan/r13-a-saved-transaction-is-never-offered-a-retry.md`).
///
/// A shield's `send` persists the transaction in one all-or-nothing engine
/// write and only then broadcasts. An error this flow cannot prove came BEFORE
/// that write — `StoreCorrupt` (`raw_tx_bytes`), `InvalidState{Wiped}` (the
/// wipe-mid-send check), any untyped throw, anything a host does after running
/// the spend — must land on [ShieldOutcomeUnknown] ("check before shielding
/// again", Close only), never on "Couldn't shield" + Try again. The kinds that
/// provably precede it keep today's routes.
void main() {
  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  ShieldController ctl(ProviderContainer c) =>
      c.read(shieldControllerProvider.notifier);
  ShieldState st(ProviderContainer c) => c.read(shieldControllerProvider);

  /// A container at [ShieldReady] over a fake whose `send` is scripted by
  /// [arrange], with the balance and activity readers kept alive (as the
  /// wallet screen beneath the sheet keeps them).
  Future<({ProviderContainer c, FakeWalletSession fake})> ready({
    void Function(FakeWalletSession fake)? arrange,
    WalletSendAuthorizer? auth,
  }) async {
    final fake = FakeWalletSession()
      ..proposeShieldResult = shieldProposalFixture(proposalId: 42);
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
    await ctl(c).prepare();
    expect(st(c), isA<ShieldReady>());
    return (c: c, fake: fake);
  }

  group('R13 row 2 — shield: errors that may follow persistence → '
      'ShieldOutcomeUnknown', () {
    for (final (name, thrown) in <(String, Object)>[
      ('StoreCorrupt', err(const WalletErrorKind.storeCorrupt())),
      (
        'InvalidState{Wiped}',
        err(const WalletErrorKind.invalidState(phase: LifecyclePhase.wiped)),
      ),
      ('an untyped throw', StateError('not a WalletApiError')),
    ]) {
      test('$name from send → ShieldOutcomeUnknown, never SendSignFailed / '
          'busy + Try again', () async {
        final h = await ready(arrange: (f) => f.sendThrows = thrown);
        await ctl(h.c).confirm();
        expect(st(h.c), isA<ShieldOutcomeUnknown>());
        expect(h.fake.sendCount, 1);
      });
    }

    test('landing on unknown refreshes the balance AND the activity list '
        '(the truth the user is pointed at)', () async {
      final h = await ready(
        arrange: (f) =>
            f.sendThrows = err(const WalletErrorKind.storeCorrupt()),
      );
      await pumpEventQueue();
      final snapshots = h.fake.snapshotCount;
      final activity = h.fake.transactionsCount;
      await ctl(h.c).confirm();
      await pumpEventQueue();
      expect(st(h.c), isA<ShieldOutcomeUnknown>());
      expect(h.fake.snapshotCount, greaterThan(snapshots), reason: 'balance');
      expect(
        h.fake.transactionsCount,
        greaterThan(activity),
        reason: 'activity',
      );
    });
  });

  group('R13 row 2 — shield: errors that provably precede persistence keep '
      'today\'s routes', () {
    for (final (name, kind, reason)
        in <(String, WalletErrorKind, ShieldFaultReason)>[
          (
            'DiskFull',
            const WalletErrorKind.diskFull(),
            ShieldFaultReason.storageFull,
          ),
          (
            'StoreBusy',
            const WalletErrorKind.storeBusy(),
            ShieldFaultReason.walletBusy,
          ),
          (
            'InvalidState{Closing} (row 1b)',
            const WalletErrorKind.invalidState(phase: LifecyclePhase.closing),
            ShieldFaultReason.walletBusy,
          ),
          (
            'ProposalStale',
            const WalletErrorKind.proposalStale(),
            ShieldFaultReason.notSyncedYet,
          ),
        ]) {
      test('$name from send → ShieldUnavailable(${reason.name})', () async {
        final h = await ready(arrange: (f) => f.sendThrows = err(kind));
        await ctl(h.c).confirm();
        final s = st(h.c);
        expect(s, isA<ShieldUnavailable>());
        expect((s as ShieldUnavailable).reason, reason);
      });
    }

    test(
      'SignFailed from send → ShieldDone(SendSignFailed) (control)',
      () async {
        final h = await ready(
          arrange: (f) =>
              f.sendThrows = err(const WalletErrorKind.signFailed()),
        );
        await ctl(h.c).confirm();
        expect((st(h.c) as ShieldDone).outcome, isA<SendSignFailed>());
      },
    );

    test('ProposalAlreadyUsed from send → ShieldDone(SendAlreadySubmitted) '
        '(control: the short-circuit stays)', () async {
      final h = await ready(
        arrange: (f) =>
            f.sendThrows = err(const WalletErrorKind.proposalAlreadyUsed()),
      );
      await ctl(h.c).confirm();
      expect((st(h.c) as ShieldDone).outcome, isA<SendAlreadySubmitted>());
    });
  });

  group('R13 rows 1b + 2 — shield: the host acts AFTER running the spend', () {
    // `send` refused with a kind that is on the single-step list; the host
    // then replaced that answer with its own. What reaches the catch is not
    // the SDK's own throw, so it proves nothing.
    for (final (name, thenThrow) in [
      ('throws', hostFaultAfter),
      ('declines', hostDeclineAfter),
      ('throws SessionChanged', hostSessionChangedAfter),
    ]) {
      test(
        'the host runs the spend then $name → ShieldOutcomeUnknown',
        () async {
          final auth = ScriptedAuthorizer(afterAction: thenThrow);
          final h = await ready(
            arrange: (f) =>
                f.sendThrows = err(const WalletErrorKind.signFailed()),
            auth: auth,
          );
          await ctl(h.c).confirm();
          expect(st(h.c), isA<ShieldOutcomeUnknown>());
          expect(auth.actionRuns, 1);
          expect(h.fake.sendCount, 1, reason: 'the spend really ran');
        },
      );
    }

    test(
      'a decline BEFORE the spend → back to the confirm sheet (today)',
      () async {
        final h = await ready(
          auth: ScriptedAuthorizer(
            beforeAction: () => throw const WalletSpendAuthorizationDenied(),
          ),
        );
        final readyState = st(h.c);
        await ctl(h.c).confirm();
        expect(st(h.c), same(readyState));
        expect(h.fake.sendCount, 0);
      },
    );

    test('SessionChanged BEFORE the spend → no bridge call and NOT unknown '
        '(today)', () async {
      final h = await ready(
        auth: ScriptedAuthorizer(
          beforeAction: () => throw const WalletSpendSessionChanged(),
        ),
      );
      await ctl(h.c).confirm();
      expect(h.fake.sendCount, 0);
      expect(st(h.c), isNot(isA<ShieldOutcomeUnknown>()));
    });
  });

  group('R13 row 1b — shield: a throw after `send` RETURNED its results '
      'shows the landed outcome, never unknown', () {
    for (final (name, thenThrow) in [
      ('an untyped host fault', hostFaultAfter),
      ('a decline', hostDeclineAfter),
      ('SessionChanged', hostSessionChangedAfter),
    ]) {
      test(
        '$name after a landed broadcast → ShieldDone(SendSucceeded)',
        () async {
          final h = await ready(
            arrange: (f) =>
                f.sendResults = const [TxSubmitResult.success(txidHex: 'aa')],
            auth: ScriptedAuthorizer(afterAction: thenThrow),
          );
          await ctl(h.c).confirm();
          final s = st(h.c);
          expect(s, isA<ShieldDone>());
          expect((s as ShieldDone).outcome, isA<SendSucceeded>());
        },
      );
    }

    test('an untyped throw from the delivery read after a landed, unbroadcast '
        'shield → ShieldDone(SendKept)', () async {
      final h = await ready(
        arrange: (f) => f
          ..sendResults = const [TxSubmitResult.grpcFailure(txidHex: 'bb')]
          ..deliveryStateThrows = StateError('untyped delivery read'),
      );
      await ctl(h.c).confirm();
      final s = st(h.c);
      expect(s, isA<ShieldDone>());
      expect((s as ShieldDone).outcome, isA<SendKept>());
    });
  });

  group('R13 row 4 — the shield sheet\'s unknown view', () {
    /// The sheet pushed as a route over a base screen, so Close can be
    /// observed actually closing it.
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
                    builder: (_) => const Scaffold(body: ShieldSheet()),
                  ),
                ),
                child: const Text('open-shield'),
              ),
            ),
          ),
        ),
      ),
    );

    Future<WalletLocalizations> landUnknown(
      WidgetTester tester,
      FakeWalletSession session,
    ) async {
      await tester.pumpWidget(harness(session));
      await tester.tap(find.text('open-shield'));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ShieldSheet)),
      );
      await tester.tap(find.text(l10n.walletShieldConfirmButton));
      await tester.pumpAndSettle();
      return l10n;
    }

    FakeWalletSession corrupting() => FakeWalletSession()
      ..proposeShieldResult = shieldProposalFixture(proposalId: 42)
      ..sendThrows = err(const WalletErrorKind.storeCorrupt());

    testWidgets('renders the shield unknown title and body', (tester) async {
      final l10n = await landUnknown(tester, corrupting());
      expect(find.text(l10n.walletShieldUnknownTitle), findsOneWidget);
      expect(find.text(l10n.walletShieldUnknownBody), findsOneWidget);
    });

    testWidgets('offers no Try again and no "couldn\'t shield"', (
      tester,
    ) async {
      final l10n = await landUnknown(tester, corrupting());
      expect(find.text(l10n.walletShieldRetry), findsNothing);
      expect(find.text(l10n.walletShieldFailedTitle), findsNothing);
    });

    testWidgets('its ONLY action closes the sheet, and nothing re-proposes', (
      tester,
    ) async {
      final session = corrupting();
      await landUnknown(tester, session);
      final actions = find.byWidgetPredicate((w) => w is ButtonStyleButton);
      expect(actions, findsOneWidget, reason: 'Close only');
      await tester.tap(actions);
      await tester.pumpAndSettle();
      expect(find.byType(ShieldSheet), findsNothing);
      expect(session.proposeShieldCount, 1, reason: 'no fresh propose');
      expect(session.sendCount, 1, reason: 'no second send');
    });
  });
}
