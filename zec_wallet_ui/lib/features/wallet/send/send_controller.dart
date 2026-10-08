import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart'
    show DeliveryState, SendProposal, TxSubmitResult;

import '../send_authorization.dart';
import '../started_spend.dart';
import '../wallet_providers.dart';
import '../wallet_session.dart';
import 'send_state.dart';
import 'wallet_send_request.dart';
import 'zec_amount.dart';

/// The send flow state machine (inc-2d-ui): form → propose → review → send →
/// result, plus the offline-first queue branch. Drives the send screen; Rust
/// stays the single source of truth (design invariant 1) — every money step
/// forwards straight to the SDK and no money state is cached here.
///
/// Non-autoDispose (mirrors `onboardingControllerProvider`; Riverpod 3.x has no
/// `AutoDisposeNotifier`): a fresh entry resets via [resetToForm]. It reads the
/// live [WalletSession] and the wallet network through providers (cached in
/// [build] for the actions), exactly the seam the screen tests override.
final sendControllerProvider = NotifierProvider<SendController, SendState>(
  SendController.new,
);

/// The TERMINAL FACT of the most recently completed send flow (FR-26), keyed by
/// [SendFlowOutcome.flowId]. Written by [SendController]; read by whoever owns
/// that flow's host report.
///
/// A SEPARATE channel from [sendControllerProvider] on purpose. The rendering
/// state is global and has no flow identity on it, and the controller
/// deliberately REFUSES some state writes — a landing that arrives after a
/// session flip is swallowed by the instance-identity guard, and a screen
/// reading that silence would report "nothing was created" over a transaction
/// that was signed and broadcast. This provider carries the fact regardless of
/// whether the state write happened.
final sendFlowOutcomeProvider =
    NotifierProvider<SendFlowOutcomeNotifier, SendFlowOutcome?>(
      SendFlowOutcomeNotifier.new,
    );

class SendFlowOutcomeNotifier extends Notifier<SendFlowOutcome?> {
  @override
  SendFlowOutcome? build() => null;

  /// Publish a flow's terminal. Deliberately watches nothing, so a session flip
  /// never resets it — the outcome of a flow that ran under the OLD identity
  /// must still reach the host that opened it.
  void publish(SendFlowOutcome outcome) => state = outcome;

  /// Drop a terminal once its owed report has been DELIVERED (duress hygiene,
  /// debt closed).
  ///
  /// The retention above is deliberate and stays: an UNDELIVERED outcome must
  /// survive an identity flip. But once the host has been told, the payload has
  /// done its whole job — and what it holds is the previous identity's TXIDS,
  /// resident in a ROOT provider that outlives the screen. The send screen
  /// clears its draft on a flip precisely so a coercer cannot read it; leaving
  /// payment identifiers here defeats that for the same threat.
  ///
  /// Flow-scoped on purpose: clears only if the resident terminal is the one
  /// named, so a screen can never drop a sibling flow's undelivered report.
  void consume(int flowId) {
    if (state?.flowId == flowId) state = null;
  }
}

class SendController extends Notifier<SendState> {
  WalletSession? _session;

  /// Identity of the flow the machine is currently running (FR-26).
  ///
  /// A "flow" is one entry's use of this controller: it begins at a reset (a
  /// screen entering) or a rebuild (an identity switch) and ends at a terminal.
  /// The id exists because ONE root-scoped controller serves every send screen,
  /// so "the machine reached a terminal" is not the same question as "MY flow
  /// reached a terminal" — and answering the first while asked the second is
  /// how one payment gets booked twice by two stacked entries.
  int get flowId => _flowId;
  int _flowId = 0;

  /// Flows whose spend closure was actually ENTERED (FR-26).
  ///
  /// The screen grades an abandoned flow at teardown, and `ref` is unsafe
  /// there, so it used to latch on "the machine reached a SendSubmitting /
  /// SendQueuing transient". That is a different question: a queue-time compose
  /// failure reaches the transient and signs nothing, and an authorizer denial
  /// reaches it and returns the user to a live Review. Both then graded as
  /// "we lost the answer" over a flow that provably created nothing (
  /// review). Only the controller knows whether the spend was entered; this is
  /// how it says so.
  bool hasEnteredSpend(int flowId) => _enteredFlows.contains(flowId);
  final Set<int> _enteredFlows = {};

  /// Mark [flowId] abandoned: the screen that ran it is gone (S13 §1a H1).
  ///
  /// The Back hold stops Back and `maybePop` only. A host `pop`/`go`/
  /// `replace` or a session teardown still disposes the screen, and during
  /// the host's authorizer prompt this controller's spend closure can still
  /// run afterwards — money moving under a screen that already told its host
  /// "no transaction". An abandoned flow's closure refuses BEFORE it enters,
  /// so that answer is structurally true. A flow that already entered is
  /// unaffected: its payment is under way and keeps going.
  void abandonFlow(int flowId) {
    if (_enteredFlows.contains(flowId)) return;
    _abandonedFlows.add(flowId);
  }

  final Set<int> _abandonedFlows = {};

