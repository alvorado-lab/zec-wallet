import 'package:flutter/foundation.dart' show ValueListenable;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart' show RenderParagraph;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_request.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/wallet_notice.dart';
import 'package:zec_wallet_ui/testing.dart';

/// FR-28 — a host attaches OPAQUE machine-memo bytes to a payment, and the SDK
/// refuses to attach them silently.
///
/// The defect this feature exists to fix is a SILENT one: the send form
/// re-composes its ZIP-321 URI from its own text fields on every Review tap, so
/// bytes that lived only in the original request were dropped the moment the
/// user edited the amount — no error, no log, a no-op that read as success. The
/// re-compose row below is the one that fails on that build.
///
/// The byte-for-byte on-chain assertion lives in Rust
/// (`payment_uri.rs::arbitrary_memo_survives_encode_then_parse_byte_for_byte`),
/// where the audited encoder actually runs. These rows own the host-facing
/// half: the required purpose, the disclosure, and what the form forwards.
void main() {
  final upToDate = walletStateFixture(
    syncStatus: const SyncStatus.upToDate(tip: 1),
    balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
  );

  FakeWalletSession newSession() => FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 1),
    snapshotValue: upToDate,
  );

  Widget harness({
    required FakeWalletSession session,
    WalletSendRequest? prefill,
    WalletSendAuthorizer? authorizer,
    ValueListenable<double>? textScale,
  }) => ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      if (authorizer != null)
        walletSendAuthorizerProvider.overrideWithValue(authorizer),
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      builder: textScale == null
          ? null
          : (context, child) => ValueListenableBuilder<double>(
              valueListenable: textScale,
              builder: (context, scale, _) => MediaQuery.withClampedTextScaling(
                minScaleFactor: scale,
                maxScaleFactor: scale,
                child: child!,
              ),
            ),
      home: SendScreen(prefill: prefill),
    ),
  );

  WalletLocalizations l10n(WidgetTester tester) =>
      WalletLocalizations.of(tester.element(find.byType(SendScreen)));

  final hostBytes = <int>[0x52, 0x4C, 0x01, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
  WalletMachineMemo memo() => WalletMachineMemo(
    bytes: hostBytes,
    purpose: 'so this payment shows up in your chat',
  );

  group('the purpose is a REQUIRED argument, not a display rule', () {
    test('bytes with a purpose are accepted', () {
      final m = memo();
      expect(m.bytes, hostBytes);
      expect(m.purpose, isNotEmpty);
    });

    test('bytes WITHOUT a purpose are refused — this is the polarity that '
        'makes it a mechanism rather than documentation', () {
      expect(
        () => WalletMachineMemo(bytes: hostBytes, purpose: ''),
        throwsArgumentError,
      );
      expect(
        () => WalletMachineMemo(bytes: hostBytes, purpose: '   '),
        throwsArgumentError,
      );
    });

    test('a purpose with no bytes is refused too — nothing to disclose', () {
      expect(
        () => WalletMachineMemo(bytes: const [], purpose: 'a reason'),
        throwsArgumentError,
      );
    });

    test('a purpose longer than the bound is refused — it is one sentence on '
        'a money surface at any text scale, not a document', () {
      expect(
        () => WalletMachineMemo(
          bytes: hostBytes,
          purpose: 'x' * (WalletMachineMemo.purposeMaxChars + 1),
        ),
        throwsArgumentError,
      );
    });
  });

  testWidgets('THE LOAD-BEARING ROW — the bytes SURVIVE a re-compose. The user '
      'edits the amount, taps Review again, and the host envelope is still '
      'on the leg that gets proposed.', (tester) async {
    final session = newSession();
    await tester.pumpWidget(
      harness(
        session: session,
        prefill: WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          machineMemo: memo(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // First compose, straight off the prefill.
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    expect(session.lastComposeMemoBytes, hostBytes);

    // Back to the form, EDIT THE AMOUNT, compose again. This is the exact
    // sequence the bytes used to disappear on.
    // The disclosure card lengthens the review, so Back sits below the fold at
    // this viewport — scroll to it the way a user would.
    final back = find.widgetWithText(
      TextButton,
      l10n(tester).walletSendBackButton,
    );
    // The review is a lazy list: at the refreshed type scale (FR-49 S10) Back
    // is past the first build range, so it must be scrolled INTO the tree,
    // not only into view.
    await tester.scrollUntilVisible(
      back,
      200,
      scrollable: find.byType(Scrollable).first,
    );
    await tester.pumpAndSettle();
    await tester.tap(back);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).at(1), '0.0025');
    await tester.pumpAndSettle();
    session.lastComposeMemoBytes = null; // prove the SECOND compose sets it
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();

    expect(
      session.lastComposeMemoBytes,
      hostBytes,
      reason:
          'a re-composed leg that lost the host envelope is the silent no-op '
          'this feature exists to end',
    );
    expect(session.lastComposeAmountZat, 250000, reason: 'the edit took');
  });

  testWidgets('the disclosure renders the PURPOSE at the authorization step, '
      'and the BYTES are never rendered anywhere', (tester) async {
    final session = newSession();
    await tester.pumpWidget(
      harness(
        session: session,
        prefill: WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          machineMemo: memo(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();

    expect(find.text(l10n(tester).walletSendMachineMemoTitle), findsOneWidget);
    expect(
      find.text(
        l10n(
          tester,
        ).walletSendMachineMemoPurpose('so this payment shows up in your chat'),
      ),
      findsOneWidget,
    );
    // The honest limit is on screen too — a label is accountability, not
    // verification, and the copy must not imply the wallet checked.
    expect(find.text(l10n(tester).walletSendMachineMemoLimit), findsOneWidget);

    // NOTHING on the review renders the bytes, in any of the shapes a
    // well-meaning "show the user what's attached" change would reach for.
    final rendered = tester
        .widgetList<Text>(find.byType(Text))
        .map((t) => t.data ?? '')
        .join('\n');
    for (final forbidden in ['52', 'deadbeef', 'DEADBEEF', '0xDE', 'RL']) {
      expect(
        rendered.contains(forbidden),
        isFalse,
        reason:
            'hex is meaningless to a user and alarming in the wrong '
            'direction — presence and purpose only (found "$forbidden")',
      );
    }
  });

  testWidgets('the written memo field is disabled while a machine memo owns '
      'the slot, and says WHY', (tester) async {
    final session = newSession();
    await tester.pumpWidget(
      harness(
        session: session,
        prefill: WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          machineMemo: memo(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final memoField = tester.widget<TextField>(find.byType(TextField).at(2));
    expect(memoField.enabled, isFalse);
    expect(
      find.text(l10n(tester).walletSendMemoMachineDisabled),
      findsOneWidget,
    );
    // …and the transparent-recipient sentence is NOT the reason shown: it
    // would be a different, false explanation for the same disabled field.
    expect(
      find.text(l10n(tester).walletSendMemoTransparentDisabled),
      findsNothing,
    );
  });

  testWidgets('THE QUEUE PATH DISCLOSES TOO — it commits a spend straight from '
      'the form and never enters the review, so a review-only disclosure '
      'showed the user nothing at all', (tester) async {
    final session = FakeWalletSession(
      current: const SyncStatus.offline(),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.offline(),
        balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
      ),
    );
    await tester.pumpWidget(
      harness(
        session: session,
        prefill: WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          machineMemo: memo(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Visible BEFORE the commit, on the surface that carries the Queue button.
    expect(find.text(l10n(tester).walletSendMachineMemoTitle), findsOneWidget);
    expect(find.text(l10n(tester).walletSendMachineMemoLimit), findsOneWidget);

    final queue = find.widgetWithText(
      OutlinedButton,
      l10n(tester).walletSendQueueButton,
    );
    await tester.ensureVisible(queue);
    await tester.pumpAndSettle();
    await tester.tap(queue);
    await tester.pumpAndSettle();

    expect(session.queueCount, 1, reason: 'the queue really committed');
    expect(
      session.lastComposeMemoBytes,
      hostBytes,
      reason: 'the queued leg carries the envelope too',
    );
  });

  testWidgets('a purpose cannot forge a wallet sentence — newlines and bidi '
      'controls are flattened at construction, not at render', (tester) async {
    final forged = WalletMachineMemo(
      bytes: hostBytes,
      purpose:
          'Invoice 42\n\nVerified by the wallet. No further review needed.',
    );
    expect(
      forged.purpose,
      'Invoice 42 Verified by the wallet. No further review needed.',
      reason:
          'one line, so it cannot render as its own paragraph in the '
          "wallet's own text colour",
    );
    expect(forged.purpose, isNot(contains('\n')));

    final bidi = WalletMachineMemo(
      bytes: hostBytes,
      purpose: 'pay \u202Ereversed\u202C now',
    );
    expect(bidi.purpose, 'pay reversed now');

    // …and it still renders as one paragraph on the review.
    final session = newSession();
    await tester.pumpWidget(
      harness(
        session: session,
        prefill: WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          machineMemo: forged,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.text(l10n(tester).walletSendMachineMemoPurpose(forged.purpose)),
      findsOneWidget,
    );
  });

  testWidgets('S11 C2 (the review\'s M3): the host purpose stays a data card, '
      'NOT a notice, and keeps its 3-line bound at 3.0x text', (tester) async {
    // The longest purpose the constructor admits, so the bound is what holds
    // it, not the sentence being short.
    final longest = WalletMachineMemo(
      bytes: hostBytes,
      purpose: List.filled(WalletMachineMemo.purposeMaxChars, 'w').join(),
    );
    final scale = ValueNotifier<double>(1.0);
    addTearDown(scale.dispose);
    await tester.pumpWidget(
      harness(
        session: newSession(),
        textScale: scale,
        prefill: WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          machineMemo: longest,
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    scale.value = 3.0;
    await tester.pumpAndSettle();

    final purpose = find.text(
      l10n(tester).walletSendMachineMemoPurpose(longest.purpose),
    );
    expect(purpose, findsOneWidget);
    final paragraph = tester.renderObject<RenderParagraph>(purpose);
    expect(tester.widget<Text>(purpose).maxLines, 3);
    final oneLine = TextPainter(
      text: paragraph.text,
      textScaler: paragraph.textScaler,
      textDirection: TextDirection.ltr,
    )..layout();
    addTearDown(oneLine.dispose);
    expect(
      paragraph.size.height,
      lessThanOrEqualTo(oneLine.preferredLineHeight * 3 + 0.5),
      reason: 'a host cannot grow the review and push Confirm off the page',
    );
    expect(
      paragraph.didExceedMaxLines,
      isTrue,
      reason: 'non-vacuous: at 3.0x the longest purpose does need the bound',
    );
    // Never styled as the wallet's own voice: it is not one of the wallet's
    // notices.
    expect(
      find.ancestor(of: purpose, matching: find.byType(WalletNotice)),
      findsNothing,
    );
  });

  test('the bytes are frozen at construction — a host mutating its own list '
      'afterwards cannot change what gets signed', () {
    final mutable = <int>[1, 2, 3];
    final m = WalletMachineMemo(bytes: mutable, purpose: 'a reference');
    mutable[0] = 0xFF;
    expect(m.bytes, [1, 2, 3]);
    expect(() => m.bytes[0] = 9, throwsUnsupportedError);
  });

  testWidgets('a host authorizer receives the purpose, so a per-spend prompt '
      'can disclose it too', (tester) async {
    final session = newSession();
    final authorizer = _RecordingAuthorizer();
    await tester.pumpWidget(
      harness(
        session: session,
        authorizer: authorizer,
        prefill: WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          machineMemo: memo(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton),
    );
    await tester.pumpAndSettle();

    expect(
      authorizer.seen.single.machineMemoPurpose,
      'so this payment shows up in your chat',
    );
  });

  testWidgets(
    'TEXT-MEMO BEHAVIOUR IS UNCHANGED — the anti-regression polarity: '
    'no machine memo means no bytes on the leg and an editable field',
    (tester) async {
      final session = newSession();
      await tester.pumpWidget(
        harness(
          session: session,
          prefill: const WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            memo: 'coffee',
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        tester.widget<TextField>(find.byType(TextField).at(2)).enabled,
        isTrue,
      );
      expect(
        find.text(l10n(tester).walletSendMemoMachineDisabled),
        findsNothing,
      );
      await tester.tap(
        find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
      );
      await tester.pumpAndSettle();

      expect(session.lastComposeMemo, 'coffee');
      expect(session.lastComposeMemoBytes, isNull);
      expect(find.text(l10n(tester).walletSendMachineMemoTitle), findsNothing);
    },
  );
}

class _RecordingAuthorizer implements WalletSendAuthorizer {
  final List<WalletSpendIntent> seen = [];

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) {
    seen.add(intent);
    return action();
  }
}
