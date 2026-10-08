import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_presentation.dart'
    show exactBlockCount;
import 'package:zec_wallet_ui/features/wallet/sync_status_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Sync-detail sheet tests for the sync-off arms. The sheet is pumped
/// DIRECTLY over a fake session (the sheet-body idiom from
/// wallet_screen_test); the drive reaches disabledByHost through the REAL
/// [walletSyncControllerProvider] via the policy seam — the R1 idiom.
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(SyncStatusSheet)));

Widget _harness(FakeWalletSession session, {bool syncPolicyOn = true}) {
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      walletSyncPolicyProvider.overrideWithValue(syncPolicyOn),
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const Scaffold(body: SyncStatusSheet()),
    ),
  );
}

void main() {
  testWidgets('S205-b: a RETAINED Scanning under sync-off renders NO '
      '"Progress"/"Blocks left" rows — figure rows claiming an in-flight '
      'scan must not sit beside the "Sync off" headline', (tester) async {
    // The stream retains the last good status across the policy-off stop —
    // exactly the Scanning shape whose figure rows the sheet must suppress.
    final fake = FakeWalletSession(
      current: const SyncStatus.scanning(
        from: 1000,
        to: 3000,
        percent: 0.5,
        spendableReady: true,
        rewound: false,
      ),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(fake, syncPolicyOn: false));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // The sync-off story owns the header + explanation…
    expect(find.text(l10n.walletSyncDisabled), findsOneWidget);
    expect(find.text(l10n.walletSyncExplainDisabled), findsOneWidget);
    // …and the in-flight-scan figure rows are gone (the progress BAR is
    // already suppressed by the presentation's syncDisabled arm).
    expect(find.text(l10n.walletSyncSheetProgress), findsNothing);
    expect(find.text('50%'), findsNothing);
    expect(find.text(l10n.walletSyncSheetBlocksLeft), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets('S205-b: the walletSyncDisabledDetail line is filtered from '
      'the sheet body — the explanation already carries the message (the '
      'S175 near-identical-pair rule; the detail stays for the badge a11y '
      'label)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
      ),
    );
    await tester.pumpWidget(_harness(fake, syncPolicyOn: false));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSyncExplainDisabled), findsOneWidget);
    expect(
      find.text(l10n.walletSyncDisabledDetail),
      findsNothing,
      reason:
          'stacking "Turn on syncing…" directly under an explanation that '
          'already ends "…until syncing is turned on" is the redundant pair '
          'the S175 rule filters',
    );
  });

  testWidgets('S205-b: a retained UpToDate under sync-off KEEPS the '
      '"Synced to" row — it IS the last synced state the disabled '
      'explanation promises', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 1579873),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 1579873),
      ),
    );
    await tester.pumpWidget(_harness(fake, syncPolicyOn: false));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSyncDisabled), findsOneWidget);
    expect(find.text(l10n.walletSyncSheetSyncedTo), findsOneWidget);
    expect(find.text('1,579,873'), findsOneWidget);
  });

  testWidgets('#399: a live connectivity stall tells the calm normal-offline '
      'story and offers "Try now" — the tap restarts the loop (the ladder '
      'reset)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(
        reason: StallReason.endpointUnreachable,
      ),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // The calm explanation owns the slot (never the generic "hit a problem")
    // and carries both hedges ITSELF; the walletStallEndpoint reason line is
    // suppressed beneath it (review M4 — the near-identical-pair
    // rule; it still rides the badge a11y label).
    expect(find.text(l10n.walletSyncExplainStalledOffline), findsOneWidget);
    expect(find.text(l10n.walletSyncExplainStalled), findsNothing);
    expect(find.text(l10n.walletStallEndpoint), findsNothing);

    final startsBefore = fake.startCount;
    await tester.tap(find.text(l10n.walletSyncTryNow));
    await tester.pumpAndSettle();
    expect(fake.stopCount, 1, reason: 'the ladder-resetting stop ran');
    expect(fake.startCount, startsBefore + 1, reason: 'and the fresh start');
  });

  testWidgets('#399: every LIVE stall arm gets "Try now" (a freed-up disk '
      'deserves an immediate attempt too)', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(reason: StallReason.storageFull),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSyncTryNow), findsOneWidget);
    // The hard stall keeps the generic explanation + its typed reason.
    expect(find.text(l10n.walletSyncExplainStalled), findsOneWidget);
    expect(find.text(l10n.walletStallStorage), findsOneWidget);
  });

  testWidgets('#399: NO "Try now" under sync-off — the retained stall is not '
      'live and the honest affordance is the host\'s settings', (tester) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(
        reason: StallReason.endpointUnreachable,
      ),
      snapshotValue: walletStateFixture(),
    );
    await tester.pumpWidget(_harness(fake, syncPolicyOn: false));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSyncDisabled), findsOneWidget);
    expect(find.text(l10n.walletSyncTryNow), findsNothing);
  });

  testWidgets('#399: a failed DRIVE keeps its own retry — "Try again" '
      'renders, "Try now" does not (mutually exclusive by the drive enum)', (
    tester,
  ) async {
    final fake = FakeWalletSession(
      current: const SyncStatus.stalled(
        reason: StallReason.endpointUnreachable,
      ),
      snapshotValue: walletStateFixture(),
    )..failStart = true;
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSyncRetry), findsOneWidget);
    expect(find.text(l10n.walletSyncTryNow), findsNothing);
  });

  // ---------------------------------------------------------------- T-2 ----
  // UI-1 row T-2 (`docs/plan/production-readiness-phase-1.md` §4r U-2): the
  // sheet is where the EXACT numbers belong, and at the contract commit it has
  // figure rows for `Scanning`, `UpToDate` and `Offline` only — the other four
  // reached-tip variants fall to `default: break` (§4m #5), so a user who taps
  // the badge to ask "how far did it get?" is told the state and no height at
  // all. `newestKnown` is rendered NOWHERE, so "the server is behind" carries
  // no answer to "by how much".
  //
  // The rows below assert by the LABEL key the sheet already owns
  // (`walletSyncSheetSyncedTo`) and, for the behind figure, by the NUMBER —
  // U-2's new key does not exist at the contract commit and this half is
  // blind to the name the implementer gives it. `textContaining` so the
  // figure counts whether it lands in its own value cell (the sheet's row
  // shape) or inside an ICU plural sentence.
  group('T-2 the sheet\'s figure rows for every reached-tip variant '
      '(§4r U-2)', () {
    const pools = PoolServiceReport(
      sapling: PoolService.unsupported(),
      orchard: PoolService.served(roots: 7),
      ironwood: PoolService.served(roots: 0),
    );
    const grace = UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400);

    Future<WalletLocalizations> pumpSheet(
      WidgetTester tester,
      SyncStatus status,
    ) async {
      final fake = FakeWalletSession(
        current: status,
        snapshotValue: walletStateFixture(syncStatus: status),
      );
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      return _l10n(tester);
    }

    // The three siblings that carry a tip and nothing else the sheet needs.
    // `headline` is the arm's OWN headline key — the precondition that the
    // fixture reached the sheet at all, so a red below is the missing row and
    // never a fixture that never rendered.
    final syncedTo =
        <
          ({
            String name,
            SyncStatus status,
            int tip,
            String Function(WalletLocalizations) headline,
          })
        >[
          (
            name: 'UpToDateLimited',
            status: const SyncStatus.upToDateLimited(tip: 1579873),
            tip: 1579873,
            headline: (l) => l.walletSyncUpToDateLimited,
          ),
          (
            name: 'UpToDateDegraded',
            status: const SyncStatus.upToDateDegraded(
              tip: 1579873,
              pools: pools,
            ),
            tip: 1579873,
            headline: (l) => l.walletSyncUpToDateDegraded,
          ),
          (
            name: 'UpToDateUnverified',
            status: const SyncStatus.upToDateUnverified(
              tip: 1579873,
              grace: grace,
              streakReported: false,
            ),
            tip: 1579873,
            headline: (l) => l.walletSyncUnverified,
          ),
        ];

    for (final c in syncedTo) {
      testWidgets('${c.name} renders the "Synced to" row', (tester) async {
        final l10n = await pumpSheet(tester, c.status);
        expect(
          find.text(c.headline(l10n)),
          findsOneWidget,
          reason: 'precondition: the sheet is on the ${c.name} arm',
        );

        expect(
          find.text(l10n.walletSyncSheetSyncedTo),
          findsOneWidget,
          reason:
              '${c.name} is a completed pass that reached `tip` — the sheet '
              'must say how far it got (§4r U-2)',
        );
        expect(
          find.text(exactBlockCount(c.tip, l10n.localeName)),
          findsOneWidget,
          reason: 'and the EXACT height, the sheet\'s own vocabulary',
        );
      });
    }

    testWidgets('EndpointBehind renders BOTH "Synced to" and the behind-by '
        'figure — "behind the network" with no distance is half a fact '
        '(§4m #5)', (tester) async {
      const tip = 1579873;
      const newestKnown = 1600000;
      final l10n = await pumpSheet(
        tester,
        const SyncStatus.endpointBehind(tip: tip, newestKnown: newestKnown),
      );
      expect(
        find.text(l10n.walletSyncEndpointBehind),
        findsOneWidget,
        reason: 'precondition: the sheet is on the behind arm',
      );

      expect(
        find.text(l10n.walletSyncSheetSyncedTo),
        findsOneWidget,
        reason: 'the pass DID complete against this server — say to where',
      );
      expect(
        find.text(exactBlockCount(tip, l10n.localeName)),
        findsOneWidget,
        reason: 'that server\'s tip, exactly',
      );
      expect(
        find.textContaining(
          exactBlockCount(newestKnown - tip, l10n.localeName),
        ),
        findsAtLeastNWidgets(1),
        reason:
            'M = newestKnown - tip = ${newestKnown - tip}, the LOWER BOUND on '
            'how far behind the server is — the one number that turns "behind '
            'the network" into a decision (§4r U-2)',
      );
    });

    testWidgets('CONTROL: no behind row when newestKnown - tip < 1 — "behind '
        'by at least 0 blocks" is not a fact', (tester) async {
      // 555,555: no `0` digit anywhere in the rendered sheet for this arm
      // (headline, explanation, "Synced to block", "Connection", "Tor off",
      // the direct-transport paragraph and "Close" are all digit-free), so a
      // stray `0` can ONLY be an unguarded behind-by figure.
      final l10n = await pumpSheet(
        tester,
        const SyncStatus.endpointBehind(tip: 555555, newestKnown: 555555),
      );
      final headline = l10n.walletSyncEndpointBehind;
      expect(
        find.text(headline),
        findsOneWidget,
        reason: 'precondition: the sheet is on the behind arm',
      );
      // ANTI-VACUITY: a `findsNothing` proves nothing unless the finder can
      // see this sheet at all. The tail of the arm's own headline is text we
      // KNOW renders — derived from the key, never an English literal.
      expect(
        find.textContaining(headline.substring(headline.length ~/ 2)),
        findsAtLeastNWidgets(1),
        reason: 'textContaining CAN match inside this sheet',
      );

      expect(
        find.textContaining('0'),
        findsNothing,
        reason:
            'a server AT the newest height this wallet knows of is not behind '
            'by a measurable amount; the row is suppressed, not zeroed',
      );
    });
  });
}
