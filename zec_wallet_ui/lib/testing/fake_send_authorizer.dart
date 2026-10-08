import 'dart:async';

import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';

/// A recording [WalletSendAuthorizer] for host-VM tests — proves the seam's
/// contract from the outside: every money-committing action routes through
/// [authorizeSpend] EXACTLY ONCE (see [intents]), and a denial ([denyAll])
/// reaches the controller BEFORE any bridge call (pair with the fake
/// session's call counters).
class FakeSendAuthorizer implements WalletSendAuthorizer {
  FakeSendAuthorizer({this.denyAll = false, this.prompt});

  /// When true, every [authorizeSpend] throws [WalletSpendAuthorizationDenied]
  /// WITHOUT running the action — the user dismissing the host's prompt.
  bool denyAll;

  /// When set, every [authorizeSpend] throws THIS (after recording the
  /// intent) without running the action — a host authorizer whose own
  /// credential step failed (NOT a user cancel). The controllers must
  /// classify it like a real failure, with zero bridge calls made.
  Object? throwBeforeAction;

  /// When set, AWAITED first (after recording the intent) — the call PARKS at
  /// the "host prompt" so a test can interleave events (a session swap, a
  /// second action) before the outcome lands (#330 harness). Called per
  /// authorization, so a test can gate calls selectively; it may itself throw
  /// [WalletSpendAuthorizationDenied] to deny just that call. On normal
  /// completion the call proceeds to [denyAll]/[throwBeforeAction]/the action.
  Future<void> Function(WalletSpendIntent intent)? prompt;

  /// Every intent authorized (or denied), in order — asserting on its length
  /// pins the exactly-once invariant; on its `kind`/`amountZat`, the prompt
  /// facts the host would render.
  final List<WalletSpendIntent> intents = [];

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    intents.add(intent);
    final prompt = this.prompt;
    if (prompt != null) await prompt(intent);
    if (denyAll) throw const WalletSpendAuthorizationDenied();
    final failure = throwBeforeAction;
    if (failure != null) throw failure;
    return action();
  }
}
