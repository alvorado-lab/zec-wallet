import 'dart:async';

import 'package:zec_wallet/zec_wallet.dart';

import '../send/zec_amount.dart';
import 'swap_assets.dart';

/// The swap flow's states (D-2b-1 / §3.3b), a sealed family so the screen renders
/// each phase with an exhaustive `switch` (no default blind spot). The flow is
/// form → (quote) → review → (execute) → [deposit] → tracking. Rendering layer
/// ONLY — every transition runs through [SwapController]; no money/swap state is
/// stored here (design invariant 1; Rust is the single source of truth). The DTOs
/// it carries ([SwapQuote]) are display projections — the durable single-flight
/// claim, the deposit, and the kill door all live Rust-side.
///
/// BOTH directions (§3.3b L8): OutOfZec (ZEC → a foreign asset; exact ZEC in, the
/// wallet sends the §4.4 deposit, then tracks) and IntoZec (a foreign asset → ZEC;
/// exact foreign in, the USER sends the deposit externally — the
/// [SwapAwaitingDeposit] D7 screen — then tracks). The default landing is IntoZec
/// (buy ZEC — a privacy wallet's default).
sealed class SwapFlowState {
  const SwapFlowState();
}

/// Which direction the form is composing (host UI concept — the bridge
/// `SwapDirection` additionally carries the chosen asset, which the form has only
/// once the user picks one).
enum SwapFlowDirection {
  /// other asset → ZEC. The user sends the deposit externally; delivery lands on
  /// a fresh wallet address and is nudged to shield.
  intoZec,

  /// ZEC → other asset. The wallet sends the §4.4 de-shielding deposit.
  outOfZec,
}

/// A direction-parameterised quote request from the form (§3.3b L8 — one shared
/// form, two shapes). A sealed pair so [SwapController.quote] dispatches with an
/// exhaustive `switch` and neither shape carries the other's nullable fields.
sealed class SwapFormInput {
  const SwapFormInput({required this.amountText, required this.slippageBps});

  /// The exact-IN amount the user typed (ZEC decimal for OutOfZec; a FOREIGN
  /// decimal string for IntoZec — validated Rust-side, NEVER parsed to zatoshis
  /// host-side, §3.3b L8).
  final String amountText;

  /// The user-tuned slippage tolerance (§3.3b D4), basis points. The SDK enforces
  /// the hard ceiling (`SLIPPAGE_MAX_BPS`); the form requests a safe value.
  final int slippageBps;
}

/// OutOfZec: ZEC → [asset], with the user's foreign receive [destination].
class OutOfZecInput extends SwapFormInput {
  const OutOfZecInput({
    required this.asset,
    required super.amountText,
    required this.destination,
    required super.slippageBps,
  });

  /// The curated foreign TARGET asset.
  final SwapAsset asset;

  /// The user's receive address on the destination chain (required).
  final String destination;
}

/// IntoZec: [token] → ZEC, with the user's source-chain [refundAddress].
class IntoZecInput extends SwapFormInput {
  const IntoZecInput({
    required this.token,
    required super.amountText,
    required this.refundAddress,
    required super.slippageBps,
  });

  /// The picked foreign SOURCE asset (from the dynamic D5 token list).
  final SwapAsset token;

  /// The user's refund address ON THE SOURCE CHAIN (required; where coins go back
  /// if the swap fails — §3.3b D6). Opaque, validated Rust-side at the boundary.
  final String refundAddress;
}

/// The editable swap form. [fault] is an inline, honest message from a failed
/// quote/execute (a bad destination, an expired quote, a provider hiccup) —
/// `null` on first entry.
class SwapFormState extends SwapFlowState {
  const SwapFormState({this.fault});

  final SwapFormFault? fault;
}

/// `swapQuote` in flight — a bounded provider round-trip, no money movement.
/// Transient; a spinner.
class SwapQuoting extends SwapFlowState {
  const SwapQuoting();
}

/// The review screen: the EXACT quote numbers + the §2.6 privacy disclosure the
/// user must acknowledge before any money moves, then [SwapController.execute]
/// registers the swap (and, for OutOfZec, queues the §4.4 deposit). Holds the
/// display [quote] (its durable single-flight is claimed by id on execute), the
/// [direction] (so review renders the right framing — de-shield warning for
/// OutOfZec, the positive ends-shielded line + refund verification for IntoZec),
/// the foreign [counter] asset, echoed back so the user verifies WHAT asset, and
/// for OutOfZec the [payoutAddress], so the user verifies WHERE it goes.
class SwapReview extends SwapFlowState {
  const SwapReview({
    required this.quote,
    required this.direction,
    required this.counter,
    this.payoutAddress,
    this.fault,
  });