  /// Whether the host request the running flow was opened with has lost its
  /// ability to spend (stage S8 `deadline`, R05) — the entry's mount grace
  /// ran out before a screen appeared and the host was told "no transaction",
  /// which is final for that request.
  ///
  /// A property of the REQUEST, handed in by the screen that holds it at the
  /// entry reset ([resetToForm]) — the one place a screen already tells this
  /// controller which flow is its own. NOT keyed on the report channel (a
  /// bare-request re-navigation drops it), NOT on the flow id (an in-place
  /// route update re-mints it; the screen re-binds the bit with every reset),
  /// and NOT on "reported" (a denied authorization and a retired channel are
  /// both reported without revoking anything, and the flow stays spendable).
  ///
  /// Read at BOTH seams where a spend is asked for — [confirm] and
  /// [queueOffline] — BEFORE the host's authorizer is invoked: the authorizer
  /// is an opaque per-spend bracket that must not learn about the entry, so a
  /// refusal that reached it would already be at the wrong door. The screen
  /// renders the expired state on its own as well; this is the money gate.
  bool get requestRevoked => _requestRevoked;
  bool _requestRevoked = false;

  /// The requests the entry's mount grace has revoked, for the lifetime of
  /// this process (stage S8 `deadline`; ADR-0558, the diff-review fold).
  ///
  /// Held HERE, on the one root-scoped controller, because a screen State
  /// cannot carry the bit through every door: the build kept it in
  /// `SendScreen`'s State and threaded it across an in-place route update,
  /// and a FRESH mount of the send route with the same bare request — the
  /// user pops the expired screen, the host `go`es back with the request it
  /// holds a final negative for — started with no previous State to read and
  /// paid. A screen holding a bare request (no channel) asks [isRevokedRequest]
  /// in `initState` and `didUpdateWidget` alike, so both doors read one answer.
  ///
  /// Written by the entry's grace timer ([revokeRequest]) BEFORE the reporter
  /// is revoked and the negative delivered. Monotone: nothing removes an
  /// entry, and it survives a session flip exactly as [_requestRevoked] does
  /// (fields outlive `build()` re-runs) — the host's negative is about the
  /// request, not the wallet. A LIVE channel keeps precedence over it in the
  /// screen: a fresh `push` of the same request is a new grant and pays, the
  /// restart the copy promises; only the bare doors read this memory. Identity
  /// is [sameSendRequest]'s. What it holds is the host's own request values
  /// (address, amount, memo, label) — never a txid or anything on chain.
  final List<WalletSendRequest> _revokedRequests = [];

  /// Record that the entry's mount grace revoked [request] (see
  /// [_revokedRequests]). Idempotent under [sameSendRequest].
  void revokeRequest(WalletSendRequest request) {
    if (isRevokedRequest(request)) return;
    _revokedRequests.add(request);
  }

  /// Whether [request] is one the entry's mount grace revoked — the same
  /// request under [sameSendRequest] — and therefore must not pay through a
  /// bare door (a re-navigation with the request alone, in place or on a fresh
  /// mount). The screen reads this for a bare request; a request that arrived
  /// on a live channel is answered by the channel, not by this memory.
  bool isRevokedRequest(WalletSendRequest request) =>
      _revokedRequests.any((revoked) => sameSendRequest(revoked, request));

  /// A late async continuation touching `ref`/`state` after dispose throws;
  /// every post-await step re-checks this (the onboarding-controller discipline).
  bool _disposed = false;

  @override
  SendState build() {
    // A watched-dep change (the session flips) RE-RUNS build() on the SAME
    // notifier, firing the prior cycle's onDispose first → `_disposed` would
    // stick `true` and wedge every later `_set` (#330 — one identity switch
    // left the screen permanently dead). Reset it each build so the controller
    // stays live across a session change; the INSTANCE-IDENTITY guards on the
    // post-await arms keep the old cycle's in-flight continuations from
    // writing into the new one.
    _disposed = false;
    // A rebuild is a NEW flow: the identity switch that triggers it resets the
    // machine to a clean form, and anything a previous entry was waiting on
    // belongs to the flow that ran under the previous identity.
    _flowId++;
    ref.onDispose(() => _disposed = true);
    // Cached for the actions. The session seam can legitimately flip to null (the
    // wallet closed) — `build()` re-runs and resets to a clean form, which is the
    // honest behavior (a send screen with no live wallet must not hold a stale
    // proposal).
    _session = ref.watch(walletSessionProvider);
    return const SendForm();
  }

  /// Reset to a clean form — called on screen entry so a prior terminal result
  /// (a completed/queued send) never lingers into a new send. A NO-OP while a
  /// step is in flight (review F1): the screens are exit-able
  /// mid-Submitting, so re-entering must RE-ATTACH to the running flow —
  /// clobbering the transient here would make the identity guards swallow the
  /// landing outcome, hiding a completed send exactly in the re-pay
  /// temptation window (mirrors [backToForm]'s in-flight guard).
  ///
  /// [requestRevoked] binds the entering request's revocation to the new flow
  /// (see [requestRevoked]); a revoked request lands on [SendRequestExpired]
  /// instead of a form it could never submit. The default is an organic or
  /// live entry, which clears any bit a previous flow carried — an organic
  /// send is a new act, never the revoked request's.
  void resetToForm({bool requestRevoked = false}) {
    if (_inFlight) return;
    // A NEW flow, even when the state value is unchanged (SendForm → SendForm
    // writes the same const and notifies nobody). The id is what tells an
    // ALREADY-MOUNTED screen that the machine is no longer running its flow —
    // without it, an idle entry sitting under a stacked one stays "armed" and
    // books the payment made on top of it.
    _flowId++;
    _requestRevoked = requestRevoked;
    _set(requestRevoked ? const SendRequestExpired() : const SendForm());
  }

