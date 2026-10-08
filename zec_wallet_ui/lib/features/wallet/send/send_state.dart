import 'dart:async' show TimeoutException;

import 'package:zec_wallet/zec_wallet.dart';

import '../wallet_session.dart';
import 'wallet_send_report.dart';
import 'zec_amount.dart';

/// The send flow's states (inc-2d-ui), a sealed family so the screen renders each
/// phase with an exhaustive `switch` (no default blind spot). The flow is
/// form → (propose) → review → (send) → result, with an offline-first queue
/// branch. Rendering layer ONLY — every transition runs through
/// [SendController]; no money state is stored here (design invariant 1; Rust is
/// the single source of truth). The DTOs it carries ([SendProposal]) are display
/// projections — the opaque proposal token stays Rust-side.
sealed class SendState {
  const SendState();
}

/// The editable send form. [fault] is an inline, honest message from a failed
/// prepare/queue (a bad address, not enough funds, …) — `null` on first entry.
class SendForm extends SendState {
  const SendForm({this.fault});

  final SendFormFault? fault;
}

/// `propose` in flight — deterministic + local (note-selection/fee), no network.
/// Transient; a spinner.
class SendPreparing extends SendState {
  const SendPreparing();
}

/// The confirm screen: show the EXACT numbers + the §5.1 de-shield disclosure,
/// then [SendController.confirm] signs and broadcasts. Holds the display
/// [proposal] (its opaque token is consumed by id on confirm) and the [recipient]
/// the user typed, echoed back so they verify WHO they're paying.
class SendReview extends SendState {
  const SendReview({
    required this.proposal,
    required this.recipient,
    this.machineMemoPurpose,
  });

  final SendProposal proposal;

  /// FR-28: the human-readable purpose for an attached machine memo, or null
  /// when the payment carries none. Carried on the REVIEW because that is the
  /// authorization step — the disclosure the user reads and the sentence the
  /// host's own prompt receives come from this one value, so they can never
  /// drift apart. The BYTES are not here: they are not display data, and the
  /// review must never render them.
  final String? machineMemoPurpose;

  /// The recipient address the user entered (display-only — the proposal carries
  /// pools + amounts, not addresses; this lets the confirm screen show it back).
  final String recipient;
}

/// `send` in flight (sign in a blocking proving task + persist + broadcast).
/// Transient; a spinner. Holds the proposal so a UI can keep showing the figures.
class SendSubmitting extends SendState {
  const SendSubmitting(this.proposal);

  final SendProposal proposal;
}

/// `queueSend` in flight (durably persist the intent). Transient; a spinner.
class SendQueuing extends SendState {
  const SendQueuing();
}

/// Terminal result of a `send` — see [SendOutcome] for the honest variants.
class SendSent extends SendState {
  const SendSent(this.outcome, {this.txids = const []});

  final SendOutcome outcome;

  /// The transaction ids this send minted, hex, in mint order — the identifiers
  /// [SendOutcome] deliberately does not carry (it is SHARED with the shield and
  /// move-to-transparent flows, which have no host to report to, and it counts
  /// rather than identifies).
  ///
  /// Present on every arm that MINTED a transaction, whatever the endpoint said
  /// about it — a broadcast failure is data, not a lost payment. Empty for the
  /// arms where nothing was created (a refused sign, an already-consumed token),
  /// and empty for a send whose results carried no id at all (the forward-compat
  /// `TxSubmitResult.unknown`).
  ///
  /// Used by the FR-26 host report; the result screen renders counts, never ids.
  final List<String> txids;
}

/// Terminal result of an offline `queueSend` — the intent is durably stored and
/// will send on the next online sync.
class SendQueued extends SendState {
  const SendQueued({this.queuedSendId});

  /// The core's own id for the parked send (`ParkedSend.id`, the value
  /// `queueSend` returned) — the key the wallet's parked/re-stage surfaces are
  /// keyed by, and the only handle a host has onto a payment that has no txid
  /// yet. `null` only on the paths that never reached `queueSend`.
  final String? queuedSendId;
}

/// The host's request this flow was opened with can no longer spend (stage S8
/// `deadline`, R05): the entry's mount grace ran out before a send screen
/// appeared, the host was told "no transaction", and that answer is final for
/// the request. Nothing to pay, nothing to retry — the screen states the fact
/// and the user starts again from the host.
///
/// Reached two ways, both honest: the entry reset of a screen whose request
/// arrived revoked lands here instead of on [SendForm], and a spend asked for
/// under a revoked request — [SendController.confirm] or
/// [SendController.queueOffline], the two seams where the host's authorizer
/// would otherwise be invoked — is refused onto it. Terminal-shaped: the only
/// exit is leaving the screen.
class SendRequestExpired extends SendState {
  const SendRequestExpired();
}

/// The flow ENTERED its spend and then lost the answer (S7 U1): the closure
/// ran — a transaction may be signed and broadcast, or an intent durably
/// queued — and something past that point threw or declined (the host's own
/// authorizer code, an untyped throw). Neither "sent" nor "nothing was sent"
/// is true, so the screen says neither: it points at where the truth is
/// (Activity for a send, the pending payments for a queue) and offers no retry.
/// Deliberately carries NO error — nothing here renders a throw's text.
///
/// The screen-side twin of [SendFlowIndeterminate]. A typed error the SDK's
/// own `send` / `queueSend` raised is NOT this state: that is the core's
/// answer about the spend and keeps its classification.
class SendOutcomeUnknown extends SendState {
  const SendOutcomeUnknown({required this.queued});

  /// The queue path ([SendController.queueOffline]) rather than a send.
  final bool queued;
}

// ---------------------------------------------------------------------------
// The TERMINAL FACT about one send flow (FR-26)
// ---------------------------------------------------------------------------

/// What became of ONE send flow, addressed by [flowId] — the authoritative
/// answer, published by [SendController] and consumed by whoever owns that
/// flow's host report.
///
/// **Why this exists instead of reading [SendState].** A report is a claim
/// about ONE flow, and [SendState] is a GLOBAL rendering state with no flow
/// identity on it — so a screen watching it can grade a payment that was never
/// its own (two entries stacked on one controller). Worse, the controller
/// legitimately DROPS a state write it must not perform: a landing that arrives
/// after a session flip is swallowed by the instance-identity guard, and reading
/// that silence as "nothing happened" claims no money moved over a transaction
/// that was signed and broadcast. This channel carries the fact even when the
/// state write is correctly refused.
///
/// The four angles of the review each reached one of those two shapes
/// independently; they are the same defect, and this is the one mechanism that
/// closes both.
sealed class SendFlowOutcome {
  const SendFlowOutcome(this.flowId);

  /// The flow this is the terminal of. A consumer reports ONLY for the flow it
  /// owns — see [SendController.flowId].
  final int flowId;
}

