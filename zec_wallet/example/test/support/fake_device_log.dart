import 'dart:async';

import 'package:zec_wallet/zec_wallet.dart'
    show DeviceLogLevel, DeviceLogLine, DeviceLogSeverity;
import 'package:zec_wallet_example/core/logging/device_log.dart';

/// A [DeviceLogBridge] with no native library: it records every call into
/// [calls] and hands each `watch` a stream the test feeds through [emit].
class FakeDeviceLogBridge implements DeviceLogBridge {
  FakeDeviceLogBridge({List<String>? calls}) : calls = calls ?? [];

  final List<String> calls;

  /// What `set` answers for a level; by default, the level asked for.
  DeviceLogLevel Function(DeviceLogLevel asked) effectiveFor = (l) => l;

  final List<StreamController<DeviceLogLine>> streams = [];
  var _seq = 0;

  @override
  DeviceLogLevel set(DeviceLogLevel level) {
    calls.add('log.set ${level.name}');
    return effectiveFor(level);
  }

  /// Like the SDK's `HostSlot::watch`, a new watch REPLACES the last and
  /// drops it: the previous stream is closed.
  @override
  Stream<DeviceLogLine> watch() {
    calls.add('log.watch');
    if (streams.isNotEmpty) unawaited(streams.last.close());
    final c = StreamController<DeviceLogLine>();
    streams.add(c);
    return c.stream;
  }

  @override
  String sdkVersion() => '9.9.9-test';

  /// Sends one line down the latest subscription.
  void emit(
    String text, {
    DeviceLogSeverity severity = DeviceLogSeverity.info,
  }) {
    streams.last.add(
      DeviceLogLine(
        seq: BigInt.from(_seq++),
        severity: severity,
        tag: 'zec_wallet_core',
        text: text,
      ),
    );
  }
}
