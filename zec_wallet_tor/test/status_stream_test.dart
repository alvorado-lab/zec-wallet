// P23 (docs/specs/tor-plugin.md §8): the stream emits on a phase change and on
// a readiness change, not on an identical push; a status reported from a
// thread that is not this isolate's arrives on this isolate's event loop
// (`NativeCallable.listener`); `dispose` closes the stream. Plus the plugin's
// own lifecycle forwarding (§3.4) and the zeroed native bridge copy.
import 'dart:async';
import 'dart:ffi';
import 'dart:isolate';

import 'package:ffi/ffi.dart';
import 'package:flutter/widgets.dart' show AppLifecycleState;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_tor/src/bindings_generated.dart';
import 'package:zec_wallet_tor/zec_wallet_tor.dart';

import 'fake_native_api.dart';

const _torDir = '/data/user/0/app/files/tor';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('over the fake native api', () {
    late FakeTorNativeApi native;

    setUp(() {
      native = FakeTorNativeApi();
      ZecWalletTor.debugReset(nativeApi: native);
    });

    tearDown(() => ZecWalletTor.debugReset());

    test('emits on a phase change and on a readiness change, not on an '
        'identical push', () async {
      final seen = <TorPluginStatus>[];
      final sub = ZecWalletTor.statusStream.listen(seen.add);
      await ZecWalletTor.init(torDir: _torDir);
      const b10 = TorPluginStatus(
        phase: TorPluginPhase.bootstrapping,
        readiness: 10,
      );
      const ready = TorPluginStatus(
        phase: TorPluginPhase.ready,
        readiness: 100,
      );
      await native.push(b10); // readiness change
      await native.push(b10); // identical: not emitted
      await native.push(ready); // phase change
      await native.push(ready); // identical: not emitted
      await pumpEventQueue();
      expect(seen, [
        const TorPluginStatus(
          phase: TorPluginPhase.bootstrapping,
          readiness: 0,
        ),
        b10,
        ready,
      ]);
      await sub.cancel();
    });

    test('dispose closes the stream', () async {
      await ZecWalletTor.init(torDir: _torDir);
      final done = Completer<void>();
      ZecWalletTor.statusStream.listen((_) {}, onDone: done.complete);
      await ZecWalletTor.dispose();
      await done.future.timeout(const Duration(seconds: 1));
      // A report after dispose reaches nobody and throws nothing.
      await native.push(
        const TorPluginStatus(phase: TorPluginPhase.ready, readiness: 100),
      );
    });

    testWidgets('paused forwards once; resumed forwards only after a real '
        'pause', (tester) async {
      await ZecWalletTor.init(torDir: _torDir);
      native.calls.clear();
      final binding = tester.binding;

      // A notification-shade pull: inactive → resumed is not a pause.
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      expect(native.calls, isEmpty);

      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      expect(native.calls, ['on_paused']);

      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      expect(native.calls, ['on_paused', 'on_resumed']);

      // After dispose the plugin stops following the lifecycle.
      await ZecWalletTor.dispose();
      native.calls.clear();
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      expect(native.calls, isEmpty);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    });
  });

  group('over the real FFI class, with Dart callables for the exports', () {
    late _FakeExports exports;

    setUp(() {
      exports = _FakeExports();
      ZecWalletTor.debugReset(
        nativeApi: FfiTorNativeApi.withLookup(
          exports.lookup,
          allocator: exports.allocator,
        ),
      );
    });

    tearDown(() {
      ZecWalletTor.debugReset();
      exports.close();
    });

    test(
      'a status reported from another thread arrives on this isolate\'s '
      'event loop, read by value — the pointer is never dereferenced',
      () async {
        final seen = <TorPluginStatus>[];
        final sub = ZecWalletTor.statusStream.listen(seen.add);
        await ZecWalletTor.init(torDir: _torDir);
        await pumpEventQueue();
        seen.clear();

        final address = exports.onStatus!.address;

        // Called from THIS thread, the handler still does not run inside the
        // native call: it is queued for the event loop.
        exports.set(phase: ZWT_PHASE_BOOTSTRAPPING, readiness: 40);
        _invokeStatusCallback(address);
        expect(
          seen,
          isEmpty,
          reason: 'nothing runs before the event loop turns',
        );
        await pumpEventQueue();
        expect(seen, [
          const TorPluginStatus(
            phase: TorPluginPhase.bootstrapping,
            readiness: 40,
          ),
        ]);

        // Called from a helper isolate's thread, with a NULL status pointer: a
        // listener that dereferenced it would crash the test process.
        exports.set(phase: ZWT_PHASE_READY, readiness: 100);
        await Isolate.run(() => _invokeStatusCallback(address));
        await pumpEventQueue();
        expect(
          seen.last,
          const TorPluginStatus(phase: TorPluginPhase.ready, readiness: 100),
        );
        expect(seen, hasLength(2));
        await sub.cancel();
      },
    );

    test(
      'the native copy of the bridge lines is zeroed before it is freed',
      () async {
        const paste = 'obfs4 192.0.2.1:443 AAAA cert=x iat-mode=0';
        await ZecWalletTor.init(torDir: _torDir, bridges: paste);
        expect(exports.bridgesSeenDuringInit, paste);
        final freed = exports.allocator.freedOfLength(paste.length);
        expect(freed, isNotEmpty);
        expect(freed.last.every((b) => b == 0), isTrue);

        await ZecWalletTor.setBridges(paste);
        final again = exports.allocator.freedOfLength(paste.length);
        expect(again.length, freed.length + 1);
        expect(again.last.every((b) => b == 0), isTrue);
      },
    );

    test('the abi version is resolved by name first', () async {
      await ZecWalletTor.init(torDir: _torDir);
      expect(exports.lookups.first, 'zec_wallet_tor_abi_version');
    });
  });
}

