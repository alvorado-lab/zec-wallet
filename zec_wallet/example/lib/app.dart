import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations_fallback.dart';

import 'core/router/router.dart';
import 'core/theme/appearance_prefs.dart';
import 'core/theme/example_type.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'core/theme/theme_mode_provider.dart';
import 'l10n/app_localizations.dart';

/// Root widget — rendering layer ONLY (design invariant 1: Rust owns all
/// state). The frame holds no application data; everything here is theme,
/// navigation, and l10n plumbing (app-frame spec §3).
class WalletExampleApp extends ConsumerWidget {
  const WalletExampleApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Watched in build() — never inside the `builder` closure
    // (flutter-patterns § Gotchas: ref.watch in build only).
    final userTextScale = ref.watch(textScaleProvider);

    return MaterialApp.router(
      onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
      debugShowCheckedModeBanner: false,
      theme: exampleLightTheme,
      // AMOLED true-black variant when the user opts in (dark-only).
      darkTheme: ref.watch(amoledProvider)
          ? exampleAmoledDarkTheme
          : exampleDarkTheme,
      // Seeded flash-free from SharedPreferences in main() via
      // initialThemeModeProvider; default ThemeMode.system.
      themeMode: ref.watch(themeModeProvider),
      themeAnimationDuration: themeAnimationDuration,
      // Exactly what a host app does: its own generated localizations class
      // plus the wallet package's FALLBACK delegate, side by side (distinct
      // types — no collision). The fallback wrapper — not the raw generated
      // delegate — is the contract: a host locale the wallet ARBs don't cover
      // reads English instead of crashing the first wallet frame (B2).
      localizationsDelegates: const [
        ...AppLocalizations.localizationsDelegates,
        walletLocalizationsFallbackDelegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) {
        final mq = MediaQuery.of(context);
        // Effective text scale = OS Dynamic Type → iOS HIG floor → user
        // multiplier → absolute safety clamp. The composition lives in
        // effectiveTextScale() (single source of truth, named-tested).
        //
        // `textScaler.scale(1.0)` is a lossless read of the OS factor:
        // Flutter maps Dynamic Type steps to linear factors, so
        // `TextScaler.linear(factor)` round-trips it exactly.
        final factor = effectiveTextScale(
          osFactor: mq.textScaler.scale(1.0),
          userScale: userTextScale,
        );
        return MediaQuery(
          data: mq.copyWith(textScaler: TextScaler.linear(factor)),
          // Status-bar + nav-bar icon brightness matches the active theme.
          // Applied globally so AppBar-less screens are covered too.
          child: AnnotatedRegion<SystemUiOverlayStyle>(
            value: overlayStyleFor(WalletColors.of(context)),
            child: child!,
          ),
        );
      },
      routerConfig: ref.watch(routerProvider),
    );
  }
}