/// The flow reached a `send`. [outcome] is the honest reduction; [txids] are the
/// ids it minted (see [sendTxids]); [isTwoStepTex] is the proposal's shape
/// signal, carried because the reducer's own two-step residual cannot be
/// re-derived downstream (core #307). [singleRecipientZat] is the SENT
/// proposal's one-recipient total (FR-46) — copied from the proposal whose id
/// went to `send`, never a re-proposed or displayed one; `null` for two
/// recipients.
class SendFlowSent extends SendFlowOutcome {
  const SendFlowSent(
    super.flowId, {
    required this.outcome,
    required this.txids,
    required this.isTwoStepTex,
    this.singleRecipientZat,
  });

  final SendOutcome outcome;
  final List<String> txids;
  final bool isTwoStepTex;
  final int? singleRecipientZat;
}

/// The flow durably queued an intent offline. No transaction, no txid — but the
/// core's [queuedSendId] IS the join key onto the parked-send surfaces.
class SendFlowQueued extends SendFlowOutcome {
  const SendFlowQueued(super.flowId, {required this.queuedSendId});

  final String? queuedSendId;
}

/// The flow ENTERED the spend and then lost the answer — the sign+broadcast ran
/// and something threw after it (a host authorizer's own bookkeeping, an SDK
/// error raised past the point of no return). Money MAY have moved and this
/// layer cannot tell.
///
/// Distinct from [SendFlowNothingCreated] on purpose: collapsing the two is how
/// a machine-readable payment record comes to assert "no money moved" over a
/// minted, persisted transaction.
class SendFlowIndeterminate extends SendFlowOutcome {
  const SendFlowIndeterminate(super.flowId);
}

/// The flow ended with NOTHING signed and nothing queued — structurally, not by
/// inference: the spend closure was never entered.
class SendFlowNothingCreated extends SendFlowOutcome {
  const SendFlowNothingCreated(super.flowId);
}

// ---------------------------------------------------------------------------
// Outcome of a completed `send`
// ---------------------------------------------------------------------------

/// The honest result of a `send` — a broadcast failure is NOT a lost payment
/// (each tx is persisted and re-sent by the resubmission machinery), so the
/// variants distinguish "went out", "saved for retry", "already done", and
/// "couldn't build" rather than success/error.
sealed class SendOutcome {
  const SendOutcome();
}

/// Every tx broadcast successfully.
class SendSucceeded extends SendOutcome {
  const SendSucceeded(this.txCount);

  final int txCount;
}

/// Some (or none) of the txs reached the network. Each is PERSISTED and will be
/// re-broadcast on the next sync — money-safe, the user already confirmed the
/// numbers. The honest "saved, will finish sending when you're back online".
class SendSavedForRetry extends SendOutcome {
  const SendSavedForRetry({required this.broadcast, required this.total});

  /// How many txs actually went out (0..total).
  final int broadcast;

  /// Total txs the proposal minted (>1 when crossing pools, §1.7).
  final int total;
}

/// A two-step TEX (ZIP-320) send where SOME but not all legs were accepted —
/// typically tx0 (the unshield to a wallet-controlled one-time address) out but
/// the forwarding tail (tx1) not; since core #307 also the reverse shape, where
/// tx0 read "already known" (the wallet's own background re-broadcast raced the
/// send and landed it first) and tx1 was accepted — either way the endpoint knows
/// tx0. The funds are IN MOTION on a one-time address the wallet controls —
/// money has LEFT the shielded pool but NOT confirmed at the recipient. This is
/// NOT an ordinary "each tx independently saved-for-retry" partial: the two legs
/// are SEQUENCED (tx1 spends tx0's output, so it can only mine after tx0). The honest
/// terminal copy says ONLY the permanently-true things — "on its way" + "don't
/// send it again" + "recover from your wallet if it doesn't complete" — and MUST
/// NOT promise auto-completion: tx1 can expire (~40 blocks, the common mobile
/// case) into a strand that only the shipped recovery (the wallet-screen
/// recover-now / sweep) resolves (spec §3.2i-2 UX-honesty (i); the durable
/// parked/recoverable surface owns the later in-motion→stranded truth and
/// re-notifies). Distinguished from [SendSavedForRetry] purely by
/// [SendProposal.isTwoStepTex] (the SSOT shape signal, 2e-2b-v-5a) — only the
/// interactive SEND path produces it; shield (t→z) and move-to-transparent
/// (z→own-t) have no TEX destination, so they never do.
class SendTexInMotion extends SendOutcome {
  const SendTexInMotion({required this.broadcast, required this.total});

  /// Legs that went out — always ≥1 here (a zero-broadcast two-step put nothing
  /// in motion and reads as [SendSavedForRetry]).
  final int broadcast;

  /// Legs the two-step minted (2 for a ZIP-320 pair).
  final int total;
}

/// The wallet KEPT the signed payment but has not promised to send it on its
/// own: for some transaction that did not go out, the core's per-transaction
/// delivery reading was not [DeliveryState.retryPending] — a held state, no
/// state at all, or a reading that could not be taken. Distinct from
/// [SendSavedForRetry] on purpose (stage S8 `obligation`, row 10): "saved —
/// we'll finish sending" is said ONLY when the core reports that it will retry;
/// this arm says "saved" and points at Activity, where the live delivery state
/// is rendered. Money-safe either way — the bytes are persisted.
class SendKept extends SendOutcome {
  const SendKept({required this.broadcast, required this.total});

  /// How many txs actually went out (0..total).
  final int broadcast;

  /// Total txs the proposal minted.
  final int total;
}

/// The one-shot token was already consumed (a double-tap / re-entry). The notes
/// are NEVER broadcast twice (§6.3) — honest "already submitted", not an error.
class SendAlreadySubmitted extends SendOutcome {
  const SendAlreadySubmitted();
}

/// Build/sign failed — no money moved. "Couldn't complete this payment; nothing
/// was sent." The user re-proposes (the consumed token can't be re-sent).
class SendSignFailed extends SendOutcome {
  const SendSignFailed();
}