void _invokeStatusCallback(int address) {
  Pointer<NativeFunction<zwt_status_fnFunction>>.fromAddress(
    address,
  ).asFunction<Dartzwt_status_fnFunction>()(nullptr, nullptr);
}

/// Dart callables standing in for the plugin's exports, so the production
/// [FfiTorNativeApi] — the struct layout, the spans, the listener — runs with
/// no native library.
class _FakeExports {
  _FakeExports() {
    _add<Uint32 Function()>(
      'zec_wallet_tor_abi_version',
      NativeCallable<Uint32 Function()>.isolateLocal(
        () => ZWT_ABI_VERSION,
        exceptionalReturn: 0,
      ),
    );
    _add(
      'zec_wallet_tor_init',
      NativeCallable<
        Int32 Function(Pointer<zwt_config>, zwt_status_fn, Pointer<Void>)
      >.isolateLocal(_init, exceptionalReturn: ZWT_RC_PANICKED),
    );
    _add(
      'zec_wallet_tor_status',
      NativeCallable<Int32 Function(Pointer<zwt_status>)>.isolateLocal(
        _status,
        exceptionalReturn: ZWT_RC_PANICKED,
      ),
    );
    _add(
      'zec_wallet_tor_set_bridges',
      NativeCallable<Int32 Function(Pointer<Uint8>, Size)>.isolateLocal(
        _span,
        exceptionalReturn: ZWT_RC_PANICKED,
      ),
    );
    _add(
      'zec_wallet_tor_dispose',
      NativeCallable<Int32 Function()>.isolateLocal(
        () => ZWT_RC_OK,
        exceptionalReturn: ZWT_RC_PANICKED,
      ),
    );
  }

  final Map<String, NativeCallable<Function>> _callables = {};
  final List<String> lookups = [];
  final allocator = _RecordingAllocator();

  Pointer<NativeFunction<zwt_status_fnFunction>>? onStatus;
  String? bridgesSeenDuringInit;
  int _phase = ZWT_PHASE_BOOTSTRAPPING;
  int _readiness = 0;

  void _add<T extends Function>(String name, NativeCallable<T> c) =>
      _callables[name] = c;

  Pointer<T> lookup<T extends NativeType>(String name) {
    lookups.add(name);
    final c = _callables[name];
    if (c == null) throw ArgumentError('no export $name');
    return c.nativeFunction.cast<T>();
  }

  void set({required int phase, required int readiness}) {
    _phase = phase;
    _readiness = readiness;
  }

  int _init(
    Pointer<zwt_config> config,
    zwt_status_fn onStatusFn,
    Pointer<Void> ctx,
  ) {
    onStatus = onStatusFn;
    final c = config.ref;
    bridgesSeenDuringInit = c.bridges == nullptr
        ? null
        : String.fromCharCodes(c.bridges.asTypedList(c.bridges_len));
    return ZWT_RC_OK;
  }

  int _status(Pointer<zwt_status> out) {
    out.ref
      ..phase = _phase
      ..readiness = _readiness
      ..blockage = ZWT_BLOCKAGE_NONE
      ..failure_class = ZWT_CLASS_NONE;
    return ZWT_RC_OK;
  }

  int _span(Pointer<Uint8> ptr, int len) => ZWT_RC_OK;

  void close() {
    for (final c in _callables.values) {
      c.close();
    }
  }
}

/// [calloc], recording the bytes each allocation held at the moment it was
/// freed.
class _RecordingAllocator implements Allocator {
  final Map<int, int> _sizes = {};
  final List<List<int>> _freed = [];

  List<List<int>> freedOfLength(int length) =>
      _freed.where((b) => b.length == length).toList();

  @override
  Pointer<T> allocate<T extends NativeType>(int byteCount, {int? alignment}) {
    final p = calloc.allocate<T>(byteCount, alignment: alignment);
    _sizes[p.address] = byteCount;
    return p;
  }

  @override
  void free(Pointer<NativeType> pointer) {
    final size = _sizes.remove(pointer.address);
    if (size != null) {
      _freed.add(List.of(pointer.cast<Uint8>().asTypedList(size)));
    }
    calloc.free(pointer);
  }
}
