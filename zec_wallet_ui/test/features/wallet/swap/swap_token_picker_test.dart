import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/sheet_layout.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_token_picker.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_tokens_provider.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// Stage S11 C4: the token picker opens through the one sheet entry, and so
/// gains the 560 desktop width cap it lacked (the one sheet of the nine
/// without it).
void main() {
  const launcherKey = ValueKey('launcher');

  testWidgets('the token picker is capped at $walletSheetMaxWidth on a wide '
      'window', (tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          swapTokensProvider.overrideWith(
            (ref) => Future.value(swapTokenListFixture()),
          ),
        ],
        child: MaterialApp(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          home: const Scaffold(body: SizedBox(key: launcherKey)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final picked = showSwapTokenPicker(
      tester.element(find.byKey(launcherKey)),
      title: 'Pick a token',
    );
    await tester.pumpAndSettle();

    // `BottomSheet` itself spans the window and centres its constrained
    // `Material` inside; that surface is the widest `Material` under it (the
    // wallet_sheet_test measure).
    final sheet = find.byType(BottomSheet);
    expect(sheet, findsOneWidget);
    final surface = find
        .descendant(of: sheet, matching: find.byType(Material))
        .evaluate()
        .map((e) => tester.getSize(find.byElementPredicate((x) => x == e)))
        .map((s) => s.width)
        .reduce((a, b) => a >= b ? a : b);
    expect(
      surface,
      lessThanOrEqualTo(walletSheetMaxWidth),
      reason: 'the picker stretched across a wide window',
    );
    // The window is wider than the cap, so the cap is what held it.
    expect(tester.view.physicalSize.width, greaterThan(walletSheetMaxWidth));

    // Dismissed, the picker still resolves to "no choice" (its future is the
    // route's, passed through the sheet entry).
    await tester.tapAt(const Offset(10, 10));
    await tester.pumpAndSettle();
    expect(await picked, isNull);
  });
}
