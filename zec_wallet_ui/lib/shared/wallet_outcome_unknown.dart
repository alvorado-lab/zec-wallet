/// The honest "we lost the answer" terminal every money flow shares (S7 U1,
/// R13 §4.3): the spend ran and something after it threw or declined, so the
/// flow can say neither "done" nor "nothing happened". It points at where the
/// truth is and offers Close only — a Try again here is how one spend gets
/// made twice.
///
/// NOT exported: the title, body and close label are the calling flow's own
/// l10n keys, passed in.
library;

import 'package:flutter/material.dart';

import '../core/theme/colors.dart';
import '../core/theme/icons.dart';

/// An orange info icon, the title, the body and ONE action that closes. Never
/// renders an error's text, and takes no retry by construction. A live
/// region, like the result views, so a screen reader announces it.
class WalletOutcomeUnknownView extends StatelessWidget {
  const WalletOutcomeUnknownView({
    super.key,
    required this.title,
    required this.body,
    required this.closeLabel,
    required this.onClose,
  });

  final String title;
  final String body;
  final String closeLabel;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Semantics(
      container: true,
      liveRegion: true,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          WalletIcon(WalletGlyph.info, size: 48, color: colors.orange),
          const SizedBox(height: 16),
          Text(
            title,
            style: textTheme.headlineSmall,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          Text(
            body,
            style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          FilledButton(onPressed: onClose, child: Text(closeLabel)),
        ],
      ),
    );
  }
}
