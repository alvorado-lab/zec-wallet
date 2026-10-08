import 'dart:async';

import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart' show DeviceLogLevel;
import 'package:zec_wallet_example/core/ffi/wallet_ffi.dart';

/// The FFI-init memo/deadline machinery (#356-F1 + the review
/// folds) — the trickiest boot logic, pinned against a scripted init so a
/// regression (e.g. the deadline sentinel collapsing back to a plain
/// TimeoutException, which re-wedges every retry by keeping a settled-failed
/// future memoized) turns red here instead of on a device.
void main() {
  tearDown(debugResetWalletFfi);

  test('TIMEOUT keeps the memo: the retry re-awaits the SAME init (never a '
      'second RustLib.init) and returns true the moment it lands', () {
    fakeAsync((async) {
      var initCalls = 0;
      final held = Completer<void>();
      debugResetWalletFfi(
        rustInit: () {
          initCalls++;
          return held.future;
        },
      );

      bool? first;
      initWalletFfi().then((v) => first = v);
      async.elapse(walletFfiInitTimeout);
      async.flushMicrotasks();
      expect(first, isFalse, reason: 'bounded — the splash never hangs');
      expect(initCalls, 1);

      // The slow init lands; the retry resolves true WITHOUT re-invoking.
      held.complete();
      bool? second;
      initWalletFfi().then((v) => second = v);
      async.flushMicrotasks();
      expect(second, isTrue);
      expect(initCalls, 1, reason: 'FRB forbids double-init — memo held');
    });
  });

  test('a FAILED init clears the memo: the next call attempts fresh', () {
    fakeAsync((async) {
      var initCalls = 0;
      debugResetWalletFfi(
        rustInit: () {
          initCalls++;
          return initCalls == 1
              ? Future<void>.error(StateError('load failed'))
              : Future<void>.value();
        },
      );

      bool? first;
      initWalletFfi().then((v) => first = v);
      async.flushMicrotasks();
      expect(first, isFalse);

      bool? second;
      initWalletFfi().then((v) => second = v);
      async.flushMicrotasks();
      expect(second, isTrue);
      expect(initCalls, 2, reason: 'a poisoned future is never re-awaited');
    });
  });

  test('a TIMED-OUT-then-FAILED memo costs ONE call: the stale failure is '
      'consumed and a fresh attempt runs in the same retry (never a dead tap, '
      'never a double attempt)', () {
    fakeAsync((async) {
      var initCalls = 0;
      final held = Completer<void>();
      debugResetWalletFfi(
        rustInit: () {
          initCalls++;
          return initCalls == 1 ? held.future : Future<void>.value();
        },
      );

      bool? first;
      initWalletFfi().then((v) => first = v);
      async.elapse(walletFfiInitTimeout);
      async.flushMicrotasks();
      expect(first, isFalse);

      // The abandoned init now FAILS (consumed silently — no zone error).
      held.completeError(StateError('late failure'));
      async.flushMicrotasks();

      // The user's single retry tap: consumes the stale failure, recurses
      // ONCE into a fresh attempt, and succeeds.
      bool? second;
      initWalletFfi().then((v) => second = v);
      async.flushMicrotasks();
      expect(second, isTrue);
      expect(initCalls, 2, reason: 'exactly one fresh attempt, no loop');
    });
  });

  test(
    'an init that itself fails with a TimeoutException is a FAILURE, not our '
    'deadline: the memo clears (the sentinel-vs-TimeoutException pin)',
    () {
      fakeAsync((async) {
        var initCalls = 0;
        debugResetWalletFfi(
          rustInit: () {
            initCalls++;
            return initCalls == 1
                ? Future<void>.error(TimeoutException('inner handshake'))
                : Future<void>.value();
          },
        );

        bool? first;
        initWalletFfi().then((v) => first = v);
        async.flushMicrotasks();
        expect(first, isFalse);

        bool? second;
        initWalletFfi().then((v) => second = v);
        async.flushMicrotasks();
        expect(
          second,
          isTrue,
          reason: 'a TimeoutException-failed init must not wedge the memo',
        );
        expect(initCalls, 2);
      });
    },
  );

  // FR-35: the SDK's device log is the HOST's to switch. Since stage S5 `row`
  // the define is the SECOND door, not the only one: the user's stored choice
  // outranks it, and with neither the build default applies (Detailed in
  // debug, Errors only otherwise; `resolveDeviceLogLevel`, tested in
  // `device_log_prefs_test.dart`). This pins only the define's own mapping:
  // exact words, and anything else is OFF, because a misspelt request for a
  // log must never read as a request for MORE of one.
  test('only the exact words ask for a device log; anything else is OFF', () {
    expect(deviceLogLevelFor('errors'), DeviceLogLevel.errors);
    expect(deviceLogLevelFor('detailed'), DeviceLogLevel.detailed);
    for (final other in [
      '',
      'off',
      'Detailed',
      'DETAILED',
      'detailed ',
      'debug',
      'trace',
      'on',
      'true',
      '1',
    ]) {
      expect(
        deviceLogLevelFor(other),
        DeviceLogLevel.off,
        reason: '"$other" is not a level this app asks for',
      );
    }
  });
}
