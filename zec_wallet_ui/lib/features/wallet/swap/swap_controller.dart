import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'dart:async';

import '../send/zec_amount.dart';
import '../send_authorization.dart';
import '../wallet_providers.dart';
import '../wallet_rescan_controller.dart';
import '../wallet_session.dart';
import 'swap_assets.dart';
import 'swap_state.dart';
import 'swap_status_provider.dart';

/// Host-side backstop timeout for a swap network round-trip (quote/execute). The
/// SDK has its own transport timeouts; this guarantees the UI never hangs on
/// "Getting a quote…" / "Starting…" if a dial stalls on a dead/flaky network — it
/// surfaces the honest "couldn't get a quote, try again" fault (the receive-address
/// timeout precedent). Generous enough for a slow Tor path (a real quote can take
/// ~20s) yet bounded so the spinner can never spin forever.
const kSwapNetworkTimeout = Duration(seconds: 60);

/// Conservative NETWORK-FEE allowance the #367 spendable pre-check adds to the
/// OutOfZec deposit before comparing against what is spendable right now.
/// The exact ZIP-317 fee is knowable only at execute-sign (sign-at-execute —
/// there is no swap propose/preview round-trip), so review-time honesty uses a
/// bound: 25 000 zat = five ZIP-317 logical actions, comfortably above the
/// typical 2–4-action shielded→transparent deposit. Direction of error is
/// deliberate: too SMALL lets a doomed execute through to the engine's
/// swallowed sign-time InsufficientFunds (the invisible-parked blast
/// radius — now at least VISIBLE via the durable home, but still a dead end);
/// too large merely refuses a hairline swap the fee might genuinely kill.
/// A fragmented wallet (many dust inputs) can exceed any constant — the
/// engine-side gate stays the backstop; this makes the common case honest.
const kSwapDepositFeeAllowanceZat = 25000;

/// The swap flow state machine (D-2b-1 / §3.3b): form → quote → review → execute
/// → [deposit] → tracking. Drives the swap screen; Rust stays the single source
/// of truth (design invariant 1) — every step forwards straight to the SDK and no
/// swap state is cached here. BOTH directions ([SwapFlowDirection]): OutOfZec
/// (ZEC → a foreign asset; the wallet sends the §4.4 deposit) and IntoZec (a
/// foreign asset → ZEC; the USER sends the deposit externally, so execute lands on
/// the [SwapAwaitingDeposit] D7 screen before tracking).
///
/// Non-autoDispose (mirrors [sendControllerProvider]; Riverpod 3.x has no
/// `AutoDisposeNotifier`): a fresh entry resets via [resetToForm]. It reads the
/// live [WalletSession] through providers (cached in [build] for the actions),
/// exactly the seam the screen tests override.
final swapControllerProvider = NotifierProvider<SwapController, SwapFlowState>(
  SwapController.new,
);

class SwapController extends Notifier<SwapFlowState> {
  WalletSession? _session;

  /// A late async continuation touching `ref`/`state` after dispose throws;
  /// every post-await step re-checks this (the send/onboarding discipline).
  bool _disposed = false;

  /// The LIVE post-commitment state carried across same-identity rebuilds
  /// (W-swap-5, #366 — "stop wiping live tracking"): [_set] stamps it whenever
  /// the state is [SwapExecuted] / [SwapAwaitingDeposit] (money committed;
  /// pure render projections — no session-bound resources) and clears it on
  /// any other state. [build] returns it when the wallet IDENTITY is
  /// unchanged, so a rescan's session swap no longer resets a live tracking /
  /// deposit screen to a blank form mid-money. An IDENTITY change (delete →
  /// create, duress flip) still wipes — the #381 fence discipline: the old
  /// wallet's swap must never render into the new wallet's screen. SCOPE
  /// (arch review N5): retention covers ALREADY-LANDED committed states; a
  /// commit whose continuation lands DURING a same-identity session swap is
  /// still dropped by the #330 instance-identity guard (the pre-existing
  /// discipline) — the durable record then re-offers the swap from the home
  /// list, and for IntoZec the unpersisted deposit instructions are gone
  /// either way (the honest tracking arm renders).
  SwapFlowState? _liveAcrossRebuild;

