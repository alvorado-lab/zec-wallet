import 'dart:async';
import 'dart:typed_data';

import 'package:zec_wallet_ui/features/wallet/send/wallet_send_request.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// A host-VM fake for [WalletSession] — no native library, no device. It
/// mirrors the real `watchSyncStatus` contract (CURRENT status emitted on
/// subscribe) and gives tests direct handles to drive the live stream
/// (push / error / complete) and to count subscribes, cancels, snapshots,
/// and start/stop calls.
class FakeWalletSession implements WalletSession {
  FakeWalletSession({
    SyncStatus current = const SyncStatus.idle(),
    WalletState? snapshotValue,
    this.failOnSubscribe = false,
    this.dropAfterReplay = false,
    this.snapshotThrows = false,
  }) : _current = current,
       _snapshot = snapshotValue ?? walletStateFixture(syncStatus: current);

  SyncStatus _current;
  WalletState _snapshot;

  /// When true, every subscription immediately errors instead of emitting —
  /// a persistently failing endpoint, for the reconnect-backoff test.
  bool failOnSubscribe;

  /// When true, every subscription delivers its current-first replay THEN
  /// immediately errors — a flapping link that connects, replays, and drops.
  /// Used to prove the replay alone does NOT reset the reconnect backoff.
  bool dropAfterReplay;

  /// When true, [snapshot] throws — a cold read failing (e.g. a busy DB).
  bool snapshotThrows;

  /// When true, [startSync] throws — the (rare) sync-loop start command failing
  /// (e.g. the handle closed underneath us). Drives the drive-failed path.
  bool failStart = false;

  /// When true, [stopSync] throws — a (rarer) stop failing. Used to prove a
  /// stop failure does NOT masquerade as a start failure (the loop is still up).
  bool failStop = false;

  int subscribeCount = 0;
  int cancelCount = 0;
  int snapshotCount = 0;
  int startCount = 0;
  int stopCount = 0;

  // --- Send pipeline (inc-2d-ui) injection ----------------------------------
  /// Returned by [propose] unless [proposeThrows] is set; defaults to a simple
  /// shielded proposal.
  SendProposal? proposeResult;

  /// When set, [propose] throws this (e.g. a typed `WalletApiError`).
  Object? proposeThrows;

  /// When set, [propose] PARKS (after counting/recording) until this gate
  /// completes — the mid-flight interleaving harness (#330): a test can swap
  /// the wallet session (or fire a second action) while a propose is in
  /// flight, then release it and assert the stale continuation writes nothing.
  Completer<void>? proposeGate;

  /// Returned by [send] unless [sendThrows] is set; defaults to one success.
  List<TxSubmitResult>? sendResults;

  /// When set, [send] throws this.
  Object? sendThrows;

  /// What [deliveryState] answers per txid (stage S8 `obligation`). A txid NOT
  /// listed reads [DeliveryState.retryPending] — the reading the core gives an
  /// interactive send whose broadcast failed (its persisted row IS the
  /// obligation), so the existing send-flow tests keep their meaning. A test of
  /// the negative lists the txid with `null` or another state, or sets
  /// [deliveryStateThrows].
  Map<String, DeliveryState?> deliveryStates = {};

  /// When set, [deliveryState] throws this.
  Object? deliveryStateThrows;

  /// How many times [deliveryState] was asked.
  int deliveryStateCount = 0;

  /// When set, [send] PARKS (after counting/recording) until this gate
  /// completes — the sibling of [proposeGate] for the sign+broadcast half. The
  /// harness for anything that has to happen WHILE a send is in flight: leaving
  /// the screen mid-submit, stacking a second entry over a running send, or
  /// releasing the landing afterwards to prove it wrote nothing it shouldn't.
  Completer<void>? sendGate;

  /// Returned by [queueSend] unless [queueThrows] is set.
  String queuedId = 'queued-1';

  /// When set, [queueSend] throws this.
  Object? queueThrows;

  /// Returned by [proposeShield]. `null` (the default) models "nothing to shield"
  /// (below threshold / no transparent funds); set a [shieldProposalFixture] to
  /// model a shieldable balance. Ignored when [proposeShieldThrows] is set.
  SendProposal? proposeShieldResult;

  /// When set, [proposeShield] throws this (e.g. a typed `proposalStale`).
  Object? proposeShieldThrows;

  /// When true, [proposeShield] returns a future that never completes — the
  /// wedged-FFI/blocking-pool-starvation edge the controller's timeout must convert
  /// into an honest error state rather than an infinite "Preparing…" spinner.
  bool proposeShieldNeverCompletes = false;

  /// When set, [proposeShield] PARKS until this gate completes — the
  /// mid-flight interleaving harness (#330), like [proposeGate].
  Completer<void>? proposeShieldGate;

  /// Drives [validateRecipient] in widget/unit tests. Default: a shielded,
  /// memo-capable recipient — so the existing send tests that enter the default
  /// recipient and tap Review keep a VALID (Review-enabled) recipient. Override
  /// to `ValidatedAddress(memoCapable: false)` to model a transparent recipient,
  /// or set [validateRecipientThrows] to a typed `WalletApiError`
  /// (`addressInvalid` / `networkMismatch`) to model a bad/wrong-network paste.
  ValidatedAddress validateRecipientResult = const ValidatedAddress(
    memoCapable: true,
  );

  /// When set, [validateRecipient] throws this (a malformed / other-network
  /// address — the real bridge throws a typed `WalletApiError`).
  Object? validateRecipientThrows;

  int validateRecipientCount = 0;
  String? lastValidatedRecipient;

  /// When set, [composePaymentUri] throws this (e.g. a typed `addressInvalid`).
  Object? composeThrows;

  /// Drives [parseSendRequest] (the FR-25 ZIP-321 prefill door). When set, the
  /// fake returns this canned request — a prefill/lock test drives the send
  /// screen with a KNOWN request, no native parser. Default `null` → the fake
  /// echoes a minimal request whose `address` is the raw URI (the default
  /// [validateRecipient] classifies any address valid, so the prefilled form
  /// is Review-enabled).
  WalletSendRequest? parseSendRequestResult;

  /// When set, [parseSendRequest] throws this — model a malformed /
  /// wrong-network / multi-leg / unsupported-memo URI (the real adapter throws a
  /// typed [WalletSendRequestException]).
  Object? parseSendRequestThrows;

  int parseSendRequestCount = 0;
  String? lastParsedUri;

  int composeCount = 0;
  int proposeCount = 0;
  int proposeShieldCount = 0;
  int sendCount = 0;
  int queueCount = 0;
  String? lastComposeRecipient;
  int? lastComposeAmountZat;
  String? lastComposeMemo;

  /// FR-28: the opaque machine-memo bytes the last compose was handed, copied
  /// verbatim. `null` when the leg carried none.
  List<int>? lastComposeMemoBytes;
  String? lastProposeUri;
  String? lastQueueUri;
  int? lastSendProposalId;