  final SwapQuote quote;
  final SwapFlowDirection direction;

  /// OutOfZec: the exact foreign payout address this quote was requested for —
  /// where the provider sends [counter]. The quote DTO does not carry it, so the
  /// review keeps the request's own value and renders it in full for the user
  /// to verify before any money moves (N03, 2026-10-06 follow-up review). The
  /// SDK has already checked that the provider's echo names this same
  /// recipient: a quote whose echo differs is refused before a review exists
  /// (the 2026-10-07 review, finding 3). A
  /// §5.4 NEVER-log value. `null` for IntoZec, whose payout is the wallet's own
  /// SDK-minted address.
  final String? payoutAddress;

  /// The FOREIGN side of the swap (the OutOfZec target / the IntoZec source) —
  /// its display label + chain/symbol. The quote carries amounts + the deposit
  /// address, not the asset menu label; this echoes the asset back (and the
  /// deposit screen reads its chain/symbol for the "send … on …" instruction).
  final SwapAsset counter;

  /// An inline, RETRYABLE fault rendered ON the review (#367) — today only the
  /// store-busy execute miss, where the quote is UNCONSUMED and still valid so
  /// bouncing to the form (a forced re-quote) would needlessly burn it. `null`
  /// on a clean review.
  final SwapFormFault? fault;
}

/// IntoZec post-execute (§3.3b D7 / L7): the swap is registered, but unlike
/// OutOfZec the WALLET sends nothing — the USER must send the source coin to the
/// provider's deposit address. Holds the [swapId] (for the tracking hand-off),
/// the [quote] (deposit address + amount-in + deadline), and the source [counter]
/// asset (so the deposit screen says "send {amountIn} {symbol} on {chain}"). The
/// "I've sent the funds" affordance advances to [SwapExecuted] live tracking.
class SwapAwaitingDeposit extends SwapFlowState {
  const SwapAwaitingDeposit({
    required this.swapId,
    required this.quote,
    required this.counter,
  });

  final String swapId;
  final SwapQuote quote;
  final SwapAsset counter;
}

/// `swapExecute` in flight (claim the durable single-flight, register intent,
/// queue the deposit). Transient; a spinner. Holds the quote so a UI can keep
/// showing the figures.
class SwapExecuting extends SwapFlowState {
  const SwapExecuting(this.quote);

  final SwapQuote quote;
}

/// Terminal-of-the-form result: the swap is registered and the opaque
/// [swapId] minted. The screen hands off to `swapStatusProvider(swapId)` for the
/// live §7 tracking (and renders "tracking unavailable" from the host's own kill
/// state if swap is killed — §3.5; the stream has no wire signal for a kill).
class SwapExecuted extends SwapFlowState {
  const SwapExecuted(this.swapId, this.direction, {this.reattached = false});

  /// The provider swap id — opaque; do NOT parse it. It is ALSO a §5.4 NEVER-log
  /// value host-side (the SwapId is the provider deposit address); render it only
  /// as the tracking key, never log it.
  final String swapId;

  /// Carried so the killed-swap message is honest per direction (§3.3b L8): an
  /// OutOfZec kill says "tracking unavailable"; an IntoZec kill says "your funds
  /// will appear after your next sync" (the scoped poll detects the delivery).
  final SwapFlowDirection direction;

  /// True when this tracking state came from a durable-record RE-ATTACH
  /// ([SwapController.attachTo]) rather than a fresh execute in this process
  /// run (#367). Load-bearing for the IntoZec pending copy: a fresh execute's
  /// "send your funds before the quote expires" is followable (the D7 deposit
  /// screen showed the address/memo), but post-restart those instructions are
  /// deliberately NOT persisted — the re-attach copy must not instruct an
  /// impossible action (and must NOT re-render the address: the memo is gone,
  /// and an address-without-memo deposit is a memo-chain fund-loss door).
  final bool reattached;
}

// ---------------------------------------------------------------------------
// Form faults
// ---------------------------------------------------------------------------

/// A FIXABLE form fault surfaced inline so the user corrects the input (or
/// re-quotes) on the SAME form — never a full-screen failure that discards what
/// they typed. Reads only the typed `SwapApiError.kind` (or a host-side parse
/// fault) — never an error payload — so nothing sensitive (address/amount) leaks
/// (§5.4).
sealed class SwapFormFault {
  const SwapFormFault();
}

