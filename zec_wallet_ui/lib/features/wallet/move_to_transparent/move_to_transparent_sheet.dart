import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/address_text.dart';
import '../../../shared/decimal_input_formatter.dart';
import '../../../shared/sheet_states.dart';
import '../../../shared/wallet_outcome_unknown.dart';
import '../../../shared/wallet_sheet.dart';
import '../deshield_warning.dart';
import '../labeled_zat_row.dart';
import '../recoverable_ephemeral.dart' show summarizeRecoverable;
import '../send/form_fault_view.dart';
import '../send/large_send_confirm.dart';
import '../send/send_state.dart';
import '../sync_status_presentation.dart' show walletSyncPausedQualified;
import '../transparent_funds/transparent_funds_providers.dart';
import '../wallet_providers.dart';
import '../wallet_sync_controller.dart' show walletSyncPassesRunProvider;
import '../wallet_rescan_controller.dart'
    show WalletCatchUpNone, walletCatchUpCueProvider;
import '../zat_format.dart';
import 'move_public_after.dart';
import 'move_to_transparent_controller.dart';
import 'move_to_transparent_state.dart';

/// Open the Move-to-transparent expert action (§3.2i-1) as a modal bottom sheet
/// (reached from the wallet overflow menu — the "expert layer" home). It is the
/// deliberate, WARNED mirror of the Recv-3 Shield action: it de-shields a chosen
/// amount to the wallet's OWN transparent address (for an exchange that rejects a
/// shielded-source deposit). Self-contained: it loads the own t-addr on open,
/// takes an amount, proposes + shows the §5.1 de-shield disclosure, signs +
/// broadcasts on confirm, then reports the honest outcome.
Future<void> showMoveToTransparentSheet(BuildContext context) {
  // The shared sheet frame, whose width cap keeps the narrow form from
  // stretching into a full-width strip on a wide desktop window.
  //
  // S13 §1.3 — the rescan sheet's shape, as Shield: scrim and drag locked
  // (a drag pops directly, past any PopScope), Back held while submitting.
  return showWalletSheet<void>(
    context,
    isDismissible: false,
    enableDrag: false,
    builder: (_) => const MoveToTransparentSheet(),
  );
}

/// The move sheet body (public for widget tests — pump it directly in a
/// `ProviderScope` over a `FakeWalletSession`).
class MoveToTransparentSheet extends ConsumerStatefulWidget {
  const MoveToTransparentSheet({super.key});

  @override
  ConsumerState<MoveToTransparentSheet> createState() =>
      _MoveToTransparentSheetState();
}

