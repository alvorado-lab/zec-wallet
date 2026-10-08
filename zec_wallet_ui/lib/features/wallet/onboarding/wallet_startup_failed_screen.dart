import 'package:flutter/material.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import '../../../shared/wallet_cta.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// The honest BOOT-WIRING failure surface (#356-F1): what a host renders when
/// it MEANT to wire the wallet but the startup work failed — the FFI library
/// didn't load, the data directory didn't resolve, or either timed out.
///
/// WHY a dedicated screen: without it, a failed wiring degrades to the
/// unwired seams and [OnboardingUnavailable] renders the "wallet setup
/// arrives in a later build" copy — FALSE on a build that ships the wallet
/// (the device finding: an FFI boot fault told the user the feature
/// doesn't exist, with no way to retry). This screen tells the truth (the
/// wallet couldn't START) and offers the retry the failure deserves; the
/// not-set-up copy stays for hosts that genuinely ship without the wallet.
///
/// MONEY-HONESTY: the body restates that funds are not affected — a wallet's
/// funds live on-chain, recoverable from the recovery phrase, so a boot
/// failure (however scary) never means loss.
///
/// Host contract: render this INSTEAD of the wallet shell when boot wiring
/// fails (the reference consumer's boot gate in `example/lib/main.dart` is
/// the canonical wiring — bounded awaits around the startup work, this screen
/// with a retry that re-runs them on failure). [retrying] disables the button
/// and spins while a retry attempt is in flight.
class WalletStartupFailedScreen extends StatelessWidget {
  const WalletStartupFailedScreen({
    required this.onRetry,
    this.retrying = false,
    super.key,
  });

  /// Re-runs the host's startup work (idempotently — see the reference
  /// consumer's memoized FFI init). Called only from the enabled button.
  final VoidCallback onRetry;

  /// True while a retry attempt is in flight — disables the button and shows
  /// the in-progress spinner (the standard double-tap affordance).
  final bool retrying;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            // The 16 side gutter (S13 Build B).
            padding: const EdgeInsets.fromLTRB(16, 24, 16, 24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                WalletIcon(WalletGlyph.error, size: 48, color: colors.orange),
                const SizedBox(height: 16),
                Text(
                  l10n.walletStartupFailedTitle,
                  style: textTheme.headlineSmall,
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 12),
                Text(
                  l10n.walletStartupFailedBody,
                  style: textTheme.bodyMedium?.copyWith(
                    color: colors.textMuted,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 24),
                WalletCta(
                  child: FilledButton.icon(
                    onPressed: retrying ? null : onRetry,
                    icon: retrying
                        ? const SizedBox(
                            height: 20,
                            width: 20,
                            child: CircularProgressIndicator.adaptive(
                              strokeWidth: 2,
                            ),
                          )
                        : const WalletIcon(WalletGlyph.retry),
                    label: Text(l10n.walletOnboardingRetry),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