  // --- Swap on-ramp (D-2) injection -----------------------------------------
  /// When set, [enableNearSwap] throws this; else it records the config.
  Object? enableNearSwapThrows;
  SwapProviderConfig? lastEnableConfig;
  bool? lastEnableSwapEnabled;
  SwapKill? lastEnableDeclaredKill;

  /// Returned by [swapQuote] unless [swapQuoteThrows] is set.
  SwapQuote? swapQuoteResult;
  Object? swapQuoteThrows;
  QuoteRequest? lastSwapQuoteRequest;

  /// When true, [swapQuote] never completes — models a stalled dial so a test can
  /// drive the controller's host-side timeout (the "Getting a quote…" hang).
  bool swapQuoteNeverCompletes = false;

  /// When set, [swapQuote] PARKS until this gate completes — the mid-flight
  /// interleaving harness (#330), like [proposeGate].
  Completer<void>? swapQuoteGate;

  /// Returned by [swapExecute] unless [swapExecuteThrows] is set.
  String swapExecuteResult = 'swap-1';
  Object? swapExecuteThrows;
  SwapQuote? lastSwapExecuteQuote;

  /// When set, [swapExecute] PARKS until this gate completes — holds the
  /// screen in [SwapExecuting] to exercise the #367 execute pop-guard (the
  /// [swapQuoteGate] sibling).
  Completer<void>? swapExecuteGate;

  /// Returned by [swapListTokens] unless [swapListTokensThrows] is set.
  SwapTokenList? swapListTokensResult;
  Object? swapListTokensThrows;
  int swapListTokensCount = 0;

