import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_slippage.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// Slippage model + control tests (§3.3b D4). The pure functions (tier, parse,
/// format) are unit-tested at their boundaries (gate 7 — no magic numbers); the
/// widget test pins the preset/custom behavior. The SDK is the authority — these
/// are the host-side convenience layer.
void main() {
  group('slippageTier boundaries', () {
    test('just below the may-fail floor', () {
      expect(slippageTier(kSlippageMayFailBelowBps - 1), SlippageTier.mayFail);
    });
    test('exactly at the may-fail floor is normal', () {
      expect(slippageTier(kSlippageMayFailBelowBps), SlippageTier.normal);
    });
    test('the default is normal', () {
      expect(slippageTier(kSlippageDefaultBps), SlippageTier.normal);
    });
    test('exactly at the risky threshold is still normal', () {
      expect(slippageTier(kSlippageRiskyAboveBps), SlippageTier.normal);
    });
    test('just above the risky threshold', () {
      expect(slippageTier(kSlippageRiskyAboveBps + 1), SlippageTier.risky);
    });
    test('exactly at the hard ceiling is risky, not rejected', () {
      expect(slippageTier(kSlippageMaxBps), SlippageTier.risky);
    });
    test('one bp over the hard ceiling is tooHigh', () {
      expect(slippageTier(kSlippageMaxBps + 1), SlippageTier.tooHigh);
    });
  });

  group('parseSlippagePercentToBps', () {
    test('whole percent', () => expect(parseSlippagePercentToBps('2'), 200));
    test(
      'fractional percent',
      () => expect(parseSlippagePercentToBps('2.5'), 250),
    );
    test('half percent', () => expect(parseSlippagePercentToBps('0.5'), 50));
    test(
      'trims whitespace',
      () => expect(parseSlippagePercentToBps('  1  '), 100),
    );
    test('empty is null', () => expect(parseSlippagePercentToBps(''), isNull));
    test(
      'non-numeric is null',
      () => expect(parseSlippagePercentToBps('abc'), isNull),
    );
    test(
      'negative is null',
      () => expect(parseSlippagePercentToBps('-1'), isNull),
    );
  });

  group('formatBpsAsPercent', () {
    test(
      'whole percent trims the decimal',
      () => expect(formatBpsAsPercent(200), '2'),
    );
    test('fractional percent', () => expect(formatBpsAsPercent(250), '2.5'));
    test('half percent', () => expect(formatBpsAsPercent(50), '0.5'));
  });

  group('SlippageControl widget', () {
    Widget harness({
      required int valueBps,
      required ValueChanged<int> onChanged,
    }) => MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: Scaffold(
        body: SlippageControl(valueBps: valueBps, onChanged: onChanged),
      ),
    );

    testWidgets('renders the presets + custom chip', (tester) async {
      await tester.pumpWidget(harness(valueBps: 200, onChanged: (_) {}));
      await tester.pumpAndSettle();
      expect(find.text('0.5%'), findsOneWidget);
      expect(find.text('1%'), findsOneWidget);
      expect(find.text('2%'), findsOneWidget);
    });

    testWidgets('tapping a preset reports its bps', (tester) async {
      int? got;
      await tester.pumpWidget(
        harness(valueBps: 200, onChanged: (v) => got = v),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('1%'));
      await tester.pumpAndSettle();
      expect(got, 100);
    });

    testWidgets('a too-tight value shows the may-fail advisory', (
      tester,
    ) async {
      await tester.pumpWidget(harness(valueBps: 10, onChanged: (_) {}));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SlippageControl)),
      );
      expect(find.text(l10n.walletSwapSlippageMayFail), findsOneWidget);
    });

    testWidgets('S13: a custom "1,5" is 1.5% (150 bps); a second separator is '
        'refused', (tester) async {
      int? got;
      await tester.pumpWidget(
        harness(valueBps: 200, onChanged: (v) => got = v),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SlippageControl)),
      );
      await tester.tap(find.text(l10n.walletSwapSlippageCustom));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '1,5');
      await tester.pump();
      expect(got, 150);
      await tester.enterText(find.byType(TextField), '1,5,');
      await tester.pump();
      expect(
        tester.widget<TextField>(find.byType(TextField)).controller!.text,
        '1.5',
      );
    });
  });
}