/// Host-side ZEC amount parse failure (before any bridge call) — see
/// [ZecAmountFault].
class SwapAmountFault extends SwapFormFault {
  const SwapAmountFault(this.fault);

  final ZecAmountFault fault;
}

/// A categorical fault with no payload — rendered from [reason].
class SwapCategoricalFault extends SwapFormFault {
  const SwapCategoricalFault(this.reason);

  final SwapFaultReason reason;
}

/// The OutOfZec deposit's ZEC side exceeds the HOST's send ceiling
/// (`walletSendCeilingZatProvider` — e.g. an alpha roll-out cap; maintainer
/// 2026-07-10: the ceiling BOUNDS the swap deposit, so every alpha money-out
/// path stays under one cap). Carries the ceiling so the inline copy states the
/// limit — the SAME typed over-ceiling fault the send form uses (FR-23). Refused
/// at quote-review, BEFORE the authorize-spend bracket. IntoZec never hits this
/// (no wallet funds leave — the user sends the deposit externally). Host POLICY,
/// not a protocol bound: the amount itself is valid.
class SwapOverCeiling extends SwapFormFault {
  const SwapOverCeiling(this.ceilingZat);

  final int ceilingZat;
}

/// The OutOfZec deposit (plus a conservative network-fee allowance) exceeds
/// what is SPENDABLE right now (#367 — the pre-check the execute chokepoint
/// never had: without it a catching-up 0-balance wallet could quote → ack →
/// execute, the sign-time InsufficientFunds is swallowed by contract, and the
/// UI landed "Swap started" over a deposit that could never fund). Refused at
/// quote-review, BEFORE the authorize-spend bracket — money-safe either way
/// (nothing leaves the pool); this makes the refusal HONEST and immediate.
/// Carries the amounts for the inline copy (render-only, §5.4 never-log) and
/// [catchingUp] so a mid-catch-up refusal hedges ("your balance may still be
/// catching up") instead of asserting a final verdict over a partial figure.
class SwapInsufficientSpendable extends SwapFormFault {
  const SwapInsufficientSpendable({
    required this.neededZat,
    required this.spendableZat,
    required this.catchingUp,
  });

  /// The quote's ZEC deposit side + the fee allowance.
  final int neededZat;
  final int spendableZat;
  final bool catchingUp;
}

/// The payload-free swap-fault categories (the honest message axis; no codes).
enum SwapFaultReason {
  /// The foreign destination address was empty (host-side; OutOfZec requires it).
  destinationRequired,

  /// The IntoZec source amount was empty (host-side; the foreign decimal is
  /// otherwise validated Rust-side — this is the only host-side amount check for
  /// IntoZec, since the foreign decimal is NOT parsed to zatoshis here, §3.3b L8).
  foreignAmountRequired,

  /// The IntoZec refund address was empty (host-side; IntoZec requires a
  /// source-chain refund target — §3.3b D6; the non-empty string is then
  /// validated at the Rust boundary, never parsed as a Zcash address).
  refundAddressRequired,

  /// The destination was rejected by the SDK (not a valid foreign address, a
  /// Zcash address — a ZEC→ZEC round-trip — or oversized).
  destinationInvalid,

  /// The quote is no longer executable — its deadline lapsed, or its durable
  /// single-flight row is gone (already executed / swept; the core types every
  /// take-miss this way since #367). Re-quote for fresh numbers, never sign
  /// past it.
  quoteExpired,

  /// The provider's quote fell outside the user-anchored bound — rejected BEFORE
  /// anything is signed (a deposit is never anchored to provider numbers alone).
  quoteOutOfBounds,

  /// The requested slippage exceeded the SDK's hard ceiling (defensive — the app
  /// requests a safe default).
  slippageTooHigh,

  /// The provider is unreachable / erroring — retryable; the wallet is unaffected.
  providerUnavailable,

  /// The swap request never reached the provider in time — a host-side network
  /// timeout ([kSwapNetworkTimeout]) or a broken connection. Distinct from
  /// [providerUnavailable] (the provider answered with an error): here the most
  /// likely cause is the user's own connectivity, so the message says so.
  connectionFailed,

