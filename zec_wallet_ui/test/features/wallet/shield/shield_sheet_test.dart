import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../../support/a11y_activation.dart';

/// Shield-sheet widget tests (Recv-3). Money display (gross/fee/net) and the
/// privacy-positive framing are the load-bearing assertions: a transposed figure
/// or a spurious de-shield warning would mislead a user about money or privacy,
/// so both are pinned over the real widget tree (with a fake session behind the
/// port). The confirm sheet drives the real `ShieldController`.
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(ShieldSheet)));

Widget _harness(FakeWalletSession session) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const Scaffold(body: ShieldSheet()),
    ),
  );
}

void main() {
  testWidgets(
    'over threshold: shows the EXACT gross/fee/net + the privacy note',
    (tester) async {
      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(
          totalZat: 500000,
          feeZat: 15000,
        ); // net 485000
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle(); // the post-frame prepare runs

      final l10n = _l10n(tester);
      // MONEY-CORRECTNESS: gross, fee, and net each render exactly (net = gross − fee).
      expect(
        find.text(l10n.walletAmount(formatZec(500000))),
        findsOneWidget,
        reason: 'gross transparent being shielded',
      );
      expect(
        find.text(l10n.walletAmount(formatZec(15000))),
        findsOneWidget,
        reason: 'the ZIP-317 fee',
      );
      expect(
        find.text(l10n.walletAmount(formatZec(485000))),
        findsOneWidget,
        reason: 'net that lands shielded = gross − fee',
      );
      // PRIVACY: the positive "moving into your private balance" note, never a
      // de-shield warning (a shield is privacy-positive).
      expect(find.text(l10n.walletShieldNote), findsOneWidget);
      expect(find.text(l10n.walletShieldConfirmButton), findsOneWidget);
      expect(session.proposeShieldCount, 1);
    },
  );

  testWidgets('confirm shields via the SAME send token, then shows done', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..proposeShieldResult = shieldProposalFixture(proposalId: 42)
      ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
    await tester.pumpWidget(_harness(session));
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    await tester.tap(find.text(l10n.walletShieldConfirmButton));
    await tester.pumpAndSettle();

    expect(session.sendCount, 1, reason: 'the shield reuses the send pipeline');
    expect(
      session.lastSendProposalId,
      42,
      reason: 'the SAME one-shot token the proposal carried',
    );
    expect(find.text(l10n.walletShieldDoneTitle), findsOneWidget);
  });

  // The row above drives Confirm with `tester.tap(find.text(…))` — a POINTER
  // event that hits the live FilledButton and never consults the semantics
  // tree. It was green while the inner `ExcludeSemantics` deleted that button's
  // node outright, leaving a screen reader with a "Shield now, button" whose
  // double-tap invoked nothing. Contract: `test/support/a11y_activation.dart`.
  testWidgets(
    'the shield confirm is ACTIVATABLE by a screen reader, and the semantics '
    'action submits the SAME send token',
    (tester) async {
      final handle = tester.ensureSemantics();
      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 42)
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expectActivatable(
        tester,
        find.semantics.byLabel(l10n.walletShieldConfirmButton),
        reason: 'the shield confirm is a money terminal',
      );
      await tester.pumpAndSettle();

      expect(session.sendCount, 1, reason: 'the action is wired to onConfirm');
      expect(session.lastSendProposalId, 42);
      expect(find.text(l10n.walletShieldDoneTitle), findsOneWidget);
      handle.dispose();
    },
  );

  testWidgets(
    'below threshold: honest "nothing to shield", no confirm button',
    (tester) async {
      final session = FakeWalletSession()..proposeShieldResult = null;
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      expect(find.text(l10n.walletShieldNothingTitle), findsOneWidget);
      expect(find.text(l10n.walletShieldConfirmButton), findsNothing);
      expect(session.proposeShieldCount, 1);
      expect(session.sendCount, 0);
    },
  );

  testWidgets('a stale wallet: honest failure with a Try again', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..proposeShieldThrows = WalletApiError(
        code: 'RW',
        message: 's',
        kind: const WalletErrorKind.proposalStale(),
      );
    await tester.pumpWidget(_harness(session));
    await tester.pumpAndSettle();

    final l10n = _l10n(tester);
    expect(find.text(l10n.walletShieldFailedTitle), findsOneWidget);
    expect(find.text(l10n.walletShieldStaleBody), findsOneWidget);
    // Try again re-proposes (no double-broadcast — nothing was sent).
    await tester.tap(find.text(l10n.walletShieldRetry));
    await tester.pumpAndSettle();
    expect(session.proposeShieldCount, 2);
    expect(session.sendCount, 0);
  });

  testWidgets(
    'a consumed token: honest "already submitted", never re-broadcast',
    (tester) async {
      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture()
        ..sendThrows = WalletApiError(
          code: 'RW',
          message: 's',
          kind: const WalletErrorKind.proposalAlreadyUsed(),
        );
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();

      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletShieldConfirmButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletShieldAlreadyTitle), findsOneWidget);
    },
  );

  testWidgets(
    'the confirm button is reachable by its semantic label (iOS merge)',
    (tester) async {
      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture();
      await tester.pumpWidget(_harness(session));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(
        find.bySemanticsLabel(l10n.walletShieldConfirmButton),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'a wedged proposeShield times out into an honest error, never spins forever',
    (tester) async {
      // The unstable-network / wedged-FFI edge: proposeShield is a LOCAL call, so a
      // hang is a wedged boundary, not a slow network. Without the controller timeout
      // the sheet would spin on "Preparing…" forever on a money surface. Mirrors the
      // receive-screen wedged-load timeout test.
      final session = FakeWalletSession()..proposeShieldNeverCompletes = true;
      await tester.pumpWidget(_harness(session));
      await tester
          .pump(); // the post-frame prepare() fires → ShieldPreparing, awaits the wedge
      final l10n = _l10n(tester);
      expect(
        find.text(l10n.walletShieldPreparing),
        findsOneWidget,
        reason: 'still preparing before the timeout (the FFI is wedged)',
      );

      // Advance past the honest-degradation timeout (reused from the receive flow).
      await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
      await tester.pump(); // rebuild into the error state
      expect(
        find.text(l10n.walletShieldFailedTitle),
        findsOneWidget,
        reason:
            'a wedged prepare becomes an honest, retryable error — not a perpetual spinner',
      );
      expect(find.text(l10n.walletShieldRetry), findsOneWidget);
      expect(session.proposeShieldCount, 1);
    },
  );

  testWidgets('a mid-sheet wallet-session flip SELF-HEALS — the sheet '
      're-proposes against the new session instead of spinning "Preparing…" '
      'forever (S153 review MAJOR-1)', (tester) async {
    // The controller resets to ShieldIdle on a flip but never re-proposes on
    // its own, and this sheet renders Idle as the SAME spinner as Preparing —
    // pre-an identity switch mid-sheet was an infinite spinner with no
    // retry (the move sheet had the listen self-heal; this pins shield's).
    final fakeA = FakeWalletSession()
      ..proposeShieldResult = shieldProposalFixture(totalZat: 500000);
    final fakeB = FakeWalletSession()
      ..proposeShieldResult = shieldProposalFixture(
        totalZat: 700000,
        feeZat: 15000,
      );
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
          home: const Scaffold(body: ShieldSheet()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    expect(
      find.text(l10n.walletAmount(formatZec(500000))),
      findsOneWidget,
      reason: 'session A\'s review is up',
    );

    // The identity flips mid-sheet.
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ShieldSheet)),
      listen: false,
    );
    container.read(sessionSwitch.notifier).state = fakeB;
    await tester.pumpAndSettle();

    expect(
      find.text(l10n.walletAmount(formatZec(700000))),
      findsOneWidget,
      reason: 'the sheet re-proposed against the NEW session',
    );
    expect(
      find.text(l10n.walletShieldPreparing),
      findsNothing,
      reason: 'never a stuck spinner',
    );
    expect(fakeB.proposeShieldCount, 1);
    expect(fakeA.proposeShieldCount, 1);
  });

  testWidgets(
    'the wallet-ended fault carries an actionable BODY, never a title-only '
    'terminal (S153 wrap review NIT-2, fixed S154)',
    (tester) async {
      // Reachable via the flip-to-null self-heal: prepare() with no session →
      // ShieldUnavailable(walletUnavailable). The user must be told what to DO,
      // not just "Couldn't shield" (the move sheet's walletMoveWalletEnded is
      // the copy precedent).
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(null)],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: ShieldSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletShieldFailedTitle), findsOneWidget);
      expect(
        find.text(l10n.walletShieldWalletEnded),
        findsOneWidget,
        reason: 'the body says the session ended and what to do next',
      );
    },
  );

  testWidgets(
    '#403 R8c: the SAVED-FOR-RETRY sheet survives 320x640 at 2x scale — the '
    'sync-paused qualifier must not be the sacrificial tail',
    (tester) async {
      // This sheet shipped with NO SingleChildScrollView while its
      // move-to-transparent sibling had one, and #401 R1b then APPENDED the
      // sync-paused qualifier to exactly this body. The added honesty is LAST,
      // so at large text scale it was the first thing clipped: a money surface
      // where the qualifying truth falls off the bottom is worse than one that
      // never carried it. A RenderFlex overflow throws during layout, which the
      // binding surfaces through takeException — so a clipped sheet FAILS here.
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      tester.view.devicePixelRatio = 1.0;
      tester.view.physicalSize = const Size(320, 640);

      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(
          totalZat: 500000,
          feeZat: 15000,
        )
        // An unbroadcast tx — the saved-for-retry arm.
        ..sendResults = const [TxSubmitResult.grpcFailure(txidHex: 'aa')];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(session),
            // No sync passes ⇒ the body carries the appended qualifier, which is
            // the longest form this sheet ever renders.
            walletSyncPolicyProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: MediaQuery(
              data: const MediaQueryData(textScaler: TextScaler.linear(2.0)),
              child: const Scaffold(body: ShieldSheet()),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      // The confirm button sits below the fold at this scale — which is the
      // point: it is reachable ONLY because the sheet scrolls.
      await tester.ensureVisible(find.text(l10n.walletShieldConfirmButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletShieldConfirmButton));
      await tester.pumpAndSettle();

      expect(
        tester.takeException(),
        isNull,
        reason: 'no overflow: the sheet scrolls instead of clipping',
      );
      expect(find.text(l10n.walletShieldSavedTitle), findsOneWidget);
      expect(
        find.byType(SingleChildScrollView),
        findsWidgets,
        reason: 'the structural fix, not just an accidentally-fitting layout',
      );
    },
  );
}