  /// The status a swap subscription replays on subscribe (current-first).
  SwapStatus swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 0);

  /// When true, every swap subscription immediately errors — an establish-time
  /// typed failure (SwapDisabled / closed handle / bad id), which the notifier
  /// surfaces and STOPS on (the core never routes a transient fault here).
  bool failSwapOnSubscribe = false;

  /// When true, the swap subscription connects but emits NOTHING — the
  /// real-world "re-attached to a provider-GC'd order the core retries
  /// forever" shape (the poll never terminates, the notifier never leaves
  /// loading). Drives the HIGH-1 escape test.
  bool holdSwapSubscribe = false;

  int enableNearSwapCount = 0;
  int swapQuoteCount = 0;
  int swapExecuteCount = 0;
  int swapSubscribeCount = 0;
  int swapCancelCount = 0;
  String? lastWatchedSwapId;

  StreamController<SwapStatus>? _activeSwap;

  bool get hasActiveSwapSubscription {
    final c = _activeSwap;
    return c != null && !c.isClosed;
  }

  /// Push a new live swap status to the active subscription (and make it the
  /// value a later subscribe replays).
  void pushSwap(SwapStatus status) {
    swapCurrent = status;
    _activeSwap?.add(status);
  }

  /// Complete the active swap stream (EOF) — the core ending the poll WITHOUT a
  /// terminal first (a Hard kill / teardown; the core never EOFs on a transport
  /// fault). The notifier must STOP on this, never reconnect.
  void completeSwap() => _activeSwap?.close();

  /// The CORE's terminal pattern: emit the terminal status, THEN close the
  /// stream (the poll loop emits the final status and ends). The notifier must
  /// read this as a clean end, NOT a reconnect.
  void pushSwapTerminal(SwapStatus status) {
    pushSwap(status);
    completeSwap();
  }

  StreamController<SyncStatus>? _active;

  bool get hasActiveSubscription {
    final c = _active;
    return c != null && !c.isClosed;
  }

  /// Push a new live status to the active subscription (and make it the
  /// value a later subscribe replays).
  void push(SyncStatus status) {
    _current = status;
    _active?.add(status);
  }

  /// Fault the active stream (transport drop / EOF-as-error).
  void emitError([Object error = 'fake transport drop']) {
    _active?.addError(error);
  }

  /// Complete the active stream (EOF).
  void complete() {
    _active?.close();
  }

  /// Set the cold-read state. Also moves [_current] so the two stay coherent —
  /// see [snapshot] for why the fake must not present a split the core cannot.
  void setSnapshot(WalletState state) {
    _snapshot = state;
    _current = state.syncStatus;
  }

  @override
  Stream<SyncStatus> watchSyncStatus() {
    subscribeCount++;
    final controller = StreamController<SyncStatus>();
    controller.onListen = () {
      if (failOnSubscribe) {
        controller.addError('fake subscribe failure');
      } else {
        controller.add(_current);
        if (dropAfterReplay) controller.addError('fake post-replay drop');
      }
    };
    controller.onCancel = () {
      cancelCount++;
      if (identical(_active, controller)) _active = null;
    };
    _active = controller;
    return controller.stream;
  }

  @override
  Future<WalletState> snapshot() async {
    snapshotCount++;
    if (snapshotThrows) throw StateError('fake snapshot failure');
    // ONE source of truth for the current status (#409 R1, arch review). The
    // real core cannot present a split here: `SyncController::status()` and
    // `subscribe()` both read the single `watch::Sender`, so a cold read and
    // the live stream always agree. This fake stored the status twice and
    // `push()` moved only the stream's copy, so a test could observe a
    // stream/cold-read divergence the product structurally cannot produce —
    // the very shape of the defect #409 R1 exists to fix (two stored copies of
    // one fact, one of which nothing drives). Production reads this path:
    // `walletSnapshotProvider` feeds the sync sheet's before-first-emit
    // fallback.
    return WalletState(
      balance: _snapshot.balance,
      syncStatus: _current,
      tor: _snapshot.tor,
      tip: _snapshot.tip,
      lastSynced: _snapshot.lastSynced,
      everSynced: _snapshot.everSynced,
      rescanRebuilding: _snapshot.rescanRebuilding,
      seq: _snapshot.seq,
    );
  }

  // --- Incoming-funds event stream (ADR-0536) -------------------------------

  StreamController<IncomingFundsEvent>? _activeIncoming;

  /// Subscription bookkeeping for the incoming stream (parallel to
  /// [subscribeCount]/[cancelCount], which stay sync-status-only).
  int incomingSubscribeCount = 0;
  int incomingCancelCount = 0;

  /// The `sinceCursor` values passed to [watchIncomingFunds], in call order —
  /// tests pin that a reconnect/resume re-subscribes WITH the last event's
  /// cursor (the catch-up contract), not from scratch.
  final List<String?> incomingCursorsSeen = [];

  /// When `true`, a subscribe faults immediately (the malformed-cursor typed
  /// reject / transport-drop shape — a stream ERROR, not a sync throw).
  bool failOnIncomingSubscribe = false;

  /// The `replay` event a new subscription emits first. Defaults to the
  /// count-0 baseline the core produces for a null cursor.
  IncomingFundsEvent incomingReplay = const IncomingFundsEvent(
    kind: IncomingFundsEventKind.replay,
    newTxCount: 0,
    totalTxDetected: 0,
    cursor: 'w1:0',
  );

  bool get hasActiveIncomingSubscription {
    final c = _activeIncoming;
    return c != null && !c.isClosed;
  }

  /// Push a live/memo-refresh event to the active incoming subscription.
  void pushIncoming(IncomingFundsEvent event) {
    _activeIncoming?.add(event);
  }

  /// Fault the active incoming stream (transport drop).
  void emitIncomingError([Object error = 'fake incoming drop']) {
    _activeIncoming?.addError(error);
  }

  /// Complete the active incoming stream (EOF — teardown shape).
  void completeIncoming() {
    _activeIncoming?.close();
  }

  @override
  Stream<IncomingFundsEvent> watchIncomingFunds({String? sinceCursor}) {
    incomingSubscribeCount++;
    incomingCursorsSeen.add(sinceCursor);
    final controller = StreamController<IncomingFundsEvent>();
    controller.onListen = () {
      if (failOnIncomingSubscribe) {
        controller.addError('fake incoming subscribe failure');
      } else {
        controller.add(incomingReplay);
      }
    };
    controller.onCancel = () {
      incomingCancelCount++;
      if (identical(_activeIncoming, controller)) _activeIncoming = null;
    };
    _activeIncoming = controller;
    return controller.stream;
  }

  // --- Recoverable one-time-address funds (2e-2b) injection -----------------
  /// The recoverable one-time-address (ephemeral) funds. Defaults to empty (the
  /// dormant production state); set entries to drive the balance card's subset
  /// row in a widget test.
  List<RecoverableEphemeralFunds> recoverableEphemeralFundsResult = const [];
  int recoverableEphemeralFundsCount = 0;

  @override
  Future<List<RecoverableEphemeralFunds>> recoverableEphemeralFunds() {
    recoverableEphemeralFundsCount++;
    if (recoverableEphemeralFundsThrows != null) {
      return Future<List<RecoverableEphemeralFunds>>.error(
        recoverableEphemeralFundsThrows!,
      );
    }
    return Future<List<RecoverableEphemeralFunds>>.value(
      recoverableEphemeralFundsResult,
    );
  }

  /// When set, [recoverableEphemeralFunds] errors with this — the failed-read edge
  /// the "recover now" CTA must gate on (a failed read must NOT read as "nothing to
  /// recover", hiding held funds).
  Object? recoverableEphemeralFundsThrows;

  // --- Sweep / parked-send (2e-2b-v) injection ------------------------------
  /// The summary [sweepEphemeralFunds] returns unless [sweepThrows] is set.
  /// Defaults to an empty no-op (the dormant production state); set a funded
  /// [ephemeralSweepSummaryFixture] to drive the recovery sheet's outcome.
  EphemeralSweepSummary sweepResult = const EphemeralSweepSummary(
    scanned: 0,
    swept: 0,
    recoveredZat: 0,
    failed: 0,
    truncated: 0,
  );
  Object? sweepThrows;

  /// When true, [sweepEphemeralFunds] never completes — the slow/stalled sweep
  /// the recover button's in-flight (disabled + spinner) state must hold through.
  bool sweepNeverCompletes = false;
  int sweepCount = 0;

  @override
  Future<EphemeralSweepSummary> sweepEphemeralFunds() {
    sweepCount++;
    if (sweepNeverCompletes) return Completer<EphemeralSweepSummary>().future;
    if (sweepThrows != null) {
      return Future<EphemeralSweepSummary>.error(sweepThrows!);
    }
    return Future<EphemeralSweepSummary>.value(sweepResult);
  }

  /// The outcome [reclaimEphemeralSlots] returns unless [reclaimThrows] is set
  /// (#315 slice 2). Defaults to `Minted` (the reopen-initiated path); set
  /// `NothingToReclaim`/`NotBroadcast` or [reclaimThrows] to drive the others.
  ReclaimOutcome reclaimResult = const ReclaimOutcome.minted(amountZat: 50000);
  Object? reclaimThrows;

  /// When true, [reclaimEphemeralSlots] never completes — the in-flight
  /// (disabled) state the reclaim button must hold through.
  bool reclaimNeverCompletes = false;
  int reclaimCount = 0;

  @override
  Future<ReclaimOutcome> reclaimEphemeralSlots() {
    reclaimCount++;
    if (reclaimNeverCompletes) return Completer<ReclaimOutcome>().future;
    if (reclaimThrows != null) {
      return Future<ReclaimOutcome>.error(reclaimThrows!);
    }
    return Future<ReclaimOutcome>.value(reclaimResult);
  }

  /// The parked sends [listParkedSends] returns unless [listParkedSendsThrows] is
  /// set. Defaults to empty (no parked send); set [parkedSendFixture] rows to drive
  /// the "saved & pending" surface.
  List<ParkedSend> parkedSendsResult = const [];
  Object? listParkedSendsThrows;
  int listParkedSendsCount = 0;

  /// When set, [listParkedSends] PARKS (after counting) until this completes —
  /// the [birthdayHeightGate] shape, for observing the section's in-flight
  /// re-pull window (the #407 R5 retry-label live region renders only there).
  Completer<void>? listParkedSendsGate;

  @override
  Future<List<ParkedSend>> listParkedSends() async {
    listParkedSendsCount++;
    final gate = listParkedSendsGate;
    if (gate != null) await gate.future;
    if (listParkedSendsThrows != null) {
      throw listParkedSendsThrows!;
    }
    return parkedSendsResult;
  }

  /// The in-flight two-step sends [listInFlightSends] returns unless
  /// [listInFlightSendsThrows] is set. Defaults to empty (nothing mid-flight); set
  /// rows to drive the durable "on its way — don't send it again" cue (#309).
  List<InFlightSend> inFlightSendsResult = const [];
  Object? listInFlightSendsThrows;
  int listInFlightSendsCount = 0;

  @override
  Future<List<InFlightSend>> listInFlightSends() {
    listInFlightSendsCount++;
    if (listInFlightSendsThrows != null) {
      return Future<List<InFlightSend>>.error(listInFlightSendsThrows!);
    }
    return Future<List<InFlightSend>>.value(inFlightSendsResult);
  }

  /// The durable in-flight swaps [listInFlightSwaps] returns unless
  /// [listInFlightSwapsThrows] is set. Defaults to empty (nothing in flight);
  /// set rows to drive the durable swap home (W-swap-5, #366).
  List<SwapRecord> inFlightSwapsResult = const [];
  Object? listInFlightSwapsThrows;
  int listInFlightSwapsCount = 0;

  @override
  Future<List<SwapRecord>> listInFlightSwaps() {
    listInFlightSwapsCount++;
    if (listInFlightSwapsThrows != null) {
      return Future<List<SwapRecord>>.error(listInFlightSwapsThrows!);
    }
    return Future<List<SwapRecord>>.value(inFlightSwapsResult);
  }

  /// What [dismissSwapRecord] returns unless [dismissSwapRecordThrows] is set
  /// (`true` = removed; `false` models the idempotent already-absent case).
  bool dismissSwapRecordResult = true;
  Object? dismissSwapRecordThrows;
  int dismissSwapRecordCount = 0;
  String? lastDismissedSwapId;

  @override
  Future<bool> dismissSwapRecord({required String swapId}) async {
    dismissSwapRecordCount++;
    lastDismissedSwapId = swapId;
    if (dismissSwapRecordThrows != null) throw dismissSwapRecordThrows!;
    return dismissSwapRecordResult;
  }

  // --- #390 deep-scan ("Check older swap addresses") injection --------------
  /// What [checkOlderSwapAddresses] returns unless [checkOlderSwapAddressesThrows]
  /// is set. Defaults to a one-STEP widen with the band fully pending.
  SwapAddressCheckReport checkOlderSwapAddressesResult =
      const SwapAddressCheckReport(
        widenedBy: 1024,
        coveredSwaps: 1088,
        pending: 1024,
      );
  Object? checkOlderSwapAddressesThrows;

  /// When true, [checkOlderSwapAddresses] never completes — the in-flight
  /// (disabled + spinner) state the deep-scan button must hold through.
  bool checkOlderSwapAddressesNeverCompletes = false;
  int checkOlderSwapAddressesCount = 0;

  @override
  Future<SwapAddressCheckReport> checkOlderSwapAddresses() {
    checkOlderSwapAddressesCount++;
    if (checkOlderSwapAddressesNeverCompletes) {
      return Completer<SwapAddressCheckReport>().future;
    }
    if (checkOlderSwapAddressesThrows != null) {
      return Future<SwapAddressCheckReport>.error(
        checkOlderSwapAddressesThrows!,
      );
    }
    return Future<SwapAddressCheckReport>.value(checkOlderSwapAddressesResult);
  }

  /// What [swapAddressCheckCoverage] returns unless [swapAddressCheckCoverageThrows]
  /// is set. Defaults to a restored wallet mid-check (63 covered, band pending).
  SwapAddressCoverage swapAddressCheckCoverageResult =
      const SwapAddressCoverage(coveredSwaps: 63, pending: 63);
  Object? swapAddressCheckCoverageThrows;
  int swapAddressCheckCoverageCount = 0;

  @override
  Future<SwapAddressCoverage> swapAddressCheckCoverage() {
    swapAddressCheckCoverageCount++;
    if (swapAddressCheckCoverageThrows != null) {
      return Future<SwapAddressCoverage>.error(swapAddressCheckCoverageThrows!);
    }
    return Future<SwapAddressCoverage>.value(swapAddressCheckCoverageResult);
  }

  /// What [recordSwapOutcome] returns unless [recordSwapOutcomeThrows] is set
  /// (`true` = pinned; `false` models the idempotent absent/lapsed/already-
  /// pinned case). Records the last pin so tests assert the #367
  /// observation→pin contract (the tracking stream pins, never dismisses).
  bool recordSwapOutcomeResult = true;
  Object? recordSwapOutcomeThrows;
  int recordSwapOutcomeCount = 0;
  String? lastOutcomeSwapId;
  SwapOutcome? lastRecordedOutcome;

  @override
  Future<bool> recordSwapOutcome({
    required String swapId,
    required SwapOutcome outcome,
  }) async {
    recordSwapOutcomeCount++;
    lastOutcomeSwapId = swapId;
    lastRecordedOutcome = outcome;
    if (recordSwapOutcomeThrows != null) throw recordSwapOutcomeThrows!;
    return recordSwapOutcomeResult;
  }

  /// What [cancelParkedSend] returns unless [cancelParkedSendThrows] is set
  /// (`true` = removed; set `false` to model an already-gone / draining / reused
  /// no-op the host must NOT present as "cancelled").
  bool cancelParkedSendResult = true;
  Object? cancelParkedSendThrows;
  int cancelParkedSendCount = 0;
  int? lastCancelId;
  int? lastCancelCreatedAt;

  @override
  Future<bool> cancelParkedSend({
    required int id,
    required int createdAt,
  }) async {
    cancelParkedSendCount++;
    lastCancelId = id;
    lastCancelCreatedAt = createdAt;
    if (cancelParkedSendThrows != null) throw cancelParkedSendThrows!;
    return cancelParkedSendResult;
  }

  /// What [retryParkedSend] returns unless [retryParkedSendThrows] is set
  /// (`true` = re-armed; set `false` to model an already-gone / draining / reused
  /// no-op the host re-reads the parked list on).
  bool retryParkedSendResult = true;
  Object? retryParkedSendThrows;
  int retryParkedSendCount = 0;
  int? lastRetryId;
  int? lastRetryCreatedAt;

  @override
  Future<bool> retryParkedSend({
    required int id,
    required int createdAt,
  }) async {
    retryParkedSendCount++;
    lastRetryId = id;
    lastRetryCreatedAt = createdAt;
    if (retryParkedSendThrows != null) throw retryParkedSendThrows!;
    return retryParkedSendResult;
  }

  /// What [authorizeParkedSend] resolves to unless [authorizeParkedSendThrows] is
  /// set (FR-23-b / #361). Defaults to the happy `signed`; drive
  /// [ParkedAuthorization.stillQueued] to exercise the "still waiting, NOT a
  /// failure" copy, and `notFound` for the already-gone re-read path.
  ParkedAuthorization authorizeParkedSendResult = ParkedAuthorization.signed;
  Object? authorizeParkedSendThrows;
  int authorizeParkedSendCount = 0;
  int? lastAuthorizeId;
  int? lastAuthorizeCreatedAt;

  /// Set to hold [authorizeParkedSend] open until the future completes — the
  /// slow-sign shape (a ZK prove is tens of seconds on a phone), for single-flight
  /// and mid-call-dispose tests. Null (the default) resolves immediately.
  Future<void>? authorizeParkedSendGate;

  @override
  Future<ParkedAuthorization> authorizeParkedSend({
    required int id,
    required int createdAt,
  }) async {
    authorizeParkedSendCount++;
    lastAuthorizeId = id;
    lastAuthorizeCreatedAt = createdAt;
    if (authorizeParkedSendGate != null) await authorizeParkedSendGate;
    if (authorizeParkedSendThrows != null) throw authorizeParkedSendThrows!;
    return authorizeParkedSendResult;
  }

  // --- Transaction history (FR-1) injection ---------------------------------
  /// The FIRST page's rows (`after == null`). Defaults to empty (no history);
  /// override with [txSummaryFixture] rows to drive the activity list.
  List<TxSummary> transactionsResult = const [];

  /// The FIRST page's opaque cursor (`null` ⇒ no more pages — the default).
  String? nextCursorResult;

  /// Optional keyset map for multi-page tests: keyed by the `after` cursor
  /// (`null` = first page), each value is the page to return. When non-empty it
  /// takes precedence over [transactionsResult]/[nextCursorResult], so a test can
  /// drive a full "load more" walk deterministically.
  final Map<String?, HistoryPage> pagesByCursor = {};

  /// When set, [transactions] returns a future that errors with this.
  Object? transactionsThrows;

  /// When true, [transactions] never completes — the wedged-FFI edge the activity
  /// provider's timeout converts into the honest error state, not an endless spinner.
  bool transactionsNeverCompletes = false;

  int transactionsCount = 0;
  int? lastTransactionsLimit;
  String? lastTransactionsAfter;

  @override
  Future<HistoryPage> transactions({required int limit, String? after}) {
    transactionsCount++;
    lastTransactionsLimit = limit;
    lastTransactionsAfter = after;
    if (transactionsNeverCompletes) return Completer<HistoryPage>().future;
    if (transactionsThrows != null) {
      return Future<HistoryPage>.error(transactionsThrows!);
    }
    if (pagesByCursor.isNotEmpty) {
      final page =
          pagesByCursor[after] ?? const HistoryPage(rows: [], nextCursor: null);
      return Future<HistoryPage>.value(page);
    }
    return Future<HistoryPage>.value(
      HistoryPage(rows: transactionsResult, nextCursor: nextCursorResult),
    );
  }

  /// #397: the fake watch-only flag — the chrome key. Override per test.
  bool isWatchOnlyResult = false;

  @override
  Future<bool> isWatchOnly() => Future<bool>.value(isWatchOnlyResult);

  /// The fake UFVK the session-port export returns (G1 / #361 companion) — a
  /// plausible `uview…` shape; the screen only renders + copies it.
  String exportUfvkResult =
      'uviewfakeexportedviewingkeyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';
  Object? exportUfvkThrows;
  int exportUfvkCount = 0;

  @override
  Future<String> exportUfvk() {
    exportUfvkCount++;
    if (exportUfvkThrows != null) {
      return Future<String>.error(exportUfvkThrows!);
    }
    return Future<String>.value(exportUfvkResult);
  }

  /// The fake receive address (a plausible mainnet UA-ish string; the screen
  /// only renders it). Override per test as needed.
  String currentAddressResult =
      'u1fakereceiveaddressxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';
  Object? currentAddressThrows;
  int currentAddressCount = 0;

  /// When true, [currentAddress] returns a future that never completes — the
  /// hung-FFI mobile edge the receive provider's timeout must convert into the
  /// honest error state rather than an infinite spinner.
  bool currentAddressNeverCompletes = false;

  @override
  Future<String> currentAddress() {
    currentAddressCount++;
    if (currentAddressThrows != null) {
      return Future<String>.error(currentAddressThrows!);
    }
    if (currentAddressNeverCompletes) return Completer<String>().future;
    return Future<String>.value(currentAddressResult);
  }

  /// The fake TRANSPARENT receive address (a plausible mainnet t-addr; the screen
  /// only renders it). Distinct from [currentAddressResult] so the toggle test can
  /// prove the screen swaps between the two.
  String currentTransparentAddressResult =
      't1fakeTransparentReceiveAddrxxxxxxxxx';
  Object? currentTransparentAddressThrows;
  int currentTransparentAddressCount = 0;

  /// When true, [currentTransparentAddress] never completes — the hung-FFI edge the
  /// transparent-address provider's timeout converts into an honest error state.
  bool currentTransparentAddressNeverCompletes = false;

  /// When set, [currentTransparentAddress] PARKS until this gate completes —
  /// the mid-flight interleaving harness (#330), like [proposeGate].
  Completer<void>? currentTransparentAddressGate;

  @override
  Future<String> currentTransparentAddress() async {
    currentTransparentAddressCount++;
    final gate = currentTransparentAddressGate;
    if (gate != null) await gate.future;
    if (currentTransparentAddressThrows != null) {
      throw currentTransparentAddressThrows!;
    }
    if (currentTransparentAddressNeverCompletes) {
      return Completer<String>().future;
    }
    return currentTransparentAddressResult;
  }

  /// The fake FRESH diversified receive address (FR-8 / Recv-4). Mint
  /// semantics: every call returns a DISTINCT address, so the fake appends the
  /// (post-increment) call count and advances [mintDiversifiedAddressIndex] —
  /// a repeated-tap test observes two different addresses without re-wiring.
  /// Override [mintDiversifiedAddressResult] for an exact-string pin (the
  /// suffix is still appended on the second call onward: count 1 returns it
  /// verbatim).
  String mintDiversifiedAddressResult =
      'u1fakefreshdiversifiedaddressxxxxxxxxxxxxxxxxxxxxxxxxxxx';
  int mintDiversifiedAddressIndex = 1099511627776; // the region base, 2^40
  Object? mintDiversifiedAddressThrows;
  int mintDiversifiedAddressCount = 0;

  /// When true, [mintDiversifiedAddress] never completes — the hung-FFI edge
  /// the receive screen's action timeout converts into the honest error
  /// snackbar rather than a stuck spinner.
  bool mintDiversifiedAddressNeverCompletes = false;

  @override
  Future<WalletMintedAddress> mintDiversifiedAddress() {
    mintDiversifiedAddressCount++;
    if (mintDiversifiedAddressThrows != null) {
      return Future<WalletMintedAddress>.error(mintDiversifiedAddressThrows!);
    }
    if (mintDiversifiedAddressNeverCompletes) {
      return Completer<WalletMintedAddress>().future;
    }
    final address = mintDiversifiedAddressCount == 1
        ? mintDiversifiedAddressResult
        : '$mintDiversifiedAddressResult$mintDiversifiedAddressCount';
    return Future<WalletMintedAddress>.value(
      WalletMintedAddress(
        address: address,
        diversifierIndex: mintDiversifiedAddressIndex++,
      ),
    );
  }

  /// The fake scan floor (#317). Defaults to a plausible mainnet birthday;
  /// `null` models the pre-provision wallet. [birthdayHeightThrows] models the
  /// wedged-read fallback arm the rescan sheet must degrade through.
  int? birthdayHeightResult = 2400000;
  Object? birthdayHeightThrows;
  int birthdayHeightCount = 0;

  /// When set, [birthdayHeight] PARKS (after counting) until this completes —
  /// lets a test observe the rescan sheet's transient "resolving" window
  /// (Start disabled, the recommended-range cue) before the floor lands.
  Completer<void>? birthdayHeightGate;

  @override
  Future<int?> birthdayHeight() async {
    birthdayHeightCount++;
    final gate = birthdayHeightGate;
    if (gate != null) await gate.future;
    if (birthdayHeightThrows != null) throw birthdayHeightThrows!;
    return birthdayHeightResult;
  }

  // ── The sync-server picker (P3-13) ────────────────────────────────────────

  /// What [syncServers] returns — the host's offered list.
  List<SyncServer> syncServersResult = const [];

  /// What [syncServerStatus] returns; null ⇒ the default `zec.rocks` status
  /// with no choice and no fallback.
  SyncServerStatus? syncServerStatusResult;
  Object? syncServerStatusThrows;

  /// What [probeSyncServer] answers; null ⇒ a probe at tip 3,482,911.
  SyncServerProbe? probeResult;
  Object? probeThrows;
  Completer<void>? probeGate;
  SyncServerChoice? lastProbedChoice;
  int probeCount = 0;

  @override
  Future<List<SyncServer>> syncServers() async => syncServersResult;

  @override
  Future<SyncServerStatus> syncServerStatus() async {
    if (syncServerStatusThrows != null) throw syncServerStatusThrows!;
    return syncServerStatusResult ??
        const SyncServerStatus(
          effectiveUrl: 'https://zec.rocks:443',
          defaultUrl: 'https://zec.rocks:443',
          choice: null,
          fallback: null,
        );
  }

  @override
  Future<SyncServerProbe> probeSyncServer(SyncServerChoice choice) async {
    probeCount++;
    lastProbedChoice = choice;
    final gate = probeGate;
    if (gate != null) await gate.future;
    if (probeThrows != null) throw probeThrows!;
    return probeResult ?? const SyncServerProbe(tip: 3482911);
  }

  /// When set, [startSync] PARKS (after counting) until this gate completes —
  /// the WEDGED-BRIDGE harness (#409 R2), twin of [stopSyncGate]. A start that
  /// never answers is the case the [walletFfiWedgeTimeout] bound exists for, and
  /// it is NOT the same as [failStart]: a throw is the SDK saying no, whereas a
  /// silent start is the SDK saying nothing while the Rust loop it was asked to
  /// spawn most likely runs.
  Completer<void>? startSyncGate;

  @override
  Future<void> startSync() async {
    startCount++;
    final gate = startSyncGate;
    if (gate != null) await gate.future;
    if (failStart) throw StateError('fake start failure');
  }

  /// When set, [stopSync] PARKS (after counting) until this gate completes —
  /// the mid-flight interleaving harness for the sync controller's command
  /// chain (#330 class / ), like [proposeGate].
  Completer<void>? stopSyncGate;

  @override
  Future<void> stopSync() async {
    stopCount++;
    final gate = stopSyncGate;
    if (gate != null) await gate.future;
    if (failStop) throw StateError('fake stop failure');
    // FAITHFUL TO THE CORE (#409 R1). `SyncController::stop()` ends by
    // publishing `SyncStatus::Idle` (`sync_controller.rs:328`, pinned by the
    // core's own tests), so every successful stop MOVES THE OBSERVED STATUS.
    // This fake counted the call and pushed nothing, and that omission is not a
    // harmless simplification: it is why the shipped #407 R3 escalation pin
    // passed against a predicate that resets on any non-`Stalled` value. A kick
    // is a stop+start, so in production the kick's own teardown fed `Idle` back
    // into the reset listener and cancelled the very cooldown it had just set —
    // a condition no test could reach while this method stayed silent.
    //
    // Deliberately AFTER the `failStop` throw: the core cannot publish `Idle`
    // for a stop that did not complete.
    push(const SyncStatus.idle());
  }

  @override
  ValidatedAddress validateRecipient(String address) {
    validateRecipientCount++;
    lastValidatedRecipient = address;
    if (validateRecipientThrows != null) throw validateRecipientThrows!;
    return validateRecipientResult;
  }

  @override
  String composePaymentUri({
    required String recipient,
    required int amountZat,
    String? memoText,
    List<int>? memoBytes,
  }) {
    composeCount++;
    lastComposeRecipient = recipient;
    lastComposeAmountZat = amountZat;
    lastComposeMemo = memoText;
    // FR-28: recorded, never interpreted — a test asserts the exact bytes the
    // form handed the encoder, which is the only place the "survives a
    // re-compose" contract can be checked without a device.
    lastComposeMemoBytes = memoBytes == null ? null : List<int>.of(memoBytes);
    if (composeThrows != null) throw composeThrows!;
    // A synthetic, deterministic URI — the host-VM fake never touches the native
    // ZIP-321 encoder; the round-trip fidelity is covered at the SDK level.
    return 'zcash:$recipient?amount=$amountZat';
  }

  @override
  WalletSendRequest parseSendRequest(String uri) {
    parseSendRequestCount++;
    lastParsedUri = uri;
    if (parseSendRequestThrows != null) throw parseSendRequestThrows!;
    // The host-VM fake never touches the native ZIP-321 parser; the real
    // parse + seam policy is covered by the adapter's pure mapper tests. Default
    // echoes the URI as the address so a prefill test that doesn't pin a request
    // still gets a deterministic, Review-enabled recipient.
    return parseSendRequestResult ?? WalletSendRequest(address: uri);
  }

  @override
  Future<SendProposal> propose(String requestUri) async {
    proposeCount++;
    lastProposeUri = requestUri;
    final gate = proposeGate;
    if (gate != null) await gate.future;
    if (proposeThrows != null) throw proposeThrows!;
    return proposeResult ?? sendProposalFixture();
  }

  @override
  Future<List<TxSubmitResult>> send(int proposalId) async {
    sendCount++;
    lastSendProposalId = proposalId;
    final gate = sendGate;
    if (gate != null) await gate.future;
    if (sendThrows != null) throw sendThrows!;
    return sendResults ?? const [TxSubmitResult.success(txidHex: 'aabbccdd')];
  }

  @override
  Future<DeliveryState?> deliveryState(String txidHex) async {
    deliveryStateCount++;
    if (deliveryStateThrows != null) throw deliveryStateThrows!;
    return deliveryStates.containsKey(txidHex)
        ? deliveryStates[txidHex]
        : DeliveryState.retryPending;
  }

  @override
  Future<String> queueSend(String requestUri) async {
    queueCount++;
    lastQueueUri = requestUri;
    if (queueThrows != null) throw queueThrows!;
    return queuedId;
  }

  /// Returned by [machineMemos]; keyed by txid so a test can model "this
  /// transaction carries an envelope and that one does not".
  final Map<String, List<Uint8List>> machineMemosByTxid = {};

  /// When set, [machineMemos] throws this (e.g. a typed scope refusal).
  Object? machineMemosThrows;

  String? lastMachineMemosTxid;

  @override
  Future<List<Uint8List>> machineMemos(String txidHex) async {
    lastMachineMemosTxid = txidHex;
    if (machineMemosThrows != null) throw machineMemosThrows!;
    return machineMemosByTxid[txidHex] ?? const [];
  }

  @override
  Future<SendProposal?> proposeShield() async {
    proposeShieldCount++;
    final gate = proposeShieldGate;
    if (gate != null) await gate.future;
    if (proposeShieldNeverCompletes) return Completer<SendProposal?>().future;
    if (proposeShieldThrows != null) throw proposeShieldThrows!;
    return proposeShieldResult;
  }

  // --- Swap on-ramp (D-2) ----------------------------------------------------

  @override
  Future<void> enableNearSwap({
    required SwapProviderConfig config,
    required bool swapEnabled,
    SwapKill? declaredKill,
  }) async {
    enableNearSwapCount++;
    lastEnableConfig = config;
    lastEnableSwapEnabled = swapEnabled;
    lastEnableDeclaredKill = declaredKill;
    if (enableNearSwapThrows != null) throw enableNearSwapThrows!;
  }

  @override
  Future<SwapQuote> swapQuote({required QuoteRequest request}) async {
    swapQuoteCount++;
    lastSwapQuoteRequest = request;
    final gate = swapQuoteGate;
    if (gate != null) await gate.future;
    if (swapQuoteNeverCompletes) return Completer<SwapQuote>().future;
    if (swapQuoteThrows != null) throw swapQuoteThrows!;
    return swapQuoteResult ?? swapQuoteFixture();
  }

  @override
  Future<String> swapExecute({required SwapQuote quote}) async {
    swapExecuteCount++;
    lastSwapExecuteQuote = quote;
    if (swapExecuteGate != null) await swapExecuteGate!.future;
    if (swapExecuteThrows != null) throw swapExecuteThrows!;
    return swapExecuteResult;
  }

  @override
  Future<SwapTokenList> swapListTokens() async {
    swapListTokensCount++;
    if (swapListTokensThrows != null) throw swapListTokensThrows!;
    return swapListTokensResult ?? swapTokenListFixture();
  }

  @override
  Stream<SwapStatus> watchSwapStatus({required String swapId}) {
    swapSubscribeCount++;
    lastWatchedSwapId = swapId;
    final controller = StreamController<SwapStatus>();
    controller.onListen = () {
      if (failSwapOnSubscribe) {
        controller.addError('fake swap subscribe failure');
      } else if (!holdSwapSubscribe) {
        controller.add(swapCurrent);
      }
      // holdSwapSubscribe: connected but silent → the notifier stays loading.
    };
    controller.onCancel = () {
      swapCancelCount++;
      if (identical(_activeSwap, controller)) _activeSwap = null;
    };
    _activeSwap = controller;
    return controller.stream;
  }
}

