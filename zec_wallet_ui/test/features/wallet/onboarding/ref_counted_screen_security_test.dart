import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/testing/fake_onboarding.dart';

/// The reference-counting decorator over the [ScreenSecurity] port (#344). The
/// raw adapter is a single window-level flag (Android FLAG_SECURE); more than
/// one secure screen can be alive at once, so the decorator keeps the flag
/// engaged while ANY holder is active — the first enable engages, the last
/// disable releases. These pin the coalescing, the shared engagement truth, the
/// native call ORDER under concurrency, the underflow clamp, error-resilience,
/// and the real widget-stack scenario the bug describes.

/// Records the raw adapter's call SEQUENCE so a test can assert coalescing AND
/// order (the count-only [FakeScreenSecurity] can't show order).
class _RecordingScreenSecurity implements ScreenSecurity {
  final List<String> calls = <String>[];

  @override
  Future<bool> enable() async {
    calls.add('enable');
    return true;
  }

  @override
  Future<void> disable() async => calls.add('disable');

  @override
  bool get isScreenshotBlockSupported => true;
}

/// An adapter whose transitions THROW — proves the decorator swallows a failing
/// engage/release: it never poisons the serialized chain and never breaks the
/// screen (the port's best-effort contract).
class _ThrowingScreenSecurity implements ScreenSecurity {
  int enableCount = 0;
  int disableCount = 0;

  @override
  Future<bool> enable() async {
    enableCount++;
    throw StateError('native enable blew up');
  }

  @override
  Future<void> disable() async {
    disableCount++;
    throw StateError('native disable blew up');
  }

  @override
  bool get isScreenshotBlockSupported => true;
}

/// An adapter whose transitions NEVER complete (a host handler that received the
/// call and forgot to `result(...)`) — proves the decorator's timeout unwedges
/// the serialized chain instead of pending forever.
class _HangingScreenSecurity implements ScreenSecurity {
  @override
  Future<bool> enable() => Completer<bool>().future;

  @override
  Future<void> disable() => Completer<void>().future;

  @override
  bool get isScreenshotBlockSupported => true;
}

/// An adapter whose FIRST engage is gated on a completer the test controls, so a
/// disable can be issued while that engage is genuinely still in flight (the
/// fast mount→unmount window). Records order so the release is provably ordered
/// AFTER the in-flight engage.
class _GatedEngageScreenSecurity implements ScreenSecurity {
  final List<String> calls = <String>[];
  final Completer<void> engageGate = Completer<void>();
  bool _firstEngage = true;

  @override
  Future<bool> enable() async {
    if (_firstEngage) {
      _firstEngage = false;
      await engageGate.future; // hold the engage in flight
    }
    calls.add('enable');
    return true;
  }

  @override
  Future<void> disable() async => calls.add('disable');

