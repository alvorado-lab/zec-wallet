import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// S13 §1.3 — Shield and Move cannot be left while they submit: not by the
/// scrim, not by a drag, not by Back. Driven through the real `show…Sheet`
/// entry points, because the scrim and the drag belong to the route.
void main() {
  Widget launcher(
    FakeWalletSession session,
    Future<void> Function(BuildContext) open,
  ) => ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: Scaffold(
        body: Builder(
          builder: (context) => Center(
            child: TextButton(
              onPressed: () => unawaited(open(context)),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ),
  );

  /// Try every way out a user has, and report whether [sheet] survived all.
  Future<void> expectNoExit(WidgetTester tester, Finder sheet) async {
    // The scrim: a tap well above the sheet.
    await tester.tapAt(const Offset(20, 20));
    await tester.pump(const Duration(milliseconds: 400));
    expect(sheet, findsOneWidget, reason: 'the scrim does not close it');
    // A dismissible scrim would route to the Back hold's dialog; clear it so
    // the drag below lands on the sheet, not on a dialog.
    final stay = find.byKey(const ValueKey('sheet-leave-stay'));
    if (stay.evaluate().isNotEmpty) {
      await tester.tap(stay);
      await tester.pump(const Duration(milliseconds: 400));
    }
    // A drag down on the sheet.
    await tester.drag(sheet, const Offset(0, 500));
    await tester.pump(const Duration(milliseconds: 400));
    expect(sheet, findsOneWidget, reason: 'a drag does not close it');
    // Back (what the system back runs) asks instead of leaving; Stay stays.
    unawaited(Navigator.of(tester.element(sheet)).maybePop());
    await tester.pump(const Duration(milliseconds: 400));
    expect(sheet, findsOneWidget, reason: 'Back does not close it');
    final l10n = WalletLocalizations.of(tester.element(sheet));
    expect(find.text(l10n.walletSheetLeaveBody), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('sheet-leave-stay')));
    await tester.pump(const Duration(milliseconds: 400));
    expect(sheet, findsOneWidget, reason: 'Stay keeps the sheet');
  }

  /// H2 on the sheets: Back → Leave always gets the user out, mid-submit.
  Future<void> expectLeave(WidgetTester tester, Finder sheet) async {
    unawaited(Navigator.of(tester.element(sheet)).maybePop());
    await tester.pump(const Duration(milliseconds: 400));
    await tester.tap(find.byKey(const ValueKey('sheet-leave-confirm')));
    // The dialog's exit, then the sheet's (the spinner never settles, so
    // pump in steps rather than pumpAndSettle).
    for (var i = 0; i < 6; i++) {
      await tester.pump(const Duration(milliseconds: 300));
    }
    expect(sheet, findsNothing, reason: 'Leave is never stranded');
  }

  testWidgets('Shield: no exit while submitting; Back works once it lands', (
    tester,
  ) async {
    final gate = Completer<void>();
    final session = FakeWalletSession()
      ..proposeShieldResult = shieldProposalFixture()
      ..sendGate = gate
      ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
    await tester.pumpWidget(launcher(session, showShieldSheet));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    final sheet = find.byType(ShieldSheet);
    final l10n = WalletLocalizations.of(tester.element(sheet));
    await tester.tap(find.text(l10n.walletShieldConfirmButton));
    await tester.pump();
    expect(session.sendCount, 1, reason: 'submitting');

    await expectNoExit(tester, sheet);

    gate.complete();
    await tester.pumpAndSettle();
    unawaited(Navigator.of(tester.element(sheet)).maybePop());
    await tester.pumpAndSettle();
    expect(sheet, findsNothing, reason: 'a settled sheet closes on Back');
  });

  FakeWalletSession moveSession() =>
      FakeWalletSession(current: const SyncStatus.upToDate(tip: 1))
        ..currentTransparentAddressResult = 't1myownTaddr'
        ..setSnapshot(
          walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 1),
            balance: balanceFixture(spendableZat: 100000000),
          ),
        )
        ..proposeResult = sendProposalFixture(hasTransparentRecipient: true);

  testWidgets('Move: no exit while submitting, but Leave always gets out', (
    tester,
  ) async {
    final gate = Completer<void>();
    final session = moveSession()..sendGate = gate;
    await tester.pumpWidget(launcher(session, showMoveToTransparentSheet));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    final sheet = find.byType(MoveToTransparentSheet);
    final l10n = WalletLocalizations.of(tester.element(sheet));
    await tester.enterText(find.byType(TextField), '0.002');
    await tester.tap(find.text(l10n.walletMoveReviewButton));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text(l10n.walletMoveConfirmButton));
    await tester.tap(find.text(l10n.walletMoveConfirmButton));
    await tester.pump();
    expect(session.sendCount, 1, reason: 'submitting');

    await expectNoExit(tester, sheet);
    // The submit never lands (a hung broadcast): Leave still gets out.
    await expectLeave(tester, sheet);
    gate.complete();
    await tester.pumpAndSettle();
  });

  testWidgets('Move: "0,5" in the amount field moves 0.5 ZEC', (tester) async {
    final session = moveSession();
    await tester.pumpWidget(launcher(session, showMoveToTransparentSheet));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    final sheet = find.byType(MoveToTransparentSheet);
    final l10n = WalletLocalizations.of(tester.element(sheet));
    await tester.enterText(find.byType(TextField), '0,5');
    await tester.tap(find.text(l10n.walletMoveReviewButton));
    await tester.pumpAndSettle();
    expect(session.lastComposeAmountZat, 50000000);
  });
}
