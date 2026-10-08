import 'bindings_generated.dart';
import 'status.dart';

/// Every way a plugin call can fail — a CLOSED set. Catch [TorPluginError]
/// and switch on [TorPluginError.kind]; never match on its message.
///
/// The first fourteen are the header's `ZWT_RC_*` codes, each bound to its
/// generated constant. The last three have no native code: they are the
/// Dart side's own refusals.
enum TorPluginErrorKind {
  /// The wallet's native library is not in this process: call the wallet's
  /// `RustLib.init()` before `ZecWalletTor.init`.
  walletNotLoaded(ZWT_RC_WALLET_NOT_LOADED),

  /// The wallet refused the registration: its host-dialer contract is a
  /// different version from the one this plugin was built against.
  abiMismatch(ZWT_RC_ABI_MISMATCH),

  /// Another transport already holds the wallet's dialer slot (first
  /// registration wins); Tor was not started.
  slotOccupied(ZWT_RC_SLOT_OCCUPIED),

  /// The wallet's dialer registry is poisoned; restart the app.
  registryPoisoned(ZWT_RC_REGISTRY_POISONED),

  /// The wallet refused the plugin's descriptor — a build defect.
  descriptorRefused(ZWT_RC_DESCRIPTOR_REFUSED),

  /// `torDir` is empty, relative, or cannot be created or opened. From
  /// `clearState`, also: the reset could not be done or recorded now (a
  /// full disk, say); nothing was removed and nothing is pending.
  invalidDataDir(ZWT_RC_INVALID_DATA_DIR),

  /// The bridge lines were refused before anything changed;
  /// [TorPluginError.failureClass] says which rule they broke.
  bridgesRefused(ZWT_RC_BRIDGES_REFUSED),

  /// The Tor client could not be built after registration; the plugin
  /// retired and cleared the wallet's slot. [TorPluginError.failureClass]
  /// carries the class.
  engineSetup(ZWT_RC_ENGINE_SETUP),

  /// No successful `init` yet — or `clearState` while Tor is running
  /// (`dispose` first).
  notInitialized(ZWT_RC_NOT_INITIALIZED),

  /// `dispose` ran; `init` again to register.
  disposed(ZWT_RC_DISPOSED),

  /// A required pointer was null — a defect in this Dart layer.
  nullArg(ZWT_RC_NULL_ARG),

  /// The plugin panicked inside the call; nothing it could undo changed.
  panicked(ZWT_RC_PANICKED),

  /// `clearState` or `init` while a Tor client from before `dispose` is still
  /// stopping. Nothing changed: call again shortly. `dispose` returns once the
  /// plugin is unregistered, which is not yet stopped. A `clearState` refused
  /// this way is recorded, and the next `init` finishes it: a host cannot
  /// withdraw a reset it asked for.
  stopping(ZWT_RC_STOPPING),

  /// `clearState`, `init` or `setBridges` after a Tor client did not finish
  /// shutting down in time: it may still write its state for as long as this
  /// process runs, so no new client may start. Nothing changed. Restart the
  /// app, then call `clearState` (or `init`) before anything else. In a wipe
  /// flow, tell the user the reset finishes on the next start; the plugin
  /// records the pending clear and finishes it at the next `init` regardless.
  restartRequired(ZWT_RC_RESTART_REQUIRED),

  /// The native library's contract version differs from this Dart
  /// package's: the two halves come from different releases. Refused before
  /// any other call (plan D-11); [TorPluginError.nativeAbiVersion] carries
  /// what the library reported.
  pluginAbiMismatch(null),

  /// The plugin's native library, or one of its exports, could not be found
  /// in this process — the package is not built into this app.
  libraryUnavailable(null),

  /// A return code this Dart release does not know. Never a crash;
  /// [TorPluginError.code] carries it for the log.
  unknown(null);

  const TorPluginErrorKind(this.rc);

  /// The header's `ZWT_RC_*` for this kind; `null` for the Dart-side kinds.
  final int? rc;