  /// The wallet-life identity the CURRENT build cycle belongs to — captured
  /// in [build] and stamped onto [_liveAcrossRebuild] by [_set], so the fence
  /// holds by construction (arch review N1): a state can only ever be carried
  /// under the identity of the cycle that produced it, independent of
  /// scheduler ordering between a landing continuation and an identity flip.
  Object? _identity;
  Object? _liveIdentity;

  @override
  SwapFlowState build() {
    // A watched-dep change (the session flips) RE-RUNS build() on the SAME
    // notifier, firing the prior cycle's onDispose first → `_disposed` would
    // stick `true` and wedge every later `_set` (#330). Reset it each build;
    // the instance-identity guards on the post-await arms keep the old
    // cycle's in-flight continuations from writing into the new one.
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    // The wallet-life identity (#381): SAME identity across a rebuild ⇒ a live
    // post-commitment swap survives (a rescan swaps the session, not the
    // wallet); a DIFFERENT identity ⇒ wipe. Captured on the field so _set
    // stamps the CYCLE's identity, never a post-flip read (arch review N1).
    final identity = ref.watch(walletIdentityProvider);
    _identity = identity;
    // Cached for the actions. The session seam can legitimately flip to null (the
    // wallet closed) — pre-commitment states reset to a clean form, the honest
    // behavior (a swap screen with no live wallet must not hold a stale quote).
    _session = ref.watch(walletSessionProvider);
    if (identity != _liveIdentity) {
      _liveAcrossRebuild = null;
      return const SwapFormState();
    }
    return _liveAcrossRebuild ?? const SwapFormState();
  }

  /// Reset to a clean form — called on screen entry so a stale quote (or a
  /// terminal fault) never lingers into a new swap. A NO-OP while a step is in
  /// flight (review F1) AND — since W-swap-5 (#366) — while a COMMITTED
  /// swap is live ([SwapExecuted] / [SwapAwaitingDeposit]): re-entering the
  /// screen RE-ATTACHES to the running flow instead of wiping the one surface
  /// tracking money in motion (pre-#366 tracking was one-shot — leaving the
  /// screen orphaned a live swap with no way back). Leaving a live swap is
  /// explicit: the terminal card's Done calls [startNewSwap].
  ///
  /// The live no-op branch is a RE-ATTACH in its own right (#386, the
  /// converged HIGH): the wallet screen's Swap button pushes the screen with
  /// the live [SwapExecuted] still held — no [attachTo] call anywhere on that
  /// path — so this entry hook is where a stale synthesized-NotFound latch
  /// must re-poll. Without it, a back-nav exit from the NotFound card (the
  /// no-dismiss Done's sibling exit; [resetToForm] deliberately keeps the
  /// live state) re-rendered the stale heuristic with ZERO provider traffic
  /// on every subsequent direct entry for the process lifetime — a desktop
  /// host never self-heals via process death. Safe here: the screen calls
  /// this post-frame (never mid-build).
  void resetToForm() {
    if (_inFlight || _live) {
      final current = state;
      if (current is SwapExecuted) {
        _repollStaleNotFound(current.swapId);
      }
      return;
    }
    _set(const SwapFormState());
  }

  /// Explicitly leave a live swap's tracking for a clean form (W-swap-5) — the
  /// terminal card's Done (the swap is over; the status stream's terminal
  /// observation already dismissed the durable record) or a deliberate
  /// "start another swap". Still refuses mid-transient (a stray tap must not
  /// abandon an in-flight quote/execute — [backToForm]'s guard).
  void startNewSwap() {
    if (_inFlight) return;
    _set(const SwapFormState());
  }