/// A neutral [TxSummary] history row for tests (FR-1) — override only what a case
/// cares about. Defaults to a confirmed incoming tx with a memo.
TxSummary txSummaryFixture({
  String txidHex =
      'aa00000000000000000000000000000000000000000000000000000000000000',
  int? minedHeight = 100,
  TxStatus status = const TxStatus.confirmed(depth: 5),
  int netAmountZat = 250000,
  int? feeZat,
  bool hasMemo = true,
  bool hasTransparentOutput = false,
  int? timestamp = 1700000000,
}) {
  return TxSummary(
    txidHex: txidHex,
    minedHeight: minedHeight,
    status: status,
    netAmountZat: netAmountZat,
    feeZat: feeZat,
    hasMemo: hasMemo,
    hasTransparentOutput: hasTransparentOutput,
    timestamp: timestamp,
  );
}

/// A neutral [ParkedSend] for "saved & pending" surface tests (2e-2b-v-3) —
/// amount-only, with an id + enqueue timestamp the cancel flow passes back.
/// Defaults to the two-step (one-time-address) kind, the surface's original
/// shape; pass [ParkedSendKind.singleStep] for a plainly-queued offline send
/// (#331 — the same surface carries both).
ParkedSend parkedSendFixture({
  int id = 7,
  ParkedSendKind kind = ParkedSendKind.twoStep,
  int amountZat = 70000,
  int createdAt = 1700000000,
  bool paused = false,
  // #400 R2: `true` models a MID-SIGNATURE row (`state = Submitting`) — claimed and
  // being proved, or abandoned there by a process kill. It offers no authorize
  // affordance and reads "preparing to send".
  bool sending = false,
  // FR-17 (#396): null models a pre-FR-17 row (the nullable arm the surface
  // must tolerate); pass bytes to model a post-FR-17 row with its re-stage
  // binding.
  Uint8List? binding,
  // Ironwood/NU6.3 + GRACE-1: WHY the drain will not sign this row — the
  // network upgrade, or a server that will not say which network it is on.
  // Defaults null so every existing fixture keeps modelling a healthy queued
  // row.
  SigningBlock? signingBlock,
}) {
  return ParkedSend(
    id: id,
    kind: kind,
    amountZat: amountZat,
    createdAt: createdAt,
    paused: paused,
    sending: sending,
    binding: binding,
    signingBlock: signingBlock,
  );
}