/// Reduce the per-tx [TxSubmitResult] list to an honest [SendOutcome]. A tx is
/// "out" only on [TxSubmitResult_Success]; every other arm (grpc/submit failure,
/// not-attempted) means persisted-but-unsent, and whether the wallet will
/// retry it is the CORE's to say, not this reducer's to assume.
///
/// [delivery] is the core's per-transaction delivery reading for the txids that
/// did not come back accepted (see [deliveryStatesFor]) — stage S8
/// `obligation`, row 10: [SendSavedForRetry] is produced ONLY when every such
/// transaction reads [DeliveryState.retryPending] (or is already
/// [DeliveryState.accepted] / [DeliveryState.confirmed] — the wallet's own
/// background pass landed it between the send and this read, the core #307
/// race shape, now readable); when EVERY one already reads accepted or
/// confirmed the payment is out and the outcome is [SendSucceeded]. Anything
/// else — a held [DeliveryState.persisted], `null` (no reading, or the read
/// failed), a txid the map does not know, a result arm that carries no txid —
/// is [SendKept]: saved, no promise. An empty list (a proposal always mints
/// ≥1 tx — defensive) has nothing the core could retry, so it reads [SendKept]
/// 0/0, never a promise over nothing.
///
/// [isTwoStepTex] is the proposal's SSOT shape signal ([SendProposal.isTwoStepTex],
/// 2e-2b-v-5a). A two-step TEX with ANY leg accepted (but not all) is
/// [SendTexInMotion], NOT [SendSavedForRetry] — the legs are sequenced, and an
/// accepted leg means the endpoint knows tx0 (an honest endpoint can only accept
/// tx1 if it does), so the funds are in motion on a wallet-controlled one-time
/// address and the copy must not over-promise auto-completion. A zero-broadcast
/// two-step put nothing in motion, so it takes the delivery-decided arms above.
/// Pure + total — unit-tested at its boundary without a device.
SendOutcome summarizeSendOutcome(
  List<TxSubmitResult> results, {
  required bool isTwoStepTex,
  required Map<String, DeliveryState?> delivery,
}) {
  final total = results.length;
  if (total == 0) return const SendKept(broadcast: 0, total: 0);
  final broadcast = results.whereType<TxSubmitResult_Success>().length;
  if (broadcast == total) return SendSucceeded(total);
  if (isTwoStepTex && broadcast > 0) {
    return SendTexInMotion(broadcast: broadcast, total: total);
  }
  var allOut = true;
  for (final result in results) {
    if (result is TxSubmitResult_Success) continue;
    final txid = _txidOf(result);
    final state = txid == null ? null : delivery[txid];
    switch (state) {
      case DeliveryState.accepted:
      case DeliveryState.confirmed:
        break;
      case DeliveryState.retryPending:
        allOut = false;
      case DeliveryState.persisted:
      case DeliveryState.unknown:
      case null:
        return SendKept(broadcast: broadcast, total: total);
    }
  }
  if (allOut) return SendSucceeded(total);
  return SendSavedForRetry(broadcast: broadcast, total: total);
}

/// The txid a submit result names, or `null` for an arm that carries none (the
/// forward-compat `unknown`). The one place [sendTxids] and the reducer agree on.
String? _txidOf(TxSubmitResult result) => switch (result) {
  TxSubmitResult_Success(:final txidHex) => txidHex,
  TxSubmitResult_GrpcFailure(:final txidHex) => txidHex,
  TxSubmitResult_SubmitFailure(:final txidHex) => txidHex,
  TxSubmitResult_NotAttempted(:final txidHex) => txidHex,
  _ => null,
};

/// The core's delivery reading for every transaction in [results] that did NOT
/// come back accepted, keyed by txid — the [summarizeSendOutcome] input that
/// makes "saved for retry" a reported fact (stage S8 `obligation`, row 10)
/// rather than a fallback. A read that fails typed is recorded as `null`: the
/// reducer then lands on [SendKept] (saved, no promise) — under-promising is
/// the honest direction when the wallet's answer could not be taken.
Future<Map<String, DeliveryState?>> deliveryStatesFor(
  WalletSession session,
  List<TxSubmitResult> results,
) async {
  final out = <String, DeliveryState?>{};
  for (final result in results) {
    if (result is TxSubmitResult_Success) continue;
    final txid = _txidOf(result);
    if (txid == null) continue;
    try {
      out[txid] = await session.deliveryState(txid);
    } on WalletApiError {
      out[txid] = null;
    }
  }
  return out;
}

/// [deliveryStatesFor] for a spend whose results already LANDED (R13 §4.2):
/// any throw — not only a typed one — reads as "no delivery state", so the
/// landed outcome still shows ("saved", no promise), never "outcome unknown".
/// Unknown means only that the answer was lost; here it was not.
Future<Map<String, DeliveryState?>> landedDeliveryStates(
  WalletSession session,
  List<TxSubmitResult> results,
) async {
  try {
    return await deliveryStatesFor(session, results);
  } catch (_) {
    return const <String, DeliveryState?>{};
  }
}

/// The transaction ids in [results], in order — every arm that HAS one, not
/// only the accepted ones. A tx that got no verdict (`grpcFailure`), was
/// rejected from the mempool (`submitFailure`), or was never attempted still
/// EXISTS: it is signed, persisted, and re-broadcast by the resubmission
/// machinery, so a host must be able to cite it. The forward-compat
/// `TxSubmitResult.unknown` arm carries no id and contributes none — which is
/// why a caller checks for emptiness rather than assuming one id per result.
/// Pure + total.
List<String> sendTxids(List<TxSubmitResult> results) => [
  for (final result in results)
    if (_txidOf(result) case final String txid) txid,
];

/// The ONE translation from a flow's terminal fact to the host's report
/// (FR-26). Pure + total, so it is unit-tested at its boundary with no widget
/// tree and no device.
///
/// Lives HERE, beside the other reducers, and not in the public report file:
/// the report DTOs are the host's vocabulary and must not depend on this
/// package's internal flow states — the dependency runs internal → public.
///
/// Three honesty rules are enforced here rather than trusted to callers:
/// - **The txid list, not the outcome variant, decides whether a transaction
///   exists.** An outcome that says "saved for retry" over an EMPTY result set
///   created nothing to cite, and reporting that as a transaction hands the host
///   a payment record it can never reconcile.
/// - **The motion axis is three-valued.** A two-step (ZIP-320) send whose legs
///   ALL came back unaccepted can be the core #307 already-known race — the
///   wallet's own background pass landed them first and the funds ARE in motion
///   — and this layer cannot tell. Saying `false` there invites a host to offer
///   "pay another way" over a payment already on its way.
/// - **The recipient amount rides a TAGGED report only** (FR-46): set when
///   [correlationId] is non-null, whatever the motion — an untagged push gets
///   nothing new.
WalletSendReport walletSendReportFor(
  SendFlowOutcome flow, {
  String? correlationId,
}) {
  switch (flow) {
    case SendFlowNothingCreated():
      return WalletSendNoTransaction(correlationId: correlationId);
    case SendFlowIndeterminate():
      return WalletSendUnclassified(correlationId: correlationId);
    case SendFlowQueued(:final queuedSendId):
      return WalletSendQueuedOffline(
        queuedSendId: queuedSendId,
        correlationId: correlationId,
      );
    case SendFlowSent(
      :final outcome,
      :final txids,
      :final isTwoStepTex,
      :final singleRecipientZat,
    ):
      switch (outcome) {
        case SendAlreadySubmitted():
          return WalletSendAlreadySubmitted(correlationId: correlationId);
        case SendSignFailed():
          return WalletSendNoTransaction(correlationId: correlationId);
        case SendSucceeded():
        case SendSavedForRetry():
        case SendKept():
        case SendTexInMotion():
          if (txids.isEmpty) {
            return WalletSendUnclassified(correlationId: correlationId);
          }
          final broadcast = switch (outcome) {
            SendSucceeded(:final txCount) => txCount,
            SendSavedForRetry(:final broadcast) => broadcast,
            SendKept(:final broadcast) => broadcast,
            SendTexInMotion(:final broadcast) => broadcast,
            _ => 0,
          };
          return WalletSendTransactionCreated(
            txids: txids,
            broadcastCount: broadcast,
            motion: switch (outcome) {
              SendTexInMotion() => WalletSendMotion.inMotion,
              // The #307 residual: a two-step with NOTHING accepted is either
              // "nothing went out" or "the background pass already landed both".
              // Indistinguishable here, so say so.
              _ when isTwoStepTex && broadcast == 0 =>
                WalletSendMotion.indeterminate,
              _ => WalletSendMotion.notInMotion,
            },
            // FR-46: TAGGED reports only — the tag is what makes the host's
            // join honest; an untagged push gets nothing new (ADR-0558's
            // one-sided-tag rule).
            recipientAmountZat: correlationId == null
                ? null
                : singleRecipientZat,
            correlationId: correlationId,
          );
      }
  }
}

