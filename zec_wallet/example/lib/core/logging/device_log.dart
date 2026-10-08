import 'dart:async';
import 'dart:collection' show ListQueue;
import 'dart:convert' show utf8;

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:share_plus/share_plus.dart';
import 'package:share_plus_platform_interface/share_plus_platform_interface.dart'
    show SharePlatform;
import 'package:zec_wallet/zec_wallet.dart' as sdk;
import 'package:zec_wallet/zec_wallet.dart'
    show CustodyDisclosure, DeviceLogLevel, DeviceLogLine, SyncServerChoice;
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import 'device_log_prefs.dart';

// The example's half of the SDK's device log (stage S5 `row`, FR-35): the
// user's level applied right after init and again after every Delete, a live
// subscription into a bounded ring, and a log the user can send.
//
// What the ring holds is exactly what `watchDeviceLog` delivered — lines the
// SDK already gated by the level and cut to its §5.4 allowlist — plus a
// receipt time and this app's one boundary line. Nothing is added from Dart.
//
// THE SUBSCRIPTION IS NOT PAUSED IN THE BACKGROUND, deliberately and unlike
// the rule for UI streams (app-frame spec §3; `tor_plugin.dart`'s note):
// cancelling on `paused` would drop the background-sync lines a sent log
// exists to carry, and pausing would only move the backlog into Dart's
// controller. The SDK bounds the rate (60 lines a minute) and the ring bounds
// the memory.

/// The ring's size: at most 500 × 512 B = 256 KB, about 8 minutes at the SDK's
/// host-stream ceiling (stage S5 §2, `row`).
const deviceLogRingCapacity = 500;

/// Written into the ring after every Delete, returned or thrown: the lines
/// about the Delete stay (they are what diagnoses a failed one), and this
/// keeps them from reading as continuous with the next wallet's.
const deviceLogBoundaryLine = '— wallet deleted; log re-armed —';

/// On a platform that shares through a `mailto:` link, the payload's cap.
/// Percent-encoding at most triples it (24 KB), well under Linux's 128 KiB
/// single-argument limit and inside what common mail handlers accept.
const deviceLogMailLinkCapBytes = 8 * 1024;

/// What the example asks of the SDK's device log, so the ring, the re-arm and
/// the row are testable without the native library.
abstract interface class DeviceLogBridge {
  /// `setDeviceLog`: answers the EFFECTIVE level.
  DeviceLogLevel set(DeviceLogLevel level);

  /// `watchDeviceLog`: one subscriber per process; a new call replaces the
  /// last, whose stream closes.
  Stream<DeviceLogLine> watch();

  /// `sdkVersion`, for the shared log's header.
  String sdkVersion();
}

/// The production [DeviceLogBridge]: the SDK's free functions.
final class SdkDeviceLogBridge implements DeviceLogBridge {
  const SdkDeviceLogBridge();

  @override
  DeviceLogLevel set(DeviceLogLevel level) => sdk.setDeviceLog(level: level);

  @override
  Stream<DeviceLogLine> watch() => sdk.watchDeviceLog();

  @override
  String sdkVersion() => sdk.sdkVersion();
}

/// The device log's state for this process: the user's level, the level the
/// SDK answered, and the ring. Rendering reads only what is cached here — no
/// bridge call happens on a rebuild.
final class DeviceLogController extends ChangeNotifier {
  DeviceLogController({
    required DeviceLogBridge bridge,
    required Future<void> Function(DeviceLogLevel) store,
    DateTime Function() clock = DateTime.now,
  }) : _bridge = bridge,
       _store = store,
       _clock = clock;

  final DeviceLogBridge _bridge;
  final Future<void> Function(DeviceLogLevel) _store;
  final DateTime Function() _clock;
  final ListQueue<String> _ring = ListQueue();
  StreamSubscription<DeviceLogLine>? _sub;
  DeviceLogLevel _choice = DeviceLogLevel.off;
  DeviceLogLevel _effective = DeviceLogLevel.off;

  /// The level this app asked for (the user's pick, or the build's default).
  DeviceLogLevel get choice => _choice;

  /// The level the SDK answered: what the row renders.
  DeviceLogLevel get effective => _effective;

  /// The ring, oldest first.
  List<String> get lines => List.unmodifiable(_ring);