/// An in-flight two-step row (#309) — first leg broadcast, send not complete;
/// drives the durable "on its way — don't send it again" cue.
InFlightSend inFlightSendFixture({
  int amountZat = 80000,
  int createdAt = 1700000000,
}) {
  return InFlightSend(amountZat: amountZat, createdAt: createdAt);
}

/// A durable in-flight swap row (W-swap-5, #366) — drives the wallet-screen
/// swap home + the guard-fault "view swap" re-attach. Defaults to OutOfZec
/// (the armed-deposit shape the hardware photo showed invisible).
SwapRecord swapRecordFixture({
  String id = 'swap-record-1',
  SwapRecordDirection direction = SwapRecordDirection.outOfZec,
  int createdAt = 1700000000,
  // FAR-FUTURE default (#367): the home row renders the neutral past-window
  // line once `depositDeadline` lapses against the REAL clock (display-only),
  // so a default in the past would flip every fixture row off the live
  // direction-honest line the tests assert. Pass a past stamp explicitly to
  // exercise the past-window arm.
  int? depositDeadline = 4100000000,
  int expiresAt = 4100172800,
  SwapOutcome? outcome,
}) {
  return SwapRecord(
    id: id,
    direction: direction,
    createdAt: createdAt,
    depositDeadline: depositDeadline,
    expiresAt: expiresAt,
    outcome: outcome,
  );
}

