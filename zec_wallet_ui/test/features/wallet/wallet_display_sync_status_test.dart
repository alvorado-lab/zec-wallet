import 'package:fake_async/fake_async.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_display_sync_status.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';

/// The #399 posture-dwell layer: display surfaces must not flap when FR-21's
/// sub-second dial verdicts bounce the raw status Stalled↔Scanning on every
/// doomed retry. These tests pin the transition rule (pure) AND the notifier's
/// timer behavior (fake_async — real Timers under a controlled clock).
class _StubSyncNotifier extends SyncStatusNotifier {
  @override
  AsyncValue<SyncStatus> build() => const AsyncValue<SyncStatus>.loading();

  void emit(SyncStatus status) => state = AsyncValue<SyncStatus>.data(status);

  void emitError(Object error) =>
      state = AsyncValue<SyncStatus>.error(error, StackTrace.current);

  /// The raw provider's REBUILD envelope (a session flip emits a fresh
  /// loading before the new identity's first value).
  void emitLoading() => state = const AsyncValue<SyncStatus>.loading();
}

const _stalled = SyncStatus.stalled(reason: StallReason.endpointUnreachable);
const _stalledStorage = SyncStatus.stalled(reason: StallReason.storageFull);
const _upToDate = SyncStatus.upToDate(tip: 100);
const _scanning = SyncStatus.scanning(
  from: 100,
  to: 200,
  percent: 0.3,
  spendableReady: true,
  rewound: false,
);

