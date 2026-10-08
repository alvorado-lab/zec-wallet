import 'package:flutter/cupertino.dart' show CupertinoAlertDialog;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/deshield_warning.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/send/form_fault_view.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/address_text.dart';
import 'package:zec_wallet_ui/shared/wallet_notice.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Send-screen widget tests (inc-2d-ui). Money display and the §5.1 de-shield
/// disclosure are the load-bearing assertions: a transposed figure or a missing
/// privacy warning would pass a happy-path review and mislead a user about money
/// or privacy, so both are pinned here over the real widget tree (with a fake
/// session behind the port).
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(SendScreen)));

Widget _harness({
  required FakeWalletSession session,
  FakeScreenSecurity? security,
  bool nullSession = false,
  ThemeData? theme,
  TextScaler? textScaler,
  List<Override> extraOverrides = const [],
}) {
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(nullSession ? null : session),
      if (security != null) screenSecurityProvider.overrideWithValue(security),
      ...extraOverrides,
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: theme ?? lightTheme,
      // Force a text scale (accessibility) when a test asks — money figures and
      // the irreversible action button must stay un-clipped at extreme scale.
      builder: textScaler == null
          ? null
          : (context, child) => MediaQuery(
              data: MediaQuery.of(context).copyWith(textScaler: textScaler),
              child: child!,
            ),
      home: const SendScreen(),
    ),
  );
}

FakeWalletSession _funded({
  SyncStatus current = const SyncStatus.upToDate(tip: 1),
}) {
  return FakeWalletSession(
    current: current,
    snapshotValue: walletStateFixture(
      syncStatus: current,
      balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
    ),
  );
}

Future<void> _enterForm(
  WidgetTester tester, {
  String address = 'u1recipient',
  String amount = '1',
}) async {
  // address = field 0, amount = field 1, memo = field 2.
  await tester.enterText(find.byType(TextField).at(0), address);
  await tester.enterText(find.byType(TextField).at(1), amount);
  await tester.pump();
}

/// The Review button widget — `onPressed == null` means it is DISABLED (the
/// foolproof recipient gate). The default fake classifies any non-empty
/// recipient as shielded-valid, so the existing flows keep it enabled.
FilledButton _reviewButton(WidgetTester tester, WalletLocalizations l10n) =>
    tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.walletSendReviewButton),
    );

/// The memo field (the third TextField) — `.enabled` reflects the live memo gate.
bool _memoEnabled(WidgetTester tester) =>
    tester.widget<TextField>(find.byType(TextField).at(2)).enabled ?? true;

/// Drive the form to the review screen with the given proposal (the default fake
/// recipient classifies shielded-valid, so Review is enabled). Shared by the #226
/// money-safety review tests.
Future<WalletLocalizations> _toReview(
  WidgetTester tester,
  FakeWalletSession fake, {
  ThemeData? theme,
  TextScaler? textScaler,
  String address = 'u1recipient',
}) async {
  await tester.pumpWidget(
    _harness(session: fake, theme: theme, textScaler: textScaler),
  );
  await tester.pumpAndSettle();
  final l10n = _l10n(tester);
  await _enterForm(tester, address: address);
  await tester.tap(find.text(l10n.walletSendReviewButton));
  await tester.pumpAndSettle();
  return l10n;
}

