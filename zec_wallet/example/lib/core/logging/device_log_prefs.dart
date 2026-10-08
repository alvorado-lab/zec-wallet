import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet/zec_wallet.dart' show DeviceLogLevel;

/// The user's device-log choice (Settings → Appearance → Device log), and the
/// one function that turns it into the level this app asks the SDK for.
/// Stage S5 `row`; spec `docs/specs/app-frame.md` §2.1.
///
/// The SECOND non-cosmetic exception to `appearance_prefs.dart`'s rule, beside
/// `use_tor_plugin`, for the same reason: it is read before the wallet exists
/// (straight after the FFI init), so it cannot live behind the wallet. It holds
/// no wallet state and no secret — one of three level names.
const deviceLogLevelPrefsKey = 'device_log_level';

/// The device-log level a DEVELOPER asked this build for:
/// `--dart-define=ZEC_WALLET_DEVICE_LOG=off|errors|detailed`.
///
/// It sets the level only on a device where nobody has picked one: a stored
/// choice always wins ([resolveDeviceLogLevel]). On a device with a stored
/// choice, pick in Settings or clear the app's data. It is a Dart define, not
/// `ZEC_WALLET_DEVICE_TIMING`: that one opens the MEASUREMENT build and marks
/// the artifact `+devtiming`; this one marks nothing, because it changes no
/// artifact — only which level this app asks the SDK for.
const String deviceLogDefine = String.fromEnvironment('ZEC_WALLET_DEVICE_LOG');

/// The level a define value asks for. Anything that is not exactly `errors`
/// or `detailed` — a typo, another case, `off` — is OFF: a misspelt request
/// for a log must never be read as a request for MORE of one.
@visibleForTesting
DeviceLogLevel deviceLogLevelFor(String define) => switch (define) {
  'errors' => DeviceLogLevel.errors,
  'detailed' => DeviceLogLevel.detailed,
  _ => DeviceLogLevel.off,
};

/// The stored choice, or `null` when the user has never chosen.
///
/// NULLABLE ON PURPOSE, and not this app's `getX(key) ?? default` idiom: an
/// absent key ("never chosen") must stay distinct from an explicit `off`, or
/// a user who never opened Settings would read as one who turned the log off
/// and the build default would never apply. A value that is not one of the
/// three names (another type included) is also "never chosen".
String? storedDeviceLogChoice(SharedPreferences prefs) {
  if (!prefs.containsKey(deviceLogLevelPrefsKey)) return null;
  final value = prefs.get(deviceLogLevelPrefsKey);
  return value is String && _levelNamed(value) != null ? value : null;
}

/// [storedDeviceLogChoice] from the platform store, bounded like the other
/// pre-`runApp` reads. A store that cannot be read is "never chosen": this is
/// a diagnostic, and it must never cost the wallet its start.
Future<String?> readStoredDeviceLogChoice({
  Future<SharedPreferences> Function() open = SharedPreferences.getInstance,
}) async {
  try {
    return storedDeviceLogChoice(
      await open().timeout(const Duration(seconds: 5)),
    );
  } catch (e) {
    debugPrint('device log choice unreadable (${e.runtimeType})');
    return null;
  }
}

/// Persists the user's pick. Best-effort, like the cosmetic prefs: the
/// in-session level stands if the write fails.
Future<void> storeDeviceLogChoice(DeviceLogLevel level) async {
  try {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(deviceLogLevelPrefsKey, level.name);
  } catch (_) {
    // Best-effort persist (app-frame spec §6).
  }
}

/// THE precedence, in one place (stage S5 `row`):
/// 1. the user's stored choice;
/// 2. otherwise the `ZEC_WALLET_DEVICE_LOG` define ([deviceLogLevelFor]);
/// 3. otherwise the build default: Detailed in debug, Errors only otherwise
///    (maintainer, Q2: "Detailed in debug, Errors only in release").
///
/// [isDebug] is a parameter because `kDebugMode` is always true under
/// `flutter test`; production passes `kDebugMode` and nothing else
/// ([resolveDeviceLogLevelForThisBuild]). A PROFILE build therefore takes
/// Errors only — a derivation from `kDebugMode`, which the maintainer confirmed
/// .
DeviceLogLevel resolveDeviceLogLevel({
  String? stored,
  required String define,
  required bool isDebug,
}) {
  final chosen = _levelNamed(stored);
  if (chosen != null) return chosen;
  if (define.isNotEmpty) return deviceLogLevelFor(define);
  return isDebug ? DeviceLogLevel.detailed : DeviceLogLevel.errors;
}

/// [resolveDeviceLogLevel] as this build answers it: the only production call.
DeviceLogLevel resolveDeviceLogLevelForThisBuild(String? stored) =>
    resolveDeviceLogLevel(
      stored: stored,
      define: deviceLogDefine,
      isDebug: kDebugMode,
    );

DeviceLogLevel? _levelNamed(String? name) {
  for (final level in DeviceLogLevel.values) {
    if (level.name == name) return level;
  }
  return null;
}