/// A neutral [EphemeralSweepSummary] for recovery-sheet tests (2e-2b-v-2).
/// Defaults to a clean funded recovery (one address swept, nothing failed or
/// truncated); override to model partial faults / a heavy-wallet truncation.
EphemeralSweepSummary ephemeralSweepSummaryFixture({
  int scanned = 1,
  int swept = 1,
  int recoveredZat = 250000,
  int failed = 0,
  int truncated = 0,
}) {
  return EphemeralSweepSummary(
    scanned: scanned,
    swept: swept,
    recoveredZat: recoveredZat,
    failed: failed,
    truncated: truncated,
  );
}

/// A neutral [SwapQuote] for tests — override only what a case cares about.
/// Defaults to an OutOfZec-shaped quote (a de-shielding deposit) so the
/// disclosure carries the privacy-critical flags a swap review must render.
SwapQuote swapQuoteFixture({
  String id = 'swap-1',
  String depositAddress = 'tdeposit',
  String? depositMemo,
  int expiresAt = 2000000000,
  String amountIn = '0.5',
  String minAmountOut = '49.5',
  int zecSideZat = 50000000,
  String? refundTo,
  bool deshields = true,
  bool endsShielded = false,
  bool providerLegsTransparent = true,
  List<DisclosureItem> providerSees = const [
    DisclosureItem.amounts,
    DisclosureItem.destinationAddress,
  ],
  // FR-17 (#396): a quote returned by `swapQuote` always carries its
  // spend-binding nonce, so the fixture defaults to a deterministic one.
  Uint8List? binding,
}) {
  return SwapQuote(
    id: id,
    depositAddress: depositAddress,
    depositMemo: depositMemo,
    expiresAt: expiresAt,
    amountIn: amountIn,
    minAmountOut: minAmountOut,
    zecSideZat: zecSideZat,
    refundTo: refundTo,
    disclosure: SwapPrivacyDisclosure(
      endsShielded: endsShielded,
      deshields: deshields,
      providerLegsTransparent: providerLegsTransparent,
      providerSees: providerSees,
    ),
    binding: binding ?? defaultSwapQuoteBinding,
  );
}

