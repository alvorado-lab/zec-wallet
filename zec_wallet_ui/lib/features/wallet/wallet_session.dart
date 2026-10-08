import 'dart:typed_data' show Uint8List;

import 'package:zec_wallet/zec_wallet.dart';

import 'send/wallet_send_request.dart';

/// The ONE seam between the rendering layer and the wallet FFI handle.
///
/// WHY a Dart-side port (design invariant 2 — hexagonal; flutter-patterns
/// § State Management): the live-sync providers must be unit-testable on
/// the host VM with NO native library and NO device (a real [WalletHandle]
/// is an opaque FRB object that cannot be constructed in a `flutter test`).
/// The providers depend on THIS interface; tests inject a fake, and the
/// future onboarding slice injects the real adapter by overriding
/// [walletSessionProvider]. This is NOT a Dart-side data cache — Rust stays
/// the single source of truth (design invariant 1); every method forwards
/// straight to the core.
///
/// Surface kept minimal — exactly what the host UI drives. Key-material
/// methods (`restore`, `revealMnemonic`) deliberately live OUTSIDE this
/// port: they are separate, crypto-reviewed key-crossing slices and the
/// sync/balance UI never touches them.
abstract interface class WalletSession {
  /// The live `SyncStatus` stream (spec §3.3). Emits the CURRENT status
  /// immediately on subscribe (so a re-subscribe after a background gap
  /// re-renders at once), then every change coalesced latest-wins. It NEVER
  /// completes on a transient fault — a stall is a `SyncStatus.stalled`
  /// EVENT, not an error or EOF.
  Stream<SyncStatus> watchSyncStatus();

  /// The on-resume cold snapshot (spec §3.3): balance, sync status, Tor
  /// state, chain tip, balance age, and the monotonic `seq`. Cheap; safe to
  /// call on resume before re-subscribing to the live stream.
  Future<WalletState> snapshot();

  /// The incoming-funds event stream (spec §3.3 / ADR-0536, amended by ADR-0539 —
  /// the FR-1 "funds arrived" hook). ONE `replay` event first (arrivals already
  /// in history strictly above [sinceCursor]; a null cursor gets a count-0
  /// baseline that hands the subscriber a cursor to keep), then a `live` event
  /// per scan batch that detected arrivals, and a `memoRefresh` nudge when a
  /// later enhancement pass changes how existing rows read — either decrypted
  /// tx data landed, or a transaction's chain status did, so `memoRefresh` means
  /// "re-pull", NEVER "new memo data exists". The payload is counts + heights + an opaque cursor
  /// ONLY (safe to log or forward); details are a pull via [transactions].
  /// Delivery is at-least-once with latest-wins coalescing —
  /// `totalTxDetected` is the coalesce-proof monotonic. A rescan/restore
  /// REPLAYS history with old spans: the stream is a freshness signal, never
  /// an accounting ledger. A malformed [sinceCursor] surfaces as a stream
  /// error (typed `storeCorrupt`) — including a [HistoryPage.nextCursor]
  /// keyset token passed here by mistake (the two cursor families are
  /// deliberately incompatible). NOTIFICATION-driving hosts: gate on
  /// `spanToHeight > your own acked watermark` (the max you have told the
  /// user about — tracked on YOUR side; the cursor is only for
  /// [sinceCursor]) plus your own consumed-txid ledger for exactly-once.
  Stream<IncomingFundsEvent> watchIncomingFunds({String? sinceCursor});

  /// The wallet's RECOVERABLE one-time-address funds (2e-2b) — amounts sitting on
  /// wallet-controlled single-use (ephemeral) transparent addresses: an expired
  /// TEX forward whose second step never completed, OR an exchange that returned a
  /// deposit to such an address. Each entry is AMOUNT-ONLY — the one-time address
  /// is wallet-internal and NEVER crosses the bridge (§5.4 never-render). The
  /// amount is a SUBSET of `BalanceSnapshot.transparentZat`/`.totalZat` (the engine
  /// already folds these outputs into the displayed balance), so the host renders
  /// it as "X OF your balance is on a one-time address", NEVER "+X" (that would
  /// over-count holdings ~2×). Cheap, LOCAL (a SQLite read), no network, no money
  /// movement. LIVE since gate-removal (2e-2b-v-5a) — empty only on a wallet that
  /// has stranded nothing on a one-time address.
  Future<List<RecoverableEphemeralFunds>> recoverableEphemeralFunds();

