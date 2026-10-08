import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/wallet_notice.dart';
import '../sync_status_presentation.dart' show graceEndedText;
import '../wallet_sync_controller.dart' show walletSyncPassesRunProvider;
import '../wallet_rescan_controller.dart'
    show WalletCatchUpNone, walletCatchUpCueProvider;
import '../zat_format.dart';
import 'send_state.dart';
import 'zec_amount.dart';

/// Render an inline [SendFormFault] as an honest, fixable message (a notice
/// line in the warning tone — orange, never red; red stays reserved for a
/// money failure). Reads only the typed fault (never an error payload — §5.4), so
/// nothing sensitive (address/amount) leaks.
///
/// SHARED between the Send form and the Move-to-transparent expert action
/// (§3.2i-1): both surface the SAME propose-time faults (insufficient funds,
/// not-synced, amounts-expired, wallet-busy, an amount parse fault), so the copy
/// must never drift between them. A [ConsumerWidget] (not Stateless) since
/// #381 (a): the insufficient-funds arm reads the GLOBAL catch-up cue itself —
/// unlike [queueOffered] this is not surface knowledge, and reading it here
/// keeps the two hosting forms' catch-up honesty from ever drifting apart.
class SendFormFaultView extends ConsumerWidget {
  const SendFormFaultView({
    super.key,
    required this.fault,
    this.queueOffered = false,
  });

  final SendFormFault fault;

  /// Whether THIS surface actually offers the offline-queue affordance —
  /// drives the not-synced copy's "or queue this to send later" clause. The
  /// SURFACE knows, not this view: the send form passes its
  /// `walletOfflineQueueSupportedProvider` read (review H1 — with the
  /// affordance hidden, copy inviting it is a lie), and the
  /// move-to-transparent sheet never offers a queue at all, so the safe
  /// default is FALSE (never invite an absent affordance).
  final bool queueOffered;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    final String message;
    final details = <String>[];
    final f = fault;
    if (f is SendAmountFault) {
      message = amountFaultMessage(l10n, f.fault);
    } else if (f is SendOverCeiling) {
      // Host policy (walletSendCeilingZatProvider) — the copy states the limit
      // honestly as an app restriction, never as an invalid amount.
      message = l10n.walletSendFaultOverCeiling(formatZec(f.ceilingZat));
    } else if (f is SendInsufficientFunds) {
      message = l10n.walletSendFaultInsufficient(
        formatZec(f.availableZat),
        formatZec(f.requiredZat),
      );
      if (f.pendingIncomingZat > 0) {
        details.add(
          l10n.walletSendFaultInsufficientPending(
            formatZec(f.pendingIncomingZat),
          ),
        );
      }
      // Mid-catch-up the "you have X" figure is the PARTIAL repopulating
      // balance, not a verdict (#381 (a)): pendingIncomingZat structurally
      // cannot carry UNSCANNED funds, so without this line the fault reads
      // as final exactly when it is least so. Independent of the pending
      // line — both can be true, so they stack. NOT when no sync pass will
      // run (#405 → the SSOT): "as the wallet syncs" would claim the
      // active progress the notSyncedYet arm below just stopped claiming —
      // the same card must not carry both. A FAILED start is as unable to
      // deliver that progress as a host-off policy.
      if (ref.watch(walletCatchUpCueProvider) is! WalletCatchUpNone &&
          ref.watch(walletSyncPassesRunProvider)) {
        details.add(l10n.walletSendFaultInsufficientCatchingUp);
      }
    } else if (f is SendServerSilentFault) {
      // GRACE-1 (§4p G-6): the same sentence the sync surface shows for an
      // ended grace — one source for the reason and the next step ("switch
      // servers", plus the device clock for the clock rule); never the
      // update-the-app copy.
      message = graceEndedText(l10n, f.by, f.blocksSinceLastCurrent);
    } else if (f is SendCategoricalFault) {
      // The not-synced copy must not say "wait for sync to catch up" while
      // no sync pass will run (nothing is catching up). Read HERE,
      // like the catch-up cue above, so both hosting forms (Send and
      // Move-to-transparent) tell the same honest story with no drift.
      //
      // #405: the SSOT, not the policy. Under a FAILED start this fell
      // through to the plain arm and told the user to "wait for sync to catch
      // up" — the wait-forever the whole qualifier family exists to prevent,
      // on the one screen with no ambient badge to contradict it.
      message = _categoricalFaultMessage(
        l10n,
        f.reason,
        queueOffered,
        syncNotRunning: !ref.watch(walletSyncPassesRunProvider),
      );
    } else {
      message = l10n.walletSendFaultCouldNotPrepare;
    }

