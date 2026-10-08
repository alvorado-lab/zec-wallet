import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart'
    show RescanAllHistory, RescanFromTime, RescanFromWalletBirthday;
import 'package:zec_wallet_ui/features/wallet/sync_status_presentation.dart'
    show WalletHostTransport, exactBlockCount;
import 'package:zec_wallet_ui/features/wallet/swap_deep_scan.dart'
    show swapDeepScanInFlightProvider, swapDeepScanProgressProvider;
import 'package:zec_wallet_ui/features/wallet/swap/swap_activation.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_config.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_sheet.dart'
    show SyncStatusSheet;
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_coin.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_sync_controller.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Resolve l10n from a live element so finders couple to KEYS, not English
/// literals (the app_frame_test idiom).
WalletLocalizations _l10nAt(WidgetTester tester, Finder anchor) =>
    WalletLocalizations.of(tester.element(anchor));

/// An OUTGOING activity amount as the row renders it (FR-49 S12, C5): the
/// SDK's figure with its ASCII hyphen replaced by the true minus sign U+2212,
/// in the localized unit. [netZat] is the (negative) net amount.
String _outgoingAmount(WalletLocalizations l10n, int netZat) {
  assert(netZat < 0, 'an outgoing amount is negative');
  return l10n.walletAmount('−${formatZec(-netZat)}');
}

/// A caution transport (the SDK fell back off its private path): the sync
/// bar SHOWS in every sync state, including a fresh UpToDate (FR-49 S12, C2),
/// so the badge-layout guarantees can be pinned in every state.
const _cautionTor = TorState.fellBack();

Widget _harness({
  WalletSession? session,
  ThemeData? theme,
  double? textScale,
  List<Override> extraOverrides = const [],
}) {
  return ProviderScope(
    overrides: [
      if (session != null) walletSessionProvider.overrideWithValue(session),
      ...extraOverrides,
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: theme ?? lightTheme,
      home: const WalletScreen(),
      // Force a fixed text scale (the dynamic-text / overflow test) by clamping
      // MediaQuery's scaler around the whole app.
      builder: textScale == null
          ? null
          : (context, child) => MediaQuery.withClampedTextScaling(
              minScaleFactor: textScale,
              maxScaleFactor: textScale,
              child: child!,
            ),
    ),
  );
}

