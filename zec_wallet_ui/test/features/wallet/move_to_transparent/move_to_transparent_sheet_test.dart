import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/deshield_warning.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/transparent_funds_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../../support/a11y_activation.dart';

/// Move-to-transparent sheet widget tests (§3.2i-1). The LOAD-BEARING assertions
/// are privacy + money honesty: the §5.1 de-shield warning MUST appear on the
/// review (a missing one would hide that the funds go public), and the honest
/// "nothing to move" empty state must replace the form (never a dead-end). The
/// sheet drives the real `MoveToTransparentController` behind a fake session.
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(MoveToTransparentSheet)));

Widget _harness(FakeWalletSession session, {List<Override> extra = const []}) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session), ...extra],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const Scaffold(body: MoveToTransparentSheet()),
    ),
  );
}

/// The catch-up cue pinned ON — the sheet's #381 (b) arms key on this.
List<Override> get _catchingUp => [
  walletCatchUpCueProvider.overrideWithValue(const WalletCatchUpSyncing()),
];

/// SYNCED by default (up-to-date status) so the catch-up cue derives None —
/// the sheet's #381 (b) arms are pinned separately with the cue overridden ON.
FakeWalletSession _funded({int spendableZat = 1000000}) =>
    FakeWalletSession(current: const SyncStatus.upToDate(tip: 1))
      ..currentTransparentAddressResult = 't1myownTaddr'
      ..setSnapshot(
        walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 1),
          balance: balanceFixture(spendableZat: spendableZat),
        ),
      );