void main() {
  group('#401 R3a — the payment-entry fields announce name AND state', () {
    // The device walk reported all three fields unnamed. That was an ARTIFACT
    // of `uiautomator dump`, which emits `text` / `content-desc` and no hint at all,
    // while Flutter routes a text field's label to the node's hint. Measured against
    // the real semantics tree afterwards: `labelText` IS the field node's label, and
    // an ancestor `Semantics(label:)` is PREPENDED to it. These two tests pin what
    // actually matters — that the name keeps arriving (a refactor to a bare
    // `hintText` would drop it), and that a DISABLED field says why on the field
    // itself rather than only in a sibling line.
    testWidgets('all three fields carry their label on the field node itself', (
      tester,
    ) async {
      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      final handle = tester.ensureSemantics();

      for (final (index, label) in [
        (0, l10n.walletSendRecipientLabel),
        (1, l10n.walletSendAmountLabel),
        (2, l10n.walletSendMemoLabel),
      ]) {
        expect(
          tester.getSemantics(find.byType(TextField).at(index)).label,
          contains(label),
          reason: 'field $index must announce its own name',
        );
      }
      handle.dispose();
    });

    testWidgets('a memo field disabled by a TRANSPARENT recipient announces the '
        'reason on the field, not only in a sibling line', (tester) async {
      // Flutter's native disabled flag is silent on TalkBack/VoiceOver, so without
      // the ancestor label the user meets a field that refuses input for no stated
      // reason — the same argument that put the locked note on the recipient field.
      // This one is the genuine gap #401 R3a closed; the field NAME was never
      // missing.
      final session = _funded()
        ..validateRecipientResult = const ValidatedAddress(memoCapable: false);
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 't1transparent');
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(_memoEnabled(tester), isFalse, reason: 'the gate must be closed');
      final handle = tester.ensureSemantics();

      final label = tester.getSemantics(find.byType(TextField).at(2)).label;
      expect(label, contains(l10n.walletSendMemoLabel));
      expect(label, contains(l10n.walletSendMemoTransparentDisabled));

      // …and EXACTLY ONCE (#403 R8b). The visible sibling line carries the same
      // sentence, so before it was excluded a screen reader said the reason
      // twice while walking one control: once as part of the field's name, then
      // again as the next node. The line stays on screen for sighted users; only
      // its semantics are dropped.
      expect(
        find.descendant(
          of: find.byType(ExcludeSemantics),
          matching: find.text(l10n.walletSendMemoTransparentDisabled),
        ),
        findsOneWidget,
        reason: 'the visible copy is present but not announced a second time',
      );
      handle.dispose();
    });
  });

  testWidgets('form renders the available balance and the review action', (
    tester,
  ) async {
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // 500_000_000 zat = 5 ZEC; format through the same integer path the UI uses
    // so a future formatZec change can't make this assertion silently always-pass.
    expect(
      find.text(l10n.walletSendAvailable(formatZec(500000000))),
      findsOneWidget,
    );
    expect(find.text(l10n.walletSendReviewButton), findsOneWidget);
    // Online + synced: no offline queue affordance by default.
    expect(find.text(l10n.walletSendQueueButton), findsNothing);
    // And the synced form never carries the catch-up qualifier.
    expect(
      find.text(l10n.walletSendAvailableCatchingUp(formatZec(500000000))),
      findsNothing,
    );
  });

  testWidgets('the Available line is QUALIFIED while the wallet is catching '
      'up — a partial (or literal-zero) spendable must not read as final '
      '(#381 (a), the #380 swap-line rule)', (tester) async {
    // A rebuilt mid-catch-up wallet: zero spendable so the misread this pins
    // against is the worst one ("Available to send: 0 ZEC" as a verdict).
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(spendableZat: 0, totalZat: 0),
      ),
    );
    await tester.pumpWidget(
      _harness(
        session: fake,
        extraOverrides: [
          walletCatchUpCueProvider.overrideWithValue(
            const WalletCatchUpSyncing(),
          ),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(
      find.text(l10n.walletSendAvailableCatchingUp(formatZec(0))),
      findsOneWidget,
    );
    expect(find.text(l10n.walletSendAvailable(formatZec(0))), findsNothing);
  });

  testWidgets('S205-c: the catch-up qualifier DROPS under the host\'s '
      'sync-off policy — the Available line falls to the PLAIN figure ("still '
      'catching up" would claim the active progress the same form\'s fault '
      'arm just stopped claiming)', (tester) async {
    // The exact catching-up state the test above qualifies, differing only in
    // the sync-policy seam: the plain figure + the ambient sync-off story
    // (badge, disabled-Send reason, activity note) carry the caveat instead.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(spendableZat: 0, totalZat: 0),
      ),
    );
    await tester.pumpWidget(
      _harness(
        session: fake,
        extraOverrides: [
          walletCatchUpCueProvider.overrideWithValue(
            const WalletCatchUpSyncing(),
          ),
          walletSyncPolicyProvider.overrideWithValue(false),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(
      find.text(l10n.walletSendAvailable(formatZec(0))),
      findsOneWidget,
      reason: 'the plain figure — no active-progress claim while nothing runs',
    );
    expect(
      find.text(l10n.walletSendAvailableCatchingUp(formatZec(0))),
      findsNothing,
    );
  });

  testWidgets('the field hints are muted gray, not near-black input text', (
    tester,
  ) async {
    // The maintainer report: the placeholder read too dark on the light theme. Each
    // field's hint must use the MUTED token so it reads as guidance, not a value.
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    // Resolve the token from the live tree (theme-agnostic) so this can't pass
    // for the wrong reason if the harness theme ever changes.
    final muted = WalletColors.of(
      tester.element(find.byType(SendScreen)),
    ).textMuted;
    final fields = tester.widgetList<TextField>(find.byType(TextField));
    expect(fields, isNotEmpty);
    for (final field in fields) {
      expect(
        field.decoration?.hintStyle?.color,
        muted,
        reason: 'every send field hint is the muted token',
      );
    }
  });

  testWidgets('a bad amount surfaces an honest inline fault, not a code', (
    tester,
  ) async {
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // The field's formatter refuses a second separator outright (S13 M1), but
    // a lone separator passes it and is caught by the parser as not-a-number
    // (the parser is the real gate — the formatter is the input layer).
    await _enterForm(tester, amount: '.');
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendFaultAmountNotANumber), findsOneWidget);
  });

  testWidgets(
    'the inline fault is a live region — announced when it appears (#329-4)',
    (tester) async {
      // a11y: at large text scale the fault lands below the fold under a
      // disabled Review; a screen-reader user gets no visual "why", so it must
      // announce on appearance.
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await _enterForm(tester, amount: '1.2.3');
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(
        tester
            .getSemantics(find.byType(SendFormFaultView))
            .flagsCollection
            .isLiveRegion,
        isTrue,
        reason: 'a fixable send fault must be announced when it appears',
      );
      handle.dispose();
    },
  );

  testWidgets(
    'a fault that lands below the fold is scrolled into view (#329-2)',
    (tester) async {
      // A small phone viewport + large text scale pushes the fault (the foot of
      // the scrollable form) off-screen; it must be scrolled into view so the
      // user sees WHY Review is disabled.
      tester.view.physicalSize = const Size(320, 480);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      await tester.pumpWidget(
        _harness(session: _funded(), textScaler: const TextScaler.linear(2.0)),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await _enterForm(tester, amount: '1.2.3');
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      final faultFinder = find.byType(SendFormFaultView);
      expect(faultFinder, findsOneWidget);
      // The fault is on-screen (overlaps the visible Send surface) after settle.
      final faultRect = tester.getRect(faultFinder);
      final screenRect = tester.getRect(find.byType(SendScreen));
      expect(faultRect.top, lessThan(screenRect.bottom));
      expect(faultRect.bottom, greaterThan(screenRect.top));
      // And a scroll actually happened — the form list moved off its origin to
      // reveal the fault (proves the auto-scroll, not merely a short form that
      // never needed it).
      final position = tester
          .state<ScrollableState>(
            find.ancestor(of: faultFinder, matching: find.byType(Scrollable)),
          )
          .position;
      expect(position.pixels, greaterThan(0.0));
    },
  );

  testWidgets('a shielded review shows EXACT money and NO de-shield warning', (
    tester,
  ) async {
    final fake = _funded()
      ..proposeResult = sendProposalFixture(
        totalZat: 150500, // 0.001505 ZEC
        feeZat: 500, // 0.000005 ZEC
        changeZat: 9499500, // 0.094995 ZEC
        hasTransparentRecipient: false,
      );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendReviewTitle), findsOneWidget);
    // Integer-exact, no float drift.
    expect(find.text(l10n.walletAmount('0.001505')), findsOneWidget); // total
    expect(find.text(l10n.walletAmount('0.000005')), findsOneWidget); // fee
    expect(find.text(l10n.walletAmount('0.094995')), findsOneWidget); // change
    // A fully-shielded send must NOT show the public-payment warning.
    expect(find.text(l10n.walletSendDeshieldTitle), findsNothing);
    // …and the review STATES the privacy class explicitly (§3.2i-3 (c)).
    expect(find.text(l10n.walletSendPrivacyShielded), findsOneWidget);
    expect(find.text(l10n.walletSendPrivacyTransparent), findsNothing);
  });

  testWidgets('a transparent recipient surfaces the §5.1 de-shield warning', (
    tester,
  ) async {
    final fake = _funded()
      ..proposeResult = sendProposalFixture(
        totalZat: 100500,
        hasTransparentRecipient: true,
        steps: const [
          ProposalStep(
            recipients: [
              ProposalRecipient(
                pool: OutputPool.transparent,
                amountZat: 100000,
              ),
            ],
          ),
        ],
      );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendDeshieldTitle), findsOneWidget);
    expect(find.text(l10n.walletSendDeshieldBody), findsOneWidget);
    // …and the compact privacy statement names the PUBLIC class (§3.2i-3 (c)).
    expect(find.text(l10n.walletSendPrivacyTransparent), findsOneWidget);
    expect(find.text(l10n.walletSendPrivacyShielded), findsNothing);
  });

  testWidgets('a mixed-pool proposal (one shielded + one transparent) still surfaces '
      'the de-shield warning', (tester) async {
    // Realistic: a single step pays a shielded AND a transparent recipient. The
    // proposal carries hasTransparentRecipient = true, so the §5.1 warning MUST
    // show — a partly-public payment is still a public payment, and the privacy
    // disclosure must not require an ALL-transparent send to appear.
    final fake = _funded()
      ..proposeResult = sendProposalFixture(
        hasTransparentRecipient: true,
        steps: const [
          ProposalStep(
            recipients: [
              ProposalRecipient(pool: OutputPool.orchard, amountZat: 60000),
              ProposalRecipient(pool: OutputPool.transparent, amountZat: 40000),
            ],
          ),
        ],
      );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendDeshieldTitle), findsOneWidget);
    expect(find.text(l10n.walletSendDeshieldBody), findsOneWidget);
  });

  group(
    'S11 C2 — the de-shield warning is a CARD-notice (the review\'s M1)',
    () {
      // It is the only thing between the user and an irreversible public
      // payment (Send has no acknowledgement step), so it takes the card form
      // with its title in the warning colour — never the one-line notice.
      testWidgets('a transparent recipient: a titled warning card, the title '
          'in orange', (tester) async {
        final fake = _funded()
          ..proposeResult = sendProposalFixture(hasTransparentRecipient: true);
        final l10n = await _toReview(tester, fake);

        final title = find.text(l10n.walletSendDeshieldTitle);
        final notice = tester.widget<WalletNotice>(
          find.ancestor(of: title, matching: find.byType(WalletNotice)),
        );
        // Only `WalletNotice.card` carries a title; the line form has none.
        expect(notice.title, l10n.walletSendDeshieldTitle);
        expect(notice.message, l10n.walletSendDeshieldBody);
        expect(notice.tone, WalletNoticeTone.warning);
        expect(
          tester.widget<Text>(title).style?.color,
          WalletColors.light.orange,
        );
        // The notice sits inside the DeshieldWarning, and it keeps its one
        // merged screen-reader label.
        expect(
          find.ancestor(of: title, matching: find.byType(DeshieldWarning)),
          findsOneWidget,
        );
      });

      testWidgets('a shielded recipient: no de-shield warning at all', (
        tester,
      ) async {
        final fake = _funded()
          ..proposeResult = sendProposalFixture(hasTransparentRecipient: false);
        await _toReview(tester, fake);

        expect(find.byType(DeshieldWarning), findsNothing);
        expect(
          tester
              .widgetList<WalletNotice>(find.byType(WalletNotice))
              .where((n) => n.title != null),
          isEmpty,
          reason: 'no card-notice on a private payment',
        );
      });
    },
  );

  testWidgets('a successful send reaches the sent result', (tester) async {
    final fake = _funded()
      ..proposeResult = sendProposalFixture()
      ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendSentTitle), findsOneWidget);
  });

  testWidgets('"Send another" refreshes the available balance (no stale figure)', (
    tester,
  ) async {
    // After a send the spent notes are committed in the wallet DB. Tapping "Send
    // another" must re-read the cold snapshot so the available hint reflects the
    // LOWER post-send balance — composing a back-to-back send against the stale
    // pre-send figure would mislead the user (and the SDK would reject it).
    final fake =
        _funded() // 5 ZEC available
          ..proposeResult = sendProposalFixture()
          ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSendSentTitle), findsOneWidget);

    // The send committed: the wallet now reports a lower spendable balance.
    fake.setSnapshot(
      walletStateFixture(
        balance: balanceFixture(spendableZat: 100000000, totalZat: 100000000),
      ),
    );
    await tester.tap(find.text(l10n.walletSendAnother));
    await tester.pumpAndSettle();

    // The form's available hint reflects the fresh 1 ZEC, never the stale 5 ZEC.
    expect(
      find.text(l10n.walletSendAvailable(formatZec(100000000))),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletSendAvailable(formatZec(500000000))),
      findsNothing,
    );
  });

  testWidgets('a broadcast failure shows the honest saved-for-retry result', (
    tester,
  ) async {
    final fake = _funded()
      ..proposeResult = sendProposalFixture()
      ..sendResults = const [TxSubmitResult.grpcFailure(txidHex: 'aabb')];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();

    // Never "payment failed" — the money is saved and will retry.
    expect(find.text(l10n.walletSendSavedTitle), findsOneWidget);
    expect(find.text(l10n.walletSendSentTitle), findsNothing);
    // All txs failed (a single-tx proposal): the "nothing reached the network,
    // it's all saved" body, NOT the partial "part went out" body — the user
    // must not be told some money left when none did.
    expect(find.text(l10n.walletSendSavedBody), findsOneWidget);
    expect(find.text(l10n.walletSendPartialBody), findsNothing);
  });

  testWidgets('a partial pool-crossing broadcast shows the "part went out" body, '
      'not the all-saved body', (tester) async {
    // A pool-crossing send (§1.7) mints >1 tx. The unstable-network reality: one
    // lands, one doesn't. The result must be the DISTINCT partial body — "part of
    // your payment went out; the rest will complete on the next sync" — not the
    // all-failed "we couldn't reach the network" body. Conflating the two would
    // either understate (saved-body when money already moved) or overstate the
    // delivery. The summarizer's partial count is unit-tested; THIS pins that the
    // screen actually renders the partial branch (`broadcast > 0 && < total`).
    final fake = _funded()
      ..proposeResult = sendProposalFixture()
      ..sendResults = const [
        TxSubmitResult.success(txidHex: 'aa'),
        TxSubmitResult.grpcFailure(txidHex: 'bb'),
      ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendSavedTitle), findsOneWidget);
    expect(find.text(l10n.walletSendPartialBody), findsOneWidget);
    // And NOT the all-failed body (which would wrongly imply nothing left).
    expect(find.text(l10n.walletSendSavedBody), findsNothing);
    expect(find.text(l10n.walletSendSentTitle), findsNothing);
  });

  testWidgets('a partial two-step TEX send shows the honest IN-MOTION result, not the '
      'ordinary saved-for-retry copy', (tester) async {
    // A TEX (ZIP-320) recipient forces a two-step [tx0, tx1]: tx0 unshields to a
    // wallet-controlled one-time address, tx1 forwards to the recipient. The
    // unstable-network reality: tx0 lands, tx1 doesn't. The funds are IN MOTION on
    // the ephemeral, so the result must be the DISTINCT in-motion copy ("don't send
    // it again; recover from your wallet if it doesn't finish"), NEVER the ordinary
    // saved-for-retry "the rest will complete on the next sync" — that would
    // over-promise auto-completion the moment tx1 expires into a recoverable strand
    // (spec §3.2i-2 UX-honesty (i)). The proposal's isTwoStepTex SSOT flag is the
    // sole discriminator; the summarizer's routing is unit-tested, THIS pins that the
    // screen renders the in-motion branch end-to-end through the controller.
    final fake = _funded()
      ..proposeResult = sendProposalFixture(isTwoStepTex: true)
      ..sendResults = const [
        TxSubmitResult.success(txidHex: 'tx0'),
        TxSubmitResult.grpcFailure(txidHex: 'tx1'),
      ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendInMotionTitle), findsOneWidget);
    expect(find.text(l10n.walletSendInMotionBody), findsOneWidget);
    // NEVER the ordinary saved/partial copy (over-promises auto-completion), nor a
    // green "sent", nor a red "failed" — money is in motion + recoverable.
    expect(find.text(l10n.walletSendSavedTitle), findsNothing);
    expect(find.text(l10n.walletSendPartialBody), findsNothing);
    expect(find.text(l10n.walletSendSentTitle), findsNothing);
    expect(find.text(l10n.walletSendFailedTitle), findsNothing);
    // The body says "don't send it again" while funds are mid-flight, so the screen
    // must NOT offer "Send another" (it would contradict its own guidance and tempt
    // a re-pay). Only "Done" — the user starts a fresh send via Done → Send.
    expect(find.text(l10n.walletSendDone), findsOneWidget);
    expect(find.text(l10n.walletSendAnother), findsNothing);
    // And no "try again" (money DID move — the consumed token is not re-tappable).
    expect(find.text(l10n.walletSendTryAgain), findsNothing);
  });

  testWidgets(
    'a FULLY broadcast two-step TEX send is an ordinary success (no in-motion '
    'ambiguity when both legs land)',
    (tester) async {
      // Both tx0 and tx1 reached the network: nothing is mid-flight from the wallet's
      // standpoint, so the standard "broadcast to the network" success is honest — the
      // in-motion arm fires ONLY on a partial.
      final fake = _funded()
        ..proposeResult = sendProposalFixture(isTwoStepTex: true)
        ..sendResults = const [
          TxSubmitResult.success(txidHex: 'tx0'),
          TxSubmitResult.success(txidHex: 'tx1'),
        ];
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await _enterForm(tester);
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendSentTitle), findsOneWidget);
      expect(find.text(l10n.walletSendInMotionTitle), findsNothing);
    },
  );

  testWidgets('a proposal that went stale while backgrounded returns to the editable '
      'form with the amounts-expired fault', (tester) async {
    // The confirm screen sat backgrounded long enough that the anchor expired;
    // tapping Send throws ProposalStale. The user must land BACK ON THE FORM (to
    // re-propose for fresh numbers — never retry the consumed token) with the
    // honest AMOUNTS-EXPIRED message (the wallet IS synced — the numbers aged out;
    // NOT the propose-path "not synced"), NOT on a terminal result screen. The
    // controller test proves the routing; this pins the screen re-renders the form.
    final fake = _funded()
      ..proposeResult = sendProposalFixture()
      ..sendThrows = WalletApiError(
        code: 'RW',
        message: 's',
        kind: const WalletErrorKind.proposalStale(),
      );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();

    // Back on the form: the editable fields are present again (not a result),
    // the amounts-expired fault is shown, and no terminal "sent/failed" screen.
    expect(find.byType(TextField), findsNWidgets(3));
    expect(find.text(l10n.walletSendFaultAmountsExpired), findsOneWidget);
    expect(find.text(l10n.walletSendFaultNotSynced), findsNothing);
    expect(find.text(l10n.walletSendReviewButton), findsOneWidget);
    expect(find.text(l10n.walletSendSentTitle), findsNothing);
    expect(find.text(l10n.walletSendFailedTitle), findsNothing);
  });

  testWidgets('offline shows the queue-for-later affordance', (tester) async {
    await tester.pumpWidget(
      _harness(session: _funded(current: const SyncStatus.offline())),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSendQueueButton), findsOneWidget);
  });

  group('S301 — a public payment is acknowledged before it can go (founder: '
      '"add the Send acknowledgement like Swap")', () {
    const ackKey = ValueKey('send-public-ack');
    const queueAckKey = ValueKey('send-queue-public-ack');
    // The form's own list (its text fields are scrollables too).
    final formList = find
        .descendant(
          of: find.byType(ListView).first,
          matching: find.byType(Scrollable),
        )
        .first;
    // The form is a lazy list: scrolled down to the box, its first fields are
    // no longer built, so `TextField.at(0)` would name another field.
    Future<void> backToTop(WidgetTester tester) async {
      await tester.drag(formList, const Offset(0, 2000));
      await tester.pumpAndSettle();
    }

    // The review is a lazy list: Send now is built only once scrolled to.
    Future<FilledButton> sendNow(
      WidgetTester tester,
      WalletLocalizations l10n,
    ) async {
      await tester.scrollUntilVisible(
        find.text(l10n.walletSendConfirmButton),
        200,
      );
      await tester.pumpAndSettle();
      return tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletSendConfirmButton),
      );
    }

    OutlinedButton queue(WidgetTester tester, WalletLocalizations l10n) =>
        tester.widget<OutlinedButton>(
          find.widgetWithText(OutlinedButton, l10n.walletSendQueueButton),
        );

    FakeWalletSession publicProposal() => _funded()
      ..proposeResult = sendProposalFixture(hasTransparentRecipient: true)
      ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];

    testWidgets('the review: Send now is disabled until the box is ticked, '
        'then it sends', (tester) async {
      final fake = publicProposal();
      final l10n = await _toReview(tester, fake);

      expect(find.byKey(ackKey), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(ackKey),
          matching: find.text(l10n.walletSendPublicAckLabel),
        ),
        findsOneWidget,
      );
      expect((await sendNow(tester, l10n)).onPressed, isNull);

      await tester.ensureVisible(find.byKey(ackKey));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ackKey));
      await tester.pumpAndSettle();
      expect((await sendNow(tester, l10n)).onPressed, isNotNull);

      await tester.ensureVisible(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(fake.sendCount, 1);
    });

    testWidgets('a shielded payment carries no box and no extra tap', (
      tester,
    ) async {
      final fake = _funded()
        ..proposeResult = sendProposalFixture(hasTransparentRecipient: false);
      final l10n = await _toReview(tester, fake);

      expect(find.byKey(ackKey), findsNothing);
      expect((await sendNow(tester, l10n)).onPressed, isNotNull);
    });

    testWidgets('a new proposal starts unticked: Back, then Review again', (
      tester,
    ) async {
      final fake = publicProposal();
      final l10n = await _toReview(tester, fake);
      await tester.tap(find.byKey(ackKey));
      await tester.pumpAndSettle();
      expect((await sendNow(tester, l10n)).onPressed, isNotNull);

      // The review is a lazy list: Back is built only once scrolled to.
      await tester.scrollUntilVisible(
        find.text(l10n.walletSendBackButton),
        200,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendBackButton));
      await tester.pumpAndSettle();
      fake.proposeResult = sendProposalFixture(
        proposalId: 2,
        hasTransparentRecipient: true,
      );
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(find.byKey(ackKey), findsOneWidget);
      expect((await sendNow(tester, l10n)).onPressed, isNull);
    });

    testWidgets('Queue (which never shows the review) waits on the same box '
        'for a public recipient, and a recipient change withdraws it', (
      tester,
    ) async {
      final fake = _funded(current: const SyncStatus.offline())
        ..validateRecipientResult = const ValidatedAddress(memoCapable: false);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await _enterForm(tester, address: 't1transparent');

      // The Queue path shows the review's warning card too: a queued payment
      // never reaches the review.
      await tester.scrollUntilVisible(
        find.byKey(queueAckKey),
        200,
        scrollable: formList,
      );
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendDeshieldTitle), findsOneWidget);
      expect(queue(tester, l10n).onPressed, isNull);

      await tester.tap(find.byKey(queueAckKey));
      await tester.pump();
      expect(queue(tester, l10n).onPressed, isNotNull);

      await backToTop(tester);
      await tester.enterText(find.byType(TextField).at(0), 't1another');
      await tester.pump();
      expect(queue(tester, l10n).onPressed, isNull);
    });

    testWidgets('a new amount withdraws the Queue acknowledgement: it was '
        'given for one payment (S301 diff review)', (tester) async {
      final fake = _funded(current: const SyncStatus.offline())
        ..validateRecipientResult = const ValidatedAddress(memoCapable: false);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await _enterForm(tester, address: 't1transparent');
      await tester.scrollUntilVisible(
        find.byKey(queueAckKey),
        200,
        scrollable: formList,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(queueAckKey));
      await tester.pump();
      expect(queue(tester, l10n).onPressed, isNotNull);

      await backToTop(tester);
      await tester.enterText(find.byType(TextField).at(1), '2');
      await tester.pump();
      expect(queue(tester, l10n).onPressed, isNull);
    });

    testWidgets(
      'coming back to the form (Back from the review, "Send another") '
      'withdraws the Queue acknowledgement (S301 diff review)',
      (tester) async {
        final fake = _funded(current: const SyncStatus.offline())
          ..validateRecipientResult = const ValidatedAddress(memoCapable: false)
          ..proposeResult = sendProposalFixture(hasTransparentRecipient: true);
        await tester.pumpWidget(_harness(session: fake));
        await tester.pumpAndSettle();
        final l10n = _l10n(tester);
        await _enterForm(tester, address: 't1transparent');
        await tester.scrollUntilVisible(
          find.byKey(queueAckKey),
          200,
          scrollable: formList,
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(queueAckKey));
        await tester.pump();
        expect(queue(tester, l10n).onPressed, isNotNull);

        await tester.tap(find.text(l10n.walletSendReviewButton));
        await tester.pumpAndSettle();
        await tester.scrollUntilVisible(
          find.text(l10n.walletSendBackButton),
          200,
        );
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletSendBackButton));
        await tester.pumpAndSettle();

        expect(queue(tester, l10n).onPressed, isNull);
      },
    );

    testWidgets('Queue for a shielded recipient carries no box', (
      tester,
    ) async {
      final fake = _funded(current: const SyncStatus.offline())
        ..validateRecipientResult = const ValidatedAddress(memoCapable: true);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await _enterForm(tester);

      expect(find.byKey(queueAckKey), findsNothing);
      expect(queue(tester, l10n).onPressed, isNotNull);
    });
  });

  testWidgets('#399: CONNECTIVITY stalls offer the queue — the drain promise '
      '("a future online sync sends this") is true there', (tester) async {
    for (final reason in const [
      StallReason.endpointUnreachable,
      StallReason.torUnavailable,
    ]) {
      await tester.pumpWidget(
        _harness(
          session: _funded(current: SyncStatus.stalled(reason: reason)),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSendQueueButton),
        findsOneWidget,
        reason: '$reason is the connectivity family — queue offered',
      );
      // The hint is behind the Queue button's (i) since S13.
      expect(find.byKey(const ValueKey('send-queue-info')), findsOneWidget);
    }
  });

  testWidgets('#399: NON-connectivity stalls do NOT offer the queue — a '
      'storage-full/internal/reorg user is ONLINE, and the old any-Stalled '
      'gate told them "waiting for a connection" while their connection was '
      'fine (the S206-b money-path MED)', (tester) async {
    for (final reason in const [
      StallReason.storageFull,
      StallReason.internal,
      StallReason.chainReorg,
      StallReason.unknown,
    ]) {
      await tester.pumpWidget(
        _harness(
          session: _funded(current: SyncStatus.stalled(reason: reason)),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSendQueueButton),
        findsNothing,
        reason: '$reason must not offer the queue',
      );
      expect(
        find.text(l10n.walletSendQueueHint),
        findsNothing,
        reason: 'nor the hint that names it',
      );
    }
  });

  testWidgets(
    'flag-false + the not-synced fault: the NO-QUEUE copy renders and no '
    'queue action exists — copy must never invite a hidden affordance '
    '(S152 review H1/H4)',
    (tester) async {
      final fake = _funded()
        ..proposeThrows = WalletApiError(
          code: 'RW',
          message: 's',
          kind: const WalletErrorKind.proposalStale(),
        );
      await tester.pumpWidget(
        _harness(
          session: fake,
          extraOverrides: [
            walletOfflineQueueSupportedProvider.overrideWithValue(false),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await _enterForm(tester);
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendFaultNotSyncedNoQueue), findsOneWidget);
      expect(
        find.text(l10n.walletSendFaultNotSynced),
        findsNothing,
        reason: '"or queue this to send later" with no queue button is a lie',
      );
      expect(find.text(l10n.walletSendQueueButton), findsNothing);
    },
  );

  testWidgets(
    'flag-true (default) + the not-synced fault keeps the queue invitation '
    'copy AND the button it names',
    (tester) async {
      final fake = _funded()
        ..proposeThrows = WalletApiError(
          code: 'RW',
          message: 's',
          kind: const WalletErrorKind.proposalStale(),
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await _enterForm(tester);
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendFaultNotSynced), findsOneWidget);
      expect(
        find.text(l10n.walletSendQueueButton),
        findsOneWidget,
        reason: 'the copy offers the queue, so the button must be there',
      );
    },
  );

  testWidgets(
    'a host whose custody cannot sign at drain HIDES the queue affordance '
    'even offline (walletOfflineQueueSupportedProvider — #327)',
    (tester) async {
      // Per-send-credential custody can never sign a background drain: the
      // affordance must be ABSENT, not offered-then-faulted — the same offline
      // state as the test above, differing only in the capability seam.
      await tester.pumpWidget(
        _harness(
          session: _funded(current: const SyncStatus.offline()),
          extraOverrides: [
            walletOfflineQueueSupportedProvider.overrideWithValue(false),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expect(find.text(l10n.walletSendQueueButton), findsNothing);
      expect(find.text(l10n.walletSendQueueHint), findsNothing);
      // The interactive send path is untouched.
      expect(find.text(l10n.walletSendReviewButton), findsOneWidget);
    },
  );

  testWidgets(
    'S205-b: the host\'s sync policy OFF hides the queue affordance even '
    'OFFLINE — the queue drains only on sync passes, so a queued payment '
    'would sit behind a promise that cannot execute',
    (tester) async {
      // The exact offline state that normally OFFERS the queue, differing
      // only in the sync-policy seam — the R1 gate on showQueue.
      await tester.pumpWidget(
        _harness(
          session: _funded(current: const SyncStatus.offline()),
          extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expect(find.text(l10n.walletSendQueueButton), findsNothing);
      expect(find.text(l10n.walletSendQueueHint), findsNothing);
      // The interactive send path is untouched (hidden, not offered-then-
      // stranded — the same honesty rule as queueSupported).
      expect(find.text(l10n.walletSendReviewButton), findsOneWidget);
    },
  );

  testWidgets(
    'S205-b: policy-off + the not-synced fault renders the SYNC-OFF copy — '
    'never "wait for sync to catch up" (nothing is catching up) and never '
    'the queue invitation (the affordance is gated off by the same policy)',
    (tester) async {
      final fake = _funded()
        ..proposeThrows = WalletApiError(
          code: 'RW',
          message: 's',
          kind: const WalletErrorKind.proposalStale(),
        );
      await tester.pumpWidget(
        _harness(
          session: fake,
          extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await _enterForm(tester);
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(
        find.text(l10n.walletSendFaultNotSyncedSyncNotRunning),
        findsOneWidget,
        reason: 'the sync-off arm owns the not-synced copy under policy-off',
      );
      expect(find.text(l10n.walletSendFaultNotSynced), findsNothing);
      expect(find.text(l10n.walletSendFaultNotSyncedNoQueue), findsNothing);
      expect(find.text(l10n.walletSendQueueButton), findsNothing);
    },
  );

  testWidgets('FLAG_SECURE is requested on show and released on dispose', (
    tester,
  ) async {
    final security = FakeScreenSecurity();
    await tester.pumpWidget(_harness(session: _funded(), security: security));
    await tester.pumpAndSettle();
    expect(security.enableCount, 1);
    expect(security.disableCount, 0);

    // Replace the screen → dispose → protection released.
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
    expect(security.disableCount, 1);
  });

  testWidgets('no live wallet renders the honest unavailable state', (
    tester,
  ) async {
    await tester.pumpWidget(_harness(session: _funded(), nullSession: true));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSendUnavailable), findsOneWidget);
    // No form fields without a live wallet (money-safety).
    expect(find.byType(TextField), findsNothing);
  });

  testWidgets('#397 P1a — a WATCH-ONLY wallet renders the honest can\'t-send '
      'state (never a send form the SDK would refuse on propose)', (
    tester,
  ) async {
    // Reached only via a deep-link / prefilled entry (the wallet-screen chrome
    // hides Send). The screen re-gates on the watch-only kind, synchronously.
    await tester.pumpWidget(
      _harness(
        session: _funded(),
        extraOverrides: [isWatchOnlyProvider.overrideWithValue(true)],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSendWatchOnly), findsOneWidget);
    // No send form on a watch-only wallet (the money-safety gate).
    expect(find.byType(TextField), findsNothing);
  });

  // --- Live recipient validation + memo gating (slice 2) --------------------

  testWidgets(
    'a shielded recipient shows the private status, keeps the memo enabled, '
    'and enables Review',
    (tester) async {
      final fake = _funded()
        ..validateRecipientResult = const ValidatedAddress(memoCapable: true);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField).at(0), 'u1shielded');
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendRecipientShielded), findsOneWidget);
      expect(_memoEnabled(tester), isTrue);
      expect(find.text(l10n.walletSendMemoTransparentDisabled), findsNothing);
      expect(_reviewButton(tester, l10n).onPressed, isNotNull);
    },
  );

  testWidgets(
    'a transparent recipient warns public, disables + explains the memo, '
    'but still allows Review',
    (tester) async {
      final fake = _funded()
        ..validateRecipientResult = const ValidatedAddress(memoCapable: false);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField).at(0), 't1transparent');
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendRecipientTransparent), findsOneWidget);
      expect(_memoEnabled(tester), isFalse);
      expect(find.text(l10n.walletSendMemoTransparentDisabled), findsOneWidget);
      // Transparent is honest + public — warned (here + at Review), never blocked.
      expect(_reviewButton(tester, l10n).onPressed, isNotNull);
    },
  );

  testWidgets(
    'an invalid recipient shows the honest line and DISABLES Review',
    (tester) async {
      final fake = _funded()
        ..validateRecipientThrows = WalletApiError(
          code: 'RW',
          message: 's',
          kind: const WalletErrorKind.addressInvalid(),
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField).at(0), 'not-an-address');
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendRecipientInvalid), findsOneWidget);
      expect(
        _reviewButton(tester, l10n).onPressed,
        isNull,
        reason: 'foolproof: a send to a bad address is not offered',
      );
      // The memo stays neutral (enabled) — the user may still be drafting.
      expect(_memoEnabled(tester), isTrue);
    },
  );

  testWidgets(
    'a wrong-network recipient shows the DISTINCT line and disables Review',
    (tester) async {
      final fake = _funded()
        ..validateRecipientThrows = WalletApiError(
          code: 'RW',
          message: 's',
          kind: const WalletErrorKind.networkMismatch(),
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField).at(0), 'utestnetaddr');
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendRecipientWrongNetwork), findsOneWidget);
      // NOT conflated with a plain "invalid" — a different, renderable state.
      expect(find.text(l10n.walletSendRecipientInvalid), findsNothing);
      expect(_reviewButton(tester, l10n).onPressed, isNull);
    },
  );

  testWidgets(
    'the empty form reserves the status line and disables Review (no scary copy)',
    (tester) async {
      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      // The status region is ALWAYS present (reserved height ⇒ a later verdict
      // never shifts the fields below), but carries no warning on a fresh form.
      expect(find.byKey(const Key('send-recipient-status')), findsOneWidget);
      expect(find.text(l10n.walletSendRecipientInvalid), findsNothing);
      expect(find.text(l10n.walletSendRecipientShielded), findsNothing);
      // Nothing to send yet → Review disabled.
      expect(_reviewButton(tester, l10n).onPressed, isNull);
    },
  );

  testWidgets(
    'the recipient field caps at 512 chars — an over-long paste never crosses '
    'the bridge whole (gate 7: ADDRESS_MAX_BYTES boundary)',
    (tester) async {
      final fake = _funded();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();

      // A 513-char paste: the LengthLimitingTextInputFormatter truncates to 512
      // BEFORE onChanged fires, so the synchronous FFI never sees the 513th char.
      await tester.enterText(find.byType(TextField).at(0), 'x' * 513);
      await tester.pumpAndSettle();

      expect(fake.lastValidatedRecipient, isNotNull);
      expect(
        fake.lastValidatedRecipient!.length,
        lessThanOrEqualTo(512),
        reason:
            'the field caps the paste at ADDRESS_MAX_BYTES before the bridge',
      );
    },
  );

  testWidgets('offline + invalid recipient: the Queue action is ALSO disabled', (
    tester,
  ) async {
    // The offline queue affordance shares the recipient gate — queuing a send to
    // a bad address is never useful. (The Review gate is covered above; this pins
    // the parallel Queue gate on the offline branch.)
    final fake = _funded(current: const SyncStatus.offline())
      ..validateRecipientThrows = WalletApiError(
        code: 'RW',
        message: 's',
        kind: const WalletErrorKind.addressInvalid(),
      );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await tester.enterText(find.byType(TextField).at(0), 'garbage');
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendQueueButton), findsOneWidget);
    expect(
      tester
          .widget<OutlinedButton>(
            find.widgetWithText(OutlinedButton, l10n.walletSendQueueButton),
          )
          .onPressed,
      isNull,
      reason: 'queuing to an invalid address is never useful',
    );
  });

  testWidgets(
    'a memo typed before a transparent paste is DROPPED at compose, never sent',
    (tester) async {
      // Real-world edge: the user drafts a memo for a shielded recipient, then
      // pastes a TRANSPARENT address. The memo field disables; the stale memo must
      // NOT cross to the SDK (it would trip memoToTransparent) — dropped at compose
      // while preserved in the field (it returns if a shielded recipient is picked).
      final fake = _funded()
        ..validateRecipientResult = const ValidatedAddress(memoCapable: true)
        ..proposeResult = sendProposalFixture();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField).at(0), 'u1shielded');
      await tester.enterText(find.byType(TextField).at(2), 'thanks!');
      await tester.enterText(find.byType(TextField).at(1), '1');
      await tester.pumpAndSettle();

      // Now paste a transparent address — flip the live verdict, re-enter field 0.
      fake.validateRecipientResult = const ValidatedAddress(memoCapable: false);
      await tester.enterText(find.byType(TextField).at(0), 't1transparent');
      await tester.pumpAndSettle();

      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(fake.lastComposeRecipient, 't1transparent');
      expect(
        fake.lastComposeMemo,
        isNull,
        reason: 'the stale memo is dropped for a transparent recipient',
      );
    },
  );

  testWidgets(
    'a shielded recipient\'s memo actually REACHES compose (positive path)',
    (tester) async {
      // The counterpart to the drop test: prove the memo is FORWARDED for a
      // shielded recipient. Without this, a regression that dropped EVERY memo
      // (not just transparent ones) would pass the whole suite — a silent,
      // invisible data loss on a money screen.
      final fake = _funded()
        ..validateRecipientResult = const ValidatedAddress(memoCapable: true)
        ..proposeResult = sendProposalFixture();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField).at(0), 'u1shielded');
      await tester.enterText(find.byType(TextField).at(1), '1');
      await tester.enterText(find.byType(TextField).at(2), 'thanks!');
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(
        fake.lastComposeMemo,
        'thanks!',
        reason: 'a shielded recipient must receive the typed memo verbatim',
      );
    },
  );

  testWidgets('the memo recovers after shielded → transparent → shielded', (
    tester,
  ) async {
    // The documented `_memoToSend` promise: a memo drafted for a shielded
    // recipient survives a transparent detour (disabled + dropped) and returns
    // intact — and reaches compose — once a shielded recipient is picked again.
    final fake = _funded()
      ..validateRecipientResult = const ValidatedAddress(memoCapable: true)
      ..proposeResult = sendProposalFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await tester.enterText(find.byType(TextField).at(0), 'u1shielded');
    await tester.enterText(find.byType(TextField).at(2), 'thanks!');
    await tester.enterText(find.byType(TextField).at(1), '1');
    await tester.pumpAndSettle();

    // → transparent: the memo field disables and the WHY-note appears.
    fake.validateRecipientResult = const ValidatedAddress(memoCapable: false);
    await tester.enterText(find.byType(TextField).at(0), 't1transparent');
    await tester.pumpAndSettle();
    expect(_memoEnabled(tester), isFalse);
    expect(find.text(l10n.walletSendMemoTransparentDisabled), findsOneWidget);

    // → back to shielded: memo re-enabled, the note gone, the draft preserved.
    fake.validateRecipientResult = const ValidatedAddress(memoCapable: true);
    await tester.enterText(find.byType(TextField).at(0), 'u1shieldedagain');
    await tester.pumpAndSettle();
    expect(_memoEnabled(tester), isTrue);
    expect(find.text(l10n.walletSendMemoTransparentDisabled), findsNothing);

    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    expect(
      fake.lastComposeMemo,
      'thanks!',
      reason: 'the drafted memo recovers once the recipient is shielded again',
    );
  });

  testWidgets(
    'the live "Transparent · public" label and the Review de-shield warning '
    'agree for the SAME address (privacy honesty, invariant 5)',
    (tester) async {
      // The two privacy signals come from INDEPENDENT sources — the live label
      // from validateRecipient.memoCapable, the Review warning from the engine's
      // own hasTransparentRecipient. Pin that they fire TOGETHER for one address
      // (a disagreement would be a silent privacy bug).
      final fake = _funded()
        ..validateRecipientResult = const ValidatedAddress(memoCapable: false)
        ..proposeResult = sendProposalFixture(
          hasTransparentRecipient: true,
          steps: const [
            ProposalStep(
              recipients: [
                ProposalRecipient(
                  pool: OutputPool.transparent,
                  amountZat: 100000,
                ),
              ],
            ),
          ],
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField).at(0), 't1transparent');
      await tester.enterText(find.byType(TextField).at(1), '1');
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendRecipientTransparent), findsOneWidget);

      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSendDeshieldTitle),
        findsOneWidget,
        reason:
            'the binding de-shield disclosure fires for the same transparent address',
      );
    },
  );

  testWidgets('editing a valid recipient into garbage re-disables Review', (
    tester,
  ) async {
    // The live gate must re-evaluate on EVERY change, not only first entry: a
    // user who has a valid address and then breaks it must not keep Review
    // enabled (the foolproof gate is dynamic, not sticky).
    final fake = _funded();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await tester.enterText(find.byType(TextField).at(0), 'u1shielded');
    await tester.pumpAndSettle();
    expect(_reviewButton(tester, l10n).onPressed, isNotNull);

    fake.validateRecipientThrows = WalletApiError(
      code: 'RW',
      message: 's',
      kind: const WalletErrorKind.addressInvalid(),
    );
    await tester.enterText(find.byType(TextField).at(0), 'now-broken');
    await tester.pumpAndSettle();
    expect(_reviewButton(tester, l10n).onPressed, isNull);
    expect(find.text(l10n.walletSendRecipientInvalid), findsOneWidget);
  });

  testWidgets('a padded paste validates live AND composes the TRIMMED address '
      '(no "valid then rejected")', (tester) async {
    // Clipboard padding (leading/trailing spaces) is common on mobile. The live
    // verdict trims before validating, and the controller trims before compose,
    // so a padded valid address is Review-enabled AND reaches the SDK trimmed —
    // never a contradictory "live said valid, send then rejected it".
    final fake = _funded()
      ..validateRecipientResult = const ValidatedAddress(memoCapable: true)
      ..proposeResult = sendProposalFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    await tester.enterText(find.byType(TextField).at(0), '  u1shielded  ');
    await tester.enterText(find.byType(TextField).at(1), '1');
    await tester.pumpAndSettle();
    // live verdict validated the TRIMMED string
    expect(fake.lastValidatedRecipient, 'u1shielded');
    expect(_reviewButton(tester, l10n).onPressed, isNotNull);

    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    // compose received the TRIMMED address, not the padded one
    expect(fake.lastComposeRecipient, 'u1shielded');
  });

  // --- Money-safety: large-amount confirm + self-send note + foolproof address
  //     review (#226) ---------------------------------------------------------

  testWidgets(
    'the review screen renders the recipient in monospace GROUPS (foolproof)',
    (tester) async {
      final fake = _funded()..proposeResult = sendProposalFixture();
      final l10n = await _toReview(tester, fake);
      expect(find.text(l10n.walletSendReviewTitle), findsOneWidget);

      // The default form recipient is 'u1recipient' → grouped in 4s for verification
      // via the shared AddressVerificationText brick (a Text, deliberately NOT
      // selectable so a copy can't capture the render-only spaces).
      expect(
        find.text(groupAddress('u1recipient')),
        findsOneWidget,
        reason: 'the address is shown in 4-char groups, loss-free',
      );
      // The full (ungrouped) address is the screen-reader label so it is announced
      // whole, not as a stutter of 4-char fragments.
      expect(
        find.byWidgetPredicate(
          (w) => w is Semantics && w.properties.label == 'u1recipient',
        ),
        findsOneWidget,
        reason:
            'the foolproof review exposes the full address to a screen reader',
      );
    },
  );

  testWidgets('a self-send proposal shows the passive self-send note', (
    tester,
  ) async {
    final fake = _funded()..proposeResult = sendProposalFixture(selfSend: true);
    final l10n = await _toReview(tester, fake);
    expect(find.text(l10n.walletSendSelfSendNote), findsOneWidget);
  });

  testWidgets('an ordinary recipient shows NO self-send note', (tester) async {
    final fake = _funded()
      ..proposeResult = sendProposalFixture(); // selfSend: false
    final l10n = await _toReview(tester, fake);
    expect(find.text(l10n.walletSendSelfSendNote), findsNothing);
  });

  testWidgets(
    'an ordinary send signs in ONE tap — no large-amount dialog (no fatigue)',
    (tester) async {
      final fake = _funded()
        ..proposeResult =
            sendProposalFixture() // largeSend: null
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
      final l10n = await _toReview(tester, fake);

      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();

      // No deliberate dialog interposed; the send went straight through.
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsNothing);
      expect(fake.sendCount, 1);
      expect(find.text(l10n.walletSendSentTitle), findsOneWidget);
    },
  );

  testWidgets(
    'a LARGE send interposes ONE deliberate confirm before any money moves',
    (tester) async {
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 150000000, // 1.5 ZEC
          largeSend: LargeSendReason.nearTotalBalance,
        )
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
      final l10n = await _toReview(tester, fake);

      // Tapping "Send now" opens the dialog — it does NOT sign yet.
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);
      expect(
        fake.sendCount,
        0,
        reason: 'no money moves until the deliberate confirm',
      );
      // The exact amount rides the irreversible action button (unmistakable).
      final action = l10n.walletSendLargeConfirmAction(
        l10n.walletAmount(formatZec(150000000)),
      );
      expect(find.text(action), findsOneWidget);

      // Confirming the dialog signs once and reaches the sent result.
      await tester.tap(find.text(action));
      await tester.pumpAndSettle();
      expect(fake.sendCount, 1);
      expect(find.text(l10n.walletSendSentTitle), findsOneWidget);
    },
  );

  testWidgets(
    'cancelling the large-send dialog stays on review, sends nothing',
    (tester) async {
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 150000000,
          largeSend: LargeSendReason.overAbsoluteThreshold,
        );
      final l10n = await _toReview(tester, fake);

      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendLargeConfirmCancel));
      await tester.pumpAndSettle();

      // Back on the review screen; nothing signed.
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsNothing);
      expect(find.text(l10n.walletSendReviewTitle), findsOneWidget);
      expect(fake.sendCount, 0);
      expect(find.text(l10n.walletSendSentTitle), findsNothing);
    },
  );

  testWidgets('DISMISSING the large-send dialog via the barrier sends nothing', (
    tester,
  ) async {
    // The OTHER cancel path: a tap OUTSIDE the dialog returns `null`, not `false`.
    // Both must early-return — a money guard that only handled the Cancel button
    // would let an accidental outside-tap (`null`) fall through to a send.
    final fake = _funded()
      ..proposeResult = sendProposalFixture(
        totalZat: 150000000,
        largeSend: LargeSendReason.both,
      );
    final l10n = await _toReview(tester, fake);

    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);
    // Tap the modal barrier (top-left, away from the centered dialog) → null.
    await tester.tapAt(const Offset(10, 10));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSendLargeConfirmTitle), findsNothing);
    expect(find.text(l10n.walletSendReviewTitle), findsOneWidget);
    expect(
      fake.sendCount,
      0,
      reason: 'a barrier dismiss is a cancel, not a send',
    );
  });

  group('S11 C3 — the adaptive large-send confirm never reads a dismissal as '
      'consent (a spend over the fake session, watched at the authorizer)', () {
    Future<(WalletLocalizations, _CountingAuthorizer, FakeWalletSession)>
    toLargeConfirm(WidgetTester tester, TargetPlatform platform) async {
      // A fresh tree per run (a ProviderScope's overrides cannot change).
      await tester.pumpWidget(const SizedBox());
      final authorizer = _CountingAuthorizer();
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 150000000,
          largeSend: LargeSendReason.both,
        )
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
      await tester.pumpWidget(
        _harness(
          session: fake,
          theme: lightTheme.copyWith(platform: platform),
          extraOverrides: [
            walletSendAuthorizerProvider.overrideWithValue(authorizer),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await _enterForm(tester);
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);
      return (l10n, authorizer, fake);
    }

    testWidgets('Android: a barrier tap and a back pop never reach the '
        'authorizer', (tester) async {
      var (l10n, authorizer, fake) = await toLargeConfirm(
        tester,
        TargetPlatform.android,
      );
      expect(find.byType(AlertDialog), findsOneWidget);
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsNothing);
      expect(authorizer.calls, 0, reason: 'a barrier tap is not consent');
      expect(fake.sendCount, 0);

      (l10n, authorizer, fake) = await toLargeConfirm(
        tester,
        TargetPlatform.android,
      );
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsNothing);
      expect(find.text(l10n.walletSendReviewTitle), findsOneWidget);
      expect(authorizer.calls, 0, reason: 'a back pop is not consent');
      expect(fake.sendCount, 0);
    });

    testWidgets('iOS: a Cupertino alert whose barrier is inert; Cancel never '
        'reaches the authorizer', (tester) async {
      final (l10n, authorizer, fake) = await toLargeConfirm(
        tester,
        TargetPlatform.iOS,
      );
      expect(find.byType(CupertinoAlertDialog), findsOneWidget);
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSendLargeConfirmTitle),
        findsOneWidget,
        reason: 'the iOS barrier is inert',
      );
      await tester.tap(find.text(l10n.walletSendLargeConfirmCancel));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendReviewTitle), findsOneWidget);
      expect(authorizer.calls, 0);
      expect(fake.sendCount, 0);
    });

    testWidgets('the confirm tap DOES reach the authorizer, once (the row '
        'above is not vacuous)', (tester) async {
      for (final platform in [TargetPlatform.android, TargetPlatform.iOS]) {
        final (l10n, authorizer, fake) = await toLargeConfirm(tester, platform);
        await tester.tap(
          find.text(
            l10n.walletSendLargeConfirmAction(
              l10n.walletAmount(formatZec(150000000)),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(authorizer.calls, 1, reason: '$platform');
        expect(fake.sendCount, 1, reason: '$platform');
      }
    });
  });

  // The reason → body-copy mapping, ONE isolated test per variant so a failure
  // names the offending reason. `unknown` falls through to the strongest "both"
  // copy (fail safe toward more caution).
  for (final (reason, label, expected)
      in <(LargeSendReason, String, String Function(WalletLocalizations))>[
        (
          LargeSendReason.nearTotalBalance,
          'nearTotalBalance',
          (l) => l.walletSendLargeConfirmNearTotal,
        ),
        (
          LargeSendReason.overAbsoluteThreshold,
          'overAbsoluteThreshold',
          (l) => l.walletSendLargeConfirmOverThreshold,
        ),
        (LargeSendReason.both, 'both', (l) => l.walletSendLargeConfirmBoth),
        (
          LargeSendReason.unknown,
          'unknown→both fallthrough',
          (l) => l.walletSendLargeConfirmBoth,
        ),
      ]) {
    testWidgets('the large-send dialog body is keyed off the reason — $label', (
      tester,
    ) async {
      final fake = _funded()
        ..proposeResult = sendProposalFixture(largeSend: reason);
      final l10n = await _toReview(tester, fake);
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(expected(l10n)), findsOneWidget);
    });
  }

  // --- Operational real-world edges (#226 second-round) ----------------------

  String largeAction(WalletLocalizations l10n, int zat) =>
      l10n.walletSendLargeConfirmAction(l10n.walletAmount(formatZec(zat)));

  testWidgets(
    'a stale anchor thrown AFTER the large-send dialog is approved does NOT '
    'bypass the SDK gate — lands on the form, amounts-expired, money unmoved',
    (tester) async {
      // The dialog makes the user deliberate, WIDENING the TTL window. If the
      // anchor expires mid-dialog, approving must not slip a stale send through:
      // confirm() → ProposalStale → back to the form with the honest amounts-
      // expired fault, no success. The single most important combined money path.
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 150000000,
          largeSend: LargeSendReason.nearTotalBalance,
        )
        ..sendThrows = WalletApiError(
          code: 'RW',
          message: 's',
          kind: const WalletErrorKind.proposalStale(),
        );
      final l10n = await _toReview(tester, fake);

      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(largeAction(l10n, 150000000)));
      await tester.pumpAndSettle();

      expect(
        find.byType(TextField),
        findsNWidgets(3),
      ); // back on the editable form
      expect(find.text(l10n.walletSendFaultAmountsExpired), findsOneWidget);
      expect(find.text(l10n.walletSendSentTitle), findsNothing);
      expect(
        fake.sendCount,
        1,
        reason:
            'the send was attempted and the SDK gate rejected it — not skipped',
      );
    },
  );

  testWidgets(
    'a LARGE + TRANSPARENT send shows BOTH the de-shield warning and the '
    'large-amount confirm (neither suppresses the other)',
    (tester) async {
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 150000000,
          hasTransparentRecipient: true,
          largeSend: LargeSendReason.both,
          steps: const [
            ProposalStep(
              recipients: [
                ProposalRecipient(
                  pool: OutputPool.transparent,
                  amountZat: 149999500,
                ),
              ],
            ),
          ],
        )
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
      final l10n = await _toReview(tester, fake);

      // Review shows the privacy loss…
      expect(find.text(l10n.walletSendDeshieldTitle), findsOneWidget);
      // …asks for the public-payment acknowledgement first…
      await tester.tap(find.byKey(const ValueKey('send-public-ack')));
      await tester.pumpAndSettle();
      // …and confirming still gates on the large-amount dialog (money unmoved yet).
      await tester.scrollUntilVisible(
        find.text(l10n.walletSendConfirmButton),
        200,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);
      expect(fake.sendCount, 0);
      await tester.tap(find.text(largeAction(l10n, 150000000)));
      await tester.pumpAndSettle();
      expect(fake.sendCount, 1);
    },
  );

  testWidgets(
    'a LARGE self-send shows the self-send note AND still gates on the confirm '
    'dialog (neither passive cue suppresses the deliberate gate)',
    (tester) async {
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 150000000,
          selfSend: true,
          largeSend: LargeSendReason.nearTotalBalance,
        )
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
      final l10n = await _toReview(tester, fake);

      expect(find.text(l10n.walletSendSelfSendNote), findsOneWidget);
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);
    },
  );

  testWidgets(
    'cancelling the large-send dialog leaves the flow COMPLETABLE — re-tapping '
    'Send sends exactly once (a cancel never consumes the token / wedges it)',
    (tester) async {
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 150000000,
          largeSend: LargeSendReason.both,
        )
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
      final l10n = await _toReview(tester, fake);

      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSendLargeConfirmCancel));
      await tester.pumpAndSettle();
      expect(fake.sendCount, 0);

      // Re-tap Send → the dialog re-appears → approve → sends exactly once.
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);
      await tester.tap(find.text(largeAction(l10n, 150000000)));
      await tester.pumpAndSettle();
      expect(fake.sendCount, 1);
      expect(find.text(l10n.walletSendSentTitle), findsOneWidget);
    },
  );

  testWidgets(
    'the irreversible amount renders without overflow at a large text scale',
    (tester) async {
      // A11y + money clarity: the figure that AUTHORIZES the spend must render at a
      // large text scale without a RenderFlex crash. A tall viewport keeps the
      // review's Send button reachable (a lazy ListView won't build it off-screen).
      tester.view.physicalSize = const Size(1200, 3600);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      const awkward = 99999999; // 0.99999999 ZEC — a deliberately wide string
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: awkward,
          largeSend: LargeSendReason.overAbsoluteThreshold,
        );
      final l10n = await _toReview(
        tester,
        fake,
        textScaler: const TextScaler.linear(2.5),
      );

      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      // The FULL action string (incl. the exact amount) is present and un-clipped.
      expect(find.text(largeAction(l10n, awkward)), findsOneWidget);
      expect(
        tester.takeException(),
        isNull,
        reason: 'no overflow at 2.5x scale',
      );
    },
  );

  testWidgets(
    'a long (78-char) recipient renders chunked on a NARROW screen without '
    'overflow',
    (tester) async {
      // Mobile reality: a full unified address is the widest thing on the review
      // card. On a narrow phone it must wrap (the chunked groups reflow), never
      // throw a RenderFlex overflow on the money screen.
      tester.view.physicalSize = const Size(320, 1400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      const longUa =
          'u1qsy8h0c2f3k4m5n6p7r8s9t0v1w2x3y4z5a6b7c8d9e0f1g2h3j4k5l6m7n8p9q0r1s2t3u4v5';
      final fake = _funded()..proposeResult = sendProposalFixture();
      await _toReview(tester, fake, address: longUa);

      expect(find.text(groupAddress(longUa)), findsOneWidget);
      expect(
        tester.takeException(),
        isNull,
        reason: 'the long address wraps, never overflows',
      );
    },
  );

  testWidgets(
    'the review money rows (total / fee / change) render without overflow at '
    '1.4x scale AND 320px width COMBINED',
    (tester) async {
      // The tests above pin 2.5x-on-wide and 320px-at-1.0x separately; a narrow
      // phone WITH large accessibility text is the realistic worst case for the
      // fixed label+amount money rows. A tall viewport keeps the form's Review
      // button reachable at the larger scale.
      tester.view.physicalSize = const Size(320, 1400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      // The squeeze under test is the LABELS (the row protects the money figure
      // by design, so the amounts stay realistic-width — it is the label that
      // must flex + ellipsize instead of overflowing).
      final fake = _funded()
        ..proposeResult = sendProposalFixture(
          totalZat: 99990000, // 0.9999 ZEC
          feeZat: 10000, // 0.0001 ZEC
          changeZat: 9995000, // 0.09995 ZEC — nonzero so the change row renders
        );
      final l10n = await _toReview(
        tester,
        fake,
        textScaler: const TextScaler.linear(1.4),
      );

      // All three money rows are present (change renders because changeZat > 0)…
      expect(find.text(l10n.walletSendTotalLabel), findsOneWidget);
      expect(find.text(l10n.walletSendFeeLabel), findsOneWidget);
      expect(find.text(l10n.walletSendChangeLabel), findsOneWidget);
      // …and none of them overflowed (the Flexible labels absorb the squeeze).
      expect(
        tester.takeException(),
        isNull,
        reason: 'no overflow at 1.4x scale on a 320px width',
      );
    },
  );

  testWidgets('the new money-safety surfaces render under the DARK theme', (
    tester,
  ) async {
    // Project rule: support BOTH themes. A semantic token that resolved only in
    // light would crash or vanish in dark — pin the self-send note, the chunked
    // recipient, and the large-send dialog all render under darkTheme.
    final fake = _funded()
      ..proposeResult = sendProposalFixture(
        totalZat: 150000000,
        selfSend: true,
        largeSend: LargeSendReason.both,
      );
    final l10n = await _toReview(tester, fake, theme: darkTheme);

    expect(find.text(l10n.walletSendSelfSendNote), findsOneWidget);
    expect(find.text(groupAddress('u1recipient')), findsOneWidget);
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a wallet-session flip CLEARS the typed form (S153 wrap review '
      '— duress hygiene: the OLD identity\'s draft must never render in the '
      'NEW identity\'s form)', (tester) async {
    final fakeA = _funded();
    final fakeB = _funded();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
        ],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const SendScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // The real identity's draft: recipient + amount typed.
    await tester.enterText(find.byType(TextField).at(0), 'u1realrecipient');
    await tester.enterText(find.byType(TextField).at(1), '0.5');
    await tester.pumpAndSettle();
    expect(find.text('u1realrecipient'), findsOneWidget);
    expect(find.text('0.5'), findsOneWidget);

    // The identity flips (decoy/duress) while the screen stays mounted.
    final container = ProviderScope.containerOf(
      tester.element(find.byType(SendScreen)),
      listen: false,
    );
    container.read(sessionSwitch.notifier).state = fakeB;
    await tester.pumpAndSettle();

    expect(
      find.text('u1realrecipient'),
      findsNothing,
      reason: 'the coercer must never see the real identity\'s recipient',
    );
    expect(find.text('0.5'), findsNothing, reason: '…nor its amount');
  });

  testWidgets(
    'the available line reserves EXACT space: the address field does not '
    'move when the figure lands — even at a 2.0× scale with a wide amount, '
    'and the amount renders in full, never truncated (#356/S174 MED)',
    (tester) async {
      tester.view.physicalSize = const Size(360 * 3, 800 * 3);
      tester.view.devicePixelRatio = 3.0;
      addTearDown(tester.view.reset);
      const zat = 12345678901; // 123.45678901 ZEC — the wide-figure case
      final session = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 1),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 1),
          balance: balanceFixture(spendableZat: zat, totalZat: zat),
        ),
      );
      await tester.pumpWidget(
        _harness(session: session, textScaler: const TextScaler.linear(2.0)),
      );
      // FIRST frame: the snapshot future hasn't resolved — the line is the
      // hidden placeholder, its space reserved.
      final before = tester.getRect(find.byType(TextField).at(0));

      await tester.pumpAndSettle(); // the figure lands
      final after = tester.getRect(find.byType(TextField).at(0));
      expect(
        after,
        before,
        reason:
            'the reserve must be exact — no field jump when the '
            'available figure appears',
      );
      // The full figure renders (FittedBox scales, never ellipsizes — the
      // #329 amounts rule).
      final l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSendAvailable(formatZec(zat))),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('the QUALIFIED available line is NOT crammed into the one-line '
      'FittedBox — the figure stays legible at AX scale mid-catch-up, and the '
      'text-scale actually takes effect (S191 UX HIGH regression pin)', (
    tester,
  ) async {
    // A rebuilt mid-catch-up wallet with a WIDE partial figure — the exact
    // trap: the qualifier makes the string 2-3x longer, so a one-line
    // FittedBox scaleDown would shrink the whole line (figure included) to an
    // unreadable few px AND — being width-bound — ignore the 2.0x scale.
    const zat = 12345678901; // 123.45678901 ZEC
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(spendableZat: zat, totalZat: zat),
      ),
    );
    await tester.pumpWidget(
      _harness(
        session: fake,
        textScaler: const TextScaler.linear(2.0),
        extraOverrides: [
          walletCatchUpCueProvider.overrideWithValue(
            const WalletCatchUpSyncing(),
          ),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    final qualified = l10n.walletSendAvailableCatchingUp(formatZec(zat));
    final textFinder = find.text(qualified);
    expect(textFinder, findsOneWidget);
    // The qualified line must NOT sit under a FittedBox (that is the shrink
    // trap this fix removes — it wraps like the move/swap lines instead).
    expect(
      find.ancestor(of: textFinder, matching: find.byType(FittedBox)),
      findsNothing,
      reason: 'a qualified line inside a FittedBox is the S191 legibility bug',
    );
    // And the text renders at the user's chosen scale (2.0x): the effective
    // font size is the theme size scaled, never shrunk width-bound.
    final widget = tester.widget<Text>(textFinder);
    final resolved =
        widget.style?.fontSize ??
        Theme.of(tester.element(textFinder)).textTheme.bodySmall!.fontSize!;
    final painter = TextPainter(
      text: TextSpan(text: qualified, style: widget.style),
      textScaler: const TextScaler.linear(2.0),
      textDirection: TextDirection.ltr,
    )..layout(maxWidth: tester.getSize(textFinder).width);
    // The rendered glyph height reflects the 2.0x scale (≈ 2x the unscaled
    // line) — proof the scale takes effect and nothing shrank it.
    expect(
      painter.preferredLineHeight,
      greaterThan(resolved * 1.5),
      reason:
          'the text-scale must actually enlarge the figure, not be '
          'cancelled by a width-bound FittedBox',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('S7 U1: a send whose answer is lost (the SDK throws after '
      'persisting) lands on "check Activity first" — never "nothing was '
      'sent", no Try again, no error text', (tester) async {
    // R13 §4.2: a host throw after a send that RETURNED shows the landed
    // outcome, so the lost answer here is the SDK's own post-persist throw.
    final fake = _funded()
      ..proposeResult = sendProposalFixture()
      ..sendThrows = WalletApiError(
        code: 'RW',
        message: 's',
        kind: const WalletErrorKind.storeCorrupt(),
      );
    await tester.pumpWidget(
      _harness(
        session: fake,
        extraOverrides: [
          walletSendAuthorizerProvider.overrideWithValue(
            const _RunsThenThrowsAuthorizer(),
          ),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    await _enterForm(tester);
    await tester.tap(find.text(l10n.walletSendReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSendConfirmButton));
    await tester.pumpAndSettle();

    expect(fake.sendCount, 1, reason: 'the spend really ran');
    expect(find.text(l10n.walletSendUnknownTitle), findsOneWidget);
    expect(find.text(l10n.walletSendUnknownBody), findsOneWidget);
    expect(find.text(l10n.walletSendFailedTitle), findsNothing);
    expect(find.text(l10n.walletSendFailedBody), findsNothing);
    expect(find.text(l10n.walletSendTryAgain), findsNothing);
    expect(find.text(l10n.walletSendConfirmButton), findsNothing);
    expect(find.textContaining('bookkeeping'), findsNothing);
    expect(find.text(l10n.walletSendDone), findsOneWidget);
  });
}

/// Runs the spend, then throws — a host's own bookkeeping failing after the
/// money moved (S7 U1).
class _RunsThenThrowsAuthorizer implements WalletSendAuthorizer {
  const _RunsThenThrowsAuthorizer();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    await action();
    throw StateError('host bookkeeping failed after the spend');
  }
}

/// Counts every spend the package brings to the host's authorization seam,
/// then runs it (the pass-through shape with a counter).
class _CountingAuthorizer implements WalletSendAuthorizer {
  int calls = 0;

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) {
    calls++;
    return action();
  }
}
