// The Dart side owns every code -> name table (the header: "outputs are
// closed codes — the Dart side owns every code -> name table"). These pin
// each table against the generated header constants, and pin that an unknown
// code decodes to a typed value, never a crash and never a string match.
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_tor/src/bindings_generated.dart';
import 'package:zec_wallet_tor/zec_wallet_tor.dart';

void main() {
  test('every ZWT_RC_* maps to its own kind, and the table is closed', () {
    const table = {
      ZWT_RC_WALLET_NOT_LOADED: TorPluginErrorKind.walletNotLoaded,
      ZWT_RC_ABI_MISMATCH: TorPluginErrorKind.abiMismatch,
      ZWT_RC_SLOT_OCCUPIED: TorPluginErrorKind.slotOccupied,
      ZWT_RC_REGISTRY_POISONED: TorPluginErrorKind.registryPoisoned,
      ZWT_RC_DESCRIPTOR_REFUSED: TorPluginErrorKind.descriptorRefused,
      ZWT_RC_INVALID_DATA_DIR: TorPluginErrorKind.invalidDataDir,
      ZWT_RC_BRIDGES_REFUSED: TorPluginErrorKind.bridgesRefused,
      ZWT_RC_ENGINE_SETUP: TorPluginErrorKind.engineSetup,
      ZWT_RC_NOT_INITIALIZED: TorPluginErrorKind.notInitialized,
      ZWT_RC_DISPOSED: TorPluginErrorKind.disposed,
      ZWT_RC_NULL_ARG: TorPluginErrorKind.nullArg,
      ZWT_RC_PANICKED: TorPluginErrorKind.panicked,
      ZWT_RC_STOPPING: TorPluginErrorKind.stopping,
      ZWT_RC_RESTART_REQUIRED: TorPluginErrorKind.restartRequired,
    };
    // The header's fourteen error codes are -1..=-14, contiguous.
    expect(table.keys.toSet(), {for (var i = -1; i >= -14; i--) i});
    for (final MapEntry(key: rc, value: kind) in table.entries) {
      expect(TorPluginErrorKind.fromRc(rc), kind, reason: 'rc $rc');
      expect(kind.rc, rc);
    }
    // Every kind that carries a native code is in the table; the three that
    // do not are the Dart side's own.
    expect(TorPluginErrorKind.values.where((k) => k.rc == null).toSet(), {
      TorPluginErrorKind.pluginAbiMismatch,
      TorPluginErrorKind.libraryUnavailable,
      TorPluginErrorKind.unknown,
    });
    expect(
      TorPluginErrorKind.values.where((k) => k.rc != null),
      hasLength(table.length),
    );
  });

  test('an unknown return code is the unknown kind, carried for the log', () {
    for (final rc in [-15, -99, 1, 42]) {
      final e = TorPluginError.fromRc(rc);
      expect(e.kind, TorPluginErrorKind.unknown);
      expect(e.code, rc);
      expect(e.message, contains('$rc'));
    }
  });

  test('OK is not an error', () {
    expect(() => TorPluginErrorKind.fromRc(ZWT_RC_OK), throwsArgumentError);
  });

  test('phases decode from the header codes; an unknown one is unknown', () {
    const table = {
      ZWT_PHASE_IDLE: TorPluginPhase.idle,
      ZWT_PHASE_BOOTSTRAPPING: TorPluginPhase.bootstrapping,
      ZWT_PHASE_READY: TorPluginPhase.ready,
      ZWT_PHASE_FAILED: TorPluginPhase.failed,
      ZWT_PHASE_SUSPENDED: TorPluginPhase.suspended,
      ZWT_PHASE_NOT_REGISTERED: TorPluginPhase.notRegistered,
    };
    for (final MapEntry(key: code, value: phase) in table.entries) {
      expect(TorPluginPhase.fromCode(code), phase);
    }
    expect(TorPluginPhase.values, hasLength(table.length + 1));
    expect(TorPluginPhase.fromCode(6), TorPluginPhase.unknown);
    expect(TorPluginPhase.fromCode(-1), TorPluginPhase.unknown);
  });

  test('blockages decode from the header codes; an unknown one is unknown', () {
    const table = {
      ZWT_BLOCKAGE_NONE: TorBlockage.none,
      ZWT_BLOCKAGE_DISABLED: TorBlockage.disabled,
      ZWT_BLOCKAGE_OFFLINE: TorBlockage.offline,
      ZWT_BLOCKAGE_FILTERING: TorBlockage.filtering,
      ZWT_BLOCKAGE_CANT_REACH_TOR: TorBlockage.cantReachTor,
      ZWT_BLOCKAGE_CLOCK_SKEWED: TorBlockage.clockSkewed,
      ZWT_BLOCKAGE_CANT_BOOTSTRAP: TorBlockage.cantBootstrap,
      ZWT_BLOCKAGE_UNKNOWN: TorBlockage.unknown,
    };
    for (final MapEntry(key: code, value: b) in table.entries) {
      expect(TorBlockage.fromCode(code), b);
    }
    expect(TorBlockage.values, hasLength(table.length));
    expect(TorBlockage.fromCode(8), TorBlockage.unknown);
  });

  test('failure classes are codes 1..=21 in the header order, plus none', () {
    const table = {
      ZWT_CLASS_NONE: TorFailureClass.none,
      ZWT_CLASS_BOOTSTRAP_DEADLINE: TorFailureClass.bootstrapDeadline,
      ZWT_CLASS_NOT_REGISTERED: TorFailureClass.notRegistered,
      ZWT_CLASS_BOOTSTRAP_FAILED: TorFailureClass.bootstrapFailed,
      ZWT_CLASS_SETUP: TorFailureClass.setup,
      ZWT_CLASS_NOT_BOOTSTRAPPED: TorFailureClass.notBootstrapped,
      ZWT_CLASS_BRIDGE_CONFIG_TOO_LONG: TorFailureClass.bridgeConfigTooLong,
      ZWT_CLASS_BRIDGE_TOO_MANY_LINES: TorFailureClass.bridgeTooManyLines,
      ZWT_CLASS_BRIDGE_LINE_TOO_LONG: TorFailureClass.bridgeLineTooLong,
      ZWT_CLASS_BRIDGE_PT_UNSUPPORTED: TorFailureClass.bridgePtUnsupported,
      ZWT_CLASS_BRIDGE_UNUSABLE: TorFailureClass.bridgeUnusable,
      ZWT_CLASS_BRIDGE_LINE_EMPTY: TorFailureClass.bridgeLineEmpty,
      ZWT_CLASS_BRIDGE_INVALID_TRANSPORT_OR_ADDRESS:
          TorFailureClass.bridgeInvalidTransportOrAddress,
      ZWT_CLASS_BRIDGE_INVALID_ADDRESS: TorFailureClass.bridgeInvalidAddress,
      ZWT_CLASS_BRIDGE_INVALID_IDENTITY: TorFailureClass.bridgeInvalidIdentity,
      ZWT_CLASS_BRIDGE_DUPLICATE_IDENTITY:
          TorFailureClass.bridgeDuplicateIdentity,
      ZWT_CLASS_BRIDGE_UNSUPPORTED_IDENTITY_TYPE:
          TorFailureClass.bridgeUnsupportedIdentityType,
      ZWT_CLASS_BRIDGE_UNSUPPORTED_CHANNEL_METHOD:
          TorFailureClass.bridgeUnsupportedChannelMethod,
      ZWT_CLASS_BRIDGE_DIRECT_PARAMETERS_NOT_ALLOWED:
          TorFailureClass.bridgeDirectParametersNotAllowed,
      ZWT_CLASS_BRIDGE_NO_RSA_IDENTITY: TorFailureClass.bridgeNoRsaIdentity,
      ZWT_CLASS_BRIDGE_SUPPORT_DISABLED: TorFailureClass.bridgeSupportDisabled,
      // The liveness rule's own class, appended at C3b.
      ZWT_CLASS_CIRCUITS_FAILING: TorFailureClass.circuitsFailing,
    };
    expect(table.keys.toList(), [for (var i = 0; i <= 21; i++) i]);
    for (final MapEntry(key: code, value: c) in table.entries) {
      expect(TorFailureClass.fromCode(code), c, reason: 'class $code');
      expect(c.code, code);
    }
    // The declaration order IS the code order (the header's order).
    expect(
      TorFailureClass.values
          .where((c) => c != TorFailureClass.unknown)
          .map((c) => c.code),
      [for (var i = 0; i <= 21; i++) i],
    );
    expect(TorFailureClass.fromCode(22), TorFailureClass.unknown);
    expect(TorFailureClass.values.where((c) => c.isBridge).map((c) => c.code), [
      for (var i = 6; i <= 20; i++) i,
    ]);
  });

  test('a status decodes by value and never throws on a bad field', () {
    final s = TorPluginStatus.fromCodes(
      phase: ZWT_PHASE_FAILED,
      readiness: 250,
      blockage: 99,
      failureClass: 99,
    );
    expect(s.phase, TorPluginPhase.failed);
    expect(s.readiness, 100);
    expect(s.blockage, TorBlockage.unknown);
    expect(s.failureClass, TorFailureClass.unknown);
    expect(s.isReady, isFalse);
  });
}