  /// Sets [level] and (re)subscribes. Never throws: a diagnostic must never
  /// cost the wallet (the init awaits this).
  void arm(DeviceLogLevel level) {
    _choice = level;
    _effective = _set(level);
    _cancelSub();
    _subscribe();
    notifyListeners();
  }

  /// The user's pick in Settings: stored, then applied.
  ///
  /// - **Off** clears the ring (the user asked for no log, so none is kept or
  ///   sent) AND cancels the live subscription. A line the SDK had already
  ///   posted before its gate closed would otherwise land in the ring just
  ///   after the clear.
  /// - **Any other level** needs only `setDeviceLog`. The subscription is
  ///   kept, so a line in flight during an Errors↔Detailed switch is not
  ///   dropped by a cancel. It subscribes only when there is no live
  ///   subscription: after Off, or after a wipe closed the stream.
  ///
  /// **If the SDK throws on the change, nothing changes**, on every branch,
  /// Off included: the pick, the level the row shows, the ring and the
  /// subscription all stay as they were, and the pick is not stored. The row
  /// shows only a level the SDK answered; a refused Off is not read as Off,
  /// since the SDK's gate may still be open.
  Future<void> choose(DeviceLogLevel level) async {
    if (level == DeviceLogLevel.off) {
      final answered = _trySet(level);
      if (answered == null) return;
      _choice = level;
      _effective = answered;
      _cancelSub();
      _ring.clear();
      notifyListeners();
    } else if (_sub == null) {
      final answered = _trySet(level);
      if (answered == null) return;
      _choice = level;
      _effective = answered;
      _subscribe();
      notifyListeners();
    } else {
      final answered = _trySet(level);
      if (answered == null) return;
      _choice = level;
      _effective = answered;
      notifyListeners();
    }
    await _store(level);
  }

  /// Drops the live subscription, if any: the one place THIS side ends one
  /// ([arm] before it subscribes again, and [choose] on Off). The other way
  /// `_sub` goes null is the stream finishing on its own (a wipe closes it),
  /// and that `onDone` clears only the subscription it belongs to.
  void _cancelSub() {
    final old = _sub;
    _sub = null;
    unawaited(old?.cancel());
  }

  void _subscribe() {
    try {
      late final StreamSubscription<DeviceLogLine> sub;
      sub = _bridge.watch().listen(
        _receive,
        // The TYPE only, as everywhere in this file. An error on the stream
        // costs that event, never the subscription or the app.
        onError: (Object e) =>
            debugPrint('device log: stream error (${e.runtimeType})'),
        // Only if it is still THE subscription: belt-and-braces, since [arm]
        // cancels the old one first and a cancelled one gets no `onDone`.
        onDone: () {
          if (identical(_sub, sub)) _sub = null;
        },
      );
      _sub = sub;
    } catch (e) {
      debugPrint('device log: could not subscribe (${e.runtimeType})');
    }
  }

  /// After a Delete, returned or thrown: every wipe closed the stream and set
  /// the log Off (the SDK's duress seam), so arm it again and mark the seam in
  /// the ring. The ring is NOT cleared.
  void rearmAfterWipe(DeviceLogLevel level) {
    arm(level);
    _append('${_stamp()} $deviceLogBoundaryLine');
  }

  /// The text Share sends: a header, then the ring — capped on a platform
  /// that shares through a mail link ([deviceLogShareText]).
  String shareText({required bool mailLink}) {
    String version;
    try {
      version = _bridge.sdkVersion();
    } catch (_) {
      version = 'unknown';
    }
    return deviceLogShareText(
      header: 'zec_wallet $version, device log at ${_effective.name}',
      lines: lines,
      mailLink: mailLink,
    );
  }

  /// `setDeviceLog`, with a throw read as Off: for [arm] only (the init and
  /// the re-arm after a wipe). There nothing has been answered yet, and Off
  /// is what the SDK is in: its own default is Off, and every wipe sets it
  /// Off before anything else. So Off is the one level that does not lie,
  /// and the row then shows Off with its "can't keep a log" line. A user's
  /// CHANGE never comes here ([choose] leaves everything as it was).
  DeviceLogLevel _set(DeviceLogLevel level) =>
      _trySet(level) ?? DeviceLogLevel.off;

