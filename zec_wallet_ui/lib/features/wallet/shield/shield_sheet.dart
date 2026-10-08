import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/sheet_states.dart';
import '../../../shared/wallet_outcome_unknown.dart';
import '../../../shared/wallet_sheet.dart';
import '../labeled_zat_row.dart';
import '../send/send_state.dart';
import '../sync_status_presentation.dart' show walletSyncPausedQualified;
import '../wallet_providers.dart';
import '../wallet_sync_controller.dart' show walletSyncPassesRunProvider;
import 'shield_controller.dart';
import 'shield_state.dart';

/// Open the Recv-3 shield flow as a modal bottom sheet (reached from the balance
/// card's "Shield" action). Self-contained: it proposes the shield on open, shows
/// the confirmable numbers (gross / fee / net shielded), signs+broadcasts on
/// confirm, then reports the honest outcome. Privacy-POSITIVE — it shows a
/// "moving to your private balance" note, NEVER a de-shield warning.
Future<void> showShieldSheet(BuildContext context) {
  // The shared sheet frame, whose width cap keeps the narrow confirm from
  // stretching into a full-width strip on a wide desktop window.
  //
  // S13 §1.3 — the rescan sheet's shape: scrim and drag locked for the
  // sheet's life (a drag closes a sheet by a direct pop, which no PopScope
  // can hold), Back held by the sheet itself while the shield submits. Every
  // settled state carries its own Cancel or Close.
  return showWalletSheet<void>(
    context,
    isDismissible: false,
    enableDrag: false,
    builder: (_) => const ShieldSheet(),
  );
}

/// The shield sheet body (public for widget tests — pump it directly in a
/// `ProviderScope` over a `FakeWalletSession`).
class ShieldSheet extends ConsumerStatefulWidget {
  const ShieldSheet({super.key});

  @override
  ConsumerState<ShieldSheet> createState() => _ShieldSheetState();
}