void main() {
  testWidgets(
    'mounting the wallet screen ANCHORS the address views — both derives '
    'run exactly once, before Receive is ever opened (#386, E2E-2)',
    (tester) async {
      // The anchor in _WalletActive (the auto-shield precedent) keeps the two
      // receive-address views container-lifetime ACTIVE: the once-per-identity
      // derive starts at first Active render and can never be parked/paused
      // by leaving the Receive screen — the address is typically ready before
      // the user first opens Receive.
      final fake = FakeWalletSession();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      expect(
        fake.currentAddressCount,
        1,
        reason: 'the shielded derive is anchored at wallet-active render',
      );
      expect(
        fake.currentTransparentAddressCount,
        1,
        reason: 'the transparent derive is anchored at wallet-active render',
      );
      await tester.pumpAndSettle();
      expect(
        fake.currentAddressCount + fake.currentTransparentAddressCount,
        2,
        reason: 'once per identity — the anchor never churns re-derives',
      );
    },
  );

  testWidgets('no session: honest not-set-up state, NO balance or deposit', (
    tester,
  ) async {
    await tester.pumpWidget(_harness()); // null session = production default
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletNotSetUpTitle), findsOneWidget);
    expect(find.text(l10n.walletNotSetUpBody), findsOneWidget);
    // Money-safety: a not-backed-up wallet shows NO balance and never invites
    // a deposit.
    expect(find.text(l10n.walletBalanceLabel), findsNothing);
  });

  testWidgets('up-to-date wallet renders an EXACT balance (integer money)', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        tor: const TorState.active(runtime: TorRuntimeKind.dialer()),
        balance: balanceFixture(totalZat: 123450000, spendableZat: 123450000),
        lastSynced: const SyncStamp(height: 100, at: 0),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncUpToDate), findsOneWidget);
    // 123450000 zat = 1.2345 ZEC, exactly (no float drift).
    expect(find.text(l10n.walletAmount('1.2345')), findsWidgets);
    // The transport indicator is ICON-only on the badge row; its label
    // rides the badge's a11y string. The full label lives in the sheet.
    //
    // The fixture's runtime is an injected `Dialer` — a byte-stream dialer a
    // Rust host handed the SDK, which declared no name and no exposure. This
    // row read `walletTorActive` until stage S1 `copy`; FR-32 (a) is that the
    // SDK cannot attest onion routing for it, so the a11y string a
    // screen-reader user hears is now the unattested one.
    final handle = tester.ensureSemantics();
    expect(
      find.bySemanticsLabel(
        RegExp(RegExp.escape(l10n.walletTorActiveUnattested)),
      ),
      findsOneWidget,
    );
    expect(
      find.bySemanticsLabel(RegExp(RegExp.escape(l10n.walletTorActive))),
      findsNothing,
    );
    handle.dispose();
    // The as-of height rides IN the Balance header — the
    // fixture's stamp backs the height, so the ONE-line height+time variant
    // renders (the time is inline, never a second row).
    expect(
      find.text(
        // S12: the caption carries the time only.
        l10n.walletBalanceHeaderAt(
          balanceCaptionTime(
            DateTime.fromMillisecondsSinceEpoch(0),
            l10n.localeName,
          ),
        ),
      ),
      findsOneWidget,
    );
  });

  testWidgets('#397 watch-only chrome: the badge shows, and Send / Swap / '
      'Shield are HIDDEN (never dead-buttons), while Receive stays', (
    tester,
  ) async {
    // A watch-only wallet cannot spend/shield/swap. The chrome hides those
    // affordances (the SDK refuses them typed regardless); Receive stays (a
    // watch-only wallet still derives receive addresses from its UFVK), and a
    // "Watch-only" badge marks the header. Transparent funds are present so the
    // Shield button WOULD render for a spending wallet — proving it's the
    // watch-only gate, not an empty-balance one, that hides it.
    //
    // FR-49 S12 C3: a CONSISTENT fixture (spendable 0.5 + transparent 0.5 =
    // total 1.0). Spendable now shows only when it differs from the total, so
    // with spendable == total the "Spendable hidden" assertion below would pass
    // for the wrong reason; here spendable != total, so the watch-only gate is
    // the ONLY thing that can hide it.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(
          totalZat: 100000000,
          spendableZat: 50000000,
          transparentZat: 50000000,
        ),
        lastSynced: const SyncStamp(height: 100, at: 0),
      ),
    )..isWatchOnlyResult = true;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(
      find.byKey(const ValueKey('wallet-watch-only-badge')),
      findsOneWidget,
    );
    expect(find.text(l10n.walletWatchOnlyBadge), findsWidgets);
    // Spend affordances gone.
    expect(find.byKey(const ValueKey('wallet-action-send')), findsNothing);
    expect(find.byKey(const ValueKey('wallet-action-swap')), findsNothing);
    expect(find.byKey(const ValueKey('wallet-shield-button')), findsNothing);
    // Receive stays — a watch-only wallet still receives.
    expect(find.byKey(const ValueKey('wallet-action-receive')), findsOneWidget);

    // #397 P2 money-honesty: the card must not FRAME its funds as spendable.
    // "Spendable now" is hidden (nothing is spendable — no keys); the
    // transparent note drops its "shield to spend" clause for the watch-only
    // variant (keeping only the public-visibility fact). The holdings
    // themselves stay — the total headline + the shielded/transparent pool
    // split are still shown (watching is the point).
    expect(find.text(l10n.walletSpendableLabel), findsNothing);
    expect(find.text(l10n.walletTransparentNote), findsNothing);
    expect(find.text(l10n.walletTransparentNoteWatchOnly), findsOneWidget);
  });

  testWidgets('#397 P3 UX-L1: the watch-only badge is ONE semantics node — a '
      'screen reader says "Watch-only" once, not twice', (tester) async {
    // Without excludeSemantics the badge's child Text contributed its own node
    // beside the container label, and TalkBack announced the label twice
    // (device-confirmed).
    final handle = tester.ensureSemantics();
    final fake = FakeWalletSession()..isWatchOnlyResult = true;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.bySemanticsLabel(l10n.walletWatchOnlyBadge), findsOneWidget);
    handle.dispose();
  });

  testWidgets('#397 P3 UX-M6: the title + badge row survives a 3x text scale '
      'on a narrow screen without a RenderFlex overflow (the title ellipsizes, '
      'the badge stays whole)', (tester) async {
    tester.view.physicalSize = const Size(320, 720);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final fake = FakeWalletSession()..isWatchOnlyResult = true;
    await tester.pumpWidget(_harness(session: fake, textScale: 3.0));
    await tester.pumpAndSettle();

    // No overflow was thrown laying out the scaled app bar.
    expect(tester.takeException(), isNull);
    // The badge (the information carrier) is still present.
    expect(
      find.byKey(const ValueKey('wallet-watch-only-badge')),
      findsOneWidget,
    );
  });

  testWidgets('#397 P2 money-honesty: a SPENDING wallet keeps "Spendable now" '
      'and the spend-framed transparent note (proving the watch-only gate is '
      'what removes them)', (tester) async {
    // FR-49 S12 C3: the SAME consistent fixture as the watch-only twin above
    // (spendable 0.5 + transparent 0.5 = total 1.0). Spendable now renders
    // only when it differs from the total, which it does here — so the only
    // difference between the two tests is the watch-only flag.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(
          totalZat: 100000000,
          spendableZat: 50000000,
          transparentZat: 50000000,
        ),
        lastSynced: const SyncStamp(height: 100, at: 0),
      ),
    ); // isWatchOnly defaults false
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSpendableLabel), findsOneWidget);
    // …with the spendable figure itself, not just its label.
    expect(find.text(l10n.walletAmount(formatZec(50000000))), findsWidgets);
    expect(find.text(l10n.walletTransparentNote), findsOneWidget);
    expect(find.text(l10n.walletTransparentNoteWatchOnly), findsNothing);
  });

  testWidgets(
    'a SPENDING wallet keeps Send + the Shield button (the watch-only '
    'gate is what hides them, not the balance)',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
          balance: balanceFixture(
            totalZat: 100000000,
            spendableZat: 100000000,
            transparentZat: 50000000,
          ),
          lastSynced: const SyncStamp(height: 100, at: 0),
        ),
      ); // isWatchOnly defaults false
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();

      expect(find.byKey(const ValueKey('wallet-action-send')), findsOneWidget);
      expect(
        find.byKey(const ValueKey('wallet-shield-button')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('wallet-watch-only-badge')),
        findsNothing,
      );
    },
  );

  testWidgets('a provisioned wallet shows NO manual Start/Stop control', (
    tester,
  ) async {
    // Sync runs on its own (auto-drive by lifecycle) — the old Start/Stop
    // buttons are gone, and a healthy wallet shows no sync-failure notice.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(totalZat: 100000000),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.byIcon(Icons.play_arrow), findsNothing);
    expect(find.byIcon(Icons.pause), findsNothing);
    expect(find.text(l10n.walletSyncRetry), findsNothing);
  });

  testWidgets('a sync-start failure surfaces an honest notice + retry', (
    tester,
  ) async {
    // The rare drive-failed path (e.g. the handle closed): no silent failure —
    // an honest notice with a contextual retry, never a raw code (invariant 6).
    final fake = FakeWalletSession(
      current: const SyncStatus.idle(),
      snapshotValue: walletStateFixture(),
    )..failStart = true;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncStartFailed), findsOneWidget);
    expect(find.text(l10n.walletSyncRetry), findsOneWidget);
    // A FAILED drive is honestly "Not syncing yet" — never the optimistic
    // "Connecting…" (driving is false on a failed drive; the two are exclusive).
    expect(find.text(l10n.walletSyncIdle), findsOneWidget);
    expect(find.text(l10n.walletSyncStarting), findsNothing);

    // Recover + retry → the loop starts and the notice clears.
    fake.failStart = false;
    await tester.tap(find.text(l10n.walletSyncRetry));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSyncStartFailed), findsNothing);
    expect(fake.startCount, greaterThanOrEqualTo(2));
  });

  testWidgets('scanning renders a percent and a progress bar', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1,
        to: 100,
        percent: 0.42,
        spendableReady: true,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncScanning(42)), findsOneWidget);
    // fixed row: the spendable cue and the compact blocks-left count are
    // no longer visible row elements (the count overflowed narrow screens at
    // accessibility scales) — BOTH ride the badge's a11y label; the sheet
    // carries the exact figures; the GREEN level + unlocked Send carry
    // spendable visually.
    final handle = tester.ensureSemantics();
    expect(
      find.bySemanticsLabel(
        RegExp(RegExp.escape(l10n.walletSyncSpendableReady)),
      ),
      findsOneWidget,
    );
    expect(
      find.bySemanticsLabel(
        RegExp(RegExp.escape(l10n.walletSyncScanRemaining('99'))),
      ),
      findsOneWidget,
    );
    handle.dispose();
    expect(find.text(l10n.walletSyncScanRemaining('99')), findsNothing);
    // Past the opaque phase the bar is determinate (one indicator).
    final bar = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(bar.value, isNotNull);
  });

  testWidgets('initial deep sync is visibly ALIVE (animated bar + catching-up), never a '
      'frozen 0%', (tester) async {
    // The maintainer report: a deep first sync sits at "Scanning 0%" for a long
    // stretch because the monotonic note-fraction is still ~0 — a determinate
    // 0% bar reads as STUCK. The early phase must instead animate an
    // indeterminate bar and explain WHY it is slow.
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 2300000,
        to: 2650000,
        percent: 0.0,
        spendableReady: false,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    // NOT pumpAndSettle: the early-phase bar is an indeterminate animation that
    // never settles (by design). Pump a few frames to render the live banner.
    await tester.pump();
    await tester.pump();
    await tester.pump();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The headline is the number-less "Scanning…", NOT a stuck-looking
    // "Scanning 0%" (the maintainer report — a literal 0% that never moves reads as
    // frozen even with the live bar).
    expect(find.text(l10n.walletSyncScanningEarly), findsOneWidget);
    expect(find.text(l10n.walletSyncScanning(0)), findsNothing);
    // The honest "why it's slow" line rides the a11y label (fixed row;
    // visually it lives one tap away in the sheet)...
    final handle = tester.ensureSemantics();
    expect(
      find.bySemanticsLabel(RegExp(RegExp.escape(l10n.walletSyncCatchingUp))),
      findsOneWidget,
    );
    handle.dispose();
    // ...the bar is INDETERMINATE (value == null ⇒ animated, clearly working)...
    final bar = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(bar.value, isNull);
    // ...and no raw block-count is shown during the opaque phase (it would jump
    // around with the spend-before-sync frontier — only the >0% phase shows it).
    expect(find.textContaining('blocks left'), findsNothing);
  });

  testWidgets('a Tor-unavailable stall shows the honest fail-closed reason', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(reason: StallReason.torUnavailable),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncStalled), findsOneWidget);
    // fixed row: the typed reason rides the a11y label; the sheet shows
    // it visually (covered by the stall-sheet test below).
    final handle = tester.ensureSemantics();
    expect(
      find.bySemanticsLabel(RegExp(RegExp.escape(l10n.walletStallTor))),
      findsOneWidget,
    );
    handle.dispose();
  });

  testWidgets('offline shows that queued sends are normal, not an error', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.offline(),
      snapshotValue: walletStateFixture(syncStatus: const SyncStatus.offline()),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncOffline), findsOneWidget);
    // fixed row: the queued-sends-are-normal line rides the a11y label
    // and the sheet.
    final handle = tester.ensureSemantics();
    expect(
      find.bySemanticsLabel(
        RegExp(RegExp.escape(l10n.walletSyncOfflineDetail)),
      ),
      findsOneWidget,
    );
    handle.dispose();
  });

  testWidgets(
    'unshielded (transparent) funds stay visible with a privacy note',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 1),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 1),
          // A nonzero transparent balance (e.g. a failed auto-shield) must be
          // surfaced, not folded silently into the total.
          balance: balanceFixture(totalZat: 100000000, transparentZat: 2000000),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(find.text(l10n.walletTransparentLabel), findsOneWidget);
      expect(find.text(l10n.walletTransparentNote), findsOneWidget);
      expect(find.text(l10n.walletAmount('0.02')), findsOneWidget);
    },
  );

  testWidgets('recoverable one-time-address funds show as a SUBSET note, never '
      'added on top of the balance (2e-2b-iv)', (tester) async {
    // 0.01 ZEC sits on a one-time address — a SUBSET of the 0.02 transparent
    // (and the 1 ZEC total) already shown, NOT extra funds. The note must render
    // and the headline total must stay 1 ZEC (never 1.01 — the over-count trap).
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 1),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 1),
              balance: balanceFixture(
                totalZat: 100000000,
                transparentZat: 2000000,
              ),
            ),
          )
          ..recoverableEphemeralFundsResult = const [
            RecoverableEphemeralFunds(recoverableZat: 1000000, isFinal: true),
          ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The locational subset note (recoverable-now variant) is shown…
    expect(
      find.text(l10n.walletRecoverableEphemeralNote('0.01')),
      findsOneWidget,
    );
    // …the headline total is UNCHANGED (subset, never additive)…
    expect(find.text(l10n.walletAmount('1')), findsWidgets);
    expect(find.text(l10n.walletAmount('1.01')), findsNothing);
    // …and the confirming variant is NOT shown (this amount is reorg-final).
    expect(
      find.text(l10n.walletRecoverableEphemeralConfirmingNote('0.01')),
      findsNothing,
    );
  });

  testWidgets('#397 P2 — a WATCH-ONLY wallet\'s recoverable-final note DROPS '
      '"(recoverable)" (recovery is a spend it can\'t do) but keeps the '
      'locational fact', (tester) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 1),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 1),
              balance: balanceFixture(
                totalZat: 100000000,
                transparentZat: 2000000,
              ),
            ),
          )
          ..isWatchOnlyResult = true
          ..recoverableEphemeralFundsResult = const [
            RecoverableEphemeralFunds(recoverableZat: 1000000, isFinal: true),
          ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The watch-only variant (locational, no recoverability promise) shows;
    // the base "(recoverable)" note does NOT.
    expect(
      find.text(l10n.walletRecoverableEphemeralNoteWatchOnly('0.01')),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletRecoverableEphemeralNote('0.01')),
      findsNothing,
    );
  });

  testWidgets('a still-confirming recoverable amount reads "still confirming", '
      'never settled/ready (2e-2b-iv)', (tester) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 1),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 1),
              balance: balanceFixture(
                totalZat: 100000000,
                transparentZat: 2000000,
              ),
            ),
          )
          ..recoverableEphemeralFundsResult = const [
            RecoverableEphemeralFunds(recoverableZat: 1000000, isFinal: false),
          ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(
      find.text(l10n.walletRecoverableEphemeralConfirmingNote('0.01')),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletRecoverableEphemeralNote('0.01')),
      findsNothing,
    );
  });

  testWidgets('no recoverable funds: no one-time-address note (the dormant '
      'default)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(totalZat: 100000000, transparentZat: 2000000),
      ),
    ); // recoverableEphemeralFundsResult defaults to empty
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The transparent line is present, but no recoverable note rides under it.
    expect(find.text(l10n.walletTransparentNote), findsOneWidget);
    expect(
      find.text(l10n.walletRecoverableEphemeralNote('0.01')),
      findsNothing,
    );
  });

  testWidgets('a recoverable amount is CLAMPED to the transparent line — the '
      'subset invariant holds at the render under provider skew (2e-2b-iv)', (
    tester,
  ) async {
    // Adversarial provider skew: the recoverable list (0.05) momentarily exceeds
    // the snapshot's transparent line (0.02) — e.g. a shield dropped transparent
    // before the recoverable list re-pulled. The note must DISPLAY the clamped
    // 0.02 (never claim more on a one-time address than the whole transparent line
    // shown above it), and the headline must stay 1 ZEC.
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 1),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 1),
              balance: balanceFixture(
                totalZat: 100000000,
                transparentZat: 2000000,
              ),
            ),
          )
          ..recoverableEphemeralFundsResult = const [
            RecoverableEphemeralFunds(recoverableZat: 5000000, isFinal: true),
          ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // Clamped to the transparent line (0.02), NOT the stale-high 0.05.
    expect(
      find.text(l10n.walletRecoverableEphemeralNote('0.02')),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletRecoverableEphemeralNote('0.05')),
      findsNothing,
    );
    // The headline total is untouched.
    expect(find.text(l10n.walletAmount('1')), findsWidgets);
  });

  testWidgets('zero transparent funds show no unshielded line', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(totalZat: 100000000),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletTransparentLabel), findsNothing);
  });

  testWidgets('pool line (#389): the shielded/transparent split shows under '
      'the headline and the two figures sum EXACTLY to the total', (
    tester,
  ) async {
    // 1.25 total, 0.05 transparent → 1.2 shielded. shielded + transparent must
    // reconcile to the 1.25 headline — a money figure that silently disagreed
    // with the total it breaks down would be a comprehension hazard.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(
          totalZat: 125000000,
          spendableZat: 120000000,
          transparentZat: 5000000,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    final line = find.byKey(const ValueKey('wallet-pool-line'));
    expect(line, findsOneWidget);

    final shieldedText = l10n.walletPoolShielded('1.2');
    final transparentText = l10n.walletPoolTransparent('0.05');
    // Filter to the LABEL RichText (an Icon is also a RichText internally).
    final rich = tester.widget<RichText>(
      find.descendant(
        of: line,
        matching: find.byWidgetPredicate(
          (w) => w is RichText && w.text.toPlainText().contains(shieldedText),
        ),
      ),
    );
    final plain = rich.text.toPlainText();
    expect(plain, contains(shieldedText));
    expect(plain, contains(transparentText));
    // The two bare figures reconcile to the headline (1.2 + 0.05 = 1.25).
    expect(find.text(l10n.walletAmount('1.25')), findsWidgets);

    // The public pool wears the card's warning colour (the learned "visible
    // on-chain" colour), the shielded segment does not. FR-49 S12 C3: the
    // card is the `deep` surface, so the warning is `warningOnDeep` (the
    // on-deep twin of the old privacy-orange), and the shielded figure is
    // `onDeep`.
    final colors = WalletColors.of(tester.element(line));
    // Text.rich nests our span tree one level under the ambient
    // DefaultTextStyle, so walk ALL descendant spans to find the segment.
    final spans = <TextSpan>[];
    rich.text.visitChildren((span) {
      if (span is TextSpan) spans.add(span);
      return true;
    });
    final transparentSpan = spans.firstWhere((s) => s.text == transparentText);
    expect(transparentSpan.style?.color, colors.warningOnDeep);
    // The shielded segment carries no colour of its own and inherits the
    // line's base style — onDeep, never the warning colour. (visitChildren
    // skips text-less spans, so the base span is found by a direct walk.)
    expect(
      spans.firstWhere((s) => s.text == shieldedText).style?.color,
      isNull,
    );
    TextSpan? baseOf(InlineSpan span) {
      if (span is! TextSpan) return null;
      if (span.children?.any((c) => c is TextSpan && c.text == shieldedText) ??
          false) {
        return span;
      }
      for (final c in span.children ?? const <InlineSpan>[]) {
        final hit = baseOf(c);
        if (hit != null) return hit;
      }
      return null;
    }

    expect(baseOf(rich.text)?.style?.color, colors.onDeep);
  });

  testWidgets('pool line (#389, FR-49 S12 C3): an all-shielded, all-spendable '
      'wallet renders ONLY the header, the total and the coin — no pool line, '
      'no "All shielded" line, no breakdown row', (tester) async {
    // The everyday card (Relim §9.4 rev.2): the split and Spendable say
    // nothing the total does not, so neither renders. The #389 "always-on"
    // affirmation was dropped by C3 — the everyday card claims nothing about
    // pools.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(totalZat: 120000000, spendableZat: 120000000),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    final card = find.byKey(const ValueKey('wallet-balance-card'));
    expect(find.byKey(const ValueKey('wallet-pool-line')), findsNothing);
    expect(find.text(l10n.walletPoolAllShielded), findsNothing);
    expect(find.text(l10n.walletSpendableLabel), findsNothing);
    expect(find.text(l10n.walletTransparentLabel), findsNothing);
    expect(find.text(l10n.walletArrivingLabel), findsNothing);
    expect(find.text(l10n.walletPendingChangeLabel), findsNothing);
    expect(find.byKey(const ValueKey('wallet-shield-button')), findsNothing);

    // Positively: the coin, and exactly two texts outside it — the header and
    // the total. Anything else on the everyday card is a regression.
    expect(
      find.descendant(of: card, matching: find.byType(WalletCoin)),
      findsOneWidget,
    );
    final coinTexts = find
        .descendant(of: find.byType(WalletCoin), matching: find.byType(Text))
        .evaluate()
        .map((e) => e.widget)
        .toSet();
    final cardTexts = find
        .descendant(of: card, matching: find.byType(Text))
        .evaluate()
        .map((e) => e.widget as Text)
        .where((t) => !coinTexts.contains(t))
        .map((t) => t.data)
        .toList();
    // No stamp in the fixture -> the header pairs the session latch's own
    // moment with the tip (#317), exactly as the test reads it.
    final latch =
        ProviderScope.containerOf(
              tester.element(find.byType(WalletScreen)),
            ).read(walletSyncedTipProvider)
            as WalletSyncedTipLatched;
    final header = l10n.walletBalanceHeaderAt(
      balanceCaptionTime(latch.at, l10n.localeName),
    );
    expect(cardTexts, [header, l10n.walletAmount('1.2')]);
  });

  testWidgets('pool line (#389): a zero-balance wallet shows NO pool line — '
      'nothing to break down', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(), // all zero
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('wallet-pool-line')), findsNothing);
  });

  testWidgets(
    'pool line (#389): tapping it opens the Transparent funds sheet',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 1),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 1),
          balance: balanceFixture(
            totalZat: 125000000,
            spendableZat: 120000000,
            transparentZat: 5000000,
          ),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      await tester.tap(find.byKey(const ValueKey('wallet-pool-line')));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletTransparentFundsTitle), findsOneWidget);
    },
  );

  testWidgets('pool line (#389): never clips a figure — a long split at 2× '
      'text scale scales down without overflow', (tester) async {
    // ~21M ZEC total (near max supply) with a fractional transparent tail — the
    // worst case for a two-figure line — at 2× OS text scale.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(
          totalZat: 2100000012345678,
          spendableZat: 2100000000000000,
          transparentZat: 12345678,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake, textScale: 2.0));
    await tester.pumpAndSettle();
    // The FittedBox scales the line down with every digit intact — no
    // RenderFlex/paint overflow is thrown, and the line still renders.
    expect(tester.takeException(), isNull);
    expect(find.byKey(const ValueKey('wallet-pool-line')), findsOneWidget);
  });

  testWidgets('pool line (#389) a11y: ONE labeled button node WITH a tap '
      'action — a screen-reader double-tap opens the sheet (excludeSemantics '
      'would otherwise drop the InkWell action)', (tester) async {
    final handle = tester.ensureSemantics();
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(
          totalZat: 125000000,
          spendableZat: 120000000,
          transparentZat: 5000000,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    final node = tester.getSemantics(
      find.byKey(const ValueKey('wallet-pool-line')),
    );
    // Button, tappable, and the spoken label is the comma-joined breakdown
    // (no middot) — not a dead relabelled button.
    expect(
      node,
      isSemantics(
        isButton: true,
        hasTapAction: true,
        label:
            '${l10n.walletPoolShielded('1.2')}, '
            '${l10n.walletPoolTransparent('0.05')}',
      ),
    );
    handle.dispose();
  });

  testWidgets('pool line (#389): past ~1.4× text scale the two figures STACK '
      'onto their own full-width lines — the user\'s size is not scaled away', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(
          totalZat: 125000000,
          spendableZat: 120000000,
          transparentZat: 5000000,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake, textScale: 2.0));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // Stacked renders each segment as its OWN plain Text (the compact branch
    // fuses them into a single Text.rich), so find.text matches both — proof
    // each figure got a full-width line instead of being shrunk to share one.
    expect(find.text(l10n.walletPoolShielded('1.2')), findsOneWidget);
    expect(find.text(l10n.walletPoolTransparent('0.05')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Shield button a11y (#389 fold): a labeled button node WITH a '
      'tap action — excludeSemantics would otherwise drop the money action', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(totalZat: 100000000, transparentZat: 2000000),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(
      tester.getSemantics(find.byKey(const ValueKey('wallet-shield-button'))),
      isSemantics(
        isButton: true,
        hasTapAction: true,
        label: l10n.walletShieldButton,
      ),
    );
    handle.dispose();
  });

  testWidgets('Tor active via an unverified runtime is qualified, not green', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        tor: const TorState.active(runtime: TorRuntimeKind.unknown()),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The unattributable-runtime claim must NOT read as confident protection.
    // the label rides the badge's a11y string (the row shows the icon).
    final handle = tester.ensureSemantics();
    expect(
      find.bySemanticsLabel(
        RegExp(RegExp.escape(l10n.walletTorActiveUnverified)),
      ),
      findsOneWidget,
    );
    expect(
      // The escape below would also match the "unverified" variant (it is a
      // prefix superstring), so pin the CONFIDENT label with a word boundary:
      // "Tor active" followed by the a11y separator, not more words.
      find.bySemanticsLabel(
        RegExp('${RegExp.escape(l10n.walletTorActive)}\\.'),
      ),
      findsNothing,
    );
    handle.dispose();
  });

  testWidgets('pending incoming funds appear only when nonzero', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(
          totalZat: 100000000,
          pendingIncomingZat: 5000000,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // S12 rev.3: synced, pending incoming is "Arriving", with a `+`, outside
    // the headline.
    expect(find.text(l10n.walletArrivingLabel), findsOneWidget);
    expect(find.text(l10n.walletAmount('+0.05')), findsOneWidget);
  });

  testWidgets('a cold snapshot read failure degrades to the honest card', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.idle(),
      snapshotThrows: true,
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // Recoverable, plain-language, no error code (invariant 6).
    expect(find.text(l10n.walletSnapshotUnavailable), findsOneWidget);
  });

  testWidgets('sync banner reads as ONE merged screen-reader node', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(reason: StallReason.torUnavailable),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // iOS would otherwise read the headline and detail as separate (or
    // doubled) nodes; the banner exposes one coherent label. The transport
    // label rides between headline and details (the row shows only
    // the transport ICON; the fixture's TorState is off).
    final expected =
        '${l10n.walletSyncStalled}. ${l10n.walletTorOff}. ${l10n.walletStallTor}';
    expect(find.bySemanticsLabel(expected), findsOneWidget);
    handle.dispose();
  });

  testWidgets('scanning NEVER reads 100% before sync is actually done', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1,
        to: 100,
        // 99.9% scanned — round() would show "100%" and lie that a partial
        // balance is final; floor must show 99 (100 is UpToDate's alone).
        percent: 0.999,
        spendableReady: false,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncScanning(99)), findsOneWidget);
    expect(find.text(l10n.walletSyncScanning(100)), findsNothing);
  });

  testWidgets('scanning floors the percent (0.4% reads 0, not 0.4)', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1,
        to: 100,
        percent: 0.004,
        spendableReady: false,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    // NOT pumpAndSettle: at floored-0% the bar animates indeterminately.
    await tester.pump();
    await tester.pump();
    await tester.pump();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // 0.4% floors into the opaque early phase — the number-less "Scanning…",
    // never a rounded-up "Scanning 0%" (which reads as frozen).
    expect(find.text(l10n.walletSyncScanningEarly), findsOneWidget);
    expect(find.text(l10n.walletSyncScanning(0)), findsNothing);
  });

  testWidgets(
    'a driven loop with no batch yet reads "Connecting…", never "Not syncing yet"',
    (tester) async {
      // The maintainer report: during the silent prep phase (commitment-tree roots +
      // chain-tip fetch) the loop is started but the SDK status is still Idle. With
      // the drive running, that must read as connecting — not a stale "Not syncing
      // yet". The real WalletSyncController auto-starts on the active surface, so a
      // plain Idle fake (startSync succeeds) drives true.
      final fake = FakeWalletSession(
        current: const SyncStatus.idle(),
        snapshotValue: walletStateFixture(),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester
          .pumpAndSettle(); // driving-Idle has no animated bar — settles
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(find.text(l10n.walletSyncStarting), findsOneWidget);
      // fixed row: the prep-phase detail rides the a11y label + the sheet.
      final handle = tester.ensureSemantics();
      expect(
        find.bySemanticsLabel(
          RegExp(RegExp.escape(l10n.walletSyncStartingDetail)),
        ),
        findsOneWidget,
      );
      handle.dispose();
      expect(find.text(l10n.walletSyncIdle), findsNothing);
    },
  );

  testWidgets(
    'a deep span: COMPACT count in the a11y label, EXACT count one tap away '
    'in the sheet, no scary number on the badge',
    (tester) async {
      // Maintainer never the scary jittery full number on the badge. Maintainer
      // one fixed row — the compact chip moved from the row into the a11y
      // label (it overflowed narrow screens at accessibility scales); the sheet
      // now owns the exact figure.
      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 1400000,
          to: 2900000,
          percent: 0.5,
          spendableReady: false,
          rewound: false,
        ),
        snapshotValue: walletStateFixture(),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // The badge shows NO count text at all; the a11y label speaks the compact
      // form ("1.5M blocks left").
      expect(find.text(l10n.walletSyncScanRemaining('1.5M')), findsNothing);
      expect(find.textContaining('1,500,000'), findsNothing);
      final handle = tester.ensureSemantics();
      expect(
        find.bySemanticsLabel(
          RegExp(RegExp.escape(l10n.walletSyncScanRemaining('1.5M'))),
        ),
        findsOneWidget,
      );
      handle.dispose();
      expect(find.byType(LinearProgressIndicator), findsOneWidget);

      // The EXACT grouped count is one tap away in the sheet.
      await tester.tap(find.byType(LinearProgressIndicator));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSyncSheetBlocksLeft), findsOneWidget);
      expect(find.text('1,500,000'), findsOneWidget);
    },
  );

  testWidgets(
    'the opaque phase can still show funds-ready (catching-up + spendable)',
    (tester) async {
      // Spend-before-sync: a wallet can have usable funds while the note-fraction
      // still rounds to 0 — both the catching-up why-line and the spendable line
      // must show. Indeterminate bar ⇒ explicit pumps, never pumpAndSettle.
      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 2300000,
          to: 2650000,
          percent: 0.0,
          spendableReady: true,
          rewound: false,
        ),
        snapshotValue: walletStateFixture(),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pump();
      await tester.pump();
      await tester.pump();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // fixed row: BOTH lines ride the badge's one a11y label (visually
      // they live in the sheet; the GREEN level + Send unlock carry spendable).
      final handle = tester.ensureSemantics();
      expect(
        find.bySemanticsLabel(
          RegExp(
            '${RegExp.escape(l10n.walletSyncCatchingUp)}.*'
            '${RegExp.escape(l10n.walletSyncSpendableReady)}',
          ),
        ),
        findsOneWidget,
      );
      handle.dispose();
    },
  );

  testWidgets('connecting renders the Tor bootstrap percent and no bar', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.connecting(torBootstrapPercent: 0.5),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncConnectingPercent(50)), findsOneWidget);
    // Only Scanning shows a determinate bar; connecting must not.
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets('connecting with no percent shows the plain connecting label', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.connecting(),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSyncConnecting), findsOneWidget);
  });

  testWidgets('pending change is shown so the breakdown reconciles to total', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        // total(1.0) = spendable(0.6) + pendingChange(0.4); the change line
        // must be visible or the lines don't sum to the headline.
        balance: balanceFixture(
          totalZat: 100000000,
          spendableZat: 60000000,
          pendingChangeZat: 40000000,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletPendingChangeLabel), findsOneWidget);
    expect(find.text(l10n.walletAmount('0.4')), findsOneWidget);
  });

  testWidgets('the full balance breakdown (incl. pending change) renders without '
      'overflow on a narrow (320px) phone at 1.4x text scale', (tester) async {
    // A11y + money clarity: every _Line in the card is a fixed label+amount
    // row; on a 320-logical-px phone with large accessibility text the labels
    // must ellipsize (never a RenderFlex overflow over the money figures).
    tester.view.physicalSize = const Size(320, 1400);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        // Every breakdown line nonzero (so ALL rows render) and summing to the
        // headline total. The squeeze under test is the LABELS ("Pending
        // change" at 1.4x is wider than a 320px row can give it) — the
        // amounts stay realistic-width because the row protects the money
        // figure by design (it is the label that must ellipsize).
        balance: balanceFixture(
          totalZat: 123440000,
          spendableZat: 61230000,
          pendingIncomingZat: 11110000,
          pendingChangeZat: 41110000,
          transparentZat: 9990000,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake, textScale: 1.4));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletSpendableLabel), findsOneWidget);
    // S12 rev.3: synced, pending incoming renders as "Arriving".
    expect(find.text(l10n.walletArrivingLabel), findsOneWidget);
    expect(find.text(l10n.walletPendingChangeLabel), findsOneWidget);
    expect(find.text(l10n.walletTransparentLabel), findsOneWidget);
    // A RenderFlex overflow would surface here; the Flexible labels prevent it.
    expect(
      tester.takeException(),
      isNull,
      reason: 'no overflow at 1.4x scale on a 320px width',
    );
  });

  testWidgets('a transient reload failure keeps the last-known balance', (
    tester,
  ) async {
    // A busy-DB snapshot throw on resume must NOT blank a live wallet — the
    // last-known state stays rendered (only a cold first-load failure shows
    // the error card).
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(totalZat: 100000000),
      ),
    );
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const WalletScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    expect(find.text(l10n.walletAmount('1')), findsWidgets); // 1 ZEC shown

    // The next cold read fails; the provider re-fetches and errors.
    fake.snapshotThrows = true;
    container.invalidate(walletSnapshotReadProvider);
    await tester.pumpAndSettle();

    // Last-known balance survives; no honest-error card (it's still live)…
    expect(find.text(l10n.walletAmount('1')), findsWidgets);
    expect(find.text(l10n.walletSnapshotUnavailable), findsNothing);
    // …but the wallet is HONEST that the balance may be stale (no silent
    // failure): the refresh-failed cue is shown alongside the kept balance.
    expect(find.text(l10n.walletBalanceStale), findsOneWidget);
  });

  testWidgets('a healthy refresh clears the stale notice', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1),
        balance: balanceFixture(totalZat: 100000000),
      ),
    );
    final container = ProviderContainer(
      overrides: [walletSessionProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const WalletScreen(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // Fail a refresh → stale notice appears.
    fake.snapshotThrows = true;
    container.invalidate(walletSnapshotReadProvider);
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletBalanceStale), findsOneWidget);

    // Recover → the notice clears (not sticky).
    fake.snapshotThrows = false;
    container.invalidate(walletSnapshotReadProvider);
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletBalanceStale), findsNothing);
  });

  testWidgets(
    'a synced wallet reports freshness, never "unknown until first sync"',
    (tester) async {
      // Maintainer report: even when UpToDate the age line read "unknown until first
      // sync" because the lastSynced stamp isn't persisted yet — derive freshness
      // from the live UpToDate status (tip) instead.
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 2900000),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 2900000),
          balance: balanceFixture(totalZat: 100000000, spendableZat: 100000000),
          // no lastSynced stamp — the current slice's reality
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // With no persisted stamp, the LATCH lends its own observation moment
      // (#317): the header renders the PAIRED height+time arm from the very
      // first up-to-date frame — one stable arm, no per-cycle pulse.
      final latch =
          ProviderScope.containerOf(
                tester.element(find.byType(WalletScreen)),
              ).read(walletSyncedTipProvider)
              as WalletSyncedTipLatched;
      expect(
        find.text(
          l10n.walletBalanceHeaderAt(
            balanceCaptionTime(latch.at, l10n.localeName),
          ),
        ),
        findsOneWidget,
      );
      // The bare no-as-of header must not ALSO render (one Text, one truth).
      expect(find.text(l10n.walletBalanceLabel), findsNothing);
    },
  );

  // --- Send gate (ADR-0533: spend-before-sync, gate on spendable, not on %) --

  testWidgets('Send is ENABLED as soon as funds are spendable (before 100%)', (
    tester,
  ) async {
    // The wallet is still scanning history (not UpToDate) but a near-tip note is
    // already spendable — Send must be live (the spend-before-sync promise),
    // with no disabled-reason line.
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1,
        to: 100,
        percent: 0.5,
        spendableReady: true,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.scanning(
          from: 1,
          to: 100,
          percent: 0.5,
          spendableReady: true,
          rewound: false,
        ),
        balance: balanceFixture(totalZat: 50000000, spendableZat: 50000000),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(_sendInk(tester).onTap, isNotNull);
    // The reason is HIDDEN (Send is live) but its space is RESERVED so the
    // Receive/Swap buttons never jump when Send toggles (maintainer: no layout
    // shift on hint show/hide).
    final vis = tester.widget<Visibility>(
      find
          .ancestor(
            of: find.text(l10n.walletSendWaitingForFunds),
            matching: find.byType(Visibility),
          )
          .first,
    );
    expect(vis.visible, isFalse);
    expect(vis.maintainSize, isTrue);
  });

  testWidgets('Send is DISABLED while syncing with nothing spendable yet', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1,
        to: 100,
        percent: 0.5,
        spendableReady: false,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.scanning(
          from: 1,
          to: 100,
          percent: 0.5,
          spendableReady: false,
          rewound: false,
        ),
        balance: balanceFixture(), // spendable 0
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // No dead-end into a form that can only fail — honest reason instead.
    expect(_sendInk(tester).onTap, isNull);
    expect(find.text(l10n.walletSendWaitingForFunds), findsOneWidget);
  });

  testWidgets('Send is DISABLED on a synced wallet with no spendable funds', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(), // synced but empty
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(_sendInk(tester).onTap, isNull);
    // The synced-but-empty reason, not the still-syncing one.
    expect(find.text(l10n.walletSendNoSpendableYet), findsOneWidget);
    expect(find.text(l10n.walletSendWaitingForFunds), findsNothing);
  });

  testWidgets(
    'balance + Send refresh when funds become spendable mid-sync (no resume)',
    (tester) async {
      // ADR-0533 "see balance asap": the cold snapshot is otherwise re-read only
      // on resume. When the SDK flips spendableReady mid-foreground-sync, the
      // screen's edge listener must re-read it so the balance appears and Send
      // unlocks without backgrounding the app.
      final scanningNoFunds = const SyncStatus.scanning(
        from: 1,
        to: 100,
        percent: 0.3,
        spendableReady: false,
        rewound: false,
      );
      final fake = FakeWalletSession(
        current: scanningNoFunds,
        snapshotValue: walletStateFixture(
          syncStatus: scanningNoFunds,
          balance: balanceFixture(), // spendable 0 at first
        ),
      );
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // Nothing spendable → Send disabled, balance zero.
      expect(_sendInk(tester).onTap, isNull);

      // Funds clear: the next cold read would show them, and the SDK flips the
      // live spendable hint.
      fake.setSnapshot(
        walletStateFixture(
          syncStatus: const SyncStatus.scanning(
            from: 1,
            to: 100,
            percent: 0.6,
            spendableReady: true,
            rewound: false,
          ),
          balance: balanceFixture(totalZat: 50000000, spendableZat: 50000000),
        ),
      );
      fake.push(
        const SyncStatus.scanning(
          from: 1,
          to: 100,
          percent: 0.6,
          spendableReady: true,
          rewound: false,
        ),
      );
      await tester.pumpAndSettle();

      // The edge listener re-read the snapshot → Send is live, balance shown.
      expect(_sendInk(tester).onTap, isNotNull);
      expect(find.text(l10n.walletAmount('0.5')), findsWidgets);
    },
  );

  testWidgets(
    'a stalled wallet gives the can\'t-sync Send reason, not "waiting for funds"',
    (tester) async {
      // The disabled-Send reason must match the badge: a hard stall won't reach
      // funds until the user acts, so "once syncing resumes", never an optimistic
      // "once sync reaches your funds" (review HARDENING — all three reviewers).
      final fake = FakeWalletSession(
        current: const SyncStatus.stalled(
          reason: StallReason.endpointUnreachable,
        ),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.stalled(
            reason: StallReason.endpointUnreachable,
          ),
          balance: balanceFixture(),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(_sendInk(tester).onTap, isNull);
      expect(find.text(l10n.walletSendSyncUnavailable), findsOneWidget);
      expect(find.text(l10n.walletSendWaitingForFunds), findsNothing);
    },
  );

  testWidgets(
    'an all-zero balance shows no "Spendable now" line (no contradiction)',
    (tester) async {
      // Maintainer: "Spendable now: 0" next to a disabled Send reads as a
      // contradiction. At an all-zero balance the breakdown is suppressed — the
      // headline "0 ZEC" stands alone and the disabled-Send reason carries the why.
      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 1,
          to: 100,
          percent: 0.5,
          spendableReady: false,
          rewound: false,
        ),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.scanning(
            from: 1,
            to: 100,
            percent: 0.5,
            spendableReady: false,
            rewound: false,
          ),
          balance: balanceFixture(), // all zero
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(find.text(l10n.walletSpendableLabel), findsNothing);
      expect(_sendInk(tester).onTap, isNull);
    },
  );

  testWidgets(
    'Send stays ENABLED through a stale refresh when funds are spendable',
    (tester) async {
      // `stale` colors the BADGE (yellow), it never gates Send — spendable funds
      // remain spendable through a failed cold refresh (review NIT coverage).
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 1),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 1),
          balance: balanceFixture(totalZat: 50000000, spendableZat: 50000000),
        ),
      );
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));
      expect(_sendInk(tester).onTap, isNotNull);

      // A cold refresh now fails → stale notice, but the last-known spendable
      // balance stays and Send stays live.
      fake.snapshotThrows = true;
      container.invalidate(walletSnapshotReadProvider);
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletBalanceStale), findsOneWidget);
      expect(_sendInk(tester).onTap, isNotNull);
    },
  );

  // ── Activity history (FR-1) ────────────────────────────────────────────────

  testWidgets('activity: empty wallet shows the honest empty state', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
      ),
    )..transactionsResult = const [];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletActivityTitle), findsOneWidget);
    expect(find.text(l10n.walletActivityEmpty), findsOneWidget);
  });

  testWidgets('activity: renders incoming + outgoing rows with SIGNED amounts', (
    tester,
  ) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 100),
            ),
          )
          ..transactionsResult = [
            // incoming +0.0025 ZEC, confirmed, with a memo
            txSummaryFixture(
              txidHex:
                  'aa00000000000000000000000000000000000000000000000000000000000001',
              netAmountZat: 250000,
              hasMemo: true,
              status: const TxStatus.confirmed(depth: 5),
            ),
            // outgoing -0.0101 ZEC, pending, with a fee
            txSummaryFixture(
              txidHex:
                  'bb00000000000000000000000000000000000000000000000000000000000002',
              netAmountZat: -1010000,
              feeZat: 10000,
              hasMemo: false,
              minedHeight: null,
              status: const TxStatus.pending(),
            ),
          ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletActivityReceived), findsOneWidget);
    expect(find.text(l10n.walletActivitySent), findsOneWidget);
    // SIGNED, exact integer money (no float drift): +0.0025 and −0.0101 ZEC.
    // FR-49 S12 C5: outgoing uses the true minus sign U+2212, never the ASCII
    // hyphen formatZec emits.
    expect(find.text(l10n.walletAmount('+0.0025')), findsOneWidget);
    expect(find.text(l10n.walletAmount('−0.0101')), findsOneWidget);
    expect(find.text(l10n.walletAmount('-0.0101')), findsNothing);
    // The confirmed row shows its depth; the unmined row shows Pending (each
    // status rides a subtitle "<status> · <time>", so match by substring).
    expect(
      find.textContaining(l10n.walletActivityConfirmations(5)),
      findsOneWidget,
    );
    expect(find.textContaining(l10n.walletActivityPending), findsOneWidget);
    // A memo glyph rides the row that carries one (exactly one of the two).
    expect(find.byIcon(Icons.mail_outline), findsOneWidget);
    // The provider requested exactly one bounded page (no accidental huge limit).
    expect(fake.lastTransactionsLimit, walletActivityPageSize);
  });

  testWidgets('activity: a ZERO-net tx is NOT "Received" / "+0"', (
    tester,
  ) async {
    // The boundary the money-display logic must get right: net 0 (self-transfer /
    // fee-only) is neutral, never an incoming "+0 ZEC Received". A non-zero balance
    // (1 ZEC) so the activity row's "0 ZEC" is unique (not the balance card's).
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(totalZat: 100000000, spendableZat: 100000000),
      ),
    )..transactionsResult = [txSummaryFixture(netAmountZat: 0, hasMemo: false)];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletActivityReceived), findsNothing);
    expect(find.text(l10n.walletActivitySent), findsOneWidget);
    expect(find.text(l10n.walletAmount('0')), findsOneWidget);
    expect(find.text(l10n.walletAmount('+0')), findsNothing);
  });

  testWidgets('activity: renders in DARK theme (tokens resolve, no crash)', (
    tester,
  ) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 100),
            ),
          )
          ..transactionsResult = [
            txSummaryFixture(netAmountZat: 250000),
            txSummaryFixture(
              txidHex:
                  'cc00000000000000000000000000000000000000000000000000000000000003',
              netAmountZat: -50000,
              status: const TxStatus.pending(),
              minedHeight: null,
            ),
          ];
    await tester.pumpWidget(_harness(session: fake, theme: darkTheme));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(tester.takeException(), isNull);
    expect(find.text(l10n.walletActivityReceived), findsOneWidget);
    expect(find.text(l10n.walletActivitySent), findsOneWidget);
  });

  testWidgets('activity: no overflow at 2x text scale (dynamic text)', (
    tester,
  ) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 100),
            ),
          )
          ..transactionsResult = [
            // A large amount + a memo at 2x scale is the worst case for the row.
            txSummaryFixture(netAmountZat: 2100000000000000, hasMemo: true),
          ];
    await tester.pumpWidget(_harness(session: fake, textScale: 2.0));
    await tester.pumpAndSettle();

    // A RenderFlex overflow would surface here; the Flexible amount must prevent it.
    expect(tester.takeException(), isNull);
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    expect(find.text(l10n.walletActivityReceived), findsOneWidget);
  });

  testWidgets('activity: status label matrix (singular/queued/failed)', (
    tester,
  ) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 100),
            ),
          )
          ..transactionsResult = [
            txSummaryFixture(
              txidHex:
                  '1100000000000000000000000000000000000000000000000000000000000001',
              status: const TxStatus.confirmed(depth: 1), // SINGULAR plural arm
            ),
            txSummaryFixture(
              txidHex:
                  '2200000000000000000000000000000000000000000000000000000000000002',
              status: const TxStatus.queued(),
              minedHeight: null,
            ),
            txSummaryFixture(
              txidHex:
                  '3300000000000000000000000000000000000000000000000000000000000003',
              status: const TxStatus.failed(),
              minedHeight: null,
            ),
          ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(
      find.textContaining(l10n.walletActivityConfirmations(1)),
      findsOneWidget,
    );
    expect(find.textContaining(l10n.walletActivityQueued), findsOneWidget);
    expect(find.textContaining(l10n.walletActivityFailed), findsOneWidget);
  });

  testWidgets('activity: expired status takes precedence (funds returned)', (
    tester,
  ) async {
    final fake =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 100),
            ),
          )
          ..transactionsResult = [
            txSummaryFixture(
              minedHeight: null,
              status: const TxStatus.expired(),
              hasMemo: false,
            ),
          ];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.textContaining(l10n.walletActivityExpired), findsOneWidget);
  });

  testWidgets('activity: a failed history read surfaces the honest error line', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
      ),
    )..transactionsThrows = StateError('fake history read failure');
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // No silent failure (invariant 10): an honest line, never a blank section.
    expect(find.text(l10n.walletActivityError), findsOneWidget);
    expect(find.text(l10n.walletActivityTitle), findsOneWidget);
  });

  testWidgets('activity: a WEDGED history read times out to the error line', (
    tester,
  ) async {
    // The hung-FFI mobile edge (blocking-pool starvation): the provider's timeout must
    // convert an endless read into the honest error state, never an infinite spinner.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
      ),
    )..transactionsNeverCompletes = true;
    await tester.pumpWidget(_harness(session: fake));
    await tester
        .pump(); // snapshot resolves → _ActivityHistory mounts, wedged read in-flight.
    // Advance past the honest-degradation bound; the spinner is gone post-error, so
    // pumpAndSettle (not a plain pump) drains the timeout → error rebuild.
    await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletActivityError), findsOneWidget);
  });

  // --- Rescan-recovery surface (FR-1b) ---------------------------------------
  // The screen-level presentation: the overflow entry, the rebuilding banner,
  // the failed notice + dismiss, and the activity-section rebuilding cue. The
  // session swap + outcome routing are pinned in the controller tests; here we
  // pin only what the user SEES.

  /// Override the rescan controller with a fixed state so the surface can be
  /// rendered in any presentation phase without driving a real rescan.
  List<Override> rescanState(WalletRescanState state) => [
    walletRescanControllerProvider.overrideWith(() => _StubRescan(state)),
  ];

  FakeWalletSession activeFake({List<TxSummary> txs = const []}) =>
      FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
          balance: balanceFixture(totalZat: 100000000),
        ),
      )..transactionsResult = txs;

  testWidgets('the rescan entry lives in the app-bar overflow menu and opens '
      'the sheet', (tester) async {
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        // The example wires the appearance seam, so its menu entry stays pinned.
        extraOverrides: [
          walletAppearanceRoutePathProvider.overrideWithValue(
            '/settings/appearance',
          ),
          // Rescan + Security render only under PACKAGE custody (a wired
          // provisioner) — this test's premise is the package-managed wallet.
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The overflow menu is present on an Active wallet (found by its tooltip —
    // the button is privately typed, so byType can't reach it).
    final menu = find.byTooltip(l10n.walletMenuTooltip);
    expect(menu, findsOneWidget);
    await tester.tap(menu);
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletRescanMenuItem), findsOneWidget);
    // Appearance + Security (the example's "extra settings" + the FR-14 custody
    // surface) live in the same menu — a removed/misspelled entry fails here.
    expect(find.text(l10n.walletAppearanceMenuItem), findsOneWidget);
    expect(find.text(l10n.walletSecurityMenuItem), findsOneWidget);

    // Selecting rescan opens the rescan sheet.
    await tester.tap(find.text(l10n.walletRescanMenuItem));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletRescanTitle), findsOneWidget);
    expect(find.text(l10n.walletRescanConfirm), findsOneWidget);
  });

  testWidgets(
    'Appearance stays reachable before a wallet exists (onboarding) — '
    'a bare app-bar action, not only the Active overflow menu',
    (tester) async {
      await tester.pumpWidget(
        _harness(
          // null session ⇒ the onboarding surface; the example wires the seam.
          extraOverrides: [
            walletAppearanceRoutePathProvider.overrideWithValue(
              '/settings/appearance',
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // Pre-Active there is no overflow menu, but the Appearance action persists so
      // the example's rendering settings are reachable during onboarding (the
      // capability the removed home placeholder used to provide).
      expect(find.byTooltip(l10n.walletMenuTooltip), findsNothing);
      expect(find.byTooltip(l10n.walletAppearanceMenuItem), findsOneWidget);
    },
  );

  testWidgets('the appearance entry points are hidden until the host wires the '
      'route seam', (tester) async {
    // Default seam (walletAppearanceRoutePathProvider ⇒ null): a host whose
    // shell owns its own display settings gets NO Appearance affordances.
    // Pre-Active: the bare app-bar palette action does not render.
    await tester.pumpWidget(
      _harness(),
    ); // null session ⇒ the onboarding surface
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    expect(find.byTooltip(l10n.walletAppearanceMenuItem), findsNothing);

    // Active: the overflow menu still opens, but carries no Appearance item.
    // (Unmount first — a rebuilt ProviderScope may not change override count.)
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        // Package custody wired (this test pins the APPEARANCE seam only, so
        // the rescan sentinel it asserts must stay rendered).
        extraOverrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
        ],
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletRescanMenuItem), findsOneWidget);
    expect(find.text(l10n.walletAppearanceMenuItem), findsNothing);
  });

  testWidgets(
    'SESSION-ONLY host (no package provisioner): rescan + Security hide from '
    'the overflow menu; the session-driven entries stay (S151 swap-in seam)',
    (tester) async {
      // A host with its own provisioning overrides walletSessionProvider and
      // leaves walletProvisionerProvider unwired — exactly this harness. The
      // provisioner-routed affordances would silently no-op (rescan →
      // RescanOutcome.notActive → nothing happens), so they must HIDE; the
      // ones that run over the live session must stay.
      await tester.pumpWidget(_harness(session: activeFake()));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRescanMenuItem), findsNothing);
      expect(find.text(l10n.walletSecurityMenuItem), findsNothing);
      // Session-driven entries survive: the Send expert layer and the
      // one-time-address check both run entirely over the WalletSession.
      expect(find.text(l10n.walletMoveMenuItem), findsOneWidget);
      expect(find.text(l10n.walletCheckOneTimeMenuItem), findsOneWidget);
    },
  );

  testWidgets('a Rebuilding state shows the reassuring banner (not a warning)', (
    tester,
  ) async {
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: rescanState(
          WalletRescanRebuilding(target: RescanFromTime(DateTime(2022, 6))),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The "from {month year}" banner explains the briefly-empty balance/activity.
    final monthYear = MaterialLocalizations.of(
      tester.element(find.byType(WalletScreen)),
    ).formatMonthYear(DateTime(2022, 6));
    expect(
      find.text(l10n.walletRescanRebuildingFrom(monthYear)),
      findsOneWidget,
    );
  });

  testWidgets('a full-history Rebuilding shows the scan-all banner copy', (
    tester,
  ) async {
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: rescanState(
          const WalletRescanRebuilding(target: RescanAllHistory()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletRescanRebuildingAll), findsOneWidget);
  });

  testWidgets(
    "the wallet-start DEFAULT Rebuilding names the wallet's own start "
    '(#317 -- the banner says which recovery the user chose)',
    (tester) async {
      await tester.pumpWidget(
        _harness(
          session: activeFake(),
          extraOverrides: rescanState(
            const WalletRescanRebuilding(
              target: RescanFromWalletBirthday(2400000),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(find.text(l10n.walletRescanRebuildingDefault), findsOneWidget);
    },
  );

  testWidgets(
    'while Rebuilding, the empty activity shows the rebuilding cue, NOT '
    '"no activity yet"',
    (tester) async {
      await tester.pumpWidget(
        _harness(
          session: activeFake(), // empty txs
          extraOverrides: rescanState(
            const WalletRescanRebuilding(target: RescanAllHistory()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(find.text(l10n.walletActivityRebuilding), findsOneWidget);
      expect(
        find.text(l10n.walletActivityEmpty),
        findsNothing,
        reason: 'a temporarily-empty history must not read as lost',
      );
    },
  );

  testWidgets('activity: "Load more" fetches + APPENDS the next keyset page (51st tx '
      'reachable)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
      ),
    );
    fake.pagesByCursor[null] = HistoryPage(
      rows: [
        txSummaryFixture(
          txidHex:
              '1100000000000000000000000000000000000000000000000000000000000001',
        ),
      ],
      nextCursor: 'page-2-cursor',
    );
    fake.pagesByCursor['page-2-cursor'] = HistoryPage(
      rows: [
        txSummaryFixture(
          txidHex:
              '2200000000000000000000000000000000000000000000000000000000000002',
        ),
      ],
      nextCursor: null,
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // Page 1 is shown WITH a "Load more" affordance (a next cursor exists).
    expect(find.text(l10n.walletActivityLoadMore), findsOneWidget);

    // Tapping it pages the keyset cursor and APPENDS page 2; no more pages after.
    await tester.tap(find.text(l10n.walletActivityLoadMore));
    await tester.pumpAndSettle();
    expect(
      fake.lastTransactionsAfter,
      'page-2-cursor',
      reason: 'paged with the page-1 cursor',
    );
    expect(
      find.text(l10n.walletActivityLoadMore),
      findsNothing,
      reason: 'last page reached ⇒ no more affordance',
    );
  });

  testWidgets('a Failed rescan shows the honest notice; Dismiss clears it', (
    tester,
  ) async {
    final stub = _StubRescan(const WalletRescanFailed());
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: [
          walletRescanControllerProvider.overrideWith(() => stub),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletRescanFailedNotice), findsOneWidget);
    await tester.tap(find.text(l10n.walletRescanFailedDismiss));
    await tester.pumpAndSettle();

    expect(stub.dismissCount, 1);
    expect(find.text(l10n.walletRescanFailedNotice), findsNothing);
  });

  testWidgets(
    'a settling-send-BLOCKED rescan shows the hours-scale fence notice, not '
    'the "moment" copy; Dismiss clears it (#364 N2 — the render if-chain is '
    'non-exhaustive, so this arm needs its own mirror of the Failed test)',
    (tester) async {
      final stub = _StubRescan(const WalletRescanBlockedBySettlingSend());
      await tester.pumpWidget(
        _harness(
          session: activeFake(),
          extraOverrides: [
            walletRescanControllerProvider.overrideWith(() => stub),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(find.text(l10n.walletRescanBlockedSettlingNotice), findsOneWidget);
      expect(
        find.text(l10n.walletRescanFailedNotice),
        findsNothing,
        reason:
            'the "try again in a moment" copy would invite retry churn '
            'against a refusal that holds until the send settles',
      );
      await tester.tap(find.text(l10n.walletRescanFailedDismiss));
      await tester.pumpAndSettle();

      expect(stub.dismissCount, 1);
      expect(find.text(l10n.walletRescanBlockedSettlingNotice), findsNothing);
    },
  );

  testWidgets('a disk-full rescan shows the actionable free-up-space notice '
      '(#375), not the "moment" copy; Dismiss clears it', (tester) async {
    final stub = _StubRescan(const WalletRescanFailedNeedsSpace());
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: [
          walletRescanControllerProvider.overrideWith(() => stub),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletRescanNeedsSpaceNotice), findsOneWidget);
    expect(
      find.text(l10n.walletRescanFailedNotice),
      findsNothing,
      reason: 'the generic "try again in a moment" copy must not render',
    );
    await tester.tap(find.text(l10n.walletRescanFailedDismiss));
    await tester.pumpAndSettle();

    expect(stub.dismissCount, 1);
    expect(find.text(l10n.walletRescanNeedsSpaceNotice), findsNothing);
  });

  testWidgets('the rescan FAILURE notices never claim "unchanged" (#379)', (
    tester,
  ) async {
    // A fault AFTER the rebuild's atomic rename recovers into the REBUILT
    // lower-birthday wallet — balance/history empty until sync repopulates
    // them — and the Dart side cannot tell that arm from the common
    // unchanged one (same error kinds). So the two failure notices may claim
    // funds-safety, never "unchanged". (The settling-send notice is exempt:
    // its fence refuses BEFORE any rebuild starts, so "unchanged" is true.)
    final stub = _StubRescan(const WalletRescanFailed());
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: [
          walletRescanControllerProvider.overrideWith(() => stub),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    for (final notice in [
      l10n.walletRescanFailedNotice,
      l10n.walletRescanNeedsSpaceNotice,
    ]) {
      expect(
        notice.toLowerCase(),
        isNot(contains('unchanged')),
        reason: 'false on the post-rename fault arms',
      );
      expect(
        notice,
        contains('funds are safe'),
        reason: 'the claim that IS true on every arm',
      );
    }
  });

  // ── The durable catch-up cue (#380) ─────────────────────────────────────────
  // The rescan controller is process-lifetime memory; these pin that the
  // SURFACE explanation survives the states where it has forgotten — a
  // relaunch mid-catch-up and a failure-notice dismiss — via the durable pair
  // (lastSynced == null + below tip), and stays away from wallets that have
  // synced before.

  const midCatchUp = SyncStatus.scanning(
    from: 2_000_000,
    to: 2_500_000,
    percent: 0.4, // determinate (a 0 fraction animates → pumpAndSettle hangs)
    spendableReady: false,
    rewound: false,
  );

  FakeWalletSession neverSyncedFake({SyncStamp? lastSynced}) =>
      FakeWalletSession(
        current: midCatchUp,
        snapshotValue: walletStateFixture(
          syncStatus: midCatchUp,
          lastSynced: lastSynced,
        ),
      );

  testWidgets('RELAUNCH mid-catch-up (#380): a never-synced wallet below tip '
      'shows the generic banner + activity cue from durable signals alone — '
      'no in-memory rescan state', (tester) async {
    // No rescan-controller override: the controller is at its built-in Idle,
    // exactly as after a process death (app update / LMK kill mid-catch-up).
    await tester.pumpWidget(_harness(session: neverSyncedFake()));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletCatchUpBanner), findsOneWidget);
    expect(find.text(l10n.walletActivityCatchingUp), findsOneWidget);
    expect(
      find.text(l10n.walletActivityEmpty),
      findsNothing,
      reason: 'a temporarily-empty history must not read as lost',
    );
  });

  testWidgets('the durable cue CLEARS on reached-tip (#380) — the same edge '
      'that clears the controller', (tester) async {
    final fake = neverSyncedFake();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    expect(find.text(l10n.walletCatchUpBanner), findsOneWidget);

    fake.push(const SyncStatus.upToDate(tip: 2_500_000));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletCatchUpBanner), findsNothing);
    expect(find.text(l10n.walletActivityCatchingUp), findsNothing);
  });

  testWidgets('a PREVIOUSLY-synced wallet mid-scan shows NO catch-up cue '
      '(#380) — routine scans stay uncluttered', (tester) async {
    await tester.pumpWidget(
      _harness(
        session: neverSyncedFake(
          lastSynced: const SyncStamp(height: 2_400_000, at: 0),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletCatchUpBanner), findsNothing);
    expect(find.text(l10n.walletActivityCatchingUp), findsNothing);
    expect(find.text(l10n.walletActivityEmpty), findsOneWidget);
  });

  testWidgets('DISMISS below tip keeps an explanation on screen (#380 (b)): '
      'the notice hands over to the durable banner, and the activity cue '
      'holds through both', (tester) async {
    final stub = _StubRescan(const WalletRescanFailed());
    await tester.pumpWidget(
      _harness(
        session: neverSyncedFake(),
        extraOverrides: [
          walletRescanControllerProvider.overrideWith(() => stub),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // Pre-dismiss: the notice is the explaining surface (no banner stacking),
    // but the activity section already carries the catch-up cue — never the
    // contradicting "No activity yet" beside a failure notice.
    expect(find.text(l10n.walletRescanFailedNotice), findsOneWidget);
    expect(find.text(l10n.walletCatchUpBanner), findsNothing);
    expect(find.text(l10n.walletActivityCatchingUp), findsOneWidget);
    expect(find.text(l10n.walletActivityEmpty), findsNothing);

    await tester.tap(find.text(l10n.walletRescanFailedDismiss));
    await tester.pumpAndSettle();

    // Post-dismiss: the durable banner takes over seamlessly — the emptied
    // wallet is never left unexplained (the P1 #4 misread).
    expect(find.text(l10n.walletRescanFailedNotice), findsNothing);
    expect(find.text(l10n.walletCatchUpBanner), findsOneWidget);
    expect(find.text(l10n.walletActivityCatchingUp), findsOneWidget);
  });

  testWidgets('RELAUNCH mid-REBUILD (#377 s357b-2): the durable breadcrumb '
      'upgrades the banner to the rescan-naming copy — the relaunch no longer '
      'demotes "your rescan is rebuilding" to the first-run framing', (
    tester,
  ) async {
    // No rescan-controller override (post-death Idle); the breadcrumb is the
    // only rescan evidence. everSynced false = the core cleared it at rescan.
    await tester.pumpWidget(
      _harness(
        session: FakeWalletSession(
          current: midCatchUp,
          snapshotValue: walletStateFixture(
            syncStatus: midCatchUp,
            rescanRebuilding: true,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletCatchUpRescanBanner), findsOneWidget);
    expect(
      find.text(l10n.walletCatchUpBanner),
      findsNothing,
      reason: 'the breadcrumb names the rescan — no generic demotion',
    );
    // The activity-section note upgrades on the same cue: the Rebuilding copy
    // (named recovery family), not the generic catching-up line.
    expect(find.text(l10n.walletActivityRebuilding), findsOneWidget);
    expect(find.text(l10n.walletActivityCatchingUp), findsNothing);
  });

  testWidgets('the catch-up banner is a live region (#377 s357b-1): its '
      'appearance is ANNOUNCED to screen readers — a balance that visibly '
      'empties/refills is a money event a blind user must not miss', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(_harness(session: neverSyncedFake()));
    await tester.pumpAndSettle();

    expect(
      tester
          .getSemantics(find.byKey(const Key('wallet-catchup-banner')))
          .flagsCollection
          .isLiveRegion,
      isTrue,
      reason:
          'the liveRegion convention on meaningful money-surface state '
          '(the _SyncBadge / arrival-SnackBar precedent)',
    );
    handle.dispose();
  });

  // ── the sync-off (disabledByHost) gates on the wallet surface ─────
  // The drive reaches disabledByHost through the REAL controller via the
  // policy seam (the R1 idiom from wallet_sync_controller_test) — no stub
  // notifier, so these pin the wired path end-to-end.

  testWidgets('sync-off (S205-b/S205-c): the catch-up banner AND the activity '
      'cue are suppressed — and the activity section renders the SYNC-OFF '
      'note, never "No activity yet" over history that exists', (tester) async {
    // The same durable-cue wallet as the #380 group (never synced, below
    // tip) — the state that WOULD show both — but the host's sync policy is
    // off, so "still catching up… will show up here" would claim a progress
    // that cannot happen. (UX HIGH) closed the fall-through to
    // the plain empty state too: "No activity yet" is a flat lie over a
    // wiped/never-synced history that exists but cannot repopulate — the
    // honest arm names BOTH the pending fill and the way back.
    await tester.pumpWidget(
      _harness(
        session: neverSyncedFake(),
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(find.text(l10n.walletCatchUpBanner), findsNothing);
    expect(find.text(l10n.walletActivityCatchingUp), findsNothing);
    expect(
      find.text(l10n.walletActivitySyncNotRunning),
      findsOneWidget,
      reason: 'names the pending fill AND the way back (S205-c UX HIGH)',
    );
    expect(
      find.text(l10n.walletActivityEmpty),
      findsNothing,
      reason: '"No activity yet" would be a flat lie over unscanned history',
    );
    // The badge tells the honest story instead.
    expect(find.text(l10n.walletSyncDisabled), findsOneWidget);
  });

  testWidgets('sync-off (S205-b/S205-c): the standing Rebuilding banner is '
      'ABSENT and the activity section renders the SYNC-OFF note — the '
      'rebuild only fills via the loop the host turned off, and a wiped '
      'history must not read "No activity yet"', (tester) async {
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: [
          ...rescanState(
            const WalletRescanRebuilding(target: RescanAllHistory()),
          ),
          walletSyncPolicyProvider.overrideWithValue(false),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(
      find.text(l10n.walletRescanRebuildingAll),
      findsNothing,
      reason:
          '"Rebuilding your history" claims active recovery — false while '
          'nothing runs; the banner returns when the policy flips back on',
    );
    expect(find.text(l10n.walletActivityRebuilding), findsNothing);
    expect(
      find.text(l10n.walletActivitySyncNotRunning),
      findsOneWidget,
      reason:
          'the honest replacement (S205-c UX HIGH): the wiped history exists '
          'but cannot repopulate until the host re-enables sync',
    );
    expect(find.text(l10n.walletActivityEmpty), findsNothing);
  });

  testWidgets('sync-off (S205-b): the disabled-Send reason is the sync-off '
      'copy — never "synced but empty" two widgets under a "Sync off" badge', (
    tester,
  ) async {
    // Retained UpToDate + zero spendable: without the gate this reads the
    // walletSendNoSpendableYet arm; the drive truth must win.
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(), // spendable 0 ⇒ Send disabled, reason shows
      ),
    );
    await tester.pumpWidget(
      _harness(
        session: fake,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(_sendInk(tester).onTap, isNull);
    expect(find.text(l10n.walletSendSyncNotRunning), findsOneWidget);
    expect(find.text(l10n.walletSendNoSpendableYet), findsNothing);
    expect(find.text(l10n.walletSendWaitingForFunds), findsNothing);
  });

  testWidgets('sync-off (S205-b): the rescan and check-older-swap-addresses '
      'menu entries are DISABLED — neither can ever complete without the '
      'sync loop (rescan would wipe the balance behind a dead promise)', (
    tester,
  ) async {
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: [
          // Package custody so the rescan entry renders at all.
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
          walletSyncPolicyProvider.overrideWithValue(false),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
    await tester.pumpAndSettle();

    // Disabled, not hidden — the entries return when the policy flips on.
    final rescanItemFinder = find.ancestor(
      of: find.text(l10n.walletRescanMenuItem),
      matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
    );
    final rescanItem = tester.widget(rescanItemFinder) as PopupMenuItem;
    expect(rescanItem.enabled, isFalse, reason: 'rescan needs the sync loop');
    final deepScanItemFinder = find.ancestor(
      of: find.text(l10n.walletDeepScanMenuItem),
      matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
    );
    final deepScanItem = tester.widget(deepScanItemFinder) as PopupMenuItem;
    expect(
      deepScanItem.enabled,
      isFalse,
      reason: 'the widened ranges are scanned by the loop the host turned off',
    );

    // the disable SAYS WHY — each grey row carries the sync-off hint
    // sub-line (the explaining badge is occluded behind the open menu, and a
    // screen reader would otherwise hear only "dimmed").
    expect(find.text(l10n.walletMenuSyncNotRunningHint), findsNWidgets(2));
    expect(
      find.descendant(
        of: rescanItemFinder,
        matching: find.text(l10n.walletMenuSyncNotRunningHint),
      ),
      findsOneWidget,
      reason: 'the rescan row explains its own disable',
    );
    expect(
      find.descendant(
        of: deepScanItemFinder,
        matching: find.text(l10n.walletMenuSyncNotRunningHint),
      ),
      findsOneWidget,
      reason: 'the deep-scan row explains its own disable',
    );
  });

  testWidgets('sync-off (S205-c): a deep scan already in flight keeps its '
      '"Checking…" label precedence — the sync-off hint renders only under '
      'the rescan row', (tester) async {
    final fake = activeFake()..checkOlderSwapAddressesNeverCompletes = true;
    await tester.pumpWidget(
      _harness(
        session: fake,
        extraOverrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
          walletSyncPolicyProvider.overrideWithValue(false),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    final container = ProviderScope.containerOf(
      tester.element(find.byType(WalletScreen)),
    );

    // Arm the in-flight latch directly (the sheet flow is pinned in
    // swap_deep_scan_test); the wedged check parks the latch true.
    unawaited(
      container
          .read(swapDeepScanInFlightProvider.notifier)
          .check()
          .then((_) {}, onError: (Object _) {}),
    );
    await tester.pump();

    await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
    await tester.pumpAndSettle();

    expect(
      find.text(l10n.walletDeepScanChecking),
      findsOneWidget,
      reason: 'the in-flight label keeps precedence over the sync-off hint',
    );
    expect(find.text(l10n.walletDeepScanMenuItem), findsNothing);
    expect(
      find.text(l10n.walletMenuSyncNotRunningHint),
      findsOneWidget,
      reason: 'only the rescan row carries the hint here',
    );
    expect(
      find.descendant(
        of: find.ancestor(
          of: find.text(l10n.walletRescanMenuItem),
          matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
        ),
        matching: find.text(l10n.walletMenuSyncNotRunningHint),
      ),
      findsOneWidget,
    );

    // Drain the wedge-timeout timer the direct check scheduled (the A1 bound
    // fires, resets the latch, and the fake clock is left clean).
    await tester.pumpAndSettle(
      walletFfiWedgeTimeout + const Duration(seconds: 1),
    );
  });

  testWidgets('the sync-off menu hint renders ONLY under policy-off (S205-c) '
      '— the default-policy menu carries no hint line', (tester) async {
    await tester.pumpWidget(
      _harness(
        session: activeFake(),
        extraOverrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletMenuSyncNotRunningHint), findsNothing);
    expect(find.text(l10n.walletRescanMenuItem), findsOneWidget);
    expect(find.text(l10n.walletDeepScanMenuItem), findsOneWidget);
  });

  // ── #405: the SAME gates under a FAILED START ────────────────────────────
  // The gate family used to read the host POLICY (via the drive's
  // disabledByHost arm). A start command that FAILED leaves the policy reading
  // TRUE while the loop never ran, so every gate below was OPEN on exactly the
  // wallet that could not honour it — the rescan entry enabled over a wipe with
  // no rebuild behind it, the activity section claiming a catch-up, the Send
  // reason claiming progress. These pin the SSOT reading, driven through the
  // REAL controller off a throwing startSync (no stub drive).

  testWidgets('#405: a FAILED start disables the rescan and deep-scan menu '
      'entries and explains each one — the policy still reads ON, so only the '
      'SSOT closes this door', (tester) async {
    await tester.pumpWidget(
      _harness(
        session: activeFake()..failStart = true,
        extraOverrides: [
          walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
          // NO policy override — the host policy is ON. That is the point.
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    final container = ProviderScope.containerOf(
      tester.element(find.byType(WalletScreen)),
    );
    expect(
      container.read(walletSyncControllerProvider),
      WalletSyncDrive.failed,
      reason: 'precondition — the start threw',
    );
    expect(
      container.read(walletSyncPolicyProvider),
      isTrue,
      reason: 'precondition — the POLICY is ON; the old read saw only this',
    );

    await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
    await tester.pumpAndSettle();

    final rescanItemFinder = find.ancestor(
      of: find.text(l10n.walletRescanMenuItem),
      matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
    );
    expect(
      (tester.widget(rescanItemFinder) as PopupMenuItem).enabled,
      isFalse,
      reason:
          'THE POINT: before #405 this was ENABLED — a confirm here wiped the '
          'DB and rebuilt against a loop that never started',
    );
    final deepScanItemFinder = find.ancestor(
      of: find.text(l10n.walletDeepScanMenuItem),
      matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
    );
    expect(
      (tester.widget(deepScanItemFinder) as PopupMenuItem).enabled,
      isFalse,
      reason: 'the widened ranges are scanned by a pass that will not run',
    );
    // Each grey row still SAYS WHY — and cause-agnostically, since
    // "turn syncing on in settings" is the wrong instruction here.
    expect(find.text(l10n.walletMenuSyncNotRunningHint), findsNWidgets(2));
  });

  testWidgets('#405: a FAILED start renders the sync-not-running activity '
      'note and suppresses the catch-up banner — never "Still catching up" '
      'over a loop that never started', (tester) async {
    await tester.pumpWidget(
      _harness(session: neverSyncedFake()..failStart = true),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    final container = ProviderScope.containerOf(
      tester.element(find.byType(WalletScreen)),
    );
    expect(
      container.read(walletSyncControllerProvider),
      WalletSyncDrive.failed,
      reason: 'precondition',
    );

    expect(
      find.text(l10n.walletCatchUpBanner),
      findsNothing,
      reason: 'an active-progress claim over a loop that is not running',
    );
    expect(find.text(l10n.walletActivityCatchingUp), findsNothing);
    expect(
      find.text(l10n.walletActivitySyncNotRunning),
      findsOneWidget,
      reason:
          'the honest arm: the fill is pending on a pass that is not running',
    );
    expect(
      find.text(l10n.walletActivityEmpty),
      findsNothing,
      reason: '"No activity yet" is a flat lie over unscanned history',
    );
    // The start-failed notice is the surface that owns the CAUSE + the remedy,
    // which is exactly why the copy above states only the condition.
    expect(find.text(l10n.walletSyncRetry), findsOneWidget);
  });

  testWidgets('#405: a FAILED start gives the disabled Send the '
      'sync-not-running reason — never "Still syncing" beside a retry notice '
      'saying syncing could not start', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(), // spendable 0 ⇒ Send disabled, reason shows
      ),
    )..failStart = true;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    expect(_sendInk(tester).onTap, isNull);
    expect(find.text(l10n.walletSendSyncNotRunning), findsOneWidget);
    expect(
      find.text(l10n.walletSendNoSpendableYet),
      findsNothing,
      reason:
          'THE POINT: before #405 the retained UpToDate arm won here and told '
          'the user their wallet was synced-but-empty',
    );
    expect(find.text(l10n.walletSendWaitingForFunds), findsNothing);
  });

  testWidgets('sync-off (S205-b): the deep-scan "still checking" banner is '
      'suppressed while the drive is disabledByHost — and returns when the '
      'policy flips back on (the cue state survives)', (tester) async {
    final policySwitch = StateProvider<bool>((ref) => true);
    // Default fake coverage reports pending 63 > 0 — the armed shape.
    final fake = activeFake();
    await tester.pumpWidget(
      _harness(
        session: fake,
        extraOverrides: [
          walletSyncPolicyProvider.overrideWith(
            (ref) => ref.watch(policySwitch),
          ),
        ],
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    final container = ProviderScope.containerOf(
      tester.element(find.byType(WalletScreen)),
    );

    // Arm the cue (a scan ran this session) — the banner shows.
    container.read(swapDeepScanProgressProvider.notifier).markRan();
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletDeepScanBannerChecking), findsOneWidget);

    // Policy off: "still checking…" would read "working" forever.
    container.read(policySwitch.notifier).state = false;
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletDeepScanBannerChecking), findsNothing);

    // Back on: the providers were untouched — the banner returns.
    container.read(policySwitch.notifier).state = true;
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletDeepScanBannerChecking), findsOneWidget);
  });

  // --- fixed-height sync badge + tap-through detail sheets ------------

  // The badge is private by design; the predicate finder is the established
  // way to anchor layout assertions on it without widening the API.
  Finder syncBadge() =>
      find.byWidgetPredicate((w) => w.runtimeType.toString() == '_SyncBadge');

  /// Pump the wallet at [status] over [tor] and return the badge height, or
  /// null when the bar is HIDDEN (FR-49 S12, C2).
  Future<double?> pumpBadgeHeight(
    WidgetTester tester,
    SyncStatus status, {
    TorState tor = _cautionTor,
  }) async {
    final fake = FakeWalletSession(
      current: status,
      snapshotValue: walletStateFixture(syncStatus: status, tor: tor),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final badge = syncBadge();
    return badge.evaluate().isEmpty ? null : tester.getSize(badge).height;
  }

  testWidgets('S150: the sync badge keeps ONE height across every state — the '
      'no-layout-shift guarantee', (tester) async {
    // FR-49 S12 C2: over a neutral transport a fresh UpToDate HIDES the bar,
    // so the height guarantee is measured over a CAUTION transport, where the
    // bar shows in every state — the same four states, the same comparison.
    const upToDateStatus = SyncStatus.upToDate(tip: 100);
    // The three arms that USED to grow the card: scanning (bar + detail),
    // stalled (reason line), offline (next-step line).
    const scanningStatus = SyncStatus.scanning(
      from: 1,
      to: 2000,
      percent: 0.42,
      spendableReady: true,
      rewound: false,
    );
    const stalledStatus = SyncStatus.stalled(
      reason: StallReason.endpointUnreachable,
    );
    const offlineStatus = SyncStatus.offline();

    final upToDate = (await pumpBadgeHeight(tester, upToDateStatus))!;
    final scanning = await pumpBadgeHeight(tester, scanningStatus);
    final stalled = await pumpBadgeHeight(tester, stalledStatus);
    final offline = await pumpBadgeHeight(tester, offlineStatus);

    // And over the NEUTRAL transport (Tor off — the everyday host): every
    // non-healthy state still shows the bar at that same one height; only the
    // healthy UpToDate hides it, and then the overflow menu carries the
    // sync-status entry so the sheet stays one tap away.
    for (final s in [scanningStatus, stalledStatus, offlineStatus]) {
      expect(
        await pumpBadgeHeight(tester, s, tor: const TorState.off()),
        upToDate,
        reason: '$s is not healthy: the bar shows, at the one height',
      );
    }
    expect(
      await pumpBadgeHeight(tester, upToDateStatus, tor: const TorState.off()),
      isNull,
      reason: 'synced and healthy over a neutral transport: the bar hides',
    );
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    expect(find.text(l10n.walletSyncUpToDate), findsNothing);
    await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
    await tester.pumpAndSettle();
    expect(
      find.text(l10n.walletSyncUpToDate),
      findsOneWidget,
      reason: 'the hidden bar\'s headline is the overflow menu\'s entry',
    );

    expect(
      scanning,
      upToDate,
      reason: 'the scan bar + count must ride IN the row, not grow it',
    );
    expect(
      stalled,
      upToDate,
      reason: 'the stall reason moved to the sheet, not the badge',
    );
    expect(
      offline,
      upToDate,
      reason: 'the offline next-step moved to the sheet, not the badge',
    );
  });

  testWidgets(
    'S150: the scan bar rides IN the badge row; the ⓘ cue shows in every '
    'state',
    (tester) async {
      // FR-49 S12 C2: a CAUTION transport, so the bar stays up through the
      // push to UpToDate below (over a neutral transport it would hide there,
      // and the ⓘ with it — covered by the S12 sync-bar tests).
      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 1,
          to: 2000,
          percent: 0.42,
          spendableReady: false,
          rewound: false,
        ),
        snapshotValue: walletStateFixture(tor: _cautionTor),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();

      expect(
        find.descendant(
          of: syncBadge(),
          matching: find.byType(LinearProgressIndicator),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: syncBadge(),
          matching: find.byIcon(Icons.info_outline),
        ),
        findsOneWidget,
      );

      fake.push(const SyncStatus.upToDate(tip: 2000));
      await tester.pumpAndSettle();
      expect(
        find.descendant(
          of: syncBadge(),
          matching: find.byType(LinearProgressIndicator),
        ),
        findsNothing,
      );
      // The details affordance is persistent — not a scanning-only cue.
      expect(
        find.descendant(
          of: syncBadge(),
          matching: find.byIcon(Icons.info_outline),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets('S150: tapping the badge opens the LIVE sync-detail sheet', (
    tester,
  ) async {
    // FR-49 S12 C2: a CAUTION transport keeps the bar up at UpToDate (over a
    // neutral one it hides, and the sheet opens from the overflow menu — the
    // S12 sync-bar tests pin that path).
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1579873),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1579873),
        tor: _cautionTor,
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    await tester.tap(syncBadge());
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSyncExplainUpToDate), findsOneWidget);
    expect(find.text(l10n.walletSyncSheetSyncedTo), findsOneWidget);
    // The EXACT number belongs to the sheet (the badge keeps compact forms).
    expect(find.text('1,579,873'), findsOneWidget);

    // LIVE while open: a status change re-renders the sheet, figures included.
    fake.push(
      const SyncStatus.scanning(
        from: 1000,
        to: 3000,
        percent: 0.5,
        spendableReady: true,
        rewound: false,
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSyncExplainScanning), findsOneWidget);
    expect(find.text(l10n.walletSyncSheetProgress), findsOneWidget);
    expect(find.text('50%'), findsOneWidget);
    expect(find.text(l10n.walletSyncSheetBlocksLeft), findsOneWidget);
    expect(find.text('2,000'), findsOneWidget);
    // Spend-before-sync stays visible in the sheet (it left the badge row).
    expect(find.text(l10n.walletSyncSpendableReady), findsOneWidget);
  });

  testWidgets('S150: the sheet explains a stall with its typed reason (which the '
      'fixed badge row no longer shows)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(reason: StallReason.torUnavailable),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The reason is NOT a visible Text before the sheet opens (a11y label only).
    expect(find.text(l10n.walletStallTor), findsNothing);

    await tester.tap(syncBadge());
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSyncExplainStalled), findsOneWidget);
    expect(find.text(l10n.walletStallTor), findsOneWidget);
  });

  testWidgets('S150 a11y: the badge is ONE labeled button node with a tap action '
      '(excludeSemantics would otherwise drop the InkWell)', (tester) async {
    final handle = tester.ensureSemantics();
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(reason: StallReason.torUnavailable),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // The label still carries the detail line the fixed row no longer draws
    // (+ the transport label the row shows only as an icon).
    final expected =
        '${l10n.walletSyncStalled}. ${l10n.walletTorOff}. ${l10n.walletStallTor}';
    expect(find.bySemanticsLabel(expected), findsOneWidget);
    expect(
      tester.getSemantics(find.bySemanticsLabel(expected)),
      isSemantics(isButton: true, hasTapAction: true),
    );
    handle.dispose();
  });

  testWidgets(
    'S150: tapping an activity row opens the transaction-detail sheet',
    (tester) async {
      final fake = activeFake(
        txs: [txSummaryFixture(netAmountZat: -250000, feeZat: 10000)],
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      await tester.ensureVisible(find.text(l10n.walletActivitySent));
      await tester.tap(find.text(l10n.walletActivitySent));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletTxExplainConfirmed), findsOneWidget);
      expect(find.text(l10n.walletTxDetailFee), findsOneWidget);
      expect(find.text(l10n.walletAmount(formatZec(10000))), findsOneWidget);
      expect(find.text(l10n.walletTxDetailHeight), findsOneWidget);
      expect(find.text(l10n.walletTxDetailMemo), findsOneWidget);
      expect(find.text(l10n.walletTxDetailCopyTxid), findsOneWidget);
    },
  );

  testWidgets(
    'S150: an expired row reads as cancelled — struck amount, funds-kept '
    'a11y, prominent sheet banner',
    (tester) async {
      final handle = tester.ensureSemantics();
      final fake = activeFake(
        txs: [
          txSummaryFixture(
            minedHeight: null,
            status: const TxStatus.expired(),
            netAmountZat: -15000,
            hasMemo: false,
          ),
        ],
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // Visual: the amount is struck through and muted ("didn't happen").
      // FR-49 S12 C5: the row's outgoing figure carries U+2212.
      final amountText = _outgoingAmount(l10n, -15000);
      final amount = tester.widget<Text>(find.text(amountText));
      expect(amount.style?.decoration, TextDecoration.lineThrough);

      // A11y: a strikethrough is silent to screen readers — the label says it.
      expect(
        find.bySemanticsLabel(RegExp(RegExp.escape(l10n.walletTxFundsKept))),
        findsOneWidget,
      );

      await tester.ensureVisible(find.text(amountText));
      await tester.tap(find.text(amountText));
      await tester.pumpAndSettle();

      // The sheet leads with the reassurance banner + the plain-language why.
      expect(find.text(l10n.walletTxFundsKept), findsOneWidget);
      expect(find.text(l10n.walletTxExplainExpired), findsOneWidget);
      handle.dispose();
    },
  );

  testWidgets('S150: a failed row gets the same funds-kept treatment', (
    tester,
  ) async {
    final fake = activeFake(
      txs: [
        txSummaryFixture(
          minedHeight: null,
          status: const TxStatus.failed(),
          netAmountZat: -15000,
          hasMemo: false,
        ),
      ],
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));

    // FR-49 S12 C5: the row's outgoing figure carries U+2212.
    final amountText = _outgoingAmount(l10n, -15000);
    await tester.ensureVisible(find.text(amountText));
    await tester.tap(find.text(amountText));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletTxFundsKept), findsOneWidget);
    expect(find.text(l10n.walletTxExplainFailed), findsOneWidget);
  });

  testWidgets(
    'S150 regression: the ⓘ is PINNED to the badge edge in every state '
    '(loose-flex dead space must never leak past it)',
    (tester) async {
      // The UX review measured the ⓘ drifting 78–251px off the right edge when
      // the headline's unused loose-flex allotment leaked past the trailing
      // group. Pin: the gap from the ⓘ to the badge's right edge stays constant
      // and small across states.
      //
      // FR-49 S12 C2: over a CAUTION transport, so the bar (and its ⓘ) is up
      // in every state measured, UpToDate included.
      Future<double> infoGap(SyncStatus status) async {
        final fake = FakeWalletSession(
          current: status,
          snapshotValue: walletStateFixture(
            syncStatus: status,
            tor: _cautionTor,
          ),
        );
        await tester.pumpWidget(_harness(session: fake));
        await tester.pumpAndSettle();
        final badgeRight = tester.getTopRight(syncBadge()).dx;
        final iconRight = tester
            .getTopRight(
              find.descendant(
                of: syncBadge(),
                matching: find.byIcon(Icons.info_outline),
              ),
            )
            .dx;
        return badgeRight - iconRight;
      }

      final upToDate = await infoGap(const SyncStatus.upToDate(tip: 100));
      final scanning = await infoGap(
        const SyncStatus.scanning(
          from: 1,
          to: 2000,
          percent: 0.42,
          spendableReady: false,
          rewound: false,
        ),
      );
      final offline = await infoGap(const SyncStatus.offline());

      // Constant across states (no horizontal jump on a state flip)...
      expect(scanning, upToDate);
      expect(offline, upToDate);
      // ...and actually AT the edge (padding-scale, not dead-space-scale).
      expect(upToDate, lessThan(20));
    },
  );

  testWidgets(
    'S150 regression: a scanning badge with a deep span survives a NARROW '
    'screen at an accessibility text scale without overflowing',
    (tester) async {
      // The UX review measured a 204px right overflow at 320dp/textScale 2.0
      // when the blocks-left chip rode the row. The chip moved to the a11y
      // label + sheet; this pins the fix (flutter_test fails this test on any
      // RenderFlex overflow).
      tester.view.physicalSize = const Size(320 * 3, 800 * 3);
      tester.view.devicePixelRatio = 3.0;
      addTearDown(tester.view.reset);

      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 1400000,
          to: 2900000,
          percent: 0.42,
          spendableReady: true,
          rewound: false,
        ),
        snapshotValue: walletStateFixture(),
      );
      await tester.pumpWidget(_harness(session: fake, textScale: 2.0));
      await tester.pumpAndSettle();

      expect(syncBadge(), findsOneWidget);
      expect(
        find.descendant(
          of: syncBadge(),
          matching: find.byIcon(Icons.info_outline),
        ),
        findsOneWidget,
      );
      // The transport shield must SURVIVE the same narrow/AX squeeze —
      // pinned, not incidental (review N3).
      expect(
        find.descendant(
          of: syncBadge(),
          matching: find.byIcon(Icons.shield_outlined),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'S150: the OPEN tx sheet follows a status change (pending → expired) '
    'instead of lying with a stale snapshot',
    (tester) async {
      final pending = txSummaryFixture(
        minedHeight: null,
        status: const TxStatus.pending(),
        netAmountZat: -15000,
        hasMemo: false,
      );
      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 1,
          to: 100,
          percent: 0.5,
          spendableReady: true,
          rewound: false,
        ),
        snapshotValue: walletStateFixture(),
      )..transactionsResult = [pending];
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // FR-49 S12 C5: the row's outgoing figure carries U+2212.
      final amountText = _outgoingAmount(l10n, -15000);
      // ensureVisible only MOVES the scroll offset; the frame that lays the
      // row out at its new position must be pumped before the tap, or the tap
      // lands where the (taller, S12) row sat before the scroll.
      await tester.ensureVisible(find.text(amountText));
      await tester.pumpAndSettle();
      await tester.tap(find.text(amountText));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletTxExplainPending), findsOneWidget);
      expect(find.text(l10n.walletTxFundsKept), findsNothing);

      // The tx expires; reaching the tip refreshes the activity list (the
      // screen's existing sync-edge listener) and the OPEN sheet re-resolves
      // its row by txid.
      fake.transactionsResult = [
        txSummaryFixture(
          txidHex: pending.txidHex,
          minedHeight: null,
          status: const TxStatus.expired(),
          netAmountZat: -15000,
          hasMemo: false,
        ),
      ];
      fake.push(const SyncStatus.upToDate(tip: 100));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletTxExplainExpired), findsOneWidget);
      expect(find.text(l10n.walletTxFundsKept), findsOneWidget);
      expect(find.text(l10n.walletTxExplainPending), findsNothing);
    },
  );

  testWidgets(
    '#392: an incoming-funds live event refreshes the activity list AT ONCE — '
    'a received payment appears mid-scan, without waiting for a sync edge',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 1,
          to: 100,
          percent: 0.5,
          spendableReady: true,
          rewound: false,
        ),
        snapshotValue: walletStateFixture(),
      )..transactionsResult = [];
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // A payment lands mid-scan: the ADR-0536 detect fires — the SYNC status
      // never changes (still Scanning, far from the tip), so only the new
      // arrival edge can surface the row promptly.
      fake.transactionsResult = [
        txSummaryFixture(
          minedHeight: 50,
          status: const TxStatus.confirmed(depth: 1),
          netAmountZat: 25000,
          hasMemo: false,
        ),
      ];
      fake.pushIncoming(
        const IncomingFundsEvent(
          kind: IncomingFundsEventKind.live,
          newTxCount: 1,
          totalTxDetected: 1,
          spanFromHeight: 50,
          spanToHeight: 50,
          cursor: 'w1:60',
        ),
      );
      await tester.pumpAndSettle();

      expect(
        find.text(l10n.walletAmount('+${formatZec(25000)}')),
        findsOneWidget,
        reason: 'the arrival edge refreshed the list while still scanning',
      );
    },
  );

  testWidgets(
    'S211: a live arrival shows the minimal cue SnackBar (founder decision) — '
    'singular and plural; a replay catch-up shows NONE (old news)',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(),
      )..transactionsResult = [];
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      fake.pushIncoming(
        const IncomingFundsEvent(
          kind: IncomingFundsEventKind.live,
          newTxCount: 1,
          totalTxDetected: 1,
          spanFromHeight: 50,
          spanToHeight: 50,
          cursor: 'w1:60',
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(
        find.text(l10n.walletPaymentReceived(1)),
        findsOneWidget,
        reason: 'the singular arrival cue shows',
      );
      await tester
          .pumpAndSettle(); // entrance completes — the dismiss timer arms
      await tester.pump(
        const Duration(seconds: 5),
      ); // fire the 4s dismiss timer
      await tester.pumpAndSettle(); // exit animation

      fake.pushIncoming(
        const IncomingFundsEvent(
          kind: IncomingFundsEventKind.live,
          newTxCount: 2,
          totalTxDetected: 3,
          spanFromHeight: 61,
          spanToHeight: 62,
          cursor: 'w1:70',
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(
        find.text(l10n.walletPaymentReceived(2)),
        findsOneWidget,
        reason: 'the plural arrival cue shows',
      );
      await tester
          .pumpAndSettle(); // entrance completes — the dismiss timer arms
      await tester.pump(
        const Duration(seconds: 5),
      ); // fire the 4s dismiss timer
      await tester.pumpAndSettle(); // exit animation

      // A REPLAY with arrivals collapses the list but shows NO cue — it is a
      // catch-up over history the user may have seen, not fresh news.
      fake.pushIncoming(
        const IncomingFundsEvent(
          kind: IncomingFundsEventKind.replay,
          newTxCount: 4,
          totalTxDetected: 3,
          spanFromHeight: 10,
          spanToHeight: 40,
          cursor: 'w1:70',
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(
        find.byType(SnackBar),
        findsNothing,
        reason: 'replay catch-ups never toast',
      );
    },
  );
  // --- transport indicator + Connection section + balance header ------

  test('S151: balanceAsOf pairs a timestamp ONLY with the displayed height '
      '(stamp first, then the latch\'s own observation moment)', () {
    final latchAt = DateTime(2026, 7, 1, 12);

    // UpToDate at the stamp's own height: both height and time render.
    final stamped = balanceAsOf(
      status: const SyncStatus.upToDate(tip: 500),
      latch: const WalletSyncedTipUnset(),
      lastSynced: const SyncStamp(height: 500, at: 1751700000),
    );
    expect(stamped.height, 500);
    expect(
      stamped.time,
      DateTime.fromMillisecondsSinceEpoch(1751700000 * 1000),
    );

    // UpToDate PAST the stamp with no matching latch: the tip wins, and the
    // stale stamp's clock time must NOT be paired with it (it would lie
    // about when the tip landed).
    final ahead = balanceAsOf(
      status: const SyncStatus.upToDate(tip: 900),
      latch: const WalletSyncedTipUnset(),
      lastSynced: const SyncStamp(height: 500, at: 1751700000),
    );
    expect(ahead.height, 900);
    expect(ahead.time, isNull);

    // UpToDate at the LATCH's tip while the stamp re-read lags: the latch's
    // own observation moment pairs — the header keeps ONE stable
    // height+time arm through the whole sync cycle (the 64px AsOfAt<->AsOf
    // pulse fix, #317).
    final latchPaired = balanceAsOf(
      status: const SyncStatus.upToDate(tip: 900),
      latch: WalletSyncedTipLatched(900, latchAt),
      lastSynced: const SyncStamp(height: 500, at: 1751700000),
    );
    expect(latchPaired.height, 900);
    expect(latchPaired.time, latchAt);

    // Not synced, no stamp: the honest unknown.
    final unknown = balanceAsOf(
      status: const SyncStatus.idle(),
      latch: const WalletSyncedTipUnset(),
      lastSynced: null,
    );
    expect(unknown.height, isNull);
    expect(unknown.time, isNull);

    // Offline with a stamp and NO session verdict (cold launch): the
    // persisted last-known height + its own time render.
    final offline = balanceAsOf(
      status: const SyncStatus.offline(),
      latch: const WalletSyncedTipUnset(),
      lastSynced: const SyncStamp(height: 321, at: 1751000000),
    );
    expect(offline.height, 321);
    expect(offline.time, isNotNull);

    // THE LATCH: mid-scan, the session's last synced tip holds the
    // header — it OUTRANKS a stale persisted stamp (fresher by
    // construction) and lends its own paired time; a stamp at the SAME
    // height takes precedence for the time (the SDK's authoritative
    // record).
    final midScan = balanceAsOf(
      status: const SyncStatus.scanning(
        from: 1,
        to: 900,
        percent: 0.5,
        spendableReady: true,
        rewound: false,
      ),
      latch: WalletSyncedTipLatched(700, latchAt),
      lastSynced: const SyncStamp(height: 500, at: 1751000000),
    );
    expect(midScan.height, 700);
    expect(midScan.time, latchAt); // the stamp's time belongs to 500
    final midScanStamped = balanceAsOf(
      status: const SyncStatus.scanning(
        from: 1,
        to: 900,
        percent: 0.5,
        spendableReady: true,
        rewound: false,
      ),
      latch: WalletSyncedTipLatched(700, latchAt),
      lastSynced: const SyncStamp(height: 700, at: 1751000000),
    );
    expect(
      midScanStamped.time,
      DateTime.fromMillisecondsSinceEpoch(1751000000 * 1000),
      reason: 'a stamp at the displayed height outranks the latch moment',
    );

    // The LIVE tip outranks the latch (the documented precedence's first leg
    // — pinned so a refactor can't quietly flip it).
    final live = balanceAsOf(
      status: const SyncStatus.upToDate(tip: 900),
      latch: WalletSyncedTipLatched(700, latchAt),
      lastSynced: null,
    );
    expect(live.height, 900);
    expect(live.time, isNull, reason: 'the latch moment belongs to 700');
  });

  test('#317: an INVALIDATED verdict suppresses BOTH the latch and the '
      'persisted stamp — the header goes honestly bare until the next '
      'up-to-date (the stamp-resurrection fix)', () {
    // The exact wrap-review scenario: a rewind invalidated the claim, and a
    // LIVE stamp still holds the just-orphaned height. The old
    // `latchedTip ?? lastSynced.height` fallback resurrected it here.
    final cleared = balanceAsOf(
      status: const SyncStatus.scanning(
        from: 1,
        to: 600,
        percent: 0.1,
        spendableReady: false,
        rewound: true,
      ),
      latch: const WalletSyncedTipInvalidated(),
      lastSynced: const SyncStamp(height: 700, at: 1751000000),
    );
    expect(cleared.height, isNull);
    expect(cleared.time, isNull);

    // A LIVE up-to-date tip always renders regardless of the verdict (the
    // verdict is about the PAST claim, never the present truth).
    final relatched = balanceAsOf(
      status: const SyncStatus.upToDate(tip: 650),
      latch: const WalletSyncedTipInvalidated(),
      lastSynced: const SyncStamp(height: 700, at: 1751000000),
    );
    expect(relatched.height, 650);
  });

  group('walletSyncedTipProvider (S151-2 — the session latch)', () {
    // Pure ProviderContainer harness: a controllable session seam + the
    // deterministic lifecycle fake (SyncStatusNotifier watches it).
    ({ProviderContainer container, FakeWalletSession fake}) latchHarness({
      FakeWalletSession? session,
    }) {
      final fake = session ?? FakeWalletSession();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
        ],
      );
      addTearDown(container.dispose);
      return (container: container, fake: fake);
    }

    // Let the fake stream's replay/pushes flush through the notifier chain.
    Future<void> settle() => Future<void>.delayed(Duration.zero);

    int? tipOf(ProviderContainer c) {
      final latch = c.read(walletSyncedTipProvider);
      return latch is WalletSyncedTipLatched ? latch.tip : null;
    }

    test(
      'latches UpToDate, HOLDS through a routine catch-up scan, and '
      'INVALIDATES on a rewind-shaped scan (target below the latch)',
      () async {
        final h = latchHarness(
          session: FakeWalletSession(current: const SyncStatus.idle()),
        );
        final sub = h.container.listen(walletSyncedTipProvider, (_, _) {});
        addTearDown(sub.close);
        await settle();
        expect(
          h.container.read(walletSyncedTipProvider),
          isA<WalletSyncedTipUnset>(),
          reason: 'no verdict yet — the persisted stamp may render',
        );

        h.fake.push(const SyncStatus.upToDate(tip: 700));
        await settle();
        expect(tipOf(h.container), 700);

        // Routine catch-up: new blocks -> a scan whose TARGET is AHEAD of the
        // latch. The latch must hold (the maintainer's no-flap requirement) even
        // though `from` (the scanned-EQUIVALENT height) dips below it.
        h.fake.push(
          const SyncStatus.scanning(
            from: 1,
            to: 750,
            percent: 0.5,
            spendableReady: true,
            rewound: false,
          ),
        );
        await settle();
        expect(tipOf(h.container), 700);

        // Rewind-shaped: a scan whose TARGET is BELOW the latch (a server
        // switch to a lower tip, or a rewind whose flagged samples were
        // coalesced away) — the latched claim is no longer safe (review A1).
        // The verdict is INVALIDATED, not unset: the persisted stamp must not
        // resurrect the dropped height (#317 wrap review).
        h.fake.push(
          const SyncStatus.scanning(
            from: 1,
            to: 600,
            percent: 0.1,
            spendableReady: true,
            rewound: false,
          ),
        );
        await settle();
        expect(
          h.container.read(walletSyncedTipProvider),
          isA<WalletSyncedTipInvalidated>(),
        );

        // The next UpToDate re-latches.
        h.fake.push(const SyncStatus.upToDate(tip: 650));
        await settle();
        expect(tipOf(h.container), 650);
      },
    );

    test(
      '#317: a rewound:true sample INVALIDATES even with the target AHEAD '
      'of the latch — the first-class reorg signal needs no target drop',
      () async {
        final h = latchHarness(
          session: FakeWalletSession(current: const SyncStatus.idle()),
        );
        final sub = h.container.listen(walletSyncedTipProvider, (_, _) {});
        addTearDown(sub.close);
        await settle();
        h.fake.push(const SyncStatus.upToDate(tip: 700));
        await settle();
        expect(tipOf(h.container), 700);

        // A reorg rewound the wallet below 700, but the pass's TARGET (the new
        // chain tip) is ahead of the latch — `to < latch` alone would miss it.
        h.fake.push(
          const SyncStatus.scanning(
            from: 1,
            to: 760,
            percent: 0.3,
            spendableReady: true,
            rewound: true,
          ),
        );
        await settle();
        expect(
          h.container.read(walletSyncedTipProvider),
          isA<WalletSyncedTipInvalidated>(),
        );

        // rewound is a pass-scoped LEVEL: later samples of the same pass carry
        // it too — re-applying is an idempotent no-op, and the verdict HOLDS
        // through a fault (only a fresh UpToDate proves the claim safe again).
        h.fake.push(
          const SyncStatus.scanning(
            from: 1,
            to: 760,
            percent: 0.7,
            spendableReady: true,
            rewound: true,
          ),
        );
        h.fake.push(const SyncStatus.stalled(reason: StallReason.unknown));
        await settle();
        expect(
          h.container.read(walletSyncedTipProvider),
          isA<WalletSyncedTipInvalidated>(),
        );

        // A NEXT pass that starts clean (rewound resets per pass) still must
        // NOT clear the verdict — only an UpToDate does.
        h.fake.push(
          const SyncStatus.scanning(
            from: 1,
            to: 760,
            percent: 0.9,
            spendableReady: true,
            rewound: false,
          ),
        );
        await settle();
        expect(
          h.container.read(walletSyncedTipProvider),
          isA<WalletSyncedTipInvalidated>(),
        );

        h.fake.push(const SyncStatus.upToDate(tip: 760));
        await settle();
        expect(tipOf(h.container), 760);
      },
    );

    test(
      '#317: a latch first built MID-REWIND starts invalidated, not unset '
      '(a cold launch into a rescanning wallet must suppress the stamp)',
      () async {
        final h = latchHarness(
          session: FakeWalletSession(
            current: const SyncStatus.scanning(
              from: 1,
              to: 600,
              percent: 0.2,
              spendableReady: false,
              rewound: true,
            ),
          ),
        );
        // Drive the status notifier first so its value exists...
        final statusSub = h.container.listen(syncStatusProvider, (_, _) {});
        addTearDown(statusSub.close);
        await settle();
        // ...then the FIRST read must seed the invalidated verdict.
        expect(
          h.container.read(walletSyncedTipProvider),
          isA<WalletSyncedTipInvalidated>(),
        );
      },
    );

    test('#317: re-affirming the SAME tip keeps the original observation '
        'moment — a replay never dresses an old claim up as fresh', () async {
      final h = latchHarness(
        session: FakeWalletSession(current: const SyncStatus.idle()),
      );
      final sub = h.container.listen(walletSyncedTipProvider, (_, _) {});
      addTearDown(sub.close);
      await settle();

      h.fake.push(const SyncStatus.upToDate(tip: 700));
      await settle();
      final first =
          h.container.read(walletSyncedTipProvider) as WalletSyncedTipLatched;

      // A routine catch-up pass ends at the SAME tip (nothing new landed).
      h.fake.push(
        const SyncStatus.scanning(
          from: 1,
          to: 700,
          percent: 0.5,
          spendableReady: true,
          rewound: false,
        ),
      );
      h.fake.push(const SyncStatus.upToDate(tip: 700));
      await settle();
      final reaffirmed =
          h.container.read(walletSyncedTipProvider) as WalletSyncedTipLatched;
      expect(reaffirmed.at, first.at, reason: 'same tip -> same moment');

      // A NEW tip refreshes the observation moment.
      h.fake.push(const SyncStatus.upToDate(tip: 750));
      await settle();
      final advanced =
          h.container.read(walletSyncedTipProvider) as WalletSyncedTipLatched;
      expect(advanced.tip, 750);
      expect(advanced.at.isBefore(first.at), isFalse);
    });

    test('RESETS on a session identity change — a post-rescan wallet must '
        'never keep claiming the pre-rescan tip over a rebuilt DB', () async {
      final a = FakeWalletSession(current: const SyncStatus.idle());
      final b = FakeWalletSession(current: const SyncStatus.idle());
      final sessionSwitch = StateProvider<WalletSession?>((ref) => a);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
          appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
        ],
      );
      addTearDown(container.dispose);
      final sub = container.listen(walletSyncedTipProvider, (_, _) {});
      addTearDown(sub.close);
      await Future<void>.delayed(Duration.zero);

      a.push(const SyncStatus.upToDate(tip: 700));
      await Future<void>.delayed(Duration.zero);
      expect(
        (container.read(walletSyncedTipProvider) as WalletSyncedTipLatched).tip,
        700,
      );

      // The rescan swap: a FRESH session identity (the graph's re-key rule).
      container.read(sessionSwitch.notifier).state = b;
      await Future<void>.delayed(Duration.zero);
      expect(
        container.read(walletSyncedTipProvider),
        isA<WalletSyncedTipUnset>(),
      );

      b.push(const SyncStatus.upToDate(tip: 900));
      await Future<void>.delayed(Duration.zero);
      expect(
        (container.read(walletSyncedTipProvider) as WalletSyncedTipLatched).tip,
        900,
      );
    });

    test('SEEDS from the current value — a latch first built AFTER the '
        'stream already reached UpToDate starts with the height', () async {
      // The stream is live and up-to-date BEFORE anything watches the latch
      // (the cold-start interleaving where the card renders late).
      final h = latchHarness(
        session: FakeWalletSession(
          current: const SyncStatus.upToDate(tip: 800),
        ),
      );
      // Drive the status notifier first so its value exists...
      final statusSub = h.container.listen(syncStatusProvider, (_, _) {});
      addTearDown(statusSub.close);
      await settle();
      // ...then the FIRST read of the latch must seed, not wait for an emit.
      expect(
        (h.container.read(walletSyncedTipProvider) as WalletSyncedTipLatched)
            .tip,
        800,
      );
    });
  });

  testWidgets(
    'S151: the badge row carries the transport ICON; the sheet carries the '
    'label, the explanation, and the Server row when the host wired one',
    (tester) async {
      // FR-49 S12 C2: a SCANNING wallet, so the bar shows over the neutral
      // (Tor off) transport under test — a fresh UpToDate over Tor off is the
      // healthy state in which the bar hides.
      const scanning = SyncStatus.scanning(
        from: 1,
        to: 2000,
        percent: 0.42,
        spendableReady: true,
        rewound: false,
      );
      final fake = FakeWalletSession(
        current: scanning,
        snapshotValue: walletStateFixture(
          syncStatus: scanning,
        ), // fixture TorState: off
      );
      await tester.pumpWidget(
        _harness(
          session: fake,
          extraOverrides: [
            walletEndpointHostProvider.overrideWithValue('zec.rocks'),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // Row: icon-only (outline: Tor off is not a protection claim), no label
      // text on the fixed row.
      expect(
        find.descendant(
          of: syncBadge(),
          matching: find.byIcon(Icons.shield_outlined),
        ),
        findsOneWidget,
      );
      expect(find.text(l10n.walletTorOff), findsNothing);

      // Sheet: the Connection section tells the full story.
      await tester.tap(syncBadge());
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSyncSheetConnection), findsOneWidget);
      expect(find.text(l10n.walletTorOff), findsOneWidget);
      expect(find.text(l10n.walletTransportExplainDirect), findsOneWidget);
      expect(find.text(l10n.walletSyncSheetServer), findsOneWidget);
      expect(find.text('zec.rocks'), findsOneWidget);
    },
  );

  testWidgets(
    'S151: no endpoint wired -> the Server row shows the SESSION server '
    '(P3-13); a HOST transport claim replaces the SDK TorState on row AND sheet',
    (tester) async {
      // FR-49 S12 C2 (rev.2 R6): a host-declared PROTECTED transport over a
      // fresh UpToDate is healthy, so the bar hides there. A SCANNING wallet
      // keeps the bar up, so the host claim's row icon and label are pinned.
      const scanning = SyncStatus.scanning(
        from: 1,
        to: 2000,
        percent: 0.42,
        spendableReady: true,
        rewound: false,
      );
      final fake = FakeWalletSession(
        current: scanning,
        snapshotValue: walletStateFixture(syncStatus: scanning),
      );
      await tester.pumpWidget(
        _harness(
          session: fake,
          extraOverrides: [
            walletHostTransportProvider.overrideWithValue(
              const WalletHostTransport(label: 'VLESS', protection: true),
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // The protected host claim earns the FILLED shield on the row.
      expect(
        find.descendant(of: syncBadge(), matching: find.byIcon(Icons.shield)),
        findsOneWidget,
      );
      // And the badge a11y label carries the host's own word, not "Tor off".
      final handle = tester.ensureSemantics();
      expect(find.bySemanticsLabel(RegExp('VLESS')), findsOneWidget);
      handle.dispose();

      await tester.tap(syncBadge());
      await tester.pumpAndSettle();
      expect(find.text('VLESS'), findsOneWidget);
      expect(find.text(l10n.walletTransportExplainHostProxy), findsOneWidget);
      expect(find.text(l10n.walletTorOff), findsNothing);
      // No endpoint wired by the HOST — but since P3-13 the Server row reads
      // the SESSION's effective server (the dial's own truth), so with a
      // session present the row shows: here the fake session's default host.
      // The row hides only when there is neither a session nor a wired host.
      expect(find.text(l10n.walletSyncSheetServer), findsOneWidget);
      expect(find.text('zec.rocks'), findsOneWidget);
    },
  );

  testWidgets(
    'S151-2: once synced, the header KEEPS "(as of block N)" through a '
    're-scan — no flap back to a bare header, no card-height shift',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 700),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 700),
          balance: balanceFixture(totalZat: 100000000),
        ), // no stamp — the SESSION latch alone must carry the height
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // No stamp -> the latch's own moment pairs (#317): ONE stable
      // height+time arm from the first frame through the scan window below.
      final latch =
          ProviderScope.containerOf(
                tester.element(find.byType(WalletScreen)),
              ).read(walletSyncedTipProvider)
              as WalletSyncedTipLatched;
      final asOf = l10n.walletBalanceHeaderAt(
        balanceCaptionTime(latch.at, l10n.localeName),
      );
      expect(find.text(asOf), findsOneWidget);
      final cardHeight = tester
          .getSize(find.byKey(const ValueKey('wallet-balance-card')))
          .height;

      // New blocks arrive -> the loop re-scans. The wallet HAS synced; the
      // header must hold the last synced tip (the old "age unknown" flap
      // was dishonest AND a layout shift — maintainer).
      fake.push(
        const SyncStatus.scanning(
          from: 700,
          to: 900,
          percent: 0.4,
          spendableReady: true,
          rewound: false,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text(asOf), findsOneWidget);
      expect(find.text(l10n.walletBalanceLabel), findsNothing);
      expect(
        tester
            .getSize(find.byKey(const ValueKey('wallet-balance-card')))
            .height,
        cardHeight,
      );
    },
  );

  testWidgets(
    'S151 regression: a LONG server host on the sheet wraps at 320dp and an '
    'accessibility text scale instead of overflowing (never ellipsized — a '
    'truncated server identity would lie)',
    (tester) async {
      tester.view.physicalSize = const Size(320 * 3, 640 * 3);
      tester.view.devicePixelRatio = 3.0;
      addTearDown(tester.view.reset);
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
        ),
      );
      // Since P3-13 the Server row reads the SESSION's effective server (the
      // dial's own truth); the host seam below is only the pre-session value.
      // The long host therefore rides the session's status too.
      fake.syncServerStatusResult = const SyncServerStatus(
        effectiveUrl: 'https://my-local-lightwalletd.tailnet-1234.ts.net:443',
        defaultUrl: 'https://my-local-lightwalletd.tailnet-1234.ts.net:443',
        choice: null,
        fallback: null,
      );
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletEndpointHostProvider.overrideWithValue(
              'my-local-lightwalletd.tailnet-1234.ts.net',
            ),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            builder: (context, child) => MediaQuery.withClampedTextScaling(
              minScaleFactor: 2.0,
              maxScaleFactor: 2.0,
              child: child!,
            ),
            home: const Scaffold(body: SyncStatusSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(
        find.text('my-local-lightwalletd.tailnet-1234.ts.net'),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'S151 privacy rule: with NO readable snapshot the sheet transport reads '
    'as NOT protected (unknown/caution), never a benign default',
    (tester) async {
      // A wedged cold snapshot (busy DB) with the stream alive: the sheet
      // must not guess the transport — TorState.unknown => the honest
      // "can't be verified" copy, outline shield, never the protected tone.
      // The sheet is pumped DIRECTLY (the sheet-body test idiom): with no
      // snapshot value at all the wallet surface shows _SnapshotError and
      // has no badge to tap, but a host deep link / an open sheet surviving
      // a snapshot invalidation still renders this widget.
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotThrows: true,
      );
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: SyncStatusSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(SyncStatusSheet));

      expect(find.text(l10n.walletTorUnknown), findsOneWidget);
      expect(find.text(l10n.walletTransportExplainUnverified), findsOneWidget);
      // The filled (protected) shield must not render anywhere on the sheet.
      expect(find.byIcon(Icons.shield), findsNothing);
    },
  );

  testWidgets(
    '#356-F8: a failed sync START on the sheet reads the honest failure '
    'explanation + a working retry — never "starts automatically, no '
    'action needed"',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.idle(),
        snapshotValue: walletStateFixture(),
      )..failStart = true;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: SyncStatusSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(SyncStatusSheet));

      // The contradiction is gone from BOTH lines the Idle arm renders…
      expect(find.text(l10n.walletSyncExplainIdle), findsNothing);
      expect(find.text(l10n.walletSyncIdleDetail), findsNothing);
      // …replaced by the honest start-failure copy + the sheet-local retry.
      // The short notice line is deliberately NOT duplicated under the
      // explanation that already says it (UX LOW — it still rides the
      // badge a11y label).
      expect(find.text(l10n.walletSyncExplainStartFailed), findsOneWidget);
      expect(find.text(l10n.walletSyncStartFailed), findsNothing);
      expect(find.text(l10n.walletSyncRetry), findsOneWidget);

      // The retry works from the sheet: the loop starts and the failure
      // copy clears (the Idle arm flips to the driving "Starting" treatment).
      fake.failStart = false;
      await tester.tap(find.text(l10n.walletSyncRetry));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSyncExplainStartFailed), findsNothing);
      expect(find.text(l10n.walletSyncRetry), findsNothing);
      expect(fake.startCount, greaterThanOrEqualTo(2));
    },
  );

  testWidgets(
    'S151: the balance header carries the as-of timestamp when the stamp '
    'backs the shown height',
    (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 500),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 500),
          lastSynced: const SyncStamp(height: 500, at: 1751700000),
          balance: balanceFixture(totalZat: 100000000),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      // ONE header line carries height AND time (never a second
      // row — the old sub-line was a per-state layout shift).
      // S12: the caption carries the time only; a date not
      // today includes the date.
      final expectedTime = balanceCaptionTime(
        DateTime.fromMillisecondsSinceEpoch(1751700000 * 1000),
        l10n.localeName,
      );
      expect(
        find.text(l10n.walletBalanceHeaderAt(expectedTime)),
        findsOneWidget,
      );
      expect(
        find.text(
          l10n.walletBalanceHeaderAsOf(exactBlockCount(500, l10n.localeName)),
        ),
        findsNothing,
      );
    },
  );
  testWidgets(
    '#317: a REWOUND scan suppresses the header height even with a LIVE '
    'stamp — the persisted height must not resurrect past the invalidation',
    (tester) async {
      // The wrap-review HIGH scenario end-to-end: the wallet synced to 700
      // (stamp persisted), then a deep reorg rewound it — the pass reports
      // rewound:true. The header must fall back to plain "Balance", NOT to
      // the stamp's 700 (which would pair the stamp's clock time with a
      // height the chain just orphaned, for the whole re-scan window).
      final fake = FakeWalletSession(
        current: const SyncStatus.scanning(
          from: 1,
          to: 800,
          percent: 0.2,
          spendableReady: false,
          rewound: true,
        ),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.scanning(
            from: 1,
            to: 800,
            percent: 0.2,
            spendableReady: false,
            rewound: true,
          ),
          lastSynced: const SyncStamp(height: 700, at: 1751700000),
          balance: balanceFixture(totalZat: 100000000),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester, find.byType(WalletScreen));

      expect(find.text(l10n.walletBalanceLabel), findsOneWidget);
      expect(
        find.text(
          l10n.walletBalanceHeaderAsOf(exactBlockCount(700, l10n.localeName)),
        ),
        findsNothing,
      );

      // The next UpToDate re-latches and the height returns.
      fake.push(const SyncStatus.upToDate(tip: 810));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletBalanceLabel), findsNothing);
    },
  );

  group('the Swap slot (#356-F3 — the action row never shifts mid-tap)', () {
    const swapPolicy = SwapHostPolicy(
      config: SwapProviderConfig(endpoint: 'https://1click.test', jwt: 'tok'),
      enabled: true,
      declaredKill: SwapKill.windDown,
    );

    FakeWalletSession activeFake() => FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(totalZat: 100000000, spendableZat: 100000000),
      ),
    );

    testWidgets(
      'policy-enabled host: the slot is RESERVED while the activation is '
      'pending — Send/Receive do NOT move when Swap resolves live',
      (tester) async {
        final enable = Completer<bool>();
        await tester.pumpWidget(
          _harness(
            session: activeFake(),
            extraOverrides: [
              swapHostPolicyProvider.overrideWithValue(swapPolicy),
              swapActivationProvider.overrideWith((ref) => enable.future),
            ],
          ),
        );
        await tester.pumpAndSettle();

        // Pending: no Swap surface (the honesty rule — nothing optimistic),
        // but the third slot is held.
        expect(find.byKey(const ValueKey('wallet-action-swap')), findsNothing);
        final sendBefore = tester.getRect(
          find.byKey(const ValueKey('wallet-action-send')),
        );
        final receiveBefore = tester.getRect(
          find.byKey(const ValueKey('wallet-action-receive')),
        );

        enable.complete(true);
        await tester.pumpAndSettle();

        // Live: the button materialized IN the reserved slot; the other two
        // are pixel-identical — the mid-tap shift is structurally gone.
        expect(
          find.byKey(const ValueKey('wallet-action-swap')),
          findsOneWidget,
        );
        expect(
          tester.getRect(find.byKey(const ValueKey('wallet-action-send'))),
          sendBefore,
        );
        expect(
          tester.getRect(find.byKey(const ValueKey('wallet-action-receive'))),
          receiveBefore,
        );
      },
    );

    testWidgets(
      'an activation that resolves OFF collapses the slot to the truthful '
      'two-button row (the one-shot settle)',
      (tester) async {
        final enable = Completer<bool>();
        await tester.pumpWidget(
          _harness(
            session: activeFake(),
            extraOverrides: [
              swapHostPolicyProvider.overrideWithValue(swapPolicy),
              swapActivationProvider.overrideWith((ref) => enable.future),
            ],
          ),
        );
        await tester.pumpAndSettle();
        final sendReserved = tester.getRect(
          find.byKey(const ValueKey('wallet-action-send')),
        );

        enable.complete(false);
        await tester.pumpAndSettle();

        // No Swap button ever appeared, and the row re-lays to two equal
        // slots — the buttons re-center rightward as the third slot goes
        // (the keyed node is the fixed-diameter button, so POSITION is the
        // observable, not width).
        expect(find.byKey(const ValueKey('wallet-action-swap')), findsNothing);
        final sendCollapsed = tester.getRect(
          find.byKey(const ValueKey('wallet-action-send')),
        );
        expect(sendCollapsed.center.dx, greaterThan(sendReserved.center.dx));
      },
    );

    testWidgets(
      'no swap policy: the two-button row from the first frame — no slot, '
      'no gap (the §3.5 removed-feature posture)',
      (tester) async {
        await tester.pumpWidget(_harness(session: activeFake()));
        await tester.pumpAndSettle();
        expect(find.byKey(const ValueKey('wallet-action-swap')), findsNothing);
        // Two equal slots: Send spans half the row, mirroring Receive.
        final send = tester.getRect(
          find.byKey(const ValueKey('wallet-action-send')),
        );
        final receive = tester.getRect(
          find.byKey(const ValueKey('wallet-action-receive')),
        );
        expect(send.width, receive.width);
        expect(receive.right, greaterThan(send.right));
      },
    );
  });

  testWidgets('S13 M3: a re-delivered live arrival makes ONE snackbar, and an '
      'arrival during catch-up still shows (its own watermark, not the '
      "coin's)", (tester) async {
    final fake = FakeWalletSession(
      // Catching up: the coin's gate is closed here, the snackbar's is not.
      current: const SyncStatus.scanning(
        from: 1,
        to: 100,
        percent: 0.5,
        spendableReady: true,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(),
    )..transactionsResult = [];
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    final cue = find.text(l10n.walletPaymentReceived(1));

    Future<void> arrive(String cursor, int span) async {
      fake.pushIncoming(
        IncomingFundsEvent(
          kind: IncomingFundsEventKind.live,
          newTxCount: 1,
          totalTxDetected: 1,
          spanFromHeight: span,
          spanToHeight: span,
          cursor: cursor,
        ),
      );
      await tester.pump();
      await tester.pump();
    }

    Future<void> dismiss() async {
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();
    }

    await arrive('w1:60', 50);
    expect(cue, findsOneWidget, reason: 'an arrival during catch-up shows');
    await dismiss();

    // The same arrival again (a resume re-delivers it; a new cursor, so the
    // value-equality filter does not catch it).
    await arrive('w1:61', 50);
    expect(cue, findsNothing, reason: 'the same money is not announced twice');
    await dismiss();

    await arrive('w1:62', 51);
    expect(cue, findsOneWidget, reason: 'a NEW arrival still shows');
  });

  testWidgets('the_menu_opens_the_server_picker_by_name', (tester) async {
    // S15 iPhone walk: behind the sync status row alone the picker was not
    // found. The menu names it, with the picker's own title.
    await tester.pumpWidget(
      _harness(
        session: FakeWalletSession(
          current: const SyncStatus.upToDate(tip: 100),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 100),
            balance: balanceFixture(totalZat: 100000000),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester, find.byType(WalletScreen));
    await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSyncServerSheetTitle));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('sync-server-close')), findsOneWidget);
  });
}

/// A fixed-state [WalletRescanController] stub for widget tests — renders any
/// presentation phase without a real rescan, and records `dismissFailure`.
class _StubRescan extends WalletRescanController {
  _StubRescan(this._initial);
  final WalletRescanState _initial;
  int dismissCount = 0;

  @override
  WalletRescanState build() => _initial;

  @override
  void dismissFailure() {
    dismissCount++;
    state = const WalletRescanIdle();
  }
}

/// The wallet surface's Send action InkWell (keyed in [_WalletActions]). `onTap`
/// is null iff Send is gated off — the spend-before-sync state under test.
InkWell _sendInk(WidgetTester tester) {
  return tester.widget<InkWell>(
    find.descendant(
      of: find.byKey(const ValueKey('wallet-action-send')),
      matching: find.byType(InkWell),
    ),
  );
}