  /// The IntoZec EXECUTE overran the host-side timeout (#367 — F5): the
  /// cause may be a slow connection OR a local stall (a busy store consuming
  /// most of the window), so the copy must not firmly blame the connection
  /// the way [connectionFailed] does. Money-safe either way: an IntoZec
  /// execute moves no wallet funds (registration happened at quote) —
  /// re-quote. (The OutOfZec execute timeout never lands here: money may be
  /// in motion, so that arm goes to TRACKING, not to a fault.)
  executeTimedOut,

  /// The provider response broke the protocol contract — typed, never trusted.
  providerMisbehaved,

  /// Swap is turned OFF at this instance (the host kill switch reached the SDK).
  /// Defensive — the surface is gated on the host swap-enabled state.
  swapOff,

  /// OUR side could not queue the §4.4 ZEC deposit after the provider registered
  /// the swap — no ZEC left the pool, so re-quote; the provider refunds an
  /// un-deposited quote after its deadline.
  depositFailed,

  /// A swap is ALREADY in progress — the SDK's one-deposit-in-flight guard
  /// refused a second deposit while the first is queued/signing/sending
  /// (W-swap-4-a-2). The remedy is the OPPOSITE of [depositFailed]: do NOT
  /// re-quote (that is the double-deposit door the guard closes) — the copy
  /// points at the in-progress swap and clears once it completes or its quote
  /// deadline lapses.
  swapInFlight,

  /// OUR side could not produce a fresh transparent refund address. The
  /// DOMINANT real cause since #368 is a pre-first-sync Sell: the refund mints
  /// through the engine, which needs the lazily-provisioned account, and the
  /// swap surface is activation-gated (not provision-gated) — so the honest
  /// remedy is "wait for the first sync to finish, then try again" (#382; the
  /// pre-#368 "stays so until reconstructed" framing described the rare
  /// structural-incapability tail, not the common path).
  refundAddressUnavailable,

  /// The IntoZec mirror of [refundAddressUnavailable] (#382): OUR side could
  /// not mint a fresh receiving address for the swap delivery — the same
  /// pre-first-sync dominant cause, the same wait-for-sync remedy (pre-#382
  /// this fell through to the generic [couldNotQuote]).
  destinationAddressUnavailable,

  /// OUR side could not maintain its durable in-flight swap state — fail-closed,
  /// nothing left the pool; re-quote.
  swapStateUnavailable,

  /// The wallet's durable swap store was momentarily BUSY at a site where
  /// NOTHING was consumed (#367 — the quote persist or the execute claim). The
  /// remedy is a plain RETRY of the same action: for an execute the quote is
  /// unconsumed and still valid (the review stays up with this fault inline);
  /// for a quote, quoting again simply works once the contending writer ends.
  storeBusy,

  /// The request was rejected as malformed / at the in-flight quote cap —
  /// re-quote. (A not-issued / already-executed miss is typed [quoteExpired]
  /// since #367.)
  requestInvalid,

  /// The quote passed to execute names a quote this wallet issued but its terms
  /// differ from the wallet's own record of it (stage S8, R01 — the SDK compares
  /// the DTO field-by-field against the durable record before the single-use
  /// claim). Nothing was consumed and nothing left the wallet; the quote the
  /// SDK returned is still executable, but this screen only ever holds that
  /// DTO, so reaching here means it was altered on the way — re-quote.
  quoteTermsDiffer,

  /// Couldn't get a quote / execute for a reason with no finer mapping.
  couldNotQuote,

  /// No live wallet session (defensive — the screen is gated to an active,
  /// swap-enabled wallet, so this is a "go back and try again", never expected).
  walletUnavailable,
}

