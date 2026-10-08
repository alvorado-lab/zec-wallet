/// Optional Tor for the `zec_wallet` SDK.
///
/// The plugin runs an arti Tor client in its own native library and, at
/// [ZecWalletTor.init], registers it with the wallet as the wallet's
/// transport through the wallet's host-dialer contract. The host then opens
/// the wallet with `TorPolicy.required_(runtime:
/// TorRuntimeConfig.hostDialer())` (or `TorPolicy.preferred`), exactly as for any
/// registered transport. See README.md for the rules a host must follow.
library;

import 'dart:async';
import 'dart:convert' show utf8;
import 'dart:typed_data';

import 'package:flutter/widgets.dart';

import 'src/bindings_generated.dart';
import 'src/errors.dart';
import 'src/native_api.dart';
import 'src/status.dart';

export 'src/errors.dart';
export 'src/native_api.dart' show TorNativeApi, FfiTorNativeApi;
export 'src/status.dart';

/// The plugin's Dart surface. One plugin per process, so every member is
/// static — the native side is process-global too.
///
/// **Order, at boot:** the wallet's `RustLib.init()` FIRST (the plugin finds
/// the wallet's library in the process and refuses with
/// [TorPluginErrorKind.walletNotLoaded] otherwise), then [init], THEN open
/// the wallet with the host-dialer Tor policy.
///
/// **Order, at wipe:** [dispose] → the wallet's wipe → [clearState] when the
/// user's intent includes the Tor identity (the guard state is a per-device
/// identifier, not wallet data; a wallet wipe alone leaves it).
///
/// Every verb throws [TorPluginError]; catch it by type and switch on its
/// [TorPluginError.kind]. Before its first native call the class reads the
/// library's contract version BY NAME and refuses a library from another
/// release ([TorPluginErrorKind.pluginAbiMismatch]).
///
/// The calls are synchronous FFI underneath (the `Future`s leave room to move
/// them off the UI isolate): [dispose] can hold the calling isolate for up to
/// the plugin's retire bound (five seconds) while in-flight connections
/// drain.
abstract final class ZecWalletTor {
  static TorNativeApi? _api;
  static bool _abiChecked = false;
  static bool _initialized = false;

  /// Bumped by every [dispose]: an [init] waiting out a stopping client
  /// compares it after each wait, so a dispose that lands meanwhile wins and
  /// the pending init never registers Tor after it (2026-10-07 external
  /// review, follow-up finding 2).
  static int _disposals = 0;
  static StreamController<TorPluginStatus>? _controller;
  static TorPluginStatus? _last;
  static AppLifecycleListener? _lifecycle;
  static bool _paused = false;

  static TorNativeApi get _native => _api ??= FfiTorNativeApi.open();

  /// Reads the native library's contract version, once per process, before
  /// anything else touches it (plan D-11 — the wallet's FR-33 lesson: two
  /// halves from different releases must fail loudly, never decode a status
  /// one field off).
  static TorNativeApi _checked() {
    final api = _native;
    if (_abiChecked) return api;
    final version = api.abiVersion();
    if (version != ZWT_ABI_VERSION) {
      throw TorPluginError(
        TorPluginErrorKind.pluginAbiMismatch,
        nativeAbiVersion: version,
      );
    }
    _abiChecked = true;
    return api;
  }

  /// Registers Tor with the wallet and starts the bootstrap; returns the
  /// status right after.
  ///
  /// [torDir] is the plugin's OWN directory — absolute, and a SIBLING of the
  /// wallet's `dbDir`, never the same directory nor inside it (the wallet's
  /// wipe sweeps every entry of `dbDir`). Exclude it from backup as you do
  /// `dbDir`. [bridges] are bridge lines as the user pasted them; `null` for
  /// none. The host persists the paste; the plugin keeps it only in memory.
  ///
  /// Idempotent: a second call registers nothing and returns the current
  /// status. Throws [TorPluginError] — [TorPluginErrorKind.walletNotLoaded]
  /// before the wallet's `RustLib.init()`, [TorPluginErrorKind.slotOccupied]
  /// when another transport registered first, and the rest of the kinds as
  /// documented on them.
  ///
  /// Right after [dispose], a Tor client from before it can still be shutting
  /// down; a new one would share its state directory. The plugin refuses
  /// until it has stopped, and this waits for that, up to [clearStateWaitMax],
  /// before throwing [TorPluginErrorKind.stopping]. A [dispose] that lands
  /// during that wait wins: this throws [TorPluginErrorKind.disposed] and
  /// registers nothing.
  static Future<TorPluginStatus> init({
    required String torDir,
    String? bridges,
  }) async {
    final api = _checked();
    final dir = _utf8(torDir);
    final lines = bridges == null ? null : _utf8(bridges);
    final deadline = DateTime.now().add(clearStateWaitMax);
    final disposalsAtStart = _disposals;
    int rc;
    try {
      rc = api.init(dir, lines, _onStatusChanged);
      while (rc == ZWT_RC_STOPPING && DateTime.now().isBefore(deadline)) {
        await Future<void>.delayed(const Duration(milliseconds: 100));
        if (_disposals != disposalsAtStart) {
          // The host disposed while this init waited: it must not register
          // Tor after that dispose.
          rc = ZWT_RC_DISPOSED;
          break;
        }
        rc = api.init(dir, lines, _onStatusChanged);
      }
    } finally {
      // The Dart String behind the paste cannot be zeroed (the documented
      // exposure); this copy of its bytes can.
      lines?.fillRange(0, lines.length, 0);
    }
    if (rc != ZWT_RC_OK) _throw(api, rc);
    _initialized = true;
    _installLifecycle();
    final status = _read(api);
    _publish(status);
    return status;
  }

