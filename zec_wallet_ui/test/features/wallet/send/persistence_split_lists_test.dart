import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';

/// R13 §4.5 row 1 — the four "this error provably came BEFORE persistence"
/// allowlists, kind by kind
/// (`docs/plan/r13-a-saved-transaction-is-never-offered-a-retry.md` §3, §4.1).
///
/// The ORACLE is [_expected]: ONE exhaustive `switch` over the sealed
/// [WalletErrorKind] with no wildcard, so a kind added to the SDK fails the
/// BUILD here until someone decides where it belongs in all four lists. It is
/// written from the design's §3 table (single-step = shield/move, parked =
/// "Send now") and from today's code for send's and queue's lists — never a
/// hand-kept list of kinds.
///
/// The SAMPLES ([_samples]) are only inputs (a sealed class cannot be
/// enumerated at runtime); `every kind has a sample` keeps them complete by
/// counting the factories in the generated `error.dart`.
typedef _Membership = ({bool send, bool queue, bool singleStep, bool parked});

const _none = (send: false, queue: false, singleStep: false, parked: false);

/// In every list: refused at the entry gates of every path.
const _all = (send: true, queue: true, singleStep: true, parked: true);

/// Raised before signing on every signing path (send, single-step, parked),
/// never by the enqueue.
const _beforeEverySign = (
  send: true,
  queue: false,
  singleStep: true,
  parked: true,
);

/// Pre-persistence on the single-step and parked paths, not on send's list
/// (today's code never listed it there; R13 does not change send's list).
const _singleStepAndParked = (
  send: false,
  queue: false,
  singleStep: true,
  parked: true,
);

/// A create the engine rolled back whole / the proposal registry: send and
/// single-step. Parked's create errors are swallowed into `StillQueued`, so
/// they never surface there.
const _sendAndSingleStep = (
  send: true,
  queue: false,
  singleStep: true,
  parked: false,
);

/// A failed COMMIT saves nothing on the single-step path (one all-or-nothing
/// engine write; `raw_tx_bytes` after it stays `StoreCorrupt`), but the same
/// kinds can follow the two-step's `mark_sent_multi` (send), the enqueue's
/// commit (queue) and `mark_sent_multi`/`read_raw_tx` (parked).
const _singleStepOnly = (
  send: false,
  queue: false,
  singleStep: true,
  parked: false,
);

_Membership _expected(WalletErrorKind kind) => switch (kind) {
  WalletErrorKind_InvalidSeedLength() => _singleStepAndParked,
  WalletErrorKind_InvalidMnemonic() => _none,
  WalletErrorKind_NoMnemonic() => _none,
  WalletErrorKind_SeedRequired() => _beforeEverySign,
  WalletErrorKind_SeedMismatch() => _beforeEverySign,
  WalletErrorKind_KeyDerivation() => _singleStepAndParked,
  WalletErrorKind_WatchOnly() => _all,
  WalletErrorKind_InvalidViewingKey() => _none,
  WalletErrorKind_InvalidEndpoint() => _none,
  WalletErrorKind_BirthdayInFuture() => _none,
  WalletErrorKind_InvalidDbDir() => _none,
  WalletErrorKind_InvalidEndpointAuth() => _none,
  // Refused at create/open (F01), never on a send path.
  WalletErrorKind_BroadcastJitterTooLong() => _none,
  WalletErrorKind_AmountOutOfRange() => _none,
  WalletErrorKind_MemoTooLong() => _none,
  WalletErrorKind_ReservedMemoNotSendable() => _none,
  WalletErrorKind_MemoRequiresShieldedRecipient() => _none,
  WalletErrorKind_AddressInvalid() => _none,
  WalletErrorKind_MemoInvalid() => _none,
  WalletErrorKind_MemoConflict() => _none,
  WalletErrorKind_ZeroValuedTransparentOutput() => _none,
  WalletErrorKind_PaymentUriInvalid() => _none,
  WalletErrorKind_TxidInvalid() => _none,
  WalletErrorKind_MachineMemoScopeInvalid() => _none,
  WalletErrorKind_KeystoreUnavailable() => _none,
  WalletErrorKind_KeystoreInconsistent() => _none,
  WalletErrorKind_SealVersionUnsupported() => _none,
  WalletErrorKind_SealInvalid() => _none,
  WalletErrorKind_VaultAbsent() => _none,
  WalletErrorKind_WrapArtifactInvalid() => _none,
  WalletErrorKind_WrapVersionUnsupported() => _none,
  WalletErrorKind_NotFound() => _none,
  WalletErrorKind_NetworkMismatch() => _none,
  WalletErrorKind_ProvisioningIncomplete() => _none,
  WalletErrorKind_WalletAlreadyExists() => _none,
  // `broadcast_persisted`'s `raw_tx_bytes` and the two-step's
  // `mark_sent_multi`: after persistence on every path.
  WalletErrorKind_StoreCorrupt() => _none,
  WalletErrorKind_DiskFull() => _singleStepOnly,
  WalletErrorKind_StoreBusy() => _singleStepOnly,
  WalletErrorKind_WalletAlreadyOpen() => _none,
  WalletErrorKind_WalletBusy() => _all,
  // Phase-dependent on the single-step path: `Closing` is the FFI's
  // `handle_closed` (the core never reached); `Wiped` is also
  // `broadcast_persisted`'s wipe check, after the tx is saved. Written as
  // `== closing` so any other phase fails closed. Parked: only
  // `require_open` / `handle_closed`, both before signing.
  WalletErrorKind_InvalidState(:final phase) => (
    send: false,
    queue: false,
    singleStep: phase == LifecyclePhase.closing,
    parked: true,
  ),
  WalletErrorKind_WalletOpen() => _none,
  WalletErrorKind_SyncServerNotOffered() => _none,
  WalletErrorKind_SyncServerUnreachable() => _none,
  WalletErrorKind_WipeWithPendingSwap() => _none,
  WalletErrorKind_RescanWithInFlightSend() => _none,
  WalletErrorKind_SwapAddressCheckRefused() => _none,
  WalletErrorKind_Sync() => _none,
  WalletErrorKind_NetworkUpgradeUnsupported() => _beforeEverySign,
  WalletErrorKind_ConsensusGraceExpired() => _beforeEverySign,
  WalletErrorKind_ConsensusNotEvaluated() => _beforeEverySign,
  WalletErrorKind_SyncRunning() => _none,
  WalletErrorKind_InsufficientFunds() => _none,
  WalletErrorKind_SendAmountRequired() => _none,
  WalletErrorKind_ProposalAlreadyUsed() => _sendAndSingleStep,
  WalletErrorKind_ProposalStale() => _sendAndSingleStep,
  WalletErrorKind_ProposeFailed() => _singleStepAndParked,
  WalletErrorKind_ProposeTransient() => _none,
  WalletErrorKind_SignFailed() => _sendAndSingleStep,
  WalletErrorKind_QueuedSendStale() => _none,
  // The two-step enrol / the enqueue's cap — neither exists on a
  // single-step shield or move, nor on the parked drain's surfacing errors.
  WalletErrorKind_QueuedSendsFull() => (
    send: true,
    queue: true,
    singleStep: false,
    parked: false,
  ),
  WalletErrorKind_TexSendLimitReached() => _sendAndSingleStep,
  WalletErrorKind_Io() => _singleStepOnly,
  WalletErrorKind_Unknown() => _none,
};

