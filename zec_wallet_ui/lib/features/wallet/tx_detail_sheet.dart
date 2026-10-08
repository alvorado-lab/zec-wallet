import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../shared/address_text.dart';
import '../../shared/wallet_notice.dart';
import '../../shared/wallet_sheet.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'hide_balance.dart';
import 'sync_status_presentation.dart'
    show exactBlockCount, walletFullDateTimeFormat;
import 'wallet_activity_controller.dart';
import 'zat_format.dart';

/// Open the transaction-detail sheet for one history row (#320; maintainer: "I
/// can't tap on any activity to see the details that usually user needs").
/// Shows everything [TxSummary] carries — amount, status with a plain-language
/// explanation, fee, date, mined height, memo presence, and the full txid with
/// a copy affordance. Memo CONTENT and the recipient address are not on the
/// summary DTO (net-new FFI plumbing) — an explicit follow-up, not shown here.
/// Freely dismissible: purely informational, it owns no in-flight operation.
Future<void> showTxDetailSheet(BuildContext context, TxSummary tx) {
  // The shared sheet frame: grows with the explanation + large text scales,
  // freely dismissible, the desktop width cap.
  return showWalletSheet<void>(context, builder: (_) => TxDetailSheet(tx: tx));
}

/// Status word for a history row / the detail sheet — ONE mapping for both
/// surfaces (the list row imports this; they must never disagree).
String txStatusLabel(WalletLocalizations l10n, TxStatus s) => switch (s) {
  TxStatus_Queued() => l10n.walletActivityQueued,
  TxStatus_Pending() => l10n.walletActivityPending,
  TxStatus_Confirmed(:final depth) => l10n.walletActivityConfirmations(depth),
  TxStatus_Expired() => l10n.walletActivityExpired,
  TxStatus_Failed() => l10n.walletActivityFailed,
  // Forward-compat arm (a future Rust status the bridge maps to Unknown):
  // render as Pending rather than crash — honest "in progress". Matched
  // EXPLICITLY (no `_`) so a regen that adds a named arm fails the build
  // loud instead of silently reading as Pending.
  TxStatus_Unknown() => l10n.walletActivityPending,
};

/// The status word REFINED by the wallet's delivery reading for a transaction
/// it created (stage S8 `obligation`, row 10) — the list row and the detail
/// sheet both use this, so the two surfaces cannot disagree. An unmined
/// wallet-created transaction reads "Retrying" while the wallet still owes its
/// broadcast and "Saved" while it is kept without a retry promise; every other
/// reading (accepted — an endpoint has it, so Pending's "sent to the network"
/// is true — confirmed, the forward-compat unknown, or no reading at all)
/// falls through to [txStatusLabel]. Matched EXPLICITLY, no `_`, for the same
/// reason as [txStatusLabel]'s Unknown arm.
///
/// Unmined INCOMING money reads "Arriving", never "Pending": "Pending" is only
/// for the user's own sends in flight (the maintainer's ruling, S13 §2). The
/// forward-compat Unknown status reads Pending, so incoming Unknown is
/// Arriving by the same rule.
String txRowStatusLabel(WalletLocalizations l10n, TxSummary tx) =>
    switch (tx.delivery) {
      DeliveryState.retryPending => l10n.walletActivityRetrying,
      DeliveryState.persisted => l10n.walletActivitySaved,
      DeliveryState.accepted ||
      DeliveryState.confirmed ||
      DeliveryState.unknown ||
      null =>
        (tx.status is TxStatus_Pending || tx.status is TxStatus_Unknown) &&
                tx.netAmountZat > 0
            ? l10n.walletArrivingLabel
            : txStatusLabel(l10n, tx.status),
    };

/// Whether this status means the transaction DID NOT move funds (expired ⇒
/// cancelled unmined, funds returned to spendable — state.rs; failed ⇒
/// endpoint-rejected, never accepted). Drives the struck-through amount and
/// the "no funds left your wallet" reassurance (the maintainer's "what does
/// expired mean" ask).
///
/// WATCH (UX review): core's chain view currently yields only
/// Pending/Confirmed/Expired — the Failed arm is dormant. If Failed is ever
/// wired to an AMBIGUOUS submit outcome (e.g. a timeout where the tx may have
/// landed), the funds-kept claim would be false for it — revisit this
/// predicate before wiring that.
///
/// NEVER when the wallet holds a delivery reading for it (S7 C1): an expired
/// attempt the wallet will send again by itself reads `Expired` +
/// [DeliveryState.retryPending] until the replacement goes out, and striking
/// it through with "no funds left your wallet" is what tempts a user to pay a
/// second time by hand.
bool txIsCancelled(TxSummary tx) =>
    tx.delivery == null &&
    (tx.status is TxStatus_Expired || tx.status is TxStatus_Failed);

