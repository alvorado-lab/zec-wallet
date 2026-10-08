import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';

import 'package:zec_wallet_ui/testing.dart';
import 'lifecycle_test_helpers.dart';

/// The incoming-funds provider contract (ADR-0536 #392). The lifecycle +
/// reconnect MACHINE is the [SyncStatusNotifier] copy and rides its exhaustive
/// suite; these tests pin what is NEW here — the cursor threading (catch-up on
/// reconnect/resume), the empty-cursor guard, and the per-build identity reset.
void main() {
  final binding = TestWidgetsFlutterBinding.ensureInitialized();

  void goPaused() {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
  }

  void goResumedFromPaused() {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  }

  const live130 = IncomingFundsEvent(
    kind: IncomingFundsEventKind.live,
    newTxCount: 2,
    totalTxDetected: 2,
    spanFromHeight: 100,
    spanToHeight: 120,
    cursor: 'w1:130',
  );

  Future<ProviderContainer> mount(
    WidgetTester tester,
    FakeWalletSession fake, {
    StateProvider<WalletSession?>? sessionSwitch,
  }) async {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    final container = ProviderContainer(
      overrides: [
        if (sessionSwitch == null)
          walletSessionProvider.overrideWithValue(fake)
        else
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(incomingFundsEventsProvider, (_, _) {});
    await tester.pump();
    return container;
  }

  testWidgets('subscribes with a NULL cursor and adopts the baseline replay', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    final container = await mount(tester, fake);

    expect(fake.incomingSubscribeCount, 1);
    expect(fake.incomingCursorsSeen, [null], reason: 'no cursor yet');
    final event = container.read(incomingFundsEventsProvider).value;
    expect(event?.kind, IncomingFundsEventKind.replay);
    expect(event?.cursor, 'w1:0');

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets(
    'a reconnect after a fault re-subscribes WITH the last event cursor '
    '(the catch-up contract — never a re-baseline)',
    (tester) async {
      final fake = FakeWalletSession(current: const SyncStatus.idle());
      final container = await mount(tester, fake);

      fake.pushIncoming(live130);
      await tester.pump();
      expect(
        container.read(incomingFundsEventsProvider).value?.cursor,
        'w1:130',
      );

      fake.emitIncomingError();
      await tester.pump();
      // The fault keeps the last event visible (never a UI error) …
      expect(
        container.read(incomingFundsEventsProvider).value?.cursor,
        'w1:130',
        reason: 'last event survives the fault',
      );
      // … and the backoff reconnect passes the remembered cursor back.
      await tester.pump(SyncStatusNotifier.minBackoff);
      expect(fake.incomingSubscribeCount, 2);
      expect(fake.incomingCursorsSeen, [null, 'w1:130']);

      goResumedFromPausedSafeReset(binding);
    },
  );

  testWidgets(
    'paused cancels; a real resume re-subscribes with the cursor, so the '
    'replay covers exactly the backgrounded gap',
    (tester) async {
      final fake = FakeWalletSession(current: const SyncStatus.idle());
      await mount(tester, fake);
      fake.pushIncoming(live130);
      await tester.pump();

      goPaused();
      await tester.pump();
      expect(fake.incomingCancelCount, 1);
      expect(fake.hasActiveIncomingSubscription, isFalse);

      goResumedFromPaused();
      await tester.pump();
      expect(fake.incomingSubscribeCount, 2);
      expect(
        fake.incomingCursorsSeen.last,
        'w1:130',
        reason: 'the resume catch-up replays from the remembered watermark',
      );
    },
  );

  testWidgets(
    'an empty memoRefresh cursor never clobbers the remembered watermark',
    (tester) async {
      final fake = FakeWalletSession(current: const SyncStatus.idle());
      await mount(tester, fake);
      fake.pushIncoming(live130);
      await tester.pump();

      // A HOSTILE empty-cursor event (the core pump never delivers one
      // post--M2 — this pins the provider's own defensive guard).
      fake.pushIncoming(
        const IncomingFundsEvent(
          kind: IncomingFundsEventKind.memoRefresh,
          newTxCount: 0,
          totalTxDetected: 2,
          cursor: '',
        ),
      );
      await tester.pump();

      fake.emitIncomingError();
      await tester.pump(SyncStatusNotifier.minBackoff);
      expect(
        fake.incomingCursorsSeen.last,
        'w1:130',
        reason: 'the empty cursor is ignored, not adopted',
      );

      goResumedFromPausedSafeReset(binding);
    },
  );

  testWidgets(
    'a session flip resets the cursor — the next identity never inherits the '
    'previous watermark (#381 cross-identity retention rule)',
    (tester) async {
      final fake1 = FakeWalletSession(current: const SyncStatus.idle());
      final fake2 = FakeWalletSession(current: const SyncStatus.idle());
      final sessionSwitch = StateProvider<WalletSession?>((_) => fake1);
      final container = await mount(
        tester,
        fake1,
        sessionSwitch: sessionSwitch,
      );

      fake1.pushIncoming(live130);
      await tester.pump();

      container.read(sessionSwitch.notifier).state = fake2;
      await tester.pump();

      expect(fake2.incomingSubscribeCount, 1);
      expect(
        fake2.incomingCursorsSeen,
        [null],
        reason: 'the fresh identity subscribes from scratch',
      );

      goResumedFromPausedSafeReset(binding);
    },
  );

  testWidgets(
    'a subscribe fault schedules a backoff retry, not a dead stream',
    (tester) async {
      final fake = FakeWalletSession(current: const SyncStatus.idle())
        ..failOnIncomingSubscribe = true;
      final container = await mount(tester, fake);
      expect(fake.incomingSubscribeCount, 1);

      fake.failOnIncomingSubscribe = false;
      await tester.pump(SyncStatusNotifier.minBackoff);
      expect(fake.incomingSubscribeCount, 2, reason: 'retried after backoff');
      await tester.pump();
      expect(
        container.read(incomingFundsEventsProvider).value?.kind,
        IncomingFundsEventKind.replay,
        reason: 'the retry delivered the baseline',
      );

      goResumedFromPausedSafeReset(binding);
    },
  );
}