  /// RE-ATTACH to a live swap by its durable record (W-swap-5, #366): the
  /// wallet screen's in-flight row / the guard-fault "view swap" hand the
  /// persisted id + coarse direction here, then push the swap screen — the
  /// tracking view re-opens the live status stream by id. A NO-OP while a
  /// transient step runs (defensive: never clobber an in-flight quote/execute),
  /// while the SAME swap is already attached (idempotent tap), and while ANY
  /// live [SwapAwaitingDeposit] holds the screen (crypto-review H2 + arch
  /// review H-A1, both probe-confirmed): its quote carries the IntoZec deposit
  /// address/amount/memo, which are deliberately NOT persisted, so replacing
  /// it with a bare [SwapExecuted] — same id OR a different row's — would
  /// one-tap-destroy the only copy of active payment instructions. The RICHER
  /// state always wins, unconditionally; the deposit screen's own
  /// markDepositSent (or its explicit leave) is the exit. The tapped OTHER
  /// swap stays one tap away in the home list once the deposit screen is done.
  void attachTo({
    required String swapId,
    required SwapFlowDirection direction,
  }) {
    if (_inFlight) return;
    final current = state;
    if (current is SwapAwaitingDeposit) return;
    // BEFORE the same-id early return (#386, the converged HIGH): the
    // COMMON re-attach — tapping the home row of the swap already attached
    // after a back-nav exit from its NotFound card — is exactly a same-id
    // attach, and the #385 first cut's invalidate below the early return
    // never ran for it (the fix worked only when a DIFFERENT swap had the
    // screen). The idempotent-tap return itself stays: same id ⇒ the state
    // is already right, only the stale latch needed the kick.
    _repollStaleNotFound(swapId);
    if (current is SwapExecuted && current.swapId == swapId) return;
    // reattached (#367): the tracking view must not instruct an action only
    // the issuing process run could follow (the IntoZec deposit instructions
    // are deliberately not persisted).
    _set(SwapExecuted(swapId, direction, reattached: true));
  }

  /// Re-poll a swap-status element stuck on a synthesized NotFound (#385
  /// review H-2; hoisted + shared with the entry path by #386): a
  /// synthesized NotFound latches the status element `_done` — deliberate for
  /// a terminal — but the NotFound card's Done no longer dismisses the record,
  /// so RE-ATTACH (the overdue home row, the guard-fault "View swap", or a
  /// direct screen entry with the live state held) is the designed mainline;
  /// without the invalidate, the latched element re-rendered the stale
  /// heuristic with ZERO provider traffic for the rest of the process run
  /// (breaking `_outcomeOf`'s "a later re-attach re-polls the provider"
  /// rationale — and a desktop host never self-heals via process death; the
  /// core re-polls honestly on invalidate, its consecutive-404 counter is
  /// loop-local). Scoped to the NotFound HEURISTIC only: a REAL terminal's
  /// latch stays (its Done dismissed the record; the reused-id class is the
  /// parked MED-1). `exists` first — a cold attach must not
  /// instantiate-then-invalidate its own element. A PINNED record skips the
  /// re-poll whole (review MED): the chain/API-observed pin outranks
  /// anything a re-poll could learn (HARD-G — the provider likely GC'd the
  /// order, which is HOW the NotFound latched over a pin), and the card's
  /// pinned-over-NotFound delegate renders the pin INSTANTLY from the latched
  /// element — invalidating would displace that money truth with the full
  /// ~75–135 s consecutive-404 re-earn busy on every re-entry. An
  /// unloaded/errored list (`records == null`) falls through to the invalidate
  /// — the defensive path: it is unreachable WITH a
  /// pin in practice, because a synthesized-NotFound latch takes ~75–135 s of
  /// in-process tracking to form, during which the always-mounted in-flight
  /// section has long since loaded the non-autoDispose list (its `.value`
  /// then stays non-null for the process life). If it ever were reached, the
  /// re-poll runs under the same-session retention (the seeded state IS
  /// a data arm), so a pin landing mid-re-poll renders IMMEDIATELY via the
  /// card's pinned-over-NotFound delegate — the chain-observed truth outranks
  /// the heuristic without waiting out the 404 re-earn; the honest fallback
  /// either way (review corrected the pre-retention wording here).
  void _repollStaleNotFound(String swapId) {
    if (!ref.exists(swapStatusProvider(swapId))) return;
    final latchedAsync = ref.read(swapStatusProvider(swapId));
    // An ACTIVE re-poll retains the latched value while `isLoading` (the
    // same-session retention): re-entering DURING the ~75–135 s re-earn must
    // not restart it — each restart re-subscribes and resets the core's
    // consecutive-404 run (a re-entry loop would otherwise keep a GC'd order
    // polling indefinitely: battery + provider traffic). Pre-the bare
    // loading reset guarded this by accident (`.value` read null); the guard
    // is now explicit.
    if (latchedAsync.isLoading) return;
    final latched = latchedAsync.value;
    if (latched is! SwapStatus_Failed ||
        latched.code != SwapFailureCode.notFound) {
      return;
    }
    final records = ref.read(walletInFlightSwapsProvider).value;
    if (records != null) {
      for (final record in records) {
        if (record.id != swapId) continue;
        if (record.outcome != null) return;
        break;
      }
    }
    ref.invalidate(swapStatusProvider(swapId));
  }

