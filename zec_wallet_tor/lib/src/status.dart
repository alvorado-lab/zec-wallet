import 'bindings_generated.dart';

/// The plugin's phase — `zwt_status.phase`, one of the header's
/// `ZWT_PHASE_*` values.
///
/// Every code is bound to the generated constant, so a header change that
/// renumbers a phase fails `just wallet-tor-ffigen-verify` (the regenerated
/// constant differs) instead of decoding one phase as another.
enum TorPluginPhase {
  /// Initialised, nothing started yet.
  idle(ZWT_PHASE_IDLE),

  /// Tor is starting up; the wallet does not dial through it yet.
  bootstrapping(ZWT_PHASE_BOOTSTRAPPING),

  /// Tor is up and the wallet may dial through it.
  ready(ZWT_PHASE_READY),

  /// The last bootstrap attempt failed; [TorPluginStatus.failureClass] says
  /// why and [TorPluginStatus.blockage] what arti saw. The plugin retries on
  /// its own backoff; `ZecWalletTor.retryBootstrap` cuts the wait.
  failed(ZWT_PHASE_FAILED),

  /// The app is in the background: readiness is 0 and arti is quiet.
  suspended(ZWT_PHASE_SUSPENDED),

  /// Not registered with the wallet: after `dispose`, or after an engine
  /// that could not be built was retired. `init` registers again.
  notRegistered(ZWT_PHASE_NOT_REGISTERED),

  /// A code this Dart release does not know — the native half is newer or
  /// broken. Never a crash; the ABI version check should have refused the
  /// pair first.
  unknown(-1);

  const TorPluginPhase(this.code);

  /// The header's integer for this phase (`-1` for [unknown], which has none).
  final int code;

  /// The phase for a native code; an unrecognised one is [unknown].
  static TorPluginPhase fromCode(int code) {
    for (final p in values) {
      if (p != unknown && p.code == code) return p;
    }
    return unknown;
  }
}

/// What arti reports is stopping the bootstrap — `zwt_status.blockage`, the
/// header's `ZWT_BLOCKAGE_*` (arti's `BlockageKind`, closed).
enum TorBlockage {
  /// Nothing is blocking.
  none(ZWT_BLOCKAGE_NONE),

  /// Tor networking is disabled in the client's configuration.
  disabled(ZWT_BLOCKAGE_DISABLED),

  /// arti could not open any connection. NOTE: a censor dropping Tor traffic
  /// produces this too, so it is never read as "the device is offline".
  offline(ZWT_BLOCKAGE_OFFLINE),

  /// Connections open but are cut or filtered.
  filtering(ZWT_BLOCKAGE_FILTERING),

  /// The Tor network cannot be reached.
  cantReachTor(ZWT_BLOCKAGE_CANT_REACH_TOR),

  /// The device clock is too far off for Tor's directory to validate.
  clockSkewed(ZWT_BLOCKAGE_CLOCK_SKEWED),

  /// The bootstrap cannot complete for another reason arti names.
  cantBootstrap(ZWT_BLOCKAGE_CANT_BOOTSTRAP),

  /// arti reported a blockage it does not classify — and also where the code
  /// itself is one this Dart release does not know.
  unknown(ZWT_BLOCKAGE_UNKNOWN);

  const TorBlockage(this.code);

  /// The header's integer for this blockage.
  final int code;

  /// The blockage for a native code; an unrecognised one is [unknown].
  static TorBlockage fromCode(int code) {
    for (final b in values) {
      if (b.code == code) return b;
    }
    return unknown;
  }
}