  /// MANUALLY recover funds stranded on wallet-controlled one-time (ephemeral)
  /// transparent addresses into the wallet's OWN shielded balance (2e-2b-v-2) — the
  /// "recover now" action behind the [recoverableEphemeralFunds] note, PLUS late
  /// exchange returns / a 2nd deposit the automatic surface can't see. MONEY-MOVING:
  /// it signs + broadcasts one consolidating tx per funded address (each over its own
  /// circuit, never co-spent — so a recovery never links the one-time-address set
  /// on-chain). The returned [EphemeralSweepSummary] is COUNTS-only (the one-time
  /// addresses NEVER cross the bridge); `recoveredZat` is PROVISIONAL (accepted ≠
  /// mined — render "pending" until the scan settles it) and IDEMPOTENT on re-run
  /// (the engine excludes already-spent UTXOs, so it can never double-spend);
  /// `failed > 0` ⇒ some funds stay on-chain + re-runnable; `truncated > 0` ⇒ a heavy
  /// wallet hit the per-invocation cap — run again. Throws a typed `WalletApiError`
  /// (`seedRequired` when the wallet cannot sign, invalid-state on a closed handle).
  /// Gate the affordance on a SUCCESSFUL [recoverableEphemeralFunds] read showing
  /// funds — NEVER offer it from an error/empty fallback (a failed read must not read
  /// as "nothing to recover", hiding held funds). LIVE since gate-removal
  /// (2e-2b-v-5a): a no-op only when no one-time address holds funds.
  Future<EphemeralSweepSummary> sweepEphemeralFunds();

  /// REOPEN a one-time-address (TEX) send window bricked by leaked reservations (#315
  /// slice 2) — sends that reserved a one-time address but never confirmed, which the
  /// wallet cannot free on its own. Self-mints a small amount from your SHIELDED balance
  /// to the highest provably-abandoned one-time address; mining it reopens the whole
  /// window. MONEY-MOVING + EXPLICIT: authorize it (the #327 seam) with the honest-cost
  /// disclosure (the mint + a later recovery are TWO transactions, ~4 network fees; the
  /// moved principal returns to your wallet via [sweepEphemeralFunds]). Returns a
  /// [ReclaimOutcome]: `Minted` (INITIATED — the window reopens once it confirms),
  /// `NothingToReclaim` (no abandoned reservation — the window is transient), or
  /// `NotBroadcast` (money-safe transport miss; retry). It unblocks the WINDOW only — it
  /// NEVER re-sends a paused/queued payment (no double-pay). ⚠ HOST GATING: offer it ONLY
  /// when a send is actually parked/paused — the SDK cannot tell a bricked window from a
  /// healthy one (no engine occupancy read), so a call otherwise mints uselessly (fee-waste,
  /// never fund-loss). Throws a typed `WalletApiError` (`seedRequired` with no key,
  /// `insufficientFunds` when the shielded balance can't fund the mint, invalid-state on a
  /// closed handle).
  Future<ReclaimOutcome> reclaimEphemeralSlots();

  /// EVERY queued send that has not completed yet (2e-2b-v-3, widened by #331):
  /// a one-time-address (TEX) two-step awaiting drain / ceiling-parked, or a
  /// plain single-step send waiting in the offline queue — the [ParkedSend.kind]
  /// says which (presentation-only). They sit in the queue with NO on-chain
  /// transaction (INVISIBLE in the activity list), so across an app relaunch
  /// this surface is the ONLY place the committed spend exists — hidden, a
  /// queued send is a DOUBLE-PAY window (the user re-enters it and both drain
  /// on reconnect). Each [ParkedSend] is amount + id + kind + createdAt; the
  /// recipient address NEVER crosses the bridge (§5.4 never-render). A
  /// read-only PULL; each parked send auto-broadcasts once it can drain —
  /// present it per the DOUBLE-PAY caution (never invite a re-send) and
  /// EARMARK its amount over the balance (it stays spendable). Cheap, LOCAL,
  /// no network. LIVE since gate-removal (2e-2b-v-5a): both the interactive
  /// send and `queueSend` paths can park here.
  Future<List<ParkedSend>> listParkedSends();

  /// The IN-FLIGHT two-step (TEX) sends (#309): first leg signed + broadcast, send not yet
  /// complete — money is IN MOTION through a wallet-controlled one-time address. Drives the
  /// DURABLE wallet-screen "on its way — don't send it again" cue (the post-send result screen's
  /// caution is dismissible; without this, an in-flight two-step reads as an ordinary pending
  /// send exactly when a worried user is most tempted to re-pay). DISJOINT from
  /// [listParkedSends] (parked = queued, nothing signed; in-flight = signed + broadcast). A row
  /// leaves by three exits: completion, the strand transition ([recoverableEphemeralFunds] takes
  /// over — the two can OVERLAP while the unshield settles; both cues together is correct), or a
  /// return to the queue (every tx expired unmined — no money moved; it reappears parked,
  /// cancellable again). AMOUNT-only (§5.4 — no recipient/txid); the amount annotates the SAME pending payment
  /// the activity list shows — never double-count it. Cheap, LOCAL, no network; empty is the
  /// common case.
  Future<List<InFlightSend>> listInFlightSends();