  /// The live session, or null after surfacing the honest "wallet unavailable"
  /// fault (the screen is gated to an active wallet, so null is a defensive
  /// "wallet closed mid-screen", never expected). One home for the guard the
  /// three actions share.
  WalletSession? _requireSession() {
    final session = _session;
    if (session == null) {
      _set(
        const SendForm(
          fault: SendCategoricalFault(SendFaultReason.walletUnavailable),
        ),
      );
    }
    return session;
  }

  /// Return to the form from the review/result/queued screens (the Back / Send
  /// another / Edit actions). A no-op mid-flight (a transient state) so a stray
  /// tap can't abandon an in-flight sign.
  void backToForm() {
    if (state is SendPreparing ||
        state is SendSubmitting ||
        state is SendQueuing) {
      return;
    }
    _set(const SendForm());
  }

  /// Compose the ZIP-321 URI from the form and propose (the FIRST half). On
  /// success → [SendReview] with the confirmable numbers; on a typed failure →
  /// back to [SendForm] with an honest inline fault. The amount is parsed
  /// host-side first (integer-exact, never via double).
  Future<void> prepare({
    required String address,
    required String amountText,
    String? memo,
    WalletMachineMemo? machineMemo,
  }) async {
    if (_inFlight) return; // re-entrancy: ignore taps while a step runs
    final session = _requireSession();
    if (session == null) return;

    final parsed = parseZecAmount(amountText);
    if (parsed is ZecAmountInvalid) {
      _set(SendForm(fault: SendAmountFault(parsed.fault)));
      return;
    }
    final zat = (parsed as ZecAmountValid).zat;
    // The host's policy ceiling (e.g. an alpha cap) — refused BEFORE any
    // bridge call, like the parse gate above (`null` = no ceiling).
    final ceiling = ref.read(walletSendCeilingZatProvider);
    if (ceiling != null && zat > ceiling) {
      _set(SendForm(fault: SendOverCeiling(ceiling)));
      return;
    }
    final recipient = address.trim();

    // Synchronous transient transition BEFORE the first await — the double-tap
    // interlock (a second prepare sees `_inFlight` and no-ops). Deliberately
    // NON-const: the post-await guards match on INSTANCE IDENTITY (#330) — a
    // canonicalized const would alias a different build cycle's transient, so
    // a session flip + fresh prepare could let the DEAD cycle's continuation
    // write its stale proposal over the live one.
    // ignore: prefer_const_constructors
    final preparing = SendPreparing();
    _set(preparing);

    final String uri;
    try {
      // Compose through the port (the bridge crossing lives in the adapter); a
      // bad address / un-sendable memo throws a typed error here, synchronously.
      uri = session.composePaymentUri(
        recipient: recipient,
        amountZat: zat,
        memoText: memo,
        // FR-28: the host's opaque bytes ride EVERY compose, not just the
        // first. The form re-composes from its own fields on each Review tap,
        // which is exactly where the bytes used to be dropped silently.
        memoBytes: machineMemo?.bytes,
      );
    } catch (error) {
      _set(SendForm(fault: classifyProposeFailure(error)));
      return;
    }

    try {
      // The same honest-degradation timeout the shield prepare uses (
      // wrap review): propose is a LOCAL, no-network call, so a hang means
      // the FFI boundary is WEDGED — and with the entry resets now in-flight
      // no-ops (re-attachment, review F1) a wedged Preparing would otherwise
      // be a permanently stuck money screen. TimeoutException classifies to
      // couldNotPrepareTransient: a wallet still scanning, not bad input.
      final proposal = await session
          .propose(uri)
          .timeout(walletFfiWedgeTimeout);
      if (_disposed || !identical(state, preparing)) return;
      _set(
        SendReview(
          proposal: proposal,
          recipient: recipient,
          // FR-28: the purpose travels to the REVIEW so the disclosure and the
          // host's authorization prompt read the same sentence from one place.
          machineMemoPurpose: machineMemo?.purpose,
        ),
      );
    } catch (error) {
      if (_disposed || !identical(state, preparing)) return;
      _set(SendForm(fault: classifyProposeFailure(error)));
    }
  }