  /// Return to the form from review (the Back / Edit action). A no-op mid-flight
  /// (a transient state) so a stray tap can't abandon an in-flight quote/execute.
  void backToForm() {
    if (state is SwapQuoting || state is SwapExecuting) return;
    _set(const SwapFormState());
  }

  /// The live session, or null after surfacing the honest "wallet unavailable"
  /// fault (the screen is gated to an active, swap-enabled wallet, so null is a
  /// defensive "wallet closed mid-screen", never expected). One home for the
  /// guard the actions share.
  WalletSession? _requireSession() {
    final session = _session;
    if (session == null) {
      // Fresh (non-const) fault instance — see _quoteOutOfZec's invariant note.
      _set(
        SwapFormState(
          fault: SwapCategoricalFault(SwapFaultReason.walletUnavailable),
        ),
      );
    }
    return session;
  }

  /// Request a bounds-checked quote (§2.6) for either direction. Dispatches on the
  /// sealed [SwapFormInput] so each shape carries only its own fields. On success →
  /// [SwapReview] with the confirmable numbers + the §2.6 disclosure; on a typed
  /// failure → back to [SwapFormState] with an honest inline fault. Funds NEVER
  /// move here — quote is deterministic and side-effect-free.
  Future<void> quote(SwapFormInput input) async {
    if (_inFlight) return; // re-entrancy: ignore taps while a step runs
    final session = _requireSession();
    if (session == null) return;

    switch (input) {
      case OutOfZecInput():
        await _quoteOutOfZec(session, input);
      case IntoZecInput():
        await _quoteIntoZec(session, input);
    }
  }

  /// OutOfZec (ZEC → [OutOfZecInput.asset]): the ZEC amount is parsed host-side
  /// first (integer-exact, never via double); the foreign destination is required
  /// (rejected host-side before any provider call). The SDK supplies the fresh,
  /// single-use transparent refund address (a host-supplied one is IGNORED).
  Future<void> _quoteOutOfZec(
    WalletSession session,
    OutOfZecInput input,
  ) async {
    final parsed = parseZecAmount(input.amountText);
    if (parsed is ZecAmountInvalid) {
      _set(SwapFormState(fault: SwapAmountFault(parsed.fault)));
      return;
    }
    final zat = (parsed as ZecAmountValid).zat;

    final dest = input.destination.trim();
    if (dest.isEmpty) {
      // Deliberately NON-const (#364 S5; the send form's #329-2 invariant):
      // these validation faults land directly from a possibly fault-carrying
      // form with NO transient in between, and the screen's scroll-to-fault
      // listener re-fires on instance identity — a const-canonicalized fault
      // would make a same-reason repeat (Get quote tapped twice on the same
      // empty field) identical to its predecessor and silently skip the
      // re-scroll. Every direct-from-form fault set below follows suit.
      _set(
        SwapFormState(
          fault: SwapCategoricalFault(SwapFaultReason.destinationRequired),
        ),
      );
      return;
    }

    final request = QuoteRequest(
      direction: SwapDirection.outOfZec(to: input.asset.toAssetId()),
      exact: ExactSide.in_(amount: SwapAmount.zec(zat: zat)),
      slippageToleranceBps: input.slippageBps,
      destination: dest,
      // OutOfZec: the SDK supplies the fresh, single-use wallet refund address.
      refundAddress: null,
    );

    await _runQuote(session, request, SwapFlowDirection.outOfZec, input.asset);
  }