  /// CANCEL a parked (queued) send (2e-2b-v-4a) — the user-facing escape hatch to
  /// DISCARD a parked send of EITHER kind (the SAFE counter-affordance; never a
  /// re-send; the guard is kind-agnostic — any still-`Queued` row is fundless). Pass
  /// BOTH the `id` AND the `createdAt` from the [ParkedSend] the user is cancelling.
  /// Returns `true` if removed, `false` if it was already gone / began sending / its
  /// rowid was reused (idempotent). On `false`, re-read [listParkedSends] — but do NOT
  /// present it as definitively "cancelled" nor invite a re-send: a send that began
  /// sending may be IN-FLIGHT (direct the user to the activity list; a re-send risks a
  /// DOUBLE-PAY). MONEY-SAFE: only a still-`Queued` intent is deleted (nothing signed,
  /// no tx on-chain). IRREVERSIBLE — the host MUST confirm before calling. Throws a
  /// typed `WalletApiError` (invalid-state on a closed handle).
  Future<bool> cancelParkedSend({required int id, required int createdAt});

  /// RETRY a `paused` parked send (#315 slice 1) — resume the SAME queued intent after
  /// the wallet gave up auto-retrying it ([ParkedSend.paused]): retries of a
  /// one-time-address send are capped because each one permanently uses up one of a
  /// small number of address slots. When the user believes conditions changed (back
  /// online, the recipient service reachable), THIS is the resume path — never cancel +
  /// re-enter (double-pay risk + it silently evades the retry cap on a fresh send).
  /// Pass BOTH the `id` AND the `createdAt` from the [ParkedSend]. Returns `true` if
  /// re-armed (it attempts again on the next background pass — not instantly), `false`
  /// if already gone / began sending / rowid reused (idempotent — re-read
  /// [listParkedSends]). MONEY-SAFE: only re-arms the existing intent's retry budget;
  /// nothing is signed or sent by the call itself. Throws a typed `WalletApiError`
  /// (invalid-state on a closed handle).
  Future<bool> retryParkedSend({required int id, required int createdAt});

  /// AUTHORIZE a parked send NOW (FR-23-b / #361) — sign a send the user already
  /// committed, at this user-present moment, instead of waiting for a background
  /// pass to sign it. Pass BOTH the `id` AND the `createdAt` from the [ParkedSend].
  ///
  /// **The package always calls this INSIDE the `WalletSendAuthorizer.authorizeSpend`
  /// bracket**, so a host that stages a per-spend credential has it staged when the
  /// SDK pulls the seed. That is what makes an offline queue drainable at host
  /// custody at all: a background drain has no live user, so it cannot sign — see
  /// `walletOfflineQueueSupportedProvider`. A host with a BOUND native seed port
  /// stages for THIS row's [ParkedSend.binding] (FR-17); a stage recorded for any
  /// other row is refused and the send stays parked (funds safe).
  ///
  /// ONE ROW PER CALL, ONE CALL PER BRACKET — the row's binding rides the seed pull,
  /// so one staged credential serves exactly one row; an "authorize all" affordance
  /// LOOPS brackets, it never wraps N rows in one.
  ///
  /// Distinct from [retryParkedSend], which re-arms a `paused` row's retry budget and
  /// signs NOTHING. They compose: re-arm first, then authorize.
  ///
  /// MONEY-SAFE: it moves only the TIMING of the signature — the payment was
  /// committed when the user queued it. Read [ParkedAuthorization]'s host contract
  /// before writing copy: only `signed` may be phrased as "sending now", and
  /// `stillQueued` is NOT a failure (phrasing it as one invites a re-entry, which is
  /// a DOUBLE-PAY). Throws a typed `WalletApiError` — invalid-state on a closed
  /// handle, watch-only on a structurally seedless wallet, and a seed-required /
  /// seed-mismatch when the credential was not staged (or was staged for another row).
  Future<ParkedAuthorization> authorizeParkedSend({
    required int id,
    required int createdAt,
  });

  /// The transaction history for the activity list (FR-1), newest first and
  /// paginated by an OPAQUE keyset cursor: pending (unmined) rows sort above
  /// confirmed ones; `after` is `null` for the first page or the previous
  /// [HistoryPage.nextCursor] for the following page (passed back VERBATIM — never
  /// parse it); `limit` caps the page (clamped Rust-side). The keyset cursor means a
  /// page boundary never drops a same-height row nor strands the confirmed history
  /// behind a full page of pending txs. Each [TxSummary] carries the SIGNED net
  /// amount, status/confirmations, fee, and a memo flag — DISPLAY data only (no key
  /// material; §5.4 NEVER-LOG: txids/amounts/cursors). Reads librustzcash's
  /// `v_transactions` view off the sync lock, so it's cheap and never blocks scanning.
  Future<HistoryPage> transactions({required int limit, String? after});