  /// Sign + broadcast the reviewed proposal (the SECOND half) — consumes the
  /// one-shot token by id. A per-tx broadcast failure is DATA, not a throw, so it
  /// lands on the result screen as "saved for retry"; a thrown typed error routes
  /// per [classifySendFailure] (already-submitted / sign-failed / stale → form).
  Future<void> confirm() async {
    final current = state;
    if (current is! SendReview) return; // only from the confirm screen
    // Stage S8 `deadline`: a request the host already holds a final "no
    // transaction" for cannot pay — refused HERE, before the session is even
    // asked for and before the authorizer below is read, so no host bracket
    // opens for a spend that will not happen. Lands on the expired state, not
    // back on Review: a review with a Confirm that can never go through is a
    // money surface lying about what the tap does.
    if (_requestRevoked) {
      _set(const SendRequestExpired());
      return;
    }
    // S13 H1: an abandoned flow's spend would refuse inside the closure —
    // refuse it HERE instead, so the host's prompt never opens for a spend
    // that cannot happen (the fold review's LOW). Reached from a screen
    // that re-attached to a Review another screen left.
    if (_abandonedFlows.contains(_flowId)) {
      _restartAbandoned();
      return;
    }
    final session = _requireSession();
    if (session == null) return;
    final proposal = current.proposal;

    // The host-authorization seam (#327) — read before the transition so a
    // disposed ref can't be touched later. The transient state below also
    // covers the host's prompt (it renders modally over the screen).
    final authorizer = ref.read(walletSendAuthorizerProvider);

    // Synchronous transient transition before the await — the double-tap
    // interlock (a second confirm sees `state != SendReview` and no-ops, so the
    // token is consumed at most once; the SDK's own one-shot guard is the
    // backstop, surfacing as SendAlreadySubmitted if a tap still slips through).
    // Captured for the INSTANCE-IDENTITY guards below (#330): the authorizer
    // await spans user think-time at the host's prompt, so a session flip (and
    // even a whole fresh confirm) can happen mid-flight — a type check alone
    // would pass for the NEW cycle's transient.
    final submitting = SendSubmitting(proposal);
    _set(submitting);
    // FR-26: the flow this confirm belongs to, and whether the spend was ever
    // ENTERED. The flag is the difference between "no money moved" and "we lost
    // the answer": everything below the `session.send` line can throw AFTER the
    // transaction is signed, persisted and broadcast — the host's own
    // `authorizeSpend` wraps the closure, so its bookkeeping throwing lands in
    // the same `catch` as a refused sign. Reserving the hard "nothing was
    // created" claim for the not-entered case is the whole point.
    final flowId = _flowId;
    // The started-spend bookkeeping (R13 §4.2, [runStartedSpend]): whether the
    // closure reached `session.send`, the SDK's OWN record of what it returned,
    // and whether a throw is `send`'s own. `authorizeSpend` is HOST code
    // wrapping our closure, and its return value is whatever the host chooses
    // to hand back — a host that runs the closure and discards the result, or
    // never runs it, could otherwise return a fabricated
    // `List<TxSubmitResult>` and make the wallet's own result screen say "Sent"
    // over invented txids. Everything below reads the verdict, never the
    // authorizer's return. Only a kind the core provably raises BEFORE anything
    // is persisted ([sendErrorPrecedesPersistence]) keeps its classification
    // once the spend started; everything else past that point — a store fault
    // the core can raise after the transaction is persisted, the host's code
    // throwing, declining or reporting a session change after the closure ran,
    // an untyped throw — is an answer this layer lost, and lands on
    // [SendOutcomeUnknown].
    final run = await runStartedSpend<List<TxSubmitResult>>(
      precedesPersistence: sendErrorPrecedesPersistence,
      // Run AT MOST ONCE. `send_authorization.dart` states this as a contract
      // clause; a clause with no predicate behind it is a comment. A second run
      // here would consume a second proposal token, and on the queue twin it is
      // a straight double-pay at drain.
      atMostOnceMessage:
          'WalletSendAuthorizer ran the spend action twice — the contract '
          'is at most once (send_authorization.dart)',
      onEntered: () => _enteredFlows.add(flowId),
      authorize: (start) => authorizer.authorizeSpend(
        WalletSpendIntent(
          kind: WalletSpendKind.send,
          amountZat: proposal.totalZat,
          // #383 R3 display-facts: bind the host's prompt to WHAT is signed.
          // The elided form of the review's recipient (render-never-log).
          // `selfSend` is safe to forward as the whole-spend self claim ONLY
          // because this flow composes a SINGLE-payment URI (composePaymentUri
          // above) — the core's selfSend is ANY-leg, so with exactly one
          // payment leg "any leg is self" ≡ "the recipient is self" (
          // see the coupling note on [WalletSpendIntent.recipientIsSelf]). A
          // future multi-payment propose path must derive its own value.
          recipientAbbrev: abbreviateWalletAddress(current.recipient),
          recipientIsSelf: proposal.selfSend,
          feeZat: proposal.feeZat,
          // FR-17 (#396): the proposal's spend-binding nonce — a host-custody
          // supplier records it at stage time so the sign-time native seed
          // pull can only serve THIS reviewed proposal.
          bindingToken: proposal.binding,
          // FR-28: a per-spend-credential host's prompt IS the authorization
          // moment for its user, so it gets the same sentence the review shows.
          machineMemoPurpose: current.machineMemoPurpose,
        ),
        () {
          // The identity-switch fence: an approval landing after a
          // session flip must never spend from the DEAD identity's wallet —
          // no surface could show the outcome (see the seam contract). The
          // `ref.mounted` leg maps whole-scope teardown to the same
          // documented type instead of leaking riverpod's internal
          // unmounted-ref throw through the host's authorizer.
          if (!ref.mounted ||
              !identical(ref.read(walletSessionProvider), session)) {
            throw const WalletSpendSessionChanged();
          }
          // `start` checks at most once, then — S13 §1a H1 — refuses an
          // abandoned flow before entering (the screen that ran it is gone and
          // has told its host "no transaction", so it stays true), then marks
          // the spend started and records what `send` returns or throws.
          return start(
            () => session.send(proposal.proposalId),
            beforeEnter: () {
              if (_abandonedFlows.contains(flowId)) {
                throw const _FlowAbandoned();
              }
            },
          );
        },
      ),
    );
    switch (run) {
      case SpendLanded(:final value, :final errorAfter):
        await _landSent(
          session,
          proposal,
          submitting,
          flowId,
          value,
          // A throw after `send` returned (the host's code, a delivery read):
          // the landing stands, read with no delivery state (R13 §4.2).
          readDelivery: errorAfter == null,
        );
      case SpendNotStarted(error: null):
        // The authorizer returned without the closure ever running: nothing
        // was signed. Honest refusal rather than a report built on nothing.
        _publishFlow(SendFlowNothingCreated(flowId));
        if (_disposed || !identical(state, submitting)) return;
        _set(current);
      case SpendNotStarted(error: WalletSpendAuthorizationDenied()):
        // The user (or a host policy) declined the host's authorization prompt
        // BEFORE any bridge call — the proposal token is unconsumed, so land
        // back on review, ready for another confirm. The host's own prompt was
        // the communication; no additional fault banner (see the seam
        // contract). The transient guard (review H2, tightened to IDENTITY in
        // #330): restore ONLY over our own Submitting instance — if the machine
        // moved on mid-prompt (a session swap re-ran build, possibly all the way
        // into a NEW confirm's own Submitting), writing the old review back
        // would resurrect a dead session's proposal.
        //
        // FR-26 — and the correction the review forced. A denial returns the
        // user to a LIVE Review with a working Confirm, so the flow is not
        // over: publishing a terminal here consumed the one-shot channel, and
        // the very next tap paid while the host held "nothing was created" and
        // the payee held the money. A nothing-created terminal must only be
        // published for a flow that can no longer spend.
        if (_disposed || !identical(state, submitting)) return;
        _set(current);
      case SpendNotStarted(error: WalletSpendSessionChanged()):
        // The SDK's own identity-switch fence fired — NOTHING was attempted
        // (the fence throws before `session.send`). Explicit arm: the generic
        // classifier below would shape this as a "payment failed" lie, kept
        // off-screen today only by the identity guard — never rely on that
        // incidentally. The host is owed the honest "nothing was created" —
        // structurally, not by inference.
        _publishFlow(SendFlowNothingCreated(flowId));
      case SpendNotStarted(error: _FlowAbandoned()):
        // S13 §1a H1: nothing was signed — the refusal is ahead of `entered`.
        _publishFlow(SendFlowNothingCreated(flowId));
        if (_disposed || !identical(state, submitting)) return;
        _restartAbandoned();
      case SpendNotStarted(:final error?):
        // Not entered: nothing was created, structurally.
        final classified = classifySendFailure(error);
        if (classified is SendSent) {
          _publishFlow(
            SendFlowSent(
              flowId,
              outcome: classified.outcome,
              txids: classified.txids,
              isTwoStepTex: proposal.isTwoStepTex,
              singleRecipientZat: proposal.singleRecipientZat,
            ),
          );
        }
        // Not entered and routed back to the FORM (a stale anchor, a busy
        // store): the flow is still ALIVE and still spendable, so nothing is
        // published — closing the one-shot channel there is what let a retry
        // pay while the host held "nothing was created".
        if (_disposed || !identical(state, submitting)) return;
        _set(classified);
      case SpendRefusedBeforePersist(:final error):
        // `session.send`'s own typed refusal of a kind raised before anything
        // is persisted (S7 U1): its classification is still the truth.
        final classified = classifySendFailure(error);
        _publishAfterEntered(classified, flowId, proposal);
        if (_disposed || !identical(state, submitting)) return;
        _set(classified);
      case SpendAnswerLost():
        // ENTERED DOMINATES THE CLASSIFIER, in EVERY arm — a decline, a
        // session-changed report, or any throw from a host that ran the spend
        // first (R13 §4.2): the money it moved does not un-move because of
        // what the prompt returned, and a live Review there would pay a
        // second time. No classification survives here: the SDK's OWN
        // `ProposalAlreadyUsed` is on the precedes-persistence list and never
        // reaches this arm, so one that does was substituted by the host —
        // and "already submitted" is a claim that money moved, which only the
        // SDK may make.
        // The host is told we lost the answer — published BEFORE the identity
        // guard, so it hears it even when the fence fired and `build()` moved
        // the state — and so is the screen (S7 U1): no "nothing was sent", no
        // Try again, no error text — "check Activity first".
        _publishFlow(SendFlowIndeterminate(flowId));
        _landOutcomeUnknown(submitting, queued: false);
    }
  }

