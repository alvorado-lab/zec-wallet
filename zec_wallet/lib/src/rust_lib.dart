/// The package's entry point: the SAME `RustLib.init(...)` hosts have always
/// called, with the bridge's wire-contract version checked first (FR-33).
///
/// flutter_rust_bridge generates the real entry class (renamed to
/// [ZecWalletRustLib] in `flutter_rust_bridge.yaml`); this class keeps the
/// public name and shape, so no host changes a line. What it adds: it loads the
/// native library itself, reads the library's bridge version from the plain C
/// export `zec_wallet_bridge_abi_version` — BY NAME, never through the bridge,
/// whose calls are dispatched by numeric ids that a mismatched pair may not
/// share — compares it with [kBridgeAbiVersion], and only then runs the
/// generated init with that same library. A mismatched pair throws
/// [BridgeAbiMismatch] and exchanges no bridge message at all.
library;

import 'dart:ffi' as ffi;

import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show loadExternalLibrary;
// ignore: invalid_use_of_internal_member -- ExternalLibrary and BaseHandler
// are the types the generated init takes; this is the entrypoint that
// re-exports them for hosts (see zec_wallet.dart)
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show BaseHandler, ExternalLibrary;

import 'bridge_abi.dart';
import 'rust/frb_generated.dart' show ZecWalletRustLib, ZecWalletRustLibApi;

/// The name of the C export the native library answers its bridge version on.
const String kBridgeAbiVersionSymbol = 'zec_wallet_bridge_abi_version';

/// An export EVERY wallet library has carried since FR-15 (July 2026) — long
/// before the version export. When the version lookup fails, this lookup tells
/// an OLDER wallet library (it resolves: a mismatch) from NO wallet library (it
/// does not: the loader's own "absent"). Prefixed, so it can never be answered
/// by another flutter_rust_bridge library in the same process — an unprefixed
/// `frb_*` symbol could.
///
/// It bites only under the PROCESS loader. With the loader the README now
/// documents on Apple — `ExternalLibrary.open('zec_wallet.framework/…')` — an
/// absent library already throws `ArgumentError` from `open`, before this runs.
/// And under `process` in an app holding a second FRB library, a passing
/// version check does NOT prove dispatch lands here: FRB's seven unprefixed
/// symbols may answer from the other image, and the guard that catches THAT is
/// FRB's own content-hash `StateError` — a third outcome a host should expect
/// beside `ArgumentError` and [BridgeAbiMismatch] (measured, an iPhone
/// carrying both frameworks).
const String kWalletPresenceProbeSymbol = 'zec_wallet_register_seed_port';

/// Initialise the wallet's native library. Call once, before any wallet API.
final class RustLib {
  RustLib._();

  /// The one instance, kept so the README's documented double-init guard
  /// (`if (!RustLib.instance.initialized) …`) compiles and keeps its type.
  static final RustLib instance = RustLib._();

  /// Whether [init] has completed.
  bool get initialized => ZecWalletRustLib.instance.initialized;

  /// As flutter_rust_bridge's generated `init`, with the FR-33 check first.
  /// Throws [BridgeAbiMismatch] when the native library speaks a different
  /// bridge version than this package, or is too old to report one. A library
  /// that is not in the process at all still fails as it always did, with
  /// `dart:ffi`'s `ArgumentError` — from the loader wherever the library is
  /// opened BY NAME (Android, Linux, Windows, and Apple with the scoped
  /// `zec_wallet.framework/zec_wallet` open the README documents), or from the
  /// version lookup under the process loader, which opens nothing — which
  /// hosts read as "no wallet in this build".
  static Future<void> init({
    ZecWalletRustLibApi? api,
    BaseHandler? handler,
    ExternalLibrary? externalLibrary,
    bool forceSameCodegenVersion = true,
  }) async {
    final library =
        externalLibrary ??
        await loadExternalLibrary(
          ZecWalletRustLib.kDefaultExternalLibraryLoaderConfig,
        );
    verifyBridgeAbi(
      () => _nativeBridgeAbiVersion(library),
      walletLibraryPresent: () => _walletLibraryPresent(library),
    );
    await ZecWalletRustLib.init(
      api: api,
      handler: handler,
      externalLibrary: library,
      forceSameCodegenVersion: forceSameCodegenVersion,
    );
  }

  /// As the generated `initMock`: no native library is loaded, so there is no
  /// pair to check.
  static void initMock({required ZecWalletRustLibApi api}) =>
      ZecWalletRustLib.initMock(api: api);

  /// As the generated `dispose`.
  static void dispose() => ZecWalletRustLib.dispose();
}

bool _walletLibraryPresent(ExternalLibrary library) =>
    library.ffiDynamicLibrary.providesSymbol(kWalletPresenceProbeSymbol);

int _nativeBridgeAbiVersion(ExternalLibrary library) => library
    .ffiDynamicLibrary
    .lookupFunction<ffi.Uint32 Function(), int Function()>(
      kBridgeAbiVersionSymbol,
    )();