/// One instance of every kind; the phase-carrying kinds once per phase.
List<WalletErrorKind> get _samples => [
  WalletErrorKind.invalidSeedLength(len: BigInt.from(7)),
  const WalletErrorKind.invalidMnemonic(wordIndex: 3),
  const WalletErrorKind.noMnemonic(),
  const WalletErrorKind.seedRequired(),
  const WalletErrorKind.seedMismatch(),
  const WalletErrorKind.keyDerivation(),
  const WalletErrorKind.watchOnly(),
  const WalletErrorKind.invalidViewingKey(),
  const WalletErrorKind.invalidEndpoint(reason: 'r'),
  const WalletErrorKind.birthdayInFuture(),
  const WalletErrorKind.invalidDbDir(reason: 'r'),
  const WalletErrorKind.invalidEndpointAuth(reason: 'r'),
  WalletErrorKind.broadcastJitterTooLong(
    maxMs: BigInt.from(30001),
    ceilingMs: BigInt.from(30000),
  ),
  const WalletErrorKind.amountOutOfRange(),
  WalletErrorKind.memoTooLong(len: BigInt.from(600), max: BigInt.from(512)),
  const WalletErrorKind.reservedMemoNotSendable(),
  const WalletErrorKind.memoRequiresShieldedRecipient(),
  const WalletErrorKind.addressInvalid(),
  const WalletErrorKind.memoInvalid(),
  const WalletErrorKind.memoConflict(),
  const WalletErrorKind.zeroValuedTransparentOutput(),
  const WalletErrorKind.paymentUriInvalid(),
  const WalletErrorKind.txidInvalid(),
  const WalletErrorKind.machineMemoScopeInvalid(reason: 'r'),
  const WalletErrorKind.keystoreUnavailable(),
  const WalletErrorKind.keystoreInconsistent(permanentlyInvalidated: false),
  const WalletErrorKind.sealVersionUnsupported(found: 9),
  const WalletErrorKind.sealInvalid(),
  const WalletErrorKind.vaultAbsent(),
  const WalletErrorKind.wrapArtifactInvalid(),
  const WalletErrorKind.wrapVersionUnsupported(found: 9),
  const WalletErrorKind.notFound(),
  const WalletErrorKind.networkMismatch(),
  const WalletErrorKind.provisioningIncomplete(),
  const WalletErrorKind.walletAlreadyExists(),
  const WalletErrorKind.storeCorrupt(),
  const WalletErrorKind.diskFull(),
  const WalletErrorKind.storeBusy(),
  const WalletErrorKind.walletAlreadyOpen(),
  for (final phase in LifecyclePhase.values)
    WalletErrorKind.walletBusy(phase: phase),
  for (final phase in LifecyclePhase.values)
    WalletErrorKind.invalidState(phase: phase),
  const WalletErrorKind.walletOpen(),
  const WalletErrorKind.syncServerNotOffered(),
  const WalletErrorKind.syncServerUnreachable(),
  const WalletErrorKind.wipeWithPendingSwap(count: 1),
  const WalletErrorKind.rescanWithInFlightSend(),
  WalletErrorKind.swapAddressCheckRefused(
    reason: SwapAddressCheckRefusal.values.first,
  ),
  WalletErrorKind.sync_(stall: StallReason.values.first),
  const WalletErrorKind.networkUpgradeUnsupported(
    expectedBranchId: 1,
    judgedAtHeight: 2,
  ),
  WalletErrorKind.consensusGraceExpired(by: GraceExpiry.values.first),
  const WalletErrorKind.consensusNotEvaluated(),
  const WalletErrorKind.syncRunning(),
  const WalletErrorKind.insufficientFunds(
    availableZat: 1,
    requiredZat: 2,
    pendingIncomingZat: 0,
  ),
  const WalletErrorKind.sendAmountRequired(),
  const WalletErrorKind.proposalAlreadyUsed(),
  const WalletErrorKind.proposalStale(),
  const WalletErrorKind.proposeFailed(),
  const WalletErrorKind.proposeTransient(),
  const WalletErrorKind.signFailed(),
  const WalletErrorKind.queuedSendStale(),
  const WalletErrorKind.queuedSendsFull(),
  const WalletErrorKind.texSendLimitReached(),
  const WalletErrorKind.io(),
  const WalletErrorKind.unknown(),
];