/// The deterministic FR-17 spend-binding nonce [swapQuoteFixture] defaults to
/// — distinct from [defaultSendProposalBinding] so a test that mixes a
/// proposal and a quote can tell whose bytes reached a seam.
final Uint8List defaultSwapQuoteBinding = Uint8List.fromList(
  List.filled(32, 9),
);

/// A neutral [SwapTokenList] for IntoZec picker tests — two plausible source
/// assets, `fresh` by default. Override `fresh: false` to model serve-stale, or
/// `tokens: const []` (fresh) to model the honest "no assets available" state.
SwapTokenList swapTokenListFixture({
  List<SwapToken>? tokens,
  bool fresh = true,
}) {
  return SwapTokenList(
    tokens:
        tokens ??
        const [
          SwapToken(
            chain: 'eth',
            symbol: 'usdc',
            decimals: 6,
            providerAssetId: 'nep141:eth.usdc',
            priceUsd: 1.0,
          ),
          SwapToken(
            chain: 'btc',
            symbol: 'btc',
            decimals: 8,
            providerAssetId: 'nep141:btc',
            priceUsd: 65000.0,
          ),
        ],
    fresh: fresh,
  );
}

/// A neutral [SendProposal] for tests — override only what a case cares about.
/// Defaults to a single shielded (Orchard) recipient (no de-shield disclosure).
SendProposal sendProposalFixture({
  int proposalId = 1,
  int totalZat = 100500,
  int feeZat = 500,
  int changeZat = 0,
  bool hasTransparentRecipient = false,
  bool isShield = false,
  // A ZIP-320 TEX two-step (the partial-terminal "in motion" outcome keys off this).
  // Default `false` (an ordinary single-step send) so existing tests are unaffected.
  bool isTwoStepTex = false,
  List<ProposalStep>? steps,
  // Money-safety signals (#226). Default to an ORDINARY send (no large-amount
  // confirm, not a self-send) so the existing send tests are unaffected; a case
  // that exercises the large-send dialog / self-send note sets these explicitly.
  LargeSendReason? largeSend,
  bool selfSend = false,
  // FR-46: the one recipient's total as signed. Defaults to the default step's
  // lone recipient amount; a case pins its own (or `null` for two recipients).
  int? singleRecipientZat = 100000,
  // FR-17 (#396): required on the real DTO; deterministic bytes by default so
  // a test that pins the seam's pass-through sets its own distinct fill.
  Uint8List? binding,
}) {
  return SendProposal(
    proposalId: proposalId,
    totalZat: totalZat,
    feeZat: feeZat,
    changeZat: changeZat,
    steps:
        steps ??
        const [
          ProposalStep(
            recipients: [
              ProposalRecipient(pool: OutputPool.orchard, amountZat: 100000),
            ],
          ),
        ],
    targetHeight: 2_000_000,
    hasTransparentRecipient: hasTransparentRecipient,
    isShield: isShield,
    isTwoStepTex: isTwoStepTex,
    largeSend: largeSend,
    selfSend: selfSend,
    singleRecipientZat: singleRecipientZat,
    binding: binding ?? defaultSendProposalBinding,
  );
}

