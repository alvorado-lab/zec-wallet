import 'dart:typed_data' show Uint8List;

import 'package:zec_wallet/zec_wallet.dart';

import 'diversifier_narrowing.dart';
import 'send/wallet_send_request.dart';
import 'wallet_session.dart';

/// The production [WalletSession] adapter (spec §3.2g iii-B-2-b): a thin forward
/// over the opaque FRB [WalletHandle]. Rust stays the single source of truth
/// (design invariant 1) — every method delegates straight to the core; no state
/// is cached here.
///
/// WHY so thin / why no unit test of its own: a real [WalletHandle] is an opaque
/// FRB object that cannot be constructed in `flutter test`, so this wrapper is
/// exercised by the on-device e2e (the live stream + cold snapshot crossing the
/// real bridge). The host-VM tests run against the in-memory `FakeWalletSession`
/// behind the same [WalletSession] port; this class only narrows the wide handle
/// surface to exactly the four methods the sync UI drives — the key-material
/// methods (`revealMnemonic`) deliberately stay OFF this port (they live on the
/// provisioner), so a balance-only screen cannot reach the seed.
///
/// HANDLE OWNERSHIP: the handle is owned by the `FrbWalletProvisioner` that
/// created/opened it (one wallet per app session); this session borrows it. The
/// deterministic `handle.close()` on app teardown rides the provisioner's
/// lifecycle, not this wrapper (manager-carried: the desktop forced-exit close,
/// task #71 — tracked in docs/handoff/wallet-log.md).
/// DO NOT override `==`/`hashCode` on this class. The live-sync provider graph
/// keys off session OBJECT IDENTITY: `walletSessionProvider` compares by `==`, so
/// a rescan that swaps in a FRESH `FrbWalletSession` (over the rebuilt handle) is
/// what triggers `syncStatusProvider`/`walletSnapshotProvider`/`walletTransactions
/// Provider`/`walletSyncControllerProvider` to rebuild and re-subscribe from the
/// lower birthday. A value-equality override (e.g. comparing the underlying
/// handle) would make two distinct sessions compare equal and SILENTLY break every
/// session swap — the post-rescan sync restart most of all (the gate-rebuild
/// behaviour is pinned by `rescanActiveWallet (FR-1b)` asserting the swapped
/// session is `isNot(same(original))`).
class FrbWalletSession implements WalletSession {
  FrbWalletSession(this._handle, {required Network network})
    : _network = network;

  final WalletHandle _handle;

  /// The wallet's own network — the sole lowering authority for
  /// [composePaymentUri] (a cross-network address fails typed at compose time).
  /// Threaded from the host-policy `WalletConfig` by the provisioner; the screen
  /// never needs to know mainnet/testnet.
  final Network _network;

  @override
  Stream<SyncStatus> watchSyncStatus() => _handle.watchSyncStatus();

  @override
  Stream<IncomingFundsEvent> watchIncomingFunds({String? sinceCursor}) =>
      _handle.watchIncomingFunds(sinceCursor: sinceCursor);

  @override
  Future<WalletState> snapshot() => _handle.snapshot();

  @override
  Future<List<SyncServer>> syncServers() => _handle.syncServers();

  @override
  Future<SyncServerStatus> syncServerStatus() => _handle.syncServerStatus();

  @override
  Future<SyncServerProbe> probeSyncServer(SyncServerChoice choice) =>
      _handle.probeSyncServer(choice: choice);

  @override
  Future<List<RecoverableEphemeralFunds>> recoverableEphemeralFunds() =>
      _handle.recoverableEphemeralFunds();

  @override
  Future<EphemeralSweepSummary> sweepEphemeralFunds() =>
      _handle.sweepEphemeralFunds();

  @override
  Future<ReclaimOutcome> reclaimEphemeralSlots() =>
      _handle.reclaimEphemeralSlots();

  @override
  Future<List<ParkedSend>> listParkedSends() => _handle.listParkedSends();

  @override
  Future<List<InFlightSend>> listInFlightSends() => _handle.listInFlightSends();

  @override
  Future<bool> cancelParkedSend({required int id, required int createdAt}) =>
      _handle.cancelParkedSend(id: id, createdAt: createdAt);

  @override
  Future<bool> retryParkedSend({required int id, required int createdAt}) =>
      _handle.retryParkedSend(id: id, createdAt: createdAt);

  @override
  Future<ParkedAuthorization> authorizeParkedSend({
    required int id,
    required int createdAt,
  }) => _handle.authorizeParkedSend(id: id, createdAt: createdAt);

  @override
  Future<HistoryPage> transactions({required int limit, String? after}) =>
      _handle.transactions(limit: limit, after: after);

  @override
  Future<bool> isWatchOnly() => _handle.isWatchOnly();

  @override
  Future<String> exportUfvk() => _handle.exportUfvk();

  @override
  Future<String> currentAddress() => _handle.currentAddress();

  @override
  Future<String> currentTransparentAddress() =>
      _handle.currentTransparentAddress();

  @override
  Future<WalletMintedAddress> mintDiversifiedAddress() async {
    final minted = await _handle.mintDiversifiedAddress();
    return WalletMintedAddress(
      address: minted.address,
      // Exact-narrowing guard extracted to a pure, unit-pinned function
      // (review fold; extraction — see diversifier_narrowing.dart).
      diversifierIndex: narrowDiversifierIndex(minted.diversifierIndex),
    );
  }

