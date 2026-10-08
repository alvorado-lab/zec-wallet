/// The two phases every money sheet shares (stage S11 C7): the spinner while
/// it proposes or submits, and the outcome it ends on. The shield and
/// move-to-transparent sheets each carried an identical private copy.
///
/// NOT exported: the labels are the calling sheet's own l10n keys, passed in.
library;

import 'package:flutter/material.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

import '../core/theme/colors.dart';
import '../core/theme/icons.dart';
import 'wallet_dialog.dart';

/// Back on a money sheet while it submits (S13 §1.3, the diff review's
/// MEDIUM): the sheet holds Back, and this asks instead — the submit is
/// unbounded, so the hold must never strand the user. Leaving does not cancel
/// anything (the transaction outlives the sheet); Leave pops the sheet
/// directly, past its own hold. The maintainer's words.
Future<void> confirmLeaveSheetWhileSending(BuildContext context) async {
  final l10n = WalletLocalizations.of(context);
  final leave = await showWalletConfirm(
    context,
    title: l10n.walletSendLeaveTitle,
    body: l10n.walletSheetLeaveBody,
    confirmLabel: l10n.walletSendLeaveConfirm,
    cancelLabel: l10n.walletSendLeaveStay,
    kind: WalletConfirmKind.neutral,
    confirmKey: const ValueKey('sheet-leave-confirm'),
    cancelKey: const ValueKey('sheet-leave-stay'),
  );
  if (!leave || !context.mounted) return;
  // Pop only the SHEET: if something was pushed above it while the dialog
  // was up (a late host prompt), popping would dismiss that instead (the
  // fold review's LOW). The user deals with what is on top first.
  final route = ModalRoute.of(context);
  if (route != null && !route.isCurrent) return;
  Navigator.of(context).pop();
}

/// A transient spinner phase (loading, proposing, submitting).
class WalletSheetBusy extends StatelessWidget {
  const WalletSheetBusy({super.key, required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 24),
      child: Row(
        children: [
          const SizedBox(
            width: 20,
            height: 20,
            child: CircularProgressIndicator.adaptive(strokeWidth: 2),
          ),
          const SizedBox(width: 16),
          // FLEXIBLE (#403 R8c, UX review m10): a bare `Text` in a `Row`
          // has unbounded width, so a long localized label at a large text
          // scale overflowed (measured 41px at ru/2.0×/320dp) instead of
          // wrapping.
          Flexible(
            child: Text(
              label,
              style: textTheme.bodyLarge?.copyWith(color: colors.text),
            ),
          ),
        ],
      ),
    );
  }
}

/// A terminal phase: an icon, a title, an optional body, a Close, and an
/// optional "Try again". A live region, so a screen reader announces the
/// outcome.
class WalletSheetResult extends StatelessWidget {
  const WalletSheetResult({
    super.key,
    required this.icon,
    required this.title,
    required this.body,
    required this.onClose,
    required this.closeLabel,
    this.onRetry,
    this.retryLabel,
  }) : assert(onRetry == null || retryLabel != null, 'a retry needs its label');

  final WalletGlyph icon;
  final String title;
  final String? body;
  final VoidCallback onClose;
  final String closeLabel;
  final VoidCallback? onRetry;
  final String? retryLabel;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final onRetry = this.onRetry;
    return Semantics(
      liveRegion: true,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              WalletIcon(icon, color: colors.textMuted),
              const SizedBox(width: 12),
              Expanded(child: Text(title, style: textTheme.titleMedium)),
            ],
          ),
          if (body != null) ...[
            const SizedBox(height: 8),
            Text(
              body!,
              style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
            ),
          ],
          const SizedBox(height: 20),
          if (onRetry != null) ...[
            SizedBox(
              width: double.infinity,
              child: FilledButton(onPressed: onRetry, child: Text(retryLabel!)),
            ),
            const SizedBox(height: 8),
          ],
          // With a retry, Close steps down to a text button so "Try again" is
          // the one filled action.
          SizedBox(
            width: double.infinity,
            child: onRetry != null
                ? TextButton(onPressed: onClose, child: Text(closeLabel))
                : FilledButton(onPressed: onClose, child: Text(closeLabel)),
          ),
        ],
      ),
    );
  }
}
