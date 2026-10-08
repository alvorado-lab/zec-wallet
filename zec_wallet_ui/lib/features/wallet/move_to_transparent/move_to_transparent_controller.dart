import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart' show DeliveryState, TxSubmitResult;

import '../send/send_state.dart';
import '../send/zec_amount.dart';
import '../send_authorization.dart';
import '../started_spend.dart';
import '../wallet_activity_controller.dart' show walletActivityProvider;
import '../wallet_providers.dart';
import '../wallet_session.dart';
import 'move_to_transparent_state.dart';

/// The Move-to-transparent flow state machine (§3.2i-1): loadAddress →
/// amountEntry → propose → review → send → result. Drives the move sheet; Rust
/// stays the single source of truth (design invariant 1) — every money step
/// forwards straight to the SDK and no money state is cached here.
///
/// Non-autoDispose (mirrors `sendControllerProvider` / `shieldControllerProvider`;
/// Riverpod 3.x has no `AutoDisposeNotifier`); a fresh sheet entry calls [start].
/// It reads the live [WalletSession] through `walletSessionProvider` (cached in
/// [build] for the actions), exactly the seam the tests override with a fake.
final moveToTransparentControllerProvider =
    NotifierProvider<MoveToTransparentController, MoveToTransparentState>(
      MoveToTransparentController.new,
    );

class MoveToTransparentController extends Notifier<MoveToTransparentState> {
  WalletSession? _session;

  /// A late async continuation touching `ref`/`state` after dispose throws; every
  /// post-await step re-checks this (the send/shield-controller discipline).
  bool _disposed = false;

  @override
  MoveToTransparentState build() {
    // A watched-dep change (the session flips) RE-RUNS build() on the SAME
    // notifier, firing the prior cycle's onDispose first → `_disposed` would
    // stick `true` and wedge every later `_set`. Reset it each build so the
    // controller stays live across a session change (the sheet re-drives `start`
    // via a `walletSessionProvider` listen — see the sheet).
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    // Cached for the actions. The session can legitimately flip (wallet closed /
    // re-opened) — build() re-runs and resets to loading.
    _session = ref.watch(walletSessionProvider);
    return const MoveLoadingAddress();
  }

  /// Sheet entry: reset to loading, then fetch the wallet's own transparent
  /// address. A fresh entry never lingers a prior terminal result. A NO-OP
  /// while a propose/sign is in flight (review F1): re-opening the sheet
  /// mid-flow RE-ATTACHES to the running step — clobbering the transient
  /// would make the identity guards swallow the landing outcome. (A pending
  /// address LOAD is restartable — it is the entry step itself.)
  Future<void> start() async {
    if (state is MovePreparing || state is MoveSubmitting) return;
    _set(const MoveLoadingAddress());
    await _loadAddress();
  }

  /// Fetch the OWN transparent address (Recv-2 / ADR-0528) — a LOCAL, no-network
  /// derivation. On success → [MoveAmountEntry]; on a wedged-FFI hang (the
  /// honest-degradation timeout the receive providers use) or any fault →
  /// [MoveUnavailable] (retryable). The address is the FIXED, SDK-supplied
  /// de-shield destination — never user input.
  Future<void> _loadAddress() async {
    final session = _requireSession();
    if (session == null) return;
    // NON-const: the post-await guards match on INSTANCE IDENTITY (#330) — a
    // stale load resolving after a session flip must never write the DEAD
    // identity's own-address into the fresh cycle (a cross-identity de-shield
    // destination would be a money hazard).
    // ignore: prefer_const_constructors
    final loading = MoveLoadingAddress();
    _set(loading);
    try {
      final address = await session.currentTransparentAddress().timeout(
        walletFfiWedgeTimeout,
      );
      if (_disposed || !identical(state, loading)) return;
      _set(MoveAmountEntry(ownAddress: address));
    } catch (_) {
      if (_disposed || !identical(state, loading)) return;
      _set(const MoveUnavailable(MoveFaultReason.couldNotLoadAddress));
    }
  }

  /// Retry the address load — ONLY from the terminal [MoveUnavailable] fault (the
  /// one state whose UI offers it). Guarded so a stray/late call can never clobber
  /// an in-flight prepare/send by resetting to loading (defensive consistency with
  /// [backToForm]'s source-state guard).
  Future<void> retryLoad() {
    if (state is! MoveUnavailable) return Future<void>.value();
    return _loadAddress();
  }