  /// `setDeviceLog`, or `null` when it threw: for a level CHANGE, which then
  /// leaves everything as it was ([choose]).
  DeviceLogLevel? _trySet(DeviceLogLevel level) {
    try {
      return _bridge.set(level);
    } catch (e) {
      debugPrint('device log: could not be set (${e.runtimeType})');
      return null;
    }
  }

  void _receive(DeviceLogLine line) => _append(
    '${_stamp()} ${line.severity.name.toUpperCase()} ${line.tag} '
    '#${line.seq} ${line.text}',
  );

  void _append(String entry) {
    _ring.addLast(entry);
    while (_ring.length > deviceLogRingCapacity) {
      _ring.removeFirst();
    }
    notifyListeners();
  }

  /// Receipt time, local, to the millisecond: the SDK's lines carry none.
  String _stamp() {
    final t = _clock();
    String two(int n) => n.toString().padLeft(2, '0');
    return '${two(t.hour)}:${two(t.minute)}:${two(t.second)}.'
        '${t.millisecond.toString().padLeft(3, '0')}';
  }
}

/// The shared text: [header], then [lines], newest last.
///
/// Where Share becomes a `mailto:` link ([mailLink]: Linux, and Windows before
/// 10 RS5) the whole text rides in the URI's `body=`, which a mail handler may
/// silently cut and which the launched handler typically receives as a
/// process argument. There it keeps the header and the NEWEST lines within
/// [deviceLogMailLinkCapBytes], and says how many it left out.
///
/// The cap holds unconditionally: the header is cut to
/// [deviceLogMailLinkHeaderCapBytes] first, so it and the marker always leave
/// room, and the joined text is clamped to the cap last, whatever the budget
/// arithmetic said.
String deviceLogShareText({
  required String header,
  required List<String> lines,
  required bool mailLink,
}) {
  if (!mailLink) return [header, ...lines].join('\n');
  final head = _clampUtf8(header, deviceLogMailLinkHeaderCapBytes);
  int bytes(String s) => utf8.encode(s).length + 1; // + its newline
  // The marker's size with the largest count it could carry.
  var budget =
      deviceLogMailLinkCapBytes - bytes(head) - bytes(_leftOut(lines.length));
  var from = lines.length;
  while (from > 0 && bytes(lines[from - 1]) <= budget) {
    budget -= bytes(lines[from - 1]);
    from--;
  }
  return _clampUtf8(
    [head, if (from > 0) _leftOut(from), ...lines.sublist(from)].join('\n'),
    deviceLogMailLinkCapBytes,
  );
}

/// The most of a mail-link header that is kept (the header is this app's own
/// text; a longer one is cut, not allowed to crowd out the log).
const deviceLogMailLinkHeaderCapBytes = 1024;

/// [s] cut to at most [maxBytes] of UTF-8, at a character boundary.
String _clampUtf8(String s, int maxBytes) {
  final encoded = utf8.encode(s);
  if (encoded.length <= maxBytes) return s;
  var end = maxBytes;
  // Back off a continuation byte (10xxxxxx) so no character is split.
  while (end > 0 && (encoded[end] & 0xC0) == 0x80) {
    end--;
  }
  return utf8.decode(encoded.sublist(0, end));
}

String _leftOut(int n) =>
    '$n earlier lines left out: a mail link carries only the latest';

/// Whether Share becomes a `mailto:` link here: share_plus registers its Dart
/// mail-link implementation on Linux, and on Windows only before 10 RS5.
bool shareGoesThroughAMailLink() {
  final platform = SharePlatform.instance;
  return platform is SharePlusLinuxPlugin || platform is SharePlusWindowsPlugin;
}

/// Summons the OS share sheet with [text] and nothing else.
Future<void> shareDeviceLogText(ShareParams params) async {
  await SharePlus.instance.share(params);
}

/// The process's device log. One per process, like the SDK's subscription.
final DeviceLogController deviceLog = DeviceLogController(
  bridge: const SdkDeviceLogBridge(),
  store: storeDeviceLogChoice,
);

/// The Settings row's handle on [deviceLog]; overridden in tests.
final deviceLogProvider = Provider<DeviceLogController>((ref) => deviceLog);

/// How Share hands its params to the OS; overridden in tests.
final deviceLogShareProvider = Provider<Future<void> Function(ShareParams)>(
  (ref) => shareDeviceLogText,
);

/// Whether Share goes through a mail link here; overridden in tests.
final deviceLogMailLinkProvider = Provider<bool>(
  (ref) => shareGoesThroughAMailLink(),
);

