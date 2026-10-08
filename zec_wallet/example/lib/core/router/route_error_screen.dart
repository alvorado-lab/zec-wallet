import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../../l10n/app_localizations.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';

/// Unknown-route fallback (app-frame spec §6): plain language, a working
/// next step, no error codes.
class RouteErrorScreen extends StatelessWidget {
  const RouteErrorScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(),
      body: Center(
        // Scrolls when it can't fit (the 2.0× text-scale ceiling on a
        // small phone — the fixed column would otherwise overflow).
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(l10n.routeErrorTitle, style: textTheme.headlineMedium),
              const SizedBox(height: 12),
              Text(
                l10n.routeErrorBody,
                style: textTheme.bodyLarge,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              FilledButton(
                // ≥44×44 touch target (flutter-patterns § Widget
                // Conventions) — Material's 40-high visual would pass via
                // tap-target padding, but the explicit minimum keeps the
                // named test's semantics-bounds assertion honest.
                style: FilledButton.styleFrom(minimumSize: const Size(64, 48)),
                onPressed: () => context.go(WalletRoutes.wallet),
                child: Text(l10n.routeErrorGoHome),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