/// Plain-language explanation of the status for the detail sheet.
String txStatusExplanation(WalletLocalizations l10n, TxStatus s) => switch (s) {
  TxStatus_Queued() => l10n.walletTxExplainQueued,
  TxStatus_Pending() => l10n.walletTxExplainPending,
  TxStatus_Confirmed() => l10n.walletTxExplainConfirmed,
  TxStatus_Expired() => l10n.walletTxExplainExpired,
  TxStatus_Failed() => l10n.walletTxExplainFailed,
  // The unknown LABEL reads as Pending (see txStatusLabel), but the
  // explanatory sentence must not AFFIRM a broadcast the binding can't
  // verify — a neutral, honest unknown instead.
  TxStatus_Unknown() => l10n.walletTxExplainUnknown,
};

/// The explanation REFINED by the delivery reading, the sibling of
/// [txRowStatusLabel]: the Pending sentence affirms "sent to the Zcash
/// network", which is FALSE for a transaction the wallet is still retrying
/// (no endpoint has taken it yet) and for one it is holding — those two get
/// their own honest sentences; everything else falls through. An EXPIRED
/// attempt the wallet will send again (S7 C1) says so, and says not to.
String txRowStatusExplanation(WalletLocalizations l10n, TxSummary tx) =>
    switch (tx.delivery) {
      DeliveryState.retryPending =>
        tx.status is TxStatus_Expired
            ? l10n.walletTxExplainRetryingExpired
            : l10n.walletTxExplainRetrying,
      DeliveryState.persisted => l10n.walletTxExplainSaved,
      DeliveryState.accepted ||
      DeliveryState.confirmed ||
      DeliveryState.unknown ||
      null => txStatusExplanation(l10n, tx.status),
    };

/// The sheet body (public for widget tests — pump it directly in a
/// `ProviderScope`, the shield-sheet idiom).
class TxDetailSheet extends ConsumerWidget {
  const TxDetailSheet({required this.tx, super.key});

  /// The row as tapped — the fallback when the live list no longer carries it
  /// (cold reload, paged out). The txid is the row's identity.
  final TxSummary tx;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // LIVE: re-resolve this row by txid from the activity provider, which the
    // wallet screen already refreshes on the balance-changing sync edges — so
    // a Pending→Confirmed/Expired transition updates the OPEN sheet in place
    // (UX review: a user opens this precisely to check status; a stale
    // "waiting to be confirmed" on an already-expired tx is the worst case).
    // The tapped summary is the fallback; rows carry through every phase, so
    // a transient refetch fault can blank nothing here.
    var tx = this.tx;
    for (final row in ref.watch(walletActivityProvider).rows) {
      if (row.txidHex == tx.txidHex) {
        tx = row;
        break;
      }
    }

    // Same direction/amount language as the list row (zero-net = neutral
    // outgoing, never "Received").
    final incoming = tx.netAmountZat > 0;
    final title = incoming
        ? l10n.walletActivityReceived
        : l10n.walletActivitySent;
    // Hide balance (FR-49 W-7): the amount and the fee mask while hidden —
    // on screen and to a screen reader (the sheet is information; nothing
    // signs from it).
    final hidden = ref.watch(walletBalanceHiddenProvider);
    // The same signed form as the list row (FR-49 S12, C5): `+` for incoming,
    // the true minus sign U+2212 for outgoing.
    final figure = formatZec(tx.netAmountZat);
    final signed = incoming
        ? '+$figure'
        : figure.startsWith('-')
        ? '−${figure.substring(1)}'
        : figure;
    final amountText = displayedAmount(
      l10n.walletAmount(signed),
      hidden: hidden,
    );
    final cancelled = txIsCancelled(tx);
    // A cancelled (expired/failed) amount reads as "didn't happen": struck
    // through and muted, matching the list row.
    final amountStyle = textTheme.headlineSmall?.copyWith(
      color: cancelled
          ? colors.textMuted
          : (incoming ? colors.green : colors.text),
      fontWeight: FontWeight.w700,
      decoration: cancelled ? TextDecoration.lineThrough : null,
    );

    final rows = <(String, String)>[
      (l10n.walletTxDetailStatus, txRowStatusLabel(l10n, tx)),
      if (tx.feeZat != null)
        (
          l10n.walletTxDetailFee,
          displayedAmount(
            l10n.walletAmount(formatZec(tx.feeZat!)),
            hidden: hidden,
          ),
        ),
      if (tx.timestamp != null)
        (l10n.walletTxDetailDate, _formatDate(tx.timestamp!, l10n.localeName)),
      if (tx.minedHeight != null)
        (
          l10n.walletTxDetailHeight,
          exactBlockCount(tx.minedHeight!, l10n.localeName),
        ),
      if (tx.hasMemo)
        (l10n.walletTxDetailMemo, l10n.walletTxDetailMemoAttached),
      // §3.2i-3 (c): the transparency fact must SURVIVE the tap from the
      // badged history row (UX review M3 — the detail sheet is where a
      // user checks facts). Rendered ONLY when flagged: the flag's absence
      // means "no publicly-visible output leg", which is NOT a positive
      // "fully private" claim (e.g. a shield tx spends transparent INPUTS) —
      // so the sheet asserts the public fact and stays silent otherwise.
      if (tx.hasTransparentOutput)
        (l10n.walletTxDetailVisibility, l10n.walletActivityPublicBadge),
    ];