// ---------------------------------------------------------------------------
// Form faults
// ---------------------------------------------------------------------------

/// A FIXABLE form fault surfaced inline so the user corrects the input on the
/// SAME form (never a full-screen failure that discards what they typed). Reads
/// only the typed `WalletApiError.kind` (or a host-side parse fault) — never an
/// error payload — so nothing sensitive (address/amount) leaks (§5.4).
sealed class SendFormFault {
  const SendFormFault();
}

/// Host-side amount parse failure (before any bridge call) — see [ZecAmountFault].
class SendAmountFault extends SendFormFault {
  const SendAmountFault(this.fault);

  final ZecAmountFault fault;
}

/// The amount exceeds the HOST's send ceiling (`walletSendCeilingZatProvider`
/// — e.g. an alpha roll-out cap). Carries the ceiling so the inline copy can
/// state the limit; refused BEFORE any compose/propose bridge call. Distinct
/// from [ZecAmountFault.outOfRange] (the protocol supply bound): this is host
/// POLICY, and the copy must say so honestly rather than imply an invalid
/// amount.
class SendOverCeiling extends SendFormFault {
  const SendOverCeiling(this.ceilingZat);

  final int ceilingZat;
}

/// Not enough spendable value. Carries the audited note-selector's figures (all
/// zatoshis) so the host can say "you have X spendable, need Z, Y still
/// arriving". Never logged (§5.4) — the static `message` is.
class SendInsufficientFunds extends SendFormFault {
  const SendInsufficientFunds({
    required this.availableZat,
    required this.requiredZat,
    required this.pendingIncomingZat,
  });

  final int availableZat;
  final int requiredZat;
  final int pendingIncomingZat;
}

/// A categorical fault with no payload — rendered from [reason].
class SendCategoricalFault extends SendFormFault {
  const SendCategoricalFault(this.reason);

  final SendFaultReason reason;
}

/// The server will not say which network it is on and the wallet's grace for
/// such a server has run out — `RW-SYNC-003`, GRACE-1 (§4p). Carries the SDK's
/// reason (blocks / the device clock / never confirmed) and, for the blocks
/// sentence, the count it names. Its OWN class, not a [SendFaultReason]: the
/// copy needs the payload, and it must never fold into
/// [SendFaultReason.networkUpgradeUnsupported] — nothing was upgraded, an
/// update fixes nothing, and the next step is "switch servers" (or "check the
/// device's date and time"). Never logged; the static `message` is.
class SendServerSilentFault extends SendFormFault {
  const SendServerSilentFault({required this.by, this.blocksSinceLastCurrent});

  final GraceExpiry by;
  final int? blocksSinceLastCurrent;
}

/// The payload-free form-fault categories (the honest message axis; no codes).
enum SendFaultReason {
  /// The address didn't parse for this network.
  addressInvalid,

  /// A memo was given but the recipient can't receive one (transparent).
  memoToTransparent,

  /// The memo exceeds its ZIP-302 length bound.
  memoTooLong,

  /// The memo is otherwise un-sendable (reserved framing / bad bytes).
  memoNotSendable,

  /// TWO memos were attached to one payment — a text memo AND host machine
  /// bytes. **A programming error in the app, not something the user did or
  /// can fix**, which is why it does not share [memoNotSendable]'s "remove it
  /// and try again" copy: neither memo is individually invalid, and telling
  /// the user their memo is corrupt is wrong for both audiences (
  /// post-build review; it rode `MemoInvalid`/RW-PAY-006 until an earlier revision).
  memoConflict,

  /// The composed amount is out of range per the SDK (post-encode).
  amountOutOfRange,

  /// The address is for a different network than this wallet.
  networkMismatch,

  /// The composed payment URI was rejected (oversized/malformed).
  uriInvalid,

  /// The wallet isn't synced far enough to anchor a proposal yet — wait for
  /// sync, or queue the send for later. The offline-first fork's trigger.
  notSyncedYet,

  /// The Zcash network was upgraded and this app version can no longer build a
  /// transaction the network will accept (`RW-SYNC-002`). NOT a sync problem
  /// and NOT the user's doing: waiting changes nothing, only an app update
  /// does. Distinct from [notSyncedYet] for exactly that reason — telling
  /// someone to wait for a sync that will never help is the silence this
  /// whole feature exists to end.
  networkUpgradeUnsupported,

  /// A REVIEWED proposal's anchor went stale between confirm and send — the TTL
  /// ran out while the user deliberated (the large-amount confirm dialog widens
  /// this window). Distinct from [notSyncedYet]: the wallet IS synced; the
  /// NUMBERS expired, so the honest action is "review the payment again", not
  /// "wait for sync / queue". (Only the SEND path maps here; on the propose path
  /// the same kind means "can't anchor yet" → [notSyncedYet].)
  amountsExpired,

  /// The durable offline queue is at capacity — let it drain, then retry.
  queueFull,

  /// The wallet is mid-lifecycle (closing/repairing), or its store lost a
  /// write race to a concurrent sync commit past the SDK's own bounded retry
  /// (storeBusy — transient, nothing was written) — retry in a moment.
  walletBusy,

