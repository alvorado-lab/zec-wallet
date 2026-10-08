// Stage S5 `row` (docs/plan/stage-5-the-device-log-a-user-can-send.md §2):
// the stored choice, and the one function that turns it into a level.
import 'dart:io' show Directory, File;

import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet/zec_wallet.dart' show DeviceLogLevel;
import 'package:zec_wallet_example/core/logging/device_log_prefs.dart';

void main() {
  test('resolve_prefers_stored_then_define_then_the_build_default', () {
    const off = DeviceLogLevel.off;
    const errors = DeviceLogLevel.errors;
    const detailed = DeviceLogLevel.detailed;
    // (stored, define, isDebug) -> level
    final table = <(String?, String, bool, DeviceLogLevel)>[
      // 1. A stored choice wins over the define and the build.
      ('off', 'detailed', true, off),
      ('errors', 'detailed', true, errors),
      ('detailed', 'errors', false, detailed),
      ('off', '', true, off),
      // 2. Nothing stored: the define.
      (null, 'errors', true, errors),
      (null, 'detailed', false, detailed),
      // A misspelt define asks for nothing, never for MORE.
      (null, 'Detailed', true, off),
      // A stored value that is not one of the three names is never chosen.
      ('verbose', 'errors', true, errors),
      // 3. Neither: the build default (maintainer, Q2).
      (null, '', true, detailed),
      (null, '', false, errors),
      ('verbose', '', false, errors),
    ];
    for (final (stored, define, isDebug, want) in table) {
      expect(
        resolveDeviceLogLevel(stored: stored, define: define, isDebug: isDebug),
        want,
        reason: 'stored=$stored define="$define" isDebug=$isDebug',
      );
    }
  });

  test('an_absent_key_is_never_chosen_and_an_explicit_off_is_off', () async {
    SharedPreferences.setMockInitialValues({});
    final absent = await SharedPreferences.getInstance();
    expect(storedDeviceLogChoice(absent), isNull);
    // Never chosen, so a debug build gets its default, not Off.
    expect(
      resolveDeviceLogLevel(
        stored: storedDeviceLogChoice(absent),
        define: '',
        isDebug: true,
      ),
      DeviceLogLevel.detailed,
    );

    SharedPreferences.setMockInitialValues({deviceLogLevelPrefsKey: 'off'});
    final off = await SharedPreferences.getInstance();
    expect(storedDeviceLogChoice(off), 'off');

    // A value of the wrong type is never chosen either, and does not throw.
    SharedPreferences.setMockInitialValues({deviceLogLevelPrefsKey: true});
    expect(
      storedDeviceLogChoice(await SharedPreferences.getInstance()),
      isNull,
    );
  });

  test('production_resolves_with_kDebugMode', () {
    // `kDebugMode` is always true under `flutter test`, and so is
    // `!kReleaseMode`: no call can tell them apart here. So the call sites
    // are read. A profile build must take the release side (Q2, by
    // derivation); `!kReleaseMode` would move it to Detailed.
    final calls = <String>[];
    for (final f in Directory('lib').listSync(recursive: true)) {
      if (f is! File || !f.path.endsWith('.dart')) continue;
      final src = f.readAsStringSync();
      for (final m in RegExp(
        r'resolveDeviceLogLevel\(([^)]*)\)',
      ).allMatches(src)) {
        final args = m.group(1)!;
        // The definition itself.
        if (args.contains('required bool isDebug')) continue;
        calls.add('${f.path}: $args');
      }
    }
    expect(calls, hasLength(1), reason: 'one production call site: $calls');
    expect(
      calls.single,
      contains('isDebug: kDebugMode'),
      reason: 'the build default must follow kDebugMode: ${calls.single}',
    );
  });
}