  @override
  bool get isScreenshotBlockSupported => true;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('RefCountedScreenSecurity', () {
    test(
      'a single enable engages the adapter once and reports its ack',
      () async {
        final inner = FakeScreenSecurity();
        final rc = RefCountedScreenSecurity(inner);

        expect(await rc.enable(), isTrue);
        expect(inner.enableCount, 1);
        expect(rc.activeCount, 1);
      },
    );

    test(
      'stacked enables engage the adapter ONCE; a piggyback reports the same '
      'engagement truth',
      () async {
        final inner = FakeScreenSecurity();
        final rc = RefCountedScreenSecurity(inner);

        final a = await rc.enable(); // 0→1: engages
        final b = await rc.enable(); // 1→2: piggyback, no new native call
        expect(a, isTrue);
        expect(b, isTrue);
        expect(
          inner.enableCount,
          1,
          reason: 'only the first holder engages the flag',
        );
        expect(rc.activeCount, 2);
      },
    );

    test(
      'only the LAST disable releases the adapter (the stacked-twin fix)',
      () async {
        final inner = FakeScreenSecurity();
        final rc = RefCountedScreenSecurity(inner);

        await rc.enable(); // A
        await rc.enable(); // B
        await rc.disable(); // B leaves — must NOT drop the flag under A
        expect(
          inner.disableCount,
          0,
          reason: 'a screen still holds the protection',
        );
        expect(rc.activeCount, 1);

        await rc.disable(); // A leaves — now release
        expect(inner.disableCount, 1);
        expect(inner.enableCount, 1);
        expect(rc.activeCount, 0);
      },
    );

    test(
      'interleaved enter/leave/enter re-engages and calls native IN ORDER',
      () async {
        final inner = _RecordingScreenSecurity();
        final rc = RefCountedScreenSecurity(inner);

        await rc.enable(); // A enters: engage
        await rc.disable(); // A leaves: release
        await rc.enable(); // B enters: engage again
        expect(inner.calls, ['enable', 'disable', 'enable']);
        expect(rc.activeCount, 1);
      },
    );

    test('rapid enable→disable→enable with NO await between serializes the '
        'native calls in order', () async {
      final inner = _RecordingScreenSecurity();
      final rc = RefCountedScreenSecurity(inner);

      // Fire all three in the same synchronous turn — the synchronous count
      // transitions (0→1, 1→0, 0→1) decide the order; the decorator must
      // serialize the native side to match.
      final f1 = rc.enable();
      final f2 = rc.disable();
      final f3 = rc.enable();
      await f1;
      await f2;
      await f3;

      expect(inner.calls, ['enable', 'disable', 'enable']);
      expect(rc.activeCount, 1);
    });

    test(
      'FAST UNMOUNT: a disable issued while the engage is STILL IN FLIGHT '
      'releases AFTER it — the flag never sticks ON (#333/#351 nav lifecycle)',
      () async {
        // The exact bug shape: a secure screen mounts (initState → enable) and is
        // popped a beat later (dispose → disable) BEFORE the native engage acked.
        // The synchronous count (0→1→0) plus the serialized chain must land the
        // native calls as engage-THEN-release so the window flag ends OFF; a
        // release that raced ahead of the pending engage would leave FLAG_SECURE
        // stuck on over the next, non-secret screen.
        final inner = _GatedEngageScreenSecurity();
        final rc = RefCountedScreenSecurity(inner);

        final enableF = rc
            .enable(); // 0→1: engage queued, but GATED (in flight)
        final disableF = rc.disable(); // 1→0: release queued behind the engage
        // Neither native call has run yet — the engage is still awaiting its gate.
        expect(inner.calls, isEmpty);
        expect(
          rc.activeCount,
          0,
          reason: 'the synchronous count already balanced',
        );

        inner.engageGate.complete(); // the native engage finally acks
        await enableF;
        await disableF;

        // Ordered engage→release, and the flag is released (count 0): no leak.
        expect(inner.calls, ['enable', 'disable']);
        expect(rc.activeCount, 0);
      },
    );

    test('re-engages after a full release', () async {
      final inner = FakeScreenSecurity();
      final rc = RefCountedScreenSecurity(inner);

      await rc.enable();
      await rc.disable();
      await rc.enable();
      await rc.disable();
      expect(inner.enableCount, 2);
      expect(inner.disableCount, 2);
      expect(rc.activeCount, 0);
    });

    test(
      'an unbalanced disable is CLAMPED — the count never goes negative',
      () async {
        final inner = FakeScreenSecurity();
        final rc = RefCountedScreenSecurity(inner);

        // No active holder → clamp, no native call (the debug report on this path
        // is expected in the test log; it must not throw).
        await rc.disable();
        expect(inner.disableCount, 0);
        expect(rc.activeCount, 0);

        // The clamp must not have corrupted the counter: a real enable still does
        // a genuine 0→1 engage.
        expect(await rc.enable(), isTrue);
        expect(inner.enableCount, 1);
        expect(rc.activeCount, 1);
      },
    );

    test(
      'a capable-but-unwired adapter (Noop) reports UNPROTECTED, even stacked',
      () async {
        final rc = RefCountedScreenSecurity(const NoopScreenSecurity());

        expect(await rc.enable(), isFalse);
        expect(
          await rc.enable(),
          isFalse,
          reason: 'still honestly unprotected',
        );
        expect(rc.isScreenshotBlockSupported, isFalse);
      },
    );

    test('a throwing adapter is swallowed — enable reports UNPROTECTED and the '
        'transition chain stays healthy', () async {
      final inner = _ThrowingScreenSecurity();
      final rc = RefCountedScreenSecurity(inner);

      expect(
        await rc.enable(),
        isFalse,
        reason: 'a failed engage reads as unprotected',
      );
      expect(inner.enableCount, 1);

      // The chain is NOT poisoned: a later balanced pair still transitions.
      await rc.disable(); // release throws internally, swallowed
      expect(inner.disableCount, 1);
      expect(rc.activeCount, 0);

      // And a fresh cycle still runs.
      expect(await rc.enable(), isFalse);
      expect(inner.enableCount, 2);
    });

    test('a HUNG native call times out as UNPROTECTED and does NOT wedge the '
        'chain — later transitions still run', () async {
      final inner = _HangingScreenSecurity();
      final rc = RefCountedScreenSecurity(
        inner,
        transitionTimeout: const Duration(milliseconds: 20),
      );

      // enable() never acks natively; the decorator must still resolve
      // (honestly unprotected) instead of hanging forever.
      expect(await rc.enable(), isFalse);
      expect(rc.activeCount, 1);

      // The serialized chain is NOT poisoned by the hung engage: a later disable
      // still completes (also via its timeout).
      await rc.disable();
      expect(rc.activeCount, 0);

      // And a fresh cycle still runs — the chain stayed healthy.
      expect(await rc.enable(), isFalse);
      expect(rc.activeCount, 1);
    });

    test('isScreenshotBlockSupported delegates to the wrapped adapter', () {
      expect(
        RefCountedScreenSecurity(
          FakeScreenSecurity(isScreenshotBlockSupported: true),
        ).isScreenshotBlockSupported,
        isTrue,
      );
      expect(
        RefCountedScreenSecurity(
          const NoopScreenSecurity(),
        ).isScreenshotBlockSupported,
        isFalse,
      );
    });

    test('inner exposes the wrapped adapter (for the co-wiring unwrap)', () {
      const noop = NoopScreenSecurity();
      expect(RefCountedScreenSecurity(noop).inner, same(noop));
    });
  });

