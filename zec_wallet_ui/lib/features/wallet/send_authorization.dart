/// The host send-authorization seam (#327 — security review F1).
///
/// Some custody models authorize each SPEND individually: the host holds no
/// standing signing capability and unlocks it per transaction (e.g. an
/// app-passphrase-gated staged credential, or a biometric-per-send policy).
/// The package's controllers route EVERY money-committing bridge call through
/// [WalletSendAuthorizer.authorizeSpend], so such a host plugs its
/// prompt → unlock → sign → re-lock cycle in by overriding
/// `walletSendAuthorizerProvider` — no session decorator required. The default
/// [WalletPassthroughSendAuthorizer] runs the call unmodified, which is
/// correct for sealed-keychain custody (the SDK signs whenever asked).
library;

import 'dart:typed_data';

/// What kind of money-committing action is being authorized — drives the
/// host's prompt copy ("Authorize this payment" vs "Authorize recovering
/// funds"). Every arm the package can emit; a host `switch` stays exhaustive.
enum WalletSpendKind {
  /// An interactive send to an external recipient (signs + broadcasts now).
  send,

  /// An offline-queued send. Emitted at TWO distinct moments, and a host prompt
  /// may want different copy for each — [WalletSpendIntent.bindingToken] tells
  /// them apart (null ⇔ the commit moment):
  ///
  ///  * **COMMIT** (`session.queueSend`, binding null): the intent is durably
  ///    persisted now; signing happens later. See the caveat on
  ///    [WalletSendAuthorizer.authorizeSpend].
  ///  * **AUTHORIZE-NOW** (`session.authorizeParkedSend`, FR-23-b, binding set):
  ///    the user asked the SDK to sign a parked send at this moment, so the
  ///    signature happens INSIDE this bracket — like [send], not deferred. This
  ///    is how an offline queue drains at host custody at all.
  queuedSend,

  /// Shielding the wallet's own transparent funds (signs + broadcasts now).
  shield,

  /// Moving shielded funds to the wallet's own transparent address
  /// (signs + broadcasts now).
  unshield,

  /// The manual one-time-address recovery sweep (signs + broadcasts one tx
  /// per funded address — the total is unknown up front, so
  /// [WalletSpendIntent.amountZat] is null).
  sweep,

  /// Reopening a bricked one-time-address (TEX) send window (#315): a small
  /// self-mint from the wallet's own shielded pool to its own one-time address
  /// so the window frees up (signs + broadcasts now). Wallet-INTERNAL — the
  /// moved amount returns to the wallet via [sweep]; the true cost is the two
  /// transactions' network fees. [WalletSpendIntent.amountZat] is null (the fee
  /// isn't known up front, and the principal is not a debit — it returns).
  reclaim,

  /// A swap's outgoing ZEC deposit (OutOfZec execute). Signs + broadcasts NOW,
  /// inside this bracket (FR-23-a / custody invariant): the deposit is durably
  /// enqueued then signed at execute, while the user is present — so swap works
  /// at every custody tier (a host-custody wallet serves the seed here). NOT the
  /// deferred-signing caveat that [queuedSend] still carries. An IntoZec execute
  /// moves no wallet funds and is never authorized.
  swapDeposit,
}

/// Who initiated the spend being authorized — a host POLICY input (#328).
/// [automatic] marks a spend NO user gesture triggered; the only automatic
/// spend the package emits today is the auto-shield loop's self-transfer
/// ([WalletSpendKind.shield] — funds stay inside the wallet).
enum WalletSpendOrigin {
  /// A user-confirmed action (a confirm/queue/execute tap). The default.
  userInitiated,

  /// A package policy loop, with no user waiting on a prompt. A host
  /// authorizer that PROMPTS per spend should decide these by POLICY instead:
  /// auto-approve (an automatic spend is always a wallet-internal
  /// self-transfer — never an external payment), prompt anyway, or throw
  /// [WalletSpendAuthorizationDenied]. For THIS origin a promptless denial is
  /// fine (the usual communicate-first precondition guards a user staring at
  /// a confirm button; here there is none) — the loop stops for the session
  /// and the funds stay honestly visible on the balance card.
  ///
  /// PACKAGE INVARIANT (hosts may rely on it; review): every automatic
  /// intent the package emits has `kind == WalletSpendKind.shield` — a
  /// self-transfer into the wallet's own shielded pool. Any future automatic
  /// flow that would widen this MUST amend this doc and the host guidance
  /// first; the auto-shield test suite pins the pairing.
  automatic,
}

