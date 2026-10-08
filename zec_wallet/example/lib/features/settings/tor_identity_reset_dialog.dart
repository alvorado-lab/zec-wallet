import 'package:flutter/material.dart';

import '../../core/router/router.dart';
import '../../l10n/app_localizations.dart';

/// The offer after a wallet delete while the Tor plugin is on: also reset the
/// Tor identity (`ZecWalletTor.clearState`)? `true` only when the user taps
/// Reset; a dismissal, Keep, or no screen to show it on is `false`: the
/// plugin's state is never deleted unasked.
///
/// Shown over the root navigator because the provisioner, which runs the
/// delete, has no widget of its own.
Future<bool> offerTorIdentityReset() async {
  final context = rootNavigatorKey.currentContext;
  if (context == null || !context.mounted) return false;
  final l10n = AppLocalizations.of(context);
  final result = await showAdaptiveDialog<bool>(
    context: context,
    builder: (dialogContext) => AlertDialog.adaptive(
      title: Text(l10n.torIdentityResetTitle),
      content: Text(l10n.torIdentityResetBody),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(false),
          child: Text(l10n.torIdentityResetCancel),
        ),
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(true),
          child: Text(l10n.torIdentityResetConfirm),
        ),
      ],
    ),
  );
  return result ?? false;
}

/// A reset the user asked for failed: say so, and offer to try again. `true`
/// only when the user taps Try again; no screen to show it on is `false`.
Future<bool> reportTorIdentityResetFailed() async {
  final context = rootNavigatorKey.currentContext;
  if (context == null || !context.mounted) return false;
  final l10n = AppLocalizations.of(context);
  final result = await showAdaptiveDialog<bool>(
    context: context,
    builder: (dialogContext) => AlertDialog.adaptive(
      title: Text(l10n.torIdentityResetFailedTitle),
      content: Text(l10n.torIdentityResetFailedBody),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(false),
          child: Text(l10n.torIdentityResetFailedDismiss),
        ),
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(true),
          child: Text(l10n.torIdentityResetFailedRetry),
        ),
      ],
    ),
  );
  return result ?? false;
}
