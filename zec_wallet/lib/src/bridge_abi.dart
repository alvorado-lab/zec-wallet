/// FR-33 — the bridge's own wire-contract version, as this Dart package was
/// generated against it.
///
/// The Dart package and the native library can be paired from different
/// builds. flutter_rust_bridge's own pairing check did not notice a change to
/// the data shapes crossing the bridge, so a mismatched pair used to start
/// cleanly and then decode values wrongly — once as "Tor active" for a path
/// that was not. `RustLib.init` (see `rust_lib.dart`) now compares this
/// number with the one the native library reports, before any bridge call.
library;

/// The wire-contract version this package's generated bindings speak. It MUST
/// equal `BRIDGE_ABI_VERSION` in `rust/src/bridge_abi.rs`; the Rust test
/// `the_dart_bridge_abi_constant_equals_the_rust_one` parses this line, so keep
/// it a single `const int` declaration with an integer literal.
const int kBridgeAbiVersion = 11;

/// The Dart package and the native wallet library were built from different
/// versions of the bridge, so values crossing between them would be decoded
/// wrongly. Thrown by `RustLib.init` before any bridge call is made.
///
/// Deliberately an [Exception], never an [ArgumentError]: a host that reads
/// `ArgumentError` from init as "this build has no wallet library" must read
/// this as "the wallet library is here but will not start" — which is true.
/// That includes a native library too OLD to report a version at all
/// ([nativeVersion] is null).
class BridgeAbiMismatch implements Exception {
  const BridgeAbiMismatch({required this.dartVersion, this.nativeVersion});

  /// The version this Dart package was generated against.
  final int dartVersion;

  /// The version the native library reports, or null when it predates the
  /// version export entirely.
  final int? nativeVersion;

  @override
  String toString() {
    final native = nativeVersion == null
        ? 'reports no bridge version (it predates the check)'
        : 'reports bridge version $nativeVersion';
    return 'BridgeAbiMismatch: the zec_wallet Dart package speaks bridge '
        'version $dartVersion but the native wallet library $native. Rebuild '
        'the app so both halves come from the same zec_wallet release.';
  }
}

/// Compares [kBridgeAbiVersion] with the native library's version, read by
/// [readNativeVersion]. Throws [BridgeAbiMismatch] when they differ.
///
/// When the version export cannot be found, the lookup's [ArgumentError] means
/// one of TWO things, and [walletLibraryPresent] tells them apart:
/// * the wallet library IS loaded but predates the check — an older wallet
///   export resolves — so it is [BridgeAbiMismatch] with no native version;
/// * NO wallet library is loaded at all — nothing resolves — so the original
///   [ArgumentError] is RETHROWN, exactly what init threw before this check
///   existed. Under the PROCESS loader (which opens nothing) a missing library
///   is first noticed at a symbol lookup, which is why this branch exists; with
///   the scoped `zec_wallet.framework/zec_wallet` open the README documents on
///   Apple, `open` itself throws first. Either way a host that reads
///   `ArgumentError` as "this build has no wallet" must keep reading it so
///   (the FR-33 security review's HIGH).
void verifyBridgeAbi(
  int Function() readNativeVersion, {
  required bool Function() walletLibraryPresent,
}) {
  final int native;
  try {
    native = readNativeVersion();
  } on ArgumentError {
    if (!walletLibraryPresent()) rethrow;
    throw const BridgeAbiMismatch(dartVersion: kBridgeAbiVersion);
  }
  if (native != kBridgeAbiVersion) {
    throw BridgeAbiMismatch(
      dartVersion: kBridgeAbiVersion,
      nativeVersion: native,
    );
  }
}