/// The display-facts of the spend being authorized — enough for the host's
/// prompt copy, nothing more. §5.4 never-log values: render them in the
/// prompt, never write them to a log.
final class WalletSpendIntent {
  const WalletSpendIntent({
    required this.kind,
    this.amountZat,
    this.origin = WalletSpendOrigin.userInitiated,
    this.recipientAbbrev,
    this.payoutAbbrev,
    this.recipientIsSelf = false,
    this.feeZat,
    this.bindingToken,
    this.machineMemoPurpose,
  });

  final WalletSpendKind kind;

  /// The figure the flow knows at authorization time, in zatoshis — null when
  /// nothing honest exists yet (sweep / reclaim discover their totals as they
  /// run). PER-KIND semantics (a prompt that renders "Authorize
  /// {amount}" must know which claim it makes):
  ///
  ///  * send / shield / unshield / auto-shield: the proposal's TOTAL debit
  ///    (recipient amounts + fee) — [feeZat] is included in this figure.
  ///  * queuedSend: the TYPED PRINCIPAL only — signing is deferred, so the
  ///    fee is unknown and added at drain; the wallet will be debited MORE
  ///    than this figure.
  ///  * swapDeposit: the quote's ZEC side (what the swap provider receives) —
  ///    the network fee is on top; the wallet is debited more than this.
  ///
  /// A host prompt should render the total-kinds as "total (incl. fee)" and
  /// the principal-kinds as the amount with the fee named separately/unknown
  /// — never one fixed phrasing across kinds.
  final int? amountZat;

  /// Who initiated this spend — see [WalletSpendOrigin]. Host policy input;
  /// does not change any package-side money behavior.
  final WalletSpendOrigin origin;

  /// An ELIDED display form of the recipient (#383 R3 — via
  /// [abbreviateWalletAddress]), so a per-spend prompt can bind confirm→sign
  /// to WHAT is signed ("Authorize 1.2 ZEC total, incl. fee, to
  /// u1abcd…wxyz?" — phrase totals per the [amountZat] kind table, never
  /// "Send {total} to": the recipient receives total − fee). §5.4: RENDER it
  /// in the prompt, never log it — even elided, it correlates on-chain. Null
  /// when there is no meaningful external recipient string: the self-transfer
  /// kinds (see [recipientIsSelf]) and a drain-signed [WalletSpendKind.queuedSend]
  /// carry what the flow knows at authorization time, nothing invented.
  ///
  /// For [WalletSpendKind.swapDeposit] this is the swap PROVIDER'S deposit
  /// address — the user has never seen or copied it, so present it as "the
  /// swap provider's deposit address", not as something to visually verify
  /// (there is no reference copy; the address was validated core-side at
  /// quote time and the execute pays exactly that parse).
  final String? recipientAbbrev;

  /// [WalletSpendKind.swapDeposit] only: an ELIDED display form (via
  /// [abbreviateWalletAddress]) of the user's own FOREIGN payout address — where
  /// the provider sends the swapped asset. Unlike [recipientAbbrev] (the
  /// provider's deposit address, which the user has never seen), this is the
  /// address the user typed, pasted or scanned and verified in full on the
  /// review, so a prompt may show it for the user to match. Kept separate from
  /// [recipientAbbrev] on purpose: the deposit and the payout are different
  /// addresses on different chains. Null for every other kind. §5.4: RENDER it,
  /// never log it.
  final String? payoutAbbrev;

