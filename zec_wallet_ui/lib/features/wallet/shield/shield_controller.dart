import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart' show DeliveryState, TxSubmitResult;

import '../send/send_state.dart';
import '../send_authorization.dart';
import '../started_spend.dart';
import '../wallet_activity_controller.dart' show walletActivityProvider;
import '../wallet_providers.dart';
import '../wallet_session.dart';
import 'shield_state.dart';

/// The shield flow state machine (Recv-3): open → proposeShield → ready → send →
/// result. Drives the shield sheet; Rust stays the single source of truth (design
/// invariant 1) — every money step forwards straight to the SDK and no money
/// state is cached here. Mirrors `sendControllerProvider` (non-autoDispose,
/// Riverpod 3.x has no `AutoDisposeNotifier`); a fresh sheet entry calls [reset]
/// then [prepare].
final shieldControllerProvider =
    NotifierProvider<ShieldController, ShieldState>(ShieldController.new);

class ShieldController extends Notifier<ShieldState> {
  WalletSession? _session;

  /// A late async continuation touching `ref`/`state` after dispose throws; every
  /// post-await step re-checks this (the send/onboarding-controller discipline).
  bool _disposed = false;

  @override
  ShieldState build() {
    // A watched-dep change (the session flips) RE-RUNS build() on the SAME
    // notifier, firing the prior cycle's onDispose first → `_disposed` would
    // stick `true` and wedge every later `_set` (#330). Reset it each build;
    // the instance-identity guards on the post-await arms keep the old
    // cycle's in-flight continuations from writing into the new one.
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    // Cached for the actions. The session can legitimately flip to null (the
    // wallet closed) — `build()` re-runs and resets to idle.
    _session = ref.watch(walletSessionProvider);
    return const ShieldIdle();
  }

  /// Reset to idle — called on sheet entry so a prior terminal result never
  /// lingers into a new shield. A NO-OP while a step is in flight (
  /// review F1): re-opening the sheet mid-submit RE-ATTACHES to the running
  /// flow — clobbering the transient would make the identity guards swallow
  /// the landing outcome.
  void reset() {
    if (_inFlight) return;
    _set(const ShieldIdle());
  }

  /// The live session, or null after surfacing the honest "wallet unavailable"
  /// fault (the sheet is gated to an active wallet, so null is a defensive
  /// "wallet closed mid-sheet", never expected).
  WalletSession? _requireSession() {
    final session = _session;
    if (session == null) {
      _set(const ShieldUnavailable(ShieldFaultReason.walletUnavailable));
    }
    return session;
  }

  /// Propose the shield (the FIRST half) — DETERMINISTIC, LOCAL (no keys/proofs/
  /// network). On a shieldable balance → [ShieldReady] with the confirmable
  /// numbers; on `null` → [ShieldNothingToShield] (below the threshold, honest
  /// no-op); on a typed failure → [ShieldUnavailable].
  Future<void> prepare() async {
    if (_inFlight) return; // re-entrancy: ignore taps while a step runs
    final session = _requireSession();
    if (session == null) return;

    // Synchronous transient transition BEFORE the first await — the double-tap
    // interlock (a second prepare sees `_inFlight` and no-ops). NON-const: the
    // post-await guards match on INSTANCE IDENTITY (#330) — a canonicalized
    // const would alias a different build cycle's transient.
    // ignore: prefer_const_constructors
    final preparing = ShieldPreparing();
    _set(preparing);
    try {
      // The SAME honest-degradation timeout the receive-address providers use
      // (`walletFfiWedgeTimeout`): `proposeShield` is a LOCAL, no-network DB
      // read on the blocking pool, so a hang means the FFI boundary is WEDGED — not a
      // slow network. Without this the sheet would spin on "Preparing…" forever on a
      // money surface with no escape. A `TimeoutException` is not a `WalletApiError`,
      // so it routes through the catch to `ShieldUnavailable(couldNotPrepare)` (with a
      // "Try again"), exactly like the receive flow's wedged-load handling.
      final proposal = await session.proposeShield().timeout(
        walletFfiWedgeTimeout,
      );
      if (_disposed || !identical(state, preparing)) return;
      _set(
        proposal == null
            ? const ShieldNothingToShield()
            : ShieldReady(proposal),
      );
    } catch (error) {
      if (_disposed || !identical(state, preparing)) return;
      _set(classifyShieldPrepareFailure(error));
    }
  }

