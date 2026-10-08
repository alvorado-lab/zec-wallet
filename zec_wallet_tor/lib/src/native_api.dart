import 'dart:ffi';
import 'dart:io' show Platform;
import 'dart:typed_data';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart' show visibleForTesting;

import 'bindings_generated.dart';
import 'errors.dart';
import 'status.dart';

/// The plugin's C ABI as `ZecWalletTor` uses it: one method per export, each
/// returning the raw `ZWT_RC_*` code.
///
/// The seam that lets the Dart logic be tested without the native library:
/// [FfiTorNativeApi] is the production implementation over the generated
/// bindings; a test supplies a fake.
///
/// Byte spans are handed over as Dart [Uint8List]s; the implementation copies
/// each into native memory valid for the call only, and ZEROES the native
/// copy of the bridge lines before freeing it. The caller zeroes its own
/// [Uint8List] afterwards.
abstract interface class TorNativeApi {
  /// `zec_wallet_tor_abi_version()`. Throws
  /// `TorPluginError(libraryUnavailable)` when the library or the export is
  /// not in the process.
  int abiVersion();

  /// `zec_wallet_tor_init`. [onStatusChanged] runs on the Dart isolate's
  /// event loop each time the plugin reports a phase or readiness change; it
  /// carries no payload — the caller reads [status] (see
  /// [FfiTorNativeApi.init] for why).
  int init(
    Uint8List torDir,
    Uint8List? bridges,
    void Function() onStatusChanged,
  );

  /// `zec_wallet_tor_status`: the return code and, when it is `ZWT_RC_OK`,
  /// the decoded status.
  (int, TorPluginStatus?) status();

  /// `zec_wallet_tor_set_bridges`; `null` = no bridges.
  int setBridges(Uint8List? bridges);

  /// `zec_wallet_tor_retry_bootstrap`.
  int retryBootstrap();

  /// `zec_wallet_tor_clear_state`.
  int clearState(Uint8List torDir);

  /// `zec_wallet_tor_dispose`.
  int dispose();

  /// `zec_wallet_tor_on_paused`.
  int onPaused();

  /// `zec_wallet_tor_on_resumed`.
  int onResumed();
}

/// The production [TorNativeApi] over the ffigen bindings.
///
/// The library is found the FFI-plugin template's way: `.process()` on
/// Apple — the archive is linked into the plugin's own framework under
/// `use_frameworks!`, or into the Runner under static linkage, and the
/// process lookup finds the uniquely prefixed exports in either image — the
/// shared object on Android and Linux, the DLL on Windows.
final class FfiTorNativeApi implements TorNativeApi {
  FfiTorNativeApi._(this._lookup, this._alloc)
    : _bindings = ZecWalletTorBindings.fromLookup(_lookup);

  /// Opens the plugin's native library. Throws
  /// `TorPluginError(libraryUnavailable)` when it is not in this app.
  factory FfiTorNativeApi.open() {
    final DynamicLibrary lib;
    try {
      lib = _openLibrary();
    } on ArgumentError {
      throw const TorPluginError(TorPluginErrorKind.libraryUnavailable);
    }
    return FfiTorNativeApi._(lib.lookup, calloc);
  }

  /// Over an arbitrary symbol [lookup] and [allocator] — for tests that stand
  /// Dart callables in for the exports and watch what is allocated and freed.
  @visibleForTesting
  factory FfiTorNativeApi.withLookup(
    Pointer<T> Function<T extends NativeType>(String symbolName) lookup, {
    Allocator allocator = calloc,
  }) => FfiTorNativeApi._(lookup, allocator);

  static DynamicLibrary _openLibrary() {
    if (Platform.isIOS || Platform.isMacOS) return DynamicLibrary.process();
    if (Platform.isAndroid || Platform.isLinux) {
      return DynamicLibrary.open('libzec_wallet_tor.so');
    }
    if (Platform.isWindows) return DynamicLibrary.open('zec_wallet_tor.dll');
    throw UnsupportedError('zec_wallet_tor: unsupported platform');
  }

  final Pointer<T> Function<T extends NativeType>(String symbolName) _lookup;
  final Allocator _alloc;
  final ZecWalletTorBindings _bindings;