class _MoveToTransparentSheetState
    extends ConsumerState<MoveToTransparentSheet> {
  final _amountController = TextEditingController();

  @override
  void initState() {
    super.initState();
    // Load the own t-addr after the first frame so the notifier mutation is legal
    // and the post-frame mounted re-check holds.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      ref.read(moveToTransparentControllerProvider.notifier).start();
    });
  }

  @override
  void dispose() {
    _amountController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Self-heal a mid-sheet wallet-session flip: the controller's build() resets
    // to MoveLoadingAddress on a session change but does NOT re-fetch on its own,
    // so re-drive `start()` here. `ref.listen` fires only on a CHANGE (never the
    // initial value), so it never double-runs with the initState mount call.
    // The typed amount clears too (wrap review — identity-switch hygiene:
    // the OLD identity's draft must never render in the NEW identity's sheet).
    ref.listen(walletSessionProvider, (_, _) {
      _amountController.clear();
      ref.read(moveToTransparentControllerProvider.notifier).start();
    });
    final state = ref.watch(moveToTransparentControllerProvider);
    return PopScope<Object?>(
      // S13 §1.3: no leaving by Back while the move signs and broadcasts —
      // Back asks instead, as on the shield sheet.
      canPop: state is! MoveSubmitting,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) unawaited(confirmLeaveSheetWhileSending(context));
      },
      child: SafeArea(
        child: Padding(
          // Pad out the keyboard so the amount field stays visible.
          padding: EdgeInsets.fromLTRB(
            20,
            4,
            20,
            24 + MediaQuery.of(context).viewInsets.bottom,
          ),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [_body(context, state)],
            ),
          ),
        ),
      ),
    );
  }

  Widget _body(BuildContext context, MoveToTransparentState state) {
    final l10n = WalletLocalizations.of(context);
    return switch (state) {
      MoveLoadingAddress() => WalletSheetBusy(label: l10n.walletMoveLoading),
      MoveAmountEntry(:final ownAddress, :final fault) => _AmountEntry(
        ownAddress: ownAddress,
        fault: fault,
        controller: _amountController,
        onReview: () => ref
            .read(moveToTransparentControllerProvider.notifier)
            .prepare(_amountController.text),
        onCancel: () => Navigator.of(context).maybePop(),
      ),
      MovePreparing() => WalletSheetBusy(label: l10n.walletMovePreparing),
      MoveReady(:final proposal, :final ownAddress, :final movedZat) => _Review(
        proposal: proposal,
        ownAddress: ownAddress,
        movedZat: movedZat,
        onConfirm: () => unawaited(_confirm(context, proposal)),
        onBack: () =>
            ref.read(moveToTransparentControllerProvider.notifier).backToForm(),
      ),
      MoveSubmitting() => WalletSheetBusy(label: l10n.walletMoveSubmitting),
      MoveSent(:final outcome) => _outcome(context, outcome),
      // R13 §4.3: the move may already be saved — Close only, never a Try
      // again whose fresh propose would de-shield OTHER notes a second time.
      MoveOutcomeUnknown() => SizedBox(
        width: double.infinity,
        child: WalletOutcomeUnknownView(
          title: l10n.walletMoveUnknownTitle,
          body: l10n.walletMoveUnknownBody,
          closeLabel: l10n.walletMoveClose,
          onClose: () => Navigator.of(context).maybePop(),
        ),
      ),
      MoveUnavailable(:final reason) => _unavailable(context, reason),
    };
  }

  /// The confirm action — fires the SAME large-send confirmation a payment does
  /// (money-safety parity; the SDK's `largeSend` rides the proposal), THEN signs.
  /// Captures the notifier before the await so nothing touches `context`/`ref`
  /// across the dialog gap (lint-clean, dispose-safe).
  Future<void> _confirm(BuildContext context, SendProposal proposal) async {
    final notifier = ref.read(moveToTransparentControllerProvider.notifier);
    final reason = proposal.largeSend;
    if (reason != null) {
      final approved = await showLargeSendConfirm(
        context,
        totalZat: proposal.totalZat,
        reason: reason,
      );
      if (approved != true) return; // cancelled / dismissed — stay on review
    }
    // A light tap under the thumb for the money confirm (S13 §1.7).
    unawaited(HapticFeedback.lightImpact());
    unawaited(notifier.confirm());
  }

  Widget _outcome(BuildContext context, SendOutcome outcome) {
    final l10n = WalletLocalizations.of(context);
    return switch (outcome) {
      SendSucceeded() => WalletSheetResult(
        icon: WalletGlyph.transparent,
        title: l10n.walletMoveDoneTitle,
        body: l10n.walletMoveDoneBody,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletMoveClose,
      ),
      // A z→own-t move has no TEX destination, so SendTexInMotion is structurally
      // unreachable here; folded into the saved rendering for switch-exhaustiveness
      // and a money-safe fallback if a future path ever produced one.
      SendSavedForRetry() || SendTexInMotion() => WalletSheetResult(
        icon: WalletGlyph.savedForRetry,
        title: l10n.walletMoveSavedTitle,
        // QUALIFIED when no sync pass will run (#401 R1b/R5) — same rule as the
        // shield and send saved-for-retry arms: the body promises a later sync
        // finishes this, and `after_synced` is the only thing that drives it.
        body: walletSyncPausedQualified(
          l10n,
          l10n.walletMoveSavedBody,
          syncPassesRun: ref.watch(walletSyncPassesRunProvider),
        ),
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletMoveClose,
      ),
      // Kept, no retry promise from the wallet (stage S8 `obligation`, row 10):
      // the shared "saved" copy that points at Activity for the live state.
      SendKept() => WalletSheetResult(
        icon: WalletGlyph.saved,
        title: l10n.walletSendKeptTitle,
        body: l10n.walletSendKeptBody,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletMoveClose,
      ),
      SendAlreadySubmitted() => WalletSheetResult(
        icon: WalletGlyph.transparent,
        title: l10n.walletMoveAlreadyTitle,
        body: l10n.walletMoveAlreadyBody,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletMoveClose,
      ),
      SendSignFailed() => WalletSheetResult(
        icon: WalletGlyph.error,
        title: l10n.walletMoveFailedTitle,
        body: null,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletMoveClose,
        // The token is consumed on a sign failure — re-propose from scratch
        // (reload the address + a fresh amount), never retry the dead token.
        onRetry: () =>
            ref.read(moveToTransparentControllerProvider.notifier).start(),
        retryLabel: l10n.walletMoveRetry,
      ),
    };
  }

  Widget _unavailable(BuildContext context, MoveFaultReason reason) {
    final l10n = WalletLocalizations.of(context);
    return WalletSheetResult(
      icon: WalletGlyph.error,
      title: l10n.walletMoveFailedTitle,
      // Both reasons get an honest body (no silent title-only failure — the
      // #251 no-dead-end lesson): couldNotLoad is retryable; walletUnavailable
      // (the session ended mid-sheet) is a "reopen" defensive path.
      body: reason == MoveFaultReason.couldNotLoadAddress
          ? l10n.walletMoveCouldNotLoad
          : l10n.walletMoveWalletEnded,
      onClose: () => Navigator.of(context).maybePop(),
      closeLabel: l10n.walletMoveClose,
      // couldNotLoadAddress is retryable (re-fetch the address); walletUnavailable
      // is a defensive "go back" with nothing to retry.
      onRetry: reason == MoveFaultReason.couldNotLoadAddress
          ? () => ref
                .read(moveToTransparentControllerProvider.notifier)
                .retryLoad()
          : null,
      retryLabel: l10n.walletMoveRetry,
    );
  }
}

