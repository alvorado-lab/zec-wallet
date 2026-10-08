import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/hide_balance.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_request.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_address_scanner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// S13 Build A, the Send form (plan §1.2 as amended by §1a): the comma, Paste
/// and Scan, the host-request guards on both (H3), and the review's
/// "Recipient gets" line (M2). Behaviour, never pixels.
void main() {
  FakeWalletSession funded() => FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 1),
    snapshotValue: walletStateFixture(
      syncStatus: const SyncStatus.upToDate(tip: 1),
      balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
    ),
  );

  /// What the camera "reads" in a test; null is a cancel.
  String? scanned;

  Widget harness(
    FakeWalletSession session, {
    WalletSendRequest? prefill,
    bool scanSupported = true,
    List<Override> extra = const [],
  }) => ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      addressScannerSupportedProvider.overrideWithValue(scanSupported),
      addressScannerProvider.overrideWithValue((_) async => scanned),
      ...extra,
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: SendScreen(prefill: prefill),
    ),
  );

  WalletLocalizations l10n(WidgetTester tester) =>
      WalletLocalizations.of(tester.element(find.byType(SendScreen)));

  TextEditingController field(WidgetTester tester, int i) =>
      tester.widget<TextField>(find.byType(TextField).at(i)).controller!;

  final paste = find.byKey(const ValueKey('send-paste'));
  final scan = find.byKey(const ValueKey('send-scan'));
  final scanFault = find.byKey(const ValueKey('send-scan-fault'));

  setUp(() => scanned = null);

  Future<void> review(WidgetTester tester) async {
    await tester.tap(find.text(l10n(tester).walletSendReviewButton));
    await tester.pumpAndSettle();
  }

  group('the comma (M1)', () {
    testWidgets('typing "0,5" sends 0.5 ZEC — the session records '
        '50,000,000 zat', (tester) async {
      final session = funded();
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 'u1recipient');
      await tester.enterText(find.byType(TextField).at(1), '0,5');
      await tester.pump();
      expect(field(tester, 1).text, '0.5');
      await review(tester);
      expect(session.lastComposeAmountZat, 50000000);
    });

    testWidgets('a second separator is refused, the field keeps 0.5', (
      tester,
    ) async {
      await tester.pumpWidget(harness(funded()));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(1), '0,5');
      await tester.enterText(find.byType(TextField).at(1), '0,5,');
      await tester.pump();
      expect(field(tester, 1).text, '0.5');
    });
  });

  group('Paste', () {
    testWidgets('fills the recipient and classifies it', (tester) async {
      final session = funded();
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async => call.method == 'Clipboard.getData'
            ? <String, dynamic>{'text': '  u1pasted  '}
            : null,
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      await tester.tap(paste);
      await tester.pumpAndSettle();
      expect(field(tester, 0).text, 'u1pasted');
      expect(session.lastValidatedRecipient, 'u1pasted');
      expect(
        find.text(l10n(tester).walletSendRecipientShielded),
        findsOneWidget,
      );
    });
  });

  group('Scan', () {
    testWidgets('a zcash: URI replaces the whole draft — recipient, amount '
        'and memo — through the SDK parser', (tester) async {
      final session = funded()
        ..parseSendRequestResult = const WalletSendRequest(
          address: 'u1scanned',
          amountZat: 250000000,
          memo: 'thanks',
          label: 'never shown',
        );
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      // A typed draft the scan must REPLACE, not merge with.
      await tester.enterText(find.byType(TextField).at(0), 'u1typed');
      await tester.enterText(find.byType(TextField).at(1), '9');
      await tester.enterText(find.byType(TextField).at(2), 'typed memo');
      scanned = 'zcash:u1scanned?amount=2.5&memo=dGhhbmtz';
      await tester.tap(scan);
      await tester.pumpAndSettle();
      expect(session.lastParsedUri, scanned);
      expect(field(tester, 0).text, 'u1scanned');
      expect(field(tester, 1).text, '2.5');
      expect(field(tester, 2).text, 'thanks');
      expect(find.text('never shown'), findsNothing);
      expect(scanFault, findsNothing);
    });

    testWidgets('a URI with no amount or memo clears the typed ones', (
      tester,
    ) async {
      final session = funded()
        ..parseSendRequestResult = const WalletSendRequest(address: 'u1b');
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(1), '9');
      await tester.enterText(find.byType(TextField).at(2), 'typed memo');
      scanned = 'zcash:u1b';
      await tester.tap(scan);
      await tester.pumpAndSettle();
      expect(field(tester, 0).text, 'u1b');
      expect(field(tester, 1).text, '');
      expect(field(tester, 2).text, '');
    });

    testWidgets('a malformed URI faults inline and changes nothing', (
      tester,
    ) async {
      final session = funded()
        ..parseSendRequestThrows = const WalletSendRequestException(
          WalletSendRequestFault.malformed,
        );
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 'u1typed');
      await tester.enterText(find.byType(TextField).at(1), '9');
      scanned = 'zcash:garbage';
      await tester.tap(scan);
      await tester.pumpAndSettle();
      expect(scanFault, findsOneWidget);
      expect(field(tester, 0).text, 'u1typed');
      expect(field(tester, 1).text, '9');
      // The next recipient edit clears the fault.
      await tester.enterText(find.byType(TextField).at(0), 'u1other');
      await tester.pump();
      expect(scanFault, findsNothing);
    });

    testWidgets('an oversized scan is refused before the parser sees it', (
      tester,
    ) async {
      final session = funded();
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      scanned = 'zcash:${'a' * 2100}';
      await tester.tap(scan);
      await tester.pumpAndSettle();
      expect(session.parseSendRequestCount, 0);
      expect(scanFault, findsOneWidget);
      expect(field(tester, 0).text, '');
    });

    testWidgets('a bare address fills the recipient only; text that is not '
        'an address faults and changes nothing', (tester) async {
      final session = funded();
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(1), '3');
      scanned = 'u1bare';
      await tester.tap(scan);
      await tester.pumpAndSettle();
      expect(field(tester, 0).text, 'u1bare');
      expect(field(tester, 1).text, '3', reason: 'recipient only');
      expect(session.parseSendRequestCount, 0);

      session.validateRecipientThrows = WalletApiError(
        code: 'RW',
        message: 's',
        kind: const WalletErrorKind.addressInvalid(),
      );
      scanned = 'hello world';
      await tester.tap(scan);
      await tester.pumpAndSettle();
      expect(scanFault, findsOneWidget);
      expect(field(tester, 0).text, 'u1bare');
    });

    testWidgets('no Scan where the camera reader is unsupported', (
      tester,
    ) async {
      await tester.pumpWidget(harness(funded(), scanSupported: false));
      await tester.pumpAndSettle();
      expect(scan, findsNothing);
      expect(paste, findsOneWidget);
    });
  });

  group("H3 — Scan and Paste never override a host's request", () {
    testWidgets('not drawn over a locked recipient', (tester) async {
      await tester.pumpWidget(
        harness(
          funded(),
          prefill: const WalletSendRequest(
            address: 'u1contact',
            lockRecipient: true,
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(paste, findsNothing);
      expect(scan, findsNothing);
    });

    testWidgets('not drawn beside a host machine memo', (tester) async {
      await tester.pumpWidget(
        harness(
          funded(),
          prefill: WalletSendRequest(
            address: 'u1contact',
            machineMemo: WalletMachineMemo(
              bytes: const [0x52, 0x4C, 0x01],
              purpose: 'so this payment shows up in your chat',
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(paste, findsNothing);
      expect(scan, findsNothing);
    });

    testWidgets('drawn on an ordinary prefill (an unlocked request)', (
      tester,
    ) async {
      await tester.pumpWidget(
        harness(funded(), prefill: const WalletSendRequest(address: 'u1x')),
      );
      await tester.pumpAndSettle();
      expect(paste, findsOneWidget);
      expect(scan, findsOneWidget);
    });
  });

  group('M2 — the review shows what the recipient gets', () {
    final gets = find.byKey(const ValueKey('send-review-recipient-gets'));

    testWidgets("FR-46's singleRecipientZat, fee and change excluded, and "
        'never masked by Hide balance', (tester) async {
      final proposal = sendProposalFixture(
        totalZat: 150010000,
        feeZat: 10000,
        singleRecipientZat: 150000000,
      );
      expect(
        proposal.singleRecipientZat! + proposal.feeZat,
        proposal.totalZat,
        reason: 'the fixture is a coherent payment',
      );
      final session = funded()..proposeResult = proposal;
      await tester.pumpWidget(
        harness(
          session,
          extra: [walletBalanceHiddenProvider.overrideWith(_Hidden.new)],
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 'u1r');
      await tester.enterText(find.byType(TextField).at(1), '1.5');
      await review(tester);
      expect(gets, findsOneWidget);
      expect(
        find.descendant(of: gets, matching: find.textContaining('1.5')),
        findsWidgets,
      );
      expect(
        find.descendant(of: gets, matching: find.textContaining('•')),
        findsNothing,
      );
    });

    testWidgets("a TEX two-step shows FR-46's figure — not either step's "
        'recipient row', (tester) async {
      // Every figure distinct, so reading the wrong source is visible: the
      // first leg (to the one-time address), the last step's row, and FR-46's
      // own singleRecipientZat (§1a M2: "Not `steps.last.recipients`").
      final session = funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 100020000,
          feeZat: 20000,
          isTwoStepTex: true,
          singleRecipientZat: 100000000,
          steps: const [
            ProposalStep(
              recipients: [
                ProposalRecipient(
                  pool: OutputPool.transparent,
                  amountZat: 100010000,
                ),
              ],
            ),
            ProposalStep(
              recipients: [
                ProposalRecipient(
                  pool: OutputPool.transparent,
                  amountZat: 100005000,
                ),
              ],
            ),
          ],
        );
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 'texrecipient');
      await tester.enterText(find.byType(TextField).at(1), '1');
      await review(tester);
      final l10n = WalletLocalizations.of(tester.element(gets));
      expect(
        find.descendant(of: gets, matching: find.text(l10n.walletAmount('1'))),
        findsOneWidget,
        reason: 'singleRecipientZat, 1 ZEC',
      );
      for (final wrong in ['1.0001', '1.00005']) {
        expect(
          find.descendant(of: gets, matching: find.textContaining(wrong)),
          findsNothing,
          reason: 'a step row ($wrong) is not what the recipient gets',
        );
      }
    });

    testWidgets('no line when there is more than one recipient', (
      tester,
    ) async {
      final session = funded()
        ..proposeResult = sendProposalFixture(singleRecipientZat: null);
      await tester.pumpWidget(harness(session));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 'u1r');
      await tester.enterText(find.byType(TextField).at(1), '1');
      await review(tester);
      expect(gets, findsNothing);
    });
  });
}

class _Hidden extends WalletBalanceHidden {
  @override
  bool build() => true;
}
