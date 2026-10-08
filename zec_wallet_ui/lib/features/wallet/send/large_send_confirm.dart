import 'package:flutter/material.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/wallet_dialog.dart';
import '../zat_format.dart';

/// Show the ONE deliberate large-amount confirmation (inc-2d money-safety).
/// Returns `true` iff the user confirms; `false` on cancel, a barrier tap or a
/// back pop (→ stay on the review screen, no money moves) — never `null`
/// ([showWalletConfirm]'s contract). The exact amount rides the irreversible
/// action button so it is unmistakable at the moment of confirming; the body
/// is keyed off [reason]. A modal barrier, so the underlying Confirm button
/// can't be double-fired while it's open.
///
/// A `forward` confirm (stage S11 C3): a filled button on Android, iOS's
/// default (bold) action on a Cupertino alert.
///
/// SHARED by the Send review and the Move-to-transparent expert action
/// (§3.2i-1): both fire it off the SDK's `proposal.largeSend`, so the rare,
/// money-critical "are you sure" earns the SAME unmistakable copy everywhere.
Future<bool> showLargeSendConfirm(
  BuildContext context, {
  required int totalZat,
  required LargeSendReason reason,
}) {
  final l10n = WalletLocalizations.of(context);
  return showWalletConfirm(
    context,
    title: l10n.walletSendLargeConfirmTitle,
    body: _largeSendBody(l10n, reason),
    cancelLabel: l10n.walletSendLargeConfirmCancel,
    confirmLabel: l10n.walletSendLargeConfirmAction(
      l10n.walletAmount(formatZec(totalZat)),
    ),
    kind: WalletConfirmKind.forward,
  );
}

/// Body copy for the large-send dialog, keyed off the SDK's [LargeSendReason].
/// The forward-compat `unknown` arm AND any future variant fall through the
/// default to the STRONGEST copy — failing safe toward more caution, never
/// crashing (the FFI enum's documented "keep a default arm" contract).
String _largeSendBody(WalletLocalizations l10n, LargeSendReason reason) {
  return switch (reason) {
    LargeSendReason.nearTotalBalance => l10n.walletSendLargeConfirmNearTotal,
    LargeSendReason.overAbsoluteThreshold =>
      l10n.walletSendLargeConfirmOverThreshold,
    // `both`, `unknown`, and any future variant → the strongest "large +
    // near-total" copy.
    _ => l10n.walletSendLargeConfirmBoth,
  };
}