WalletApiError _err(WalletErrorKind kind) =>
    WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

/// Every sample whose list membership disagrees with the oracle, named so a
/// red reads as "which kind, which way" without a debugger.
List<String> _mismatches(
  bool Function(Object) list,
  bool Function(_Membership) column,
) => [
  for (final kind in _samples)
    if (list(_err(kind)) != column(_expected(kind)))
      '$kind: expected ${column(_expected(kind))}, list says '
          '${list(_err(kind))}',
];

void main() {
  group('R13 §4.5 row 1 — the precedes-persistence lists, kind by kind', () {
    test('every WalletErrorKind has a sample (the oracle is exhaustive by '
        'construction; this keeps its INPUTS complete)', () {
      final source = File(
        '../zec_wallet/lib/src/rust/api/error.dart',
      ).readAsStringSync();
      final factories = RegExp(
        r'const factory WalletErrorKind\.',
      ).allMatches(source).length;
      expect(factories, greaterThan(0), reason: 'the counter found the file');
      final sampled = {for (final k in _samples) k.runtimeType};
      expect(
        sampled.length,
        factories,
        reason:
            'a WalletErrorKind factory has no sample here — add one, and '
            'place the kind in _expected',
      );
    });

    test('send: sendErrorPrecedesPersistence matches the oracle', () {
      expect(_mismatches(sendErrorPrecedesPersistence, (m) => m.send), isEmpty);
    });

    test('queue: queueErrorPrecedesPersistence matches the oracle', () {
      expect(
        _mismatches(queueErrorPrecedesPersistence, (m) => m.queue),
        isEmpty,
      );
    });

    test('single-step (shield, move): singleStepSendErrorPrecedesPersistence '
        'matches the oracle', () {
      expect(
        _mismatches(
          singleStepSendErrorPrecedesPersistence,
          (m) => m.singleStep,
        ),
        isEmpty,
      );
    });

    test('parked "Send now": parkedAuthorizeErrorPrecedesPersistence matches '
        'the oracle', () {
      expect(
        _mismatches(parkedAuthorizeErrorPrecedesPersistence, (m) => m.parked),
        isEmpty,
      );
    });

    test('single-step InvalidState is phase-dependent: Closing in, Wiped out '
        '(the two rows the design names)', () {
      expect(
        singleStepSendErrorPrecedesPersistence(
          _err(
            const WalletErrorKind.invalidState(phase: LifecyclePhase.closing),
          ),
        ),
        isTrue,
        reason: 'a closed handle never reached the core',
      );
      expect(
        singleStepSendErrorPrecedesPersistence(
          _err(const WalletErrorKind.invalidState(phase: LifecyclePhase.wiped)),
        ),
        isFalse,
        reason: 'broadcast_persisted\'s wipe check runs after the tx is saved',
      );
    });

    test('a non-WalletApiError is in NO list — an untyped throw is never '
        'proof of anything', () {
      final untyped = StateError('not typed');
      expect(sendErrorPrecedesPersistence(untyped), isFalse);
      expect(queueErrorPrecedesPersistence(untyped), isFalse);
      expect(singleStepSendErrorPrecedesPersistence(untyped), isFalse);
      expect(parkedAuthorizeErrorPrecedesPersistence(untyped), isFalse);
    });
  });
}