  /// IntoZec ([IntoZecInput.token] → ZEC): the amount is a FOREIGN decimal string
  /// passed through to the SDK verbatim — NEVER parsed to zatoshis host-side
  /// (§3.3b L8); only a host-side non-empty check (the SDK validates the decimal).
  /// The refund address is required + non-empty here, then validated at the Rust
  /// boundary (§3.3b D6 — opaque, never parsed as a Zcash address). No destination
  /// is sent: delivery goes to the wallet's own fresh address (SDK-minted).
  Future<void> _quoteIntoZec(WalletSession session, IntoZecInput input) async {
    final amount = input.amountText.trim();
    if (amount.isEmpty) {
      // NON-const — see _quoteOutOfZec's invariant note (#364 S5).
      _set(
        SwapFormState(
          fault: SwapCategoricalFault(SwapFaultReason.foreignAmountRequired),
        ),
      );
      return;
    }

    final refund = input.refundAddress.trim();
    if (refund.isEmpty) {
      // NON-const — see _quoteOutOfZec's invariant note (#364 S5).
      _set(
        SwapFormState(
          fault: SwapCategoricalFault(SwapFaultReason.refundAddressRequired),
        ),
      );
      return;
    }

    final request = QuoteRequest(
      direction: SwapDirection.intoZec(from: input.token.toAssetId()),
      exact: ExactSide.in_(amount: SwapAmount.foreign(amount: amount)),
      slippageToleranceBps: input.slippageBps,
      // IntoZec: delivery is the wallet's own fresh address — no destination.
      destination: null,
      refundAddress: refund,
    );

    await _runQuote(session, request, SwapFlowDirection.intoZec, input.token);
  }

  /// Shared quote round-trip: forward to the SDK and land on [SwapReview]
  /// (carrying the direction + the foreign [counter] asset for the review framing)
  /// or back to the form with an honest fault. DRY across both directions.
  Future<void> _runQuote(
    WalletSession session,
    QuoteRequest request,
    SwapFlowDirection direction,
    SwapAsset counter,
  ) async {
    // Synchronous transient transition BEFORE the first await — the double-tap
    // interlock (a second quote sees `_inFlight` and no-ops; everything from
    // quote() to here is synchronous, so the interlock window is unchanged).
    // NON-const: the post-await guards match on INSTANCE IDENTITY (#330) — a
    // canonicalized const would alias a different build cycle's transient.
    // ignore: prefer_const_constructors
    final quoting = SwapQuoting();
    _set(quoting);
    try {
      // A host-side timeout so a stalled dial can't hang "Getting a quote…"
      // forever — TimeoutException is classified to the honest couldNotQuote
      // fault (re-quote), exactly like any other quote failure.
      final quote = await session
          .swapQuote(request: request)
          .timeout(kSwapNetworkTimeout);
      if (_disposed || !identical(state, quoting)) return;
      // FR-23 (maintainer 2026-07-10): the alpha send ceiling BOUNDS the OutOfZec
      // deposit — refuse an over-ceiling swap HERE, at review, before the
      // authorize-spend bracket (every alpha money-out path under one cap). The
      // reviewed quote's ZEC side IS the deposit amount. IntoZec moves no wallet
      // funds (the user sends the deposit externally), so the cap never applies.
      if (direction == SwapFlowDirection.outOfZec) {
        final ceiling = ref.read(walletSendCeilingZatProvider);
        if (ceiling != null && quote.zecSideZat > ceiling) {
          _set(SwapFormState(fault: SwapOverCeiling(ceiling)));
          return;
        }
        // #367 spendable pre-check: the execute
        // chokepoint never re-validates funds — a catching-up 0-balance wallet
        // could quote → ack → execute, the engine SWALLOWS the sign-time
        // InsufficientFunds by contract, and the UI landed "Swap started" over
        // a deposit that can never fund while SwapAlreadyInFlight blocks every
        // new quote. Refuse HERE, at the same review door as the ceiling
        // (deposit + the fee allowance vs spendable-now). A missing snapshot
        // (cold read still in flight) fails OPEN — the engine gate stays the
        // money-safety backstop and the durable home keeps a doomed deposit
        // visible; this check exists for honesty, not custody.
        final spendable = ref
            .read(walletSnapshotProvider)
            .value
            ?.balance
            .spendableZat;
        final needed = quote.zecSideZat + kSwapDepositFeeAllowanceZat;
        if (spendable != null && needed > spendable) {
          // The catch-up cue only HEDGES the refusal copy — a composition
          // that doesn't wire the rescan stack (its provider chain throws)
          // must not degrade the honest refusal into a generic fault.
          bool catchingUp;
          try {
            catchingUp =
                ref.read(walletCatchUpCueProvider) is! WalletCatchUpNone;
          } on Object {
            catchingUp = false;
          }
          _set(
            SwapFormState(
              fault: SwapInsufficientSpendable(
                neededZat: needed,
                spendableZat: spendable,
                catchingUp: catchingUp,
              ),
            ),
          );
          return;
        }
      }
      _set(
        SwapReview(
          quote: quote,
          direction: direction,
          counter: counter,
          // N03: the payout address the quote was requested for — the review
          // renders it for verification (IntoZec sends none: `null`).
          payoutAddress: request.destination,
        ),
      );
    } catch (error) {
      if (_disposed || !identical(state, quoting)) return;
      _set(SwapFormState(fault: classifySwapFailure(error)));
    }
  }

