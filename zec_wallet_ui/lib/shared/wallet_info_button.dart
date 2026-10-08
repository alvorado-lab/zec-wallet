import 'package:flutter/material.dart';

import '../core/theme/colors.dart';
import '../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'wallet_sheet.dart';

/// The (i) after a label (S13 §1.1; DESIGN §6.20): an explanation that used to
/// sit on screen as a paragraph moves behind it WHOLE, under its existing
/// string. A STATE or a COST never moves here — those stay on screen.
///
/// A 44 target whose accessible name is "More about <label>"; a tap opens
/// [body] in a wallet sheet with a close action.
class WalletInfoButton extends StatelessWidget {
  const WalletInfoButton({
    super.key,
    required this.label,
    required this.body,
    this.title,
  });

  /// What the explanation is about — the label the button follows. Read
  /// aloud as "More about <label>".
  final String label;

  /// The sheet's heading; [label] when null.
  final String? title;

  /// The explanation itself, shown whole.
  final String body;

  /// The target's side, pinned by the S13 row.
  static const double targetSize = 44;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final name = l10n.walletInfoButtonLabel(label);
    return Semantics(
      button: true,
      label: name,
      excludeSemantics: true,
      child: SizedBox.square(
        dimension: targetSize,
        child: IconButton(
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints.tightFor(
            width: targetSize,
            height: targetSize,
          ),
          icon: WalletIcon(WalletGlyph.info, size: 20, color: colors.textMuted),
          onPressed: () => showWalletSheet<void>(
            context,
            builder: (sheetContext) =>
                _InfoSheet(title: title ?? label, body: body),
          ),
        ),
      ),
    );
  }
}

class _InfoSheet extends StatelessWidget {
  const _InfoSheet({required this.title, required this.body});

  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.min,
          children: [
            // The sheets' one title row and close circle (S13 §1.7).
            WalletSheetHeader(
              title: title,
              closeKey: const ValueKey('wallet-info-close'),
            ),
            const SizedBox(height: 12),
            Text(body, style: textTheme.bodyMedium),
          ],
        ),
      ),
    );
  }
}
