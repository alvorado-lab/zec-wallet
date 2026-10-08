// Stage S5 `row` (docs/plan/stage-5-the-device-log-a-user-can-send.md §2):
// the ring the example keeps, its re-arm after a Delete, and what Share sends.
import 'dart:async' show runZonedGuarded;
import 'dart:convert' show utf8;

import 'package:flutter/widgets.dart' show AppLifecycleState;
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet/zec_wallet.dart' show DeviceLogLevel;
import 'package:zec_wallet_example/core/ffi/wallet_ffi.dart';
import 'package:zec_wallet_example/core/logging/device_log.dart';
import 'package:zec_wallet_example/core/logging/device_log_prefs.dart';

import '../../support/fake_device_log.dart';
import '../../support/fake_tor_plugin.dart' show LoggingProvisioner;

DeviceLogController _controller(FakeDeviceLogBridge bridge) =>
    DeviceLogController(bridge: bridge, store: (_) async {});

void main() {
  late List<String> log;
  late FakeDeviceLogBridge bridge;

  setUp(() {
    log = [];
    bridge = FakeDeviceLogBridge(calls: log);
    SharedPreferences.setMockInitialValues({});
  });

  tearDown(debugResetWalletFfi);

  test('the_ring_keeps_the_last_500_lines', () async {
    final c = _controller(bridge)..arm(DeviceLogLevel.detailed);
    for (var i = 0; i <= deviceLogRingCapacity; i++) {
      bridge.emit('line $i');
    }
    await pumpEventQueue();
    expect(deviceLogRingCapacity, 500);
    expect(c.lines, hasLength(500));
    // The OLDEST went; the newest stays.
    expect(c.lines.first, endsWith('line 1'));
    expect(c.lines.last, endsWith('line 500'));
  });

  test('a_rearm_marks_the_ring_boundary', () async {
    final c = _controller(bridge)..arm(DeviceLogLevel.detailed);
    final wallet = LoggingProvisioner(log)..exists = true;
    final provisioner = DeviceLogRearmingProvisioner(
      inner: wallet,
      rearm: () => rearmDeviceLogAfterWipe(c, readStored: () async => null),
    );
    bridge
      ..emit('before one')
      ..emit('before two');
    await pumpEventQueue();

    await provisioner.deleteWallet();
    expect(c.lines, hasLength(3));
    expect(c.lines[0], endsWith('before one'));
    expect(c.lines[1], endsWith('before two'));
    expect(c.lines[2], endsWith(deviceLogBoundaryLine));

    bridge.emit('after the first');
    await pumpEventQueue();
    wallet.failDelete = StateError('the wipe faulted');
    await expectLater(provisioner.deleteWallet(), throwsStateError);
    expect(c.lines, hasLength(5));
    expect(c.lines[3], endsWith('after the first'));
    expect(c.lines[4], endsWith(deviceLogBoundaryLine));
    expect(
      c.lines.where((l) => l.endsWith(deviceLogBoundaryLine)),
      hasLength(2),
    );
  });

  test('a_delete_rearms_the_log_after_the_wipe_returns_or_throws', () async {
    SharedPreferences.setMockInitialValues({deviceLogLevelPrefsKey: 'errors'});
    final c = _controller(bridge)..arm(DeviceLogLevel.errors);
    final wallet = LoggingProvisioner(log)..exists = true;
    final provisioner = DeviceLogRearmingProvisioner(
      inner: wallet,
      rearm: () => rearmDeviceLogAfterWipe(c),
    );
    expect(log, ['log.set errors', 'log.watch']);

    log.clear();
    await provisioner.deleteWallet();
    expect(log, ['wallet.wipe', 'log.set errors', 'log.watch']);

    log.clear();
    wallet.failForceDelete = StateError('the wipe faulted');
    await expectLater(provisioner.forceDeleteWallet(), throwsStateError);
    expect(log, ['wallet.forceWipe', 'log.set errors', 'log.watch']);
    expect(bridge.streams, hasLength(3), reason: 'a new subscription each');
  });

  testWidgets('the_subscription_survives_a_pause', (tester) async {
    final c = _controller(bridge)..arm(DeviceLogLevel.detailed);
    for (final s in [
      AppLifecycleState.inactive,
      AppLifecycleState.hidden,
      AppLifecycleState.paused,
    ]) {
      tester.binding.handleAppLifecycleStateChanged(s);
    }
    await tester.pump();
    // A background sync's line, while paused.
    bridge.emit('while paused');
    await tester.pump();
    expect(c.lines, hasLength(1));
    expect(c.lines.single, endsWith('while paused'));
    expect(bridge.streams, hasLength(1), reason: 'never re-subscribed');
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });

  test(
    'a_mail_link_share_is_capped_at_8_kb_and_says_how_many_lines_it_left_out',
    () {
      final lines = [
        for (var i = 0; i < deviceLogRingCapacity; i++) 'line $i ${'x' * 480}',
      ];
      const header = 'zec_wallet 9.9.9-test, device log at detailed';
      final capped = deviceLogShareText(
        header: header,
        lines: lines,
        mailLink: true,
      );
      expect(utf8.encode(capped).length, lessThanOrEqualTo(8 * 1024));
      final out = capped.split('\n');
      expect(out.first, header);
      final kept = out.length - 2;
      expect(kept, greaterThan(0));
      expect(
        out[1],
        '${lines.length - kept} earlier lines left out: '
        'a mail link carries only the latest',
      );
      expect(out.last, lines.last, reason: 'the newest line is kept');

      // Everywhere else the whole ring goes.
      final whole = deviceLogShareText(
        header: header,
        lines: lines,
        mailLink: false,
      );
      expect(whole.split('\n'), [header, ...lines]);
    },
  );

  test('a_mail_link_share_stays_under_the_cap_with_an_oversized_header', () {
    final lines = [for (var i = 0; i < 20; i++) 'line $i'];
    final header = 'zec_wallet ${'v' * 10000}';
    final capped = deviceLogShareText(
      header: header,
      lines: lines,
      mailLink: true,
    );
    expect(utf8.encode(capped).length, lessThanOrEqualTo(8 * 1024));
    final out = capped.split('\n');
    expect(
      utf8.encode(out.first).length,
      lessThanOrEqualTo(deviceLogMailLinkHeaderCapBytes),
      reason: 'the header is cut, so it cannot crowd out the log',
    );
    expect(out.first, startsWith('zec_wallet vvv'));
    expect(out.last, lines.last, reason: 'the newest line is still kept');
  });

  test('picking_off_stops_the_live_subscription', () async {
    final c = _controller(bridge)..arm(DeviceLogLevel.detailed);
    bridge.emit('kept');
    await pumpEventQueue();
    expect(c.lines, hasLength(1));

    // A line the SDK posted just before its gate closed, still in flight.
    bridge.emit('in flight');
    final chosen = c.choose(DeviceLogLevel.off);
    await pumpEventQueue();
    await chosen;
    expect(c.lines, isEmpty, reason: 'nothing lands after Off cleared it');

    // The control: picking a level again subscribes again.
    await c.choose(DeviceLogLevel.errors);
    bridge.emit('after');
    await pumpEventQueue();
    expect(c.lines, hasLength(1));
    expect(c.lines.single, endsWith('after'));
  });

  test('a_stream_error_costs_the_line_not_the_subscription', () async {
    // An error the subscription does not handle escapes to the zone: catch
    // it here, so the test fails on an assertion that names it.
    final unhandled = <Object>[];
    late DeviceLogController c;
    await runZonedGuarded(() async {
      c = _controller(bridge)..arm(DeviceLogLevel.detailed);
      bridge.streams.last.addError(StateError('a native hiccup'));
      bridge.emit('after the error');
      await pumpEventQueue();
    }, (e, _) => unhandled.add(e));
    expect(unhandled, isEmpty, reason: 'a stream error escaped the log');
    expect(c.lines, hasLength(1));
    expect(c.lines.single, endsWith('after the error'));
  });

  test('the_stored_choice_is_applied_before_the_wallet_opens', () async {
    // A FRESH process: nothing armed, the FFI never initialised.
    SharedPreferences.setMockInitialValues({deviceLogLevelPrefsKey: 'off'});
    final c = _controller(bridge);
    debugResetWalletFfi(
      rustInit: () => rustInitThenArmDeviceLog(
        rustLibInit: () async => log.add('rust.init'),
        log: c,
      ),
    );
    expect(await initWalletFfi(), isTrue);
    await LoggingProvisioner(log).open();
    // Off was chosen: this debug run's Detailed default does not override it.
    expect(log, ['rust.init', 'log.set off', 'log.watch', 'wallet.open']);
    expect(c.choice, DeviceLogLevel.off);
  });

  test('a_level_change_keeps_the_subscription_and_drops_no_line', () async {
    final c = _controller(bridge)..arm(DeviceLogLevel.errors);
    // A line the SDK posted just before the switch, still in flight.
    bridge.emit('in flight');
    final chosen = c.choose(DeviceLogLevel.detailed);
    await pumpEventQueue();
    await chosen;
    expect(c.lines, hasLength(1), reason: 'the switch dropped a line');
    expect(c.lines.single, endsWith('in flight'));
    expect(
      bridge.streams,
      hasLength(1),
      reason: 'only setDeviceLog, no re-watch',
    );
    expect(log.last, 'log.set detailed');
    expect(c.effective, DeviceLogLevel.detailed);
  });

  test('a_level_change_the_sdk_refuses_leaves_everything_as_it_was', () async {
    final stored = <DeviceLogLevel>[];
    final c = DeviceLogController(
      bridge: bridge,
      store: (l) async => stored.add(l),
    )..arm(DeviceLogLevel.errors);
    bridge.emit('before');
    await pumpEventQueue();
    bridge.effectiveFor = (l) =>
        l == DeviceLogLevel.detailed ? throw StateError('refused') : l;

    await c.choose(DeviceLogLevel.detailed);
    expect(
      (c.choice, c.effective),
      (DeviceLogLevel.errors, DeviceLogLevel.errors),
      reason: 'the row keeps showing the level really in force',
    );
    expect(stored, isEmpty, reason: 'a change that did not apply is not kept');
    expect(c.lines, hasLength(1));
    expect(c.lines.single, endsWith('before'));

    // The subscription is untouched: the next line still arrives.
    bridge.emit('after');
    await pumpEventQueue();
    expect(c.lines, hasLength(2));
    expect(bridge.streams, hasLength(1));
  });

  test('a_refused_pick_after_off_leaves_everything_as_it_was', () async {
    final stored = <DeviceLogLevel>[];
    final c = DeviceLogController(
      bridge: bridge,
      store: (l) async => stored.add(l),
    )..arm(DeviceLogLevel.errors);
    await c.choose(DeviceLogLevel.off);
    expect(stored, [DeviceLogLevel.off]);
    log.clear();
    // No live subscription now, so a pick takes the re-arm branch.
    bridge.effectiveFor = (l) =>
        l == DeviceLogLevel.detailed ? throw StateError('refused') : l;

    await c.choose(DeviceLogLevel.detailed);
    expect(
      (c.choice, c.effective),
      (DeviceLogLevel.off, DeviceLogLevel.off),
      reason: 'the row keeps showing the level really in force',
    );
    expect(stored, [DeviceLogLevel.off], reason: 'the refused pick is kept');
    expect(log, ['log.set detailed'], reason: 'no subscription was opened');
    expect(bridge.streams, hasLength(1));
    expect(c.lines, isEmpty);
  });

  test('a_refused_off_keeps_showing_the_level_the_sdk_answered', () async {
    final stored = <DeviceLogLevel>[];
    final c = DeviceLogController(
      bridge: bridge,
      store: (l) async => stored.add(l),
    )..arm(DeviceLogLevel.detailed);
    bridge.emit('before');
    await pumpEventQueue();
    bridge.effectiveFor = (l) =>
        l == DeviceLogLevel.off ? throw StateError('refused') : l;

    await c.choose(DeviceLogLevel.off);
    expect(
      (c.choice, c.effective),
      (DeviceLogLevel.detailed, DeviceLogLevel.detailed),
      reason: 'Off was not answered, so the row must not claim it',
    );
    expect(stored, isEmpty, reason: 'an Off that did not apply is not kept');
    expect(c.lines, hasLength(1), reason: 'the ring is kept');

    // The SDK's gate may still be open: its lines still land.
    bridge.emit('after');
    await pumpEventQueue();
    expect(c.lines, hasLength(2));
    expect(bridge.streams, hasLength(1));
  });

  test('a_refused_arm_shows_off_the_level_the_sdk_starts_in', () {
    // Nothing has been answered yet. The SDK's own default is Off, and a wipe
    // sets it Off, so Off is what is in force when `arm` is refused.
    bridge.effectiveFor = (_) => throw StateError('refused');
    final c = _controller(bridge)..arm(DeviceLogLevel.detailed);
    expect(
      (c.choice, c.effective),
      (DeviceLogLevel.detailed, DeviceLogLevel.off),
      reason: 'the row shows Off and says the log is unavailable',
    );
  });
}
