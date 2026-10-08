import 'wallet_send_request.dart';

/// FR-26 — what the package tells a host about the send flow the host's own
/// entry point opened ([WalletSendEntry.push]).
///
/// WHY THIS EXISTS. The entry's navigation future completes when the send
/// screen POPS, and a pop is true of the user who paid and of the user who
/// backed out alike — one signal covering both branches, committing to
/// neither. A host that composes a payment record on it can put a false claim
/// of payment in front of the payee. This family is the branch the pop refused
/// to take.
///
/// THE HOST DECIDES NOTHING ABOUT THE MONEY. Every variant is a statement about
/// what the WALLET did; the wallet's own surfaces (activity, the in-flight cue,
/// the balance) remain the authority on what happens next — a transaction that
/// did not reach the network is re-broadcast by the resubmission machinery
/// without the host's involvement.
///
/// PAYLOAD DISCIPLINE (§5.4). A report carries transaction identifiers, counts
/// and the host's own [correlationId] — never the recipient, the memo, or
/// anything else the user typed, and never the amount with ONE deliberate
/// exception: a TAGGED [WalletSendTransactionCreated] carries
/// [WalletSendTransactionCreated.recipientAmountZat] (FR-46) — the host already
/// knows the recipient it locked and asked for this figure; an untagged report
/// carries no amount at all. Nothing here is logged by the
/// package. **A txid is a §5.4 never-log item that this seam deliberately hands
/// to third-party code**, so a host holds it to the same bar: it is a payment
/// identifier, not a display string.
///
/// TWO LIMITS, STATED SO A HOST DOES NOT DISCOVER THEM IN PRODUCTION:
/// - **The report is PROCESS-LOCAL and cannot be restored.** It travels on the
///   route's `extra`, which does not survive Android process death: a restored
///   `/wallet/send` opens with no prefill and no channel, and the future the
///   host was awaiting dies with the process. A host that persists a pending
///   record must keep its own reconciliation path (the wallet's activity
///   surface, joined on the txid it may not yet have) rather than assuming this
///   future always arrives.
/// - **Only the FIRST payment of a flow is reported.** The channel is one-shot.
///   Once a report is delivered the package removes every affordance that could
///   spend again from that entry — "Send another" AND "Try again" — and a
///   terminal is only published for a flow that can no longer spend, so a
///   denied prompt or a refused sign leaves the channel open for the payment
///   the user actually makes. Still: a host must not treat one report as proof
///   that exactly one payment was ever made.
sealed class WalletSendReport {
  const WalletSendReport({required this.correlationId});

  /// The opaque token the host put on [WalletSendRequest.correlationId], echoed
  /// back verbatim. `null` when the host set none.
  ///
  /// The package never interprets, parses, renders, logs or transmits it — it
  /// exists so a host that routes reports into a shared sink (rather than
  /// awaiting each push at its call site) can join the report to the record it
  /// opened, without inventing a join on the recipient + amount or on the
  /// in-flight authorizer bracket. Both of those joins are wrong: the first is
  /// a display-fact join on a money surface, the second is free at one
  /// in-flight send and is not a proof at N.
  final String? correlationId;
}

/// NO TRANSACTION WAS CREATED — nothing was signed, nothing was broadcast,
/// nothing is queued, and no money moved.
///
/// The flow ended before it could mint one: the user left by any exit (system
/// back, the AppBar arrow, a Done on a form), declined at the host's own
/// authorization prompt, or the build/sign step refused — or the send screen
/// never appeared within the entry's grace (a host redirect held the route).
/// The honest host behaviour is to compose NO payment record — not a failed
/// one, not a pending one.
///
/// DELIVERED BY THE ENTRY'S MOUNT GRACE, THIS IS FINAL FOR THE REQUEST IT
/// NAMES (ADR-0557): the grace revokes the request before it reports, so a
/// screen that mounts late renders "this request expired" with no pay path,
/// and a host `replace`/`go` onto the send path with the same request (the
/// same `correlationId`, or the same fields) is refused. A host that wants the
/// payment after this issues a NEW request (a fresh `push` is a new grant).
/// The other producers of this report — a user who left, a declined
/// authorization, a refused build/sign — revoke nothing: the flow they ended
/// is still the user's to pay (FR-26 row 687).
final class WalletSendNoTransaction extends WalletSendReport {
  const WalletSendNoTransaction({super.correlationId});
}

/// A TRANSACTION EXISTS. This is the decidable fact, and it is deliberately not
/// "the broadcast succeeded".
///
/// Every transaction in [txids] was signed and PERSISTED by the wallet. Some of
/// them may not have reached the network yet ([broadcastCount] is how many the
/// endpoint accepted) — a broadcast failure is DATA, not an error, and the
/// wallet re-broadcasts on the next sync. A host that treats
/// `broadcastCount < txids.length` as "the payment failed" will be wrong about
/// money that is on its way.
final class WalletSendTransactionCreated extends WalletSendReport {
  WalletSendTransactionCreated({
    required this.txids,
    required this.broadcastCount,
    required this.motion,
    this.recipientAmountZat,
    super.correlationId,
  }) : assert(
         txids.isNotEmpty,
         'a transaction-exists report with no id is not this variant — it is '
         'WalletSendUnclassified (a host cannot reconcile a payment it cannot '
         'cite)',
       ),
       assert(
         broadcastCount >= 0 && broadcastCount <= txids.length,
         'broadcastCount counts a subset of txids',
       );