/// The amount-entry phase: title + a muted de-shield note + the read-only OWN
/// transparent destination + the amount field (with the available-shielded cap
/// hint) + an inline fault + the Review / Cancel actions. Reads the cold snapshot
/// for the spendable figure; with ZERO shielded spendable it shows the honest
/// "nothing to move yet" instead of a form (never a dead-end).
class _AmountEntry extends ConsumerWidget {
  const _AmountEntry({
    required this.ownAddress,
    required this.fault,
    required this.controller,
    required this.onReview,
    required this.onCancel,
  });

  final String ownAddress;
  final SendFormFault? fault;
  final TextEditingController controller;
  final VoidCallback onReview;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // The available-to-move figure — shielded spendable (the §2.5 SSOT), the
    // source of a de-shield. Last-known through a transient reload error.
    final spendable = ref
        .watch(walletSnapshotProvider)
        .value
        ?.balance
        .spendableZat;

    // Honest empty state: nothing shielded to move yet (never a dead spinner /
    // a form that can only fail). `null` (snapshot still loading) is NOT zero —
    // show the form and let propose be the binding gate. Mid-catch-up the
    // cause is UNSCANNED funds, not unconfirmed ones (#381 (b), hardware-
    // proven): the default body's "Once funds confirm…" would
    // misattribute, so the cue swaps in the catching-up explanation.
    if (spendable == 0) {
      // The catching-up body claims active progress — dropped whenever no
      // sync pass will run (the qualifier-family rule; #405 → the
      // SSOT, so a FAILED start drops it too): the default body's confirm
      // framing is the lesser misread there, and the ambient sync story owns
      // the why.
      final catchingUp =
          ref.watch(walletCatchUpCueProvider) is! WalletCatchUpNone &&
          ref.watch(walletSyncPassesRunProvider);
      return WalletSheetResult(
        icon: WalletGlyph.info,
        title: l10n.walletMoveNothingTitle,
        body: catchingUp
            ? l10n.walletMoveNothingCatchingUpBody
            : l10n.walletMoveNothingBody,
        onClose: onCancel,
        closeLabel: l10n.walletMoveClose,
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        WalletSheetHeader(
          title: l10n.walletMoveSheetTitle,
          closeKey: const Key('move-sheet-close'),
          onClose: onCancel,
        ),
        const SizedBox(height: 8),
        Text(
          l10n.walletMoveSheetSubtitle,
          style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
        ),
        const SizedBox(height: 20),

        // The fixed, SDK-supplied destination — the wallet's OWN t-addr, shown
        // read-only (never user input). Glyph-by-glyph monospace (the shared
        // verification brick) so the user can recognise it as theirs.
        Text(
          l10n.walletMoveDestinationLabel,
          style: textTheme.labelMedium?.copyWith(color: colors.textMuted),
        ),
        const SizedBox(height: 4),
        AddressVerificationText(ownAddress),
        const SizedBox(height: 20),

        if (spendable != null) ...[
          Text(
            // Qualified while catching up (#381 (b)) — same rule as the send
            // and swap Available lines: a partial figure must not read final.
            // The qualifier drops when no pass will run (#405 → the
            // SSOT): no active-progress claim while nothing runs; the badge
            // story carries the caveat.
            ref.watch(walletCatchUpCueProvider) is WalletCatchUpNone ||
                    !ref.watch(walletSyncPassesRunProvider)
                ? l10n.walletMoveAvailable(formatZec(spendable))
                : l10n.walletMoveAvailableCatchingUp(formatZec(spendable)),
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
          const SizedBox(height: 8),
        ],
        TextField(
          controller: controller,
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          // Money is integer-exact: digits + one separator (a comma reads as the
          // point) at the input layer, and size-cap before the value reaches the
          // FFI (the parser is the real gate). 20 > the longest valid amount;
          // an over-long edit is refused whole, never truncated.
          inputFormatters: const [WalletDecimalInputFormatter(maxLength: 20)],
          decoration: InputDecoration(
            labelText: l10n.walletSendAmountLabel,
            hintText: l10n.walletSendAmountHint,
            hintStyle: TextStyle(color: colors.textMuted),
          ),
        ),
        if (fault != null) ...[
          const SizedBox(height: 16),
          SendFormFaultView(fault: fault!),
        ],
        const SizedBox(height: 24),
        SizedBox(
          width: double.infinity,
          child: FilledButton(
            onPressed: onReview,
            child: Text(l10n.walletMoveReviewButton),
          ),
        ),
        const SizedBox(height: 8),
        SizedBox(
          width: double.infinity,
          child: TextButton(
            onPressed: onCancel,
            child: Text(l10n.walletMoveCancel),
          ),
        ),
      ],
    );
  }
}

