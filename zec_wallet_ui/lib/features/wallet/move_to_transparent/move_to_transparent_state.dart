import 'package:zec_wallet/zec_wallet.dart';

import '../send/send_state.dart';

/// The Move-to-transparent flow's states (§3.2i-1 — the Send expert layer's
/// first slice), a sealed family so the sheet renders each phase with an
/// exhaustive `switch` (no default blind spot). The flow is
/// loadAddress → amountEntry → (propose) → review → (send) → result.
///
/// Move-to-transparent is the deliberate, WARNED mirror of the Recv-3 Shield
/// action: it de-shields a chosen amount to the wallet's OWN transparent address
/// (for exchanges that reject a shielded-source deposit). It carries NO new money
/// logic — every step forwards straight to [WalletSession]
/// (`currentTransparentAddress` / `composePaymentUri` / `propose` / `send`), and
/// it REUSES the send flow's [SendFormFault] (inline propose-time faults) and
/// [SendOutcome] (the honest terminal). Rust stays the single source of truth
/// (design invariant 1); no money state is cached here.
sealed class MoveToTransparentState {
  const MoveToTransparentState();
}

/// Transient: fetching the wallet's own transparent address on sheet open. A
/// spinner.
class MoveLoadingAddress extends MoveToTransparentState {
  const MoveLoadingAddress();
}

/// The amount-entry form. [ownAddress] is the wallet's OWN transparent address —
/// the FIXED, SDK-supplied destination (shown read-only; never user input, so no
/// fat-finger risk). [fault] is an inline propose-time fault (insufficient funds,
/// not-synced yet, …) or a host-side amount parse fault — `null` on first entry.
class MoveAmountEntry extends MoveToTransparentState {
  const MoveAmountEntry({required this.ownAddress, this.fault});

  final String ownAddress;
  final SendFormFault? fault;
}

/// `composePaymentUri` + `propose` in flight — deterministic + local
/// (note-selection / fee), no network. Transient; a spinner.
class MovePreparing extends MoveToTransparentState {
  const MovePreparing(this.ownAddress);

  final String ownAddress;
}

/// The review screen: the §5.1 de-shield disclosure FIRST, the own-address note,
/// then the EXACT amount / fee / total, then [MoveToTransparentController.confirm]
/// signs + broadcasts. Holds the display [proposal] (its opaque token is consumed
/// by id on confirm) and the [ownAddress] (echoed back so the user sees WHERE the
/// funds de-shield to). [movedZat] is the amount the user typed, as parsed: the
/// review adds it to what is already public to say whether the result can be
/// shielded back (stage S14).
class MoveReady extends MoveToTransparentState {
  const MoveReady({
    required this.proposal,
    required this.ownAddress,
    required this.movedZat,
  });

  final SendProposal proposal;
  final String ownAddress;
  final int movedZat;
}

/// `send` in flight (sign in a blocking proving task + persist + broadcast).
/// Transient; a spinner. Holds the proposal so the figures stay on screen.
class MoveSubmitting extends MoveToTransparentState {
  const MoveSubmitting(this.proposal);

  final SendProposal proposal;
}

/// Terminal result of a `send` — the honest [SendOutcome] set, REUSED (a
/// broadcast miss is "saved, re-sends on the next sync", never a lie nor a fund
/// loss).
class MoveSent extends MoveToTransparentState {
  const MoveSent(this.outcome);

  final SendOutcome outcome;
}

/// The move's spend STARTED and its answer was lost (R13 §4.3): `send` threw a
/// kind that can follow persistence ([singleStepSendErrorPrecedesPersistence]
/// does not accept it), or the host's code threw or declined after the spend
/// ran. The de-shield may already be saved, so neither "moved" nor "couldn't
/// move" is true — the sheet points at Activity and offers Close only. A Try
/// again here would start a fresh propose over OTHER notes: a second
/// de-shield, a second fee. Carries no error: nothing renders a throw's text.
class MoveOutcomeUnknown extends MoveToTransparentState {
  const MoveOutcomeUnknown();
}

/// A terminal, non-form fault: the own-address fetch failed (a wedged FFI /
/// timeout — retryable), or there is no live wallet session (defensive). Distinct
/// from the inline [MoveAmountEntry] faults, which keep the amount-entry context.
class MoveUnavailable extends MoveToTransparentState {
  const MoveUnavailable(this.reason);

  final MoveFaultReason reason;
}

/// The payload-free terminal-fault categories (the honest message axis; no
/// codes; §5.4 — nothing sensitive).
enum MoveFaultReason {
  /// Couldn't load the wallet's transparent address (a wedged FFI / the
  /// honest-degradation timeout fired). Retryable.
  couldNotLoadAddress,

  /// No live wallet session (defensive — the sheet is gated to an active wallet,
  /// so this is a "go back and try again", never expected).
  walletUnavailable,
}

/// Map a `send` (sign + broadcast) failure to the next [MoveToTransparentState].
/// Mirrors the send flow's `classifySendFailure`, but routes a stale/busy fault
/// back to the AMOUNT-ENTRY (this flow's "form") rather than a send form — there
/// is no separate form screen here. Reads the typed `kind` ONLY (never a payload
/// — §5.4); a non-FRB error is the generic sign-failed terminal. Pure + total, so
/// it is unit-tested at its boundary without a device.
MoveToTransparentState classifyMoveSendFailure(
  Object error,
  String ownAddress,
) {
  if (error is WalletApiError) {
    return switch (error.kind) {
      // The token was consumed (double-tap / re-entry): the notes are NEVER
      // broadcast twice (§6.3). Honest "already submitted", not an error.
      WalletErrorKind_ProposalAlreadyUsed() => const MoveSent(
        SendAlreadySubmitted(),
      ),
      // The anchor went stale between confirm and send (the large-amount dialog
      // widens this window): the reviewed NUMBERS expired, so re-propose for
      // fresh figures (back to the amount form), never retry the same token.
      WalletErrorKind_ProposalStale() => MoveAmountEntry(
        ownAddress: ownAddress,
        fault: const SendCategoricalFault(SendFaultReason.amountsExpired),
      ),
      // storeBusy joins for parity with the send flow (#373 follow-up): a write that
      // lost its race to a sync commit wrote nothing — the honest transient, never the
      // red sign-failed dead-end.
      WalletErrorKind_WalletBusy() ||
      WalletErrorKind_InvalidState() ||
      WalletErrorKind_StoreBusy() => MoveAmountEntry(
        ownAddress: ownAddress,
        fault: const SendCategoricalFault(SendFaultReason.walletBusy),
      ),
      // Out of disk persisting the created move-tx (#373 follow-up): the honest "free up
      // space", routed BACK TO THE FORM (nothing broadcast, funds untouched), never the
      // generic "Couldn't move" that retries into a deterministic re-fail on a full disk.
      WalletErrorKind_DiskFull() => MoveAmountEntry(
        ownAddress: ownAddress,
        fault: const SendCategoricalFault(SendFaultReason.storageFull),
      ),
      // signFailed (or anything else): no money moved; the consumed token can't
      // be re-sent, so the result offers "try again" → a fresh flow.
      _ => const MoveSent(SendSignFailed()),
    };
  }
  return const MoveSent(SendSignFailed());
}