  /// Is this a WATCH-ONLY wallet (#397 §3.7 D4/D5)? The reference UI keys its
  /// chrome on this: a "Watch-only" header badge, hidden Send/Shield/Swap
  /// affordances, and the Security screen's no-backup ("Export viewing key")
  /// arm. The typed [WalletErrorKind.watchOnly] refusals stand at the SDK
  /// regardless, so a host ignoring this only ever sees honest errors — never
  /// a wrong behavior. Cheap, local, no network, no key material.
  Future<bool> isWatchOnly();

  /// EXPORT this wallet's Unified Full Viewing Key (#397 §3.7 D1 / ADR-0538) — the
  /// `uview…` string that grants FULL history visibility (every past and future
  /// transaction, amounts, memos) and NO spend authority.
  ///
  /// Why it lives on the SESSION port and not only on the provisioner (the G1 port
  /// asymmetry, #361 companion): [isWatchOnly] is here, so a session-only host gets
  /// the whole watch-only CHROME for free — but without this it could not offer the
  /// EXPORT the chrome points at, and would have to reach around the port to the raw
  /// bridge handle and re-implement the §3.7 D9 gate itself. A capability the package
  /// renders must be a capability the package's own port can serve.
  ///
  /// EGRESS CONTRACT — this is the ONE sanctioned viewing-key egress. A host calling
  /// it directly MUST keep the same envelope the package's export screen provides:
  /// re-authorize first (`WalletRevealAuthorizer` — the §3.7 D9 backup-grade gate),
  /// warn BEFORE revealing that sharing is effectively irreversible, block screenshots
  /// while it is on screen, and hold the string for the shortest possible scope. §5.4
  /// NEVER-LOG: display / copy / QR it, never write it to a log or crash report.
  /// Throws a typed `WalletApiError` (invalid-state on a closed handle).
  Future<String> exportUfvk();

  /// The wallet's current receive address (the unified address for account 0) —
  /// what the user shares to RECEIVE ZEC. NOT key material (it is public by
  /// design; the spending key never leaves Rust), so it belongs on this port.
  /// §5.4 NEVER-LOG applies (an address is a never-log value) — display it,
  /// allow copy, but never write it to a log. Cheap, local, no network.
  Future<String> currentAddress();

  /// The wallet's TRANSPARENT receive address (Recv-2 / ADR-0528) — the account's
  /// external-scope P2PKH t-address, surfaced ALONGSIDE the shielded UA behind the
  /// receive screen's address-type toggle (default shielded). PUBLIC + visible
  /// on-chain + reused-address-linkable: the UI labels it as such and keeps the
  /// shielded UA the recommended default. NOT key material (the spending key never
  /// leaves Rust); §5.4 NEVER-LOG applies. Cheap, local, no network.
  Future<String> currentTransparentAddress();

  /// Mint the NEXT public diversified receive address (FR-8 / Recv-4, ADR-0537):
  /// a fresh unlinkable unified address for a contact/invoice that still credits
  /// this one wallet — payments to it are detected by the normal shielded scan
  /// (including after a seed-only restore), with no extra host work. Same
  /// receiver set as [currentAddress] (shielded-only), so no compatibility
  /// downgrade. Every call returns a NEW address from the SDK's never-recycle,
  /// restore-surviving counter; deterministic per
  /// [WalletMintedAddress.diversifierIndex] — keep the index as the durable
  /// host-side attribution key. Cheap, local, no network, not gated on sync.
  /// Throws the typed bridge error before an account is provisioned (exotic —
  /// accounts import eagerly at create). §5.4 NEVER-LOG applies to BOTH fields.
  Future<WalletMintedAddress> mintDiversifiedAddress();

  /// The account's birthday height — the scan FLOOR the wallet's history
  /// starts at (#317). `null` before the account is provisioned (the first
  /// sync hasn't run yet). The rescan sheet uses it as the range DEFAULT: a
  /// rescan from the floor covers everything THIS wallet has ever seen
  /// without hiding older funds (an above-floor rebuild hides the span
  /// between the old and new floor — the SDK's LOWER-ONLY contract) and
  /// without scanning pointlessly before the wallet existed. NOT a clamp on
  /// user picks: an explicitly EARLIER date is the post-restore recovery path
  /// (this floor may itself be a too-recent restore estimate). A height,
  /// never money. Cheap, local, no network.
  Future<int?> birthdayHeight();

  /// Start the background sync loop — required for [watchSyncStatus] to
  /// advance past `idle`. Idempotent.
  Future<void> startSync();

  /// Stop the background sync loop. Idempotent; durable progress is kept
  /// (the chain is the source of truth). Pauses sync WITHOUT closing.
  Future<void> stopSync();