  /// Parse the amount, compose a payment to the OWN t-addr (NO memo — a
  /// transparent recipient can't carry one), and propose (the FIRST half). On
  /// success → [MoveReady] (the review with the de-shield disclosure); on a
  /// host-side parse fault or a typed propose failure → back to [MoveAmountEntry]
  /// with an honest inline fault. The amount is parsed host-side first
  /// (integer-exact, never via double); the SDK remains the binding gate.
  Future<void> prepare(String amountText) async {
    final current = state;
    if (current is! MoveAmountEntry) return; // only from the form
    final session = _requireSession();
    if (session == null) return;
    final ownAddress = current.ownAddress;

    final parsed = parseZecAmount(amountText);
    if (parsed is ZecAmountInvalid) {
      _set(
        MoveAmountEntry(
          ownAddress: ownAddress,
          fault: SendAmountFault(parsed.fault),
        ),
      );
      return;
    }
    final zat = (parsed as ZecAmountValid).zat;

    // Synchronous transient transition BEFORE the first await — the double-tap
    // interlock (a second prepare sees state != MoveAmountEntry and no-ops, so
    // it can't kick off a second propose). Captured for the INSTANCE-IDENTITY
    // guards below (#330).
    final preparing = MovePreparing(ownAddress);
    _set(preparing);

    final String uri;
    try {
      // Compose through the port (the bridge crossing lives in the adapter). The
      // recipient is the wallet's OWN address — `composePaymentUri` re-validates
      // it against the wallet's own network anyway (the SDK stays binding).
      uri = session.composePaymentUri(
        recipient: ownAddress,
        amountZat: zat,
        memoText: null,
      );
    } catch (error) {
      _set(
        MoveAmountEntry(
          ownAddress: ownAddress,
          fault: classifyProposeFailure(error),
        ),
      );
      return;
    }

    try {
      // The honest-degradation timeout (wrap review — mirrors the shield
      // prepare): propose is LOCAL, so a hang means the FFI boundary is
      // wedged, and with start() now an in-flight no-op (re-attachment) a
      // wedged Preparing would otherwise be permanently stuck.
      // TimeoutException classifies to the honest couldNotPrepare.
      final proposal = await session
          .propose(uri)
          .timeout(walletFfiWedgeTimeout);
      if (_disposed || !identical(state, preparing)) return;
      _set(
        MoveReady(proposal: proposal, ownAddress: ownAddress, movedZat: zat),
      );
    } catch (error) {
      if (_disposed || !identical(state, preparing)) return;
      _set(
        MoveAmountEntry(
          ownAddress: ownAddress,
          fault: classifyProposeFailure(error),
        ),
      );
    }
  }