/// Why the plugin failed or refused — `zwt_status.failure_class`, the
/// header's `ZWT_CLASS_*`, codes 1..=21 in the header's order plus [none].
/// Append-only on the native side: a code never changes meaning.
enum TorFailureClass {
  none(ZWT_CLASS_NONE, 'No failure.'),
  bootstrapDeadline(
    ZWT_CLASS_BOOTSTRAP_DEADLINE,
    'Tor did not finish starting within the bootstrap deadline.',
  ),
  notRegistered(
    ZWT_CLASS_NOT_REGISTERED,
    'The plugin is not registered with the wallet.',
  ),
  bootstrapFailed(ZWT_CLASS_BOOTSTRAP_FAILED, 'Tor failed to start.'),
  setup(
    ZWT_CLASS_SETUP,
    'The Tor client could not be built (its directory or configuration).',
  ),
  notBootstrapped(ZWT_CLASS_NOT_BOOTSTRAPPED, 'Tor has not started yet.'),
  bridgeConfigTooLong(
    ZWT_CLASS_BRIDGE_CONFIG_TOO_LONG,
    'The bridge text is longer than the plugin accepts.',
  ),
  bridgeTooManyLines(
    ZWT_CLASS_BRIDGE_TOO_MANY_LINES,
    'The bridge text has more lines than the plugin accepts.',
  ),
  bridgeLineTooLong(
    ZWT_CLASS_BRIDGE_LINE_TOO_LONG,
    'A bridge line is longer than the plugin accepts.',
  ),
  bridgePtUnsupported(
    ZWT_CLASS_BRIDGE_PT_UNSUPPORTED,
    'A bridge line names a pluggable transport this build does not carry.',
  ),
  bridgeUnusable(
    ZWT_CLASS_BRIDGE_UNUSABLE,
    'The bridge lines parsed but could not be used.',
  ),
  bridgeLineEmpty(ZWT_CLASS_BRIDGE_LINE_EMPTY, 'A bridge line is empty.'),
  bridgeInvalidTransportOrAddress(
    ZWT_CLASS_BRIDGE_INVALID_TRANSPORT_OR_ADDRESS,
    'A bridge line starts with neither a transport name nor an address.',
  ),
  bridgeInvalidAddress(
    ZWT_CLASS_BRIDGE_INVALID_ADDRESS,
    'A bridge line has an address that does not parse.',
  ),
  bridgeInvalidIdentity(
    ZWT_CLASS_BRIDGE_INVALID_IDENTITY,
    'A bridge line has a relay identity that does not parse.',
  ),
  bridgeDuplicateIdentity(
    ZWT_CLASS_BRIDGE_DUPLICATE_IDENTITY,
    'A bridge line gives the same kind of identity twice.',
  ),
  bridgeUnsupportedIdentityType(
    ZWT_CLASS_BRIDGE_UNSUPPORTED_IDENTITY_TYPE,
    'A bridge line uses an identity type this build does not support.',
  ),
  bridgeUnsupportedChannelMethod(
    ZWT_CLASS_BRIDGE_UNSUPPORTED_CHANNEL_METHOD,
    'A bridge line asks for a connection method this build does not support.',
  ),
  bridgeDirectParametersNotAllowed(
    ZWT_CLASS_BRIDGE_DIRECT_PARAMETERS_NOT_ALLOWED,
    'A direct bridge line carries transport parameters, which it may not.',
  ),
  bridgeNoRsaIdentity(
    ZWT_CLASS_BRIDGE_NO_RSA_IDENTITY,
    'A bridge line has no RSA identity, which Tor requires.',
  ),
  bridgeSupportDisabled(
    ZWT_CLASS_BRIDGE_SUPPORT_DISABLED,
    'Bridge support is not built into this Tor client.',
  ),

  /// A ready client whose dials kept failing: the plugin reports health
  /// FAILED to the wallet and rebuilds the client after its backoff.
  circuitsFailing(
    ZWT_CLASS_CIRCUITS_FAILING,
    'Tor was ready but its connections kept failing; it is being rebuilt.',
  ),

  /// A code this Dart release does not know.
  unknown(-1, 'The plugin reported a failure this release does not know.');

  const TorFailureClass(this.code, this.description);

  /// The header's integer for this class (`-1` for [unknown], which has none).
  final int code;

  /// One sentence for a developer or a log — not end-user copy (a host
  /// localises its own). Composed on the Dart side; no string crosses the C
  /// ABI.
  final String description;

  /// Whether this class is a refusal of the pasted bridge lines.
  bool get isBridge =>
      code >= ZWT_CLASS_BRIDGE_CONFIG_TOO_LONG &&
      code <= ZWT_CLASS_BRIDGE_SUPPORT_DISABLED;

  /// The class for a native code; an unrecognised one is [unknown].
  static TorFailureClass fromCode(int code) {
    for (final c in values) {
      if (c != unknown && c.code == code) return c;
    }
    return unknown;
  }
}

/// The plugin's status: one `zwt_status` read by value — four closed
/// integers, nothing else crosses.
final class TorPluginStatus {
  const TorPluginStatus({
    required this.phase,
    required this.readiness,
    this.blockage = TorBlockage.none,
    this.failureClass = TorFailureClass.none,
  });

  /// Decodes the four native integers. Never throws: an unknown code maps to
  /// the enum's `unknown`, and a readiness outside 0..=100 (a native defect)
  /// is clamped rather than passed on.
  factory TorPluginStatus.fromCodes({
    required int phase,
    required int readiness,
    required int blockage,
    required int failureClass,
  }) => TorPluginStatus(
    phase: TorPluginPhase.fromCode(phase),
    readiness: readiness.clamp(0, 100),
    blockage: TorBlockage.fromCode(blockage),
    failureClass: TorFailureClass.fromCode(failureClass),
  );

  final TorPluginPhase phase;

  /// 0..=100: the readiness the plugin last pushed to the wallet. The wallet
  /// dials only at 100.
  final int readiness;

  final TorBlockage blockage;

  final TorFailureClass failureClass;

  /// Whether the wallet can dial through Tor now.
  bool get isReady => phase == TorPluginPhase.ready && readiness == 100;

  @override
  bool operator ==(Object other) =>
      other is TorPluginStatus &&
      other.phase == phase &&
      other.readiness == readiness &&
      other.blockage == blockage &&
      other.failureClass == failureClass;

  @override
  int get hashCode => Object.hash(phase, readiness, blockage, failureClass);

  @override
  String toString() =>
      'TorPluginStatus(${phase.name}, $readiness%, '
      'blockage: ${blockage.name}, class: ${failureClass.name})';
}
