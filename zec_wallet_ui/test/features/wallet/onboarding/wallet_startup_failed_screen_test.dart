import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_startup_failed_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// The boot-wiring failure surface (#356-F1). Load-bearing properties: the
/// HONEST couldn't-start copy (never the not-set-up "arrives in a later
/// build"), a funds-are-safe reassurance, and a retry that actually fires —
/// disabled (with the in-progress spinner) while a retry is in flight.
void main() {
  Widget harness({required VoidCallback onRetry, bool retrying = false}) {
    return MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: WalletStartupFailedScreen(onRetry: onRetry, retrying: retrying),
    );
  }

  WalletLocalizations l10nOf(WidgetTester tester) => WalletLocalizations.of(
    tester.element(find.byType(WalletStartupFailedScreen)),
  );

  testWidgets('renders the honest couldn\'t-start copy + a live retry', (
    tester,
  ) async {
    var retries = 0;
    await tester.pumpWidget(harness(onRetry: () => retries++));
    await tester.pumpAndSettle();
    final l10n = l10nOf(tester);

    expect(find.text(l10n.walletStartupFailedTitle), findsOneWidget);
    expect(find.text(l10n.walletStartupFailedBody), findsOneWidget);
    // The false not-set-up copy must NOT be this surface.
    expect(find.text(l10n.walletNotSetUpBody), findsNothing);

    await tester.tap(find.text(l10n.walletOnboardingRetry));
    expect(retries, 1);
  });

  testWidgets('retrying: the button is disabled and shows the spinner', (
    tester,
  ) async {
    var retries = 0;
    await tester.pumpWidget(harness(onRetry: () => retries++, retrying: true));
    await tester.pump();
    final l10n = l10nOf(tester);

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    await tester.tap(find.text(l10n.walletOnboardingRetry));
    expect(retries, 0, reason: 'no double-fire while a retry is in flight');
  });
}