  /// True when the package POSITIVELY knows the funds return to THIS wallet
  /// (shield / unshield / sweep / reclaim, and an interactive send whose
  /// proposal detected an own-address recipient). False means "external or
  /// unknown" — never treat false as a verified-external claim. Lets a host
  /// prompt render "internal transfer, funds stay in your wallet" honestly.
  ///
  /// COUPLING: the interactive-send arm forwards the core's
  /// `selfSend`, whose Rust semantics are ANY-leg (`true` if any payment leg
  /// pays an own address). That equals the whole-spend claim above ONLY
  /// because the send flow composes a SINGLE-payment URI by construction
  /// (see the note at the send controller's confirm site). Any future path
  /// that proposes a multi-payment request (scan-to-pay ZIP-321, a host
  /// prefill) must NOT forward `selfSend` into this field unchanged — a
  /// mixed request (own + external legs) would read `true` while money
  /// leaves the wallet.
  final bool recipientIsSelf;

  /// The network fee in zatoshis, where the flow holds a proposal that knows
  /// it at authorization time; null when signing is deferred (queuedSend) or
  /// the fee is discovered later (sweep, reclaim, swap deposit).
  final int? feeZat;

  /// FR-17 (#396): the SDK-minted spend-binding nonce for the EXACT
  /// proposal/quote this authorization covers. A host with a native seed port
  /// records it at stage time, and its supplier fail-closes a native seed
  /// pull presenting anything else — the review↔sign confusion defense. Null
  /// for the operations that pull unbound (the shield-to-self sweep /
  /// reclaim) and for offline QUEUE-time authorization (the binding is minted
  /// at enqueue and surfaced on the parked-send row for a re-stage). NOT key
  /// material — but do not log it (it correlates a proposal with its sign
  /// moment; §5.4 like every field here).
  ///
  /// ENFORCEMENT PREREQUISITE (else recording this is a silent no-op): FR-17
  /// only bites when the host registered the BOUND native seed supplier
  /// (`zec_wallet_register_seed_port_bound`) AND its supply callback compares
  /// this token (strict equality; unbound stage ⇔ null pull) and returns
  /// unavailable on mismatch. A default passthrough authorizer or a
  /// sealed-keychain custody wallet has no native pull to bind — the token is
  /// inert there, which is fine.
  ///
  /// QUEUE handoff (both moments — see [WalletSpendKind.queuedSend]). At
  /// QUEUE-TIME this is null: the intent is only being committed, and signing is
  /// deferred, so a stage recorded here can never serve the later bound drain
  /// pull (the send would park unsigned — funds safe). At AUTHORIZE-NOW time
  /// (FR-23-b `authorizeParkedSend`) it is the PARKED ROW's own binding, copied
  /// straight from `ParkedSend.binding` — stage for exactly this value and the
  /// sign inside your bracket succeeds; stage for any other row and it is
  /// refused, leaving the send parked rather than wrongly signed.
  final Uint8List? bindingToken;

  /// FR-28 — when the payment carries an opaque MACHINE MEMO, the short
  /// human-readable purpose whoever attached it supplied. Null for the ordinary
  /// case (no machine memo).
  ///
  /// The package already discloses this itself — on the form (which is where
  /// the OFFLINE QUEUE commits, without ever passing a review) and again on the
  /// review — per send and non-dismissible, so a host prompt that ignores it is
  /// not hiding anything.
  /// It is offered because a credential prompt IS the authorization moment for
  /// a per-spend-credential host, and a user asked to approve a spend should be
  /// able to see everything that spend attaches without leaving the prompt.
  ///
  /// RENDER it, never log it (§5.4) — and render it as what it is: an
  /// attacker-controllable string. The wallet cannot check that it describes
  /// the bytes; a label is accountability, not verification.
  final String? machineMemoPurpose;
}

/// Elide an address for prompt display (#383 R3): keeps enough of both ends
/// to visually match against a copied address, short enough that a prompt
/// stays one line. Deterministic, display-only — §5.4: never log the result.
String abbreviateWalletAddress(String address) {
  final a = address.trim();
  if (a.length <= 20) return a;
  return '${a.substring(0, 12)}…${a.substring(a.length - 6)}';
}