/// Map a `swapQuote`/`swapExecute` failure to a [SwapFormFault]. Reads the typed
/// `SwapApiError.kind` ONLY (never a payload — §5.4); a non-FRB error is the
/// generic [SwapFaultReason.couldNotQuote]. Pure + total, so it is unit-tested at
/// its boundary without a device.
///
/// Quote AND execute share this mapper: every execute failure also routes back to
/// the FORM (re-quote), because the core claims the durable single-flight FIRST —
/// after a failed execute the quote may be consumed, so re-executing the same
/// quote would be `QuoteExpired` (#367); re-quoting is the only safe path. The
/// TWO exceptions the controller special-cases before this mapper's verdict:
/// [SwapFaultReason.swapInFlight] (never re-quote — the double-deposit door) and
/// [SwapFaultReason.storeBusy] at execute (the quote is UNCONSUMED — retry the
/// same review, don't burn it).
SwapFormFault classifySwapFailure(Object error) {
  if (error is SwapApiError) {
    return switch (error.kind) {
      SwapErrorKind_SlippageToleranceTooHigh() => const SwapCategoricalFault(
        SwapFaultReason.slippageTooHigh,
      ),
      SwapErrorKind_QuoteOutOfBounds() => const SwapCategoricalFault(
        SwapFaultReason.quoteOutOfBounds,
      ),
      SwapErrorKind_QuoteExpired() => const SwapCategoricalFault(
        SwapFaultReason.quoteExpired,
      ),
      SwapErrorKind_DestinationInvalid() => const SwapCategoricalFault(
        SwapFaultReason.destinationInvalid,
      ),
      SwapErrorKind_RequestInvalid() => const SwapCategoricalFault(
        SwapFaultReason.requestInvalid,
      ),
      SwapErrorKind_ProviderUnavailable() => const SwapCategoricalFault(
        SwapFaultReason.providerUnavailable,
      ),
      SwapErrorKind_ProviderProtocol() => const SwapCategoricalFault(
        SwapFaultReason.providerMisbehaved,
      ),
      SwapErrorKind_SwapDisabled() => const SwapCategoricalFault(
        SwapFaultReason.swapOff,
      ),
      SwapErrorKind_DepositSendFailed() => const SwapCategoricalFault(
        SwapFaultReason.depositFailed,
      ),
      // The one execute failure whose remedy is NOT re-quote: a deposit is
      // already in flight (the SDK's cross-quote guard) — tell the user to
      // check the in-progress swap, never to re-quote into a double.
      SwapErrorKind_SwapAlreadyInFlight() => const SwapCategoricalFault(
        SwapFaultReason.swapInFlight,
      ),
      SwapErrorKind_RefundAddressUnavailable() => const SwapCategoricalFault(
        SwapFaultReason.refundAddressUnavailable,
      ),
      // The IntoZec sibling (#382): its own wait-for-sync copy instead of the
      // generic could-not-quote fallthrough it rode pre-#382.
      SwapErrorKind_DestinationAddressUnavailable() =>
        const SwapCategoricalFault(
          SwapFaultReason.destinationAddressUnavailable,
        ),
      SwapErrorKind_SwapStateUnavailable() => const SwapCategoricalFault(
        SwapFaultReason.swapStateUnavailable,
      ),
      // The retryable busy (#367): nothing was consumed — "try again", not a
      // re-quote dead-end (the controller keeps the review up at execute).
      SwapErrorKind_SwapStateBusy() => const SwapCategoricalFault(
        SwapFaultReason.storeBusy,
      ),
      // #397 §3.7 D3: a watch-only wallet structurally cannot swap.
      // Defensively-unreachable here TODAY — the kind is produced only at
      // `enableNearSwap`, which the activation provider degrades to swap-off
      // before any swap screen exists, and the watch-only chrome hides the
      // surface — but if a future change ever surfaces it at quote/execute,
      // the honest verdict is "swap is off" (true, and permanent for this
      // wallet), NEVER the re-quote / "try again" framing of the generic
      // fallthrough below. If that future arrives, this arm's swapOff reuse
      // ("…right now") becomes a wrong-tense stopgap: promote it to a
      // dedicated permanent-framing reason (the walletSwapUnavailableWatchOnly
      // copy is the model) as part of that change, not after it.
      SwapErrorKind_WatchOnly() => const SwapCategoricalFault(
        SwapFaultReason.swapOff,
      ),
      // S8 (R01): the DTO's terms differ from the SDK's record — refused before
      // the claim, nothing consumed; the honest remedy is a fresh quote.
      SwapErrorKind_QuoteTermsDiffer() => const SwapCategoricalFault(
        SwapFaultReason.quoteTermsDiffer,
      ),
      // A kind this binding doesn't know — the stable code still rides
      // `error.code`; render the honest generic, never a wrong specific.
      _ => const SwapCategoricalFault(SwapFaultReason.couldNotQuote),
    };
  }
  // A host-side network timeout ([kSwapNetworkTimeout]) or a broken transport
  // never produced a typed SwapApiError — the request didn't reach 1Click. Show
  // the connectivity-focused message ("check your connection"), distinct from
  // the provider answering with an error.
  if (error is TimeoutException) {
    return const SwapCategoricalFault(SwapFaultReason.connectionFailed);
  }
  return const SwapCategoricalFault(SwapFaultReason.couldNotQuote);
}
