import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_sync_controller.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The auto-sync DRIVE contract (spec §3.3; flutter-patterns § Stream
/// Lifecycle): the reference app owns sync policy — run the loop while a
/// deposit-ready wallet exists AND the app is foreground, suspend on
/// background, resume on return. No manual button. Driven against a fake
/// session on the host VM — deterministic, no native lib, no network.
void main() {
  final binding = TestWidgetsFlutterBinding.ensureInitialized();

  // Legal lifecycle walks (AppLifecycleListener asserts on illegal jumps):
  // the machine is a linear chain resumed↔inactive↔hidden↔paused.
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

  // Walk back to foreground from wherever a prior test left the shared binding,
  // through legal transitions, so each test starts resumed.
  void resetForeground() {
    final s = binding.lifecycleState;
    if (s == AppLifecycleState.resumed) return;
    if (s == AppLifecycleState.paused) {
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    }
    if (s == AppLifecycleState.paused || s == AppLifecycleState.hidden) {
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    }
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  }

  /// Mount the provider graph with [fake] as the session (null = no wallet) and
  /// activate the controller (something must watch a kept-alive provider for it
  /// to build). Starts foreground unless [bornPaused].
  Future<ProviderContainer> mount(
    WidgetTester tester, {
    FakeWalletSession? fake,
    bool bornPaused = false,
    bool bornHidden = false,
    List<Override> extraOverrides = const [],
  }) async {
    resetForeground();
    if (bornPaused) goPaused();
    if (bornHidden) {
      // Desktop minimized at launch: hidden, NOT paused (stop the walk short).
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    }
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        ...extraOverrides,
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(walletSyncControllerProvider, (_, _) {});
    await tester.pumpAndSettle();
    return container;
  }

  WalletSyncDrive drive(ProviderContainer c) =>
      c.read(walletSyncControllerProvider);

  testWidgets('a foreground session auto-starts the loop (no manual button)', (
    tester,
  ) async {
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);

    expect(fake.startCount, 1, reason: 'sync started on its own');
    expect(fake.stopCount, 0);
    expect(drive(container), WalletSyncDrive.running);

    resetForeground();
  });

  testWidgets('no wallet → inactive drive, nothing started', (tester) async {
    final container = await mount(tester, fake: null);
    expect(drive(container), WalletSyncDrive.inactive);
    resetForeground();
  });

  testWidgets('background suspends the loop; foreground resumes it', (
    tester,
  ) async {
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    expect(fake.startCount, 1);

    goPaused();
    await tester.pumpAndSettle();
    expect(fake.stopCount, 1, reason: 'stopped to save battery');
    expect(drive(container), WalletSyncDrive.suspended);

    goResumedFromPaused();
    await tester.pumpAndSettle();
    expect(fake.startCount, 2, reason: 'restarted on a real resume');
    expect(drive(container), WalletSyncDrive.running);

    resetForeground();
  });

  testWidgets('inactive→resumed (window focus) does NOT re-issue a command', (
    tester,
  ) async {
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    expect(fake.startCount, 1);

    // Focus loss/gain (desktop) or a shade pull (mobile) without a real pause.
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(fake.startCount, 1, reason: 'no wasteful restart on focus noise');
    expect(fake.stopCount, 0);
    expect(drive(container), WalletSyncDrive.running);

    resetForeground();
  });

  testWidgets(
    'reaching hidden without pausing never stops (desktop minimize)',
    (tester) async {
      final fake = FakeWalletSession();
      final container = await mount(tester, fake: fake);

      // Desktop deepest state (and Android's pre-`paused` step): the loop MUST
      // keep running — only a real `paused` suspends.
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      await tester.pumpAndSettle();

      expect(fake.stopCount, 0, reason: 'hidden is not a pause');
      expect(drive(container), WalletSyncDrive.running);

      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    },
  );

  testWidgets('born backgrounded: suspended, nothing started until resume', (
    tester,
  ) async {
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake, bornPaused: true);

    expect(
      fake.startCount,
      0,
      reason: 'no background scan on a born-paused start',
    );
    expect(drive(container), WalletSyncDrive.suspended);

    goResumedFromPaused();
    await tester.pumpAndSettle();
    expect(fake.startCount, 1, reason: 'first real resume starts the loop');
    expect(drive(container), WalletSyncDrive.running);

    resetForeground();
  });

  testWidgets('born hidden (desktop minimized at launch): loop still starts', (
    tester,
  ) async {
    // `hidden` is NOT `paused`: on desktop it's the deepest state (a minimized
    // app keeps running). A controller CONSTRUCTED while hidden must START the
    // loop — only a real born-`paused` (mobile cold start behind the lock
    // screen) defers it. Pins the desktop-minimize boundary by construction.
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake, bornHidden: true);

    expect(
      fake.startCount,
      1,
      reason: 'hidden is not a pause; the loop starts',
    );
    expect(fake.stopCount, 0);
    expect(drive(container), WalletSyncDrive.running);

    resetForeground();
  });

  testWidgets('a start-command failure surfaces as failed; retry re-attempts', (
    tester,
  ) async {
    final fake = FakeWalletSession()..failStart = true;
    final container = await mount(tester, fake: fake);

    // The (rare) start command threw → honest failed state, no silent failure.
    expect(fake.startCount, 1);
    expect(drive(container), WalletSyncDrive.failed);

    // Recover the endpoint and retry → starts and goes running.
    fake.failStart = false;
    container.read(walletSyncControllerProvider.notifier).retry();
    await tester.pumpAndSettle();
    expect(fake.startCount, 2, reason: 'retry re-issued the start');
    expect(drive(container), WalletSyncDrive.running);

    resetForeground();
  });

  testWidgets('#409 R2: a wedged start reads CONSERVATIVELY as failed — which '
      'is the state that renders a way out — and the reconnect kick can still '
      'act from it', (tester) async {
    // THE ASYMMETRY THAT DECIDES THIS (security + reliability review). `failed`
    // is a state WITH a user-visible escape: `_SyncStartFailedNotice`
    // (wallet_screen.dart:889) and the sheet's retry (sync_status_sheet.dart:235)
    // both render on it. `running` is a state with NONE — and a loop that never
    // started publishes no `Stalled` either (every producer is inside run_loop:
    // sync_controller.rs:419/447/475), so the watch keeps `Idle`, which renders
    // a permanent "Connecting…" with the money copy still promising a pass.
    // Under uncertainty the honest, affordance-bearing, money-qualifying state
    // wins. What must NOT survive is the kick foreclosing itself.
    final fake = FakeWalletSession()..startSyncGate = Completer<void>();
    final container = await mount(tester, fake: fake);
    expect(fake.startCount, 1, reason: 'the start was issued');

    await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
    await tester.pumpAndSettle();

    expect(
      drive(container),
      WalletSyncDrive.failed,
      reason:
          'a bounded-out start reads conservatively, and `failed` is the '
          'only state that renders a retry affordance',
    );
    expect(
      container.read(walletSyncPassesRunProvider),
      isFalse,
      reason:
          'money copy stays qualified and the destructive-rescan fence '
          'stays shut while the loop is not known to be running',
    );
    expect(
      container.read(walletSyncControllerProvider.notifier).tryNow(),
      isTrue,
      reason:
          'THE #409 R2 FIX: the kick fires on `Stalled`, which only a '
          'RUNNING loop publishes — so if the timeout was merely a slow '
          'bridge, the kick must not be locked out by the guess',
    );

    fake.startSyncGate!.complete();
    await tester.pumpAndSettle();
    resetForeground();
  });

  testWidgets('#409 R2: a LATE start verdict is applied, not discarded — a '
      'late success clears the conservative failed', (tester) async {
    // `Future.timeout` registers its onError behind `if (timer.isActive)`, so
    // after the bound fires the late answer is not merely ignored: it counts as
    // HANDLED and never reaches the zone. Without an explicit watcher the
    // abandoned call's verdict vanishes, which is what made the first cut of
    // this fix unsafe — it wrote `running` on the guess and then never learned
    // it was wrong.
    final fake = FakeWalletSession()..startSyncGate = Completer<void>();
    final container = await mount(tester, fake: fake);
    await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(drive(container), WalletSyncDrive.failed);

    // The bridge un-wedges at last: the loop really did start.
    fake.startSyncGate!.complete();
    await tester.pumpAndSettle();

    expect(
      drive(container),
      WalletSyncDrive.running,
      reason:
          'the late SUCCESS corrects the conservative guess, so the '
          'notice clears with no user action',
    );
    expect(container.read(walletSyncPassesRunProvider), isTrue);
    resetForeground();
  });

  testWidgets('#409 R2: a late start REFUSAL confirms failed — the SDK saying '
      'no after the bound is still the SDK saying no', (tester) async {
    // The case the first cut got exactly backwards: a start that answers late
    // with WalletBusy/InvalidState (e.g. blocked behind a rescan holding the
    // FRB write lock) is a definitive refusal, and it must not be read as the
    // silence the bound was about.
    final fake = FakeWalletSession()..startSyncGate = Completer<void>();
    final container = await mount(tester, fake: fake);
    await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
    await tester.pumpAndSettle();

    fake.startSyncGate!.completeError(StateError('late refusal'));
    await tester.pumpAndSettle();

    expect(
      drive(container),
      WalletSyncDrive.failed,
      reason: 'a late refusal keeps the honest failed state + its affordance',
    );
    expect(container.read(walletSyncPassesRunProvider), isFalse);
    resetForeground();
  });

  testWidgets('teardown stops the loop (no orphaned background sync)', (
    tester,
  ) async {
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    expect(fake.startCount, 1);
    expect(fake.stopCount, 0);

    container.dispose();
    await tester.pump();
    expect(fake.stopCount, 1, reason: 'best-effort stop on dispose');

    resetForeground();
  });

  testWidgets('rapid pause/resume cycling stays balanced and ends running', (
    tester,
  ) async {
    // Lock/unlock spam: commands serialize, never run concurrently, and the
    // machine settles foreground+running — never stuck suspended.
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);

    for (var i = 0; i < 25; i++) {
      goPaused();
      await tester.pumpAndSettle();
      goResumedFromPaused();
      await tester.pumpAndSettle();
    }

    // Start = initial + one per resume; stop = one per pause. Balanced ±1.
    expect(fake.startCount, 26, reason: '1 initial + 25 resumes');
    expect(fake.stopCount, 25, reason: '25 pauses');
    expect(
      drive(container),
      WalletSyncDrive.running,
      reason: 'a foreground machine is never left suspended',
    );

    resetForeground();
  });

  testWidgets('SyncStatus stream is observed independently of the drive', (
    tester,
  ) async {
    // The drive controls the loop; the status stream is a SEPARATE concern
    // (observe-only). Activating the drive must not touch the stream — proves
    // the two are orthogonal (one source of truth each).
    final fake = FakeWalletSession(current: const SyncStatus.upToDate(tip: 1));
    await mount(tester, fake: fake);
    expect(fake.startCount, 1);
    expect(
      fake.subscribeCount,
      0,
      reason: 'the drive does not subscribe to the status stream',
    );
    resetForeground();
  });

  testWidgets('sync starts when the session ARRIVES (null → active)', (
    tester,
  ) async {
    // Pins the lifetime contract: the controller is built while the session is
    // null (inactive, nothing started), and when onboarding reaches Active the
    // session arrives → Riverpod rebuilds a FRESH controller that auto-starts.
    resetForeground();
    final fake = FakeWalletSession();
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(null)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(walletSyncControllerProvider, (_, _) {});
    await tester.pumpAndSettle();

    // No wallet yet → inactive, nothing started.
    expect(drive(container), WalletSyncDrive.inactive);
    expect(fake.startCount, 0);

    // The wallet becomes deposit-ready (onboarding reaches Active). Reading the
    // drive forces the dependent rebuild; the pump then drains the async start.
    container.updateOverrides([walletSessionProvider.overrideWithValue(fake)]);
    expect(drive(container), WalletSyncDrive.running);
    await tester.pumpAndSettle();
    expect(
      fake.startCount,
      1,
      reason: 'a fresh controller auto-started on session arrival',
    );
    resetForeground();
  });

  testWidgets('survives losing its last watcher (leaving the wallet screen)', (
    tester,
  ) async {
    // The flat /wallet route is disposed on navigate-away. A non-autoDispose
    // provider must NOT tear down when its last watcher drops — sync keeps
    // running app-wide (no stop), state persists. This is the kept-alive
    // contract the auto-drive relies on.
    resetForeground();
    final fake = FakeWalletSession();
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    final sub = container.listen(walletSyncControllerProvider, (_, _) {});
    await tester.pumpAndSettle();
    expect(fake.startCount, 1);

    sub.close(); // simulate the wallet screen unmounting
    await tester.pump();

    expect(fake.stopCount, 0, reason: 'no stop on watcher loss');
    expect(
      drive(container),
      WalletSyncDrive.running,
      reason: 'still alive + running for the container lifetime',
    );
    resetForeground();
  });

  testWidgets('a stop-command failure stays running, not failed (honest)', (
    tester,
  ) async {
    // A stop that throws leaves the loop most likely still running — the honest
    // state is `running`, NOT the start-failed notice (which would mislead the
    // user into thinking sync never started).
    final fake = FakeWalletSession()..failStop = true;
    final container = await mount(tester, fake: fake);
    expect(drive(container), WalletSyncDrive.running);

    goPaused();
    await tester.pumpAndSettle();
    expect(fake.stopCount, 1, reason: 'stop was attempted');
    expect(
      drive(container),
      WalletSyncDrive.running,
      reason: 'a failed stop is not a start failure',
    );

    resetForeground();
  });

  testWidgets('a session flip mid-command NEVER restarts the DEAD identity\'s '
      'loop — stale chain links are generation-skipped (S153 wrap review: '
      'the guard was load-bearing by probe but unpinned)', (tester) async {
    // The interleaving: pause parks stop(A) mid-await → a real resume queues
    // start(A) behind it → the session flips A→B (build re-runs, bumps the
    // generation, queues its own start(B)) → the gate opens and the chain
    // drains. Without the per-build generation capture the stale start(A)
    // link would RESTART the dead identity's sync loop after its dispose-stop
    // (background scan/network on a handle no UI is attached to).
    resetForeground();
    final fakeA = FakeWalletSession()..stopSyncGate = Completer<void>();
    final fakeB = FakeWalletSession();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(walletSyncControllerProvider, (_, _) {});
    await tester.pumpAndSettle();
    expect(fakeA.startCount, 1, reason: 'the initial foreground start');

    goPaused(); // stop(A) issues and PARKS on the gate
    await tester.pump();
    goResumedFromPaused(); // start(A) queues behind the parked stop
    await tester.pump();

    // The identity flips while the chain is parked.
    container.read(sessionSwitch.notifier).state = fakeB;
    await tester.pump();

    fakeA.stopSyncGate!.complete(); // the chain drains
    await tester.pumpAndSettle();

    expect(
      fakeA.startCount,
      1,
      reason:
          'the stale resume-start was generation-skipped — the DEAD '
          'identity\'s loop is never restarted',
    );
    expect(fakeB.startCount, 1, reason: 'the new build\'s own start ran');
    expect(drive(container), WalletSyncDrive.running);
    resetForeground();
  });

  group('#383 R1 — host sync policy', () {
    testWidgets('policy OFF: never starts, reads disabledByHost — and a REAL '
        'paused→resumed never sneaks a start past the gate', (tester) async {
      final fake = FakeWalletSession();
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      );

      expect(drive(container), WalletSyncDrive.disabledByHost);
      expect(fake.startCount, 0, reason: 'the policy gate holds at build');

      goPaused();
      await tester.pumpAndSettle();
      goResumedFromPaused();
      await tester.pumpAndSettle();
      expect(
        fake.startCount,
        0,
        reason: 'a resume must not bypass the host policy',
      );
      // The honesty pin: the pause's stop-settle and the gated resume
      // must BOTH land the drive back on disabledByHost — a leftover
      // `suspended` would let the badge fall through to a retained healthy
      // story instead of "Sync off".
      expect(
        drive(container),
        WalletSyncDrive.disabledByHost,
        reason: 'policy-off survives a background/resume cycle honestly',
      );

      resetForeground();
    });

    testWidgets('a policy flip is reactive: false→true starts the loop on the '
        'spot; true→false stops it and lands disabledByHost', (tester) async {
      final fake = FakeWalletSession();
      final policySwitch = StateProvider<bool>((ref) => false);
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [
          walletSyncPolicyProvider.overrideWith(
            (ref) => ref.watch(policySwitch),
          ),
        ],
      );
      expect(drive(container), WalletSyncDrive.disabledByHost);
      expect(fake.startCount, 0);

      container.read(policySwitch.notifier).state = true;
      await tester.pumpAndSettle();
      expect(fake.startCount, 1, reason: 'the flip-on starts the loop');
      expect(drive(container), WalletSyncDrive.running);

      container.read(policySwitch.notifier).state = false;
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.disabledByHost);
      expect(
        fake.stopCount,
        greaterThanOrEqualTo(1),
        reason: 'the prior build\'s dispose best-effort stopped the loop',
      );

      resetForeground();
    });

    testWidgets('retry() under a disabled policy is a no-op — never a policy '
        'override', (tester) async {
      final fake = FakeWalletSession();
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      );
      expect(drive(container), WalletSyncDrive.disabledByHost);

      container.read(walletSyncControllerProvider.notifier).retry();
      await tester.pumpAndSettle();
      expect(fake.startCount, 0, reason: 'retry never overrides the policy');
      expect(drive(container), WalletSyncDrive.disabledByHost);

      resetForeground();
    });
  });

  group('S205-b — the dispose-stop serializer + policy-off honesty', () {
    testWidgets('F7 ORDERING: a policy-flip stop CHAINS behind an in-flight '
        'start — the two same-handle FFI calls never race (stop fires only '
        'after the start settles)', (tester) async {
      // The exact interleaving the fold closes: build() issues startSync and
      // the call is still in flight when the host flips the policy off. The
      // dispose-stop used to be fire-and-forget — two unordered FFI calls on
      // one Rust controller, where an ordering ending in the start leaves the
      // loop RUNNING under a "Sync off" badge. Chained through [_inFlight],
      // the stop provably runs after the start settles.
      final fake = _OrderRecordingSession()..startSyncGate = Completer<void>();
      final policySwitch = StateProvider<bool>((ref) => true);
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [
          walletSyncPolicyProvider.overrideWith(
            (ref) => ref.watch(policySwitch),
          ),
        ],
      );
      expect(fake.calls, ['start'], reason: 'the start is issued and PENDING');

      // The host flips the policy while the start is parked mid-FFI.
      container.read(policySwitch.notifier).state = false;
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.disabledByHost);
      expect(
        fake.stopCount,
        0,
        reason:
            'the dispose-stop must WAIT behind the in-flight start — an '
            'unordered stop here is the race the S205-b chain closes',
      );
      expect(fake.calls, ['start']);

      // The start settles → the chained stop then fires, in order.
      fake.startSyncGate!.complete();
      await tester.pumpAndSettle();
      expect(
        fake.calls,
        ['start', 'stop'],
        reason: 'start settles FIRST, then the serialized dispose-stop',
      );
      expect(fake.stopCount, 1);
      expect(
        drive(container),
        WalletSyncDrive.disabledByHost,
        reason: 'the stale start settle never overwrites the policy story',
      );

      resetForeground();
    });

    testWidgets('rapid policy flip true→false→true: the serialized sequence '
        'ENDS in a start and the drive is running (the loop is never left '
        'stopped under a healthy badge)', (tester) async {
      final fake = _OrderRecordingSession();
      final policySwitch = StateProvider<bool>((ref) => true);
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [
          walletSyncPolicyProvider.overrideWith(
            (ref) => ref.watch(policySwitch),
          ),
        ],
      );
      expect(fake.calls, ['start']);

      // The rapid flip — no settle in between (the adversarial cadence).
      container.read(policySwitch.notifier).state = false;
      await tester.pump();
      container.read(policySwitch.notifier).state = true;
      await tester.pumpAndSettle();

      expect(
        fake.calls.last,
        'start',
        reason: 'the chain ends with the flip-back-on start — the loop runs',
      );
      expect(
        fake.stopCount,
        greaterThanOrEqualTo(1),
        reason: 'the flip-off dispose best-effort stopped the loop en route',
      );
      expect(drive(container), WalletSyncDrive.running);

      resetForeground();
    });

    testWidgets('#407 R7: a FAILED start survives a POLICY CYCLE taken while '
        'backgrounded — the born-paused arm must not hand the money promise '
        'back on the way through', (tester) async {
      // MEASURED sequence (the probe that produced this pin):
      //   A start-failed        drive=failed         passesRun=false
      //   B backgrounded        drive=failed         passesRun=false  (#403 R5)
      //   C policy-off-while-bg drive=disabledByHost passesRun=false
      //   D policy-on-while-bg  drive=SUSPENDED      passesRun=TRUE   <-- the bug
      //   E resumed             drive=failed         passesRun=false
      //
      // D is a passes-run state over a loop that never started, and it is STILL
      // the state on the first frames after the user foregrounds — the resume's
      // start writes no optimistic state and only corrects once it settles. That
      // is precisely the unqualified "we'll finish this on a later sync" flash
      // #403 R5 removed, coming back through a different door.
      //
      // The fix could NOT read the drive state: the policy-off leg at C
      // legitimately overwrites `failed` with `disabledByHost`, so by D there is
      // nothing left to preserve. It reads a remembered `_startFailed` instead,
      // which is why this test walks the WHOLE cycle rather than just pausing.
      final policy = StateProvider<bool>((ref) => true);
      final fake = FakeWalletSession()..failStart = true;
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [
          walletSyncPolicyProvider.overrideWith((ref) => ref.watch(policy)),
        ],
      );
      expect(drive(container), WalletSyncDrive.failed, reason: 'A');
      expect(container.read(walletSyncPassesRunProvider), isFalse);

      goPaused();
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.failed, reason: 'B (#403 R5)');

      container.read(policy.notifier).state = false;
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.disabledByHost, reason: 'C');

      container.read(policy.notifier).state = true;
      await tester.pumpAndSettle();
      expect(
        drive(container),
        WalletSyncDrive.failed,
        reason:
            'D — THE POINT. Before #407 R7 this was `suspended`, and '
            '`suspended` is a passes-run state: the money copy promised a sync '
            'pass that no loop was going to run',
      );
      expect(
        container.read(walletSyncPassesRunProvider),
        isFalse,
        reason: 'the promise stays qualified across the whole cycle',
      );

      resetForeground();
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.failed, reason: 'E');
    });

    testWidgets('#407 R7 counterfactual: the same policy cycle over a HEALTHY '
        'start lands on suspended — the flag tracks the failure, not the '
        'cycle', (tester) async {
      final policy = StateProvider<bool>((ref) => true);
      final fake = FakeWalletSession(); // start SUCCEEDS
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [
          walletSyncPolicyProvider.overrideWith((ref) => ref.watch(policy)),
        ],
      );
      expect(drive(container), WalletSyncDrive.running, reason: 'precondition');

      goPaused();
      await tester.pumpAndSettle();
      container.read(policy.notifier).state = false;
      await tester.pumpAndSettle();
      container.read(policy.notifier).state = true;
      await tester.pumpAndSettle();

      expect(
        drive(container),
        WalletSyncDrive.suspended,
        reason:
            'no failure to remember — qualifying a healthy wallet would be the '
            'mirror-image lie, and would make the pin above pass for free',
      );
      expect(container.read(walletSyncPassesRunProvider), isTrue);
      resetForeground();
    });

    testWidgets('born-paused + policy-off reads disabledByHost, never '
        'suspended — the policy gate is checked BEFORE the lifecycle arm in '
        'build()', (tester) async {
      final fake = FakeWalletSession();
      final container = await mount(
        tester,
        fake: fake,
        bornPaused: true,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      );

      expect(
        drive(container),
        WalletSyncDrive.disabledByHost,
        reason:
            'the honest reason wins: "sync off" explains itself; a '
            '"suspended" would fall through to a retained healthy story',
      );
      expect(fake.startCount, 0, reason: 'nothing starts under either gate');

      resetForeground();
    });

    testWidgets('stop-failure honesty under policy-off: a pause whose stop '
        'THROWS lands disabledByHost, never running (the catch arm mirrors '
        'the settle ternary)', (tester) async {
      // Policy off from build: no start ever ran, so the old unconditional
      // `running` on a stop failure would be doubly false — the loop is not
      // running AND the badge must keep telling the "Sync off" story.
      final fake = FakeWalletSession()..failStop = true;
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      );
      expect(drive(container), WalletSyncDrive.disabledByHost);
      expect(fake.startCount, 0);

      goPaused();
      await tester.pumpAndSettle();
      expect(fake.stopCount, 1, reason: 'the pause attempted the stop');
      expect(
        drive(container),
        WalletSyncDrive.disabledByHost,
        reason:
            'a failed stop must not overwrite disabledByHost with running — '
            'the policy story wins over transients (S205-b)',
      );

      resetForeground();
    });
  });

  group('S205-c — session-switch liveness + the chain belt', () {
    testWidgets('SESSION-SWITCH ordering: the dead handle\'s stop is issued '
        'BEFORE the new session\'s start — the dispose-stop and the fresh '
        'build\'s start ride ONE serialized chain (per-session attribution)', (
      tester,
    ) async {
      // The F7 pin proved the ordering on a POLICY flip (same handle);
      // this pins the SESSION flip, where the ordering is a liveness coupling
      // (the comment's "orders the dead handle's stop before the new
      // session's start"): two fakes record into ONE shared log, so the
      // assertion carries which session got which call, not just counts.
      resetForeground();
      final log = <String>[];
      final fakeA = _OrderRecordingSession(log: log, tag: 'A');
      final fakeB = _OrderRecordingSession(log: log, tag: 'B');
      final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: const SizedBox(),
        ),
      );
      container.listen(walletSyncControllerProvider, (_, _) {});
      await tester.pumpAndSettle();
      expect(log, ['startA'], reason: 'the initial foreground start');

      container.read(sessionSwitch.notifier).state = fakeB;
      await tester.pumpAndSettle();

      expect(
        log,
        ['startA', 'stopA', 'startB'],
        reason:
            'stop(A) strictly precedes start(B) — an unordered pair could '
            'leave the dead identity\'s loop running past the new start',
      );
      expect(drive(container), WalletSyncDrive.running);
      resetForeground();
    });

    testWidgets('WEDGE bound: a dead handle whose stop NEVER settles is '
        'abandoned at walletFfiWedgeTimeout — the new session\'s start still '
        'runs (the liveness the bound exists for)', (tester) async {
      // The pathological case the timeout closes: without the bound,
      // the wedged dead-handle stopSync would starve the NEW wallet\'s first
      // start forever behind "Connecting…" (desktop never gets a lifecycle
      // re-drive, and retry() chains behind the same tail).
      resetForeground();
      final log = <String>[];
      final fakeA = _OrderRecordingSession(log: log, tag: 'A')
        ..stopSyncGate = Completer<void>(); // deliberately NEVER completed
      final fakeB = _OrderRecordingSession(log: log, tag: 'B');
      final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: const SizedBox(),
        ),
      );
      container.listen(walletSyncControllerProvider, (_, _) {});
      await tester.pumpAndSettle();
      expect(log, ['startA']);

      container.read(sessionSwitch.notifier).state = fakeB;
      await tester.pumpAndSettle();

      // Before the bound elapses the chain HOLDS the new start behind the
      // wedged stop — that is the serialization working as designed.
      expect(log, ['startA', 'stopA']);
      expect(
        fakeB.startCount,
        0,
        reason: 'parked behind the wedged dead-handle stop',
      );

      // Past the package FFI wedge bound: the stop is abandoned (onTimeout
      // no-op) and the chain proceeds to the new session\'s start.
      await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
      await tester.pumpAndSettle();
      expect(
        log,
        ['startA', 'stopA', 'startB'],
        reason:
            'the bounded link abandoned the wedged stop — the new wallet '
            'is never starved behind a dead handle',
      );
      expect(drive(container), WalletSyncDrive.running);
      resetForeground();
    });

    testWidgets('the trailing chain belt: a policy read that THROWS in the '
        'stop-settle path must not reject the command chain — a later '
        'lifecycle start still runs (nothing is silently skipped)', (
      tester,
    ) async {
      // The regression the belt closes: the stop-settle catch arm
      // reads the HOST-overridable walletSyncPolicyProvider; a throwing
      // override would rethrow OUT of the catch, reject that link, and
      // silently skip every queued command until a dispose link healed the
      // chain. Interleaving: park the (failing) stop on a gate, arm the
      // poison, then release the gate in the SAME tick — the settle
      // continuation (a microtask) consults the now-poisoned provider before
      // the scheduler rebuilds the controller. If the scheduler ever wins
      // that race the test degrades gracefully to the plain property (the
      // chain still runs later commands), which is the pinned contract
      // either way.
      resetForeground();
      final fake = FakeWalletSession()
        ..failStop = true
        ..stopSyncGate = Completer<void>();
      final poisonArmed = StateProvider<bool>((ref) => false);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSyncPolicyProvider.overrideWith((ref) {
            if (ref.watch(poisonArmed)) {
              throw StateError('poisoned policy read');
            }
            return true;
          }),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: const SizedBox(),
        ),
      );
      // The poisoned window also errors the controller\'s own build (it
      // watches the policy) — tolerated here; the pinned property is the
      // COMMAND CHAIN\'s health after the poison clears.
      container.listen(
        walletSyncControllerProvider,
        (_, _) {},
        onError: (_, _) {},
      );
      await tester.pumpAndSettle();
      expect(fake.startCount, 1);

      goPaused(); // the stop issues and PARKS on the gate
      await tester.pump();
      expect(fake.stopCount, 1, reason: 'the pause attempted the stop');

      // Arm the poison and release the gate with NO await in between: the
      // failing stop settles first and its catch arm hits the throwing read
      // (verified by stack probe: the recompute fires from
      // WalletSyncController._command's catch arm, before the scheduler
      // rebuilds the controller).
      container.read(poisonArmed.notifier).state = true;
      fake.stopSyncGate!.complete();
      await tester.pumpAndSettle();

      // Heal the policy; the controller rebuilds (still paused → suspended).
      container.read(poisonArmed.notifier).state = false;
      await tester.pumpAndSettle();

      goResumedFromPaused();
      await tester.pumpAndSettle();
      expect(
        fake.startCount,
        2,
        reason:
            'the resume start rode the chain — a rejected (poisoned) link '
            'would have silently skipped it forever',
      );
      expect(drive(container), WalletSyncDrive.running);
      resetForeground();
    });

    testWidgets('teardown gate: a dispose while a start is still in flight '
        'runs the CHAINED stop once that start settles — serialized, and no '
        'orphaned background loop', (tester) async {
      // The dispose can\'t await, so the teardown stop rides the serializer:
      // it must neither overtake the parked start (the same-handle FFI race)
      // nor get dropped (an orphaned loop no UI is attached to).
      final fake = _OrderRecordingSession()..startSyncGate = Completer<void>();
      final container = await mount(tester, fake: fake);
      expect(fake.calls, ['start'], reason: 'the start is issued and PENDING');
      expect(fake.stopCount, 0);

      container.dispose(); // chains the teardown stop behind the parked start
      await tester.pump();
      expect(
        fake.stopCount,
        0,
        reason: 'serialized: the stop must NOT overtake the in-flight start',
      );

      fake.startSyncGate!.complete(); // the start settles → the chain drains
      await tester.pumpAndSettle();
      expect(fake.calls, [
        'start',
        'stop',
      ], reason: 'the chained teardown stop ran — no orphaned loop');
      expect(fake.stopCount, 1);
      resetForeground();
    });
  });

  group('#399 — tryNow (the sync sheet\'s "Try now" ladder reset)', () {
    testWidgets('tryNow issues stop THEN start — the pair that resets the '
        'core retry ladder — and stays running', (tester) async {
      final fake = _OrderRecordingSession();
      final container = await mount(tester, fake: fake);
      expect(fake.calls, ['start']);

      expect(
        container.read(walletSyncControllerProvider.notifier).tryNow(),
        isTrue,
        reason: 'the pair was enqueued — the caller may open the live window',
      );
      await tester.pumpAndSettle();
      expect(fake.calls, [
        'start',
        'stop',
        'start',
      ], reason: 'the ladder reset is a serialized stop→start pair');
      expect(drive(container), WalletSyncDrive.running);
      resetForeground();
    });

    testWidgets('tryNow serializes behind an in-flight command (never two '
        'unordered FFI calls on one handle)', (tester) async {
      final gate = Completer<void>();
      final fake = _OrderRecordingSession()..startSyncGate = gate;
      final container = await mount(tester, fake: fake);
      expect(fake.calls, ['start'], reason: 'the build start is PARKED');

      container.read(walletSyncControllerProvider.notifier).tryNow();
      await tester.pump();
      expect(
        fake.stopCount,
        0,
        reason: 'the tryNow stop must NOT overtake the in-flight start',
      );

      fake.startSyncGate = null; // only the first start parks
      gate.complete(); // release it → the queued tryNow pair drains in order
      await tester.pumpAndSettle();
      expect(fake.calls, [
        'start',
        'stop',
        'start',
      ], reason: 'the pair chained behind the released start');
      expect(drive(container), WalletSyncDrive.running);
      resetForeground();
    });

    testWidgets('a tryNow whose START fails lands on the honest failed '
        '(the retry notice renders); the stop leg alone never does', (
      tester,
    ) async {
      final fake = FakeWalletSession();
      final container = await mount(tester, fake: fake);
      expect(drive(container), WalletSyncDrive.running);

      fake.failStart = true;
      container.read(walletSyncControllerProvider.notifier).tryNow();
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.failed);

      // …and a stop-leg failure is swallowed: the start still runs.
      fake.failStart = false;
      fake.failStop = true;
      container.read(walletSyncControllerProvider.notifier).retry();
      await tester.pumpAndSettle();
      final startsBefore = fake.startCount;
      container.read(walletSyncControllerProvider.notifier).tryNow();
      await tester.pumpAndSettle();
      expect(
        fake.startCount,
        startsBefore + 1,
        reason: 'best-effort stop: its failure never blocks the start',
      );
      expect(drive(container), WalletSyncDrive.running);
      resetForeground();
    });

    testWidgets('tryNow under a disabled policy is a no-op — never a way '
        'past a deliberate host off', (tester) async {
      final fake = FakeWalletSession();
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      );
      expect(drive(container), WalletSyncDrive.disabledByHost);

      expect(
        container.read(walletSyncControllerProvider.notifier).tryNow(),
        isFalse,
        reason: 'gate-rejected — the caller must NOT open the live window',
      );
      await tester.pumpAndSettle();
      expect(fake.startCount, 0);
      expect(fake.stopCount, 0);
      expect(drive(container), WalletSyncDrive.disabledByHost);
      resetForeground();
    });

    testWidgets('S207 M1: a tap racing a JUST-ARRIVED pause never restarts '
        'sync in the background — the state gate is still `running` while '
        'the pause\'s stop is unsettled, so the pause INTENT is re-checked '
        'inside the serialized link', (tester) async {
      final fake = FakeWalletSession();
      final container = await mount(tester, fake: fake);
      expect(fake.startCount, 1);

      // The pause event lands (queues the stop; `state` is STILL `running`
      // because the pause path writes no optimistic state)…
      goPaused();
      // …and a tap already in the event queue passes the state gate NOW.
      container.read(walletSyncControllerProvider.notifier).tryNow();
      await tester.pumpAndSettle();

      expect(
        fake.startCount,
        1,
        reason:
            'the queued tryNow start must be dropped — no background '
            'restart with no later lifecycle stop coming',
      );
      expect(drive(container), WalletSyncDrive.suspended);
      resetForeground();
    });

    testWidgets('tryNow while suspended is a no-op (the stale-tap belt): a '
        'backgrounded wallet must not restart sync with no lifecycle stop '
        'coming', (tester) async {
      final fake = FakeWalletSession();
      final container = await mount(tester, fake: fake);
      goPaused();
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.suspended);
      expect(fake.stopCount, 1);

      expect(
        container.read(walletSyncControllerProvider.notifier).tryNow(),
        isFalse,
        reason: 'gate-rejected — no live window over a suspended loop',
      );
      await tester.pumpAndSettle();
      expect(fake.startCount, 1, reason: 'no background restart');
      expect(fake.stopCount, 1);
      expect(drive(container), WalletSyncDrive.suspended);
      resetForeground();
    });
  });

  // THE MONEY-COPY SSOT (#403 R5 + R10).
  //
  // `walletSyncPassesRunProvider` decides whether four money surfaces may promise
  // that the wallet finishes a payment on a later sync. #401 introduced it and
  // NOTHING pinned it: every existing test in the package reaches those surfaces
  // through `walletSyncPolicyProvider.overrideWithValue(false)`, i.e. only the
  // path that already worked before it existed. The `failed` half — the reason
  // the provider was written — had no coverage at all.
  group('#403 R5/R10 — walletSyncPassesRunProvider', () {
    bool passesRun(ProviderContainer c) => c.read(walletSyncPassesRunProvider);

    testWidgets('a FAILED start reads false while the host policy still reads '
        'true — the case the policy alone cannot see', (tester) async {
      final fake = FakeWalletSession()..failStart = true;
      final container = await mount(tester, fake: fake);

      expect(drive(container), WalletSyncDrive.failed);
      expect(
        container.read(walletSyncPolicyProvider),
        isTrue,
        reason: 'the host never turned sync off — this is the whole point',
      );
      expect(
        passesRun(container),
        isFalse,
        reason:
            'the loop never ran, so `after_synced` never fires and no queued '
            'or saved payment advances',
      );

      // …and it recovers: a retry that starts restores the promise.
      fake.failStart = false;
      container.read(walletSyncControllerProvider.notifier).retry();
      await tester.pumpAndSettle();
      expect(drive(container), WalletSyncDrive.running);
      expect(passesRun(container), isTrue);
      resetForeground();
    });

    testWidgets('a background/resume cycle over a PERSISTENTLY failing start '
        'never flickers the promise back on', (tester) async {
      // The #403 R5 flicker, with the HOST POLICY ON — which is the only shape
      // in which it exists. `stopSync` on a loop that never started is a no-op
      // and reports Ok, so the background stop used to settle on `suspended`,
      // and `suspended` is a passes-run state: every foreground return flashed
      // the UNQUALIFIED money promise until the resume's own start failed again.
      //
      // NOTE, because the review that found this proposed only half the fix: the
      // `walletSyncPolicyProvider && drive != failed` conjunction does NOT close
      // this by itself — the policy reads `true` here, so the whole predicate
      // rides on the drive. Preserving `failed` across the background stop is
      // what actually closes it. Pinning it with the policy overridden to
      // `false` would have passed on the broken code, for the wrong reason.
      final fake = FakeWalletSession()..failStart = true;
      final container = await mount(tester, fake: fake);
      expect(drive(container), WalletSyncDrive.failed);
      expect(container.read(walletSyncPolicyProvider), isTrue);
      expect(passesRun(container), isFalse);

      goPaused();
      await tester.pumpAndSettle();
      expect(
        drive(container),
        WalletSyncDrive.failed,
        reason: 'the honest reason wins over the lifecycle one',
      );
      expect(
        passesRun(container),
        isFalse,
        reason: 'a backgrounded suspend must not re-promise anything',
      );

      goResumedFromPaused();
      await tester.pumpAndSettle();
      expect(
        passesRun(container),
        isFalse,
        reason:
            'the start is still failing — this window is exactly when the old '
            'predicate said "your payment completes on its own"',
      );
      resetForeground();
    });

    testWidgets('a host that turned sync off reads false', (tester) async {
      // The policy term is DEFENCE IN DEPTH, not a fix for a reproduced bug, and
      // this pin says only what it proves. Today the term is redundant with the
      // drive: `build` returns `disabledByHost` synchronously on a policy-off,
      // and every settle path is policy-aware, so no drive value survives a
      // policy-off long enough to promise anything. The review that proposed the
      // conjunction credited it with closing a wedge; that wedge was not
      // reproduced here, and this test does not pretend otherwise.
      final fake = FakeWalletSession();
      final container = await mount(
        tester,
        fake: fake,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      );
      expect(drive(container), WalletSyncDrive.disabledByHost);
      expect(passesRun(container), isFalse);
      resetForeground();
    });

    testWidgets('an ordinary running wallet promises normally', (tester) async {
      // The counterfactual. Without it, "qualify the copy" and "qualify the copy
      // always" are indistinguishable, and the qualifier would quietly become a
      // permanent hedge on every money result.
      final fake = FakeWalletSession();
      final container = await mount(tester, fake: fake);
      expect(drive(container), WalletSyncDrive.running);
      expect(passesRun(container), isTrue);
      resetForeground();
    });
  });
}

/// A [FakeWalletSession] that RECORDS the start/stop call ORDER (the F7
/// ordering pin needs the sequence, not just the counts) and can PARK
/// [startSync] on a gate — the in-flight-start window the dispose-stop
/// chain must serialize behind. [log] lets two sessions record into ONE shared
/// list (the session-switch pin needs cross-session order), with [tag]
/// carrying the per-session attribution; the default is a private log with no
/// tag, so the single-session tests read 'start'/'stop' unchanged.
class _OrderRecordingSession extends FakeWalletSession {
  _OrderRecordingSession({List<String>? log, this.tag = ''})
    : calls = log ?? [];

  final List<String> calls;
  final String tag;

  // `startSyncGate` is INHERITED (#409 R2 promoted this subclass's local copy
  // onto the base fake, where the wedged-bridge pin also needs it). The base
  // parks in the same place this shadow did — after the count — so the recorded
  // order is unchanged: 'start' is logged, then the park begins.

  @override
  Future<void> startSync() async {
    calls.add('start$tag');
    await super.startSync();
  }

  @override
  Future<void> stopSync() {
    calls.add('stop$tag');
    return super.stopSync();
  }
}