  // The real-widget proof: two secure screens on the nav stack, popped
  // top-first, must not drop the flag while the lower one still shows a secret.
  testWidgets(
    'two stacked secure screens keep the flag until BOTH are gone (real lifecycle)',
    (tester) async {
      final inner = FakeScreenSecurity();
      final rc = RefCountedScreenSecurity(inner);

      // A plain, non-secure root so BOTH secure screens sit above it and can be
      // popped (GoRouter refuses to pop the last route).
      final router = GoRouter(
        initialLocation: '/home',
        routes: [
          GoRoute(
            path: '/home',
            builder: (c, s) => const Scaffold(body: Text('home')),
          ),
          GoRoute(
            path: '/a',
            builder: (c, s) => const _SecureProbe(label: 'A'),
          ),
          GoRoute(
            path: '/b',
            builder: (c, s) => const _SecureProbe(label: 'B'),
          ),
        ],
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [screenSecurityProvider.overrideWithValue(rc)],
          child: MaterialApp.router(routerConfig: router),
        ),
      );
      await tester.pumpAndSettle();
      // The non-secure root touches nothing.
      expect(inner.enableCount, 0);
      expect(rc.activeCount, 0);

      // Push secure screen A: the flag engages exactly once.
      router.push('/a');
      await tester.pumpAndSettle();
      expect(inner.enableCount, 1);
      expect(inner.disableCount, 0);
      expect(rc.activeCount, 1);

      // Push B over A — two secure screens live at once.
      router.push('/b');
      await tester.pumpAndSettle();
      expect(
        inner.enableCount,
        1,
        reason: 'the flag is already up — no re-engage',
      );
      expect(rc.activeCount, 2);

      // Pop B. A still shows its secret — the flag MUST stay up.
      router.pop();
      await tester.pumpAndSettle();
      expect(inner.disableCount, 0, reason: 'A still holds the protection');
      expect(rc.activeCount, 1);

      // Pop A. Now, and only now, the flag releases.
      router.pop();
      await tester.pumpAndSettle();
      expect(inner.disableCount, 1);
      expect(rc.activeCount, 0);
    },
  );
}

/// Mirrors the EXACT screen-security pattern all 5 secure screens use — capture
/// the port in `initState` + `enable()`, release in `dispose()` — so this proves
/// the decorator under real Flutter push/pop/dispose ordering without dragging
/// in a specific screen's provider scaffolding.
class _SecureProbe extends ConsumerStatefulWidget {
  const _SecureProbe({required this.label});

  final String label;

  @override
  ConsumerState<_SecureProbe> createState() => _SecureProbeState();
}

class _SecureProbeState extends ConsumerState<_SecureProbe> {
  late final ScreenSecurity _security;

  @override
  void initState() {
    super.initState();
    _security = ref.read(screenSecurityProvider);
    unawaited(_security.enable());
  }

  @override
  void dispose() {
    unawaited(_security.disable());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      Scaffold(body: Center(child: Text(widget.label)));
}
