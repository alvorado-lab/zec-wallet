import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/tx_detail_sheet.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Direct pumps of the (public) sheet body — the shield-sheet test idiom; the
/// open-from-the-row integration lives in wallet_screen_test.dart.
Widget _harness(TxSummary tx) => ProviderScope(
  // No session ⇒ the activity provider holds empty rows and the sheet falls
  // back to the tapped summary — the direct-pump contract under test.
  child: MaterialApp(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: lightTheme,
    home: Scaffold(body: TxDetailSheet(tx: tx)),
  ),
);

WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(TxDetailSheet)));

void main() {
  testWidgets('renders every field the summary carries', (tester) async {
    await tester.pumpWidget(_harness(txSummaryFixture(feeZat: 10000)));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // Fixture default: incoming, confirmed(depth 5), mined 100, memo, dated.
    expect(find.text(l10n.walletActivityReceived), findsOneWidget);
    expect(find.text(l10n.walletTxDetailStatus), findsOneWidget);
    expect(
      find.textContaining(l10n.walletActivityConfirmations(5)),
      findsOneWidget,
    );
    expect(find.text(l10n.walletTxExplainConfirmed), findsOneWidget);
    expect(find.text(l10n.walletTxDetailFee), findsOneWidget);
    expect(find.text(l10n.walletTxDetailDate), findsOneWidget);
    expect(find.text(l10n.walletTxDetailHeight), findsOneWidget);
    expect(find.text(l10n.walletTxDetailMemo), findsOneWidget);
    expect(find.text(l10n.walletTxDetailMemoAttached), findsOneWidget);
    expect(find.text(l10n.walletTxDetailTxid), findsOneWidget);
    // No cancelled banner on a healthy row.
    expect(find.text(l10n.walletTxFundsKept), findsNothing);
  });

  testWidgets('optional fields HIDE their rows instead of rendering blanks', (
    tester,
  ) async {
    await tester.pumpWidget(
      _harness(
        txSummaryFixture(
          feeZat: null,
          timestamp: null,
          minedHeight: null,
          hasMemo: false,
          status: const TxStatus.pending(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletTxDetailFee), findsNothing);
    expect(find.text(l10n.walletTxDetailDate), findsNothing);
    expect(find.text(l10n.walletTxDetailHeight), findsNothing);
    expect(find.text(l10n.walletTxDetailMemo), findsNothing);
    expect(find.text(l10n.walletTxExplainPending), findsOneWidget);
  });

  testWidgets('copy puts the FULL txid on the clipboard and confirms', (
    tester,
  ) async {
    final calls = <MethodCall>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        calls.add(call);
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );

    const txid =
        'ab12000000000000000000000000000000000000000000000000000000003399';
    await tester.pumpWidget(_harness(txSummaryFixture(txidHex: txid)));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await tester.ensureVisible(find.text(l10n.walletTxDetailCopyTxid));
    await tester.tap(find.text(l10n.walletTxDetailCopyTxid));
    await tester.pumpAndSettle();

    final copies = calls.where((c) => c.method == 'Clipboard.setData').toList();
    expect(copies, hasLength(1));
    // The FULL id — the display shortens visually (edge emphasis), the copy
    // must not.
    expect((copies.single.arguments as Map)['text'], txid);
    expect(find.text(l10n.walletTxDetailCopied), findsOneWidget);
  });

  testWidgets('unknown status renders as Pending — forward-compat, never a '
      'crash or a false claim', (tester) async {
    await tester.pumpWidget(
      _harness(
        txSummaryFixture(status: const TxStatus.unknown(), minedHeight: null),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // The LABEL falls back to Pending, but the explanatory sentence stays
    // NEUTRAL: it must not affirm a broadcast the binding can't verify.
    expect(find.text(l10n.walletTxExplainUnknown), findsOneWidget);
    expect(find.text(l10n.walletTxExplainPending), findsNothing);
    // NOT cancelled: an unknown arm must never claim "no funds left".
    expect(find.text(l10n.walletTxFundsKept), findsNothing);
  });

  testWidgets('S7 C1: an EXPIRED attempt the wallet will send again is not '
      'struck or called "no funds left" — it says the wallet re-sends it and '
      'not to send it again', (tester) async {
    TxSummary expired({DeliveryState? delivery}) => TxSummary(
      txidHex: 'bb' * 32,
      status: const TxStatus.expired(),
      netAmountZat: -150000,
      hasMemo: false,
      hasTransparentOutput: false,
      delivery: delivery,
    );
    TextDecoration? amountDecoration(WalletLocalizations l10n) => tester
        .widget<Text>(find.text(l10n.walletAmount('−0.0015')))
        .style
        ?.decoration;

    // Control: a plain expired row (no delivery reading) IS cancelled — so
    // the negative assertions below measure the refinement, not a harness
    // that never strikes anything.
    await tester.pumpWidget(_harness(expired()));
    await tester.pumpAndSettle();
    var l10n = _l10n(tester);
    expect(find.text(l10n.walletTxFundsKept), findsOneWidget);
    expect(amountDecoration(l10n), TextDecoration.lineThrough);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpWidget(
      _harness(expired(delivery: DeliveryState.retryPending)),
    );
    await tester.pumpAndSettle();
    l10n = _l10n(tester);
    expect(find.text(l10n.walletTxFundsKept), findsNothing);
    expect(amountDecoration(l10n), isNot(TextDecoration.lineThrough));
    expect(find.text(l10n.walletTxExplainRetryingExpired), findsOneWidget);
    expect(find.text(l10n.walletTxExplainExpired), findsNothing);
    // The label keeps the existing refinement.
    expect(find.text(l10n.walletActivityRetrying), findsOneWidget);
  });
}
