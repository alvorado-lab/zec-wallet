import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_controller.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_controller.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_controller.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_state.dart';
import 'package:zec_wallet_ui/features/wallet/started_spend.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_activity_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

import '../../support/scripted_authorizer.dart';

/// R13 §4.2 — the started-spend bookkeeping's IDENTITY rule
/// (`docs/plan/r13-a-saved-transaction-is-never-offered-a-retry.md`).
///
/// Once the spend started, the precedes-persistence predicate grades ONLY the
/// SDK's own throw, by identity: `identical(error, sdkThrew) &&
/// precedesPersistence(error)` in `started_spend.dart`, and its parked twin in
/// `parked_sends_section.dart`. A host that runs the spend and then throws a
/// FRESH error of a kind the predicate accepts (`proposalAlreadyUsed`,
/// `signFailed`, `queuedSendsFull`, `seedMismatch`) must not borrow that
/// kind's "nothing was saved" classification. Every after-the-spend host throw
/// in the older files is a StateError / Denied / SessionChanged — none is on a
/// precedes list — so these are the rows that go RED if the clause is dropped.
void main() {
  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  /// The host runs the spend, then throws a FRESH [kind] of its own — never
  /// the SDK's object, whatever the SDK threw.
  Future<void> Function(Object?) hostThrowsFresh(WalletErrorKind kind) =>
      (_) => Future<void>.error(err(kind));

  group('runStartedSpend — the four verdicts', () {
    const atMostOnce = 'ran the spend twice (test)';

    /// Runs [authorize] through the bookkeeping with the send predicate.
    Future<StartedSpend<String>> run(
      Future<String> Function(SpendStart<String> start) authorize, {
      void Function()? onEntered,
    }) => runStartedSpend<String>(
      authorize: authorize,
      precedesPersistence: sendErrorPrecedesPersistence,
      atMostOnceMessage: atMostOnce,
      onEntered: onEntered,
    );

    test('an authorizer that throws before calling start is SpendNotStarted '
        'carrying that same error, and the SDK call never runs', () async {
      final declined = StateError('declined before the spend');
      var entered = 0;
      final verdict = await run((start) async {
        throw declined;
      }, onEntered: () => entered++);
      expect(verdict, isA<SpendNotStarted<String>>());
      expect(
        identical((verdict as SpendNotStarted<String>).error, declined),
        isTrue,
      );
      expect(entered, 0, reason: 'the SDK call was never entered');
    });

    test('an authorizer that returns without calling start is '
        'SpendNotStarted(null), whatever it returned', () async {
      final verdict = await run((start) async => 'fabricated-by-host');
      expect(verdict, isA<SpendNotStarted<String>>());
      expect((verdict as SpendNotStarted<String>).error, isNull);
    });

    test('a refusal thrown from beforeEnter is SpendNotStarted: the SDK call '
        'never runs and onEntered never fires', () async {
      final refusal = StateError('abandoned flow');
      var sdkRuns = 0;
      var entered = 0;
      final verdict = await run(
        (start) => start(() async {
          sdkRuns++;
          return 'txid';
        }, beforeEnter: () => throw refusal),
        onEntered: () => entered++,
      );
      expect(verdict, isA<SpendNotStarted<String>>());
      expect(
        identical((verdict as SpendNotStarted<String>).error, refusal),
        isTrue,
      );
      expect(sdkRuns, 0);
      expect(entered, 0);
    });

    test('an SDK call that returns is SpendLanded with its value and no '
        'errorAfter, and onEntered fired once', () async {
      var entered = 0;
      final verdict = await run(
        (start) => start(() async => 'sdk-value'),
        onEntered: () => entered++,
      );
      expect(verdict, isA<SpendLanded<String>>());
      final landed = verdict as SpendLanded<String>;
      expect(landed.value, 'sdk-value');
      expect(landed.errorAfter, isNull);
      expect(entered, 1);
    });

    test('a host that discards the SDK value and returns its own still gets '
        'the SDK value in SpendLanded', () async {
      final verdict = await run((start) async {
        await start(() async => 'sdk-value');
        return 'fabricated-by-host';
      });
      expect((verdict as SpendLanded<String>).value, 'sdk-value');
    });

    test('a throw after the SDK call landed is SpendLanded with that throw as '
        'errorAfter — the landing stands', () async {
      final hostFault = StateError('host bookkeeping failed');
      final verdict = await run((start) async {
        await start(() async => 'sdk-value');
        throw hostFault;
      });
      expect(verdict, isA<SpendLanded<String>>());
      final landed = verdict as SpendLanded<String>;
      expect(landed.value, 'sdk-value');
      expect(identical(landed.errorAfter, hostFault), isTrue);
    });

    test('the SDK\'s own accepted refusal, propagated untouched, is '
        'SpendRefusedBeforePersist carrying the SAME object', () async {
      final sdkError = err(const WalletErrorKind.signFailed());
      final verdict = await run((start) => start(() async => throw sdkError));
      expect(verdict, isA<SpendRefusedBeforePersist<String>>());
      expect(
        identical(
          (verdict as SpendRefusedBeforePersist<String>).error,
          sdkError,
        ),
        isTrue,
      );
    });

    test('the SDK\'s own accepted refusal caught and re-thrown as the same '
        'object by the host is still SpendRefusedBeforePersist', () async {
      final sdkError = err(const WalletErrorKind.proposalAlreadyUsed());
      final verdict = await run((start) async {
        try {
          return await start(() async => throw sdkError);
        } catch (e) {
          // A fresh `throw` of the caught object, not `rethrow`.
          // ignore: use_rethrow_when_possible
          throw e;
        }
      });
      expect(verdict, isA<SpendRefusedBeforePersist<String>>());
    });

    test('the SDK\'s own refusal of a kind the predicate rejects is '
        'SpendAnswerLost carrying that error', () async {
      final sdkError = err(const WalletErrorKind.storeCorrupt());
      final verdict = await run((start) => start(() async => throw sdkError));
      expect(verdict, isA<SpendAnswerLost<String>>());
      expect(
        identical((verdict as SpendAnswerLost<String>).error, sdkError),
        isTrue,
      );
    });

    test('IDENTITY: the SDK threw an accepted kind, the host then threw a '
        'DIFFERENT object the predicate also accepts → SpendAnswerLost, never '
        'SpendRefusedBeforePersist', () async {
      final sdkError = err(const WalletErrorKind.signFailed());
      final hostError = err(const WalletErrorKind.proposalAlreadyUsed());
      final verdict = await run((start) async {
        try {
          return await start(() async => throw sdkError);
        } catch (_) {
          throw hostError;
        }
      });
      expect(verdict, isA<SpendAnswerLost<String>>());
      expect(
        identical((verdict as SpendAnswerLost<String>).error, hostError),
        isTrue,
      );
    });

    test('IDENTITY: the SDK threw a rejected kind, the host then threw a fresh '
        'accepted kind → SpendAnswerLost', () async {
      final verdict = await run((start) async {
        try {
          return await start(
            () async => throw err(const WalletErrorKind.storeCorrupt()),
          );
        } catch (_) {
          throw err(const WalletErrorKind.proposalAlreadyUsed());
        }
      });
      expect(verdict, isA<SpendAnswerLost<String>>());
    });

    test(
      'IDENTITY: a host that re-throws an EQUAL copy (== but not '
      'identical) of the SDK\'s accepted refusal → SpendAnswerLost',
      () async {
        final sdkError = err(const WalletErrorKind.signFailed());
        final copy = err(const WalletErrorKind.signFailed());
        expect(copy == sdkError, isTrue, reason: 'precondition: value-equal');
        final verdict = await run((start) async {
          try {
            return await start(() async => throw sdkError);
          } catch (_) {
            throw copy;
          }
        });
        expect(verdict, isA<SpendAnswerLost<String>>());
      },
    );

    test('IDENTITY: the SDK call never threw (it is still pending when the '
        'host throws an accepted kind) → SpendAnswerLost', () async {
      // Never completed: the SDK call is in flight, not refused.
      final pending = Completer<String>();
      var entered = 0;
      final verdict = await run((start) async {
        // The spend is entered; the host abandons the future and throws.
        start(() => pending.future).ignore();
        throw err(const WalletErrorKind.signFailed());
      }, onEntered: () => entered++);
      expect(entered, 1, reason: 'precondition: the spend was entered');
      expect(verdict, isA<SpendAnswerLost<String>>());
    });

    test('an entered spend whose SDK error the host swallows (returning '
        'normally) is SpendAnswerLost(null)', () async {
      final verdict = await run((start) async {
        try {
          return await start(
            () async => throw err(const WalletErrorKind.signFailed()),
          );
        } catch (_) {
          return 'fabricated-by-host';
        }
      });
      expect(verdict, isA<SpendAnswerLost<String>>());
      expect((verdict as SpendAnswerLost<String>).error, isNull);
    });

    test('a second start throws StateError(atMostOnceMessage) and never runs '
        'the SDK call or beforeEnter again', () async {
      var sdkRuns = 0;
      var beforeRuns = 0;
      Object? second;
      final verdict = await run((start) async {
        final value = await start(() async {
          sdkRuns++;
          return 'sdk-value';
        }, beforeEnter: () => beforeRuns++);
        try {
          await start(() async {
            sdkRuns++;
            return 'second';
          }, beforeEnter: () => beforeRuns++);
        } catch (e) {
          second = e;
        }
        return value;
      });
      expect(second, isA<StateError>());
      expect((second! as StateError).message, atMostOnce);
      expect(sdkRuns, 1);
      expect(beforeRuns, 1);
      expect((verdict as SpendLanded<String>).value, 'sdk-value');
    });

    test('a second start the host lets propagate after a landing is '
        'SpendLanded with the StateError as errorAfter', () async {
      final verdict = await run((start) async {
        await start(() async => 'sdk-value');
        return start(() async => 'second');
      });
      expect(verdict, isA<SpendLanded<String>>());
      final landed = verdict as SpendLanded<String>;
      expect(landed.value, 'sdk-value');
      expect(landed.errorAfter, isA<StateError>());
      expect((landed.errorAfter! as StateError).message, atMostOnce);
    });
  });

  group('send confirm — the SDK throws StoreCorrupt, the host then throws a '
      'FRESH accepted kind', () {
    SendController ctl(ProviderContainer c) =>
        c.read(sendControllerProvider.notifier);
    SendState st(ProviderContainer c) => c.read(sendControllerProvider);

    ({ProviderContainer c, List<SendFlowOutcome> events}) harness(
      FakeWalletSession fake,
      WalletSendAuthorizer auth,
    ) {
      final c = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(c.dispose);
      final events = <SendFlowOutcome>[];
      c.listen<SendFlowOutcome?>(sendFlowOutcomeProvider, (_, next) {
        if (next != null) events.add(next);
      });
      return (c: c, events: events);
    }

    for (final (name, kind) in <(String, WalletErrorKind)>[
      ('proposalAlreadyUsed', const WalletErrorKind.proposalAlreadyUsed()),
      ('signFailed', const WalletErrorKind.signFailed()),
    ]) {
      test('a fresh $name from the host → SendOutcomeUnknown, not the '
          'classification of $name', () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.storeCorrupt());
        final h = harness(
          fake,
          ScriptedAuthorizer(afterAction: hostThrowsFresh(kind)),
        );
        await ctl(h.c).prepare(address: 'u1x', amountText: '1');
        await ctl(h.c).confirm();
        expect(fake.sendCount, 1, reason: 'the spend really ran');
        expect(st(h.c), isA<SendOutcomeUnknown>());
        expect((st(h.c) as SendOutcomeUnknown).queued, isFalse);
      });

      test(
        'a fresh $name from the host → the host hears '
        'SendFlowIndeterminate, never SendFlowSent(SendAlreadySubmitted)',
        () async {
          final fake = FakeWalletSession()
            ..sendThrows = err(const WalletErrorKind.storeCorrupt());
          final h = harness(
            fake,
            ScriptedAuthorizer(afterAction: hostThrowsFresh(kind)),
          );
          await ctl(h.c).prepare(address: 'u1x', amountText: '1');
          await ctl(h.c).confirm();
          expect(h.events.whereType<SendFlowIndeterminate>(), hasLength(1));
          expect(
            h.events.whereType<SendFlowSent>().where(
              (e) => e.outcome is SendAlreadySubmitted,
            ),
            isEmpty,
          );
        },
      );
    }
  });

  group('send queue — queueSend throws StoreCorrupt, the host then throws a '
      'FRESH queuedSendsFull', () {
    test('→ SendOutcomeUnknown(queued: true), never the retryable form '
        'fault a full queue gets', () async {
      final fake = FakeWalletSession()
        ..queueThrows = err(const WalletErrorKind.storeCorrupt());
      final c = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(
            ScriptedAuthorizer(
              afterAction: hostThrowsFresh(
                const WalletErrorKind.queuedSendsFull(),
              ),
            ),
          ),
        ],
      );
      addTearDown(c.dispose);
      await c
          .read(sendControllerProvider.notifier)
          .queueOffline(address: 'u1x', amountText: '1');
      expect(fake.queueCount, 1, reason: 'the enqueue really ran');
      final s = c.read(sendControllerProvider);
      expect(s, isA<SendOutcomeUnknown>());
      expect((s as SendOutcomeUnknown).queued, isTrue);
    });
  });

  group('shield — the SDK throws StoreCorrupt, the host then throws a FRESH '
      'accepted kind', () {
    ShieldController ctl(ProviderContainer c) =>
        c.read(shieldControllerProvider.notifier);
    ShieldState st(ProviderContainer c) => c.read(shieldControllerProvider);

    for (final (name, kind) in <(String, WalletErrorKind)>[
      ('proposalAlreadyUsed', const WalletErrorKind.proposalAlreadyUsed()),
      ('signFailed', const WalletErrorKind.signFailed()),
    ]) {
      test('a fresh $name from the host → ShieldOutcomeUnknown', () async {
        final fake = FakeWalletSession()
          ..proposeShieldResult = shieldProposalFixture(proposalId: 42)
          ..sendThrows = err(const WalletErrorKind.storeCorrupt());
        final c = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSendAuthorizerProvider.overrideWithValue(
              ScriptedAuthorizer(afterAction: hostThrowsFresh(kind)),
            ),
          ],
        );
        addTearDown(c.dispose);
        c.listen(walletSnapshotProvider, (_, _) {});
        c.listen(walletActivityProvider, (_, _) {});
        await ctl(c).prepare();
        expect(st(c), isA<ShieldReady>());
        await ctl(c).confirm();
        expect(fake.sendCount, 1, reason: 'the spend really ran');
        expect(st(c), isA<ShieldOutcomeUnknown>());
      });
    }
  });

  group('move to transparent — the SDK throws StoreCorrupt, the host then '
      'throws a FRESH accepted kind', () {
    MoveToTransparentController ctl(ProviderContainer c) =>
        c.read(moveToTransparentControllerProvider.notifier);
    MoveToTransparentState st(ProviderContainer c) =>
        c.read(moveToTransparentControllerProvider);

    for (final (name, kind) in <(String, WalletErrorKind)>[
      ('proposalAlreadyUsed', const WalletErrorKind.proposalAlreadyUsed()),
      ('signFailed', const WalletErrorKind.signFailed()),
    ]) {
      test('a fresh $name from the host → MoveOutcomeUnknown', () async {
        final fake = FakeWalletSession()
          ..currentTransparentAddressResult = 't1myownTaddr'
          ..sendThrows = err(const WalletErrorKind.storeCorrupt());
        final c = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSendAuthorizerProvider.overrideWithValue(
              ScriptedAuthorizer(afterAction: hostThrowsFresh(kind)),
            ),
          ],
        );
        addTearDown(c.dispose);
        c.listen(walletSnapshotProvider, (_, _) {});
        c.listen(walletActivityProvider, (_, _) {});
        await ctl(c).start();
        await ctl(c).prepare('1');
        expect(st(c), isA<MoveReady>());
        await ctl(c).confirm();
        expect(fake.sendCount, 1, reason: 'the spend really ran');
        expect(st(c), isA<MoveOutcomeUnknown>());
      });
    }
  });

  group('parked "Send now" — authorizeParkedSend throws StoreBusy, the host '
      'then throws a FRESH seedMismatch', () {
    setUp(() {
      final view =
          TestWidgetsFlutterBinding.instance.platformDispatcher.views.first;
      view.physicalSize = const Size(800, 1200) * view.devicePixelRatio;
    });
    tearDown(() {
      TestWidgetsFlutterBinding.instance.platformDispatcher.views.first
          .resetPhysicalSize();
    });

    testWidgets('the row is re-read (shown absent) → AlreadyInProgress, never '
        '"unchanged, try again" on the host\'s borrowed kind', (tester) async {
      final row = parkedSendFixture(id: 9, createdAt: 1700000456);
      final fake =
          FakeWalletSession(
              current: const SyncStatus.upToDate(tip: 100),
              snapshotValue: walletStateFixture(
                syncStatus: const SyncStatus.upToDate(tip: 100),
                balance: balanceFixture(
                  spendableZat: 1000000,
                  totalZat: 1000000,
                  transparentZat: 1000000,
                ),
              ),
            )
            ..parkedSendsResult = [row]
            ..authorizeParkedSendThrows = err(
              const WalletErrorKind.storeBusy(),
            );
      final auth = ScriptedAuthorizer(
        // Only a re-read can see this: the row left the parked list.
        beforeAction: () => fake.parkedSendsResult = const [],
        afterAction: hostThrowsFresh(const WalletErrorKind.seedMismatch()),
      );
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSendAuthorizerProvider.overrideWithValue(auth),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletScreen)),
      );
      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pumpAndSettle();
      expect(fake.authorizeParkedSendCount, 1, reason: 'the spend really ran');
      expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
      expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });
  });
}
