import 'package:flutter/material.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// The honest managed-by-host state, shared by the backup and the viewing-key
/// screens (stage S11 C7): this wallet's keys are owned by the app that set
/// it up (a host-supplied or raw seed, or a session-only mount), so there is
/// no wallet-local phrase or key to show. Never invents what does not exist
/// (operating principle 6).
class WalletManagedByHost extends StatelessWidget {
  const WalletManagedByHost({super.key});

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            WalletIcon(WalletGlyph.shielded, color: colors.textMuted),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                l10n.walletBackupManagedTitle,
                style: textTheme.titleMedium,
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        Text(
          l10n.walletBackupManagedBody,
          style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
        ),
      ],
    );
  }
}