void main() {
  group('nextDisplaySyncStatus (the pure rule)', () {
    test('healthy → healthy always adopts', () {
      expect(
        nextDisplaySyncStatus(_upToDate, _scanning),
        DisplayStatusEdge.adopt,
      );
      expect(
        nextDisplaySyncStatus(_scanning, _upToDate),
        DisplayStatusEdge.adopt,
      );
    });

    test('entering the Stalled posture dwells (transient-hiccup grace)', () {
      expect(
        nextDisplaySyncStatus(_upToDate, _stalled),
        DisplayStatusEdge.dwell,
      );
      expect(
        nextDisplaySyncStatus(_scanning, _stalledStorage),
        DisplayStatusEdge.dwell,
      );
    });

    test('leaving Stalled for an attempt CLAIM dwells — the retry blip', () {
      // Scanning/Connecting/Idle/Unknown after a stall are claims of an
      // attempt, not proof of recovery: exactly the sub-second blips a dead
      // link's retry schedule produces.
      for (final claim in const [
        _scanning,
        SyncStatus.connecting(),
        SyncStatus.idle(),
        SyncStatus.unknown(),
      ]) {
        expect(
          nextDisplaySyncStatus(_stalled, claim),
          DisplayStatusEdge.dwell,
          reason: '$claim after a stall must dwell',
        );
      }
    });

    test(
      'leaving Stalled for PROOF is instant — recovery never lags truth',
      () {
        expect(
          nextDisplaySyncStatus(_stalled, _upToDate),
          DisplayStatusEdge.adopt,
        );
        // The reserved Offline arm is the calmer sibling posture, not a flap.
        expect(
          nextDisplaySyncStatus(_stalled, const SyncStatus.offline()),
          DisplayStatusEdge.adopt,
        );
      },
    );

    test(
      'a Stalled reason change is instant — new information, not flicker',
      () {
        // e.g. connectivity → storage full, which must promote to RED now.
        expect(
          nextDisplaySyncStatus(_stalled, _stalledStorage),
          DisplayStatusEdge.adopt,
        );
      },
    );
  });

  group('WalletDisplaySyncStatusNotifier (timers under fake_async)', () {
    (ProviderContainer, _StubSyncNotifier) harness() {
      final sync = _StubSyncNotifier();
      final container = ProviderContainer(
        overrides: [syncStatusProvider.overrideWith(() => sync)],
      );
      container.listen(walletDisplaySyncStatusProvider, (_, _) {});
      return (container, sync);
    }

    SyncStatus? shown(ProviderContainer c) =>
        c.read(walletDisplaySyncStatusProvider).value;

    test('cold start into a stalled wallet shows the stall immediately — the '
        'dwell exists for edges, never for steady states', () {
      fakeAsync((async) {
        final sync = _StubSyncNotifier();
        final container = ProviderContainer(
          overrides: [syncStatusProvider.overrideWith(() => sync)],
        );
        // Initialize the raw provider and land its value BEFORE the display
        // is first built — the cold-launch-into-a-stalled-wallet shape.
        container.listen(syncStatusProvider, (_, _) {});
        sync.emit(_stalled);
        container.listen(walletDisplaySyncStatusProvider, (_, _) {});
        expect(shown(container), _stalled);
        container.dispose();
      });
    });

    test('healthy → Stalled waits out the dwell, then lands', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        expect(shown(container), _upToDate, reason: 'held during the dwell');
        async.elapse(walletStallPostureDwell - const Duration(milliseconds: 1));
        expect(shown(container), _upToDate);
        async.elapse(const Duration(milliseconds: 1));
        expect(shown(container), _stalled, reason: 'survived → adopted');
        container.dispose();
      });
    });

    test('a transient hiccup never raises the alarm: Stalled that recovers '
        'inside the dwell is invisible', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        final flips = <SyncStatus?>[];
        container.listen(
          walletDisplaySyncStatusProvider,
          (_, next) => flips.add(next.value),
        );
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(const Duration(seconds: 1));
        sync.emit(_upToDate); // recovered before the dwell elapsed
        async.elapse(walletStallPostureDwell * 2);
        expect(shown(container), _upToDate);
        expect(
          flips,
          isNot(contains(_stalled)),
          reason: 'the display never showed the transient stall',
        );
        container.dispose();
      });
    });

    test('the FR-21 flap: a sub-second Scanning retry blip during an outage '
        'never surfaces — no badge flicker, no re-announcement', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _stalled);

        final flips = <SyncStatus?>[];
        container.listen(
          walletDisplaySyncStatusProvider,
          (_, next) => flips.add(next.value),
        );
        // Three retry cycles: blip Scanning for 400ms, back to Stalled.
        for (var i = 0; i < 3; i++) {
          sync.emit(_scanning);
          async.elapse(const Duration(milliseconds: 400));
          sync.emit(_stalled);
          async.elapse(const Duration(seconds: 8));
        }
        expect(shown(container), _stalled);
        expect(
          flips,
          isEmpty,
          reason:
              'the display posture never moved across three doomed '
              'retry cycles',
        );
        container.dispose();
      });
    });

    test('a retry pass that KEEPS delivering is adopted after the dwell', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        sync.emit(_scanning);
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _scanning, reason: 'survived → real pass');
        container.dispose();
      });
    });

    test('UpToDate out of a shown stall is INSTANT (proof of health)', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _stalled);
        sync.emit(_upToDate);
        expect(shown(container), _upToDate, reason: 'no dwell on proof');
        container.dispose();
      });
    });

    test('a Stalled reason change inside the posture is instant', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        sync.emit(_stalledStorage);
        expect(
          shown(container),
          _stalledStorage,
          reason: 'connectivity → storage-full must promote NOW',
        );
        container.dispose();
      });
    });

    test('S207 H1 regression: a delivering pass with ADVANCING progress is '
        'adopted after ONE dwell — samples must not restart the clock', () {
      // The core reports progress every few blocks — far faster than the
      // dwell — so keying the re-arm on value equality starved adoption for
      // the entire catch-up and pinned "Sync paused" over a healthy scan
      // (probe-confirmed by the reliability review). The clock measures
      // CONTINUOUS time on the non-Stalled side, nothing else.
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _stalled);

        SyncStatus scanAt(double pct) => SyncStatus.scanning(
          from: (100 + pct * 100).round(),
          to: 200,
          percent: pct,
          spendableReady: true,
          rewound: false,
        );
        // A new progress sample every second — each a DIFFERENT value.
        sync.emit(scanAt(0.1));
        async.elapse(const Duration(seconds: 1));
        sync.emit(scanAt(0.2));
        async.elapse(const Duration(seconds: 1));
        sync.emit(scanAt(0.3));
        async.elapse(const Duration(milliseconds: 500)); // dwell deadline
        expect(
          shown(container),
          scanAt(0.3),
          reason:
              'adopted at the ORIGINAL deadline with the freshest sample — '
              'advancing values never re-arm the clock',
        );
        container.dispose();
      });
    });

    test('S207 M1 pin: realistic 1s/2s/4s early-backoff flap from healthy — '
        'the display lands on the stall within ~7s with BOUNDED flips '
        '(never the raw edge-per-second chatter)', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        final flips = <SyncStatus?>[];
        container.listen(
          walletDisplaySyncStatusProvider,
          (_, next) => flips.add(next.value),
        );
        // The doomed early-backoff cadence: fail at t=0, sub-second retry
        // blips at ~1s/3s, gaps growing 1s→2s→4s.
        sync.emit(_stalled); // t=0
        async.elapse(const Duration(seconds: 1));
        sync.emit(_scanning); // t=1 retry blip — same-side adopt (visible)
        async.elapse(const Duration(milliseconds: 400));
        sync.emit(_stalled); // t=1.4
        async.elapse(const Duration(milliseconds: 1600));
        sync.emit(_scanning); // t=3 second blip
        async.elapse(const Duration(milliseconds: 400));
        sync.emit(_stalled); // t=3.4 — the next gap (4s) outlasts the dwell
        async.elapse(const Duration(seconds: 4));
        expect(
          shown(container),
          _stalled,
          reason: 'the honest posture lands once a gap outlasts the dwell',
        );
        // The raw stream produced 6 edges in ~7s; the display is BOUNDED:
        // at most the two same-side Scanning moves plus the one settle.
        expect(
          flips.length,
          lessThanOrEqualTo(3),
          reason: 'the dwell bounds the flicker: ${flips.length} flips',
        );
        expect(flips.last, _stalled);
        container.dispose();
      });
    });

    test('S207 M2: the post-Try-now live window mirrors raw INSTANTLY in '
        'both directions, then the dwell resumes', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _stalled);

        container.read(walletDisplaySyncStatusProvider.notifier).followRawNow();
        // The restarted loop reports an attempt: adopted INSTANTLY (the
        // user is watching their own tap play out).
        sync.emit(_scanning);
        expect(shown(container), _scanning, reason: 'live inside the window');
        // …and a fast failure returns just as instantly (honest both ways).
        sync.emit(_stalled);
        expect(shown(container), _stalled);

        // The window closes → the ordinary dwell edges resume.
        async.elapse(walletTryNowLiveWindow);
        sync.emit(_scanning);
        expect(
          shown(container),
          _stalled,
          reason: 'post-window claims dwell again',
        );
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _scanning);
        container.dispose();
      });
    });

    test('followRawNow drops the pending dwell and mirrors the raw value — '
        'the Try-now tap is never a dead tap', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        sync.emit(_scanning); // the restarted loop reports an attempt
        expect(shown(container), _stalled, reason: 'dwell holds pre-tap');
        container.read(walletDisplaySyncStatusProvider.notifier).followRawNow();
        expect(shown(container), _scanning);
        container.dispose();
      });
    });

    test('S208 E: a repeat Try-now tap RESTARTS the live window — the first '
        'tap\'s timer must not close the second tap\'s window early', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        final display = container.read(
          walletDisplaySyncStatusProvider.notifier,
        );
        display.followRawNow();
        async.elapse(const Duration(seconds: 3));
        display.followRawNow(); // second tap at t+3s — window runs to t+7s
        // t+6.5s: past the FIRST window's deadline, inside the second's.
        async.elapse(const Duration(milliseconds: 3500));
        sync.emit(_scanning);
        expect(
          shown(container),
          _scanning,
          reason: 'still live — the restarted window owns the clock',
        );
        container.dispose();
      });
    });

    test('S208 D: the raw provider\'s REBUILD envelope (session flip) ends '
        'the live window — a pre-flip tap never bypasses the new identity\'s '
        'dwell discipline', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        container.read(walletDisplaySyncStatusProvider.notifier).followRawNow();
        sync.emitLoading(); // the flip
        sync.emit(_upToDate); // the new identity's first value (verbatim)
        expect(shown(container), _upToDate);
        // An entering-Stalled edge on the NEW wallet must DWELL again.
        sync.emit(_stalled);
        expect(
          shown(container),
          _upToDate,
          reason: 'the old tap\'s window is gone — dwell applies',
        );
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _stalled);
        container.dispose();
      });
    });

    test('S208 D: stopFollowingRaw ends the window with the sheet — the '
        'dwell resumes immediately', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled);
        async.elapse(walletStallPostureDwell);
        final display = container.read(
          walletDisplaySyncStatusProvider.notifier,
        );
        display.followRawNow();
        display.stopFollowingRaw(); // the sheet closed
        sync.emit(_scanning); // a retry blip right after
        expect(
          shown(container),
          _stalled,
          reason: 'no live mirror once nobody is watching — dwell holds',
        );
        async.elapse(walletStallPostureDwell);
        expect(shown(container), _scanning);
        container.dispose();
      });
    });

    test('dispose cancels the pending dwell timer cleanly', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_upToDate);
        sync.emit(_stalled); // dwell armed
        container.dispose();
        // Nothing throws when the deadline passes after teardown.
        async.elapse(walletStallPostureDwell * 2);
        expect(async.pendingTimers, isEmpty);
      });
    });

    test('loading/error envelopes mirror verbatim (the debounce is only '
        'between two data values)', () {
      fakeAsync((async) {
        final (container, sync) = harness();
        sync.emit(_stalled);
        expect(shown(container), _stalled);
        sync.emitError(StateError('stream fault'));
        expect(
          container.read(walletDisplaySyncStatusProvider).hasError,
          isTrue,
        );
        container.dispose();
      });
    });
  });
}