  /// The transaction ids, hex, in the order the wallet minted them. NEVER
  /// empty — a report with no txid is not this variant (see
  /// [WalletSendUnclassified]).
  ///
  /// More than one is normal: a payment that crosses pools mints a set (§1.7),
  /// and a two-step TEX payment (ZIP-320) mints a sequenced pair.
  final List<String> txids;

  /// How many of [txids] the endpoint accepted (`0..txids.length`). The rest are
  /// persisted and re-broadcast by the wallet; they are not lost.
  ///
  /// Do NOT read `broadcastCount == txids.length` as "all of it went out": a
  /// result set can contain an arm that carries no id at all (the
  /// forward-compatibility case), which contributes to neither number.
  final int broadcastCount;

  /// Whether funds have left the shielded pool onto a wallet-controlled
  /// one-time address without confirming at the recipient — see
  /// [WalletSendMotion], and note it is THREE-valued for a reason.
  final WalletSendMotion motion;

  /// FR-46: the TOTAL this payment pays its one recipient, in zatoshis, AS
  /// SIGNED — the fee excluded; two payments to the same address are summed.
  ///
  /// Set ONLY on a TAGGED report (a non-null [correlationId]); an untagged
  /// report never carries it. `null` on a tagged report when the payment has
  /// more than one recipient — never a partial sum (a unified address and one
  /// of its own receivers count as two).
  ///
  /// A statement about what was SIGNED, never about arrival: it is set whatever
  /// [motion] says, and for [WalletSendMotion.inMotion] the forwarding leg can
  /// still strand without this figure being revised. Gate delivery copy on
  /// [motion] and the wallet's own surfaces, never on this field's presence.
  final int? recipientAmountZat;
}

/// The in-motion axis of a two-step (ZIP-320) payment. Three-valued because the
/// truth is: the SDK can be sure it is in motion, sure it is not, or unable to
/// tell — and a host that reads a two-state answer will act on a certainty the
/// wallet does not have.
enum WalletSendMotion {
  /// An ordinary payment, or a two-step whose legs are all accounted for. No
  /// funds are parked on a one-time address mid-transfer.
  notInMotion,

  /// A two-step (ZIP-320) payment whose legs are PARTLY out: the funds have
  /// left the shielded pool onto a one-time address the wallet controls, and
  /// have not confirmed at the recipient.
  ///
  /// The honest host copy says only the permanently-true things — it is on its
  /// way, do not send it again, recover from the wallet if it does not
  /// complete. It must NOT promise the wallet will finish on its own: the
  /// forwarding leg can expire, and only the wallet's own recovery resolves
  /// that.
  inMotion,

  /// A two-step payment where NOTHING came back accepted. That is either
  /// "nothing went out" or "the wallet's own background re-broadcast landed
  /// both legs first and the reject codes are 'already known'" — and the wallet
  /// cannot tell which, because reject codes are backend-specific and
  /// deliberately uninterpreted.
  ///
  /// Treat it as [inMotion] for anything that could cause a second payment: a
  /// host offering "pay another way" here can double-pay a payee whose money is
  /// already travelling. The wallet's own in-flight cue owns the later truth.
  indeterminate,
}

/// DURABLY QUEUED, AND NO TRANSACTION EXISTS YET — the offline branch.
///
/// The user committed the payment while the wallet could not anchor one; the
/// intent is persisted and will be signed and broadcast on the next online
/// sync. A host MUST NOT render this as sent: there is no transaction and no
/// txid to cite. There is also no later report for it — the flow that produced
/// this report is over, and the wallet's own surfaces own the drain. What the
/// host gets instead of a later report is [queuedSendId], the key those
/// surfaces are keyed by.
final class WalletSendQueuedOffline extends WalletSendReport {
  const WalletSendQueuedOffline({this.queuedSendId, super.correlationId});

  /// The wallet's own id for the parked send — the same id the parked/re-stage
  /// surfaces are keyed by. It is the ONLY handle onto a payment that has no
  /// txid yet, so a host that persists a queued record persists this with it;
  /// without it the record can never be resolved against the wallet at all.
  ///
  /// `null` only if the flow reported queued without the id being available.
  final String? queuedSendId;
}

/// THE PAYMENT WAS ALREADY SUBMITTED — a re-entry (a double tap, a re-push over
/// a flow that had already signed). The wallet refused to spend the same notes
/// twice (§6.3).
///
/// This is NOT a second payment and must never be recorded as one. It is also
/// not an error: the earlier payment stands, and the host either already holds
/// its report or can see it on the wallet's own surfaces.
final class WalletSendAlreadySubmitted extends WalletSendReport {
  const WalletSendAlreadySubmitted({super.correlationId});
}

/// THE SEAM CANNOT GRADE THIS FLOW. Distinct from "nothing happened".
///
/// Reached when the user leaves while a step is still running (the sign +
/// broadcast can outlive the screen), or when the flow ended in a shape the
/// seam has no honest name for. Money MAY have moved; if it did, the wallet
/// holds it and its own surfaces show it.
///
/// The honest host behaviour is to record nothing as paid and nothing as
/// failed, and — if the host needs certainty — to let the user look at the
/// wallet. Silently treating this as "not paid" is the failure mode this
/// variant exists to prevent.
final class WalletSendUnclassified extends WalletSendReport {
  const WalletSendUnclassified({super.correlationId});
}
