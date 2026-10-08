import 'dart:async';

import 'package:zec_wallet_ui/features/wallet/reveal_authorization.dart';

/// A recording [WalletRevealAuthorizer] for host-VM tests — proves the reveal
/// re-auth seam from the outside: the backup screen must route every reveal
/// through [authorizeReveal] and MUST NOT read the recovery words when re-auth
/// is denied or faults (pair with [FakeWalletProvisioner]'s `revealCount`).
class FakeRevealAuthorizer implements WalletRevealAuthorizer {
  FakeRevealAuthorizer({this.deny = false, this.throwBefore, this.gate});

  /// When true, [authorizeReveal] throws [WalletRevealReauthDenied] — the user
  /// dismissed the host prompt. The screen must stay pre-reveal, silently.
  bool deny;

  /// When set, [authorizeReveal] throws THIS instead — a non-cancel re-auth
  /// fault (the host's credential step errored). The screen must show its
  /// honest retry and read no words.
  Object? throwBefore;

  /// When set, AWAITED first so a test can PARK the re-auth (the host prompt
  /// held open) and interleave events — a background, a second tap — before it
  /// resolves. Complete it to let the reveal proceed.
  Completer<void>? gate;

  /// How many times re-auth was requested — asserting it pins that the reveal
  /// is gated exactly once per deliberate tap.
  int calls = 0;

  @override
  Future<void> authorizeReveal() async {
    calls++;
    final g = gate;
    if (g != null) await g.future;
    if (deny) throw const WalletRevealReauthDenied();
    final failure = throwBefore;
    if (failure != null) throw failure;
  }
}