  // --- Send pipeline (inc-2d-ui) ---------------------------------------------
  //
  // WHY these live on THIS port (and `revealMnemonic` does NOT): send carries NO
  // key material across the bridge — `propose`/`queueSend` take a ZIP-321 URI
  // String, `send` takes an opaque integer token; spending keys never leave Rust
  // (spec §3.2h). The port's key-material exclusion is about SEED/KEY crossings,
  // which these are not. And the gate that exposes a `WalletSession` at all
  // (`walletSessionProvider` ⇒ a backed-up, OnboardingActive wallet) is exactly
  // the money-safety gate a send must sit behind, so a send screen is reachable
  // only from a confirmed wallet — the boundary is structural, not a convention.

  /// Classify a recipient address against the wallet's OWN network — its memo
  /// capability (the §5.1 SHIELDED-vs-TRANSPARENT / private-vs-public axis) —
  /// WITHOUT composing or proposing anything. Drives the send form's LIVE
  /// recipient feedback + memo gating. The screen never needs to know
  /// mainnet/testnet (the same network-hiding seam as [composePaymentUri], so a
  /// host-supplied network can never be a footgun). SYNCHRONOUS, local, NO
  /// network, no money movement — safe to call on every keystroke and fully
  /// offline (the same audited `Address::parse` gate `propose` lowers through,
  /// so the live check can never disagree with what a send will accept). Throws
  /// a typed `WalletApiError` (`addressInvalid` for malformed, `networkMismatch`
  /// for an other-network address) the host maps to an honest inline status.
  ValidatedAddress validateRecipient(String address);

  /// Compose a ZIP-321 payment URI (the §2.4 lossless request token) from one
  /// form leg — the form flow's bridge into [propose]/[queueSend]; a scanned-QR
  /// flow would call those with the URI directly. SYNCHRONOUS, no money movement:
  /// the adapter validates the recipient + memo against the wallet's OWN network
  /// (so a cross-network address is the SDK's typed `NetworkMismatch`, never a
  /// host-supplied-network footgun) and returns the URI string. The bridge
  /// crossing (`encodePaymentUri`) lives in the adapter — never in the UI layer —
  /// so this whole flow stays host-VM testable behind a fake. Throws a typed
  /// `WalletApiError` for a bad address / un-sendable memo.
  ///
  /// FR-28 — [memoBytes] attaches an OPAQUE machine memo (the ZIP-302 `0xFF`
  /// arm) instead of [memoText]. The two are mutually exclusive; the SDK never
  /// interprets the bytes, and the wire zero-pads anything under 511 bytes, so
  /// a host that needs its own length frames it INSIDE them.
  String composePaymentUri({
    required String recipient,
    required int amountZat,
    String? memoText,
    List<int>? memoBytes,
  });

  /// Parse a ZIP-321 `zcash:` payment URI (a scanned QR, a deep link — HOSTILE
  /// input) into a [WalletSendRequest] for the FR-25 prefilled-send seam — the
  /// DECODE inverse of [composePaymentUri]. SYNCHRONOUS, no money movement: the
  /// adapter runs it through the audited core `payment_uri` parser against the
  /// wallet's OWN network (so a cross-network URI is the SDK's typed
  /// `NetworkMismatch`, never a host-supplied-network footgun), and applies the
  /// single-recipient / text-memo seam policy. The bridge crossing
  /// (`parsePaymentUri`) lives in the adapter — never in the UI layer — so the
  /// prefill flow stays host-VM testable behind a fake. Throws a typed
  /// [WalletSendRequestException] (malformed / wrong-network / multi-leg /
  /// unsupported-memo) so the host rejects at the seam, never a half-filled
  /// form. [WalletSendRequest.fromUri] is the public sugar over this.
  WalletSendRequest parseSendRequest(String uri);

  /// Prepare a send from a ZIP-321 payment URI (the §2.4 lossless request token
  /// the host composes via [composePaymentUri], or a scanned QR). DETERMINISTIC,
  /// LOCAL: runs the audited note-selection + fee + change over the wallet DB and
  /// returns the numbers the user confirms before signing — no keys, no proofs,
  /// no network, no DB writes, nothing sent. The proposal stays an opaque,
  /// Rust-retained one-shot token; [send] consumes it by `proposalId`.
  ///
  /// Throws a typed `WalletApiError` (insufficient funds carrying the figures,
  /// proposal-stale, payment-URI-invalid, network-mismatch, …) the controller
  /// maps to an honest inline fault.
  Future<SendProposal> propose(String requestUri);