class _ShieldSheetState extends ConsumerState<ShieldSheet> {
  @override
  void initState() {
    super.initState();
    // Reset any prior terminal state, then propose — after the first frame so the
    // notifier mutation is legal and the post-frame mounted re-check holds.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final controller = ref.read(shieldControllerProvider.notifier);
      controller.reset();
      controller.prepare();
    });
  }

  @override
  Widget build(BuildContext context) {
    // Self-heal a mid-sheet wallet-session flip (review, the move sheet's
    // pattern): the controller's build() resets to ShieldIdle on a session
    // change but does NOT re-propose on its own — and this sheet renders Idle
    // as the same spinner as Preparing, so without this the flip left an
    // infinite "Preparing…". `ref.listen` fires only on a CHANGE (never the
    // initial value), so it never double-runs with the initState mount call;
    // post-flip the controller is idle, so reset() no-ops and prepare() runs
    // against the NEW session.
    ref.listen(walletSessionProvider, (_, _) {
      final controller = ref.read(shieldControllerProvider.notifier);
      controller.reset();
      controller.prepare();
    });
    final state = ref.watch(shieldControllerProvider);
    return PopScope<Object?>(
      // S13 §1.3: no leaving by Back while the shield signs and broadcasts —
      // Back asks instead, so an unbounded submit never strands the user
      // (the diff review MEDIUM; the maintainer's words).
      canPop: state is! ShieldSubmitting,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) unawaited(confirmLeaveSheetWhileSending(context));
      },
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 4, 20, 24),
          // SCROLLABLE, like its move-to-transparent sibling (#403 R8c). This
          // sheet shipped without one, and #401 R1b then APPENDED the
          // sync-paused qualifier to its saved-for-retry body — so at large
          // text scale the honesty note is the sacrificial tail, clipped first
          // because it is last (measured: +624px overflow at 320×640). A money
          // surface whose added truth is the first thing to fall off the bottom
          // is worse than one that never carried it.
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

  Widget _body(BuildContext context, ShieldState state) {
    return switch (state) {
      ShieldIdle() || ShieldPreparing() => WalletSheetBusy(
        label: WalletLocalizations.of(context).walletShieldPreparing,
      ),
      ShieldReady(:final proposal) => _Review(
        proposal: proposal,
        onConfirm: () {
          // A light tap under the thumb for the money confirm (S13 §1.7).
          unawaited(HapticFeedback.lightImpact());
          ref.read(shieldControllerProvider.notifier).confirm();
        },
        onCancel: () => Navigator.of(context).maybePop(),
      ),
      ShieldSubmitting() => WalletSheetBusy(
        label: WalletLocalizations.of(context).walletShieldSubmitting,
      ),
      ShieldNothingToShield() => WalletSheetResult(
        icon: WalletGlyph.info,
        title: WalletLocalizations.of(context).walletShieldNothingTitle,
        body: WalletLocalizations.of(context).walletShieldNothingBody,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: WalletLocalizations.of(context).walletShieldClose,
      ),
      ShieldDone(:final outcome) => _outcome(context, outcome),
      // R13 §4.3: the shield may already be saved — Close only, never a Try
      // again that would re-propose over it.
      ShieldOutcomeUnknown() => SizedBox(
        width: double.infinity,
        child: WalletOutcomeUnknownView(
          title: WalletLocalizations.of(context).walletShieldUnknownTitle,
          body: WalletLocalizations.of(context).walletShieldUnknownBody,
          closeLabel: WalletLocalizations.of(context).walletShieldClose,
          onClose: () => Navigator.of(context).maybePop(),
        ),
      ),
      ShieldUnavailable(:final reason) => _unavailable(context, reason),
    };
  }

  Widget _outcome(BuildContext context, SendOutcome outcome) {
    final l10n = WalletLocalizations.of(context);
    return switch (outcome) {
      SendSucceeded() => WalletSheetResult(
        icon: WalletGlyph.shielded,
        title: l10n.walletShieldDoneTitle,
        body: l10n.walletShieldDoneBody,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletShieldClose,
      ),
      // A t→z shield has no TEX destination, so SendTexInMotion is structurally
      // unreachable here; folded into the saved rendering for switch-exhaustiveness
      // and a money-safe fallback if a future path ever produced one.
      SendSavedForRetry() || SendTexInMotion() => WalletSheetResult(
        icon: WalletGlyph.savedForRetry,
        title: l10n.walletShieldSavedTitle,
        // NOT walletShieldDoneBody — an unbroadcast tx is NOT confirming shortly; it
        // re-sends on a later sync (honest, money-safe). QUALIFIED when no sync pass
        // will run (#401 R1b/R5): the re-send rides `after_synced`, so without passes
        // the promise the body makes cannot be kept and the user is left believing a
        // signed transaction is handled when nothing will touch it again.
        body: walletSyncPausedQualified(
          l10n,
          l10n.walletShieldSavedBody,
          syncPassesRun: ref.watch(walletSyncPassesRunProvider),
        ),
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletShieldClose,
      ),
      // Kept, no retry promise from the wallet (stage S8 `obligation`, row 10):
      // the shared "saved" copy that points at Activity for the live state.
      SendKept() => WalletSheetResult(
        icon: WalletGlyph.saved,
        title: l10n.walletSendKeptTitle,
        body: l10n.walletSendKeptBody,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletShieldClose,
      ),
      SendAlreadySubmitted() => WalletSheetResult(
        icon: WalletGlyph.shielded,
        title: l10n.walletShieldAlreadyTitle,
        body: l10n.walletShieldDoneBody,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletShieldClose,
      ),
      SendSignFailed() => WalletSheetResult(
        icon: WalletGlyph.error,
        title: l10n.walletShieldFailedTitle,
        body: null,
        onClose: () => Navigator.of(context).maybePop(),
        closeLabel: l10n.walletShieldClose,
        onRetry: () => ref.read(shieldControllerProvider.notifier).prepare(),
        retryLabel: l10n.walletShieldRetry,
      ),
    };
  }

  Widget _unavailable(BuildContext context, ShieldFaultReason reason) {
    final l10n = WalletLocalizations.of(context);
    final body = switch (reason) {
      ShieldFaultReason.notSyncedYet => l10n.walletShieldStaleBody,
      ShieldFaultReason.walletBusy => l10n.walletShieldStaleBody,
      // Reachable via the session-flip self-heal — a title-only
      // "Couldn't shield" left the user with no idea what to do (wrap
      // review, fixed); mirrors the move sheet's walletMoveWalletEnded.
      ShieldFaultReason.walletUnavailable => l10n.walletShieldWalletEnded,
      // Out of disk (#373 follow-up): ask for space, not a bare retry that re-fails.
      ShieldFaultReason.storageFull => l10n.walletShieldStorageFullBody,
      ShieldFaultReason.couldNotPrepare => null,
      // INC-018 (b): the retryable class says what to do — "try again in a
      // moment" — never the title-only arm above (the Batch C arch pass).
      ShieldFaultReason.couldNotPrepareTransient =>
        l10n.walletShieldTransientBody,
    };
    return WalletSheetResult(
      icon: WalletGlyph.error,
      title: l10n.walletShieldFailedTitle,
      body: body,
      onClose: () => Navigator.of(context).maybePop(),
      closeLabel: l10n.walletShieldClose,
      // walletUnavailable is a defensive "go back" with nothing to retry; the
      // others can be re-proposed once sync advances / the wallet frees up.
      onRetry: reason == ShieldFaultReason.walletUnavailable
          ? null
          : () => ref.read(shieldControllerProvider.notifier).prepare(),
      retryLabel: l10n.walletShieldRetry,
    );
  }
}