  /// The device is out of disk space, so persisting the send hit `DiskFull`
  /// (#373 — the send sibling of the rescan `needsSpace` / onboarding
  /// `storageFull` cue). Retrying WITHOUT freeing space deterministically
  /// re-fails, so the copy asks for space instead of "try again"; routed BACK
  /// TO THE FORM (orange-transient, funds untouched — nothing was written or
  /// broadcast), never the red "couldn't complete" dead-end that would loop.
  storageFull,

  /// Too many one-time (ephemeral) addresses are still in flight to start another
  /// multi-step (TEX) send — the engine gap-limit ceiling. TRANSIENT: a mined
  /// forward frees a slot. (2e-2b-v-5a: LIVE — the create/sign path reserves an
  /// ephemeral for every two-step TEX now that the slice-A gates are off.)
  oneTimeAddressLimit,

  /// Couldn't prepare the payment for a reason with no finer mapping that is
  /// DETERMINISTIC on the input — retrying unchanged re-fails, so the copy asks
  /// the user to check the details. The retryable class is
  /// [couldNotPrepareTransient] (INC-018 (b)).
  couldNotPrepare,

  /// Couldn't prepare the payment because of a condition the wallet's OWN state
  /// clears without the user changing anything — a note whose witness the scan
  /// has not completed, an anchor not yet recorded, an input a concurrent
  /// proposal holds (`WalletErrorKind.proposeTransient`, INC-018 (b), phase-2
  /// P2-2). The copy says "try again in a moment" and NEVER "check the
  /// details": on the device proof the details were correct and the
  /// identical send proposed fine two minutes later. TRANSIENT (orange).
  couldNotPrepareTransient,

  /// No live wallet session (defensive — the screen is gated to an active
  /// wallet, so this is a "go back and try again", never expected).
  walletUnavailable,

  /// This wallet is WATCH-ONLY — it holds a viewing key but no spending keys,
  /// so it can never sign a send (#397 §3.7 D3, RW-VIEW-001). DEFENSE-IN-DEPTH:
  /// the wallet-screen chrome hides Send and the send screen re-gates on the
  /// watch-only kind, so this is unreachable on the normal path; the arm exists
  /// so a deep-link/race that DID reach propose/send surfaces the honest "this
  /// wallet can't send" rather than the generic couldNotPrepare (security-N2).
  /// Not fixable on the form — the copy states the permanent fact.
  watchOnly,
}

/// Map a `propose`/`encodePaymentUri` failure to a [SendFormFault]. Reads the
/// typed `WalletApiError.kind` ONLY (never a payload — §5.4); a non-FRB error is
/// the generic [SendFaultReason.couldNotPrepare]. Pure + total, so it is
/// unit-tested at its boundary without a device.
SendFormFault classifyProposeFailure(Object error) {
  if (error is WalletApiError) {
    return switch (error.kind) {
      WalletErrorKind_InsufficientFunds(
        :final availableZat,
        :final requiredZat,
        :final pendingIncomingZat,
      ) =>
        SendInsufficientFunds(
          availableZat: availableZat,
          requiredZat: requiredZat,
          pendingIncomingZat: pendingIncomingZat,
        ),
      WalletErrorKind_AddressInvalid() => const SendCategoricalFault(
        SendFaultReason.addressInvalid,
      ),
      WalletErrorKind_MemoRequiresShieldedRecipient() =>
        const SendCategoricalFault(SendFaultReason.memoToTransparent),
      WalletErrorKind_MemoTooLong() => const SendCategoricalFault(
        SendFaultReason.memoTooLong,
      ),
      WalletErrorKind_ReservedMemoNotSendable() ||
      WalletErrorKind_MemoInvalid() => const SendCategoricalFault(
        SendFaultReason.memoNotSendable,
      ),
      // Its OWN arm, not folded into memoNotSendable: a both-memos refusal is
      // a caller bug and its copy must not blame the user's memo.
      WalletErrorKind_MemoConflict() => const SendCategoricalFault(
        SendFaultReason.memoConflict,
      ),
      WalletErrorKind_AmountOutOfRange() => const SendCategoricalFault(
        SendFaultReason.amountOutOfRange,
      ),
      WalletErrorKind_NetworkMismatch() => const SendCategoricalFault(
        SendFaultReason.networkMismatch,
      ),
      // A WATCH-ONLY wallet refuses every spend at the SDK entry (RW-VIEW-001).
      // Defense-in-depth (security-N2): the chrome hides Send and the send
      // screen re-gates on the watch-only kind, so this is unreachable on the
      // normal path — but a deep-link / race that DID reach propose gets the
      // honest "this wallet can't send", never the generic couldNotPrepare.
      WalletErrorKind_WatchOnly() => const SendCategoricalFault(
        SendFaultReason.watchOnly,
      ),
      WalletErrorKind_PaymentUriInvalid() => const SendCategoricalFault(
        SendFaultReason.uriInvalid,
      ),
      // A not-yet-anchorable wallet (TTL/sync) is the offline-first fork's
      // trigger — the host offers "queue to send later".
      WalletErrorKind_ProposalStale() => const SendCategoricalFault(
        SendFaultReason.notSyncedYet,
      ),
      // Ironwood/NU6.3: the network runs consensus rules this version cannot
      // build for. Its OWN arm — folding it into notSyncedYet would offer
      // "queue to send later" over a send that cannot go on this version at
      // all, which is worse than an error: it invites the user to wait.
      WalletErrorKind_NetworkUpgradeUnsupported() => const SendCategoricalFault(
        SendFaultReason.networkUpgradeUnsupported,
      ),
      // GRACE-1 (§4p G-1/G-6): the server will not say which network it is on
      // and the grace ran out — its OWN fault, with the SDK's reason, so the
      // copy says "switch servers" (or "check the device's date and time"),
      // never the update-the-app sentence above.
      WalletErrorKind_ConsensusGraceExpired(
        :final by,
        :final blocksSinceLastCurrent,
      ) =>
        SendServerSilentFault(
          by: by,
          blocksSinceLastCurrent: blocksSinceLastCurrent,
        ),
      // And "never checked" is the not-synced-yet story (§4p item 2): the first
      // completed pass resolves it, and the queue is a fine answer meanwhile.
      WalletErrorKind_ConsensusNotEvaluated() => const SendCategoricalFault(
        SendFaultReason.notSyncedYet,
      ),
      // storeBusy joins the lifecycle-busy arm (W-swap-4-a-4): both are honest
      // "try again in a moment" transients — the store one means a write lost
      // its race to a sync commit even past the SDK's in-Rust bounded retry;
      // nothing was written, so retrying IS the remedy (never the recovery
      // journey a storeCorrupt would earn).
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_InvalidState() ||
      WalletErrorKind_StoreBusy() => const SendCategoricalFault(
        SendFaultReason.walletBusy,
      ),
      // Out of disk persisting the compose (#373): the honest "free up space",
      // not a generic "couldn't prepare" that retries into a deterministic
      // re-fail on a tight disk. Nothing was written — funds untouched.
      WalletErrorKind_DiskFull() => const SendCategoricalFault(
        SendFaultReason.storageFull,
      ),
      // A zero-valued transparent output is structurally unreachable from the form
      // (the host blocks a non-positive amount before compose). Map it EXPLICITLY
      // to the honest generic rather than silently to the wildcard — a future
      // QR/URI path that composed one would still get a true "couldn't prepare",
      // never a wrong "larger than total supply". (A dedicated reason can land
      // with the QR path.)
      WalletErrorKind_ZeroValuedTransparentOutput() =>
        const SendCategoricalFault(SendFaultReason.couldNotPrepare),
      // INC-018 (b), phase-2 P2-2: the retryable class the SDK now types apart —
      // its OWN arm, so the copy says "try again in a moment" and never sends the
      // user to correct details that are correct. Its
      // sibling `proposeFailed` stays on the generic arm below: that class is
      // deterministic on the input, and "check the details" is its honest copy.
      WalletErrorKind_ProposeTransient() => const SendCategoricalFault(
        SendFaultReason.couldNotPrepareTransient,
      ),
      // sendAmountRequired never reaches here (the form always sets an amount);
      // proposeFailed / anything else → generic.
      _ => const SendCategoricalFault(SendFaultReason.couldNotPrepare),
    };
  }
  // The controller's own bound ran out (`walletFfiWedgeTimeout`). Propose is
  // local work that queues behind the scan's db lock, so on a wallet catching
  // up this is the WAIT, not the input (the Seeker walk saw a 12–14 s
  // first address behind the same lock). "Check the details" would send the
  // user to correct details that are correct — the INC-018 (b) mistake.
  if (error is TimeoutException) {
    return const SendCategoricalFault(SendFaultReason.couldNotPrepareTransient);
  }
  return const SendCategoricalFault(SendFaultReason.couldNotPrepare);
}