  /// A spend that ENTERED and got the SDK's own typed answer: already-submitted
  /// is the one informative classification (the screen's "already submitted,
  /// not a second payment" and the host's report are the same fact); anything
  /// else is the host's "we lost the answer" (FR-26) while the screen keeps the
  /// core's classification.
  void _publishAfterEntered(
    SendState classified,
    int flowId,
    SendProposal proposal,
  ) {
    if (classified is SendSent && classified.outcome is SendAlreadySubmitted) {
      _publishFlow(
        SendFlowSent(
          flowId,
          outcome: const SendAlreadySubmitted(),
          txids: const [],
          isTwoStepTex: proposal.isTwoStepTex,
          singleRecipientZat: proposal.singleRecipientZat,
        ),
      );
    } else {
      _publishFlow(SendFlowIndeterminate(flowId));
    }
  }

  /// Land a `send` whose results came back — the SDK's own record. With
  /// [readDelivery] false (something threw after the results landed) the
  /// outcome is reduced with no delivery state: "saved", never a promise.
  Future<void> _landSent(
    WalletSession session,
    SendProposal proposal,
    SendSubmitting submitting,
    int flowId,
    List<TxSubmitResult> results, {
    required bool readDelivery,
  }) async {
    // ONE reading of the landing, shared by the host report and the screen
    // These were computed twice from the same `results`,
    // which is the shape that lets the two surfaces disagree about a payment:
    // a future summarizer that is not a pure function of its arguments — a
    // clock, a counter, a read of live state — would have the screen say one
    // thing and the host record another, with no red anywhere. The core's
    // delivery reading is taken ONCE, here, and handed to the pure reducer as
    // an argument (stage S8 `obligation`, row 10): "saved for retry" is what
    // the wallet reported, never a fallback.
    final delivery = readDelivery
        ? await landedDeliveryStates(session, results)
        : const <String, DeliveryState?>{};
    final outcome = summarizeSendOutcome(
      results,
      isTwoStepTex: proposal.isTwoStepTex,
      delivery: delivery,
    );
    final txids = sendTxids(results);
    // The flow's terminal, published BEFORE the `_disposed` and transient
    // guards below (FR-26). Those guards correctly refuse to WRITE STATE for
    // a flow the machine has moved past — but the money moved, and the host
    // that opened this flow is owed the fact. Reading their silence as an
    // absence is how "nothing was created" gets said over a broadcast
    // transaction.
    _publishFlow(
      SendFlowSent(
        flowId,
        outcome: outcome,
        txids: txids,
        isTwoStepTex: proposal.isTwoStepTex,
        // FR-46: the proposal whose id went to `send` above.
        singleRecipientZat: proposal.singleRecipientZat,
      ),
    );
    if (_disposed) return;
    // Refresh the in-flight two-step cue (#309) AT OUTCOME-LANDING, not at a
    // button press: the user can leave the result screen by ANY exit (Done,
    // "Send another", the system back gesture, the AppBar arrow) and the
    // wallet screen beneath holds the provider's pre-send read. The next sync
    // edge can be a block away (~75s) — exactly the window the durable
    // "don't send it again" cue exists for. ABOVE the identity guard (
    // review F1): the durable cue must reflect a LANDED outcome even when
    // the state write below is rightly swallowed. (The Done/"Send another"
    // invalidations remain as defense in depth.) The SNAPSHOT refreshes too
    // the in-flight cue is two-step-only, so for the
    // commonest send shape — a plain shielded send whose outcome lands while
    // the user is away — the balance was the only lever left against the
    // "did it go through?" re-pay window.
    ref.invalidate(walletInFlightSendsReadProvider);
    ref.invalidate(walletSnapshotReadProvider);
    if (!identical(state, submitting)) return;
    _set(
      SendSent(
        outcome,
        // The ids the reducer drops — the FR-26 host report cites them, so
        // they are carried on the state rather than re-derived anywhere.
        txids: txids,
      ),
    );
  }