/// The confirm phase: title + privacy-positive note + the exact gross/fee/net
/// rows + the "Shield now" / cancel actions.
class _Review extends StatelessWidget {
  const _Review({
    required this.proposal,
    required this.onConfirm,
    required this.onCancel,
  });

  final SendProposal proposal;
  final VoidCallback onConfirm;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        WalletSheetHeader(
          title: l10n.walletShieldSheetTitle,
          closeKey: const Key('shield-sheet-close'),
          onClose: onCancel,
        ),
        const SizedBox(height: 8),
        Text(
          l10n.walletShieldNote,
          style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
        ),
        const SizedBox(height: 20),
        LabeledZatRow(
          label: l10n.walletShieldAmountLabel,
          amountZat: proposal.totalZat,
        ),
        const SizedBox(height: 8),
        LabeledZatRow(
          label: l10n.walletShieldFeeLabel,
          amountZat: proposal.feeZat,
        ),
        const SizedBox(height: 8),
        LabeledZatRow(
          label: l10n.walletShieldNetLabel,
          amountZat: proposal.changeZat,
          emphasize: true,
        ),
        const SizedBox(height: 24),
        SizedBox(
          width: double.infinity,
          child: Semantics(
            container: true,
            button: true,
            label: l10n.walletShieldConfirmButton,
            // The `ExcludeSemantics` below deletes the button's node outright —
            // its tap action with it — so the action must live on THIS node,
            // bound to the same callback. Nothing else is inside the excluded
            // subtree, so the outer action is the whole remedy (probed, not
            // read off the framework docs; Relim `0c5ae1bd`, #661/#721).
            onTap: onConfirm,
            child: ExcludeSemantics(
              child: FilledButton.icon(
                onPressed: onConfirm,
                icon: const WalletIcon(WalletGlyph.shielded),
                label: Text(l10n.walletShieldConfirmButton),
              ),
            ),
          ),
        ),
        const SizedBox(height: 8),
        SizedBox(
          width: double.infinity,
          child: TextButton(
            onPressed: onCancel,
            child: Text(l10n.walletShieldClose),
          ),
        ),
      ],
    );
  }
}
