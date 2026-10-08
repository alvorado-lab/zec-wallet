import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/labeled_zat_row.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// A non-linear text scaler in the shape real iOS/Android use: it boosts small
/// (body) text MORE than large (title) text. Used to prove the money rows in one
/// card flip to the stacked shape together — probing each row's own font size
/// would stack the small rows before the larger emphasized total (a ragged
/// sign-off sheet), invisible under [TextScaler.linear] where the ratio is flat.
class _NonLinearScaler extends TextScaler {
  const _NonLinearScaler();

  @override
  double scale(double fontSize) =>
      fontSize <= 14 ? fontSize * 1.45 : fontSize * 1.30;

  @override
  double get textScaleFactor => 1.45;
}

/// The money-row SSOT: a label + an EXACT zatoshi amount. The amount is the
/// figure a user signs off on, so the guarantee under test is that it is NEVER
/// clipped — the layout stacks above ~1.4× text scale and, at the extreme,
/// scales the figure down rather than truncating it. Regression pin for #329
/// (measured 29px overflow at ru/2.0×/320dp).
void main() {
  // A long ru-style label — "Spendable now" — boxed to a 320dp phone's balance
  // card content width (320 − 2×20 padding = 280), the surface that overflowed.
  const label = 'Доступно сейчас';
  const cardContentWidth = 280.0;

  Widget harness({
    double scale = 1.0,
    required int amountZat,
    String labelText = label,
    Color? tint,
    bool emphasize = false,
    bool boldText = false,
    TextScaler? scaler,
    double width = cardContentWidth,
  }) {
    return MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: width,
            // Apply the text scale INSIDE the MaterialApp media context so it is
            // not overwritten by MaterialApp's own view-derived MediaQuery.
            child: Builder(
              builder: (context) => MediaQuery(
                data: MediaQuery.of(context).copyWith(
                  textScaler: scaler ?? TextScaler.linear(scale),
                  boldText: boldText,
                ),
                child: LabeledZatRow(
                  label: labelText,
                  amountZat: amountZat,
                  tint: tint,
                  emphasize: emphasize,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  String valueOf(WidgetTester tester, int amountZat) {
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(LabeledZatRow)),
    );
    return l10n.walletAmount(formatZec(amountZat));
  }

  testWidgets('at ordinary scale the label and amount share one row', (
    tester,
  ) async {
    const amount = 123456789; // 1.23456789 ZEC
    await tester.pumpWidget(harness(scale: 1.0, amountZat: amount));
    await tester.pumpAndSettle();

    final labelFinder = find.text(label);
    final valueFinder = find.text(valueOf(tester, amount));
    expect(labelFinder, findsOneWidget);
    expect(valueFinder, findsOneWidget);
    // Same line: the amount's top sits above the label's bottom (they overlap
    // vertically), and the amount is to the RIGHT of the label.
    expect(
      tester.getTopLeft(valueFinder).dy,
      lessThan(tester.getBottomLeft(labelFinder).dy),
      reason: 'compact row: value shares the label line',
    );
    expect(
      tester.getTopLeft(valueFinder).dx,
      greaterThan(tester.getTopRight(labelFinder).dx),
      reason: 'compact row: value is on the right',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'at ru/2.0×/320dp the amount STACKS under the label — no overflow, '
    'no clipped figure (#329 regression)',
    (tester) async {
      const amount = 123456789; // 1.23456789 ZEC
      await tester.pumpWidget(harness(scale: 2.0, amountZat: amount));
      await tester.pumpAndSettle();

      final labelFinder = find.text(label);
      final valueFinder = find.text(valueOf(tester, amount));
      expect(labelFinder, findsOneWidget);
      // The whole figure is present in the tree — not ellipsised away.
      expect(valueFinder, findsOneWidget);
      // Stacked: the amount is BELOW the label, holding the full width.
      expect(
        tester.getTopLeft(valueFinder).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(labelFinder).dy),
        reason: 'above 1.4× the amount moves under the label',
      );
      // The measured defect was a RenderFlex overflow — assert it is gone.
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('a large realistic balance does not overflow the compact row', (
    tester,
  ) async {
    // ~123,456.78901234 ZEC — a plausibly large holding at ordinary scale.
    const amount = 12345678901234;
    await tester.pumpWidget(harness(scale: 1.0, amountZat: amount));
    await tester.pumpAndSettle();

    expect(find.text(valueOf(tester, amount)), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'an extreme-width amount stacks even BELOW the 1.4× scale threshold so it '
    'is never clipped (the residual-overflow window the scale probe alone left)',
    (tester) async {
      // ~21M ZEC (the widest possible figure) at 1.3× — under the scale
      // threshold, so only the MEASURED fit check can catch it — on a narrow
      // send-confirm-width box. It must move to its own line, not overrun.
      const amount = 2099999999999999;
      await tester.pumpWidget(
        harness(scale: 1.3, amountZat: amount, width: 240),
      );
      await tester.pumpAndSettle();

      final labelFinder = find.text(label);
      final valueFinder = find.text(valueOf(tester, amount));
      expect(valueFinder, findsOneWidget);
      expect(
        tester.getTopLeft(valueFinder).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(labelFinder).dy),
        reason: 'a figure too wide to share the row stacks regardless of scale',
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'the amount is never clipped even at 3.0× on a narrow box — it scales '
    'down with every digit intact',
    (tester) async {
      const amount = 2099999999999999; // ~21M ZEC, 8 decimals — the widest case
      await tester.pumpWidget(
        harness(scale: 3.0, amountZat: amount, width: 160),
      );
      await tester.pumpAndSettle();

      // The complete figure is still in the widget tree (FittedBox scales the
      // render, it does not truncate the string) and nothing overflowed.
      expect(find.text(valueOf(tester, amount)), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('tint recolours BOTH the label and the amount', (tester) async {
    const amount = 5000000;
    const orange = Color(0xFFEE9944);
    await tester.pumpWidget(
      harness(scale: 1.0, amountZat: amount, tint: orange),
    );
    await tester.pumpAndSettle();

    final labelStyle = tester.widget<Text>(find.text(label)).style;
    final valueStyle = tester
        .widget<Text>(find.text(valueOf(tester, amount)))
        .style;
    expect(labelStyle?.color, orange);
    expect(valueStyle?.color, orange);
  });

  testWidgets('emphasize bolds the amount (SSOT total-row behaviour)', (
    tester,
  ) async {
    const amount = 5000000;
    await tester.pumpWidget(
      harness(scale: 1.0, amountZat: amount, emphasize: true),
    );
    await tester.pumpAndSettle();

    final valueStyle = tester
        .widget<Text>(find.text(valueOf(tester, amount)))
        .style;
    expect(valueStyle?.fontWeight, FontWeight.w600);
  });

  testWidgets('label and amount announce as ONE merged screen-reader node', (
    tester,
  ) async {
    await tester.pumpWidget(harness(scale: 1.0, amountZat: 5000000));
    await tester.pumpAndSettle();

    expect(
      find.descendant(
        of: find.byType(LabeledZatRow),
        matching: find.byType(MergeSemantics),
      ),
      findsOneWidget,
    );
  });

  testWidgets(
    'the emphasized total stacks at the BODY-row threshold under non-linear OS '
    'scaling — every row in a card flips together, no ragged sign-off sheet',
    (tester) async {
      // Under the non-linear scaler the 13px body rows are past 1.4× while the
      // 15px emphasized total is still under its OWN 1.4×. Probing the body
      // reference for ALL rows makes the emphasized total stack too — so a
      // confirm card never shows minor rows stacked beside a compact total.
      const amount = 5000000;
      await tester.pumpWidget(
        harness(
          scaler: const _NonLinearScaler(),
          amountZat: amount,
          emphasize: true,
        ),
      );
      await tester.pumpAndSettle();

      final labelFinder = find.text(label);
      final valueFinder = find.text(valueOf(tester, amount));
      expect(
        tester.getTopLeft(valueFinder).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(labelFinder).dy),
        reason:
            'the emphasized total uses the body-row reference, not its own '
            'larger size, so it stacks with the rows it summarizes',
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'the OS bold-text setting is accounted for in the fit measure — a '
    'near-boundary amount does not overrun the compact row when bold',
    (tester) async {
      // Bold text renders the amount wider than the raw style; the measure now
      // applies it, so the fit check matches the real render. A wide-ish figure
      // at 1.3× (below the scale stack) on a snug width with bold ON must not
      // overflow — it either fits or stacks, never clips.
      const amount = 999999999999; // ~9999.99999999 ZEC
      await tester.pumpWidget(
        harness(scale: 1.3, amountZat: amount, boldText: true, width: 220),
      );
      await tester.pumpAndSettle();

      expect(find.text(valueOf(tester, amount)), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}