/// Map a `queueSend` failure to a [SendFormFault]. Same `kind`-only discipline.
/// `queuedSendsFull` is the one queue-specific kind; everything else mirrors
/// [classifyProposeFailure]'s categorical mapping (compose-time errors are
/// identical, since queue composes the same URI).
SendFormFault classifyQueueFailure(Object error) {
  if (error is WalletApiError &&
      error.kind is WalletErrorKind_QueuedSendsFull) {
    return const SendCategoricalFault(SendFaultReason.queueFull);
  }
  return classifyProposeFailure(error);
}

// ---------------------------------------------------------------------------
// Live recipient classification (inc-2d-ui slice 2)
// ---------------------------------------------------------------------------

/// The LIVE classification of the recipient field — the §5.1 privacy axis
/// (shielded/private vs transparent/public) plus the validity the form gates the
/// Review action on. A sealed family so the screen renders each state
/// exhaustively (no default blind spot). Computed SYNCHRONOUSLY from
/// [WalletSession.validateRecipient] (local, no network — feedback even fully
/// offline) on every keystroke; carries NO money state (design invariant 1).
sealed class RecipientStatus {
  const RecipientStatus();

  /// Whether a send may proceed to Review with this recipient. Empty / invalid /
  /// wrong-network are NOT sendable (the form disables the action and the status
  /// line explains why); a valid shielded OR transparent recipient IS sendable
  /// (transparent is honest + public — warned at Review, never blocked).
  bool get isSendable =>
      this is RecipientShielded || this is RecipientTransparent;

  /// Whether the memo field is usable. Only a shielded recipient can receive a
  /// ZIP-302 memo; for a transparent recipient the form disables + explains it.
  /// Empty / invalid leave the memo neutral (enabled) — the user may still be
  /// mid-draft and shouldn't be blocked before there's even a valid recipient.
  bool get allowsMemo => this is! RecipientTransparent;
}

/// Field is blank — the neutral first state (no message, no premature warning).
class RecipientEmpty extends RecipientStatus {
  const RecipientEmpty();
}

/// A shielded recipient: PRIVATE and memo-capable ("Shielded · private").
class RecipientShielded extends RecipientStatus {
  const RecipientShielded();
}

/// A transparent / transparent-only recipient: PUBLIC on-chain and
/// memo-incapable ("Transparent · public"). Honest and allowed — the §5.1
/// de-shield disclosure warns at Review; the send is never blocked for it.
class RecipientTransparent extends RecipientStatus {
  const RecipientTransparent();
}

/// The address doesn't parse for this network — not sendable.
class RecipientInvalid extends RecipientStatus {
  const RecipientInvalid();
}

/// Well-formed but for a DIFFERENT network (e.g. a testnet address pasted into a
/// mainnet wallet) — a distinct, renderable state (not "malformed").
class RecipientWrongNetwork extends RecipientStatus {
  const RecipientWrongNetwork();
}

/// Classify the recipient field's current text against the wallet's own network.
/// PURE over the (sync, local) [WalletSession.validateRecipient] seam — reads
/// only `memoCapable` or the typed `WalletApiError.kind` (NEVER an address
/// payload — §5.4), so it is unit-tested at its boundary behind a fake. A blank
/// field is [RecipientEmpty] (no bridge call); any non-FRB error degrades to
/// [RecipientInvalid] (honest, never a raw code — invariant 6).
RecipientStatus classifyRecipient(WalletSession session, String raw) {
  final address = raw.trim();
  if (address.isEmpty) return const RecipientEmpty();
  try {
    final v = session.validateRecipient(address);
    // memoCapable ⟺ a shielded receiver is present ⟺ private. A transparent /
    // transparent-only address is !memoCapable ⇒ classified PUBLIC here, never
    // mislabelled off a precise kind a normal user wouldn't understand.
    return v.memoCapable
        ? const RecipientShielded()
        : const RecipientTransparent();
  } on WalletApiError catch (e) {
    return e.kind is WalletErrorKind_NetworkMismatch
        ? const RecipientWrongNetwork()
        : const RecipientInvalid();
  } catch (_) {
    return const RecipientInvalid();
  }
}