  /// Sign + broadcast the reviewed de-shield (the SECOND half) — consumes the
  /// one-shot token by id, the SAME `send` path a payment uses. A per-tx
  /// broadcast failure is DATA, not a throw, so it lands as "saved for retry"; a
  /// thrown typed error routes per [classifyMoveSendFailure] — once the spend
  /// started, only a kind [singleStepSendErrorPrecedesPersistence] accepts;
  /// anything else lands on [MoveOutcomeUnknown] (R13). On a money-moving
  /// outcome the cold snapshot is invalidated so the shielded balance updates
  /// promptly (funds just LEFT the shielded pool).
  Future<void> confirm() async {
    final current = state;
    if (current is! MoveReady) return; // only from the review
    final session = _requireSession();
    if (session == null) return;
    final proposal = current.proposal;
    final ownAddress = current.ownAddress;

    // The host-authorization seam (#327) — a de-shield consumes its proposal
    // through the SAME signing path a payment does, so it is authorized the
    // same way.
    final authorizer = ref.read(walletSendAuthorizerProvider);

    // Synchronous transient transition before the await — the double-tap
    // interlock (a second confirm sees state != MoveReady and no-ops, so the
    // token is consumed at most once; the SDK's own one-shot guard is the
    // backstop, surfacing as already-submitted if a tap still slips through).
    // Captured for the INSTANCE-IDENTITY guards below (#330) — the authorizer
    // await spans user think-time at the host's prompt.
    final submitting = MoveSubmitting(proposal);
    _set(submitting);
    // The started-spend bookkeeping send uses (R13 §4.2, [runStartedSpend]):
    // the outcome is read from what the closure recorded, never from the
    // authorizer's return, and once the spend started only `send`'s own
    // refusal of a kind raised before persistence
    // ([singleStepSendErrorPrecedesPersistence]) keeps its classification.
    final run = await runStartedSpend<List<TxSubmitResult>>(
      precedesPersistence: singleStepSendErrorPrecedesPersistence,
      atMostOnceMessage:
          'WalletSendAuthorizer ran the move action twice — the contract '
          'is at most once (send_authorization.dart)',
      authorize: (start) => authorizer.authorizeSpend(
        WalletSpendIntent(
          kind: WalletSpendKind.unshield,
          amountZat: proposal.totalZat,
          // #383 R3: an unshield moves funds to the wallet's OWN transparent
          // address — a self-transfer by construction; the proposal knows the fee.
          recipientIsSelf: true,
          feeZat: proposal.feeZat,
          // FR-17 (#396): the proposal's spend-binding nonce — the de-shield
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
        // Funds just left the shielded pool — refresh the cold snapshot ABOVE
        // the identity guard (review F1): the balance must reflect a
        // LANDED outcome even when the state write below is rightly swallowed.
        ref.invalidate(walletSnapshotReadProvider);
        if (!identical(state, submitting)) return;
        // A z→own-t move has no TEX destination ⇒ `isTwoStepTex` is always
        // false here; threaded for the shared summarizer's SSOT discipline
        // (never a [SendTexInMotion]).
        _set(
          MoveSent(
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
        // unconsumed; back to the review, no fault (the seam contract).
        // Identity guard (review H2, tightened in #330 — now uniform
        // across all four money controllers, which all reset `_disposed` on
        // rebuild): a denial arriving after a mid-prompt session swap must
        // never write the DEAD session's review over the fresh build's state,
        // and a bare type check would pass for a NEW cycle's own Submitting.
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
        _landClassified(error, ownAddress, submitting);
      case SpendAnswerLost():
        // No classification survives a lost answer: the SDK's OWN
        // `ProposalAlreadyUsed` is on the single-step list and lands above, so
        // one here was substituted by the host — and "already submitted"
        // claims money moved, which only the SDK may say.
        _landOutcomeUnknown(submitting);
    }
  }

  void _landClassified(
    Object error,
    String ownAddress,
    MoveSubmitting submitting,
  ) {
    if (_disposed) return;
    final next = classifyMoveSendFailure(error, ownAddress);
    // An ALREADY-SUBMITTED outcome means a prior in-flight send already consumed
    // the token and moved the funds — refresh the shielded balance too, so the
    // card never shows a stale figure. ABOVE the identity guard (review
    // F1): money moved regardless of who's looking. A stale/busy fault moved
    // NO money, so it does not invalidate (no spurious snapshot churn).
    if (next is MoveSent && next.outcome is SendAlreadySubmitted) {
      ref.invalidate(walletSnapshotReadProvider);
    }
    if (!identical(state, submitting)) return;
    _set(next);
  }

  /// The terminal for a move that STARTED and lost its answer (R13 §4.3): the
  /// surfaces that show what really happened — the balance (the sheet's
  /// available figure reads it) and Activity — are re-read first, above the
  /// identity guard, as send's `_landOutcomeUnknown` does. The sheet's other
  /// read, the recoverable one-time-address list, a move cannot change.
  void _landOutcomeUnknown(MoveSubmitting submitting) {
    if (_disposed || !ref.mounted) return;
    ref.invalidate(walletSnapshotReadProvider);
    ref.invalidate(walletActivityProvider);
    if (!identical(state, submitting)) return;
    _set(const MoveOutcomeUnknown());
  }

  /// Return to the amount form from the review (Edit). A no-op from any other
  /// state (a transient/terminal) so a stray tap can't abandon an in-flight sign.
  void backToForm() {
    final current = state;
    if (current is! MoveReady) return;
    _set(MoveAmountEntry(ownAddress: current.ownAddress));
  }

  /// The live session, or null after surfacing the honest "wallet unavailable"
  /// terminal (the sheet is gated to an active wallet, so null is a defensive
  /// "wallet closed mid-sheet", never expected).
  WalletSession? _requireSession() {
    final session = _session;
    if (session == null) {
      _set(const MoveUnavailable(MoveFaultReason.walletUnavailable));
    }
    return session;
  }

  void _set(MoveToTransparentState next) {
    if (_disposed) return;
    state = next;
  }
}
