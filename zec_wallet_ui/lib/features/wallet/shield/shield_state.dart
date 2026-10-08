import 'package:zec_wallet/zec_wallet.dart';

import '../send/send_state.dart';

/// The shield flow's states (Recv-3), a sealed family the sheet renders with an
/// exhaustive `switch`. The flow is: open → (proposeShield) → ready | nothing,
/// then ready → (send) → done. Rendering layer ONLY — every transition runs
/// through [ShieldController]; no money state is stored here (design invariant 1).
///
/// A shield REUSES the send pipeline whole, so the terminal outcome reuses the
/// send flow's [SendOutcome] (succeeded / saved-for-retry / already-submitted /
/// sign-failed) verbatim — a shield broadcast is just a `send` of a one-shot
/// token (DRY; the shield-specific copy lives in the sheet, not a parallel type).
sealed class ShieldState {
  const ShieldState();
}

/// Initial / reset — nothing prepared yet.
class ShieldIdle extends ShieldState {
  const ShieldIdle();
}

/// `proposeShield` in flight — deterministic + local (note-selection/fee), no
/// network. Transient; a spinner.
class ShieldPreparing extends ShieldState {
  const ShieldPreparing();
}

/// The confirm sheet: the EXACT numbers — gross transparent ([proposal.totalZat]),
/// fee ([proposal.feeZat]), net that lands shielded ([proposal.changeZat]) — then
/// [ShieldController.confirm] signs + broadcasts. Holds the display [proposal]
/// (its opaque token is consumed by id on confirm).
class ShieldReady extends ShieldState {
  const ShieldReady(this.proposal);

  final SendProposal proposal;
}

/// `proposeShield` returned `null` — the transparent balance is below the
/// shieldable minimum (the fee would outweigh the benefit). An honest no-op the
/// sheet explains, NEVER an error.
class ShieldNothingToShield extends ShieldState {
  const ShieldNothingToShield();
}

/// `send` (sign + broadcast the shield tx) in flight. Transient; a spinner. Holds
/// the proposal so the sheet can keep showing the figures.
class ShieldSubmitting extends ShieldState {
  const ShieldSubmitting(this.proposal);

  final SendProposal proposal;
}

/// Terminal result of the shield's `send` — the reused [SendOutcome] (the shield
/// IS a send of the retained token, §3.3a Recv-3).
class ShieldDone extends ShieldState {
  const ShieldDone(this.outcome);

  final SendOutcome outcome;
}

/// The shield's spend STARTED and its answer was lost (R13 §4.3): `send` threw a
/// kind that can follow persistence ([singleStepSendErrorPrecedesPersistence]
/// does not accept it), or the host's code threw or declined after the spend
/// ran. The transaction may already be saved, so neither "shielded" nor
/// "couldn't shield" is true — the sheet points at Activity and offers Close
/// only. A Try again here would re-propose over a saved shield. Carries no
/// error: nothing renders a throw's text.
class ShieldOutcomeUnknown extends ShieldState {
  const ShieldOutcomeUnknown();
}

/// A recoverable prepare/confirm fault that did NOT move money: the wallet isn't
/// synced far enough to anchor the shield ([notSyncedYet]), is mid-lifecycle
/// ([walletBusy]), has no live session ([walletUnavailable]), or the shield could
/// not be prepared/signed for a reason with no finer mapping ([couldNotPrepare]).
/// The sheet offers "try again" (re-prepare) or honest copy — never a code (§6).
class ShieldUnavailable extends ShieldState {
  const ShieldUnavailable(this.reason);

  final ShieldFaultReason reason;
}

/// The payload-free shield-fault categories (the honest message axis; no codes).
enum ShieldFaultReason {
  /// The wallet isn't synced far enough to anchor a shield proposal yet — wait
  /// for sync, then retry.
  notSyncedYet,

  /// The wallet is mid-lifecycle (closing/repairing) — retry in a moment.
  walletBusy,

  /// No live wallet session (defensive — the sheet is reachable only from an
  /// active wallet, so this is a "go back and try again", never expected).
  walletUnavailable,

  /// The device is out of disk space, so preparing/persisting the shield hit
  /// `DiskFull` (#373 follow-up — the shield sibling of Send's `storageFull`).
  /// Retrying WITHOUT freeing space deterministically re-fails, so the copy asks
  /// for space instead of "try again"; no funds moved.
  storageFull,

