import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/reconnect_kick.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_sync_controller.dart';

import 'package:zec_wallet_ui/testing.dart';

/// #404 — the reconnect auto-heal.
///
/// The measured gap (on device): the sync retry ladder lives in the
/// spawned loop task, so a real background/resume already resets it — but a
/// FOREGROUND reconnect (airplane toggle, wifi switch, tunnel exit) fires no
/// lifecycle event, and the badge read "Sync paused" for ~80s after the network
/// was demonstrably back, up to 256s+ after a longer outage.
///
/// The kick turns a platform network-available tick into the same stop+start
/// `tryNow()` the sync sheet's button does by hand. These pins cover the gate,
/// because a kick that fires on the wrong state is worse than no kick: it would
/// restart a healthy loop, or start one in the background past the battery
/// policy.
void main() {
  final binding = TestWidgetsFlutterBinding.ensureInitialized();

  void goPaused() {
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
  }

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

  /// A reachability port a test ticks by hand. Broadcast + re-created per test
  /// so one test's leftover listener can never tick the next one's kick.
  late StreamController<void> ticks;
  setUp(() => ticks = StreamController<void>.broadcast());
  tearDown(() => ticks.close());

  Future<ProviderContainer> mount(
    WidgetTester tester, {
    required FakeWalletSession fake,
    List<Override> extraOverrides = const [],
    bool bornPaused = false,
  }) async {
    resetForeground();
    if (bornPaused) goPaused();
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        walletNetworkReachabilityProvider.overrideWithValue(
          _FakeReachability(ticks.stream),
        ),
        ...extraOverrides,
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    // All three must be BUILT: the drive owns `tryNow()`, the kick owns the
    // gate, and the RAW status feeds that gate. Production reaches the raw
    // provider through the display one the wallet screen watches; here the
    // listen is what subscribes the fake's stream, so a `push` lands.
    container.listen(syncStatusProvider, (_, _) {});
    container.listen(walletSyncControllerProvider, (_, _) {});
    container.listen(walletReconnectKickProvider, (_, _) {});
    await tester.pumpAndSettle();
    return container;
  }

  /// Drive the RAW status the kick gates on. `syncStatusProvider` is fed by the
  /// session's stream in the fake, so push through it rather than overriding —
  /// the gate must read the same provider production does.
  Future<void> emit(
    WidgetTester tester,
    FakeWalletSession fake,
    SyncStatus status,
  ) async {
    fake.push(status);
    await tester.pumpAndSettle();
  }

  testWidgets('a tick on a LIVE STALL resets the ladder (stop+start)', (
    tester,
  ) async {
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    expect(fake.startCount, 1, reason: 'the loop auto-started');
    expect(fake.stopCount, 0);

    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    ticks.add(null);
    await tester.pumpAndSettle();

    // stop+start is the WHOLE mechanism: the backoff lives in the loop task, so
    // only tearing it down and respawning resets the ladder.
    expect(fake.stopCount, 1);
    expect(fake.startCount, 2);
    expect(container.read(walletReconnectKickProvider), 1);
    await tester.pump(walletReconnectKickCooldown); // drain the cooldown timer
    resetForeground();
  });

  testWidgets('a tick while HEALTHY does nothing — the kick is a stall '
      'shortcut, not a poll', (tester) async {
    // `tryNow()` gates on the DRIVE being running; it does not know the status.
    // Without the Stalled gate here, every network-available event would
    // restart a perfectly good scanning loop.
    final fake = FakeWalletSession();
    await mount(tester, fake: fake);
    await emit(tester, fake, const SyncStatus.upToDate(tip: 100));

    ticks.add(null);
    await tester.pumpAndSettle();

    expect(fake.stopCount, 0, reason: 'nothing to shortcut');
    expect(fake.startCount, 1);
    resetForeground();
  });

  testWidgets('a tick while BACKGROUNDED never starts sync — the battery '
      'policy owns that decision', (tester) async {
    // The money-adjacent one. A backgrounded phone regaining network must not
    // resume scanning: `tryNow()`'s `running` gate and its `_wasPaused`
    // re-check inside the serialized link both foreclose it (review M1),
    // and this pins that the new caller does not route around them.
    final fake = FakeWalletSession();
    await mount(tester, fake: fake);
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    goPaused();
    await tester.pumpAndSettle();
    final startsAfterPause = fake.startCount;

    ticks.add(null);
    await tester.pumpAndSettle();

    expect(
      fake.startCount,
      startsAfterPause,
      reason: 'no background restart, ever',
    );
    resetForeground();
  });

  testWidgets('a tick under a HOST SYNC-OFF policy is a no-op — never a way '
      'past a deliberate off', (tester) async {
    final fake = FakeWalletSession();
    await mount(
      tester,
      fake: fake,
      extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
    );
    expect(fake.startCount, 0, reason: 'the policy never let it start');

    ticks.add(null);
    await tester.pumpAndSettle();

    expect(fake.startCount, 0);
    expect(fake.stopCount, 0);
    resetForeground();
  });

  testWidgets('a BURST of ticks acts once — Android fires onAvailable per '
      'network, and a flapping link fires repeatedly', (tester) async {
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );

    ticks.add(null);
    ticks.add(null);
    ticks.add(null);
    await tester.pumpAndSettle();

    expect(
      container.read(walletReconnectKickProvider),
      1,
      reason: 'wifi + cellular + a flap must not queue three stop/start pairs',
    );
    expect(fake.stopCount, 1);

    // The kick restarted the loop, so model what that loop does next (#409 R1):
    // the core's stop publishes `Idle` (the fake does too, faithfully), and the
    // respawned loop re-stalls against the same dead endpoint. Without this the
    // tick below reads the parked `Idle` and the stall gate declines — which
    // would make this test fail for a reason it is not about.
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );

    // …and the window REOPENS, or a second genuine outage would never recover.
    // The FIRST window is the BASE one (#407 R3 escalates from the second kick
    // on — `base << (unresolved - 1)`), so one base pump is exactly right here;
    // the escalation itself is pinned in its own test below.
    await tester.pump(walletReconnectKickCooldown);
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(container.read(walletReconnectKickProvider), 2);
    expect(fake.stopCount, 2);
    // Drain the SECOND (doubled) window's timer, not the base one.
    await tester.pump(walletReconnectKickCooldown * 4);
    resetForeground();
  });

  testWidgets('#407 R3: the cooldown ESCALATES while the stall persists and '
      'RESETS the moment it clears — a flapping link must not out-run the very '
      'backoff ladder this kick shortcuts', (tester) async {
    // MEASURED failure this pins: with a fixed 10s period, a 3s flap against an
    // unreachable endpoint produced 50 kicks / 50 stopSync / 51 startSync over
    // 600s — one full teardown+respawn every 12s, forever, because every
    // `startSync` resets the Rust loop's backoff to its 1s initial value. The
    // ladder it shortcuts was designed to settle at ONE attempt per 600s.
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );

    // Four kicks. Written as an explicit walk rather than a loop: the rounds
    // are NOT symmetric, and a loop with conditionals hid that.
    //
    // Round 1's window is the BASE one, which is also the burst coalescer
    // (Android reports `onAvailable` per network, so one transition arrives as
    // two or three ticks milliseconds apart). Ticks inside it are duplicates and
    // are dropped. From round 2 the window is ESCALATED, and a tick there is a
    // genuinely later transition — it must be REMEMBERED, because nothing
    // re-sends it once the link is stable.
    //
    // Each escalated round therefore proves both halves with one tick: it must
    // not fire early (or the escalation is decorative) and it must not vanish.
    //
    // After every kick the loop it restarted is modelled: the core's stop
    // publishes `Idle` (`sync_controller.rs:328` — the fake now does too) and
    // the respawned loop re-stalls against the still-dead endpoint.
    Future<void> reStall() => emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );

    // Round 1 — base window (10s).
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(container.read(walletReconnectKickProvider), 1, reason: 'kick 1');
    await reStall();
    await tester.pump(walletReconnectKickCooldown);

    // Round 2 — fires on its own tick; window becomes 20s.
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(container.read(walletReconnectKickProvider), 2, reason: 'kick 2');
    await reStall();

    // Rounds 3 and 4 arrive via a tick delivered INSIDE the escalated window.
    var window = walletReconnectKickCooldown * 2; // 20s, then 40s
    for (var kick = 3; kick <= 4; kick++) {
      await tester.pump(walletReconnectKickCooldown);
      ticks.add(null);
      await tester.pumpAndSettle();
      expect(
        container.read(walletReconnectKickProvider),
        kick - 1,
        reason:
            'window is ${window.inSeconds}s — a base pump must not reopen it, '
            'or the escalation is decorative',
      );
      await tester.pump(window - walletReconnectKickCooldown);
      await tester.pumpAndSettle();
      expect(
        container.read(walletReconnectKickProvider),
        kick,
        reason:
            'the tick delivered inside the escalated window was REMEMBERED and '
            'fired when it expired — dropping it would strand the user on the '
            'full Rust ladder with no further tick coming',
      );
      await reStall();
      window *= 2;
    }

    final kicksWhileStalled = container.read(walletReconnectKickProvider);
    expect(kicksWhileStalled, 4);

    // THE RESET LEG. The stall clears — the kick WORKED — so the next outage
    // must get the fast 10s response back, not the tail of a 160s window.
    await emit(tester, fake, const SyncStatus.upToDate(tip: 100));
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(
      container.read(walletReconnectKickProvider),
      kicksWhileStalled + 1,
      reason: 'a cleared stall cancels the escalated window immediately',
    );
    // Same loop modelling as the escalation body above: that kick stopped the
    // loop (`Idle`) and the respawn re-stalls. Without it the tick below reads
    // the parked `Idle` and never reaches the exponent this leg is pinning.
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    await tester.pump(walletReconnectKickCooldown);
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(
      container.read(walletReconnectKickProvider),
      kicksWhileStalled + 2,
      reason:
          'and the EXPONENT reset too — one base window reopens it, which it '
          'would not have if the counter had kept climbing',
    );
    await tester.pump(walletReconnectKickCooldown * 4); // drain
    resetForeground();
  });

  testWidgets('#409 R1: the kick\'s OWN stop is not evidence it worked — the '
      'stop\'s Idle must not reset the escalation (nor must Connecting, which '
      'has no producer yet), but a real Scanning does', (tester) async {
    // The defect this pins is self-certification. `SyncController::stop()` ends
    // by publishing `Idle` (`sync_controller.rs:328`) and EVERY kick is a
    // stop+start, so a reset listener that accepts `Idle` lets each kick clear
    // its own exponent and cancel its own live cooldown — the ladder can never
    // climb. Measured on device before the fix: 16 wifi cycles, 16 kicks, one
    // per tick, `backoff_secs` pinned at 8 where the same phone reached 256
    // with no kick at all. `Idle` is the LOAD-BEARING leg here.
    //
    // The `Connecting` leg below is FORWARD-LOOKING, not observed: nothing in
    // the core emits `Connecting` today (every occurrence is a reader in
    // `tor_status.rs`; `wallet_sync_controller.dart`'s class doc records the
    // same). It pins that the allowlist stays closed if an emitter lands later.
    // An earlier version of this test asserted `Connecting` was what a fresh
    // start publishes — false, and the reason it is spelled out here.
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );

    // Kick 1 (base window), then kick 2 — which puts the window at 20s.
    ticks.add(null);
    await tester.pumpAndSettle();
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    await tester.pump(walletReconnectKickCooldown);
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(container.read(walletReconnectKickProvider), 2);

    // THE FORWARD-LOOKING LEG. `Connecting` has no emitter in the core today;
    // if one lands (the arti bootstrap was its intended producer) it would be
    // published by a fresh START — i.e. by the kick's own restart — so counting
    // it as recovery would zero the exponent and let ONE base window reopen the
    // gate. Pinned now so the allowlist cannot be widened by accident later.
    await emit(tester, fake, const SyncStatus.connecting());
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    await tester.pump(walletReconnectKickCooldown);
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(
      container.read(walletReconnectKickProvider),
      2,
      reason:
          'Connecting is the kick\'s own restart, not proof the endpoint '
          'answered — one base window must NOT reopen a 20s cooldown',
    );

    // AND THE CONTRAST, so this pins the allowlist rather than merely "never
    // reset": a genuine Scanning IS the loop consuming blocks, so it resets and
    // the next base window does reopen the gate.
    await emit(
      tester,
      fake,
      const SyncStatus.scanning(
        from: 0,
        to: 100,
        percent: 0.5,
        spendableReady: false,
        rewound: false,
      ),
    );
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    await tester.pump(walletReconnectKickCooldown);
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(
      container.read(walletReconnectKickProvider),
      3,
      reason:
          'a real Scanning is evidence the kick worked — the fast window '
          'is earned back',
    );
    await tester.pump(walletReconnectKickCooldown * 4); // drain
    resetForeground();
  });

  testWidgets('#409 R1: a tick arriving INSIDE the cooldown is remembered, not '
      'discarded — the "you are out of the tunnel" signal fires once the '
      'window opens', (tester) async {
    // Android fires `onAvailable` once PER NETWORK TRANSITION, so the tick that
    // means "the link is back for real" may be the only one there will ever be:
    // nothing re-fires once the link is stable. Dropping it was nearly free
    // while the window was effectively pinned at 10s — which is exactly what
    // the broken reset predicate made it. Fixing the escalation is what makes
    // 20/40/…/600s reachable, and therefore what grows this discard window
    // ~60x. A tick lost in there leaves recovery to the Rust ladder alone, the
    // very wait this kick exists to shortcut.
    final fake = FakeWalletSession();
    final container = await mount(tester, fake: fake);
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );

    // Two kicks first, so the window is ESCALATED (20s) rather than the base
    // one. At base length the window is the burst coalescer and a tick inside
    // it is a duplicate of the tick that just fired; past base length it is a
    // genuinely later network transition, and that is the case worth keeping.
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(container.read(walletReconnectKickProvider), 1);
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );
    await tester.pump(walletReconnectKickCooldown);
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(container.read(walletReconnectKickProvider), 2);
    await emit(
      tester,
      fake,
      const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
    );

    // THE REAL RECONNECT lands mid-window, and no further tick will come.
    await tester.pump(const Duration(seconds: 2));
    ticks.add(null);
    await tester.pumpAndSettle();
    expect(
      container.read(walletReconnectKickProvider),
      2,
      reason: 'the rate limit still holds — it must not act immediately',
    );

    // Window opens: the remembered tick is replayed exactly once.
    await tester.pump(walletReconnectKickCooldown * 2);
    await tester.pumpAndSettle();
    expect(
      container.read(walletReconnectKickProvider),
      3,
      reason:
          'the deferred tick fired when the window opened — before this '
          'fix it was dropped and nothing re-armed, so recovery fell back to '
          'the full Rust ladder',
    );

    await tester.pump(walletReconnectKickCooldown * 8); // drain
    resetForeground();
  });

  testWidgets('#407 R9: backgrounding RELEASES the reachability subscription '
      'and a real resume re-takes it — the native callback must not stay '
      'registered where the kick provably cannot act', (tester) async {
    // The kick cannot do anything while backgrounded: `tryNow()` returns false
    // under a `suspended` drive, and its serialized link re-checks `_wasPaused`
    // at execution time. A live subscription there is pure cost — and cancelling
    // the SUBSCRIPTION is what makes it real rather than cosmetic, because that
    // is what drives the EventChannel's `cancel` down to
    // `unregisterNetworkCallback` / `NWPathMonitor.cancel()`. Counting listens
    // and cancels is therefore the assertion that matters; counting ticks would
    // pass even with the callback still registered.
    var listens = 0;
    var cancels = 0;
    final counted = StreamController<void>.broadcast(
      onListen: () => listens++,
      onCancel: () => cancels++,
    );
    addTearDown(counted.close);
    // Its OWN container rather than `mount`: that helper already overrides the
    // reachability provider with the shared `ticks` stream, and riverpod
    // rejects a second override of the same provider.
    final fake = FakeWalletSession();
    resetForeground();
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        walletNetworkReachabilityProvider.overrideWithValue(
          _FakeReachability(counted.stream),
        ),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(syncStatusProvider, (_, _) {});
    container.listen(walletSyncControllerProvider, (_, _) {});
    container.listen(walletReconnectKickProvider, (_, _) {});
    await tester.pumpAndSettle();
    expect(listens, 1, reason: 'foreground: subscribed');
    expect(cancels, 0);

    goPaused();
    await tester.pumpAndSettle();
    expect(
      cancels,
      1,
      reason:
          'THE POINT: backgrounding releases it, so the OS-level callback goes '
          'away instead of running where no kick can act',
    );

    // Focus/shade noise (`inactive → resumed`) is NOT a resume and must not
    // re-register — otherwise every notification-shade pull re-arms the native
    // callback (the flutter-patterns Stream-Lifecycle rule).
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pumpAndSettle();
    expect(listens, 1, reason: 'still only the original listen');

    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(listens, 2, reason: 'a REAL resume re-takes the subscription');
    resetForeground();
  });

  testWidgets('the default port is a genuine NO-OP where no plugin answers — '
      'no tick, and NOTHING reported to FlutterError (#407 R2)', (
    tester,
  ) async {
    // Desktop, web, and any host that skipped the companion plugin land here.
    // The contract is silence, not a swallowed failure that reads like
    // coverage: the stream must never tick AND the activation must not report.
    //
    // THIS PIN WAS VACUOUS UNTIL #407 R2, and the mechanism is worth naming
    // because it is the second time (after the #401 R6a hoisted-upgrade pin)
    // that a property was asserted in the one environment where it cannot be
    // observed. `EventChannel.receiveBroadcastStream` awaits
    // `invokeMethod('listen')` in its `onListen` and hands the failure to
    // `FlutterError.reportError` — never to the stream, so the adapter's
    // `.handleError()` could not catch it. Inside FakeAsync the unmocked
    // platform-message reply never lands at all, so the failure never even
    // happened during the test: it passed against a broken adapter. Both halves
    // are fixed here — a REAL async turn so activation actually completes, and
    // an assertion on the channel that carries the failure.
    final reported = <FlutterErrorDetails>[];
    final previousOnError = FlutterError.onError;
    FlutterError.onError = reported.add;
    addTearDown(() => FlutterError.onError = previousOnError);

    final fake = FakeWalletSession();
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const SizedBox()),
    );
    container.listen(walletReconnectKickProvider, (_, _) {});
    await tester.pumpAndSettle();
    // THE REAL ASYNC TURN, and it must be a NONZERO real delay followed by a
    // pump — MEASURED, not assumed. A probe against the broken adapter showed
    // `pump`, `pumpAndSettle`, and `runAsync(Duration.zero)` all observe
    // NOTHING; only `runAsync(50ms) + pump` lets the unmocked platform-message
    // round-trip resolve and the report land. Anything weaker here and this
    // assertion goes back to being unfalsifiable, which is exactly how the
    // pre-#407 version of this test passed against a broken adapter.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pump();

    expect(
      container.read(walletNetworkReachabilityProvider),
      isA<EventChannelNetworkReachability>(),
      reason: 'the unoverridden default is the channel adapter',
    );
    expect(container.read(walletReconnectKickProvider), 0);
    expect(fake.stopCount, 0, reason: 'no handler ⇒ nothing ever fires');
    expect(
      reported.map((d) => d.exception.toString()).toList(),
      isEmpty,
      reason:
          'a host with no companion plugin must not emit a MissingPluginException '
          'per wallet-screen mount — where FlutterError.onError is wired to a '
          'crash reporter that is a third-party beacon naming our channel, once '
          'per open, from a privacy wallet',
    );
    resetForeground();
  });
}

class _FakeReachability implements NetworkReachability {
  const _FakeReachability(this.onAvailable);

  @override
  final Stream<void> onAvailable;
}