  /// Publish a flow's terminal fact (FR-26). Guarded on `ref.mounted` because
  /// this runs on post-await paths that a whole-scope teardown can outlive.
  void _publishFlow(SendFlowOutcome outcome) {
    if (!ref.mounted) return;
    ref.read(sendFlowOutcomeProvider.notifier).publish(outcome);
  }

  /// Queue the send for OFFLINE-first delivery — durably persist the intent (no
  /// network/signing/money movement) and land on [SendQueued]. The offline fork:
  /// proposing is deferred to send-time so a long-queued send never carries a
  /// stale anchor. A typed failure (e.g. queue full) returns to the form.
  Future<void> queueOffline({
    required String address,
    required String amountText,
    String? memo,
    WalletMachineMemo? machineMemo,
  }) async {
    if (_inFlight) return;
    // Stage S8 `deadline`: the queue commits a future spend, so it is the
    // second seam the revoked request is refused at — same gate as confirm(),
    // same place: before anything below (the parse, the ceiling, the compose,
    // the authorizer) runs for it.
    if (_requestRevoked) {
      _set(const SendRequestExpired());
      return;
    }
    // S13 H1: as in confirm() — no prompt for a spend that cannot happen.
    if (_abandonedFlows.contains(_flowId)) {
      _restartAbandoned();
      return;
    }
    // The entry state — restored VERBATIM on an authorization denial so a
    // fault that OFFERED the queue (notSyncedYet while online) survives the
    // cancelled prompt and the affordance doesn't vanish.
    final entry = state;
    final session = _requireSession();
    if (session == null) return;

    final parsed = parseZecAmount(amountText);
    if (parsed is ZecAmountInvalid) {
      _set(SendForm(fault: SendAmountFault(parsed.fault)));
      return;
    }
    final zat = (parsed as ZecAmountValid).zat;
    // The same host policy ceiling as prepare() — the offline queue is a send
    // too; a cap that only guarded the online path would be a bypass.
    final ceiling = ref.read(walletSendCeilingZatProvider);
    if (ceiling != null && zat > ceiling) {
      _set(SendForm(fault: SendOverCeiling(ceiling)));
      return;
    }

    // The host-authorization seam (#327): queuing COMMITS a future spend, so
    // it is authorized like one — even though the signing itself happens at
    // background drain (the [walletOfflineQueueSupportedProvider] caveat).
    final authorizer = ref.read(walletSendAuthorizerProvider);

    // NON-const for the identity guards below, like confirm()'s transient
    // (#330) — the authorizer await spans the host's prompt.
    // ignore: prefer_const_constructors
    final queuing = SendQueuing();
    _set(queuing);
    // FR-26: same discipline as confirm() — the flow's id; whether the enqueue
    // was ever entered, what it returned and what it threw are the shared
    // started-spend bookkeeping ([runStartedSpend]).
    final flowId = _flowId;
    final String uri;
    try {
      uri = session.composePaymentUri(
        recipient: address.trim(),
        amountZat: zat,
        memoText: memo,
        // FR-28: the offline queue composes the same URI, so it carries the
        // same bytes — a queued payment that lost the host's reference would
        // drain later as an unattributable one.
        memoBytes: machineMemo?.bytes,
      );
    } catch (error) {
      _set(SendForm(fault: classifyQueueFailure(error)));
      return;
    }
    final run = await runStartedSpend<String>(
      // S7 U1: past `entered`, only `queueSend`'s own typed refusal of a kind
      // raised before the intent is written ([queueErrorPrecedesPersistence]:
      // a full queue, a watch-only or closing wallet) keeps its retryable form
      // fault.
      precedesPersistence: queueErrorPrecedesPersistence,
      // At most once — and here the contract has teeth it lacked: a queued
      // send has NO one-shot token, so a second run enqueues a SECOND payment
      // and both drain (`send_authorization.dart` says so; it was a comment,
      // not a predicate — security review).
      atMostOnceMessage:
          'WalletSendAuthorizer ran the queue action twice — a queued send '
          'has no one-shot token, so this is a double pay at drain',
      onEntered: () => _enteredFlows.add(flowId),
      authorize: (start) => authorizer.authorizeSpend(
        WalletSpendIntent(
          kind: WalletSpendKind.queuedSend,
          amountZat: zat,
          // #383 R3: what is known at QUEUE time — the typed recipient. No
          // proposal exists yet (signing happens at drain), so no fee and no
          // self-send detection; false here means "unknown", per the doc.
          recipientAbbrev: abbreviateWalletAddress(address),
          // FR-17: no bindingToken — the binding is minted at enqueue and
          // surfaced on the parked-send row for re-stage.
          // FR-28: the queue commits the same memo, so it discloses the same.
          machineMemoPurpose: machineMemo?.purpose,
        ),
        () {
          // The identity-switch fence — see confirm(); a queuedSend
          // has NO one-shot token, so a fenceless late approval would commit
          // a spend the dead identity's user can never see or cancel.
          if (!ref.mounted ||
              !identical(ref.read(walletSessionProvider), session)) {
            throw const WalletSpendSessionChanged();
          }
          // At most once (see `atMostOnceMessage`), then — S13 §1a H1, as in
          // confirm() — a gone screen's flow never commits.
          return start(
            () => session.queueSend(uri),
            beforeEnter: () {
              if (_abandonedFlows.contains(flowId)) {
                throw const _FlowAbandoned();
              }
            },
          );
        },
      ),
    );
    switch (run) {
      case SpendLanded(:final value):
        // The SDK's own id, never the authorizer's return value (see
        // confirm()) — and it stands even when the host threw, declined or
        // reported a session change after `queueSend` returned it (R13 §4.2):
        // the answer was not lost. FR-26: published before the guards, same
        // reason as
        // confirm(). The id is the core's own `ParkedSend.id` — the ONLY
        // handle a host has onto a committed payment that has no txid yet,
        // and it was being discarded.
        _publishFlow(SendFlowQueued(flowId, queuedSendId: value));
        if (_disposed) return;
        // Refresh the parked "saved & pending" surface AT OUTCOME-LANDING (#309
        // holistic H2): queueing happens OFFLINE by design, so no sync edge
        // will fire — without this a queued send stays invisible on the wallet
        // screen until app resume, and a worried user re-queues (both drain
        // when connectivity returns — a double pay). ABOVE the identity guard
        // (review F1), same rationale as confirm()'s in-flight invalidation.
        ref.invalidate(walletParkedSendsReadProvider);
        if (!identical(state, queuing)) return;
        _set(SendQueued(queuedSendId: value));
      case SpendNotStarted(error: null):
        // The authorizer returned without ever running the closure: nothing was
        // committed, and the form is still live.
        if (_disposed || !identical(state, queuing)) return;
        _set(entry is SendForm ? entry : const SendForm());
      case SpendNotStarted(error: WalletSpendAuthorizationDenied()):
        // Declined at the host's prompt — nothing was queued; back to the
        // (still-populated) form EXACTLY as entered, prior fault included (a
        // notSynced fault is what OFFERS the queue button — dropping it would
        // hide the affordance the user just used). No NEW banner (the seam
        // contract). Same identity guard as confirm (review H2 / #330).
        //
        // Nothing published when the closure never ran: the form is still
        // live and the queue button still works, so this flow can still commit
        // a payment — closing its one-shot channel here would leave the next
        // one unreportable.
        if (_disposed || !identical(state, queuing)) return;
        _set(entry is SendForm ? entry : const SendForm());
      case SpendNotStarted(error: WalletSpendSessionChanged()):
        // The SDK's identity-switch fence — nothing was queued; explicit arm
        // so the classifier can never shape it as a failure. The fence throws
        // before `queueSend`, so nothing was committed.
        _publishFlow(SendFlowNothingCreated(flowId));
      case SpendNotStarted(error: _FlowAbandoned()):
        // S13 §1a H1: nothing was queued — the refusal is ahead of `entered`.
        _publishFlow(SendFlowNothingCreated(flowId));
        if (_disposed || !identical(state, queuing)) return;
        _restartAbandoned();
      case SpendNotStarted(:final error?):
        // Not entered ⇒ the flow is alive and nothing is published.
        if (_disposed || !identical(state, queuing)) return;
        _set(SendForm(fault: classifyQueueFailure(error)));
      case SpendRefusedBeforePersist(:final error):
        // Entered, and `queueSend`'s own refusal raised before the intent is
        // written: the host still hears that this layer cannot vouch for the
        // flow, and the form keeps its retryable fault.
        _publishFlow(SendFlowIndeterminate(flowId));
        if (_disposed || !identical(state, queuing)) return;
        _set(SendForm(fault: classifyQueueFailure(error)));
      case SpendAnswerLost():
        // Entered and no id came back ⇒ the intent may be durably committed —
        // `queueSend` threw a kind that can follow the write, or the host
        // swallowed its result or what it threw (S7 U1; R13 §4.2: EVERY arm,
        // a decline and a session change included, checks it).
        // The honest answer is that this layer cannot tell — published BEFORE
        // the identity guard — and the screen lands on "check your pending
        // payments first", never a form whose Queue button would commit a
        // second payment.
        _publishFlow(SendFlowIndeterminate(flowId));
        _landOutcomeUnknown(queuing, queued: true);
    }
  }