/// Map a `send` (sign+broadcast) failure to the next [SendState]. Distinct from
/// the form mappers: a send failure is terminal-ish, so it routes to either the
/// result screen (already-submitted / sign-failed) or back to the form
/// (stale ⇒ re-propose, busy ⇒ retry). Reads `kind` only (§5.4).
SendState classifySendFailure(Object error) {
  if (error is WalletApiError) {
    return switch (error.kind) {
      // The token was consumed (double-tap / re-entry): the notes are never
      // broadcast twice. Honest "already submitted", not an error.
      WalletErrorKind_ProposalAlreadyUsed() => const SendSent(
        SendAlreadySubmitted(),
      ),
      // The anchor went stale between confirm and send (a long pause — the
      // large-amount dialog widens it): the reviewed NUMBERS expired, so the
      // host re-proposes for fresh figures, never retries the same token. Honest
      // "amounts expired, review again" — NOT the propose-path "not synced" (the
      // wallet IS synced here; only the proposal's anchor aged out).
      WalletErrorKind_ProposalStale() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.amountsExpired),
      ),
      // The consensus verdict is re-evaluated on EVERY sync pass, so it can flip
      // Current → Unsupported between propose (which passed) and the user
      // tapping Confirm (which signs). Without this arm that lands in the
      // wildcard below as a generic `SendSignFailed` — "couldn't complete this
      // payment", with a Retry that cannot work — instead of the honest "this
      // version needs an update". Nothing was signed or broadcast, so it
      // returns to the FORM with the same fault the propose path uses
      // (code reviewer).
      WalletErrorKind_NetworkUpgradeUnsupported() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.networkUpgradeUnsupported),
      ),
      // GRACE-1: the grace can run out between propose and Confirm exactly as
      // the verdict above can flip — same return to the FORM, its own fault.
      WalletErrorKind_ConsensusGraceExpired(
        :final by,
        :final blocksSinceLastCurrent,
      ) =>
        SendForm(
          fault: SendServerSilentFault(
            by: by,
            blocksSinceLastCurrent: blocksSinceLastCurrent,
          ),
        ),
      WalletErrorKind_ConsensusNotEvaluated() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.notSyncedYet),
      ),
      // storeBusy joins this arm: the interactive two-step
      // enrol writes the aux outbox inside the send bracket, and a store write
      // that lost its race to a sync commit (past the SDK's in-Rust bounded
      // retry) wrote NOTHING — the orange "busy, try again in a moment" back to
      // the form is honest; the red "couldn't complete" dead-end is not.
      // (Structurally near-unreachable today — the enrol holds both the db and
      // aux guards — but the taxonomy must not depend on that invariant.)
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_InvalidState() ||
      WalletErrorKind_StoreBusy() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.walletBusy),
      ),
      // A watch-only wallet can never sign (RW-VIEW-001): defense-in-depth back
      // to the form with the honest permanent fact (the chrome + screen gate
      // make this unreachable on the normal path). Routed to the form (not the
      // result screen) so the inline fault states why, matching the propose arm.
      WalletErrorKind_WatchOnly() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.watchOnly),
      ),
      // The durable outbox is full when an interactive multi-step (TEX) send tries
      // to enrol its forwarding leg: honest "outbox full — let it drain, then
      // retry", routed BACK TO THE FORM (orange-transient), NEVER the red
      // "couldn't complete; nothing was sent" dead-end (a re-tap loop, most
      // reachable on an unstable mobile link where the drain is stalled — the
      // outbox is fullest exactly then). The SEND path reaches this via the
      // two-step enrol, LIVE since gate-removal (2e-2b-v-5a). (The QUEUE path's
      // `classifyQueueFailure` already maps this kind to the same reason.)
      WalletErrorKind_QueuedSendsFull() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.queueFull),
      ),
      // The one-time-address ceiling (dual-natured, #315): a mined forward frees
      // its slot, but a slot used by a send that never confirmed does NOT free by
      // waiting — the routed copy holds both ("some may free up as transfers
      // confirm, but this may not clear on its own; your funds are safe"), back to
      // the form as orange, never the red dead-end. LIVE since gate-removal
      // (2e-2b-v-5a): the create/sign path reserves an ephemeral for every
      // two-step TEX now that the gates are off.
      WalletErrorKind_TexSendLimitReached() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.oneTimeAddressLimit),
      ),
      // Out of disk persisting the created tx before it escaped (#373): nothing
      // was broadcast (§6.3 — the notes are re-proposable), so route BACK TO THE
      // FORM with the honest "free up space" transient, never the terminal
      // "couldn't complete" that would loop on a full disk.
      WalletErrorKind_DiskFull() => const SendForm(
        fault: SendCategoricalFault(SendFaultReason.storageFull),
      ),
      // signFailed (or anything else): no money moved; the consumed token can't
      // be re-sent, so the result offers "try again" → a fresh form.
      _ => const SendSent(SendSignFailed()),
    };
  }
  return const SendSent(SendSignFailed());
}

/// Whether [error], thrown by the SDK's own `send` AFTER the spend closure was
/// entered, is a kind the core raises only BEFORE any transaction is persisted
/// (S7 U1 fold) — so [classifySendFailure]'s "nothing was sent" / back-to-form
/// routing is still the truth. Anything else lands on [SendOutcomeUnknown].
///
/// Each kind was checked against the core's send path (`Wallet::send_by_id` →
/// `sign_proposal_upstream` → `broadcast_persisted`):
/// - `WatchOnly` — `require_spend_capable`, before the token is touched.
/// - `WalletBusy` — `require_open` at the top of `sign_proposal_upstream`.
/// - `NetworkUpgradeUnsupported`, `ConsensusGraceExpired`,
///   `ConsensusNotEvaluated` — `signing_permit`, before the seed pull.
/// - `ProposalAlreadyUsed`, `ProposalStale` — the registry peek/consume and
///   `assert_proposal_anchor_fresh`, before the create; `ProposalStale` is also
///   `map_create_err`'s anchor arm, a create that rolled back.
/// - `SeedRequired`, `SeedMismatch` — `acquire_seed` and
///   `ensure_seed_controls_account`, before the token is consumed.
/// - `QueuedSendsFull` — the two-step enrol's `enqueue`, before the create.
/// - `TexSendLimitReached`, `SignFailed` — `map_create_err` only, a create the
///   engine rolled back whole.
///
/// NOT listed, on purpose: `StoreCorrupt` (`raw_tx_bytes` in
/// `broadcast_persisted` reads the just-persisted bytes; the two-step's
/// `mark_sent_multi` runs after the create committed), and `DiskFull`,
/// `StoreBusy`, `Io` — the same `mark_sent_multi` write classifies a SQLite
/// fault into any of them after the transactions exist. `InvalidState` is
/// `broadcast_persisted`'s answer for a wallet wiped mid-send, also after.
bool sendErrorPrecedesPersistence(Object error) =>
    error is WalletApiError &&
    switch (error.kind) {
      WalletErrorKind_WatchOnly() ||
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_NetworkUpgradeUnsupported() ||
      WalletErrorKind_ConsensusGraceExpired() ||
      WalletErrorKind_ConsensusNotEvaluated() ||
      WalletErrorKind_ProposalAlreadyUsed() ||
      WalletErrorKind_ProposalStale() ||
      WalletErrorKind_SeedRequired() ||
      WalletErrorKind_SeedMismatch() ||
      WalletErrorKind_QueuedSendsFull() ||
      WalletErrorKind_TexSendLimitReached() ||
      WalletErrorKind_SignFailed() => true,
      _ => false,
    };