  /// The plugin's current status. Throws [TorPluginErrorKind.notInitialized]
  /// before [init].
  static TorPluginStatus status() => _read(_checked());

  /// Every change of the plugin's status, as it happens: the plugin reports
  /// each phase or readiness change from its own threads through a
  /// `NativeCallable.listener`, and the status arrives on this isolate's
  /// event loop. A report that leaves the status unchanged is not emitted.
  /// [dispose] closes the stream; a later [init] opens a new one.
  ///
  /// A host that subscribes pauses its SUBSCRIPTION on
  /// `AppLifecycleState.paused` and resumes it on a real `resumed`, like any
  /// stream. The plugin's own lifecycle handling does not depend on that:
  /// it listens to the app lifecycle itself.
  static Stream<TorPluginStatus> get statusStream =>
      (_controller ??= StreamController<TorPluginStatus>.broadcast()).stream;

  /// Replaces the bridge lines (`null` = none) and rebuilds the Tor client.
  /// A refused paste throws [TorPluginErrorKind.bridgesRefused] with its
  /// [TorPluginError.failureClass] and leaves the running client untouched.
  /// While an earlier client is stuck it throws
  /// [TorPluginErrorKind.restartRequired] and changes nothing: the current
  /// bridges stay in use until the app restarts.
  static Future<void> setBridges(String? bridges) async {
    final api = _checked();
    final lines = bridges == null ? null : _utf8(bridges);
    final int rc;
    try {
      rc = api.setBridges(lines);
    } finally {
      lines?.fillRange(0, lines.length, 0);
    }
    if (rc != ZWT_RC_OK) _throw(api, rc);
  }

  /// A user-driven retry: skips the backoff wait and starts a bootstrap now.
  /// Does nothing unless the phase is [TorPluginPhase.failed].
  static Future<void> retryBootstrap() async {
    final api = _checked();
    final rc = api.retryBootstrap();
    if (rc != ZWT_RC_OK) _throw(api, rc);
  }

  /// Removes the plugin's Tor state under [torDir] — the Tor-identity reset
  /// in a host's wipe flow, AFTER [dispose] and the wallet's wipe. Takes the
  /// directory because it must work after a restart with no [init]. Removes
  /// only the two subtrees the plugin creates (`state/`, `cache/`) and then
  /// [torDir] if it is left empty. Idempotent; throws
  /// [TorPluginErrorKind.notInitialized] while Tor is running.
  ///
  /// [dispose] returns once the plugin is unregistered, and a Tor client from
  /// before it can still be stopping (and about to write its state). The
  /// plugin refuses to remove anything until that client is gone, so this
  /// waits for it, up to [clearStateWaitMax], and throws
  /// [TorPluginErrorKind.stopping] if it is still stopping then. Each client
  /// runs on its own runtime, shut down when the client is retired, so
  /// "gone" includes every task Tor started for it. A normal return means the
  /// state is gone and every Tor client of this process has finished. If a
  /// client did not finish shutting down in time, this throws
  /// [TorPluginErrorKind.restartRequired] at once: restart the app and call
  /// this before [init]. Do not call [init] until the wipe is done.
  ///
  /// Both of those mean the reset is recorded in [torDir] (if it exists;
  /// otherwise there is nothing to reset), and the next
  /// [init] finishes it before Tor starts. If it cannot be recorded (a full
  /// disk, say), this throws [TorPluginErrorKind.invalidDataDir] instead:
  /// nothing was removed and nothing is pending.
  static Future<void> clearState(String torDir) async {
    final api = _checked();
    final dir = _utf8(torDir);
    final deadline = DateTime.now().add(clearStateWaitMax);
    var rc = api.clearState(dir);
    while (rc == ZWT_RC_STOPPING && DateTime.now().isBefore(deadline)) {
      await Future<void>.delayed(const Duration(milliseconds: 100));
      rc = api.clearState(dir);
    }
    if (rc != ZWT_RC_OK) _throw(api, rc);
  }