  /// The terminal for a flow that ENTERED its spend and then lost the answer
  /// (S7 U1): [SendOutcomeUnknown], with no retry and no error payload. The
  /// wallet surfaces that would show what really happened — the balance, the
  /// in-flight cue, the parked list — are re-read first, above the identity
  /// guard, for the same reason the success landings do it there: a flow the
  /// machine has moved past still moved money.
  void _landOutcomeUnknown(SendState transient, {required bool queued}) {
    if (_disposed || !ref.mounted) return;
    ref.invalidate(walletSnapshotReadProvider);
    ref.invalidate(walletInFlightSendsReadProvider);
    ref.invalidate(walletParkedSendsReadProvider);
    if (!identical(state, transient)) return;
    _set(SendOutcomeUnknown(queued: queued));
  }

  bool get _inFlight =>
      state is SendPreparing || state is SendSubmitting || state is SendQueuing;

  /// After an abandoned flow's refusal, while a screen is still watching the
  /// machine (one that re-attached to it, or one revealed under the screen
  /// that left). Restoring the old Review would leave every later tap under
  /// the abandoned id — refused, forever, with no message (the diff
  /// review's MEDIUM). So: a NEW flow, and the form with a fault that says to
  /// review again. The next Review is a tap on a live screen, which drives
  /// (and claims) the new flow.
  ///
  /// A FRESH fault instance, never the const: the queue's early refusal
  /// lands form → form with no transient between, and the screen's
  /// re-announce and re-scroll key on the fault's identity.
  void _restartAbandoned() {
    _flowId++;
    _set(
      SendForm(
        // ignore: prefer_const_constructors
        fault: SendCategoricalFault(SendFaultReason.amountsExpired),
      ),
    );
  }

  void _set(SendState next) {
    if (_disposed) return;
    state = next;
  }
}

/// The spend closure's refusal for an abandoned flow (S13 §1a H1). Private:
/// it never leaves this controller — the arms above catch it before a host
/// could see it as anything but the authorizer's own call failing.
class _FlowAbandoned implements Exception {
  const _FlowAbandoned();
}