// --- Init and the re-arm ---------------------------------------------

/// The FFI init, then the device log at the resolved level, before anything
/// touches the wallet — where the SDK asks a host to set it. Anything the log
/// throws is caught here: [initWalletFfi]'s caller reads a throw as a FAILED
/// init, and a diagnostic must never cost the wallet its start.
Future<void> rustInitThenArmDeviceLog({
  required Future<void> Function() rustLibInit,
  required DeviceLogController log,
  Future<String?> Function() readStored = readStoredDeviceLogChoice,
}) async {
  await rustLibInit();
  try {
    final asked = resolveDeviceLogLevelForThisBuild(await readStored());
    log.arm(asked);
    debugPrint(
      'wallet device log: asked ${asked.name}, effective ${log.effective.name}',
    );
  } catch (e) {
    debugPrint('wallet device log: could not be set (${e.runtimeType})');
  }
}

/// Re-reads the user's choice and re-arms [log] after a Delete. Never throws.
Future<void> rearmDeviceLogAfterWipe(
  DeviceLogController log, {
  Future<String?> Function() readStored = readStoredDeviceLogChoice,
}) async {
  try {
    log.rearmAfterWipe(resolveDeviceLogLevelForThisBuild(await readStored()));
  } catch (e) {
    debugPrint('wallet device log: could not re-arm (${e.runtimeType})');
  }
}

/// The wallet's provisioner with the device log re-armed after every Delete,
/// returned or thrown — in the pattern of `TorAwareProvisioner`.
///
/// HOUSEKEEPING FOR AN APP WITH NO DURESS TRIGGER. The SDK closes the stream
/// and sets the log Off at the start of every wipe, and this init path is
/// memoized, so without this the example would stay silently Off after a
/// Delete until the process restarts.
///
/// **DO NOT COPY THIS INTO A HOST WITH A DURESS PATH.** It re-arms after EVERY
/// `deleteWallet` and `forceDeleteWallet`, and those are the same wipe a
/// duress trigger calls. Wrapped around a duress wipe, it would switch the
/// log back on, and write a line saying a wipe just happened, at the moment
/// the phone must look untouched. A host with a duress path re-arms only on
/// its ordinary Delete, at that call site, and never on the duress path
/// (Relim goes to its wiped screen and does not re-arm). This example has
/// no duress trigger, which is the only reason the decorator is safe here.
///
/// Every other call passes straight through.
final class DeviceLogRearmingProvisioner implements WalletProvisioner {
  DeviceLogRearmingProvisioner({required this.inner, required this.rearm});

  final WalletProvisioner inner;

  /// Must not throw ([rearmDeviceLogAfterWipe] does not).
  final Future<void> Function() rearm;

  @override
  Future<void> deleteWallet() => _wipe(inner.deleteWallet);

  @override
  Future<void> forceDeleteWallet() => _wipe(inner.forceDeleteWallet);

  Future<void> _wipe(Future<void> Function() wipe) async {
    try {
      await wipe();
    } finally {
      await rearm();
    }
  }

  @override
  Future<bool> walletExists() => inner.walletExists();

  @override
  Future<WalletSession> createGenerated() => inner.createGenerated();

  @override
  Future<WalletSession> open() => inner.open();

  @override
  Future<WalletSession> restore(
    List<String> mnemonicWords, {
    DateTime? approximateCreationTime,
  }) => inner.restore(
    mnemonicWords,
    approximateCreationTime: approximateCreationTime,
  );

  @override
  Future<WalletSession> createWatchOnly(
    String ufvk, {
    required int birthdayHeight,
  }) => inner.createWatchOnly(ufvk, birthdayHeight: birthdayHeight);

  @override
  Future<WalletSession> rescanFrom(RescanTarget target) =>
      inner.rescanFrom(target);

  @override
  Future<WalletSession> switchSyncServer(SyncServerChoice choice) =>
      inner.switchSyncServer(choice);

  @override
  int estimateBirthdayHeight(DateTime time) =>
      inner.estimateBirthdayHeight(time);

  @override
  Future<List<String>> revealMnemonic() => inner.revealMnemonic();

  @override
  Future<String> exportUfvk() => inner.exportUfvk();

  @override
  Future<CustodyDisclosure> custodyDisclosure() => inner.custodyDisclosure();
}