  /// Sign + broadcast a prepared proposal — consumes the one-shot `proposalId`.
  /// A per-tx broadcast outcome is first-class DATA (the returned list), NEVER a
  /// thrown error: a tx that fails to broadcast is persisted and re-sent by the
  /// resubmission machinery on the next sync, so even a total broadcast failure
  /// loses no funds, only immediacy. Throws only for a consumed token
  /// (`proposalAlreadyUsed`), a stale anchor (`proposalStale` ⇒ re-propose), or a
  /// build/sign failure (`signFailed`).
  Future<List<TxSubmitResult>> send(int proposalId);

  /// The wallet's DELIVERY OBLIGATION for one transaction it created (stage S8
  /// `obligation`) — the same reading a history row carries on
  /// [TxSummary.delivery], for the `txidHex` a [send] result named. Read it
  /// after a `send` that was not all-success: only [DeliveryState.retryPending]
  /// licenses "saved — your wallet will send it on a later sync"; `null` (not
  /// the wallet's, expired, or no row) and every other state do not.
  Future<DeliveryState?> deliveryState(String txidHex);

  /// Queue a send for OFFLINE-first delivery (invariant 4): durably persist the
  /// send INTENT (the same ZIP-321 URI) and return immediately — no network, no
  /// signing, no money movement. It survives a process kill and proposes → signs
  /// → broadcasts on the next online sync (proposing at SEND time, not now, is
  /// what stops a long-queued send from ever carrying a stale anchor). Returns
  /// the opaque queued-send id; throws `queuedSendsFull` at the durable cap (a
  /// real send is never silently dropped).
  Future<String> queueSend(String requestUri);

  /// FR-27 — the machine-memo bytes of one transaction, scoped to the prefixes
  /// registered on the wallet's `WalletConfig`. Returns them in output order;
  /// `txidHex` is passed back verbatim from a history row.
  ///
  /// **These bytes are attacker-controllable** — they came off a public chain,
  /// and anyone can put anything in a memo attached to a payment they send you.
  /// **Prefix matching is not a security boundary**: forging a prefix is free,
  /// so the scope cuts volume, not intent. Parse with something that fails
  /// closed. Throws `machineMemoScopeInvalid` when no prefix is registered (the
  /// verb is opt-in and CLOSED by default — an empty list would read as "this
  /// transaction carries nothing" over a memo that is there) and `txidInvalid`
  /// for a malformed txid.
  ///
  /// **On the port deliberately, and it was NOT at first.** FR-27 shipped with
  /// the verb on `WalletHandle` only, on the stated grounds that a port method
  /// would need a memo consumer that did not exist. It exists now — the
  /// example app's round-trip diagnostic, which is the SDK's own second
  /// consumer and the thing that makes FR-27's `Universal:` claim compiled
  /// rather than asserted. The condition was met, so the decision moved.
  Future<List<Uint8List>> machineMemos(String txidHex);

  /// Propose shielding the wallet's detected TRANSPARENT funds into its shielded
  /// pool (Recv-3 — the companion to [currentTransparentAddress]). Exchange
  /// withdrawals + swap-in deliveries land transparent; this is the
  /// privacy-POSITIVE "move them shielded" action. DETERMINISTIC, LOCAL like
  /// [propose]: no keys, no proofs, no network, no DB writes — NOTHING is sent.
  ///
  /// Returns a [SendProposal] (`isShield == true`) when there is ≥ the 0.001-ZEC
  /// shielding threshold to shield — confirm it, then pass its `proposalId` to
  /// [send] (the SAME one-shot token the send path consumes; the shield reuses
  /// the send pipeline whole). Returns `null` when there is nothing worth
  /// shielding yet (below the threshold or no transparent funds) — the host hides
  /// the "shield now" action. Throws only a typed `WalletApiError` (proposal-stale
  /// ⇒ wait for sync, invalid-state on a closed handle). NO key material crosses.
  Future<SendProposal?> proposeShield();

  // --- Swap on-ramp (D-2) ----------------------------------------------------
  //
  // WHY these live on THIS port: swap carries NO wallet key material across the
  // bridge — `swapQuote`/`swapExecute` take/return swap DTOs + an opaque id
  // String, the spending keys never leave Rust (spec §3.2h/§3.5). The `jwt` in
  // [enableNearSwap]'s config is a PROVIDER credential (not wallet key material);
  // the SDK wraps it `Zeroizing` at the boundary and it is never logged. The same
  // `walletSessionProvider` money-safety gate (a backed-up, OnboardingActive
  // wallet) sits in front of the whole surface, so a swap is reachable only from a
  // confirmed wallet. `swap == null` (no provider attached) is a Dart-side concern
  // the host knows from whether it called [enableNearSwap]; the SDK reports it as
  // a typed `SwapDisabled` if a swap op is attempted regardless (honest, never a
  // silent no-op).