    // A11y: announce the fault the moment it appears. It renders at the foot of
    // a scrollable form and, at large text scale, below the fold under a
    // disabled Review — a screen-reader user gets no visual "why", so mark it a
    // live region (the icon is decorative; the words carry the meaning). The
    // sighted counterpart is the send form's auto-scroll-to-fault (#329-2).
    return WalletNotice(
      tone: WalletNoticeTone.warning,
      glyph: WalletGlyph.error,
      message: message,
      liveRegion: true,
      child: details.isEmpty
          ? null
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                for (final (i, detail) in details.indexed) ...[
                  if (i > 0) const SizedBox(height: 2),
                  Text(
                    detail,
                    style: textTheme.bodySmall?.copyWith(
                      color: colors.textMuted,
                    ),
                  ),
                ],
              ],
            ),
    );
  }

  /// The amount-parse copy — also the receive screen's Request amount field
  /// (the parser is shared, so is its wording).
  static String amountFaultMessage(WalletLocalizations l10n, ZecAmountFault f) {
    return switch (f) {
      ZecAmountFault.empty => l10n.walletSendFaultAmountEmpty,
      ZecAmountFault.notANumber => l10n.walletSendFaultAmountNotANumber,
      ZecAmountFault.tooManyDecimals => l10n.walletSendFaultAmountDecimals,
      ZecAmountFault.notPositive => l10n.walletSendFaultAmountNotPositive,
      ZecAmountFault.outOfRange => l10n.walletSendFaultAmountOutOfRange,
    };
  }

  static String _categoricalFaultMessage(
    WalletLocalizations l10n,
    SendFaultReason reason,
    bool queueOffered, {
    bool syncNotRunning = false,
  }) {
    return switch (reason) {
      SendFaultReason.addressInvalid => l10n.walletSendFaultAddressInvalid,
      SendFaultReason.memoToTransparent =>
        l10n.walletSendFaultMemoToTransparent,
      SendFaultReason.memoTooLong => l10n.walletSendFaultMemoTooLong,
      SendFaultReason.memoNotSendable => l10n.walletSendFaultMemoNotSendable,
      SendFaultReason.memoConflict => l10n.walletSendFaultMemoConflict,
      SendFaultReason.amountOutOfRange => l10n.walletSendFaultAmountOutOfRange,
      SendFaultReason.networkMismatch => l10n.walletSendFaultNetworkMismatch,
      SendFaultReason.uriInvalid => l10n.walletSendFaultUriInvalid,
      // "or queue this to send later" ONLY where the queue button actually
      // renders beside this fault — otherwise the honest wait-for-sync line;
      // and when no sync pass will run, the arm that doesn't promise a
      // catch-up that isn't running (#405).
      SendFaultReason.notSyncedYet =>
        syncNotRunning
            ? l10n.walletSendFaultNotSyncedSyncNotRunning
            : (queueOffered
                  ? l10n.walletSendFaultNotSynced
                  : l10n.walletSendFaultNotSyncedNoQueue),
      // Ironwood/NU6.3: one line, no variants. Unlike notSyncedYet there is
      // nothing to condition on — no sync will fix it, and the queue is
      // deliberately not offered beside it (send_screen), because "queue to
      // send later" beside a wait that only an APP UPDATE ends reads as a
      // promise the app cannot keep.
      SendFaultReason.networkUpgradeUnsupported =>
        l10n.walletSendFaultNetworkUpgrade,
      SendFaultReason.amountsExpired => l10n.walletSendFaultAmountsExpired,
      SendFaultReason.queueFull => l10n.walletSendFaultQueueFull,
      SendFaultReason.walletBusy => l10n.walletSendFaultWalletBusy,
      SendFaultReason.storageFull => l10n.walletSendFaultStorageFull,
      SendFaultReason.oneTimeAddressLimit =>
        l10n.walletSendFaultOneTimeAddressLimit,
      SendFaultReason.couldNotPrepare => l10n.walletSendFaultCouldNotPrepare,
      // INC-018 (b): the retryable class gets the "try again in a moment" copy;
      // "check the details" would blame input that is correct.
      SendFaultReason.couldNotPrepareTransient =>
        l10n.walletSendFaultCouldNotPrepareTransient,
      SendFaultReason.walletUnavailable => l10n.walletSendUnavailable,
      SendFaultReason.watchOnly => l10n.walletSendFaultWatchOnly,
    };
  }
}
