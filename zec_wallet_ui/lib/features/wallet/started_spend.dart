/// The started-spend bookkeeping every money controller shares (R13 §4.2):
/// whether the spend closure ever reached the SDK call, what that call
/// returned, and whether the error that came back is the SDK's own. One copy,
/// used by send's confirm and queue and by the shield and move confirms.
///
/// NOT exported: the controllers' own seam to the host's authorizer.
library;

/// Hand the SDK call to [runStartedSpend]'s bookkeeping, from INSIDE the spend
/// closure the host's authorizer runs. Call it as the closure's last step,
/// after the identity fence. [beforeEnter] runs after the at-most-once check
/// and before the spend counts as started — a refusal thrown there is not a
/// started spend.
typedef SpendStart<T> =
    Future<T> Function(
      Future<T> Function() sdkCall, {
      void Function()? beforeEnter,
    });

/// What became of one spend, read from the SDK's own record — never from what
/// the host's authorizer returned.
sealed class StartedSpend<T> {
  const StartedSpend();
}

/// The closure never reached the SDK call: nothing was signed or queued.
/// [error] is what the authorizer threw (a decline, the identity fence, a
/// refusal before entering), or `null` when it returned without running the
/// closure.
class SpendNotStarted<T> extends StartedSpend<T> {
  const SpendNotStarted(this.error);

  final Object? error;
}

/// The SDK call returned [value]. [errorAfter] is anything thrown after it
/// landed (the host's code, a later read) — the landing still stands: "unknown"
/// means only that the answer was lost, and here it was not.
class SpendLanded<T> extends StartedSpend<T> {
  const SpendLanded(this.value, {this.errorAfter});

  final T value;
  final Object? errorAfter;
}

/// The SDK call itself threw [error], and the flow's precedes-persistence
/// predicate accepts it: the core's own answer, raised before anything was
/// saved. The flow's classifier routes it as before.
class SpendRefusedBeforePersist<T> extends StartedSpend<T> {
  const SpendRefusedBeforePersist(this.error);

  final Object error;
}

/// The spend started and the answer was lost: the SDK threw a kind that may
/// follow persistence, or something else threw or declined after the closure
/// ran, or the authorizer returned without the SDK's result. [error] is `null`
/// in that last case. The flow lands on its "outcome unknown" state.
class SpendAnswerLost<T> extends StartedSpend<T> {
  const SpendAnswerLost(this.error);

  final Object? error;
}

/// Run one spend through the host's authorizer and say what became of it.
///
/// [authorize] calls the authorizer; its closure runs its own fences, then
/// returns `start(() => session.send(...))`. `start` enforces AT MOST ONCE (a
/// second run throws [StateError] with [atMostOnceMessage] — a second proposal
/// token, or on the queue a second payment), runs `beforeEnter`, marks the
/// spend started (and calls [onEntered]), and records the SDK call's result or
/// its own throw as they happen.
///
/// Never throws: every error comes back inside the verdict.
Future<StartedSpend<T>> runStartedSpend<T>({
  required Future<T> Function(SpendStart<T> start) authorize,
  required bool Function(Object error) precedesPersistence,
  required String atMostOnceMessage,
  void Function()? onEntered,
}) async {
  var entered = false;
  var hasLanded = false;
  T? landed;
  Object? sdkThrew;

  Future<T> start(
    Future<T> Function() sdkCall, {
    void Function()? beforeEnter,
  }) {
    if (entered) throw StateError(atMostOnceMessage);
    beforeEnter?.call();
    // Past this line a transaction may exist no matter what happens next.
    entered = true;
    onEntered?.call();
    return sdkCall().then(
      (value) {
        landed = value;
        hasLanded = true;
        return value;
      },
      onError: (Object error, StackTrace stack) {
        sdkThrew = error;
        Error.throwWithStackTrace(error, stack);
      },
    );
  }

  try {
    await authorize(start);
  } catch (error) {
    if (hasLanded) return SpendLanded<T>(landed as T, errorAfter: error);
    if (!entered) return SpendNotStarted<T>(error);
    if (identical(error, sdkThrew) && precedesPersistence(error)) {
      return SpendRefusedBeforePersist<T>(error);
    }
    return SpendAnswerLost<T>(error);
  }
  if (hasLanded) return SpendLanded<T>(landed as T);
  if (entered) return SpendAnswerLost<T>(null);
  return SpendNotStarted<T>(null);
}