    return SafeArea(
      child: SingleChildScrollView(
        // Bounded under a large text scale (the sheet stacks ~10 lines).
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 16, 20, 24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              WalletSheetHeader(
                title: title,
                closeKey: const Key('tx-detail-close'),
                leading: CircleAvatar(
                  radius: 18,
                  backgroundColor: incoming
                      ? colors.green.withValues(alpha: 0.15)
                      : colors.textMuted.withValues(alpha: 0.15),
                  child: WalletIcon(
                    incoming ? WalletGlyph.incoming : WalletGlyph.outgoing,
                    size: 18,
                    color: incoming ? colors.green : colors.textMuted,
                  ),
                ),
              ),
              const SizedBox(height: 12),
              Text(
                amountText,
                style: amountStyle,
                semanticsLabel: hidden ? l10n.walletBalanceHiddenAmount : null,
              ),
              if (cancelled) ...[
                const SizedBox(height: 12),
                // The headline reassurance, PROMINENT (maintainer: "does expired
                // mean funds are not withdrawn — it should be visually clear").
                WalletNotice(
                  tone: WalletNoticeTone.warning,
                  glyph: WalletGlyph.rebroadcast,
                  message: l10n.walletTxFundsKept,
                  messageInTone: true,
                ),
              ],
              const SizedBox(height: 12),
              Text(
                txRowStatusExplanation(l10n, tx),
                style: textTheme.bodyMedium?.copyWith(color: colors.text),
              ),
              const SizedBox(height: 16),
              for (final (label, value) in rows)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: MergeSemantics(
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(
                          child: Text(
                            label,
                            style: textTheme.bodyMedium?.copyWith(
                              color: colors.textMuted,
                            ),
                          ),
                        ),
                        Flexible(
                          child: Text(
                            value,
                            // A masked value (the fee, while hidden) is
                            // announced in words, never as its dots (R5).
                            semanticsLabel: value == maskedAmountText
                                ? l10n.walletBalanceHiddenAmount
                                : null,
                            textAlign: TextAlign.end,
                            style: textTheme.bodyMedium?.copyWith(
                              color: colors.text,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              const SizedBox(height: 12),
              Text(
                l10n.walletTxDetailTxid,
                style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              ),
              const SizedBox(height: 4),
              // Full txid, mono, selectable, ends emphasized (§5.4: rendered/
              // copied, never logged) — the shared read-only address rendering.
              AddressText(tx.txidHex, style: textTheme.bodySmall),
              const SizedBox(height: 16),
              Row(
                children: [
                  Expanded(
                    child: Semantics(
                      container: true,
                      button: true,
                      excludeSemantics: true,
                      label: l10n.walletTxDetailCopyTxid,
                      // excludeSemantics strips the button's own tap action —
                      // the a11y action must live on THIS node (the same rule
                      // as the badge/row; security review finding).
                      onTap: () => _copyTxid(context, l10n),
                      child: OutlinedButton.icon(
                        icon: const WalletIcon(WalletGlyph.copy, size: 18),
                        label: Text(
                          l10n.walletTxDetailCopyTxid,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        onPressed: () => _copyTxid(context, l10n),
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  TextButton(
                    onPressed: () => Navigator.of(context).maybePop(),
                    child: Text(l10n.walletTxDetailClose),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// The ONE copy path (pointer press and a11y tap action share it): the FULL
  /// txid to the clipboard, then the snackbar confirmation.
  Future<void> _copyTxid(BuildContext context, WalletLocalizations l10n) async {
    await Clipboard.setData(ClipboardData(text: tx.txidHex));
    unawaited(HapticFeedback.lightImpact());
    if (!context.mounted) return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(l10n.walletTxDetailCopied)));
  }

  // Full date+time — the sheet is where the exact moment belongs (the row
  // keeps the compact form). Locale-aware via the shared per-locale-memoized
  // factory (#317 — the old static froze to `Intl.defaultLocale`); the mined
  // height rides the shared `exactBlockCount` (one grouping idiom with the
  // sync sheet's figure rows).
  static String _formatDate(int unixSecs, String locale) =>
      walletFullDateTimeFormat(
        locale,
      ).format(DateTime.fromMillisecondsSinceEpoch(unixSecs * 1000).toLocal());
}