  /// The kind for a native return code. `ZWT_RC_OK` is not an error and is
  /// refused here; any other code this release does not know is [unknown].
  static TorPluginErrorKind fromRc(int rc) {
    if (rc == ZWT_RC_OK) {
      throw ArgumentError.value(rc, 'rc', 'ZWT_RC_OK is not an error');
    }
    for (final k in values) {
      if (k.rc == rc) return k;
    }
    return unknown;
  }
}

/// A plugin call failed. Thrown by every `ZecWalletTor` verb; caught by TYPE.
///
/// [message] is composed here from [kind] and the codes — no string crosses
/// the C ABI in either direction. It is for a developer or a log; a host
/// renders its own copy from [kind].
final class TorPluginError implements Exception {
  const TorPluginError(
    this.kind, {
    this.code,
    this.failureClass,
    this.nativeAbiVersion,
  });

  /// The error for a non-OK native return code, with the failure class read
  /// from the status where the kind carries one.
  factory TorPluginError.fromRc(int rc, {TorFailureClass? failureClass}) =>
      TorPluginError(
        TorPluginErrorKind.fromRc(rc),
        code: rc,
        failureClass: failureClass,
      );

  final TorPluginErrorKind kind;

  /// The native return code, for the log. Never the discriminator — [kind]
  /// is.
  final int? code;

  /// For [TorPluginErrorKind.bridgesRefused] and
  /// [TorPluginErrorKind.engineSetup]: the class the plugin recorded.
  final TorFailureClass? failureClass;

  /// For [TorPluginErrorKind.pluginAbiMismatch]: the version the native
  /// library reported.
  final int? nativeAbiVersion;

  String get message {
    final base = switch (kind) {
      TorPluginErrorKind.walletNotLoaded =>
        'The wallet library is not loaded: call RustLib.init() before '
            'ZecWalletTor.init.',
      TorPluginErrorKind.abiMismatch =>
        'The wallet refused the Tor plugin: their host-dialer contract '
            'versions differ. Use zec_wallet and zec_wallet_tor from the same '
            'release.',
      TorPluginErrorKind.slotOccupied =>
        'Another transport is already registered with the wallet; Tor was '
            'not started.',
      TorPluginErrorKind.registryPoisoned =>
        "The wallet's transport registry is unusable; restart the app.",
      TorPluginErrorKind.descriptorRefused =>
        "The wallet refused the plugin's descriptor (a build defect).",
      TorPluginErrorKind.invalidDataDir =>
        'The Tor directory is empty, relative, or cannot be created or '
            'written.',
      TorPluginErrorKind.bridgesRefused => 'The bridge lines were refused.',
      TorPluginErrorKind.engineSetup =>
        'The Tor client could not be built; the plugin unregistered.',
      TorPluginErrorKind.notInitialized =>
        'The Tor plugin is not initialised (or is running, for clearState: '
            'call dispose first).',
      TorPluginErrorKind.disposed =>
        'The Tor plugin was disposed; call init again.',
      TorPluginErrorKind.nullArg =>
        'The Tor plugin was called with a missing argument (a defect).',
      TorPluginErrorKind.panicked =>
        'The Tor plugin failed internally; nothing was changed.',
      TorPluginErrorKind.stopping =>
        'The previous Tor client is still stopping; nothing was changed. '
            'Try again shortly.',
      TorPluginErrorKind.restartRequired =>
        'The previous Tor client did not shut down in time; nothing was '
            'changed. Restart the app to finish.',
      TorPluginErrorKind.pluginAbiMismatch =>
        'The Tor plugin native library (contract $nativeAbiVersion) does not '
            'match this Dart package (contract $ZWT_ABI_VERSION).',
      TorPluginErrorKind.libraryUnavailable =>
        'The Tor plugin native library is not in this app.',
      TorPluginErrorKind.unknown =>
        'The Tor plugin returned an unknown code ($code).',
    };
    final c = failureClass;
    return c == null || c == TorFailureClass.none
        ? base
        : '$base ${c.description}';
  }

  @override
  String toString() => 'TorPluginError(${kind.name}): $message';
}