  /// Turn the swap on-ramp ON — construct the NEAR provider over the wallet's OWN
  /// transport (the same host-provided dialer + Tor policy sync uses; internal Tor
  /// is OPTIONAL — the host may bring its own) and apply the §3.5 kill state. A
  /// ONE-TIME setup, called after the wallet is open (set-once, first-wins). On a
  /// build compiled WITHOUT the adapter it degrades honestly to a typed
  /// `SwapDisabled` (the method is always present so the surface is stable). Throws
  /// a typed `SwapApiError` (`watchOnly` on a watch-only wallet — STRUCTURAL,
  /// never retryable, checked first (#397 §3.7 D3): render view-only framing,
  /// not a retry; `providerUnavailable` when the transport can't be resolved —
  /// retryable; `swapStateUnavailable` on a torn-down handle).
  Future<void> enableNearSwap({
    required SwapProviderConfig config,
    required bool swapEnabled,
    SwapKill? declaredKill,
  });

  /// Request a bounds-checked swap quote (spec §2.6) — the FIRST step of the
  /// on-ramp. DETERMINISTIC against the user's exact side; funds NEVER move. The
  /// returned [SwapQuote] carries the deposit address, the min-out floor, the ZEC
  /// side in zatoshis, and the §2.6 privacy `disclosure` the host MUST render.
  /// Throws a typed `SwapApiError` (`slippageToleranceTooHigh`, `quoteOutOfBounds`,
  /// `destinationInvalid`, `providerUnavailable`, `swapDisabled`, …) the controller
  /// maps to an honest inline fault.
  Future<SwapQuote> swapQuote({required QuoteRequest request});

  /// The dynamic source-asset list for the IntoZec picker (spec §3.3b D5/L6).
  /// LAZY — called when the user opens the IntoZec form (NOT on launch: §5.2
  /// honest-off, an idle user emits no token traffic). The SDK fetches `/v0/tokens`
  /// over the swap dialer (Tor-optional, its OWN circuit), filters the ZEC asset +
  /// $0/null-price entries, and caches the result. On a fetch fault it serves the
  /// LAST good list with [SwapTokenList.fresh] = false (the host shows a "couldn't
  /// refresh, showing cached" banner — honest degradation, never a blank picker);
  /// a first-ever fault with no cache throws a typed `SwapApiError`. An empty
  /// `tokens` with `fresh == true` is the honest "no assets available right now".
  /// No wallet key material crosses — token metadata is §5.4 never-logged.
  Future<SwapTokenList> swapListTokens();

  /// Execute a quote — register the swap intent with the provider and (for an
  /// OutOfZec quote) queue the §4.4 ZEC deposit send. Pass back the [SwapQuote]
  /// [swapQuote] returned; the SDK claims its own durable single-flight by the
  /// quote id BEFORE anything happens (idempotent against a double-tap / crash
  /// retry — exactly ONE deposit per quote), and the deposit uses the SDK's own
  /// frozen record, NEVER this DTO, so a tampered field can't redirect funds.
  /// Returns the opaque provider swap id (do NOT parse it; pass it to
  /// [watchSwapStatus]). Throws a typed `SwapApiError` (`quoteExpired` ⇒
  /// re-quote — since #367 this also covers a not-issued/already-executed
  /// quote (every take-miss reads "no longer valid"); `swapStateBusy` ⇒ retry
  /// the SAME execute (nothing consumed); `depositSendFailed` ⇒ re-quote).
  Future<String> swapExecute({required SwapQuote quote});

  /// The live `SwapStatus` stream for one in-flight swap (spec §3.3/§7). Emits the
  /// CURRENT status immediately on subscribe (so a re-subscribe after a background
  /// gap re-renders at once), then polls at a bounded cadence. It NEVER completes
  /// on a transient provider fault (a stall is retried behind the scenes, not a
  /// dead stream); it completes only on a TERMINAL status (success/refunded/failed)
  /// — drained first — or when the wallet is closed/swap is hard-killed. [swapId]
  /// is the opaque id [swapExecute] returned.
  Stream<SwapStatus> watchSwapStatus({required String swapId});

  /// The durable IN-FLIGHT SWAP surface (W-swap-5, #366): every swap whose order
  /// was registered at execute and has neither been dismissed nor self-lapsed.
  /// THE kill→relaunch re-attach — after a process restart the wallet screen
  /// lists these and re-opens live tracking by feeding [SwapRecord.id] to
  /// [watchSwapStatus]. NOT gated on swap being enabled or the §3.5 kill (a
  /// local read is not swap traffic — the user never loses sight of money in
  /// motion). Read-only, cheap + LOCAL; called on the same edges as the
  /// parked/in-flight send surfaces. §5.4: [SwapRecord.id] is render-never-log.
  Future<List<SwapRecord>> listInFlightSwaps();

