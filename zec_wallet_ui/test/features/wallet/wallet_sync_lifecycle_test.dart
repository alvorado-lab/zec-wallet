import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';
import 'lifecycle_test_helpers.dart';

/// The live-sync lifecycle contract (spec §3.3; flutter-patterns § Stream
/// Lifecycle; the rule in core/lifecycle/app_lifecycle_provider). Driven
/// against a fake session on the host VM — deterministic, no native lib, no
/// network. `testWidgets` gives the fake-async clock so reconnect backoff is
/// advanced explicitly (testing-patterns: no wall-clock sleeps).
void main() {
  final binding = TestWidgetsFlutterBinding.ensureInitialized();

  // Legal lifecycle walks (AppLifecycleListener asserts on transitions):
  // the state machine is a linear chain resumed↔inactive↔hidden↔paused.
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

  Future<ProviderContainer> mount(
    WidgetTester tester,
    FakeWalletSession fake,
  ) async {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    // The screen watches both providers; keep them alive so build() runs and
    // a resume-time invalidate actually recomputes the snapshot.
    container.listen(syncStatusProvider, (_, _) {});
    container.listen(walletSnapshotProvider, (_, _) {});
    await tester.pump();
    return container;
  }

  testWidgets('subscribes on build and emits the current status', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.upToDate(tip: 9));
    final container = await mount(tester, fake);

    expect(fake.subscribeCount, 1);
    expect(
      container.read(syncStatusProvider).value,
      isA<SyncStatus_UpToDate>(),
    );

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('paused cancels the live Rust-side stream (battery)', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1,
        to: 10,
        percent: 0.5,
        spendableReady: false,
        rewound: false,
      ),
    );
    final container = await mount(tester, fake);
    expect(fake.hasActiveSubscription, isTrue);

    goPaused();
    await tester.pump();

    expect(fake.cancelCount, 1);
    expect(fake.hasActiveSubscription, isFalse);
    // A push while paused reaches no one; the last status stays put.
    fake.push(const SyncStatus.upToDate(tip: 10));
    await tester.pump();
    expect(
      container.read(syncStatusProvider).value,
      isA<SyncStatus_Scanning>(),
    );

    goResumedFromPaused();
  });

  testWidgets('a real paused→resumed re-subscribes AND re-snapshots', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    final container = await mount(tester, fake);
    expect(fake.subscribeCount, 1);
    expect(fake.snapshotCount, 1); // the cold read at mount

    goPaused();
    await tester.pump();
    goResumedFromPaused();
    await tester.pump();

    expect(fake.subscribeCount, 2, reason: 're-subscribed for live deltas');
    expect(fake.snapshotCount, 2, reason: 'cold re-read caught background gap');
    expect(container.read(syncStatusProvider).value, isNotNull);
  });

  testWidgets('inactive→resumed (window focus) is NOT a real resume', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    final container = await mount(tester, fake);

    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();

    expect(fake.subscribeCount, 1, reason: 'no wasteful re-subscribe');
    expect(fake.snapshotCount, 1, reason: 'no wasteful re-fetch');
    expect(fake.hasActiveSubscription, isTrue);
    container.read(syncStatusProvider); // keep analyzer happy on `container`
  });

  testWidgets('reaching hidden without pause then resuming is NOT a resume', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    await mount(tester, fake);

    // Android can pass through hidden during shade pulls without ever
    // pausing; coming back must not re-subscribe.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();

    expect(fake.subscribeCount, 1);
    expect(fake.snapshotCount, 1);
  });

  testWidgets('desktop keeps the stream live while hidden (never pauses)', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    final container = await mount(tester, fake);

    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    await tester.pump();

    // hidden is the desktop deepest state — the stream MUST stay live.
    expect(fake.cancelCount, 0);
    expect(fake.hasActiveSubscription, isTrue);
    fake.push(const SyncStatus.upToDate(tip: 5));
    await tester.pump();
    expect(
      container.read(syncStatusProvider).value,
      isA<SyncStatus_UpToDate>(),
    );

    // restore foreground for a clean exit
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });

  testWidgets('a Stalled fault is DATA — the stream is not torn down', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    final container = await mount(tester, fake);

    fake.push(const SyncStatus.stalled(reason: StallReason.torUnavailable));
    await tester.pump();

    // A stall is renderable data, NOT an error, NOT a reconnect trigger.
    final stalled = container.read(syncStatusProvider);
    expect(stalled.hasError, isFalse);
    expect(stalled.value, isA<SyncStatus_Stalled>());
    expect(fake.cancelCount, 0, reason: 'fault-as-data never cancels');
    expect(fake.subscribeCount, 1, reason: 'no reconnect on a stall');

    // …and the same subscription carries the recovery.
    fake.push(const SyncStatus.upToDate(tip: 7));
    await tester.pump();
    expect(
      container.read(syncStatusProvider).value,
      isA<SyncStatus_UpToDate>(),
    );

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('a transport ERROR reconnects after backoff, keeping last good', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1,
        to: 10,
        percent: 0.3,
        spendableReady: false,
        rewound: false,
      ),
    );
    final container = await mount(tester, fake);

    fake.emitError();
    await tester.pump();

    // The error is swallowed: the last good status stays visible, the
    // subscription is gone, a reconnect is pending (not yet fired).
    expect(container.read(syncStatusProvider).hasError, isFalse);
    expect(
      container.read(syncStatusProvider).value,
      isA<SyncStatus_Scanning>(),
    );
    expect(fake.subscribeCount, 1);

    await tester.pump(SyncStatusNotifier.minBackoff);
    expect(fake.subscribeCount, 2, reason: 'reconnected after the backoff');

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('stream completion (EOF) reconnects after backoff', (
    tester,
  ) async {
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    await mount(tester, fake);

    fake.complete();
    await tester.pump();
    expect(fake.subscribeCount, 1, reason: 'no instant respin');

    await tester.pump(SyncStatusNotifier.minBackoff);
    expect(fake.subscribeCount, 2);

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('reconnect backoff grows and caps (no flaky-link hammering)', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.idle(),
      failOnSubscribe: true,
    );
    await mount(tester, fake);

    // Build subscribed once and immediately failed → reconnect pending.
    expect(fake.subscribeCount, 1);

    // Gaps double: 2s, 4s, 8s, 16s, then capped at 30s.
    await tester.pump(SyncStatusNotifier.minBackoff); // t=2
    expect(fake.subscribeCount, 2);
    await tester.pump(const Duration(seconds: 4)); // t=6
    expect(fake.subscribeCount, 3);
    await tester.pump(const Duration(seconds: 8)); // t=14
    expect(fake.subscribeCount, 4);
    await tester.pump(const Duration(seconds: 16)); // t=30
    expect(fake.subscribeCount, 5);

    // Cap proven: the next attempt waits the FULL ceiling, not a hot loop.
    await tester.pump(
      SyncStatusNotifier.maxBackoff - const Duration(seconds: 1),
    );
    expect(fake.subscribeCount, 5, reason: 'still waiting the capped delay');
    await tester.pump(const Duration(seconds: 1)); // t=60
    expect(fake.subscribeCount, 6);
    await tester.pump(SyncStatusNotifier.maxBackoff); // t=90
    expect(fake.subscribeCount, 7, reason: 'second gap is also the cap');

    // Recovery: once the endpoint is healthy the next capped attempt
    // succeeds, emits data, and schedules NO further reconnect — proving the
    // loop self-heals (and leaving no pending timer at teardown).
    fake.failOnSubscribe = false;
    await tester.pump(SyncStatusNotifier.maxBackoff); // t=120
    expect(fake.subscribeCount, 8);
    expect(fake.hasActiveSubscription, isTrue);

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('backgrounding mid-backoff cancels the pending reconnect', (
    tester,
  ) async {
    // Real mobile edge: a transport drop on a flaky link, then the user
    // backgrounds before the reconnect fires. The pending reconnect MUST be
    // cancelled — no background network, no battery drain.
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    await mount(tester, fake);

    fake.emitError(); // schedules a reconnect at minBackoff
    await tester.pump();
    expect(fake.subscribeCount, 1);

    goPaused(); // background BEFORE the reconnect timer fires
    await tester.pump();
    // Advance well past the backoff window: nothing reconnects while paused.
    await tester.pump(SyncStatusNotifier.minBackoff * 3);
    expect(fake.subscribeCount, 1, reason: 'no reconnect while backgrounded');

    // Resume re-subscribes (the recovery the paused gate deferred).
    goResumedFromPaused();
    await tester.pumpAndSettle();
    expect(fake.subscribeCount, 2);
  });

  testWidgets('born backgrounded: no stream until the first real resume', (
    tester,
  ) async {
    // Android push-trampoline cold start: the process begins life paused.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);

    final fake = FakeWalletSession(current: const SyncStatus.idle());
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(syncStatusProvider, (_, _) {});
    container.listen(walletSnapshotProvider, (_, _) {});
    await tester.pump();

    // Built while paused → the stream is NOT opened (no background battery).
    expect(fake.subscribeCount, 0, reason: 'no stream while born paused');

    // The first real resume subscribes for the very first time.
    goResumedFromPaused();
    // pumpAndSettle (not a bare pump): the resume invalidates the snapshot
    // provider, which schedules a Riverpod refresh timer that must drain.
    await tester.pumpAndSettle();
    expect(fake.subscribeCount, 1);
    expect(fake.hasActiveSubscription, isTrue);
  });

  testWidgets('paused -> detached -> resumed recovers the stream', (
    tester,
  ) async {
    // detached is reachable only FROM paused (graceful background / app
    // kill that didn't complete). If the OS resurrects the process the
    // resume must still re-subscribe (the deepest mobile background path).
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    await mount(tester, fake);
    expect(fake.subscribeCount, 1);

    goPaused();
    binding.handleAppLifecycleStateChanged(AppLifecycleState.detached);
    await tester.pump();
    expect(fake.hasActiveSubscription, isFalse, reason: 'cancelled on pause');

    // Resurrect: detached -> resumed (the engine reattaches). _wasPaused
    // survived the detached no-op, so the resume re-subscribes.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(fake.subscribeCount, 2, reason: 're-subscribed after resurrection');
  });

  testWidgets(
    'desktop: a fault WHILE hidden still reconnects (no paused ever)',
    (tester) async {
      // On desktop `paused` never fires, so the fault-reconnect path is the
      // ONLY recovery — a dead stream while minimized would be permanent.
      final fake = FakeWalletSession(current: const SyncStatus.idle());
      await mount(tester, fake);

      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      await tester.pump();

      fake.emitError(); // transport drop while minimized
      await tester.pump();
      expect(fake.subscribeCount, 1);
      await tester.pump(SyncStatusNotifier.minBackoff);
      expect(
        fake.subscribeCount,
        2,
        reason: 'reconnected while hidden (desktop)',
      );

      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    },
  );

  testWidgets(
    'disposing with a reconnect pending is safe (no fire, no throw)',
    (tester) async {
      // The _disposed guard at its boundary: a pending reconnect Timer must not
      // fire a subscribe (nor touch ref) after the provider is torn down.
      final fake = FakeWalletSession(current: const SyncStatus.idle());
      final container = await mount(tester, fake);

      fake.emitError(); // arms a reconnect timer
      await tester.pump();
      expect(fake.subscribeCount, 1);

      container.dispose(); // tear down WITH the timer pending
      await tester.pump(SyncStatusNotifier.minBackoff * 3);
      expect(fake.subscribeCount, 1, reason: 'no post-dispose subscribe');
    },
  );

  testWidgets('rapid pause/resume cycling never leaks a subscription', (
    tester,
  ) async {
    // Lock/unlock spam: every cycle must cancel before re-subscribing, so
    // exactly one subscription is ever live and cancels track subscribes.
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    await mount(tester, fake);

    for (var i = 0; i < 25; i++) {
      goPaused();
      await tester.pump();
      goResumedFromPaused();
      await tester.pumpAndSettle();
      // At every settle point: at most one live subscription.
      expect(fake.subscribeCount - fake.cancelCount, inInclusiveRange(0, 1));
    }
    expect(fake.hasActiveSubscription, isTrue);
    expect(
      fake.cancelCount,
      fake.subscribeCount - 1,
      reason: 'one cancel per prior subscribe — no orphans',
    );
  });

  testWidgets('a connect->replay->drop flap GROWS backoff (no 2s hot-retry)', (
    tester,
  ) async {
    // The unstable-mobile signature: a link that connects, replays one
    // status, then drops. The replay must NOT reset backoff to the floor —
    // otherwise the wallet hot-retries every 2s forever (battery).
    final fake = FakeWalletSession(
      current: const SyncStatus.idle(),
      dropAfterReplay: true,
    );
    await mount(tester, fake); // subscribe #1, replay, drop -> reconnect @ 2s

    expect(fake.subscribeCount, 1);
    await tester.pump(SyncStatusNotifier.minBackoff); // t=2 -> #2
    expect(fake.subscribeCount, 2);
    // The NEXT gap must be > minBackoff (it grew to 4s): 2 more seconds is
    // NOT enough. With a backoff-reset-on-replay bug this would fire at 2s.
    await tester.pump(SyncStatusNotifier.minBackoff); // t=4, still waiting
    expect(fake.subscribeCount, 2, reason: 'gap grew to 4s — no 2s hot-retry');
    await tester.pump(SyncStatusNotifier.minBackoff); // t=6 -> #3
    expect(fake.subscribeCount, 3);

    // Heal for a clean teardown (a live subscribe schedules no timer).
    fake.dropAfterReplay = false;
    await tester.pump(SyncStatusNotifier.maxBackoff);
    expect(fake.hasActiveSubscription, isTrue);

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets(
    'a subsequent emit on a live stream resets backoff to the floor',
    (tester) async {
      // The healthy-recovery half of the `_subDelivered` policy (the flap test
      // pins the other half): once a reconnected stream delivers a SECOND emit
      // (proof it is genuinely alive), the next fault must back off from the
      // FLOOR, not a stale grown ceiling.
      final fake = FakeWalletSession(current: const SyncStatus.idle());
      await mount(tester, fake);

      fake.emitError(); // backoff: wait 2s, grows to 4s
      await tester.pump(SyncStatusNotifier.minBackoff); // t=2 -> reconnect #2
      expect(fake.subscribeCount, 2); // replay only so far (backoff still 4s)

      // A real ongoing emit on the live connection → resets backoff to 2s.
      fake.push(
        const SyncStatus.scanning(
          from: 1,
          to: 10,
          percent: 0.5,
          spendableReady: false,
          rewound: false,
        ),
      );
      await tester.pump();

      fake.emitError(); // with the reset, this schedules at the 2s floor
      await tester.pump(SyncStatusNotifier.minBackoff); // +2s
      expect(
        fake.subscribeCount,
        3,
        reason: 'the live emit reset backoff — reconnect fired at the floor',
      );

      goResumedFromPausedSafeReset(binding);
    },
  );

  testWidgets('model: at-most-one subscription across a seeded event walk', (
    tester,
  ) async {
    // A deterministic model run (testing-patterns §4): drive a fixed,
    // legal sequence of lifecycle walks + faults and assert the core safety
    // invariant after EVERY step — at most one live subscription, and a
    // foreground+healthy machine is never permanently dead.
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    await mount(tester, fake);

    // A pseudo-random but fully deterministic action stream (seeded by index
    // arithmetic — no RNG, testing-patterns "deterministic only").
    for (var i = 0; i < 60; i++) {
      switch (i % 5) {
        case 0:
          goPaused();
          await tester.pump();
        case 1:
          goResumedFromPaused();
          await tester.pumpAndSettle();
        case 2:
          fake.emitError();
          await tester.pump(SyncStatusNotifier.minBackoff);
        case 3:
          fake.push(
            const SyncStatus.scanning(
              from: 1,
              to: 10,
              percent: 0.5,
              spendableReady: false,
              rewound: false,
            ),
          );
          await tester.pump();
        case 4:
          fake.complete();
          await tester.pump(SyncStatusNotifier.minBackoff);
      }
      // INVARIANT: never two live subscriptions (double battery / dup events).
      expect(
        fake.subscribeCount - fake.cancelCount,
        inInclusiveRange(0, 1),
        reason: 'step $i (action ${i % 5}) left >1 live subscription',
      );
    }

    // LIVENESS: end foreground + healthy → a bounded settle leaves the
    // stream alive, never permanently dead.
    goResumedFromPausedSafeReset(binding);
    await tester.pumpAndSettle();
    await tester.pump(SyncStatusNotifier.maxBackoff);
    expect(
      fake.hasActiveSubscription,
      isTrue,
      reason: 'foreground machine must not be permanently dead',
    );
  });

  // --- Riverpod instance-reuse (M1) -----------------------------------------

  testWidgets('a session close→reopen re-tracks AND can still reconnect (M1)', (
    tester,
  ) async {
    // Riverpod REUSES the SyncStatusNotifier instance across a
    // walletSessionProvider rebuild, firing onDispose (→ _teardown, which sets
    // _disposed=true) BEFORE re-running build(). Without the per-build reset the
    // reopened wallet's _disposed stays true, so the FIRST later fault never
    // schedules a reconnect (_scheduleReconnect early-returns) — sync silently
    // dies forever. This pins the reset: a reopened session re-tracks AND a
    // subsequent fault still recovers.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    final fake = FakeWalletSession(current: const SyncStatus.idle());
    final holder = StateProvider<WalletSession?>((ref) => fake);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(holder)),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(syncStatusProvider, (_, _) {});
    await tester.pump();
    expect(fake.subscribeCount, 1);

    container.read(holder.notifier).state = null; // wallet closed → rebuild
    await tester.pump();
    expect(
      fake.hasActiveSubscription,
      isFalse,
      reason: 'no session → no stream',
    );

    container.read(holder.notifier).state = fake; // reopened → rebuild
    await tester.pump();
    expect(
      fake.subscribeCount,
      2,
      reason: 're-tracked on the reopened session',
    );

    // The load-bearing assertion: a fault on the reopened session STILL
    // reconnects (would NOT, with a stale _disposed=true).
    fake.emitError();
    await tester.pump(SyncStatusNotifier.minBackoff);
    expect(
      fake.subscribeCount,
      3,
      reason: 'reconnect still works after a reopen (M1 reset)',
    );
    expect(fake.hasActiveSubscription, isTrue);

    goResumedFromPausedSafeReset(binding);
  });
}