  /// The ONE status listener, created at the first `init` and never closed.
  ///
  /// Two properties of `NativeCallable.listener` decide its shape:
  ///
  /// 1. It is asynchronous: the native caller returns before the Dart
  ///    callback runs, and a pointer argument must stay valid until the
  ///    callback HAS run (dart:ffi's own contract). The header makes the
  ///    `status` pointer valid for the call only, so this side never reads
  ///    it: the callback is an edge, and the listener reads the current
  ///    status by value through `zec_wallet_tor_status`.
  /// 2. Invoking a closed listener from native code is undefined behaviour,
  ///    and the header does not promise that no callback is in flight or
  ///    still to come once `dispose` returns. So the listener lives for the
  ///    process — as the plugin's own trampoline does — and `dispose` only
  ///    detaches the Dart handler from it.
  NativeCallable<zwt_status_fnFunction>? _listener;
  void Function()? _onStatusChanged;

  @override
  int abiVersion() {
    // Resolved BY NAME, on its own, before any other export is touched
    // (plan D-11): a library too old to have it is a missing export, not a
    // version.
    final int Function() fn;
    try {
      fn = _lookup<NativeFunction<Uint32 Function()>>(
        'zec_wallet_tor_abi_version',
      ).asFunction<int Function()>();
    } on ArgumentError {
      throw const TorPluginError(TorPluginErrorKind.libraryUnavailable);
    }
    return fn();
  }

  @override
  int init(
    Uint8List torDir,
    Uint8List? bridges,
    void Function() onStatusChanged,
  ) {
    _onStatusChanged = onStatusChanged;
    final listener = _listener ??=
        NativeCallable<zwt_status_fnFunction>.listener(_onNativeStatus);
    final config = _alloc<zwt_config>();
    final dir = _Span.copy(_alloc, torDir);
    final lines = _Span.copy(_alloc, bridges);
    try {
      config.ref
        ..tor_dir = dir.ptr
        ..tor_dir_len = dir.len
        ..bridges = lines.ptr
        ..bridges_len = lines.len;
      return _bindings.zec_wallet_tor_init(
        config,
        listener.nativeFunction,
        nullptr,
      );
    } finally {
      _alloc.free(config);
      dir.free();
      lines.zeroAndFree();
    }
  }

  void _onNativeStatus(Pointer<Void> ctx, Pointer<zwt_status> status) {
    // Never dereference `status` here — see [_listener].
    _onStatusChanged?.call();
  }

  @override
  (int, TorPluginStatus?) status() {
    final out = _alloc<zwt_status>();
    try {
      final rc = _bindings.zec_wallet_tor_status(out);
      if (rc != ZWT_RC_OK) return (rc, null);
      final s = out.ref;
      return (
        rc,
        TorPluginStatus.fromCodes(
          phase: s.phase,
          readiness: s.readiness,
          blockage: s.blockage,
          failureClass: s.failure_class,
        ),
      );
    } finally {
      _alloc.free(out);
    }
  }

  @override
  int setBridges(Uint8List? bridges) {
    final lines = _Span.copy(_alloc, bridges);
    try {
      return _bindings.zec_wallet_tor_set_bridges(lines.ptr, lines.len);
    } finally {
      lines.zeroAndFree();
    }
  }

  @override
  int retryBootstrap() => _bindings.zec_wallet_tor_retry_bootstrap();

  @override
  int clearState(Uint8List torDir) {
    final dir = _Span.copy(_alloc, torDir);
    try {
      return _bindings.zec_wallet_tor_clear_state(dir.ptr, dir.len);
    } finally {
      dir.free();
    }
  }

  @override
  int dispose() {
    final rc = _bindings.zec_wallet_tor_dispose();
    _onStatusChanged = null;
    return rc;
  }

  @override
  int onPaused() => _bindings.zec_wallet_tor_on_paused();

  @override
  int onResumed() => _bindings.zec_wallet_tor_on_resumed();
}

/// A (pointer, length) span in native memory, valid until freed. `null` or
/// empty input is the header's `(NULL, 0)`.
final class _Span {
  _Span._(this._alloc, this.ptr, this.len);

  factory _Span.copy(Allocator alloc, Uint8List? bytes) {
    if (bytes == null || bytes.isEmpty) return _Span._(alloc, nullptr, 0);
    final ptr = alloc<Uint8>(bytes.length);
    ptr.asTypedList(bytes.length).setAll(0, bytes);
    return _Span._(alloc, ptr, bytes.length);
  }

  final Allocator _alloc;
  final Pointer<Uint8> ptr;
  final int len;

  void free() {
    if (ptr != nullptr) _alloc.free(ptr);
  }

  /// Zero the native copy, then free it: the bridge text must not linger in
  /// freed memory.
  void zeroAndFree() {
    if (ptr == nullptr) return;
    ptr.asTypedList(len).fillRange(0, len, 0);
    _alloc.free(ptr);
  }
}
