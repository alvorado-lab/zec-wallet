import 'dart:typed_data';

import 'package:zec_wallet_tor/src/bindings_generated.dart';
import 'package:zec_wallet_tor/zec_wallet_tor.dart';

/// A scripted stand-in for the plugin's native library: the Dart logic runs
/// against it with no dylib in the process.
class FakeTorNativeApi implements TorNativeApi {
  FakeTorNativeApi({this.walletLoaded = true, this.abi = ZWT_ABI_VERSION});

  /// Whether the wallet's library is "in the process" — `init` answers
  /// `ZWT_RC_WALLET_NOT_LOADED` while it is not.
  bool walletLoaded;

  /// What `zec_wallet_tor_abi_version()` answers.
  int abi;

  /// When set, `abiVersion` throws this (a library without the export).
  Object? abiThrows;

  /// The next `init` / `set_bridges` / `clear_state` / `dispose` codes.
  int initRc = ZWT_RC_OK;
  int setBridgesRc = ZWT_RC_OK;
  int clearStateRc = ZWT_RC_OK;

  /// How many `clear_state` calls answer `ZWT_RC_STOPPING` (an earlier client
  /// still stopping) before the native side lets the removal through.
  int clearStateStoppingAnswers = 0;

  /// The same for `init`.
  int initStoppingAnswers = 0;
  int disposeRc = ZWT_RC_OK;

  bool _registered = false;

  TorPluginStatus current = const TorPluginStatus(
    phase: TorPluginPhase.bootstrapping,
    readiness: 0,
  );

  /// Every call, in order, by export name.
  final List<String> calls = [];

  /// The Uint8List the Dart side handed over for the bridge lines — the SAME
  /// object, so a test can see whether it was zeroed after the call.
  Uint8List? lastBridgesArg;

  /// A copy of the bytes as they were DURING the call.
  List<int>? lastBridgesSeen;
  List<int>? lastTorDirSeen;
  List<int>? lastClearStateDirSeen;

  void Function()? _onStatusChanged;

  /// The plugin reports a status change from its own thread; the listener
  /// delivers it on the event loop, never synchronously — the fake keeps
  /// that shape.
  Future<void> push(TorPluginStatus status) {
    current = status;
    return Future<void>(() => _onStatusChanged?.call());
  }

  @override
  int abiVersion() {
    calls.add('abi_version');
    final t = abiThrows;
    if (t != null) throw t;
    return abi;
  }

  @override
  int init(
    Uint8List torDir,
    Uint8List? bridges,
    void Function() onStatusChanged,
  ) {
    calls.add('init');
    lastTorDirSeen = List.of(torDir);
    lastBridgesArg = bridges;
    lastBridgesSeen = bridges == null ? null : List.of(bridges);
    if (!walletLoaded) return ZWT_RC_WALLET_NOT_LOADED;
    if (initStoppingAnswers > 0) {
      initStoppingAnswers--;
      return ZWT_RC_STOPPING;
    }
    if (initRc != ZWT_RC_OK) return initRc;
    _onStatusChanged = onStatusChanged;
    _registered = true;
    return ZWT_RC_OK;
  }

  @override
  (int, TorPluginStatus?) status() {
    calls.add('status');
    if (!_registered && current.failureClass == TorFailureClass.none) {
      return (ZWT_RC_NOT_INITIALIZED, null);
    }
    return (ZWT_RC_OK, current);
  }

  @override
  int setBridges(Uint8List? bridges) {
    calls.add('set_bridges');
    lastBridgesArg = bridges;
    lastBridgesSeen = bridges == null ? null : List.of(bridges);
    return setBridgesRc;
  }

  @override
  int retryBootstrap() {
    calls.add('retry_bootstrap');
    return _registered ? ZWT_RC_OK : ZWT_RC_NOT_INITIALIZED;
  }

  @override
  int clearState(Uint8List torDir) {
    calls.add('clear_state');
    lastClearStateDirSeen = List.of(torDir);
    if (_registered) return ZWT_RC_NOT_INITIALIZED;
    if (clearStateStoppingAnswers > 0) {
      clearStateStoppingAnswers--;
      return ZWT_RC_STOPPING;
    }
    return clearStateRc;
  }

  @override
  int dispose() {
    calls.add('dispose');
    if (!_registered) return ZWT_RC_NOT_INITIALIZED;
    _registered = false;
    _onStatusChanged = null;
    current = const TorPluginStatus(
      phase: TorPluginPhase.notRegistered,
      readiness: 0,
    );
    return disposeRc;
  }

  @override
  int onPaused() {
    calls.add('on_paused');
    return ZWT_RC_OK;
  }

  @override
  int onResumed() {
    calls.add('on_resumed');
    return ZWT_RC_OK;
  }
}
