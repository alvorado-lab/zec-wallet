import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations_fallback.dart';

/// Extraction review B2 regression: a host whose supportedLocales is
/// wider than the wallet ARB set must get ENGLISH wallet strings on the
/// untranslated locale — never a crash. (The raw generated delegate reports
/// `isSupported == false` for the uncovered locale, gets skipped by
/// Localizations, and the first `WalletLocalizations.of(context)` then throws
/// — every money surface down, only for non-English users.)
///
/// The probe locale is SWAHILI (`sw`): a real Material-supported locale that
/// is deliberately OUTSIDE the wallet ARB set. (It was `fr` until the UI gained
/// real translations for all 16 reference locales — a covered locale can't
/// exercise the fallback.)
void main() {
  Widget hostApp({
    required LocalizationsDelegate<WalletLocalizations> delegate,
  }) {
    return MaterialApp(
      locale: const Locale('sw'),
      supportedLocales: const [Locale('en'), Locale('sw')],
      localizationsDelegates: [
        delegate,
        ...GlobalMaterialLocalizations.delegates,
      ],
      home: Builder(
        builder: (context) => Text(WalletLocalizations.of(context).walletTitle),
      ),
    );
  }

  testWidgets('a Swahili-device host with [en, sw] gets English wallet strings '
      'through the fallback delegate — not a crash', (tester) async {
    await tester.pumpWidget(
      hostApp(delegate: walletLocalizationsFallbackDelegate),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.text('Wallet'), findsOneWidget);
  });

  testWidgets('the raw generated delegate DOES strand a wider-locale host '
      '(the hazard the fallback exists for — pinned so a future gen-l10n '
      'change that fixes this upstream is noticed)', (tester) async {
    await tester.pumpWidget(hostApp(delegate: WalletLocalizations.delegate));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNotNull);
  });
}