/// Thrown by a [WalletSendAuthorizer] when the user (or a host policy)
/// declines the spend. The controllers catch it BEFORE any classification and
/// silently restore the pre-confirm state (review stays reviewable, the
/// one-shot proposal token is unconsumed, ZERO bridge calls were made). The
/// host's own prompt is the user-facing communication channel for a denial —
/// the package deliberately shows no additional fault for it.
///
/// PRECONDITION: throw this ONLY after your own UI has communicated the
/// outcome (the user dismissed your prompt, or your policy UI explained the
/// block). A PROMPTLESS denial makes the confirm button appear dead — it
/// flashes the transient and silently bounces back. For a policy block with
/// no UI of its own, show your own message first, or throw a real error so
/// the package classifies it as a visible failure.
final class WalletSpendAuthorizationDenied implements Exception {
  const WalletSpendAuthorizationDenied();
}

/// Thrown by the SDK from INSIDE an authorized action when the wallet session
/// changed (a host identity switch / wallet re-open) between the prompt
/// opening and the approval landing — the spend is REFUSED: running it
/// would move the DEAD identity's money while every visible surface already
/// shows the new one, so the user could never see what they just approved.
/// Nothing was signed or broadcast.
///
/// Hosts SHOULD deny (or dismiss) any outstanding authorization prompt when
/// they switch identity; this fence is the SDK-side backstop for an approval
/// that lands anyway. If your authorizer surfaces action failures to the
/// user, treat THIS one as a silent no-op — the identity switch already
/// replaced the whole screen context, and "payment failed" would be untrue
/// (no payment was attempted).
///
/// MECHANISM: the fence keys off session OBJECT IDENTITY
/// (`identical` against `walletSessionProvider`'s value) — the same identity
/// rule the whole live-sync graph relies on. Never reuse one session object
/// across identities or swap a facade's inner handle in place: a same-object
/// identity switch is INVISIBLE to the fence, and for the tokenless calls
/// (queue, swap execute) that would commit the old identity's spend against
/// the new identity's wallet.
final class WalletSpendSessionChanged implements Exception {
  const WalletSpendSessionChanged();
}

