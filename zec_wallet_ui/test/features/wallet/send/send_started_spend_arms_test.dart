import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_controller.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/testing.dart';

import '../../../support/scripted_authorizer.dart';

/// R13 §4.5 row 1b, the SEND half — every catch arm checks whether the spend
/// started (`docs/plan/r13-a-saved-transaction-is-never-offered-a-retry.md`
/// §4.2). The S7 U1 group in `send_controller_test.dart` pins the generic
/// catch and the Denied arm; these pin the arms it did not reach:
/// `WalletSpendSessionChanged` thrown by the host AFTER the spend ran (confirm
/// and queue), the host event published even when the identity fence moved the
/// state, and a throw that arrives after `send` already returned its results.
void main() {
  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  SendController ctl(ProviderContainer c) =>
      c.read(sendControllerProvider.notifier);
  SendState st(ProviderContainer c) => c.read(sendControllerProvider);

  /// A container over [fake] with [auth], recording every published flow
  /// terminal (the channel keeps only the last; the rows ask about all).
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

  group('R13 row 1b — send: the host throws SessionChanged AFTER the spend', () {
    test(
      'confirm → SendOutcomeUnknown, never a Submitting left hanging',
      () async {
        // `send` itself refused with a pre-persistence kind, and the host then
        // replaced that answer with the identity-switch type: the caught error is
        // not the SDK's, so the answer is lost.
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.signFailed());
        final h = harness(
          fake,
          ScriptedAuthorizer(afterAction: hostSessionChangedAfter),
        );
        await ctl(h.c).prepare(address: 'u1x', amountText: '1');
        await ctl(h.c).confirm();
        expect(st(h.c), isA<SendOutcomeUnknown>());
        expect((st(h.c) as SendOutcomeUnknown).queued, isFalse);
        expect(fake.sendCount, 1, reason: 'the spend really ran');
      },
    );

    test(
      'confirm → the host hears Indeterminate, never NothingCreated',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.signFailed());
        final h = harness(
          fake,
          ScriptedAuthorizer(afterAction: hostSessionChangedAfter),
        );
        await ctl(h.c).prepare(address: 'u1x', amountText: '1');
        await ctl(h.c).confirm();
        expect(h.events.whereType<SendFlowIndeterminate>(), hasLength(1));
        expect(h.events.whereType<SendFlowNothingCreated>(), isEmpty);
      },
    );

    test('queueOffline → SendOutcomeUnknown(queued) + Indeterminate, never '
        'NothingCreated over an intent that may be queued', () async {
      // queueSend refused with a pre-insert kind; the host then threw the
      // identity-switch type over it.
      final fake = FakeWalletSession()
        ..queueThrows = err(const WalletErrorKind.queuedSendsFull());
      final h = harness(
        fake,
        ScriptedAuthorizer(afterAction: hostSessionChangedAfter),
      );
      await ctl(h.c).queueOffline(address: 'u1x', amountText: '1');
      expect(st(h.c), isA<SendOutcomeUnknown>());
      expect((st(h.c) as SendOutcomeUnknown).queued, isTrue);
      expect(h.events.whereType<SendFlowIndeterminate>(), hasLength(1));
      expect(h.events.whereType<SendFlowNothingCreated>(), isEmpty);
      expect(fake.queueCount, 1, reason: 'the enqueue really ran');
    });

    test('BEFORE the spend (the fence\'s own throw): still NothingCreated and '
        'no bridge call — today\'s behaviour', () async {
      final fake = FakeWalletSession();
      final h = harness(
        fake,
        ScriptedAuthorizer(
          beforeAction: () => throw const WalletSpendSessionChanged(),
        ),
      );
      await ctl(h.c).prepare(address: 'u1x', amountText: '1');
      await ctl(h.c).confirm();
      expect(fake.sendCount, 0);
      expect(h.events.whereType<SendFlowNothingCreated>(), hasLength(1));
      expect(st(h.c), isNot(isA<SendOutcomeUnknown>()));
    });

    test('BEFORE the spend (a decline): back to the live Review, nothing '
        'published — today\'s behaviour', () async {
      final fake = FakeWalletSession();
      final h = harness(
        fake,
        ScriptedAuthorizer(
          beforeAction: () => throw const WalletSpendAuthorizationDenied(),
        ),
      );
      await ctl(h.c).prepare(address: 'u1x', amountText: '1');
      await ctl(h.c).confirm();
      expect(fake.sendCount, 0);
      expect(st(h.c), isA<SendReview>());
      expect(h.events, isEmpty);
    });
  });

  group('R13 row 1b — send: Indeterminate is published BEFORE the identity '
      'guard, so it reaches the host even when the fence moved the state', () {
    // The host runs the spend, the wallet identity flips (build() resets the
    // machine to a fresh form), then the host throws. The state write is
    // rightly refused; the host's event must not be.
    ({ProviderContainer c, List<SendFlowOutcome> events}) flippable(
      FakeWalletSession first,
      FakeWalletSession second,
      Future<void> Function(Object?) thenThrow,
    ) {
      final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
      late final ProviderContainer c;
      final auth = ScriptedAuthorizer(
        afterAction: (actionError) async {
          c.read(sessionSwitch.notifier).state = second;
          await Future<void>.delayed(Duration.zero);
          await thenThrow(actionError);
        },
      );
      c = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(c.dispose);
      c.listen(sendControllerProvider, (_, _) {});
      final events = <SendFlowOutcome>[];
      c.listen<SendFlowOutcome?>(sendFlowOutcomeProvider, (_, next) {
        if (next != null) events.add(next);
      });
      return (c: c, events: events);
    }

    for (final (name, thenThrow) in [
      ('SessionChanged', hostSessionChangedAfter),
      ('a decline', hostDeclineAfter),
      ('an untyped host fault', hostFaultAfter),
    ]) {
      test('confirm, then $name after the flip → Indeterminate, no '
          'NothingCreated', () async {
        final a = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.signFailed());
        final b = FakeWalletSession();
        final h = flippable(a, b, thenThrow);
        await ctl(h.c).prepare(address: 'u1x', amountText: '1');
        await ctl(h.c).confirm();
        expect(a.sendCount, 1, reason: 'the spend ran on the first identity');
        expect(h.events.whereType<SendFlowIndeterminate>(), hasLength(1));
        expect(h.events.whereType<SendFlowNothingCreated>(), isEmpty);
      });

      test('queueOffline, then $name after the flip → Indeterminate, no '
          'NothingCreated', () async {
        final a = FakeWalletSession()
          ..queueThrows = err(const WalletErrorKind.queuedSendsFull());
        final b = FakeWalletSession();
        final h = flippable(a, b, thenThrow);
        await ctl(h.c).queueOffline(address: 'u1x', amountText: '1');
        expect(
          a.queueCount,
          1,
          reason: 'the enqueue ran on the first identity',
        );
        expect(h.events.whereType<SendFlowIndeterminate>(), hasLength(1));
        expect(h.events.whereType<SendFlowNothingCreated>(), isEmpty);
      });
    }
  });

  group('R13 row 1b — send: a throw after `send` RETURNED its results shows '
      'the landed outcome, never unknown', () {
    for (final (name, thenThrow) in [
      ('an untyped host fault', hostFaultAfter),
      ('a decline', hostDeclineAfter),
      ('SessionChanged', hostSessionChangedAfter),
    ]) {
      test(
        '$name after a landed broadcast → SendSent(SendSucceeded)',
        () async {
          final fake = FakeWalletSession()
            ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
          final h = harness(fake, ScriptedAuthorizer(afterAction: thenThrow));
          await ctl(h.c).prepare(address: 'u1x', amountText: '1');
          await ctl(h.c).confirm();
          final s = st(h.c);
          expect(s, isA<SendSent>());
          expect((s as SendSent).outcome, isA<SendSucceeded>());
        },
      );

      test('$name after a landed broadcast → SendFlowSent with the landed '
          'txids is published, not Indeterminate', () async {
        final fake = FakeWalletSession()
          ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
        final h = harness(fake, ScriptedAuthorizer(afterAction: thenThrow));
        await ctl(h.c).prepare(address: 'u1x', amountText: '1');
        await ctl(h.c).confirm();
        final sent = h.events.whereType<SendFlowSent>().toList();
        expect(sent, hasLength(1));
        expect(sent.single.txids, ['aa']);
        expect(h.events.whereType<SendFlowIndeterminate>(), isEmpty);
      });
    }

    test('an untyped throw from the delivery read after a landed, unbroadcast '
        'tx → SendSent(SendKept) (null delivery), never unknown', () async {
      // `deliveryStatesFor` catches only WalletApiError; the result is in hand.
      final fake = FakeWalletSession()
        ..sendResults = const [TxSubmitResult.grpcFailure(txidHex: 'bb')]
        ..deliveryStateThrows = StateError('untyped delivery read');
      final h = harness(fake, ScriptedAuthorizer());
      await ctl(h.c).prepare(address: 'u1x', amountText: '1');
      await ctl(h.c).confirm();
      final s = st(h.c);
      expect(s, isA<SendSent>());
      expect((s as SendSent).outcome, isA<SendKept>());
      expect(h.events.whereType<SendFlowSent>().single.txids, ['bb']);
    });
  });

  group('R13 row 1b — send: InvalidState split by phase', () {
    test(
      '{Wiped} from send → SendOutcomeUnknown (may follow persistence)',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(
            const WalletErrorKind.invalidState(phase: LifecyclePhase.wiped),
          );
        final h = harness(fake, ScriptedAuthorizer());
        await ctl(h.c).prepare(address: 'u1x', amountText: '1');
        await ctl(h.c).confirm();
        expect(st(h.c), isA<SendOutcomeUnknown>());
      },
    );
  });
}