void main() {
  testWidgets(
    'amount entry: shows the OWN-address destination + amount + Review',
    (tester) async {
      await tester.pumpWidget(_harness(_funded()));
      await tester.pumpAndSettle(); // post-frame start() loads the address

      final l10n = _l10n(tester);
      expect(find.text(l10n.walletMoveSheetTitle), findsOneWidget);
      expect(find.text(l10n.walletMoveDestinationLabel), findsOneWidget);
      expect(find.byType(TextField), findsOneWidget); // the amount field
      expect(find.text(l10n.walletMoveReviewButton), findsOneWidget);
    },
  );

  testWidgets('zero shielded spendable → honest "nothing to move", no form', (
    tester,
  ) async {
    // The default fake snapshot is spendableZat: 0; SYNCED so the cue is None.
    final session = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
    );
    await tester.pumpWidget(_harness(session));
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    expect(find.text(l10n.walletMoveNothingTitle), findsOneWidget);
    expect(find.byType(TextField), findsNothing);
    expect(find.text(l10n.walletMoveReviewButton), findsNothing);
    // Synced: the confirm-attribution body is the honest one.
    expect(find.text(l10n.walletMoveNothingBody), findsOneWidget);
    expect(find.text(l10n.walletMoveNothingCatchingUpBody), findsNothing);
  });

  testWidgets('zero spendable MID-CATCH-UP → the catching-up body, never '
      '"Once funds confirm" — the missing funds are UNSCANNED, not '
      'unconfirmed (#381 (b), hardware-proven S190-b)', (tester) async {
    final session = FakeWalletSession(); // default snapshot: spendableZat 0
    await tester.pumpWidget(_harness(session, extra: _catchingUp));
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    expect(find.text(l10n.walletMoveNothingTitle), findsOneWidget);
    expect(find.text(l10n.walletMoveNothingCatchingUpBody), findsOneWidget);
    expect(
      find.text(l10n.walletMoveNothingBody),
      findsNothing,
      reason: 'the confirm attribution would misattribute the cause',
    );
  });

  testWidgets('the Available line is QUALIFIED while catching up — the '
      'partial figure must not read as final (#381 (b))', (tester) async {
    await tester.pumpWidget(
      _harness(_funded(spendableZat: 1000000), extra: _catchingUp),
    );
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    expect(
      find.text(l10n.walletMoveAvailableCatchingUp(formatZec(1000000))),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletMoveAvailable(formatZec(1000000))),
      findsNothing,
    );
  });

  testWidgets('S205-c: zero spendable under the host\'s sync-off policy falls '
      'to the DEFAULT body — the catching-up explanation claims active '
      'progress, and the ambient sync-off story owns the why', (tester) async {
    // The exact mid-catch-up state the #381 (b) test above swaps the body
    // for, differing only in the sync-policy seam.
    final session = FakeWalletSession(); // default snapshot: spendableZat 0
    await tester.pumpWidget(
      _harness(
        session,
        extra: [
          ..._catchingUp,
          walletSyncPolicyProvider.overrideWithValue(false),
        ],
      ),
    );
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    expect(find.text(l10n.walletMoveNothingTitle), findsOneWidget);
    expect(
      find.text(l10n.walletMoveNothingBody),
      findsOneWidget,
      reason: 'the confirm framing is the lesser misread while nothing runs',
    );
    expect(find.text(l10n.walletMoveNothingCatchingUpBody), findsNothing);
  });

  testWidgets('S205-c: the Available qualifier drops under the host\'s '
      'sync-off policy — the plain figure renders; the badge story carries '
      'the caveat', (tester) async {
    await tester.pumpWidget(
      _harness(
        _funded(spendableZat: 1000000),
        extra: [
          ..._catchingUp,
          walletSyncPolicyProvider.overrideWithValue(false),
        ],
      ),
    );
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    expect(
      find.text(l10n.walletMoveAvailable(formatZec(1000000))),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletMoveAvailableCatchingUp(formatZec(1000000))),
      findsNothing,
      reason: 'no active-progress claim while the host holds sync off',
    );
  });

  testWidgets(
    'review ALWAYS shows the de-shield disclosure + own-address note (privacy)',
    (tester) async {
      final session = _funded()
        ..proposeResult = sendProposalFixture(
          proposalId: 5,
          hasTransparentRecipient: true,
        );
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), '0.002');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();

      // PRIVACY centerpiece: the §5.1 de-shield warning (shared widget, move copy)
      // + the move-specific own-address irreversibility note.
      expect(find.text(l10n.walletMoveDeshieldTitle), findsOneWidget);
      expect(find.text(l10n.walletMoveOwnAddressNote), findsOneWidget);
      expect(find.text(l10n.walletMoveConfirmButton), findsOneWidget);
      // security review MAJOR-3: with auto-shield effectively ON (this
      // harness leaves the flag at its default), the review MUST disclose that
      // the loop will shield these funds right back — else the deliberate
      // unshield is silently reverted with a second fee.
      expect(
        find.byKey(const ValueKey('move-auto-shield-note')),
        findsOneWidget,
      );

      // CORRECTNESS: composed to the OWN t-addr (SDK-supplied), no memo.
      expect(session.lastComposeRecipient, 't1myownTaddr');
      expect(session.lastComposeAmountZat, 200000);
      expect(session.lastComposeMemo, isNull);
    },
  );

  testWidgets(
    'with auto-shield deliberately OFF the review shows NO revert note '
    '(holding transparent is then the expert\'s explicit choice)',
    (tester) async {
      final session = _funded()..proposeResult = sendProposalFixture();
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(session),
            walletSettingsStoreProvider.overrideWithValue(
              FakeWalletSettingsStore(autoShieldValue: false),
            ),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: MoveToTransparentSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), '0.002');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletMoveConfirmButton), findsOneWidget);
      expect(find.byKey(const ValueKey('move-auto-shield-note')), findsNothing);
    },
  );

  testWidgets(
    'S205-b: an auto-shield-UNSUPPORTED host shows NO revert note even with '
    'the persisted switch at its default ON — the review gates on the '
    'EFFECTIVE provider, never the raw switch',
    (tester) async {
      // The persisted flag is left at its shipped default (ON): the raw-switch
      // gate would render the note, but on an unsupported host the loop never
      // arms — the note would assert an automation (and a second fee) that
      // structurally cannot happen, at a money confirm.
      final session = _funded()
        ..proposeResult = sendProposalFixture(hasTransparentRecipient: true);
      await tester.pumpWidget(
        _harness(
          session,
          extra: [walletAutoShieldSupportedProvider.overrideWithValue(false)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), '0.002');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();

      // The review renders (de-shield disclosure intact)…
      expect(find.text(l10n.walletMoveDeshieldTitle), findsOneWidget);
      expect(find.text(l10n.walletMoveConfirmButton), findsOneWidget);
      // …but the shielded-right-back claim is honestly absent.
      expect(find.byKey(const ValueKey('move-auto-shield-note')), findsNothing);
    },
  );

  testWidgets('confirm signs+broadcasts via the send token, then shows done', (
    tester,
  ) async {
    final session = _funded()
      ..proposeResult = sendProposalFixture(proposalId: 33)
      ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
    await tester.pumpWidget(_harness(session));
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    await tester.enterText(find.byType(TextField), '0.01');
    await tester.tap(find.text(l10n.walletMoveReviewButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletMoveConfirmButton));
    await tester.pumpAndSettle();

    expect(session.sendCount, 1);
    expect(
      session.lastSendProposalId,
      33,
      reason: 'the SAME one-shot token the proposal carried',
    );
    expect(find.text(l10n.walletMoveDoneTitle), findsOneWidget);
  });

  // The row above drives Confirm with `tester.tap(find.text(…))` — a POINTER
  // event that never consults the semantics tree. It was green while the inner
  // `ExcludeSemantics` (copied from the shield sheet, defect included) deleted
  // the button's node and its tap action, so a screen reader heard "Move to
  // transparent, button" and its double-tap invoked nothing.
  // Contract: `test/support/a11y_activation.dart`.
  testWidgets(
    'the move confirm is ACTIVATABLE by a screen reader, and the semantics '
    'action broadcasts via the SAME send token',
    (tester) async {
      final handle = tester.ensureSemantics();
      final session = _funded()
        ..proposeResult = sendProposalFixture(proposalId: 33)
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.enterText(find.byType(TextField), '0.01');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();

      expectActivatable(
        tester,
        find.semantics.byLabel(l10n.walletMoveConfirmButton),
        reason: 'a de-shielding money terminal',
      );
      await tester.pumpAndSettle();

      expect(session.sendCount, 1, reason: 'the action is wired to onConfirm');
      expect(session.lastSendProposalId, 33);
      expect(find.text(l10n.walletMoveDoneTitle), findsOneWidget);
      handle.dispose();
    },
  );

  testWidgets(
    'a partial broadcast shows the honest "saved" terminal, not "done"',
    (tester) async {
      final session = _funded()
        ..sendResults = const [TxSubmitResult.grpcFailure(txidHex: 'bb')];
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), '0.01');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletMoveConfirmButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletMoveSavedTitle), findsOneWidget);
      expect(find.text(l10n.walletMoveDoneTitle), findsNothing);
    },
  );

  testWidgets(
    'a sign failure shows "couldn\'t complete" + Try again re-enters the form',
    (tester) async {
      final session = _funded()
        ..sendThrows = WalletApiError(
          code: 'X',
          message: 's',
          kind: const WalletErrorKind.signFailed(),
        );
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), '0.01');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletMoveConfirmButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletMoveFailedTitle), findsOneWidget);
      // Try again re-runs start() → reloads the address → back to the amount form.
      await tester.tap(find.text(l10n.walletMoveRetry));
      await tester.pumpAndSettle();
      expect(find.byType(TextField), findsOneWidget);
    },
  );

  testWidgets(
    'an inline propose fault returns to the form with the honest message',
    (tester) async {
      final session = _funded()
        ..proposeThrows = WalletApiError(
          code: 'X',
          message: 's',
          kind: const WalletErrorKind.insufficientFunds(
            availableZat: 100,
            requiredZat: 500,
            pendingIncomingZat: 0,
          ),
        );
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), '0.01');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();

      // Back on the amount form (still has the field) with the shared fault view.
      expect(find.byType(TextField), findsOneWidget);
      expect(
        find.text(
          l10n.walletSendFaultInsufficient(formatZec(100), formatZec(500)),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'a large move requires the deliberate confirm; Cancel keeps funds put',
    (tester) async {
      final session = _funded()
        ..proposeResult = sendProposalFixture(
          proposalId: 8,
          hasTransparentRecipient: true,
          largeSend: LargeSendReason.overAbsoluteThreshold,
        );
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), '5');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();

      // Confirm fires the large-send dialog (money-safety parity with Send).
      await tester.tap(find.text(l10n.walletMoveConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSendLargeConfirmTitle), findsOneWidget);

      // Cancelling the dialog moves NO money and stays on the review.
      await tester.tap(find.text(l10n.walletSendLargeConfirmCancel));
      await tester.pumpAndSettle();
      expect(session.sendCount, 0);
      expect(find.text(l10n.walletMoveConfirmButton), findsOneWidget);

      // Re-confirming and approving the dialog proceeds to the send.
      await tester.tap(find.text(l10n.walletMoveConfirmButton));
      await tester.pumpAndSettle();
      await tester.tap(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.byType(FilledButton),
        ),
      );
      await tester.pumpAndSettle();
      expect(session.sendCount, 1);
    },
  );

  testWidgets('the de-shield warning uses MOVE copy, not the payment wording', (
    tester,
  ) async {
    final session = _funded()
      ..proposeResult = sendProposalFixture(
        proposalId: 6,
        hasTransparentRecipient: true,
      );
    await tester.pumpWidget(_harness(session));
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    await tester.enterText(find.byType(TextField), '0.002');
    await tester.tap(find.text(l10n.walletMoveReviewButton));
    await tester.pumpAndSettle();

    // The self-transfer ("move") wording, NOT the Send "payment / recipient" copy
    // (which would misread for a transfer to your own address).
    expect(find.text(l10n.walletMoveDeshieldTitle), findsOneWidget);
    expect(find.text(l10n.walletSendDeshieldTitle), findsNothing);
  });

  testWidgets('inside the real sheet, the de-shield warning card has a fill '
      'that differs from the sheet (S11 diff review M-1: no border, so the '
      'fill is its only edge)', (tester) async {
    final session = _funded()
      ..proposeResult = sendProposalFixture(
        proposalId: 7,
        hasTransparentRecipient: true,
      );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [walletSessionProvider.overrideWithValue(session)],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () => showMoveToTransparentSheet(context),
                child: const Text('open'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    await tester.enterText(find.byType(TextField), '0.002');
    await tester.tap(find.text(l10n.walletMoveReviewButton));
    await tester.pumpAndSettle();

    final warning = find.byType(DeshieldWarning);
    expect(warning, findsOneWidget);
    final cardFill =
        (tester
                    .widget<DecoratedBox>(
                      find
                          .descendant(
                            of: warning,
                            matching: find.byType(DecoratedBox),
                          )
                          .first,
                    )
                    .decoration
                as BoxDecoration)
            .color;
    // The surface the card sits on: the nearest Material above it.
    final sheetFill = tester
        .widget<Material>(
          find.ancestor(of: warning, matching: find.byType(Material)).first,
        )
        .color;
    expect(sheetFill, isNotNull);
    expect(cardFill, isNot(sheetFill));
  });

  testWidgets(
    'a session that ends mid-sheet shows an honest terminal, no retry',
    (tester) async {
      // Null session → start() → MoveUnavailable(walletUnavailable).
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(null)],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: MoveToTransparentSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      expect(find.text(l10n.walletMoveFailedTitle), findsOneWidget);
      // Honest body (the #251 no-silent-failure lesson), not a blank title-only card.
      expect(find.text(l10n.walletMoveWalletEnded), findsOneWidget);
      expect(find.text(l10n.walletMoveRetry), findsNothing); // nothing to retry
    },
  );

  testWidgets(
    'a wedged address fetch degrades honestly (not an endless spinner)',
    (tester) async {
      final session = FakeWalletSession()
        ..currentTransparentAddressNeverCompletes = true;
      await tester.pumpWidget(_harness(session));
      await tester.pump(); // kick off start()
      // Advance past the honest-degradation timeout.
      await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      expect(find.text(l10n.walletMoveCouldNotLoad), findsOneWidget);
      expect(find.text(l10n.walletMoveRetry), findsOneWidget);
    },
  );

  // S14 (plan `stage-14-move-below-the-shield-floor.md` §3): the review warns
  // when the move leaves the public balance under the 0.001 ZEC shield floor,
  // and then promises neither an automatic nor a later re-shield. Appended at
  // the end of the file so the registry's citations above stay put.
  group('S14 — a move that leaves public funds under the shield floor', () {
    const below = ValueKey('move-below-floor-note');
    const auto = ValueKey('move-auto-shield-note');

    /// A funded wallet already holding [transparentZat] public, of which
    /// [recoverableZat] sits on one-time addresses.
    FakeWalletSession publicFunded({
      int transparentZat = 0,
      int recoverableZat = 0,
    }) => FakeWalletSession(current: const SyncStatus.upToDate(tip: 1))
      ..currentTransparentAddressResult = 't1myownTaddr'
      ..recoverableEphemeralFundsResult = [
        if (recoverableZat > 0)
          RecoverableEphemeralFunds(
            recoverableZat: recoverableZat,
            isFinal: true,
          ),
      ]
      ..proposeResult = sendProposalFixture(hasTransparentRecipient: true)
      ..setSnapshot(
        walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 1),
          balance: balanceFixture(
            spendableZat: 1000000,
            transparentZat: transparentZat,
          ),
        ),
      );

    Future<WalletLocalizations> review(
      WidgetTester tester,
      FakeWalletSession session,
      String amount, {
      List<Override> extra = const [],
    }) async {
      await tester.pumpWidget(_harness(session, extra: extra));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await tester.enterText(find.byType(TextField), amount);
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletMoveConfirmButton), findsOneWidget);
      return l10n;
    }

    testWidgets('0.0005 onto nothing public: the founder\'s warning with the '
        'figure, the stays-public own-address note, and no re-shield promise '
        '(the S307 walk: auto-shield on, the funds stayed public)', (
      tester,
    ) async {
      final l10n = await review(tester, publicFunded(), '0.0005');

      expect(
        find.text(
          l10n.walletMoveBelowFloorNote(formatZec(50000), formatZec(100000)),
        ),
        findsOneWidget,
      );
      expect(find.byKey(below), findsOneWidget);
      expect(
        find.text(l10n.walletMoveOwnAddressNoteStaysPublic),
        findsOneWidget,
      );
      expect(find.text(l10n.walletMoveOwnAddressNote), findsNothing);
      expect(
        find.byKey(auto),
        findsNothing,
        reason: 'no Shield takes a public balance under the floor',
      );
    });

    testWidgets('the boundary, under: 99 999 zat public after the move warns', (
      tester,
    ) async {
      await review(tester, publicFunded(), '0.00099999');
      expect(find.byKey(below), findsOneWidget);
      expect(find.byKey(auto), findsNothing);
    });

    testWidgets('the boundary, at: 100 000 zat does not warn (the core shields '
        'a gross of at least the floor)', (tester) async {
      final l10n = await review(tester, publicFunded(), '0.001');
      expect(find.byKey(below), findsNothing);
      expect(find.text(l10n.walletMoveOwnAddressNote), findsOneWidget);
      expect(find.byKey(auto), findsOneWidget);
    });

    testWidgets('what is already public counts: 0.0006 public + 0.0005 moved '
        'clears the floor, so no warning and the auto-shield note is true', (
      tester,
    ) async {
      final l10n = await review(
        tester,
        publicFunded(transparentZat: 60000),
        '0.0005',
      );
      expect(find.byKey(below), findsNothing);
      expect(find.text(l10n.walletMoveOwnAddressNote), findsOneWidget);
      expect(find.byKey(auto), findsOneWidget);
    });

    testWidgets('funds on one-time addresses do not count: 0.0008 public, all '
        'of it recoverable, + 0.0005 still warns, with X = 0.0005', (
      tester,
    ) async {
      final l10n = await review(
        tester,
        publicFunded(transparentZat: 80000, recoverableZat: 80000),
        '0.0005',
      );
      expect(
        find.text(
          l10n.walletMoveBelowFloorNote(formatZec(50000), formatZec(100000)),
        ),
        findsOneWidget,
      );
      expect(find.byKey(auto), findsNothing);
    });

    testWidgets('over the floor but under a host\'s higher loop threshold: no '
        'warning, and no "shielded back automatically" (manual Shield can)', (
      tester,
    ) async {
      final l10n = await review(
        tester,
        publicFunded(),
        '0.0015',
        extra: [walletAutoShieldThresholdZatProvider.overrideWithValue(200000)],
      );
      expect(find.byKey(below), findsNothing);
      expect(find.text(l10n.walletMoveOwnAddressNote), findsOneWidget);
      expect(find.byKey(auto), findsNothing);
    });

    testWidgets('with auto-shield unsupported the warning still shows: it is '
        'about every Shield, not the loop', (tester) async {
      await review(
        tester,
        publicFunded(),
        '0.0005',
        extra: [walletAutoShieldSupportedProvider.overrideWithValue(false)],
      );
      expect(find.byKey(below), findsOneWidget);
      expect(find.byKey(auto), findsNothing);
    });

    testWidgets('over the floor with auto-shield unsupported: no warning and '
        'no "shielded back automatically" (§3 table, row 4)', (tester) async {
      final l10n = await review(
        tester,
        publicFunded(),
        '0.002',
        extra: [walletAutoShieldSupportedProvider.overrideWithValue(false)],
      );
      expect(find.byKey(below), findsNothing);
      expect(find.text(l10n.walletMoveOwnAddressNote), findsOneWidget);
      expect(find.byKey(auto), findsNothing);
    });

    testWidgets('the figures: X counts what is already public (0.0003 + '
        '0.0005 = 0.0008), and the floor stays 0.001 under a higher host '
        'threshold', (tester) async {
      final l10n = await review(
        tester,
        publicFunded(transparentZat: 30000),
        '0.0005',
        extra: [walletAutoShieldThresholdZatProvider.overrideWithValue(200000)],
      );
      expect(
        find.text(
          l10n.walletMoveBelowFloorNote(formatZec(80000), formatZec(100000)),
        ),
        findsOneWidget,
      );
    });

    testWidgets('a recoverable list that failed to read agrees with the loop: '
        'no one-time funds, so 0.0008 + 0.0005 clears the floor and the '
        'revert note shows (the loop will sweep)', (tester) async {
      final session = publicFunded(transparentZat: 80000)
        ..recoverableEphemeralFundsThrows = StateError('read failed');
      final l10n = await review(tester, session, '0.0005');
      expect(find.byKey(below), findsNothing);
      expect(find.text(l10n.walletMoveOwnAddressNote), findsOneWidget);
      expect(find.byKey(auto), findsOneWidget);
    });

    testWidgets('Back and a larger amount re-reviews on the NEW amount: the '
        'warning goes, the re-shield sentence returns', (tester) async {
      final l10n = await review(tester, publicFunded(), '0.0005');
      expect(find.byKey(below), findsOneWidget);

      await tester.tap(find.text(l10n.walletMoveBackButton));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '0.002');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();

      expect(find.byKey(below), findsNothing);
      expect(find.text(l10n.walletMoveOwnAddressNote), findsOneWidget);
      expect(find.byKey(auto), findsOneWidget);
    });
  });
}