  /// Execute the reviewed quote — claim the durable single-flight (by id), register
  /// the swap intent, and (OutOfZec only) queue the §4.4 ZEC deposit. On success:
  /// OutOfZec → [SwapExecuted] (the wallet sent the deposit; track it); IntoZec →
  /// [SwapAwaitingDeposit] (the D7 "send your funds" screen — the USER deposits
  /// externally, §3.3b L7). Every TYPED failure routes back to the FORM with a
  /// re-quote fault: the core claims the single-flight FIRST, so a failed execute
  /// may have consumed the quote — re-executing would be `QuoteExpired` (#367);
  /// re-quoting is the only safe path (never a double-deposit). The ONE exception
  /// is an OutOfZec TIMEOUT (W-swap-4-a-2): a `.timeout` abandons — not cancels —
  /// the Rust future, so the deposit may be signing/in motion; that arm goes to
  /// TRACKING by quote id, never to a re-quotable form (a timeout is not a
  /// failure). The SDK's `SwapAlreadyInFlight` guard backstops any host that
  /// re-quotes anyway.
  Future<void> execute() async {
    final current = state;
    if (current is! SwapReview) return; // only from the review screen
    final session = _requireSession();
    if (session == null) return;
    final quote = current.quote;
    final direction = current.direction;
    final counter = current.counter;
    final payoutAddress = current.payoutAddress;

    // The host-authorization seam (#327): ONLY the OutOfZec direction commits
    // wallet funds (the §4.4 ZEC deposit — signed at execute INSIDE this bracket,
    // FR-23-a), so only it is authorized. An IntoZec execute registers the swap
    // and mints a receive address — no wallet funds move, and prompting for a
    // spend that isn't one would be dishonest.
    final authorizer = ref.read(walletSendAuthorizerProvider);

    // Synchronous transient transition before the await — the double-tap
    // interlock (a second execute sees `state != SwapReview` and no-ops, so the
    // quote is executed at most once here; the SDK's durable single-flight is the
    // backstop, refusing a re-claim typed if a tap still slips through).
    // Captured for the INSTANCE-IDENTITY guards below (#330) — the authorizer
    // await spans user think-time at the host's prompt.
    final executing = SwapExecuting(quote);
    _set(executing);
    try {
      // Same host-side timeout backstop — a stalled execute can't hang the
      // "Starting…" spinner; any failure routes back to the form to re-quote
      // (the SDK's durable single-flight is the money-safety backstop). The
      // timeout sits INSIDE the authorized action so it bounds only the
      // network round-trip, never the user's time at the host's prompt.
      final swapId = switch (direction) {
        SwapFlowDirection.outOfZec => await authorizer.authorizeSpend(
          WalletSpendIntent(
            kind: WalletSpendKind.swapDeposit,
            amountZat: quote.zecSideZat,
            // #383 R3: bind the prompt to the provider deposit address the
            // execute will pay (elided, render-never-log). NOT a self-transfer
            // (the deposit leaves the wallet); the network fee is discovered
            // at the execute-time propose, so feeZat stays null here.
            recipientAbbrev: abbreviateWalletAddress(quote.depositAddress),
            // N03: the user's own foreign payout address, kept DISTINCT from
            // the provider deposit address above (the review showed it in
            // full; a host prompt may show it again).
            payoutAbbrev: payoutAddress == null
                ? null
                : abbreviateWalletAddress(payoutAddress),
            // FR-17 (#396): the quote's spend-binding nonce — the deposit's
            // sign-time seed pull presents it, so a host-custody supplier
            // fail-closes anything but THIS reviewed quote.
            bindingToken: quote.binding,
          ),
          () {
            // The identity-switch fence: an approval landing after
            // a session flip must never commit the DEAD identity's deposit
            // (see the seam contract). `ref.mounted` maps whole-scope
            // teardown to the same documented type.
            if (!ref.mounted ||
                !identical(ref.read(walletSessionProvider), session)) {
              throw const WalletSpendSessionChanged();
            }
            return _executeBounded(session, quote);
          },
        ),
        SwapFlowDirection.intoZec => await _executeBounded(session, quote),
      };
      if (_disposed) return;
      // An OutOfZec execute just COMMITTED the §4.4 deposit (signed at execute,
      // FR-23-a) — refresh the cold snapshot ABOVE the identity guard (
      // wrap review: the screen's own listen dies with the widget, so an
      // away-at-landing deposit otherwise refreshed nothing; shield/move
      // parity). IntoZec moves no wallet funds — nothing to refresh.
      if (direction == SwapFlowDirection.outOfZec) {
        ref.invalidate(walletSnapshotReadProvider);
      }
      // The durable swap HOME refreshes on the SAME away-at-landing edge for
      // BOTH directions (arch review M-A3): the record committed at execute,
      // and the screen's own listener (the usual invalidation site) dies with
      // the widget — without this, a pop-mid-execute left the fresh swap off
      // the home until a sync edge or resume, transiently recreating the
      // "in progress, invisible" state (and starving the guard-fault's
      // "View swap" of its record).
      ref.invalidate(walletInFlightSwapsReadProvider);
      if (!identical(state, executing)) return;
      _set(switch (direction) {
        SwapFlowDirection.outOfZec => SwapExecuted(
          swapId,
          SwapFlowDirection.outOfZec,
        ),
        SwapFlowDirection.intoZec => SwapAwaitingDeposit(
          swapId: swapId,
          quote: quote,
          counter: counter,
        ),
      });
    } on WalletSpendAuthorizationDenied {
      // Declined at the host's prompt BEFORE the execute call — the durable
      // single-flight was never claimed, so the quote stays executable; back
      // to the review, no fault (the seam contract — a stale busy banner from
      // an earlier attempt must not survive the denial, review fold). Identity
      // guard (review H2, tightened in #330): never restore over a state
      // the machine moved past — a bare type check would pass for a NEW
      // cycle's own Executing.
      if (_disposed || !identical(state, executing)) return;
      _set(
        SwapReview(
          quote: quote,
          direction: direction,
          counter: counter,
          payoutAddress: payoutAddress,
        ),
      );
    } on WalletSpendSessionChanged {
      // The SDK's identity-switch fence — nothing was attempted; explicit arm
      // so the classifier can never shape it as a failure.
      return;
    } on _SwapExecuteTimedOut {
      // The SDK execute call ITSELF timed out (W-swap-4-a-2 — the timeout→track
      // defense; the sentinel is thrown only around `session.swapExecute`, so a
      // host authorizer's own TimeoutException can never land here — review MED:
      // that one falls to the generic classifier below, honest because nothing
      // was executed and the quote is unconsumed). Dart's `.timeout` ABANDONS
      // the Rust future, it cannot cancel it: by now the durable deposit enqueue
      // has long committed (it precedes the unbounded ZK-proving sign that
      // overran the clock), so for OutOfZec money may be in motion. Routing to
      // the re-quotable form here is exactly the double-deposit door — a
      // re-quote mints a NEW quote id and defeats the per-quote single-flight
      // (the SDK's in-flight guard would refuse the second deposit, but the
      // honest UI never invites it). TRACK instead: the quote id is the SDK's
      // own execution identity (S8) — the same value execute returns and the
      // durable home row is keyed by, so it IS the tracking id — and the
      // tracking view renders the live status from "sending your deposit"
      // through the terminal outcome. The snapshot refresh sits ABOVE the
      // identity guard (parity with the success arm — an away-at-timeout
      // deposit must still refresh the balance).
      if (_disposed) return;
      if (direction == SwapFlowDirection.outOfZec) {
        ref.invalidate(walletSnapshotReadProvider);
      }
      // Home refresh above the identity guard, timeout parity with the
      // success arm (arch review M-A3): by the timeout the durable enqueue —
      // and record-first, the home row — has long committed.
      ref.invalidate(walletInFlightSwapsReadProvider);
      if (!identical(state, executing)) return;
      if (direction == SwapFlowDirection.outOfZec) {
        _set(SwapExecuted(quote.id, SwapFlowDirection.outOfZec));
      } else {
        // IntoZec moved no wallet funds and registered nothing new at execute
        // (registration happened at quote) — the re-quote form is the honest
        // arm. The HEDGED timeout reason (#367, F5): a ~30 s local store
        // stall can consume most of the window, so the copy must not firmly
        // blame the user's connection the way the quote-timeout copy does.
        _set(
          const SwapFormState(
            fault: SwapCategoricalFault(SwapFaultReason.executeTimedOut),
          ),
        );
      }
    } catch (error) {
      if (_disposed || !identical(state, executing)) return;
      final fault = classifySwapFailure(error);
      // #367: a BUSY execute claim consumed NOTHING — the quote is still
      // executable, so restore the review with the retryable fault inline
      // instead of bouncing to the form (a forced re-quote would needlessly
      // burn a valid quote). Every other failure keeps the re-quote form.
      if (fault is SwapCategoricalFault &&
          fault.reason == SwapFaultReason.storeBusy) {
        _set(
          SwapReview(
            quote: quote,
            direction: direction,
            counter: counter,
            payoutAddress: payoutAddress,
            fault: fault,
          ),
        );
        return;
      }
      _set(SwapFormState(fault: fault));
    }
  }