  /// Couldn't prepare/sign the shield for a reason with no finer mapping that is
  /// DETERMINISTIC on the wallet's state; no funds moved. The retryable class is
  /// [couldNotPrepareTransient].
  couldNotPrepare,

  /// Couldn't prepare the shield because of a condition the wallet's OWN state
  /// clears without the user doing anything — an anchor not yet recorded, an
  /// input a concurrent proposal holds, a witness the scan has not completed
  /// (`WalletErrorKind.proposeTransient`, INC-018 (b); the SAME Rust classifier
  /// Send reads, `send::propose_refusal_is_transient`, so the two flows cannot
  /// drift). The sheet says "try again in a moment" with the retry offered —
  /// never the title-only dead-end [couldNotPrepare] renders. Found by the
  /// Batch C arch pass: P2-2 first reached only the Send classifier.
  couldNotPrepareTransient,
}

/// Map a `proposeShield` failure to a [ShieldState]. Reads the typed
/// `WalletApiError.kind` ONLY (never a payload — §5.4); a non-FRB error is the
/// generic [ShieldFaultReason.couldNotPrepare]. Pure + total, unit-tested at its
/// boundary without a device.
ShieldState classifyShieldPrepareFailure(Object error) {
  if (error is WalletApiError) {
    return switch (error.kind) {
      WalletErrorKind_ProposalStale() => const ShieldUnavailable(
        ShieldFaultReason.notSyncedYet,
      ),
      // storeBusy joins this arm for parity with the send flow (#373): a shield
      // write that lost its race to a sync commit (past the SDK's bounded retry)
      // wrote nothing — the honest "busy, try again in a moment", never a
      // generic "couldn't prepare" / "sign failed" dead-end.
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_InvalidState() ||
      WalletErrorKind_StoreBusy() => const ShieldUnavailable(
        ShieldFaultReason.walletBusy,
      ),
      // Out of disk (#373 follow-up): the honest "free up space", never a generic
      // "couldn't prepare" that retries into a deterministic re-fail on a full disk.
      WalletErrorKind_DiskFull() => const ShieldUnavailable(
        ShieldFaultReason.storageFull,
      ),
      // INC-018 (b), phase-2 P2-2 (the Batch C arch pass): the retryable class
      // the SDK types apart reaches Shield as it reaches Send — its OWN arm, so
      // the sheet says "try again in a moment" and never the title-only
      // dead-end the wildcard renders.
      WalletErrorKind_ProposeTransient() => const ShieldUnavailable(
        ShieldFaultReason.couldNotPrepareTransient,
      ),
      _ => const ShieldUnavailable(ShieldFaultReason.couldNotPrepare),
    };
  }
  return const ShieldUnavailable(ShieldFaultReason.couldNotPrepare);
}

/// Map a shield `send` (sign+broadcast) failure to the next [ShieldState]. A
/// consumed token is the honest "already submitted" (the notes are never
/// broadcast twice, §6.3); a stale anchor routes to re-prepare; everything else
/// is a no-money-moved sign failure. Reads `kind` only (§5.4) — mirrors the send
/// flow's `classifySendFailure`, projected onto the shield states.
ShieldState classifyShieldSendFailure(Object error) {
  if (error is WalletApiError) {
    return switch (error.kind) {
      WalletErrorKind_ProposalAlreadyUsed() => const ShieldDone(
        SendAlreadySubmitted(),
      ),
      WalletErrorKind_ProposalStale() => const ShieldUnavailable(
        ShieldFaultReason.notSyncedYet,
      ),
      // storeBusy joins this arm for parity with the send flow (#373): a shield
      // write that lost its race to a sync commit (past the SDK's bounded retry)
      // wrote nothing — the honest "busy, try again in a moment", never a
      // generic "couldn't prepare" / "sign failed" dead-end.
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_InvalidState() ||
      WalletErrorKind_StoreBusy() => const ShieldUnavailable(
        ShieldFaultReason.walletBusy,
      ),
      // Out of disk persisting the shield tx (#373 follow-up): nothing broadcast
      // (§6.3), so route to the honest "free up space" retryable state, never the
      // terminal "couldn't shield" dead-end that would loop on a full disk.
      WalletErrorKind_DiskFull() => const ShieldUnavailable(
        ShieldFaultReason.storageFull,
      ),
      _ => const ShieldDone(SendSignFailed()),
    };
  }
  return const ShieldDone(SendSignFailed());
}
