/// **A host authorizer whose behaviour around the spend is scripted** (R13,
/// `docs/plan/r13-a-saved-transaction-is-never-offered-a-retry.md` §4.5 rows
/// 1b, 2, 3, 5).
///
/// R13's contract turns on WHERE a host's throw lands relative to the spend
/// action: a decline or a `WalletSpendSessionChanged` BEFORE the action is a
/// spend that never started (today's routes); the same throw AFTER the action
/// ran is an answer the wallet lost (the unknown state, or the parked re-read).
/// The ad-hoc authorizers in the older files each model one of those points;
/// this one models all of them, so a row names its point instead of a class.
///
/// - [beforeAction] runs before the action and may throw (a decline, a
///   session change, a host fault — the spend never starts). It is also the
///   seam a test uses to change what a LATER read will see.
/// - [afterAction] runs once the action has SETTLED, with the action's error
///   (null when it returned a value). If it throws, that throw is what the
///   wallet sees — the host running the spend and then throwing. If it returns
///   normally the action's own outcome propagates unchanged.
library;

import 'dart:async';

import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';

class ScriptedAuthorizer implements WalletSendAuthorizer {
  ScriptedAuthorizer({this.beforeAction, this.afterAction});

  /// Runs before the spend action; a throw here means the action never runs.
  FutureOr<void> Function()? beforeAction;

  /// Runs after the action settled, with its error (or null). A throw here is
  /// a host throwing AFTER the spend ran.
  FutureOr<void> Function(Object? actionError)? afterAction;

  /// How many times the action was entered — the "the spend really ran" proof.
  int actionRuns = 0;

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    final before = beforeAction;
    if (before != null) await before();
    actionRuns += 1;
    final T value;
    try {
      value = await action();
    } catch (error, stack) {
      final after = afterAction;
      if (after != null) await after(error);
      Error.throwWithStackTrace(error, stack);
    }
    final after = afterAction;
    if (after != null) await after(null);
    return value;
  }
}

/// The host runs the spend and THEN throws its own (untyped) fault.
Future<void> hostFaultAfter(Object? _) =>
    Future<void>.error(StateError('host bookkeeping failed after the spend'));

/// The host runs the spend and THEN reports a decline.
Future<void> hostDeclineAfter(Object? _) =>
    Future<void>.error(const WalletSpendAuthorizationDenied());

/// The host runs the spend and THEN reports the identity-switch type.
Future<void> hostSessionChangedAfter(Object? _) =>
    Future<void>.error(const WalletSpendSessionChanged());