  @override
  Future<int?> birthdayHeight() => _handle.birthdayHeight();

  @override
  Future<void> startSync() => _handle.startSync();

  @override
  Future<void> stopSync() => _handle.stopSync();

  /// Classify a recipient via the [validateAddress] bridge fn — the same audited
  /// `Address::parse` gate `composePaymentUri` lowers through. The wallet's OWN
  /// network is supplied here, never by the screen, so a cross-network address
  /// fails typed exactly as a compose would. Sync + local: no future, no network.
  @override
  ValidatedAddress validateRecipient(String address) =>
      validateAddress(address: address, network: _network);

  @override
  String composePaymentUri({
    required String recipient,
    required int amountZat,
    String? memoText,
    List<int>? memoBytes,
  }) {
    // The bridge crossing — `encodePaymentUri` is the audited ZIP-321 encoder
    // (the inverse of the parser `propose` lowers with). An empty memo is dropped
    // to null (a memo is only sendable to a shielded recipient; the SDK rejects a
    // memo to a transparent address typed at this call).
    final trimmedMemo = memoText?.trim();
    // FR-28: opaque bytes are NOT trimmed, normalised or inspected — they are
    // the host's own envelope and any edit here would corrupt it. An EMPTY list
    // drops to null: a zero-length machine memo is nothing to attach, and
    // encoding it would put a 511-byte field of padding on-chain for no reason.
    final bytes = (memoBytes != null && memoBytes.isNotEmpty)
        ? Uint8List.fromList(memoBytes)
        : null;
    return encodePaymentUri(
      payments: [
        PaymentDraft(
          recipientAddress: recipient,
          amountZat: amountZat,
          memoText: (trimmedMemo != null && trimmedMemo.isNotEmpty)
              ? trimmedMemo
              : null,
          memoBytes: bytes,
        ),
      ],
      network: _network,
    );
  }

  @override
  WalletSendRequest parseSendRequest(String uri) {
    // The bridge crossing — `parsePaymentUri` is the audited ZIP-321 parser (the
    // inverse of the encoder `composePaymentUri` lowers with, the DRY single
    // validator). The wallet's OWN network is supplied here, never by the host,
    // so a cross-network URI fails typed exactly as a compose would. The
    // malformed / wrong-network faults are mapped from the bridge's typed
    // `WalletApiError`; the single-leg / text-memo seam policy is enforced by
    // the pure mapper (host-VM tested without the native parser).
    final List<ParsedPayment> legs;
    try {
      legs = parsePaymentUri(uri: uri, network: _network);
    } on WalletApiError catch (error) {
      throw walletSendRequestFaultFromApiError(error);
    }
    return walletSendRequestFromParsedLegs(legs);
  }

  @override
  Future<SendProposal> propose(String requestUri) =>
      _handle.propose(requestUri: requestUri);

  @override
  Future<List<TxSubmitResult>> send(int proposalId) =>
      _handle.send(proposalId: proposalId);

  @override
  Future<DeliveryState?> deliveryState(String txidHex) =>
      _handle.deliveryState(txidHex: txidHex);

  @override
  Future<String> queueSend(String requestUri) =>
      _handle.queueSend(requestUri: requestUri);

  @override
  Future<List<Uint8List>> machineMemos(String txidHex) =>
      _handle.machineMemos(txidHex: txidHex);

  @override
  Future<SendProposal?> proposeShield() => _handle.proposeShield();

  // --- Swap on-ramp (D-2) — thin forwards over the D-1 FFI methods -----------

  @override
  Future<void> enableNearSwap({
    required SwapProviderConfig config,
    required bool swapEnabled,
    SwapKill? declaredKill,
  }) => _handle.enableNearSwap(
    config: config,
    swapEnabled: swapEnabled,
    declaredKill: declaredKill,
  );

  @override
  Future<SwapQuote> swapQuote({required QuoteRequest request}) =>
      _handle.swapQuote(request: request);

  @override
  Future<SwapTokenList> swapListTokens() => _handle.swapListTokens();

  @override
  Future<String> swapExecute({required SwapQuote quote}) =>
      _handle.swapExecute(quote: quote);

  @override
  Stream<SwapStatus> watchSwapStatus({required String swapId}) =>
      _handle.watchSwapStatus(swapId: swapId);

  @override
  Future<List<SwapRecord>> listInFlightSwaps() => _handle.listInFlightSwaps();

  @override
  Future<bool> recordSwapOutcome({
    required String swapId,
    required SwapOutcome outcome,
  }) => _handle.recordSwapOutcome(swapId: swapId, outcome: outcome);

  @override
  Future<bool> dismissSwapRecord({required String swapId}) =>
      _handle.dismissSwapRecord(swapId: swapId);

  @override
  Future<SwapAddressCheckReport> checkOlderSwapAddresses() =>
      _handle.checkOlderSwapAddresses();

  @override
  Future<SwapAddressCoverage> swapAddressCheckCoverage() =>
      _handle.swapAddressCheckCoverage();
}