  /// IntoZec only: the user tapped "I've sent the funds" on the D7 deposit screen
  /// — advance to [SwapExecuted] live tracking (the swap was already registered at
  /// execute; this is purely a UX hand-off, it sends nothing). A no-op from any
  /// other state so a stray late callback can't derail the flow.
  void markDepositSent() {
    final current = state;
    if (current is! SwapAwaitingDeposit) return;
    _set(SwapExecuted(current.swapId, SwapFlowDirection.intoZec));
  }

  bool get _inFlight => state is SwapQuoting || state is SwapExecuting;

  /// The SDK execute call under its host-side timeout, with the timeout
  /// RE-TYPED to the private [_SwapExecuteTimedOut] sentinel AT the call — so
  /// the outer catch can distinguish "the SDK call overran" (money may be in
  /// motion → track) from a host authorizer's own `TimeoutException` (its
  /// prompt timed out BEFORE the action ran — nothing executed, quote
  /// unconsumed → the generic classifier's honest re-quote form). Catching the
  /// raw `TimeoutException` at the outer level conflated the two and rendered
  /// a live "swap in progress" money screen for a swap that never executed
  /// (review MED).
  Future<String> _executeBounded(WalletSession session, SwapQuote quote) async {
    try {
      return await session
          .swapExecute(quote: quote)
          .timeout(kSwapNetworkTimeout);
    } on TimeoutException {
      throw const _SwapExecuteTimedOut();
    }
  }

  void _set(SwapFlowState next) {
    if (_disposed) return;
    // Stamp/clear the same-identity carry-over (W-swap-5): a COMMITTED swap
    // ([SwapExecuted]/[SwapAwaitingDeposit]) survives a session-swap rebuild;
    // everything else resets with the rebuild as before. The stamp is the
    // BUILD-cycle identity (_identity), not a fresh read — by-construction
    // fencing (arch review N1).
    if (next is SwapExecuted || next is SwapAwaitingDeposit) {
      _liveAcrossRebuild = next;
      _liveIdentity = _identity;
    } else {
      _liveAcrossRebuild = null;
    }
    state = next;
  }

  /// True while a COMMITTED swap is live on this controller — the states the
  /// screen re-attaches to instead of wiping (W-swap-5, #366).
  bool get _live => state is SwapExecuted || state is SwapAwaitingDeposit;
}

/// Private sentinel: `session.swapExecute` (and ONLY it) overran
/// [kSwapNetworkTimeout]. See [SwapController._executeBounded].
class _SwapExecuteTimedOut implements Exception {
  const _SwapExecuteTimedOut();
}