  /// How long [clearState] and [init] wait for an earlier Tor client to
  /// finish stopping before they throw [TorPluginErrorKind.stopping]. Longer
  /// than the native side's own 10-second bound on a client's shutdown, so a
  /// shutdown that finishes at its bound is waited for, not given up on.
  static const Duration clearStateWaitMax = Duration(seconds: 15);

  /// Retires the registration (every connection in flight fails typed on the
  /// wallet's side), clears the wallet's transport slot and drops the Tor
  /// client; closes [statusStream] and stops following the app lifecycle. A
  /// later [init] registers afresh.
  ///
  /// Safe to call when nothing is running: a plugin that was never
  /// initialised in this process, or is already disposed, is left as it is.
  static Future<void> dispose() async {
    final api = _checked();
    _disposals++;
    final rc = api.dispose();
    _initialized = false;
    _removeLifecycle();
    _last = null;
    final controller = _controller;
    _controller = null;
    await controller?.close();
    if (rc != ZWT_RC_OK &&
        rc != ZWT_RC_NOT_INITIALIZED &&
        rc != ZWT_RC_DISPOSED) {
      _throw(api, rc);
    }
  }

  /// Forgets everything this class holds and, when given, routes every call
  /// to [nativeApi] instead of the native library.
  @visibleForTesting
  static void debugReset({TorNativeApi? nativeApi}) {
    _removeLifecycle();
    _controller?.close();
    _controller = null;
    _last = null;
    _initialized = false;
    _abiChecked = false;
    _api = nativeApi;
  }

  static Uint8List _utf8(String s) => Uint8List.fromList(utf8.encode(s));

  static TorPluginStatus _read(TorNativeApi api) {
    final (rc, status) = api.status();
    if (rc != ZWT_RC_OK || status == null) {
      _throw(api, rc == ZWT_RC_OK ? ZWT_RC_NULL_ARG : rc);
    }
    return status;
  }

  static Never _throw(TorNativeApi api, int rc) {
    TorFailureClass? failureClass;
    if (rc == ZWT_RC_BRIDGES_REFUSED || rc == ZWT_RC_ENGINE_SETUP) {
      // The class travels in the status, never in a string.
      final (statusRc, status) = api.status();
      if (statusRc == ZWT_RC_OK) failureClass = status?.failureClass;
    }
    throw TorPluginError.fromRc(rc, failureClass: failureClass);
  }

  /// The listener's handler: the callback is an edge; the status is read by
  /// value here (see [FfiTorNativeApi]).
  static void _onStatusChanged() {
    final api = _api;
    if (api == null || !_initialized) return;
    final (rc, status) = api.status();
    if (rc != ZWT_RC_OK || status == null) return;
    _publish(status);
  }

  static void _publish(TorPluginStatus status) {
    if (status == _last) return;
    _last = status;
    final controller = _controller;
    if (controller != null && !controller.isClosed) controller.add(status);
  }

  /// The plugin follows the app lifecycle itself, so a host that never
  /// subscribes to [statusStream] still gets a transport that goes quiet
  /// before the OS acts. `paused` forwards to the native pause; `resumed`
  /// forwards only after a real pause — `inactive → resumed` (a
  /// notification shade, a desktop focus change) is not one. Desktop never
  /// delivers `paused`, so a desktop client stays warm.
  static void _installLifecycle() {
    if (_lifecycle != null) return;
    WidgetsFlutterBinding.ensureInitialized();
    _lifecycle = AppLifecycleListener(onStateChange: _onLifecycle);
  }

  static void _removeLifecycle() {
    _lifecycle?.dispose();
    _lifecycle = null;
    _paused = false;
  }

  static void _onLifecycle(AppLifecycleState state) {
    final api = _api;
    if (api == null || !_initialized) return;
    switch (state) {
      case AppLifecycleState.paused when !_paused:
        _paused = true;
        _forward('paused', api.onPaused());
      case AppLifecycleState.resumed when _paused:
        _paused = false;
        _forward('resumed', api.onResumed());
      default:
        break;
    }
  }

  static void _forward(String what, int rc) {
    if (rc == ZWT_RC_OK ||
        rc == ZWT_RC_NOT_INITIALIZED ||
        rc == ZWT_RC_DISPOSED) {
      return;
    }
    // The kind only — a lifecycle event must never throw into the framework.
    debugPrint(
      'zec_wallet_tor: $what not applied '
      '(${TorPluginErrorKind.fromRc(rc).name})',
    );
  }
}
