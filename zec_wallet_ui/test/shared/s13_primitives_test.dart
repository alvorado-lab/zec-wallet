import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/wallet_cta.dart';
import 'package:zec_wallet_ui/shared/wallet_info_button.dart';

/// S13 §1.1 — the two new primitives, by what a user and a host get from
/// them: the (i)'s target, name and sheet; the CTA's height floor under a
/// host's own button theme.
void main() {
  Widget app(Widget child, {ThemeData? theme}) => MaterialApp(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: theme ?? lightTheme,
    home: Scaffold(body: Center(child: child)),
  );

  group('WalletInfoButton', () {
    const button = WalletInfoButton(
      label: 'Queue',
      body: 'The whole explanation, moved here from the screen.',
    );

    testWidgets('a 44 target named "More about <label>"', (tester) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(app(button));
      expect(tester.getSize(find.byType(WalletInfoButton)), const Size(44, 44));
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletInfoButton)),
      );
      expect(
        find.bySemanticsLabel(l10n.walletInfoButtonLabel('Queue')),
        findsOneWidget,
      );
      expect(l10n.walletInfoButtonLabel('Queue'), 'More about Queue');
      handle.dispose();
    });

    testWidgets('a tap opens the moved text whole, and it closes', (
      tester,
    ) async {
      await tester.pumpWidget(app(button));
      await tester.tap(find.byType(WalletInfoButton));
      await tester.pumpAndSettle();
      expect(
        find.text('The whole explanation, moved here from the screen.'),
        findsOneWidget,
      );
      expect(find.text('Queue'), findsOneWidget, reason: 'titled by label');
      await tester.tap(find.byKey(const ValueKey('wallet-info-close')));
      await tester.pumpAndSettle();
      expect(
        find.text('The whole explanation, moved here from the screen.'),
        findsNothing,
      );
    });
  });

  group('WalletCta', () {
    // A host theme whose buttons keep the Material 48 minimum.
    final hostTheme = lightTheme.copyWith(
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(minimumSize: const Size(64, 48)),
      ),
    );

    for (final (size, floor) in [
      (WalletCtaSize.page, 56.0),
      (WalletCtaSize.sheet, 52.0),
    ]) {
      testWidgets('${size.name}: at least $floor tall under a 48 host '
          'minimum, and still the themed button', (tester) async {
        await tester.pumpWidget(
          app(
            WalletCta(
              size: size,
              child: FilledButton(onPressed: () {}, child: const Text('Go')),
            ),
            theme: hostTheme,
          ),
        );
        expect(
          tester.getSize(find.byType(FilledButton)).height,
          greaterThanOrEqualTo(floor),
        );
        expect(WalletCta.floorOf(size), floor);
      });
    }
  });
}