/// The seam itself. The package invokes [authorizeSpend] around every
/// money-committing bridge call, exactly once per user-confirmed action:
///
///  * interactive send / shield / move-to-transparent (`session.send`),
///  * the manual ephemeral sweep (`session.sweepEphemeralFunds`),
///  * the offline queue (`session.queueSend`), and
///  * a swap's OutOfZec execute (`session.swapExecute`).
///
/// A host implementation typically: prompts the user (its own UI — it owns a
/// navigator; the package passes no BuildContext), unlocks its per-send
/// signing credential, runs [action], and re-locks in a `finally` so the
/// credential can never outlive the one call it authorized:
///
/// ```dart
/// class HostSendAuthorizer implements WalletSendAuthorizer {
///   @override
///   Future<T> authorizeSpend<T>(
///       WalletSpendIntent intent, Future<T> Function() action) async {
///     final passphrase = await promptForPassphrase(intent);
///     if (passphrase == null) throw const WalletSpendAuthorizationDenied();
///     await stageSpendCredential(passphrase);
///     try {
///       return await action();
///     } finally {
///       // Swallow re-lock failures: a `finally` that throws DISCARDS the
///       // completed action's result (Dart semantics) — the spend RAN, and
///       // an escaping cleanup error would make the flow report "nothing
///       // was sent" over moved money. But do NOT ignore it: a credential
///       // that silently outlives its one call re-arms the SDK's DEFERRED
///       // signing paths (parked-send drain, a TEX second leg) until the
///       // next stage/clear or process death — alarm, log (code only),
///       // and retry the clear.
///       try {
///         await clearSpendCredential();
///       } catch (_) {
///         // alarm + schedule a clear retry — never rethrow
///       }
///     }
///   }
/// }
/// ```
///
/// CONTRACT (the type system enforces most of it — `T` is opaque, so the only
/// ways out are running [action] or throwing):
///
///  * Run [action] at most once, and return ITS result / let ITS error
///    propagate untouched — the controllers' typed-fault classification
///    depends on seeing the SDK's own errors. Never retry [action] inside
///    the authorizer: for proposal-consuming kinds the SDK's one-shot token
///    backstops a double run, but a [WalletSpendKind.queuedSend] has NO
///    token — a second run queues a SECOND send, a double pay at drain.
///  * Throw [WalletSpendAuthorizationDenied] to decline — but ONLY when your
///    own UI already communicated the denial (its doc has the precondition;
///    the package shows nothing for it). Any OTHER throw is
///    treated as a real failure and classified like a signing error. That
///    includes a throw from YOUR cleanup after [action] completed — Dart's
///    `finally` discards the completed result, so the flow would report a
///    failure over money that MOVED. Cleanup must swallow its own errors
///    (see the example).
///  * ALWAYS complete (resolve or throw): the flow holds its transient
///    "Submitting…" state until you do, so a prompt must be cancelable. The
///    package deliberately applies NO timeout here — authorization
///    legitimately takes as long as the user takes. NOTE: screen
///    re-entry now RE-ATTACHES to an in-flight flow instead of resetting it,
///    so a prompt that never completes wedges that flow until a session
///    change — there is no package-side escape by design; completion is
///    yours to guarantee.
///  * Expect CONCURRENT invocations from independent flows — a running
///    ephemeral sweep's bracket stays open across its whole multi-tx run
///    while a send confirm can start its own. Every interleaving is
///    money-safe SDK-side (one spend sub-seed; one-shot proposal tokens),
///    but a custody model with a SINGLE staging slot must serialize the
///    brackets internally (an async mutex around this method) or an early
///    clear kills the other flow's slot into a spurious typed failure.
///  * IDENTITY SWITCHES: deny/dismiss any outstanding prompt when the host
///    flips the wallet session (a duress/decoy or account switch). As the
///    backstop, an approval that lands after the flip is FENCED — [action]
///    throws [WalletSpendSessionChanged] instead of spending from the dead
///    identity's wallet, and the package shows nothing (see that type's doc).
///    Note the fence covers the not-yet-run action only: a spend whose action
///    was ALREADY in flight when the flip happened completes on the old
///    session — its result screen is deliberately discarded, and the old
///    identity's own surfaces carry the truth (activity for a send/shield/
///    move, the parked list for a queued send, in-flight for a two-step; an
///    OutOfZec swap's tracking state is session-local — the deposit itself
///    stays bounded by its quote deadline).
///  * The prompt is MODAL over the send screen (S13). While it is up the
///    user must not be able to reach the screen underneath: the screen's
///    "Still sending — your payment keeps going if you leave" is true only
///    once the action has run. If the screen is removed while the prompt is
///    still up (a host `pop`/`go`/`replace`), the flow is ABANDONED and
///    [action] refuses before spending — so the "no transaction" the host was
///    told stays true. A non-modal prompt would let a user leave a spend they
///    were still deciding on, and then have their approval refused.
///
/// DEFERRED-SIGNING CAVEAT: at the QUEUE-TIME [WalletSpendKind.queuedSend] the
/// authorization moment is the COMMIT moment, not the signing moment — the SDK's
/// background drain signs it later, and this hook cannot carry a per-send
/// credential across to that drain.
///
/// FR-23-b (#361) LIFTS the consequence: the package's parked-sends surface now
/// offers "Send now", which calls `session.authorizeParkedSend` INSIDE this
/// bracket, so a per-spend-credential host CAN drain its offline queue — one row
/// per prompt (the row's own FR-17 binding rides the seed pull, so one staged
/// credential serves exactly one row; a batch affordance loops brackets). Serve
/// that call and `walletOfflineQueueSupportedProvider = true` becomes honest;
/// leave it false while you don't, so the package never advertises a queue that
/// cannot drain.
///
/// [WalletSpendKind.swapDeposit] is NOT deferred (FR-23-a): the deposit signs
/// INSIDE this bracket, at execute — so a per-spend-credential (host-custody)
/// wallet CAN swap. The credential just needs to stay staged for the duration of
/// the `action()` (the bridge call that signs), exactly like an interactive send.
abstract interface class WalletSendAuthorizer {
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  );
}

/// The default: no host authorization step — run the signing call directly.
/// Correct for sealed-keychain custody, where the SDK holds the signing
/// capability for the wallet's whole open lifetime.
final class WalletPassthroughSendAuthorizer implements WalletSendAuthorizer {
  const WalletPassthroughSendAuthorizer();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) => action();
}
