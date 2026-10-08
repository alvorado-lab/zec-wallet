import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_enabled_provider.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_status_provider.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';
import '../lifecycle_test_helpers.dart';

/// The live swap-status lifecycle contract (spec §3.3/§7; flutter-patterns
/// § Stream Lifecycle). The swap stream is foreground-only (cancel on
/// background, re-open on a real resume, desktop stays live while hidden) and —
/// unlike the sync stream — it has NO reconnect: the core poll loop retries
/// transport faults INTERNALLY and only ever EOFs INTENTIONALLY (terminal
/// status emitted, Hard kill, or teardown), so a Dart `onDone` is never a
/// transport drop to reconnect from. A terminal status latches a clean end; a
/// pre-terminal EOF stops (the host renders "unavailable" from its own kill
/// state); an establish error surfaces and stops. Driven on the host VM against
/// a fake — deterministic, no native lib, no network.
void main() {
  final binding = TestWidgetsFlutterBinding.ensureInitialized();

  const swapId = 'swap-1';
  const pending = SwapStatus.pendingDeposit(expiresAt: 2000000000);
  const processing = SwapStatus.processing();
  const underDeposited = SwapStatus.underDeposited(
    received: '0.4',
    missing: '0.1',
    deadline: 2000000000,
  );
  const success = SwapStatus.success();
  const refunded = SwapStatus.refunded();
  const failed = SwapStatus.failed(code: SwapFailureCode.providerFailure);

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
    FakeWalletSession fake, {
    StateProvider<WalletSession?>? sessionHolder,
    StateProvider<bool>? enabledHolder,
  }) async {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    final container = ProviderContainer(
      overrides: [
        if (sessionHolder != null)
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionHolder))
        else
          walletSessionProvider.overrideWithValue(fake),
        // The notifier now watches `swapEnabledProvider` (the §3.5 host kill
        // SSOT) for its teardown-on-kill. Default it ON so the lifecycle tests
        // exercise a live swap; the kill test passes a flippable holder.
        if (enabledHolder != null)
          swapEnabledProvider.overrideWith((ref) => ref.watch(enabledHolder))
        else
          swapEnabledProvider.overrideWith((ref) => true),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(swapStatusProvider(swapId), (_, _) {});
    await tester.pump();
    return container;
  }

  testWidgets('subscribes on build and emits the current status', (
    tester,
  ) async {
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);

    expect(fake.swapSubscribeCount, 1);
    expect(fake.lastWatchedSwapId, swapId);
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Processing>(),
    );

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('a non-terminal status is DATA — the stream stays live', (
    tester,
  ) async {
    final fake = FakeWalletSession()..swapCurrent = pending;
    final container = await mount(tester, fake);

    fake.pushSwap(const SwapStatus.depositDetected());
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_DepositDetected>(),
    );
    expect(
      fake.swapSubscribeCount,
      1,
      reason: 'no resubscribe on a live update',
    );
    expect(fake.hasActiveSwapSubscription, isTrue);

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets(
    'a swap already terminal when the screen opens (replay is terminal)',
    (tester) async {
      // The most common real case: reopening a finished swap. The core's
      // current-first replay delivers the terminal as the FIRST emit.
      final fake = FakeWalletSession()..swapCurrent = success;
      final container = await mount(tester, fake);

      expect(
        container.read(swapStatusProvider(swapId)).value,
        isA<SwapStatus_Success>(),
      );
      // The fake replays the terminal but does not auto-close; the notifier has
      // latched done, so a later EOF is clean and resume never re-polls.
      fake.completeSwap();
      await tester.pump();
      goPaused();
      await tester.pump();
      goResumedFromPaused();
      await tester.pumpAndSettle();
      expect(
        fake.swapSubscribeCount,
        1,
        reason: 'a finished swap is never re-polled',
      );

      goResumedFromPausedSafeReset(binding);
    },
  );

  testWidgets('UnderDeposited stays live, then Refunded ends it', (
    tester,
  ) async {
    // The partial-deposit money path (§1.7): under-deposit is NON-terminal (the
    // swap is still live, awaiting a top-up or an auto-refund), then Refunded
    // (terminal) ends it. A regression mis-classifying UnderDeposited as terminal
    // would abandon a swap that is about to refund the user.
    final fake = FakeWalletSession()..swapCurrent = pending;
    final container = await mount(tester, fake);

    fake.pushSwap(underDeposited);
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_UnderDeposited>(),
    );
    expect(fake.hasActiveSwapSubscription, isTrue, reason: 'still live');

    fake.pushSwapTerminal(refunded); // terminal, then the core closes
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Refunded>(),
    );
    expect(
      fake.hasActiveSwapSubscription,
      isFalse,
      reason: 'terminal ended it',
    );

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('paused cancels the live poll stream (battery)', (tester) async {
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);
    expect(fake.hasActiveSwapSubscription, isTrue);

    goPaused();
    await tester.pump();

    expect(fake.swapCancelCount, 1);
    expect(fake.hasActiveSwapSubscription, isFalse);
    fake.pushSwap(const SwapStatus.depositDetected());
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Processing>(),
    );

    goResumedFromPaused();
  });

  testWidgets('a real paused→resumed re-subscribes (no snapshot for swap)', (
    tester,
  ) async {
    final fake = FakeWalletSession()..swapCurrent = pending;
    await mount(tester, fake);
    expect(fake.swapSubscribeCount, 1);
    expect(fake.snapshotCount, 0, reason: 'swap status has no cold snapshot');

    goPaused();
    await tester.pump();
    goResumedFromPaused();
    await tester.pump();

    expect(fake.swapSubscribeCount, 2, reason: 're-subscribed for live status');
    expect(fake.snapshotCount, 0, reason: 'still no snapshot on resume');

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('a terminal reached while backgrounded shows on resume', (
    tester,
  ) async {
    // The swap finishes provider-side while the app is paused; on resume the
    // re-subscription's current-first replay delivers the now-terminal status.
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);

    goPaused();
    await tester.pump();
    // The provider settled the swap during the background gap.
    fake.swapCurrent = success;
    goResumedFromPaused();
    await tester.pump();

    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Success>(),
    );
    // Latched done: a later pause/resume never re-polls the finished swap.
    final subsAfterResume = fake.swapSubscribeCount;
    goPaused();
    await tester.pump();
    goResumedFromPaused();
    await tester.pumpAndSettle();
    expect(
      fake.swapSubscribeCount,
      subsAfterResume,
      reason: 'finished — no re-poll',
    );

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('inactive→resumed (window focus) is NOT a real resume', (
    tester,
  ) async {
    final fake = FakeWalletSession()..swapCurrent = pending;
    await mount(tester, fake);

    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();

    expect(fake.swapSubscribeCount, 1, reason: 'no wasteful re-subscribe');
    expect(fake.hasActiveSwapSubscription, isTrue);
  });

  testWidgets('desktop keeps the poll live while hidden (never pauses)', (
    tester,
  ) async {
    final fake = FakeWalletSession()..swapCurrent = pending;
    final container = await mount(tester, fake);

    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    await tester.pump();

    expect(fake.swapCancelCount, 0);
    expect(fake.hasActiveSwapSubscription, isTrue);
    fake.pushSwap(processing);
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Processing>(),
    );

    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });

  // --- Terminal classification ----------------------------------------------

  testWidgets('TERMINAL status then EOF is a clean end — never re-polled', (
    tester,
  ) async {
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);

    fake.pushSwapTerminal(success);
    await tester.pump();

    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Success>(),
    );
    expect(fake.hasActiveSwapSubscription, isFalse, reason: 'stream ended');
    // #367: an OBSERVED terminal PINS the outcome into the durable home row
    // (never deletes it — deleting on observation erased outcomes the user
    // never saw); the DELETE is user-intent only (terminal Done / row Remove).
    await tester.pumpAndSettle();
    expect(fake.recordSwapOutcomeCount, 1, reason: 'terminal → pin');
    expect(fake.lastOutcomeSwapId, swapId);
    expect(fake.lastRecordedOutcome, SwapOutcome.success);
    expect(
      fake.dismissSwapRecordCount,
      0,
      reason: 'the observation never dismisses — the row renders the outcome',
    );

    // A resume after the swap is done must not re-poll.
    goPaused();
    await tester.pump();
    goResumedFromPaused();
    await tester.pumpAndSettle();
    expect(fake.swapSubscribeCount, 1, reason: 'a finished swap never resumes');

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('a pin failure on the terminal edge is swallowed — the row '
      'self-lapses; tracking is unaffected (#367)', (tester) async {
    final fake = FakeWalletSession()
      ..swapCurrent = processing
      ..recordSwapOutcomeThrows = Exception('store busy');
    final container = await mount(tester, fake);
    fake.pushSwapTerminal(success);
    await tester.pump();
    await tester.pumpAndSettle();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Success>(),
      reason: 'the terminal render never depends on the display-only cleanup',
    );
    expect(fake.recordSwapOutcomeCount, 1, reason: 'the pin was attempted');
    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('a pin that finds no row (user Removed it first) resolves '
      'quietly — Ok(false), no error, tracking unaffected (#367 belt race)', (
    tester,
  ) async {
    // The Done/Remove DELETE can land before the fire-and-forget pin: the
    // UPDATE then matches zero rows (the fake models the idempotent `false`).
    // Nothing may surface and nothing may resurrect.
    final fake = FakeWalletSession()
      ..swapCurrent = processing
      ..recordSwapOutcomeResult = false;
    final container = await mount(tester, fake);
    fake.pushSwapTerminal(success);
    await tester.pump();
    await tester.pumpAndSettle();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Success>(),
    );
    expect(fake.recordSwapOutcomeCount, 1, reason: 'the pin was attempted');
    expect(
      fake.dismissSwapRecordCount,
      0,
      reason: 'a no-row pin never falls back to a dismiss (or any write)',
    );
    goResumedFromPausedSafeReset(binding);
  });

  testWidgets(
    'a REFUNDED terminal pins its own outcome class (#367) — the '
    'home row can render the money-relevant truth after an away-observation',
    (tester) async {
      final fake = FakeWalletSession()..swapCurrent = processing;
      final container = await mount(tester, fake);
      fake.pushSwapTerminal(refunded);
      await tester.pump();
      await tester.pumpAndSettle();
      expect(
        container.read(swapStatusProvider(swapId)).value,
        isA<SwapStatus_Refunded>(),
      );
      expect(fake.recordSwapOutcomeCount, 1);
      expect(
        fake.lastRecordedOutcome,
        SwapOutcome.refunded,
        reason: 'the pinned class matches the observed terminal',
      );
      goResumedFromPausedSafeReset(binding);
    },
  );

  testWidgets('refunded is terminal (clean end)', (tester) async {
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);
    fake.pushSwapTerminal(refunded);
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Refunded>(),
    );
    expect(fake.hasActiveSwapSubscription, isFalse);
    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('failed is terminal (clean end)', (tester) async {
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);
    fake.pushSwapTerminal(failed);
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Failed>(),
    );
    expect(fake.hasActiveSwapSubscription, isFalse);
    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('Unknown is NOT terminal — the live swap is never abandoned', (
    tester,
  ) async {
    // The forward-compat arm: the core never emits Unknown, but if a future SDK
    // status crossed the bridge, treating it as "done" would silently abandon a
    // live swap. It stays live (data, not a terminal latch).
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);

    fake.pushSwap(const SwapStatus.unknown());
    await tester.pump();
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Unknown>(),
    );
    expect(fake.hasActiveSwapSubscription, isTrue, reason: 'stream stays live');

    fake.pushSwapTerminal(success);
    await tester.pump();
    expect(
      fake.hasActiveSwapSubscription,
      isFalse,
      reason: 'real terminal ends it',
    );

    goResumedFromPausedSafeReset(binding);
  });

  // --- Pre-terminal end (Hard kill / teardown) + establish error ------------

  testWidgets('a pre-terminal EOF (Hard kill / teardown) STOPS — no reconnect', (
    tester,
  ) async {
    // The core ends a poll stream WITHOUT a terminal only on a Hard kill or
    // teardown (it retries transport faults internally). The notifier must STOP
    // (the host renders "unavailable" from its own kill state), never reconnect —
    // a reconnect would re-spawn the poll the core just halted (a hot loop).
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake);

    fake.completeSwap(); // EOF, no terminal first
    await tester.pump();

    expect(fake.hasActiveSwapSubscription, isFalse);
    // The last status stays visible; the stream does NOT re-open on its own.
    expect(
      container.read(swapStatusProvider(swapId)).value,
      isA<SwapStatus_Processing>(),
    );
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 60));
    expect(
      fake.swapSubscribeCount,
      1,
      reason: 'NO auto-reconnect on a kill/teardown end',
    );

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('an establish error surfaces typed and STOPS (no retry)', (
    tester,
  ) async {
    // watchSwapStatus only errors on an establish-time typed failure
    // (SwapDisabled / closed handle / bad id) — persistent. Surface it; do not
    // retry a swap that structurally can't be tracked.
    final fake = FakeWalletSession()..failSwapOnSubscribe = true;
    final container = await mount(tester, fake);

    expect(container.read(swapStatusProvider(swapId)).hasError, isTrue);
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 60));
    expect(
      fake.swapSubscribeCount,
      1,
      reason: 'a persistent establish error is not retried',
    );

    goResumedFromPausedSafeReset(binding);
  });

  // --- Riverpod rebuild (M1) -------------------------------------------------

  testWidgets('a session flip (null then back) re-tracks cleanly', (
    tester,
  ) async {
    // Riverpod REUSES the notifier instance across a walletSessionProvider
    // rebuild; build() must re-initialize the lifecycle (the M1 fix) so a wallet
    // close→reopen / onboarding flip re-tracks an in-flight swap rather than
    // staying silently dead.
    final fake = FakeWalletSession()..swapCurrent = processing;
    final holder = StateProvider<WalletSession?>((ref) => fake);
    final container = await mount(tester, fake, sessionHolder: holder);
    expect(fake.swapSubscribeCount, 1);

    container.read(holder.notifier).state = null; // wallet closed → rebuild
    await tester.pump();
    expect(
      fake.hasActiveSwapSubscription,
      isFalse,
      reason: 'no session → no poll',
    );

    container.read(holder.notifier).state = fake; // reopened → rebuild
    await tester.pump();
    expect(
      fake.swapSubscribeCount,
      2,
      reason: 're-tracked the swap on the new session',
    );
    expect(fake.hasActiveSwapSubscription, isTrue);

    goResumedFromPausedSafeReset(binding);
  });

  // --- Host kill teardown (D-2b-2) ------------------------------------------

  testWidgets('a host kill (swapEnabled→false) tears the live poll down', (
    tester,
  ) async {
    // The §3.5 layer-2 kill: the host flips `swapEnabledProvider` off (a manifest
    // kill / wind-down). The notifier watches it, so the build re-runs, cancels
    // the subscription, and does NOT re-open — REQUIRED because this provider is
    // non-autoDispose and would otherwise keep polling a killed provider forever
    // even after the screen stops watching it.
    final kill = StateProvider<bool>((ref) => true);
    final fake = FakeWalletSession()..swapCurrent = processing;
    final container = await mount(tester, fake, enabledHolder: kill);
    expect(fake.hasActiveSwapSubscription, isTrue);
    expect(fake.swapSubscribeCount, 1);

    // Host kills swap (manifest kill / wind-down).
    container.read(kill.notifier).state = false;
    await tester.pumpAndSettle();

    expect(
      fake.hasActiveSwapSubscription,
      isFalse,
      reason: 'kill cancelled the poll',
    );
    expect(fake.swapCancelCount, 1);

    // It stays down — no auto-reconnect after a kill.
    await tester.pump(const Duration(seconds: 60));
    expect(
      fake.swapSubscribeCount,
      1,
      reason: 'a killed swap is never re-polled',
    );

    // Re-enabling re-opens the DART-side poll (the teardown/rebuild mechanism).
    // This models a reversible host flip (a windDown the operator reverses); a
    // real SDK-level Hard kill would instead surface SwapStateUnavailable on
    // re-subscribe — the core-enforced contract pinned by the 'establish error
    // surfaces typed and STOPS' test above, not re-asserted here.
    container.read(kill.notifier).state = true;
    await tester.pumpAndSettle();
    expect(
      fake.swapSubscribeCount,
      2,
      reason: 're-enabled → re-opens the poll',
    );
    expect(fake.hasActiveSwapSubscription, isTrue);

    goResumedFromPausedSafeReset(binding);
  });

  // --- Robustness ------------------------------------------------------------

  testWidgets('disposing is safe (no throw)', (tester) async {
    final fake = FakeWalletSession()..swapCurrent = pending;
    final container = await mount(tester, fake);
    expect(fake.swapSubscribeCount, 1);

    container.dispose();
    await tester.pump();
    expect(
      fake.hasActiveSwapSubscription,
      isFalse,
      reason: 'cancelled on dispose',
    );
  });

  testWidgets('rapid pause/resume cycling never leaks a subscription', (
    tester,
  ) async {
    final fake = FakeWalletSession()..swapCurrent = pending;
    await mount(tester, fake);

    for (var i = 0; i < 25; i++) {
      goPaused();
      await tester.pump();
      goResumedFromPaused();
      await tester.pumpAndSettle();
      expect(
        fake.swapSubscribeCount - fake.swapCancelCount,
        inInclusiveRange(0, 1),
      );
    }
    expect(fake.hasActiveSwapSubscription, isTrue);

    goResumedFromPausedSafeReset(binding);
  });

  testWidgets('born backgrounded: no poll until the first real resume', (
    tester,
  ) async {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);

    final fake = FakeWalletSession()..swapCurrent = pending;
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        swapEnabledProvider.overrideWith((ref) => true),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(swapStatusProvider(swapId), (_, _) {});
    await tester.pump();

    expect(fake.swapSubscribeCount, 0, reason: 'no poll while born paused');

    goResumedFromPaused();
    await tester.pumpAndSettle();
    expect(fake.swapSubscribeCount, 1);
    expect(fake.hasActiveSwapSubscription, isTrue);
  });

  testWidgets(
    'S199 retention: a same-session invalidate re-poll keeps the latched '
    'status rendered while the fresh subscription earns its first emission',
    (tester) async {
      final fake = FakeWalletSession()..swapCurrent = pending;
      final container = await mount(tester, fake);

      // Latch the synthetic NotFound terminal (the #386 stale-heuristic
      // shape: the card the user re-enters onto).
      fake.pushSwapTerminal(
        const SwapStatus.failed(code: SwapFailureCode.notFound),
      );
      await tester.pump();
      expect(
        container.read(swapStatusProvider(swapId)).value,
        isA<SwapStatus_Failed>(),
      );

      // The #386 re-entry re-poll. The fresh subscription is HELD silent —
      // the real-world provider-GC'd order re-earning its ~75–135 s 404 run.
      fake.holdSwapSubscribe = true;
      container.invalidate(swapStatusProvider(swapId));
      await tester.pump();

      final status = container.read(swapStatusProvider(swapId));
      expect(status.isLoading, isTrue, reason: 'the re-poll is live');
      expect(
        status.hasValue,
        isTrue,
        reason:
            'the latched status must stay rendered — no "Swap started" busy '
            'flash over a swap the provider said does not exist',
      );
      expect(status.value, isA<SwapStatus_Failed>());
      expect(fake.swapSubscribeCount, 2, reason: 'a REAL re-poll happened');

      // Provider truth arriving replaces the retained card.
      fake.pushSwap(processing);
      await tester.pump();
      expect(
        container.read(swapStatusProvider(swapId)).value,
        isA<SwapStatus_Processing>(),
      );

      goResumedFromPausedSafeReset(binding);
    },
  );

  // NOTE: deliberately NON-discriminating at the parent (like the KEEP test
  // in no_silent_retry_test.dart, the other direction): the parent retains
  // nothing, so this holds trivially there — it exists to fail against a
  // BROKEN retention implementation (one that carries across a flip), the
  // security direction of the change.
  testWidgets(
    'S199 fence: a SESSION flip drops the retained status — nothing carries '
    'across an identity change (the S191-b sibling class must not widen)',
    (tester) async {
      final fake = FakeWalletSession()..swapCurrent = pending;
      final holder = StateProvider<WalletSession?>((ref) => fake);
      final container = await mount(tester, fake, sessionHolder: holder);

      fake.pushSwapTerminal(
        const SwapStatus.failed(code: SwapFailureCode.notFound),
      );
      await tester.pump();
      expect(container.read(swapStatusProvider(swapId)).hasValue, isTrue);

      // A DIFFERENT session object (the duress/decoy host shape). The rebuild
      // must be BARE — no copyWithPrevious carrying the old identity's status.
      final decoy = FakeWalletSession()..holdSwapSubscribe = true;
      container.read(holder.notifier).state = decoy;
      await tester.pump();

      final status = container.read(swapStatusProvider(swapId));
      expect(status.isLoading, isTrue);
      expect(
        status.hasValue,
        isFalse,
        reason: 'a session flip must never retain the previous status',
      );

      goResumedFromPausedSafeReset(binding);
    },
  );
}