/// The queue twin of [sendErrorPrecedesPersistence], for `queueSend`
/// (`Wallet::queue_send_uri` → `queue_send` → `intent_store::enqueue`):
/// `WalletBusy` and `WatchOnly` are the entry gates, and `QueuedSendsFull` is
/// the cap checked inside the enqueue transaction BEFORE its insert. Every
/// SQLite-classified kind is left out: this layer cannot tell a fault before
/// the insert from one at the commit.
bool queueErrorPrecedesPersistence(Object error) =>
    error is WalletApiError &&
    switch (error.kind) {
      WalletErrorKind_WatchOnly() ||
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_QueuedSendsFull() => true,
      _ => false,
    };

/// The SINGLE-STEP twin of [sendErrorPrecedesPersistence], for the shield and
/// move-to-transparent `send` (R13 §4.1). Same path (`Wallet::send_by_id` →
/// `sign_proposal_upstream` → `create_signed_core` → `broadcast_persisted`),
/// but never a two-step: a shield is its own arm and a move pays the wallet's
/// own t-address, never a TEX — so `mark_sent_multi` is unreachable, and the
/// engine writes the transaction ONCE, in one transaction, on full success.
///
/// Listed, by the site that raises each:
/// - `WatchOnly` — `require_spend_capable`, before the token is touched.
/// - `WalletBusy` — `require_open` at the top of `sign_proposal_upstream`.
/// - `NetworkUpgradeUnsupported`, `ConsensusGraceExpired`,
///   `ConsensusNotEvaluated` — `signing_permit`, before the seed pull.
/// - `ProposalAlreadyUsed`, `ProposalStale` — the registry peek/consume and
///   `assert_proposal_anchor_fresh`, before the create; `ProposalStale` is also
///   `map_create_err`'s anchor arm, a create that rolled back.
/// - `SeedRequired`, `SeedMismatch` — `acquire_seed` and
///   `ensure_seed_controls_account`, before the token is consumed.
/// - `InvalidSeedLength`, `KeyDerivation` — the spending-key derivation from
///   the seed (`derivation.rs`), before anything is built.
/// - `ProposeFailed` — `create_signed_core`'s shape guard, before signing.
/// - `TexSendLimitReached`, `SignFailed` — `map_create_err` only, a create the
///   engine rolled back whole.
/// - `DiskFull`, `StoreBusy`, `Io` — the persist itself (a failed COMMIT,
///   `map_create_err` → `into_store_fault`): the engine writes once, so a failed
///   commit saved nothing. After the create, `broadcast_persisted` raises none
///   of them — its `raw_tx_bytes` read stays an unclassified `StoreCorrupt`,
///   pinned by the floor in `no_blind_store_corrupt.rs`. Keeping them here keeps
///   the "free up space" and "busy" routes (#373), which are true on this path.
/// - `InvalidState` with `phase == closing` ONLY — the FFI's `handle_closed`, a
///   handle closed before the core was reached. Written as `== closing`, not
///   "not wiped", so a future post-persistence site with another phase fails
///   closed.
///
/// NOT listed, on purpose: `StoreCorrupt` (`raw_tx_bytes` reads the
/// just-persisted bytes), `InvalidState` with any other phase (`wiped` is
/// `broadcast_persisted`'s answer for a wallet wiped mid-send, after the
/// persist), and `QueuedSendsFull` (only the two-step enrol raises it; this
/// path cannot, so the list does not claim it). An untyped throw is never
/// listed. Whatever is not here lands on the flow's "outcome unknown" state.
bool singleStepSendErrorPrecedesPersistence(Object error) =>
    error is WalletApiError &&
    switch (error.kind) {
      WalletErrorKind_WatchOnly() ||
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_NetworkUpgradeUnsupported() ||
      WalletErrorKind_ConsensusGraceExpired() ||
      WalletErrorKind_ConsensusNotEvaluated() ||
      WalletErrorKind_ProposalAlreadyUsed() ||
      WalletErrorKind_ProposalStale() ||
      WalletErrorKind_SeedRequired() ||
      WalletErrorKind_SeedMismatch() ||
      WalletErrorKind_InvalidSeedLength() ||
      WalletErrorKind_KeyDerivation() ||
      WalletErrorKind_ProposeFailed() ||
      WalletErrorKind_TexSendLimitReached() ||
      WalletErrorKind_SignFailed() ||
      WalletErrorKind_DiskFull() ||
      WalletErrorKind_StoreBusy() ||
      WalletErrorKind_Io() => true,
      WalletErrorKind_InvalidState(:final phase) =>
        phase == LifecyclePhase.closing,
      _ => false,
    };

/// The parked "Send now" twin, for `authorizeParkedSend`
/// (`Wallet::try_sign_queued_intent` → `drain_multi`: create — the wallet-db
/// commit — then `mark_sent_multi`, then `read_raw_tx`) (R13 §4.1).
///
/// Listed, by the site that raises each:
/// - `WatchOnly`, `WalletBusy` — the entry gates, before the row is claimed.
/// - `InvalidState` (any phase) — only `require_open` and the FFI's
///   `handle_closed` raise it here, both before signing.
/// - `NetworkUpgradeUnsupported`, `ConsensusGraceExpired`,
///   `ConsensusNotEvaluated` — `signing_permit`, before the seed pull.
/// - `SeedRequired`, `SeedMismatch`, `InvalidSeedLength`, `KeyDerivation` —
///   the seed pull and the key derivation, before the create.
/// - `ProposeFailed` — the queued path's shape guard in `prepare_queued`,
///   before the create.
///
/// NOT listed, on purpose: `DiskFull`, `StoreBusy`, `Io`, `StoreCorrupt` —
/// every create error is swallowed into "still queued", so the store kinds that
/// surface come from `mark_submitting` (before), `reset_to_queued` (nothing
/// saved), but also `mark_sent_multi` and `read_raw_tx` (AFTER the commit); this
/// layer cannot tell which. An untyped throw is never listed. Whatever is not
/// here takes the parked row re-read, never "unchanged" on faith.
bool parkedAuthorizeErrorPrecedesPersistence(Object error) =>
    error is WalletApiError &&
    switch (error.kind) {
      WalletErrorKind_WatchOnly() ||
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_InvalidState() ||
      WalletErrorKind_NetworkUpgradeUnsupported() ||
      WalletErrorKind_ConsensusGraceExpired() ||
      WalletErrorKind_ConsensusNotEvaluated() ||
      WalletErrorKind_SeedRequired() ||
      WalletErrorKind_SeedMismatch() ||
      WalletErrorKind_InvalidSeedLength() ||
      WalletErrorKind_KeyDerivation() ||
      WalletErrorKind_ProposeFailed() => true,
      _ => false,
    };