/// The confirm phase: the §5.1 de-shield disclosure FIRST (the shared warning),
/// the own-address irreversibility note, the destination echo, the exact
/// amount / fee / total, then the Confirm / Back actions.
class _Review extends ConsumerWidget {
  const _Review({
    required this.proposal,
    required this.ownAddress,
    required this.movedZat,
    required this.onConfirm,
    required this.onBack,
  });

  final SendProposal proposal;
  final String ownAddress;
  final int movedZat;
  final VoidCallback onConfirm;
  final VoidCallback onBack;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    // §3.2i-3 / security review MAJOR-3: with auto-shield ON (the
    // default), the loop will shield these funds RIGHT BACK once they arrive
    // — silently reverting this deliberate action and burning a second fee.
    // The user must learn that HERE, at the money confirm, not discover it
    // from a vanished transparent balance. Loading reads as ON (the shipped
    // default — over-warning is the safe direction). EFFECTIVE, not the raw
    // switch (#383 R2 / fold): on an auto-shield-unsupported host the
    // loop never arms and the toggle this note points at is hidden, so the
    // raw-switch gate asserted an automation (and a second fee) that cannot
    // happen — at a money confirm.
    //
    // S14 (maintainer, warn): neither promise holds when the move
    // leaves the public balance under the core's shield floor — no Shield,
    // manual or automatic, takes it until more arrives. The review then says
    // so with the figure, and drops "shielded back automatically" and "you can
    // shield these again later". The auto note also needs the host's loop
    // threshold met, or it promises a sweep the loop will not make. Read live,
    // not frozen at prepare(): the proposal pins the money, and a provider
    // refresh during the review only brings the figure closer to the truth.
    final transparentZat = ref
        .watch(walletSnapshotProvider)
        .value
        ?.balance
        .transparentZat;
    // A list not yet read (or failed) stays null: see [movePublicAfterZat].
    final recoverable = ref
        .watch(walletRecoverableEphemeralFundsProvider)
        .value;
    final publicAfter = movePublicAfterZat(
      transparentZat: transparentZat,
      recoverableZat: recoverable == null
          ? null
          : summarizeRecoverable(recoverable).totalZat,
      movedZat: movedZat,
    );
    final belowFloor = moveLeavesPublicBelowFloor(publicAfter);
    final autoShieldOn = moveReviewPromisesAutoShield(
      publicAfterZat: publicAfter,
      rawPublicAfterZat: moveRawPublicAfterZat(
        transparentZat: transparentZat,
        movedZat: movedZat,
      ),
      autoShieldEffective: ref.watch(walletAutoShieldEffectiveProvider),
      hostThresholdZat: ref.watch(walletAutoShieldThresholdZatProvider),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        WalletSheetHeader(
          title: l10n.walletMoveReviewTitle,
          closeKey: const Key('move-sheet-close'),
        ),
        const SizedBox(height: 16),

        // The de-shield disclosure FIRST (most load-bearing) — never let the
        // headline number bury the privacy loss. Move-specific copy: this is a
        // self-transfer, not a payment to a third party, so the generic
        // "recipient will be visible" Send wording would misread.
        DeshieldWarning(
          title: l10n.walletMoveDeshieldTitle,
          body: l10n.walletMoveDeshieldBody,
        ),
        const SizedBox(height: 12),
        // The move-specific honesty: it's YOUR OWN transparent address, and the
        // public on-chain record is permanent (the funds can be re-shielded).
        Text(
          belowFloor
              ? l10n.walletMoveOwnAddressNoteStaysPublic
              : l10n.walletMoveOwnAddressNote,
          style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
        ),
        if (belowFloor) ...[
          const SizedBox(height: 8),
          Text(
            l10n.walletMoveBelowFloorNote(
              formatZec(publicAfter),
              formatZec(walletAutoShieldFloorZat),
            ),
            key: const ValueKey('move-below-floor-note'),
            style: textTheme.bodySmall?.copyWith(color: colors.orange),
          ),
        ],
        if (autoShieldOn) ...[
          const SizedBox(height: 8),
          Text(
            l10n.walletMoveAutoShieldNote,
            key: const ValueKey('move-auto-shield-note'),
            style: textTheme.bodySmall?.copyWith(color: colors.orange),
          ),
        ],
        const SizedBox(height: 16),

        // Where — the OWN t-addr echoed back, glyph-by-glyph.
        Text(
          l10n.walletMoveDestinationLabel,
          style: textTheme.labelMedium?.copyWith(color: colors.textMuted),
        ),
        const SizedBox(height: 4),
        AddressVerificationText(ownAddress),
        const SizedBox(height: 16),

        LabeledZatRow(
          label: l10n.walletSendTotalLabel,
          amountZat: proposal.totalZat,
          emphasize: true,
        ),
        const SizedBox(height: 8),
        LabeledZatRow(
          label: l10n.walletSendFeeLabel,
          amountZat: proposal.feeZat,
        ),
        if (proposal.changeZat > 0) ...[
          const SizedBox(height: 8),
          LabeledZatRow(
            label: l10n.walletSendChangeLabel,
            amountZat: proposal.changeZat,
          ),
        ],
        const SizedBox(height: 24),
        SizedBox(
          width: double.infinity,
          // Mirror the Shield sheet's defensive a11y wrapper so the icon never
          // splits the button's semantics for a screen reader.
          child: Semantics(
            container: true,
            button: true,
            label: l10n.walletMoveConfirmButton,
            // …and it mirrors the shield sheet's DEFECT too: `ExcludeSemantics`
            // deletes the button's node and its tap action, so the action lives
            // on THIS node, bound to the same callback (Relim `0c5ae1bd`,
            // #661/#721).
            onTap: onConfirm,
            child: ExcludeSemantics(
              child: FilledButton.icon(
                onPressed: onConfirm,
                icon: const WalletIcon(WalletGlyph.transparent),
                label: Text(l10n.walletMoveConfirmButton),
              ),
            ),
          ),
        ),
        const SizedBox(height: 8),
        SizedBox(
          width: double.infinity,
          child: TextButton(
            onPressed: onBack,
            child: Text(l10n.walletMoveBackButton),
          ),
        ),
      ],
    );
  }
}