  /// Pin an OBSERVED terminal outcome into an in-flight swap record (#367) —
  /// call when the tracking stream reports a TERMINAL status, so the home row
  /// renders the truth even if the user was away at observation time (deleting
  /// on observation erased outcomes the user never saw — durably, if the
  /// process died in the gap). First-wins + idempotent (`false` for an
  /// absent / lapsed / already-pinned id). Display-only state.
  Future<bool> recordSwapOutcome({
    required String swapId,
    required SwapOutcome outcome,
  });

  /// Remove one in-flight swap record. USER-INTENT only since #367 — the
  /// terminal card's Done (the outcome rendered = seen) or the home row's
  /// explicit Remove; a terminal OBSERVATION pins via [recordSwapOutcome]
  /// instead. Display-only state: never touches the deposit outbox / guard /
  /// detection. Idempotent (`false` for an already-absent id — a raced
  /// double-dismiss is harmless).
  Future<bool> dismissSwapRecord({required String swapId});

  /// #390 — "Check older swap addresses": widen the range the wallet watches so
  /// the normal sync surfaces older swap deposits/refunds that a SEED-ONLY
  /// RESTORE of a long swap history left unchecked. It does NOT itself find funds
  /// — it WIDENS, then any older swap money appears in the balance over the next
  /// minutes of sync. Returns a COUNTS-only [SwapAddressCheckReport] (never an
  /// address/index) — render "checking as your wallet syncs; anything found will
  /// appear in your balance", NEVER "found X". Rerunnable (each accepted run goes
  /// deeper). Moves NO money (no authorizer). Throws a typed `WalletApiError`
  /// [`swapAddressCheckRefused`] (branch on `reason`: `swapDisabled` — swap is
  /// off; `checkOutstanding` — a prior check is still running, one per settlement
  /// window) or invalid-state on a closed handle.
  Future<SwapAddressCheckReport> checkOlderSwapAddresses();

  /// #390 — the render-only coverage read for the "Check older swap addresses"
  /// sheet: how far the wallet has already checked (`coveredSwaps`) and how many
  /// addresses are still to be registered + polled (`pending`; `0` = nothing
  /// outstanding). COUNTS-only. Read-only, side-effect-free — safe to re-pull
  /// while the sheet is open. Throws invalid-state on a closed handle.
  Future<SwapAddressCoverage> swapAddressCheckCoverage();

  /// The sync servers the host OFFERS (the picker, P3-13 — `WalletConfig.
  /// syncServers` as validated), WITHOUT their keys (`authValue` is always
  /// null here; `authHeader` says whether an entry is gated). Read-only.
  Future<List<SyncServer>> syncServers();

  /// Which server the wallet dials, which remembered choice produced it, and
  /// whether a fallback is in force — the connection's own truth, which is
  /// what the sync sheet's Server row renders (host only, never the URL).
  Future<SyncServerStatus> syncServerStatus();

  /// Dial [choice] under the wallet's own Tor policy and ask the server who it
  /// is (one round trip, 15 s budget). Throws typed —
  /// `WalletErrorKind.syncServerUnreachable`, `.networkMismatch`,
  /// `.syncServerNotOffered`, `.invalidEndpoint` (a custom URL the door
  /// refuses) — and NEVER switches: the picker's "Check server" step. The
  /// switch itself is the provisioner's (`WalletProvisioner.switchSyncServer`)
  /// because it swaps the session.
  Future<SyncServerProbe> probeSyncServer(SyncServerChoice choice);
}

/// A freshly minted public diversified receive address (FR-8 / Recv-4) — the
/// port-level value the [WalletSession.mintDiversifiedAddress] seam returns,
/// bridge-agnostic (the [WalletSendRequest] pattern: pure Dart, no FRB type
/// crosses the port). A VALUE CARRIER, not an entity: no `==` override —
/// compare fields, not instances. Declared AFTER the interface so the port's
/// own doc comment stays attached to it.
class WalletMintedAddress {
  const WalletMintedAddress({
    required this.address,
    required this.diversifierIndex,
  });

  /// The canonical encoded unified address — render/copy/QR it exactly like
  /// [WalletSession.currentAddress]. §5.4 NEVER-LOG.
  final String address;

  /// The ZIP-32 diversifier index the address derives at — the durable
  /// HOST-SIDE attribution key (same seed + same index ⇒ the same address,
  /// forever). WALLET-INTERNAL data: it is a sequential mint ordinal, so
  /// sharing it with a counterparty (or embedding it in an outward-facing
  /// invoice identifier) leaks mint count and lets counterparties link their
  /// addresses to one wallet. A plain `int` is exact on EVERY platform: the
  /// SDK's frozen public-receive region tops out below 2^41, far under the
  /// web's 2^53 safe-integer bound. §5.4 NEVER-LOG.
  final int diversifierIndex;
}