  /// Sign + broadcast the shield (the SECOND half) — consumes the one-shot token
  /// by id, the SAME `send` path a payment uses. A per-tx broadcast failure is
  /// DATA, not a throw, so it lands as "saved for retry"; a thrown typed error
  /// routes per [classifyShieldSendFailure] — once the spend started, only a
  /// kind [singleStepSendErrorPrecedesPersistence] accepts; anything else lands
  /// on [ShieldOutcomeUnknown] (R13). On a money-moving outcome the cold
  /// balance is invalidated so the transparent line updates promptly.
  Future<void> confirm() async {
    final current = state;
    if (current is! ShieldReady) return; // only from the confirm sheet
    final session = _requireSession();
    if (session == null) return;
    final proposal = current.proposal;

    // The host-authorization seam (#327) — a shield consumes its proposal
    // through the SAME signing path a payment does, so it is authorized the
    // same way.
    final authorizer = ref.read(walletSendAuthorizerProvider);

    // Synchronous transient transition before the await — the double-tap interlock
    // (a second confirm sees `state != ShieldReady` and no-ops, so the token is
    // consumed at most once; the SDK's own one-shot guard is the backstop,
    // surfacing as already-submitted if a tap still slips through). Captured
    // for the INSTANCE-IDENTITY guards below (#330) — the authorizer await
    // spans user think-time at the host's prompt.
    final submitting = ShieldSubmitting(proposal);
    _set(submitting);
    // The started-spend bookkeeping send uses (R13 §4.2, [runStartedSpend]):
    // the outcome is read from what the closure recorded, never from the
    // authorizer's return, and once the spend started only `send`'s own
    // refusal of a kind raised before persistence
    // ([singleStepSendErrorPrecedesPersistence]) keeps its classification.
    final run = await runStartedSpend<List<TxSubmitResult>>(
      precedesPersistence: singleStepSendErrorPrecedesPersistence,
      atMostOnceMessage:
          'WalletSendAuthorizer ran the shield action twice — the contract '
          'is at most once (send_authorization.dart)',
      authorize: (start) => authorizer.authorizeSpend(
        WalletSpendIntent(
          kind: WalletSpendKind.shield,
          amountZat: proposal.totalZat,
          // #383 R3: a shield is a self-transfer by construction; the proposal
          // knows the network fee — both ride the prompt's display-facts.
          recipientIsSelf: true,
          feeZat: proposal.feeZat,
          // FR-17 (#396): the proposal's spend-binding nonce — the shield
          // consumes its proposal through the same signing path a payment
          // does, so it binds the same way.
          bindingToken: proposal.binding,
        ),
        () {
          // The identity-switch fence: an approval landing after a
          // session flip must never spend from the DEAD identity's wallet
          // (see the seam contract). `ref.mounted` maps whole-scope teardown
          // to the same documented type.
          if (!ref.mounted ||
              !identical(ref.read(walletSessionProvider), session)) {
            throw const WalletSpendSessionChanged();
          }
          return start(() => session.send(proposal.proposalId));
        },
      ),
    );
    switch (run) {
      case SpendLanded(:final value, :final errorAfter):
        // The core's delivery reading for what did not go out (stage S8
        // `obligation`, row 10) — read from the session that sent, BEFORE the
        // guards below, so the outcome is one pure reduction of what landed.
        // A throw after `send` returned keeps the landing, read with no
        // delivery state (R13 §4.2).
        final delivery = errorAfter == null
            ? await landedDeliveryStates(session, value)
            : const <String, DeliveryState?>{};
        if (_disposed) return;
        // The transparent balance just changed (funds are leaving the
        // transparent pool); refresh the cold snapshot so the balance card
        // reflects it without waiting for the next sync tick. ABOVE the
        // identity guard (review F1): the balance must reflect a LANDED
        // outcome even when the state write below is rightly swallowed.
        ref.invalidate(walletSnapshotReadProvider);
        if (!identical(state, submitting)) return;
        // A t→z shield has no TEX destination ⇒ `isTwoStepTex` is always false
        // here; threaded for the shared summarizer's SSOT discipline (never a
        // [SendTexInMotion]).
        _set(
          ShieldDone(
            summarizeSendOutcome(
              value,
              isTwoStepTex: proposal.isTwoStepTex,
              delivery: delivery,
            ),
          ),
        );
      case SpendNotStarted(error: null || WalletSpendAuthorizationDenied()):
        // Declined at the host's prompt BEFORE any bridge call, or the
        // authorizer returned without running the spend — the token is
        // unconsumed; back to the confirm sheet, no fault (the seam contract).
        // Identity guard (review H2, tightened in #330): never restore
        // over a state the machine already moved past mid-prompt — a bare type
        // check would pass for a NEW cycle's own Submitting.
        if (_disposed || !identical(state, submitting)) return;
        _set(current);
      case SpendNotStarted(error: WalletSpendSessionChanged()):
        // The SDK's identity-switch fence — nothing was attempted; explicit arm
        // so the classifier can never shape it as a failure.
        return;
      case SpendNotStarted(:final error?) ||
          SpendRefusedBeforePersist(:final error):
        // Nothing started, or `send`'s own refusal raised before anything was
        // saved: today's classifier is still the truth.
        if (_disposed || !identical(state, submitting)) return;
        _set(classifyShieldSendFailure(error));
      case SpendAnswerLost():
        // No classification survives a lost answer: the SDK's OWN
        // `ProposalAlreadyUsed` is on the single-step list and lands above, so
        // one here was substituted by the host — and "already submitted"
        // claims money moved, which only the SDK may say.
        _landOutcomeUnknown(submitting);
    }
  }

  /// The terminal for a shield that STARTED and lost its answer (R13 §4.3):
  /// the surfaces that show what really happened — the balance and Activity —
  /// are re-read first, above the identity guard, as send's
  /// `_landOutcomeUnknown` does: a flow the machine has moved past may still
  /// have moved money.
  void _landOutcomeUnknown(ShieldSubmitting submitting) {
    if (_disposed || !ref.mounted) return;
    ref.invalidate(walletSnapshotReadProvider);
    ref.invalidate(walletActivityProvider);
    if (!identical(state, submitting)) return;
    _set(const ShieldOutcomeUnknown());
  }

  bool get _inFlight => state is ShieldPreparing || state is ShieldSubmitting;

  void _set(ShieldState next) {
    if (_disposed) return;
    state = next;
  }
}