/// The deterministic FR-17 spend-binding nonce [sendProposalFixture] (and via
/// it [shieldProposalFixture]) defaults to — distinct from
/// [defaultSwapQuoteBinding] so a test that mixes a proposal and a quote can
/// tell whose bytes reached a seam.
final Uint8List defaultSendProposalBinding = Uint8List.fromList(
  List.filled(32, 7),
);

/// A neutral SHIELD [SendProposal] for Recv-3 tests — `isShield == true`, NO
/// transparent recipient (privacy-positive), gross/fee/net consistent
/// (`changeZat == totalZat - feeZat`), the lone output in the shielded pool.
SendProposal shieldProposalFixture({
  int proposalId = 42,
  int totalZat = 500000, // gross transparent being shielded
  int feeZat = 15000,
}) {
  final netShielded = totalZat - feeZat;
  return sendProposalFixture(
    proposalId: proposalId,
    totalZat: totalZat,
    feeZat: feeZat,
    changeZat: netShielded,
    isShield: true,
    steps: [
      ProposalStep(
        recipients: [
          ProposalRecipient(pool: OutputPool.orchard, amountZat: netShielded),
        ],
      ),
    ],
  );
}

/// A complete, neutral [WalletState] for tests — override only what a case
/// cares about. `PlatformInt64` is `int` on the host VM (web is out of scope
/// for this SDK), so plain int literals construct the money fields.
///
/// [tip] defaults to what the core reports: a snapshot read at a completed
/// pass (`UpToDate`) carries that pass's own tip; any other status, none.
/// Pass it explicitly to model a read taken mid-pass (a newer tip recorded).
WalletState walletStateFixture({
  SyncStatus syncStatus = const SyncStatus.idle(),
  TorState tor = const TorState.off(),
  BalanceSnapshot? balance,
  int? tip,
  SyncStamp? lastSynced,
  bool everSynced = false,
  bool rescanRebuilding = false,
  int seq = 0,
}) {
  return WalletState(
    balance: balance ?? balanceFixture(),
    syncStatus: syncStatus,
    tor: tor,
    tip:
        tip ??
        switch (syncStatus) {
          SyncStatus_UpToDate(:final tip) => tip,
          _ => null,
        },
    lastSynced: lastSynced,
    everSynced: everSynced,
    rescanRebuilding: rescanRebuilding,
    seq: seq,
  );
}

BalanceSnapshot balanceFixture({
  int spendableZat = 0,
  int pendingIncomingZat = 0,
  int pendingChangeZat = 0,
  int transparentZat = 0,
  int totalZat = 0,
}) {
  return BalanceSnapshot(
    spendableZat: spendableZat,
    pendingIncomingZat: pendingIncomingZat,
    pendingChangeZat: pendingChangeZat,
    transparentZat: transparentZat,
    totalZat: totalZat,
  );
}
