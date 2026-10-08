import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/core/theme/typography.dart';
import 'package:zec_wallet_ui/features/wallet/hide_balance.dart';
import 'package:zec_wallet_ui/features/wallet/labeled_zat_row.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_activation.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_config.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_presentation.dart'
    show TransportTone, WalletHostTransport;
import 'package:zec_wallet_ui/features/wallet/sync_status_sheet.dart'
    show SyncStatusSheet;
import 'package:zec_wallet_ui/features/wallet/tx_detail_sheet.dart'
    show TxDetailSheet;
import 'package:zec_wallet_ui/features/wallet/wallet_coin.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_tip_follow.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart'
    show RescanAllHistory;
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Stage S12 (FR-49 W-6/W-7/W-8; `docs/plan/stage-12-the-wallet-tab.md` §3):
/// the wallet tab in the refreshed design. One group per contract clause —
/// C2 the sync bar, C3 the card's conditional rows and the coin, C4 the
/// action tiles, C5 the activity group, C6 hide balance — each read against
/// §1a revision 2 where it amends §1.
///
/// The harness is `wallet_screen_test.dart`'s, verbatim in shape: a
/// [FakeWalletSession] behind `walletSessionProvider`, the real screen, the
/// light theme.

WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(WalletScreen)));

ProviderContainer _container(WidgetTester tester) =>
    ProviderScope.containerOf(tester.element(find.byType(WalletScreen)));

Widget _harness({
  required WalletSession session,
  ThemeData? theme,
  bool disableAnimations = false,
  List<Override> extraOverrides = const [],
}) {
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      ...extraOverrides,
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: theme ?? lightTheme,
      home: const WalletScreen(),
      builder: disableAnimations
          ? (context, child) => MediaQuery(
              data: MediaQuery.of(context).copyWith(disableAnimations: true),
              child: child!,
            )
          : null,
    ),
  );
}

/// A session whose cold snapshot read waits on [gate] — the cold-load frame.
class _GatedSnapshot extends FakeWalletSession {
  final gate = Completer<void>();

  @override
  Future<WalletState> snapshot() async {
    await gate.future;
    return super.snapshot();
  }
}

/// The host's persisted "hidden" choice, restored at launch (C6: the
/// provider is overridable so a host may persist it).
class _HiddenAtLaunch extends WalletBalanceHidden {
  @override
  bool build() => true;
}

final Override _hidden = walletBalanceHiddenProvider.overrideWith(
  _HiddenAtLaunch.new,
);

Finder _syncBar() =>
    find.byWidgetPredicate((w) => w.runtimeType.toString() == '_SyncBadge');

final Finder _card = find.byKey(const ValueKey('wallet-balance-card'));
final Finder _poolLine = find.byKey(const ValueKey('wallet-pool-line'));

/// A ZEC decimal as the SDK formats it (en, `.` separator) → zatoshis, by
/// integer math only (the money rule — never through a double).
int _zatOf(String decimal) {
  final parts = decimal.split('.');
  final whole = int.parse(parts[0]);
  final frac = parts.length == 1 ? 0 : int.parse(parts[1].padRight(8, '0'));
  return whole * zatPerZec + frac;
}

/// Every string drawn under [within] (or the whole tree), offstage included:
/// every `Text` and `Text.rich` builds a [RichText].
List<String> _drawn({Finder? within}) {
  final rich = within == null
      ? find.byType(RichText, skipOffstage: false)
      : find.descendant(
          of: within,
          matching: find.byType(RichText, skipOffstage: false),
          skipOffstage: false,
        );
  return [
    for (final e in rich.evaluate()) (e.widget as RichText).text.toPlainText(),
  ];
}

/// The ZEC figures the pool line draws, parsed from its text.
List<int> _poolFigures() {
  final text = _drawn(within: _poolLine).join(' ');
  return [
    for (final m in RegExp(r'\d+(?:\.\d+)?').allMatches(text))
      _zatOf(m.group(0)!),
  ];
}

/// How many items the open overflow menu holds.
int _menuItems() =>
    find.byWidgetPredicate((w) => w is PopupMenuItem).evaluate().length;

void main() {
  // ===================================================================== C3 ==
  group('C3 / rev.2 R4 — the card renders a row only when it says something '
      'the total does not, and what it renders accounts for the total', () {
    // rev.3: on a SYNCED wallet "the total" here is the
    // headline — what the wallet has now; "Arriving" sits outside it.
    // CONSISTENT fixtures only: total = spendable + pendingIncoming +
    // pendingChange + transparent (R4, `locked_value` = 0).
    final cases =
        <
          ({
            String name,
            int total,
            int spendable,
            int pendingIncoming,
            int pendingChange,
            int transparent,
            bool watchOnly,
            bool pool,
            bool spendableRow,
          })
        >[
          (
            name: 'everyday: all shielded, all spendable',
            total: 150000000,
            spendable: 150000000,
            pendingIncoming: 0,
            pendingChange: 0,
            transparent: 0,
            watchOnly: false,
            pool: false,
            spendableRow: false,
          ),
          (
            name: 'zero balance',
            total: 0,
            spendable: 0,
            pendingIncoming: 0,
            pendingChange: 0,
            transparent: 0,
            watchOnly: false,
            pool: false,
            spendableRow: false,
          ),
          // rev.3 N1: synced, arriving money leaves the headline — the
          // headline (0.7) IS what is spendable, so no Spendable row.
          (
            name: 'arriving: the headline is what you have now',
            total: 100000000,
            spendable: 70000000,
            pendingIncoming: 30000000,
            pendingChange: 0,
            transparent: 0,
            watchOnly: false,
            pool: false,
            spendableRow: false,
          ),
          (
            name: 'pending change: spendable != total',
            total: 100000000,
            spendable: 60000000,
            pendingIncoming: 0,
            pendingChange: 40000000,
            transparent: 0,
            watchOnly: false,
            pool: false,
            spendableRow: true,
          ),
          (
            name: 'transparent > 0: pool line, row, note, Shield',
            total: 100000000,
            spendable: 75000000,
            pendingIncoming: 0,
            pendingChange: 0,
            transparent: 25000000,
            watchOnly: false,
            pool: true,
            spendableRow: true,
          ),
          (
            name: 'every axis non-zero',
            total: 100000000,
            spendable: 40000000,
            pendingIncoming: 30000000,
            pendingChange: 20000000,
            transparent: 10000000,
            watchOnly: false,
            pool: true,
            spendableRow: true,
          ),
          // Synced, everything arriving: the headline is 0 and stands alone
          // (no "Spendable now: 0"); "Arriving +0.5" says what is coming.
          (
            name: 'nothing yet, everything arriving',
            total: 50000000,
            spendable: 0,
            pendingIncoming: 50000000,
            pendingChange: 0,
            transparent: 0,
            watchOnly: false,
            pool: false,
            spendableRow: false,
          ),
          (
            name: 'watch-only, all shielded, nothing pending',
            total: 100000000,
            spendable: 100000000,
            pendingIncoming: 0,
            pendingChange: 0,
            transparent: 0,
            watchOnly: true,
            pool: false,
            spendableRow: false,
          ),
          (
            name: 'watch-only with transparent',
            total: 100000000,
            spendable: 60000000,
            pendingIncoming: 0,
            pendingChange: 0,
            transparent: 40000000,
            watchOnly: true,
            pool: true,
            spendableRow: false,
          ),
          (
            name: 'watch-only with pending incoming AND transparent',
            total: 100000000,
            spendable: 50000000,
            pendingIncoming: 20000000,
            pendingChange: 0,
            transparent: 30000000,
            watchOnly: true,
            pool: true,
            spendableRow: false,
          ),
          // rev.3: arriving money is OUTSIDE the headline, so a watch-only
          // card with only arriving funds has no breakdown row to account
          // for, and no pool line — the S12 crypto review's "Shielded 1 /
          // Pending 0.3" misreading cannot occur.
          (
            name: 'watch-only with arriving, nothing transparent',
            total: 100000000,
            spendable: 70000000,
            pendingIncoming: 30000000,
            pendingChange: 0,
            transparent: 0,
            watchOnly: true,
            pool: false,
            spendableRow: false,
          ),
          (
            name: 'watch-only with pending change, nothing transparent',
            total: 100000000,
            spendable: 80000000,
            pendingIncoming: 0,
            pendingChange: 20000000,
            transparent: 0,
            watchOnly: true,
            pool: true,
            spendableRow: false,
          ),
        ];

    for (final c in cases) {
      testWidgets('${c.name}: only the rows it earns, and they account for '
          'the total', (tester) async {
        expect(
          c.spendable + c.pendingIncoming + c.pendingChange + c.transparent,
          c.total,
          reason: 'fixture must be consistent (R4)',
        );
        final fake = FakeWalletSession(
          current: const SyncStatus.upToDate(tip: 100),
          snapshotValue: walletStateFixture(
            syncStatus: const SyncStatus.upToDate(tip: 100),
            balance: balanceFixture(
              totalZat: c.total,
              spendableZat: c.spendable,
              pendingIncomingZat: c.pendingIncoming,
              pendingChangeZat: c.pendingChange,
              transparentZat: c.transparent,
            ),
            lastSynced: const SyncStamp(height: 100, at: 0),
          ),
        )..isWatchOnlyResult = c.watchOnly;
        await tester.pumpWidget(_harness(session: fake));
        await tester.pumpAndSettle();
        final l10n = _l10n(tester);

        Finder onCard(String text) =>
            find.descendant(of: _card, matching: find.text(text));

        // rev.3 N1: synced, the headline is what the wallet has NOW.
        final head = c.total - c.pendingIncoming;
        expect(
          tester
              .widget<Text>(find.byKey(const ValueKey('wallet-balance-total')))
              .data,
          l10n.walletAmount(formatZec(head)),
          reason: 'the headline is total − arriving',
        );

        // Which rows render.
        expect(_poolLine, c.pool ? findsOneWidget : findsNothing);
        expect(
          onCard(l10n.walletSpendableLabel),
          c.spendableRow ? findsOneWidget : findsNothing,
        );
        // Since S13 the breakdown row's word IS "Arriving" (the maintainer's
        // term), so the row's absence reads as: the card carries the word
        // exactly once, and that one is inside the keyed foot — never a
        // second time as a breakdown row.
        final arriving = find.byKey(const ValueKey('wallet-balance-arriving'));
        final arrivingOnce = c.pendingIncoming > 0
            ? findsOneWidget
            : findsNothing;
        expect(
          onCard(l10n.walletArrivingLabel),
          arrivingOnce,
          reason: 'synced: pending incoming is "Arriving", not a breakdown row',
        );
        expect(
          find.descendant(
            of: arriving,
            matching: find.text(l10n.walletArrivingLabel),
          ),
          arrivingOnce,
          reason: 'the one "Arriving" on the card is the foot\'s',
        );
        expect(arriving, arrivingOnce);
        if (c.pendingIncoming > 0) {
          expect(
            find.descendant(
              of: arriving,
              matching: find.text(
                l10n.walletAmount('+${formatZec(c.pendingIncoming)}'),
              ),
            ),
            findsOneWidget,
            reason: 'Arriving carries a + : outside the figure above',
          );
        }
        expect(
          onCard(l10n.walletPendingChangeLabel),
          c.pendingChange > 0 ? findsOneWidget : findsNothing,
        );
        expect(
          onCard(l10n.walletTransparentLabel),
          c.transparent > 0 ? findsOneWidget : findsNothing,
        );
        // The note travels with the transparent row; the spend-framed
        // variant only on a spending wallet.
        expect(
          onCard(l10n.walletTransparentNote),
          c.transparent > 0 && !c.watchOnly ? findsOneWidget : findsNothing,
        );
        expect(
          onCard(l10n.walletTransparentNoteWatchOnly),
          c.transparent > 0 && c.watchOnly ? findsOneWidget : findsNothing,
        );
        expect(
          find.byKey(const ValueKey('wallet-shield-button')),
          c.transparent > 0 && !c.watchOnly ? findsOneWidget : findsNothing,
        );
        // C3 dropped the "all shielded" affirmation from the everyday card.
        expect(find.text(l10n.walletPoolAllShielded), findsNothing);
        // A watch-only wallet NEVER shows Spendable (#397 D3).
        if (c.watchOnly) {
          expect(onCard(l10n.walletSpendableLabel), findsNothing);
        }

        // Pool axis: whenever the line shows, its figures are the pools and
        // sum EXACTLY to the headline — two figures when something is
        // transparent, the one shielded figure (= the headline) when nothing
        // is.
        if (c.pool) {
          final figures = _poolFigures();
          if (c.transparent > 0) {
            expect(figures, hasLength(2), reason: 'shielded and transparent');
            expect(figures, containsAll([head - c.transparent, c.transparent]));
          } else {
            expect(figures, [head], reason: 'the shielded figure alone');
            expect(
              find.descendant(
                of: _poolLine,
                matching: find.text(l10n.walletPoolShielded(formatZec(head))),
              ),
              findsOneWidget,
            );
          }
          expect(figures.reduce((a, b) => a + b), head);
        }

        // Breakdown axis: every rendered row's DRAWN figure is its amount,
        // and the rows sum to the HEADLINE whenever any renders ("Arriving"
        // is outside it, so it is not a breakdown row). A watch-only card
        // has no Spendable row, so it must carry the pool line instead.
        final rows = tester
            .widgetList<LabeledZatRow>(
              find.descendant(
                of: _card,
                matching: find.byWidgetPredicate(
                  (w) => w is LabeledZatRow && w.sign.isEmpty,
                ),
              ),
            )
            .toList();
        for (final r in rows) {
          expect(
            find.descendant(
              of: find.byWidget(r),
              matching: find.text(l10n.walletAmount(formatZec(r.amountZat))),
            ),
            findsOneWidget,
            reason: '${r.label} draws its own figure',
          );
        }
        if (rows.isNotEmpty) {
          if (c.watchOnly) {
            expect(
              _poolLine,
              findsOneWidget,
              reason:
                  'R4: a watch-only card with a breakdown row accounts for '
                  'the headline through the pool line',
            );
          } else {
            expect(
              rows.fold<int>(0, (s, r) => s + r.amountZat),
              head,
              reason: 'rev.3 N3: spendable + change + transparent = headline',
            );
          }
        }
        expect(
          find.descendant(of: _card, matching: find.byType(WalletCoin)),
          findsOneWidget,
          reason: 'the coin is on every card',
        );
      });
    }
  });

  // ===================================================================== C2 ==
  group('C2 / rev.2 R6 — walletSyncBarVisible (pure)', () {
    const upToDate = SyncStatus.upToDate(tip: 100);
    const pools = PoolServiceReport(
      sapling: PoolService.unsupported(),
      orchard: PoolService.served(roots: 7),
      ironwood: PoolService.served(roots: 0),
    );
    bool visible({
      SyncStatus status = upToDate,
      bool stale = false,
      bool driving = true,
      bool startFailed = false,
      bool syncDisabled = false,
      TransportTone tone = TransportTone.neutral,
      bool host = false,
    }) => walletSyncBarVisible(
      status: status,
      stale: stale,
      driving: driving,
      startFailed: startFailed,
      syncDisabled: syncDisabled,
      transportTone: tone,
      hostDeclaredTransport: host,
    );

    test(
      'HIDDEN only when synced and healthy: plain UpToDate, fresh, '
      'driving, over neutral (declared or not) or HOST-declared protected',
      () {
        expect(visible(), isFalse);
        expect(visible(host: true), isFalse);
        expect(visible(tone: TransportTone.protected, host: true), isFalse);
      },
    );

    final shown = <String, bool Function()>{
      'UpToDateLimited': () =>
          visible(status: const SyncStatus.upToDateLimited(tip: 100)),
      'UpToDateDegraded': () => visible(
        status: const SyncStatus.upToDateDegraded(tip: 100, pools: pools),
      ),
      'UpToDateUnverified': () => visible(
        status: const SyncStatus.upToDateUnverified(
          tip: 100,
          grace: UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400),
          streakReported: false,
        ),
      ),
      'EndpointBehind': () => visible(
        status: const SyncStatus.endpointBehind(tip: 100, newestKnown: 900),
      ),
      'Scanning, spendable-ready (badge ok)': () => visible(
        status: const SyncStatus.scanning(
          from: 1,
          to: 200,
          percent: 0.5,
          spendableReady: true,
          rewound: false,
        ),
      ),
      'Idle': () => visible(status: const SyncStatus.idle()),
      'Connecting': () => visible(status: const SyncStatus.connecting()),
      'Stalled': () => visible(
        status: const SyncStatus.stalled(
          reason: StallReason.endpointUnreachable,
        ),
      ),
      'Offline': () => visible(status: const SyncStatus.offline()),
      'Unknown': () => visible(status: const SyncStatus.unknown()),
      'stale snapshot': () => visible(stale: true),
      'drive not running': () => visible(driving: false),
      'start failed': () => visible(startFailed: true),
      'sync disabled by host': () => visible(syncDisabled: true),
      'caution transport': () => visible(tone: TransportTone.caution),
      'danger transport': () => visible(tone: TransportTone.danger),
      'progress transport': () => visible(tone: TransportTone.progress),
      'SDK-derived protected (no host declaration, R6)': () =>
          visible(tone: TransportTone.protected),
      'caution transport even with a host declaration': () =>
          visible(tone: TransportTone.caution, host: true),
    };
    for (final e in shown.entries) {
      test('VISIBLE: ${e.key}', () => expect(e.value(), isTrue));
    }
  });

  group('C2 — the overflow menu carries Sync status exactly when the bar is '
      'hidden, and it opens the sync sheet', () {
    Future<({bool bar, int items, bool entry})> probe(
      WidgetTester tester, {
      required SyncStatus status,
      TorState tor = const TorState.off(),
      WalletHostTransport? host,
      bool syncOff = false,
      bool stale = false,
    }) async {
      final fake = FakeWalletSession(
        current: status,
        snapshotValue: walletStateFixture(
          syncStatus: status,
          tor: tor,
          lastSynced: const SyncStamp(height: 100, at: 0),
        ),
      );
      // A fresh ProviderScope (and so a fresh container) per probe: a scope
      // re-pumped in place keeps its providers' state and may not change
      // its override count.
      await tester.pumpWidget(
        KeyedSubtree(
          key: UniqueKey(),
          child: _harness(
            session: fake,
            extraOverrides: [
              if (host != null)
                walletHostTransportProvider.overrideWithValue(host),
              if (syncOff) walletSyncPolicyProvider.overrideWithValue(false),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();
      if (stale) {
        fake.snapshotThrows = true;
        _container(tester).invalidate(walletSnapshotReadProvider);
        await tester.pumpAndSettle();
      }
      final l10n = _l10n(tester);
      final bar = _syncBar().evaluate().isNotEmpty;
      await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
      await tester.pumpAndSettle();
      final items = _menuItems();
      final entry = find
          .descendant(
            of: find.byWidgetPredicate((w) => w is PopupMenuItem),
            matching: find.text(l10n.walletSyncUpToDate),
          )
          .evaluate()
          .isNotEmpty;
      // Close the menu so the next pump starts clean.
      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      return (bar: bar, items: items, entry: entry);
    }

    testWidgets('hidden ⇔ entry, across healthy and non-healthy states', (
      tester,
    ) async {
      const upToDate = SyncStatus.upToDate(tip: 100);
      final healthy = await probe(tester, status: upToDate);
      expect(healthy.bar, isFalse, reason: 'UpToDate over Tor off is healthy');
      expect(healthy.entry, isTrue);

      final hostProtected = await probe(
        tester,
        status: upToDate,
        host: const WalletHostTransport(label: 'VLESS', protection: true),
      );
      expect(hostProtected.bar, isFalse, reason: 'host-declared protected');
      expect(hostProtected.entry, isTrue);

      final visibleCases = {
        'scanning': await probe(
          tester,
          status: const SyncStatus.scanning(
            from: 1,
            to: 200,
            percent: 0.5,
            spendableReady: true,
            rewound: false,
          ),
        ),
        'caution transport': await probe(
          tester,
          status: upToDate,
          tor: const TorState.fellBack(),
        ),
        'SDK-derived protected (R6)': await probe(
          tester,
          status: upToDate,
          tor: const TorState.active(runtime: TorRuntimeKind.externalSocks5()),
        ),
        'sync off': await probe(tester, status: upToDate, syncOff: true),
        'stale snapshot': await probe(tester, status: upToDate, stale: true),
      };
      for (final e in visibleCases.entries) {
        expect(e.value.bar, isTrue, reason: '${e.key}: the bar shows');
        expect(e.value.entry, isFalse, reason: '${e.key}: no menu entry');
        expect(
          e.value.items,
          healthy.items - 1,
          reason: '${e.key}: exactly ONE item (Sync status) differs',
        );
      }
    });

    testWidgets('the menu entry opens the sync-status sheet', (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(_syncBar(), findsNothing);
      expect(find.byType(SyncStatusSheet), findsNothing);

      await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSyncUpToDate));
      await tester.pumpAndSettle();

      expect(find.byType(SyncStatusSheet), findsOneWidget);
      expect(find.text(l10n.walletSyncExplainUpToDate), findsOneWidget);
    });

    testWidgets('under reduce-motion the bar hides and returns at once, with '
        'no layout assertion (C2: instant under reduced motion)', (
      tester,
    ) async {
      const scanning = SyncStatus.scanning(
        from: 1,
        to: 2000,
        percent: 0.5,
        spendableReady: false,
        rewound: false,
      );
      final fake = FakeWalletSession(
        current: scanning,
        snapshotValue: walletStateFixture(
          syncStatus: scanning,
          lastSynced: const SyncStamp(height: 1000, at: 0),
        ),
      );
      await tester.pumpWidget(_harness(session: fake, disableAnimations: true));
      await tester.pumpAndSettle();
      expect(_syncBar(), findsOneWidget, reason: 'precondition: scanning');

      fake.push(const SyncStatus.upToDate(tip: 2000));
      // ONE frame: instant means no 250 ms collapse to wait out.
      await tester.pump();
      await tester.pump();
      expect(tester.takeException(), isNull);
      expect(_syncBar(), findsNothing, reason: 'synced and healthy: hidden');
      expect(find.byType(AnimatedSize), findsNothing);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);

      // A real catch-up (far past the synced tip), not a routine one-block
      // follow, which keeps the bar hidden by design.
      fake.push(
        const SyncStatus.scanning(
          from: 2000,
          to: 2500,
          percent: 0.5,
          spendableReady: false,
          rewound: false,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(tester.takeException(), isNull);
      expect(_syncBar(), findsOneWidget, reason: 'catching up again: shown');
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(_syncBar(), findsOneWidget);
    });
  });

  // ============================================================= C3 — coin ==
  group('C3 / rev.2 R2 R3 — the coin spins once per earned event', () {
    const scanning = SyncStatus.scanning(
      from: 1,
      to: 2000,
      percent: 0.5,
      spendableReady: false,
      rewound: false,
    );

    bool spinning(WidgetTester tester) =>
        tester.state<WalletCoinState>(find.byType(WalletCoin)).spinning;
    int bumps(WidgetTester tester) =>
        _container(tester).read(walletCoinSpinsProvider);

    /// Mount at [start]; the stamp at 1000 keeps the catch-up cue at None.
    Future<FakeWalletSession> mount(
      WidgetTester tester,
      SyncStatus start, {
      bool disableAnimations = false,
      List<Override> extraOverrides = const [],
    }) async {
      final fake = FakeWalletSession(
        current: start,
        snapshotValue: walletStateFixture(
          syncStatus: start,
          balance: balanceFixture(totalZat: 100000000, spendableZat: 100000000),
          lastSynced: const SyncStamp(height: 1000, at: 0),
        ),
      );
      await tester.pumpWidget(
        _harness(
          session: fake,
          disableAnimations: disableAnimations,
          extraOverrides: extraOverrides,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(spinning(tester), isFalse, reason: 'never on the first build');
      await tester.pumpAndSettle();
      expect(bumps(tester), 0, reason: 'the first sample earns no spin');
      return fake;
    }

    testWidgets('Scanning → UpToDate with the drive running spins, once, '
        'for 1.4 s', (tester) async {
      final fake = await mount(tester, scanning);
      fake.push(const SyncStatus.upToDate(tip: 2000));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 1);
      expect(spinning(tester), isTrue);
      await tester.pump(const Duration(milliseconds: 1399));
      expect(spinning(tester), isTrue, reason: 'the spin runs its 1.4 s');
      await tester.pump(const Duration(milliseconds: 2));
      expect(spinning(tester), isFalse, reason: 'and stops after one turn');
    });

    testWidgets(
      'never on the first build, even into an already-synced wallet',
      (tester) async {
        await mount(tester, const SyncStatus.upToDate(tip: 2000));
        expect(spinning(tester), isFalse);
      },
    );

    testWidgets('never UpToDate → UpToDate (a new tip is not an entry)', (
      tester,
    ) async {
      final fake = await mount(tester, const SyncStatus.upToDate(tip: 2000));
      fake.push(const SyncStatus.upToDate(tip: 2001));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 0);
      expect(spinning(tester), isFalse);
    });

    testWidgets('a routine one-block pass (UpToDate → Scanning → UpToDate, as '
        'the SDK emits it every block) neither spins the coin nor shows the '
        'bar (S12 security review MEDIUM)', (tester) async {
      final fake = await mount(tester, const SyncStatus.upToDate(tip: 2000));
      expect(_syncBar(), findsNothing, reason: 'precondition: synced');
      // The pass's first download tick carries the pre-batch summary
      // {0, false}: spendableReady is false even on a funded wallet.
      fake.push(
        const SyncStatus.scanning(
          from: 1999,
          to: 2001,
          percent: 0,
          spendableReady: false,
          rewound: false,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(_syncBar(), findsNothing, reason: 'following the tip: hidden');
      fake.push(const SyncStatus.upToDate(tip: 2001));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 0, reason: 'a new block is not an entry');
      expect(spinning(tester), isFalse);
      expect(_syncBar(), findsNothing);
    });

    testWidgets('a real catch-up (far past the synced tip) shows the bar and '
        'spins once when it completes', (tester) async {
      final fake = await mount(tester, const SyncStatus.upToDate(tip: 2000));
      fake.push(
        const SyncStatus.scanning(
          from: 1999,
          to: 2000 + kWalletTipFollowBlocks + 1,
          percent: 0.5,
          spendableReady: true,
          rewound: false,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(_syncBar(), findsOneWidget, reason: 'catching up: shown');
      fake.push(
        const SyncStatus.upToDate(tip: 2000 + kWalletTipFollowBlocks + 1),
      );
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 1);
      await tester.pumpAndSettle();
      expect(_syncBar(), findsNothing);
    });

    testWidgets('a rewound scan near the tip is not a follow: the bar shows '
        'and the completion spins', (tester) async {
      final fake = await mount(tester, const SyncStatus.upToDate(tip: 2000));
      fake.push(
        const SyncStatus.scanning(
          from: 1990,
          to: 2001,
          percent: 0.9,
          spendableReady: true,
          rewound: true,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(_syncBar(), findsOneWidget);
      fake.push(const SyncStatus.upToDate(tip: 2001));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 1);
      await tester.pumpAndSettle();
    });

    const pools = PoolServiceReport(
      sapling: PoolService.unsupported(),
      orchard: PoolService.served(roots: 7),
      ironwood: PoolService.served(roots: 0),
    );
    final qualified = <String, SyncStatus>{
      'UpToDateLimited': const SyncStatus.upToDateLimited(tip: 2000),
      'UpToDateDegraded': const SyncStatus.upToDateDegraded(
        tip: 2000,
        pools: pools,
      ),
      'EndpointBehind': const SyncStatus.endpointBehind(
        tip: 2000,
        newestKnown: 3000,
      ),
      'UpToDateUnverified': const SyncStatus.upToDateUnverified(
        tip: 2000,
        grace: UnknownBranchGrace.running(blocksLeft: 1000, secsLeft: 6400),
        streakReported: false,
      ),
    };
    for (final q in qualified.entries) {
      testWidgets('never Scanning → ${q.key} (a caution state is not '
          'celebrated, R2)', (tester) async {
        final fake = await mount(tester, scanning);
        fake.push(q.value);
        await tester.pump();
        await tester.pump();
        expect(bumps(tester), 0);
        expect(spinning(tester), isFalse);
      });
    }

    testWidgets('never while sync is OFF by host policy (the drive is not '
        'running, R2)', (tester) async {
      final fake = await mount(
        tester,
        scanning,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      );
      fake.push(const SyncStatus.upToDate(tip: 2000));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 0);
      expect(spinning(tester), isFalse);
    });

    testWidgets('never while the sync START failed (R2)', (tester) async {
      final fake = FakeWalletSession(
        current: scanning,
        snapshotValue: walletStateFixture(
          syncStatus: scanning,
          lastSynced: const SyncStamp(height: 1000, at: 0),
        ),
      )..failStart = true;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      fake.push(const SyncStatus.upToDate(tip: 2000));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 0);
      expect(spinning(tester), isFalse);
    });

    testWidgets('never over a STALE snapshot (badge not ok, R2)', (
      tester,
    ) async {
      final fake = await mount(tester, scanning);
      fake.snapshotThrows = true;
      _container(tester).invalidate(walletSnapshotReadProvider);
      await tester.pumpAndSettle();
      expect(
        _container(tester).read(walletSnapshotProvider).hasError,
        isTrue,
        reason: 'precondition: the snapshot is stale',
      );
      fake.push(const SyncStatus.upToDate(tip: 2000));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 0);
      expect(spinning(tester), isFalse);
    });

    testWidgets('never under the platform reduce-motion setting — the SYNC '
        'trigger (the request is counted, the coin stays still)', (
      tester,
    ) async {
      final fake = await mount(tester, scanning, disableAnimations: true);
      fake.push(const SyncStatus.upToDate(tip: 2000));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 1, reason: 'the event was earned');
      expect(spinning(tester), isFalse, reason: 'disableAnimations wins');
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });

    testWidgets('never under the platform reduce-motion setting — the ARRIVAL '
        'trigger (the request is counted, the coin stays still)', (
      tester,
    ) async {
      final fake = await mount(
        tester,
        const SyncStatus.upToDate(tip: 2000),
        disableAnimations: true,
      );
      fake.pushIncoming(
        const IncomingFundsEvent(
          kind: IncomingFundsEventKind.live,
          newTxCount: 1,
          totalTxDetected: 1,
          spanFromHeight: 2001,
          spanToHeight: 2001,
          cursor: 'w1:2001',
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 1, reason: 'the event was earned');
      expect(spinning(tester), isFalse, reason: 'disableAnimations wins');
    });

    testWidgets('a second event mid-spin neither restarts nor queues', (
      tester,
    ) async {
      final fake = await mount(tester, scanning);
      fake.push(const SyncStatus.upToDate(tip: 2000));
      await tester.pump();
      await tester.pump();
      expect(spinning(tester), isTrue);
      await tester.pump(const Duration(milliseconds: 700));

      // A live arrival newer than the latched tip earns its own spin — but
      // one is already in flight.
      fake.pushIncoming(
        const IncomingFundsEvent(
          kind: IncomingFundsEventKind.live,
          newTxCount: 1,
          totalTxDetected: 1,
          spanFromHeight: 2001,
          spanToHeight: 2001,
          cursor: 'w1:2001',
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 2, reason: 'precondition: the second event fired');
      expect(spinning(tester), isTrue);

      // 1.4 s after the FIRST start (700 + 701 ms): a restart would still
      // be turning.
      await tester.pump(const Duration(milliseconds: 701));
      expect(spinning(tester), isFalse, reason: 'no restart');
      await tester.pump(const Duration(milliseconds: 100));
      expect(spinning(tester), isFalse, reason: 'no queued second spin');
    });

    group('arrival (R3)', () {
      Future<FakeWalletSession> synced(
        WidgetTester tester, {
        List<Override> extraOverrides = const [],
      }) async {
        final fake = await mount(
          tester,
          const SyncStatus.upToDate(tip: 2000),
          extraOverrides: extraOverrides,
        );
        final latch = _container(tester).read(walletSyncedTipProvider);
        expect(latch, isA<WalletSyncedTipLatched>());
        expect((latch as WalletSyncedTipLatched).tip, 2000);
        return fake;
      }

      IncomingFundsEvent event({
        IncomingFundsEventKind kind = IncomingFundsEventKind.live,
        int newTxCount = 1,
        int? spanTo = 2050,
      }) => IncomingFundsEvent(
        kind: kind,
        newTxCount: newTxCount,
        totalTxDetected: newTxCount,
        spanFromHeight: spanTo,
        spanToHeight: spanTo,
        cursor: 'w1:${spanTo ?? 0}:$kind',
      );

      testWidgets('spins on a live arrival above the latched synced tip', (
        tester,
      ) async {
        final fake = await synced(tester);
        fake.pushIncoming(event());
        await tester.pump();
        await tester.pump();
        expect(bumps(tester), 1);
        expect(spinning(tester), isTrue);
      });

      testWidgets('never on a REPLAY (a catch-up is old news)', (tester) async {
        final fake = await synced(tester);
        fake.pushIncoming(event(kind: IncomingFundsEventKind.replay));
        await tester.pump();
        await tester.pump();
        expect(bumps(tester), 0);
        expect(spinning(tester), isFalse);
      });

      testWidgets('never when the span ends AT the latched tip (history, not '
          'news)', (tester) async {
        final fake = await synced(tester);
        fake.pushIncoming(event(spanTo: 2000));
        await tester.pump();
        await tester.pump();
        expect(bumps(tester), 0);
      });

      testWidgets('never when the span ends BELOW the latched tip', (
        tester,
      ) async {
        final fake = await synced(tester);
        fake.pushIncoming(event(spanTo: 1500));
        await tester.pump();
        await tester.pump();
        expect(bumps(tester), 0);
      });

      testWidgets('never on a count-0 advisory (the conservative fire)', (
        tester,
      ) async {
        final fake = await synced(tester);
        fake.pushIncoming(event(newTxCount: 0));
        await tester.pump();
        await tester.pump();
        expect(bumps(tester), 0);
      });

      testWidgets('never while a catch-up is in progress (a rescan '
          'rebuilding replays history as live)', (tester) async {
        final fake = await synced(
          tester,
          extraOverrides: [
            walletRescanControllerProvider.overrideWith(
              () => _FixedRescan(
                const WalletRescanRebuilding(target: RescanAllHistory()),
              ),
            ),
          ],
        );
        expect(
          _container(tester).read(walletCatchUpCueProvider),
          isA<WalletCatchUpRebuilding>(),
          reason: 'precondition: a catch-up is in progress',
        );
        fake.pushIncoming(event());
        await tester.pump();
        await tester.pump();
        expect(bumps(tester), 0);
      });
    });
  });

  // ===================================================================== C6 ==
  group('C6 / rev.2 R1 R5 — hide balance masks every GLANCE surface and '
      'never a surface the user acts on', () {
    // Distinct digit runs, so a figure found anywhere is attributable.
    const spendable = 31415926;
    const pendingIn = 4669201;
    const pendingChange = 1618033;
    const transparent = 5772156;
    const total = spendable + pendingIn + pendingChange + transparent;
    const shielded = total - transparent;
    const incoming = 12345678;
    const outgoing = -8765432;
    const fee = 13579;
    const inFlight = 9090909;
    const recoverable = 2222222;
    const parked = 6060606; // NEVER masked (R1)

    /// Every string a masked [zat] could leak as: its SDK figure and, for a
    /// sub-1 figure, its significant digits alone.
    List<String> leaks(int zat) {
      final f = formatZec(zat.abs());
      return [f, if (f.startsWith('0.')) f.substring(2)];
    }

    final masked = <String, int>{
      'total': total,
      'spendable': spendable,
      'pending incoming': pendingIn,
      'pending change': pendingChange,
      'transparent': transparent,
      'shielded (pool line)': shielded,
      'activity incoming': incoming,
      'activity outgoing': outgoing,
      'tx fee': fee,
      'in-flight': inFlight,
      'recoverable note': recoverable,
    };

    FakeWalletSession loaded() =>
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 100),
              balance: balanceFixture(
                totalZat: total,
                spendableZat: spendable,
                pendingIncomingZat: pendingIn,
                pendingChangeZat: pendingChange,
                transparentZat: transparent,
              ),
              lastSynced: const SyncStamp(height: 100, at: 0),
            ),
          )
          ..transactionsResult = [
            // Pending, no timestamp: its row label carries no digit of its
            // own, so "no digit at all" is a checkable claim for it.
            txSummaryFixture(
              txidHex:
                  'aa00000000000000000000000000000000000000000000000000000000000001',
              netAmountZat: incoming,
              status: const TxStatus.pending(),
              minedHeight: null,
              timestamp: null,
              hasMemo: false,
            ),
            txSummaryFixture(
              txidHex:
                  'bb00000000000000000000000000000000000000000000000000000000000002',
              netAmountZat: outgoing,
              feeZat: fee,
              hasMemo: false,
            ),
          ]
          ..inFlightSendsResult = [inFlightSendFixture(amountZat: inFlight)]
          ..parkedSendsResult = [parkedSendFixture(amountZat: parked)]
          ..recoverableEphemeralFundsResult = const [
            RecoverableEphemeralFunds(
              recoverableZat: recoverable,
              isFinal: true,
            ),
          ];

    Future<FakeWalletSession> pumpHidden(WidgetTester tester) async {
      // Tall enough that every section is built (the ListView is lazy).
      tester.view.physicalSize = const Size(800, 4000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);
      final fake = loaded();
      await tester.pumpWidget(
        _harness(session: fake, extraOverrides: [_hidden]),
      );
      await tester.pumpAndSettle();
      return fake;
    }

    testWidgets('no digit of a masked amount is drawn anywhere on the tab', (
      tester,
    ) async {
      await pumpHidden(tester);
      final drawn = _drawn();
      for (final m in masked.entries) {
        for (final leak in leaks(m.value)) {
          expect(
            drawn.where((s) => s.contains(leak)),
            isEmpty,
            reason: '${m.key} ($leak) must not be drawn while hidden',
          );
        }
      }
    });

    testWidgets('each glance surface shows the dots in the figure\'s place', (
      tester,
    ) async {
      await pumpHidden(tester);
      final l10n = _l10n(tester);

      // The card: the total, and every breakdown row (all four earn a row).
      expect(
        tester
            .widget<Text>(find.byKey(const ValueKey('wallet-balance-total')))
            .data,
        maskedAmountText,
      );
      final rows = find.descendant(
        of: _card,
        matching: find.byType(LabeledZatRow),
      );
      expect(rows, findsNWidgets(4), reason: 'spendable, change, T, arriving');
      for (final r in tester.widgetList<LabeledZatRow>(rows)) {
        expect(
          find.descendant(
            of: find.byWidget(r),
            // Arriving keeps its `+` masked (rev.3): the row already says
            // funds are on their way; no digit or length leaks.
            matching: find.text('${r.sign}$maskedAmountText'),
          ),
          findsOneWidget,
          reason: '${r.label} masks its figure',
        );
      }
      // The pool line: both figures.
      expect(
        maskedAmountText.allMatches(_drawn(within: _poolLine).join()).length,
        2,
      );
      // The recoverable note (R1 lists it among the card's figures).
      expect(
        find.text(l10n.walletRecoverableEphemeralNote(maskedAmountText)),
        findsOneWidget,
      );
      // Activity: both rows, inside the group.
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('wallet-activity-group')),
          matching: find.text(maskedAmountText),
        ),
        findsNWidgets(2),
      );
      // The in-flight cue.
      expect(
        find.text(l10n.walletInFlightNote(1, maskedAmountText)),
        findsOneWidget,
      );
    });

    testWidgets('NEVER masked (R1): a parked row keeps its amount and its Send '
        'now — it signs with no confirm screen', (tester) async {
      await pumpHidden(tester);
      final l10n = _l10n(tester);
      expect(
        _drawn().where((s) => s.contains(l10n.walletAmount(formatZec(parked)))),
        isNotEmpty,
        reason: 'the parked amount is the row\'s identity',
      );
      expect(find.text(l10n.walletParkedSendNow), findsOneWidget);
    });

    testWidgets('screen readers hear "Balance hidden", never a masked figure '
        '(R5)', (tester) async {
      final handle = tester.ensureSemantics();
      await pumpHidden(tester);
      final l10n = _l10n(tester);

      // The card's plain texts merge into one node (header, total, notes);
      // the total's line in it is the state. That the total's figure is
      // absent from it is pinned by the no-figure sweep below.
      final totalNode = tester.getSemantics(
        find.byKey(const ValueKey('wallet-balance-total')),
      );
      expect(
        totalNode.label.split('\n'),
        contains(l10n.walletBalanceHiddenAmount),
      );
      expect(
        totalNode.label.split('\n'),
        isNot(contains(maskedAmountText)),
        reason: 'the total is announced as the state, not as its dots',
      );
      // The pool line's node too — both segments, each with the state in its
      // figure's place, so a screen-reader user still learns that part of
      // the balance is transparent (S12 diff review LOW), and no digit.
      final hiddenWord = l10n.walletBalanceHiddenAmount;
      final poolLabel = tester.getSemantics(_poolLine).label;
      expect(
        poolLabel,
        '${l10n.walletPoolShielded(hiddenWord)}, '
        '${l10n.walletPoolTransparent(hiddenWord)}',
      );
      expect(poolLabel, isNot(matches(RegExp(r'\d'))));
      expect(poolLabel, isNot(contains(maskedAmountText)));
      // The incoming activity row: its composed label says "Balance hidden"
      // and — pending, no timestamp — carries no digit at all.
      final received = find.semantics.byPredicate(
        (n) => n.label.startsWith('${l10n.walletActivityReceived} '),
      );
      expect(received, findsOne);
      final receivedLabel = received.evaluate().single.label;
      expect(receivedLabel, contains(l10n.walletBalanceHiddenAmount));
      expect(receivedLabel, isNot(matches(RegExp(r'\d'))));
      // The outgoing row says the state too.
      final sent = find.semantics.byPredicate(
        (n) => n.label.startsWith('${l10n.walletActivitySent} '),
      );
      expect(sent, findsOne);
      expect(
        sent.evaluate().single.label,
        contains(l10n.walletBalanceHiddenAmount),
      );
      // And no node anywhere announces a masked figure.
      for (final m in masked.entries) {
        for (final leak in leaks(m.value)) {
          expect(
            find.semantics.byPredicate(
              (n) => n.label.contains(leak) || n.value.contains(leak),
            ),
            findsNothing,
            reason: '${m.key} ($leak) must not be announced while hidden',
          );
        }
      }

      // R5: the sentences that CARRY a masked amount say "Balance hidden"
      // in its place, and have no digit of their own left to speak.
      // A node's label is its merged lines; match per line.
      SemanticsFinder lineNode(String line) =>
          find.semantics.byPredicate((n) => n.label.split('\n').contains(line));
      final inFlightLine = l10n.walletInFlightNote(
        1,
        l10n.walletBalanceHiddenAmount,
      );
      final recoverableLine = l10n.walletRecoverableEphemeralNote(
        l10n.walletBalanceHiddenAmount,
      );
      for (final line in [inFlightLine, recoverableLine]) {
        expect(lineNode(line), findsOne, reason: line);
        expect(line, contains(l10n.walletBalanceHiddenAmount));
        expect(line, isNot(matches(RegExp(r'\d'))), reason: line);
      }
      // And no node on the tab reads the bullets aloud.
      expect(
        find.semantics.byPredicate(
          (n) =>
              n.label.contains(maskedAmountText) ||
              n.value.contains(maskedAmountText),
        ),
        findsNothing,
      );
      handle.dispose();
    });

    testWidgets('the tx-detail sheet masks its amount and its fee', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await pumpHidden(tester);
      final l10n = _l10n(tester);

      await tester.tap(find.text(l10n.walletActivitySent));
      await tester.pumpAndSettle();
      final sheet = find.byType(TxDetailSheet);
      expect(sheet, findsOneWidget);

      final drawn = _drawn(within: sheet);
      for (final leak in [...leaks(outgoing), ...leaks(fee)]) {
        expect(drawn.where((s) => s.contains(leak)), isEmpty, reason: leak);
      }
      final dots = find.descendant(
        of: sheet,
        matching: find.text(maskedAmountText),
      );
      expect(dots, findsNWidgets(2), reason: 'the amount and the fee');
      // The headline amount is announced as the state.
      expect(
        tester.getSemantics(dots.first).label,
        contains(l10n.walletBalanceHiddenAmount),
      );
      // The fee row (R5): its node says the state after its label, with no
      // digit in it.
      final feeNode = find.semantics.byPredicate(
        (n) => n.label.split('\n').first == l10n.walletTxDetailFee,
      );
      expect(feeNode, findsOne);
      final feeLabel = feeNode.evaluate().single.label;
      expect(feeLabel, contains(l10n.walletBalanceHiddenAmount));
      expect(feeLabel, isNot(matches(RegExp(r'\d'))));
      // And nothing on the sheet reads the bullets aloud.
      expect(
        find.semantics.byPredicate(
          (n) =>
              n.label.contains(maskedAmountText) ||
              n.value.contains(maskedAmountText),
        ),
        findsNothing,
      );
      handle.dispose();
    });

    testWidgets('the header eye toggles the provider; its tooltip names what '
        'a press WILL do', (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
          balance: balanceFixture(totalZat: total, spendableZat: total),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      final eye = find.byKey(const ValueKey('wallet-hide-balance'));
      final totalText = find.byKey(const ValueKey('wallet-balance-total'));

      expect(_container(tester).read(walletBalanceHiddenProvider), isFalse);
      expect(find.byTooltip(l10n.walletHideBalance), findsOneWidget);
      expect(
        tester.widget<Text>(totalText).data,
        l10n.walletAmount(formatZec(total)),
      );

      await tester.tap(eye);
      await tester.pumpAndSettle();
      expect(_container(tester).read(walletBalanceHiddenProvider), isTrue);
      expect(find.byTooltip(l10n.walletShowBalance), findsOneWidget);
      expect(find.byTooltip(l10n.walletHideBalance), findsNothing);
      expect(tester.widget<Text>(totalText).data, maskedAmountText);

      await tester.tap(eye);
      await tester.pumpAndSettle();
      expect(_container(tester).read(walletBalanceHiddenProvider), isFalse);
      expect(find.byTooltip(l10n.walletHideBalance), findsOneWidget);
      expect(
        tester.widget<Text>(totalText).data,
        l10n.walletAmount(formatZec(total)),
      );
    });

    testWidgets('NEVER masked (R1): the Send form\'s Available line, the '
        'typed amount and the review totals', (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 1),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 1),
          balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
        ),
      )..proposeResult = sendProposalFixture(totalZat: 100500, feeZat: 500);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(fake), _hidden],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const SendScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SendScreen)),
      );
      expect(
        ProviderScope.containerOf(
          tester.element(find.byType(SendScreen)),
        ).read(walletBalanceHiddenProvider),
        isTrue,
        reason: 'precondition: hidden',
      );
      expect(
        find.text(l10n.walletSendAvailable(formatZec(500000000))),
        findsOneWidget,
      );

      await tester.enterText(find.byType(TextField).at(0), 'u1recipient');
      await tester.enterText(find.byType(TextField).at(1), '1');
      await tester.pump();
      expect(find.text('1'), findsWidgets, reason: 'the typed amount shows');
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletSendReviewTitle), findsOneWidget);
      expect(
        find.text(l10n.walletAmount(formatZec(100500))),
        findsWidgets,
        reason: 'the total under review',
      );
      expect(
        find.text(l10n.walletAmount(formatZec(500))),
        findsOneWidget,
        reason: 'the fee under review',
      );
      expect(find.text(maskedAmountText), findsNothing);
    });

    testWidgets('NEVER masked (R1): the shield sheet\'s gross, fee and net', (
      tester,
    ) async {
      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(
          totalZat: 500000,
          feeZat: 15000,
        );
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(session),
            _hidden,
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
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ShieldSheet)),
      );
      for (final zat in [500000, 15000, 485000]) {
        expect(find.text(l10n.walletAmount(formatZec(zat))), findsOneWidget);
      }
      expect(find.text(maskedAmountText), findsNothing);
    });
  });

  // ===================================================================== C4 ==
  group('C4 — the action tiles', () {
    const swapPolicy = SwapHostPolicy(
      config: SwapProviderConfig(endpoint: 'https://1click.test', jwt: 'tok'),
      enabled: true,
      declaredKill: SwapKill.windDown,
    );

    Future<void> pumpTiles(
      WidgetTester tester, {
      required int spendable,
    }) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 100),
          balance: balanceFixture(totalZat: spendable, spendableZat: spendable),
          lastSynced: const SyncStamp(height: 100, at: 0),
        ),
      );
      await tester.pumpWidget(
        KeyedSubtree(
          key: UniqueKey(),
          child: _harness(
            session: fake,
            extraOverrides: [
              swapHostPolicyProvider.overrideWithValue(swapPolicy),
              swapActivationProvider.overrideWith((ref) async => true),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    double opacityOf(WidgetTester tester, Finder tile) => tester
        .widget<Opacity>(
          find.ancestor(of: tile, matching: find.byType(Opacity)).first,
        )
        .opacity;

    testWidgets('Send tonal, Receive primary, Swap tonal — 88 high, full '
        'opacity when enabled', (tester) async {
      await pumpTiles(tester, spendable: 100000000);
      final l10n = _l10n(tester);
      final colors = WalletColors.of(tester.element(find.byType(WalletScreen)));

      final roles = {
        'send': (
          key: const ValueKey('wallet-action-send'),
          label: l10n.walletSendButton,
          fill: colors.accentSoft,
          ink: colors.accentText,
        ),
        'receive': (
          key: const ValueKey('wallet-action-receive'),
          label: l10n.walletReceive,
          fill: colors.accent,
          ink: colors.onAccent,
        ),
        'swap': (
          key: const ValueKey('wallet-action-swap'),
          label: l10n.walletSwapButton,
          fill: colors.accentSoft,
          ink: colors.accentText,
        ),
      };
      for (final r in roles.entries) {
        final tile = find.byKey(r.value.key);
        expect(tile, findsOneWidget, reason: r.key);
        expect(
          tester.widget<Material>(tile).color,
          r.value.fill,
          reason: r.key,
        );
        expect(
          tester
              .widget<Text>(
                find.descendant(of: tile, matching: find.text(r.value.label)),
              )
              .style
              ?.color,
          r.value.ink,
          reason: '${r.key} label ink',
        );
        expect(opacityOf(tester, tile), 1.0, reason: r.key);
        expect(
          tester.getSize(tile).height,
          greaterThanOrEqualTo(88),
          reason: '${r.key}: 88 high (≥ the 48 touch minimum)',
        );
      }
    });

    testWidgets('a disabled Send is the SAME tonal tile at 38 % opacity, its '
        'reason shown in the space reserved for it', (tester) async {
      await pumpTiles(tester, spendable: 100000000);
      final l10n = _l10n(tester);
      final activityTopEnabled = tester
          .getTopLeft(find.text(l10n.walletActivityTitle))
          .dy;

      await pumpTiles(tester, spendable: 0);
      final colors = WalletColors.of(tester.element(find.byType(WalletScreen)));
      final send = find.byKey(const ValueKey('wallet-action-send'));
      expect(opacityOf(tester, send), 0.38);
      expect(tester.widget<Material>(send).color, colors.accentSoft);
      expect(
        tester
            .widget<InkWell>(
              find.descendant(of: send, matching: find.byType(InkWell)),
            )
            .onTap,
        isNull,
      );
      expect(find.text(l10n.walletSendNoSpendableYet), findsOneWidget);
      expect(
        tester.getTopLeft(find.text(l10n.walletActivityTitle)).dy,
        activityTopEnabled,
        reason: 'the reason line was reserved: nothing below moves',
      );
    });
  });

  // ===================================================================== C5 ==
  group('C5 — the activity group', () {
    testWidgets('rows sit in the group, ≥ 64 high; the signed amount is set '
        'in the registered mono face, + / accent in, U+2212 / text out', (
      tester,
    ) async {
      const mono = 'RegisteredMono';
      final theme = buildTheme(
        WalletColors.light,
        extensions: const [WalletTypography(mono: TextStyle(fontFamily: mono))],
      );
      final fake =
          FakeWalletSession(
              current: const SyncStatus.upToDate(tip: 100),
              snapshotValue: walletStateFixture(
                syncStatus: const SyncStatus.upToDate(tip: 100),
                balance: balanceFixture(
                  totalZat: 100000000,
                  spendableZat: 100000000,
                ),
                lastSynced: const SyncStamp(height: 100, at: 0),
              ),
            )
            ..transactionsResult = [
              txSummaryFixture(
                txidHex:
                    'aa00000000000000000000000000000000000000000000000000000000000001',
                netAmountZat: 250000,
                hasMemo: false,
              ),
              txSummaryFixture(
                txidHex:
                    'bb00000000000000000000000000000000000000000000000000000000000002',
                netAmountZat: -1010000,
                feeZat: 10000,
                hasMemo: false,
              ),
            ];
      tester.view.physicalSize = const Size(800, 2000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(_harness(session: fake, theme: theme));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      final colors = WalletColors.of(tester.element(find.byType(WalletScreen)));
      final group = find.byKey(const ValueKey('wallet-activity-group'));
      expect(group, findsOneWidget);

      final cases = {
        l10n.walletAmount('+${formatZec(250000)}'): colors.accent,
        l10n.walletAmount('−${formatZec(1010000)}'): colors.text,
      };
      for (final c in cases.entries) {
        final amount = find.descendant(of: group, matching: find.text(c.key));
        expect(amount, findsOneWidget, reason: '${c.key} is inside the group');
        final style = tester.widget<Text>(amount).style!;
        expect(style.fontFamily, mono, reason: 'the registered mono face');
        expect(style.fontWeight, FontWeight.w500);
        expect(style.color, c.value);
      }
      expect(
        find.text(l10n.walletAmount('-${formatZec(1010000)}')),
        findsNothing,
        reason: 'never the ASCII hyphen',
      );
      final rows = find.byWidgetPredicate(
        (w) => w.runtimeType.toString() == '_ActivityRow',
      );
      expect(rows, findsNWidgets(2));
      for (final e in rows.evaluate()) {
        expect(
          find.descendant(of: group, matching: find.byWidget(e.widget)),
          findsOneWidget,
        );
        expect(e.size!.height, greaterThanOrEqualTo(64));
      }
    });
  });

  group(
    'C1 — the header survives the early states (S12 diff review MEDIUM)',
    () {
      testWidgets('a snapshot that cannot be read keeps the title and the '
          'overflow menu — the recovery routes', (tester) async {
        final session = FakeWalletSession(snapshotThrows: true);
        await tester.pumpWidget(_harness(session: session));
        await tester.pumpAndSettle();
        final l10n = _l10n(tester);
        expect(find.text(l10n.walletTitle), findsOneWidget);
        expect(find.byTooltip(l10n.walletMenuTooltip), findsOneWidget);
        // The menu opens and carries its recovery entries.
        await tester.tap(find.byTooltip(l10n.walletMenuTooltip));
        await tester.pumpAndSettle();
        // (Security is package-custody only; this harness wires no
        // provisioner.)
        expect(find.text(l10n.walletTransparentFundsMenuItem), findsOneWidget);
      });

      testWidgets('the cold load keeps the title and the overflow menu', (
        tester,
      ) async {
        final session = _GatedSnapshot();
        await tester.pumpWidget(_harness(session: session));
        // The snapshot read is held open: this is the cold-load frame.
        await tester.pump();
        final l10n = _l10n(tester);
        expect(find.byType(CircularProgressIndicator), findsWidgets);
        expect(find.text(l10n.walletTitle), findsOneWidget);
        expect(find.byTooltip(l10n.walletMenuTooltip), findsOneWidget);
        session.gate.complete();
        await tester.pumpAndSettle();
      });
    },
  );

  group('S12 code review — the coin offstage, the amount never clipped', () {
    testWidgets('a spin request while the coin is offstage (TickerMode off) '
        'does not spin it', (tester) async {
      Widget coinAt(int spins, {required bool onstage}) => MaterialApp(
        theme: lightTheme,
        home: TickerMode(
          enabled: onstage,
          child: Center(child: WalletCoin(spins: spins)),
        ),
      );
      await tester.pumpWidget(coinAt(0, onstage: false));
      await tester.pumpWidget(coinAt(1, onstage: false));
      expect(
        tester.state<WalletCoinState>(find.byType(WalletCoin)).spinning,
        isFalse,
        reason: 'offstage: the request is dropped, never deferred',
      );
      // The same request onstage does spin — the guard, not the harness, is
      // what held it.
      await tester.pumpWidget(coinAt(1, onstage: true));
      await tester.pumpWidget(coinAt(2, onstage: true));
      expect(
        tester.state<WalletCoinState>(find.byType(WalletCoin)).spinning,
        isTrue,
      );
      await tester.pumpAndSettle();
    });

    testWidgets('an activity amount is never ellipsized: past its width cap it '
        'scales down inside a FittedBox with every digit', (tester) async {
      final session =
          FakeWalletSession(
              current: const SyncStatus.upToDate(tip: 100),
              snapshotValue: walletStateFixture(
                syncStatus: const SyncStatus.upToDate(tip: 100),
                balance: balanceFixture(
                  totalZat: 2000000000000000,
                  spendableZat: 2000000000000000,
                ),
              ),
            )
            ..transactionsResult = [
              txSummaryFixture(
                txidHex: 'cc' * 32,
                // 19,999,999.12345678 ZEC — the widest figure a wallet can hold.
                netAmountZat: 1999999912345678,
                status: const TxStatus.confirmed(depth: 3),
              ),
            ];
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      final full = l10n.walletAmount('+${formatZec(1999999912345678)}');
      final amount = find.text(full);
      expect(
        amount,
        findsOneWidget,
        reason:
            'the Text carries the whole figure (what is visible is the '
            'FittedBox assertion below)',
      );
      expect(
        tester.widget<Text>(amount).overflow,
        isNot(TextOverflow.ellipsis),
      );
      expect(
        find.ancestor(of: amount, matching: find.byType(FittedBox)),
        findsOneWidget,
        reason: 'the figure scales down rather than clipping',
      );
    });

    testWidgets('an early state on a short landscape screen at 3× text does '
        'not overflow: the header scrolls (fold review MEDIUM)', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(700, 320);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);
      final session = FakeWalletSession(snapshotThrows: true)
        ..isWatchOnlyResult = true;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(session)],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
            builder: (context, child) => MediaQuery(
              data: MediaQuery.of(
                context,
              ).copyWith(textScaler: const TextScaler.linear(3)),
              child: child!,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text(_l10n(tester).walletTitle), findsOneWidget);
    });
  });

  group('WalletColors.lerp keeps an unset coin colour unset (fold review)', () {
    test('both unset: the result keeps deriving from deep/accent', () {
      final a = WalletColors.light.copyWith(
        deep: const Color(0xFF0F3D27),
        accent: const Color(0xFF177245),
      );
      // A host palette with no coin colours: rebuild the preset's fields
      // without them.
      WalletColors unset(WalletColors c) => WalletColors(
        bg: c.bg,
        bgCard: c.bgCard,
        bgHover: c.bgHover,
        border: c.border,
        accent: c.accent,
        accentSoft: c.accentSoft,
        accentText: c.accentText,
        text: c.text,
        textMuted: c.textMuted,
        textDim: c.textDim,
        green: c.green,
        orange: c.orange,
        red: c.red,
        purple: c.purple,
        cyan: c.cyan,
        bubbleMine: c.bubbleMine,
        bubbleTheirs: c.bubbleTheirs,
        toggleOff: c.toggleOff,
        inactiveElement: c.inactiveElement,
        onAccent: c.onAccent,
        deep: c.deep,
        onDeep: c.onDeep,
        deepMuted: c.deepMuted,
        warningOnDeep: c.warningOnDeep,
      );
      final mid = unset(a).lerp(unset(WalletColors.dark), 0.5);
      // Changing deep afterwards moves the coin: it was never baked in.
      final moved = mid.copyWith(deep: const Color(0xFF000000));
      expect(moved.coinEdge, isNot(mid.coinEdge));
      expect(
        moved.coinEdge,
        Color.lerp(const Color(0xFF000000), moved.accent, 0.5),
      );
    });

    test('a set colour interpolates', () {
      final mid = WalletColors.light.lerp(
        WalletColors.light.copyWith(coinFace: const Color(0xFF000000)),
        0.5,
      );
      expect(
        mid.coinFace,
        Color.lerp(WalletColors.light.coinFace, const Color(0xFF000000), 0.5),
      );
    });
  });

  group('S12 security review — the tip follow (walletTipFollowNext, pure)', () {
    const synced = WalletTipFollow(
      status: SyncStatus.upToDate(tip: 2000),
      healthyTip: 2000,
    );
    SyncStatus scan(int to, {bool rewound = false}) => SyncStatus.scanning(
      from: 1999,
      to: to,
      percent: 0,
      spendableReady: false,
      rewound: rewound,
    );

    test('an UpToDate starts a follow at its tip', () {
      final f = walletTipFollowNext(
        const WalletTipFollow(),
        const SyncStatus.upToDate(tip: 2000),
      );
      expect(f.healthyTip, 2000);
      expect(f.following, isFalse);
    });

    test('a scan from the synced tip up to kWalletTipFollowBlocks past it '
        'follows, and keeps following', () {
      final one = walletTipFollowNext(synced, scan(2001));
      expect(one.following, isTrue);
      expect(one.healthyTip, 2000);
      expect(walletTipFollowNext(one, scan(2002)).following, isTrue);
      expect(walletTipFollowNext(synced, scan(2000)).following, isTrue);
      expect(
        walletTipFollowNext(
          synced,
          scan(2000 + kWalletTipFollowBlocks),
        ).following,
        isTrue,
      );
    });

    final notFollowing = <String, WalletTipFollow Function()>{
      'one block past the bound': () =>
          walletTipFollowNext(synced, scan(2000 + kWalletTipFollowBlocks + 1)),
      'a rewound scan': () =>
          walletTipFollowNext(synced, scan(2001, rewound: true)),
      'a scan below the synced tip': () =>
          walletTipFollowNext(synced, scan(1999)),
      'no healthy UpToDate yet': () =>
          walletTipFollowNext(const WalletTipFollow(), scan(2001)),
      'after a stall': () => walletTipFollowNext(
        walletTipFollowNext(
          synced,
          const SyncStatus.stalled(reason: StallReason.endpointUnreachable),
        ),
        scan(2001),
      ),
      'after idle (a suspend)': () => walletTipFollowNext(
        walletTipFollowNext(synced, const SyncStatus.idle()),
        scan(2001),
      ),
      'after a qualified reached tip (its bar shows anyway)': () =>
          walletTipFollowNext(
            walletTipFollowNext(
              synced,
              const SyncStatus.upToDateLimited(tip: 2000),
            ),
            scan(2001),
          ),
    };
    for (final c in notFollowing.entries) {
      test('NOT following: ${c.key}', () {
        expect(c.value().following, isFalse);
      });
    }

    test('the bar judges a following scan as the UpToDate it follows', () {
      bool visible(
        SyncStatus s, {
        bool following = false,
        bool stale = false,
      }) => walletSyncBarVisible(
        status: s,
        stale: stale,
        driving: true,
        startFailed: false,
        syncDisabled: false,
        transportTone: TransportTone.neutral,
        hostDeclaredTransport: false,
        followingTip: following,
      );
      expect(visible(scan(2001), following: true), isFalse);
      expect(visible(scan(2001)), isTrue, reason: 'not following: shown');
      expect(
        visible(scan(2001), following: true, stale: true),
        isTrue,
        reason: 'a stale snapshot still shows the bar',
      );
    });
  });

  test('S12 crypto review — a re-delivered arrival spins the coin once', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final spins = container.read(walletCoinSpinsProvider.notifier);
    spins.bumpArrival(2001);
    spins.bumpArrival(2001);
    expect(container.read(walletCoinSpinsProvider), 1);
    spins.bumpArrival(2002);
    expect(container.read(walletCoinSpinsProvider), 2);
  });

  testWidgets('S12 security review — at a large text scale an activity '
      'amount takes its own line at the user size, never shrunk', (
    tester,
  ) async {
    final session =
        FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(
              syncStatus: const SyncStatus.upToDate(tip: 100),
            ),
          )
          ..transactionsResult = [
            txSummaryFixture(
              txidHex: 'dd' * 32,
              netAmountZat: 123456789012,
              status: const TxStatus.confirmed(depth: 3),
            ),
          ];
    await tester.pumpWidget(
      ProviderScope(
        overrides: [walletSessionProvider.overrideWithValue(session)],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const WalletScreen(),
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: const TextScaler.linear(2)),
            child: child!,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    final l10n = _l10n(tester);
    final amount = find.text(l10n.walletAmount('+${formatZec(123456789012)}'));
    await tester.scrollUntilVisible(amount, 200);
    final title = find.text(l10n.walletActivityReceived);
    expect(
      tester.getRect(amount).top,
      greaterThan(tester.getRect(title).bottom),
      reason: 'the amount sits on its own line under the title',
    );
    expect(
      tester.getRect(amount).width,
      closeTo(tester.getSize(amount).width, 0.01),
      reason: 'drawn at the user size: the FittedBox did not scale it',
    );
  });

  group('S12 fold review — the follow reads the DISPLAYED status', () {
    const stalled = SyncStatus.stalled(reason: StallReason.endpointUnreachable);
    const nextBlock = SyncStatus.scanning(
      from: 1999,
      to: 2001,
      percent: 0,
      spendableReady: false,
      rewound: false,
    );
    int bumps(WidgetTester tester) =>
        _container(tester).read(walletCoinSpinsProvider);
    // Longer than walletStallPostureDwell (2.5 s), the display's stall dwell.
    const pastDwell = Duration(milliseconds: 3500);

    Future<FakeWalletSession> synced(WidgetTester tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 2000),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 2000),
          lastSynced: const SyncStamp(height: 1000, at: 0),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      expect(_syncBar(), findsNothing, reason: 'precondition: synced');
      return fake;
    }

    testWidgets('a blip shorter than the stall dwell (the core retries in '
        'about 1 s) does not end the follow: no bar, no spin', (tester) async {
      final fake = await synced(tester);
      fake.push(stalled);
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      fake.push(nextBlock);
      await tester.pump();
      await tester.pump();
      expect(_syncBar(), findsNothing, reason: 'the blip was never shown');
      fake.push(const SyncStatus.upToDate(tip: 2001));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 0, reason: 'a blip then a block is not an entry');
      await tester.pumpAndSettle();
      expect(_syncBar(), findsNothing);
    });

    testWidgets('a stall that outlasts the dwell ends the follow: the bar '
        'shows, and the recovery spins once', (tester) async {
      final fake = await synced(tester);
      fake.push(stalled);
      await tester.pump();
      await tester.pump(pastDwell);
      await tester.pumpAndSettle();
      expect(_syncBar(), findsOneWidget, reason: 'the stall is shown');
      // Stalled → Scanning also waits out the dwell (an attempt claim), and
      // the scanning bar's indicator never settles: pump, don't settle.
      fake.push(nextBlock);
      await tester.pump();
      await tester.pump(pastDwell);
      await tester.pump(const Duration(milliseconds: 300));
      expect(_syncBar(), findsOneWidget, reason: 'recovering: still shown');
      fake.push(const SyncStatus.upToDate(tip: 2001));
      await tester.pump();
      await tester.pump();
      expect(bumps(tester), 1);
      await tester.pumpAndSettle();
      expect(_syncBar(), findsNothing);
    });
  });

  test('S12 fold review — the arrival watermark belongs to its session', () {
    final holder = NotifierProvider<_SessionHolder, WalletSession?>(
      _SessionHolder.new,
    );
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWith((ref) => ref.watch(holder)),
      ],
    );
    addTearDown(container.dispose);
    container.read(holder.notifier).set(FakeWalletSession());
    final spins = container.read(walletCoinSpinsProvider.notifier);
    spins.bumpArrival(2005);
    spins.bumpArrival(2004);
    expect(
      container.read(walletCoinSpinsProvider),
      1,
      reason: 'the same session: a lower height is old news',
    );
    container.read(holder.notifier).set(FakeWalletSession());
    spins.bumpArrival(2004);
    expect(
      container.read(walletCoinSpinsProvider),
      2,
      reason: 'a new session (rescan, server switch) starts its own mark',
    );
  });

  testWidgets('S12 rev.3 N2 — a reopen while catching up: 5 ZEC held stays '
      '5 ZEC (the core holds 4 as pending until witnesses exist), with '
      '"Spendable now" and "Not spendable yet", never "Arriving"', (
    tester,
  ) async {
    const catchingUp = SyncStatus.scanning(
      from: 900,
      to: 2000,
      percent: 0.4,
      spendableReady: true,
      rewound: false,
    );
    final fake = FakeWalletSession(
      current: catchingUp,
      snapshotValue: walletStateFixture(
        syncStatus: catchingUp,
        balance: balanceFixture(
          totalZat: 500000000,
          spendableZat: 100000000,
          pendingIncomingZat: 400000000,
        ),
        lastSynced: const SyncStamp(height: 900, at: 0),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    final l10n = _l10n(tester);
    Finder onCard(String text) =>
        find.descendant(of: _card, matching: find.text(text));
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('wallet-balance-total')))
          .data,
      l10n.walletAmount(formatZec(500000000)),
      reason: 'not synced: the headline is everything held, no dip',
    );
    expect(onCard(l10n.walletSpendableLabel), findsOneWidget);
    expect(
      onCard(l10n.walletNotSpendableYetLabel),
      findsOneWidget,
      reason: 'held money is "Not spendable yet": it is already the user\'s',
    );
    expect(
      onCard(l10n.walletArrivingLabel),
      findsNothing,
      reason: 'money the user already had is never "Arriving"',
    );
    expect(find.byKey(const ValueKey('wallet-balance-arriving')), findsNothing);
    expect(_syncBar(), findsOneWidget, reason: 'the bar explains it');
  });

  group('S12 rev.3 diff review — "synced" is the SNAPSHOT\'s provenance, and '
      'every other case shows the total', () {
    const held = 500000000; // 5 ZEC
    const pending = 400000000; // 4 of it the core holds as pending

    Future<void> mount(
      WidgetTester tester, {
      required SyncStatus status,
      int? tip,
      bool watchOnly = false,
      bool staleAfter = false,
    }) async {
      final fake = FakeWalletSession(
        current: status,
        snapshotValue: walletStateFixture(
          syncStatus: status,
          tip: tip,
          balance: balanceFixture(
            totalZat: held,
            spendableZat: held - pending,
            pendingIncomingZat: pending,
          ),
          lastSynced: const SyncStamp(height: 2000, at: 0),
        ),
      )..isWatchOnlyResult = watchOnly;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      if (staleAfter) {
        fake.snapshotThrows = true;
        _container(tester).invalidate(walletSnapshotReadProvider);
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 300));
      }
    }

    void expectTotalNoArriving(WidgetTester tester, String why) {
      final l10n = _l10n(tester);
      expect(
        tester
            .widget<Text>(find.byKey(const ValueKey('wallet-balance-total')))
            .data,
        l10n.walletAmount(formatZec(held)),
        reason: why,
      );
      expect(
        find.byKey(const ValueKey('wallet-balance-arriving')),
        findsNothing,
        reason: why,
      );
    }

    testWidgets('a snapshot read MID-PASS (a newer tip recorded, its blocks '
        'not scanned) under an UpToDate status shows the total', (
      tester,
    ) async {
      await mount(
        tester,
        status: const SyncStatus.upToDate(tip: 2000),
        tip: 2001,
      );
      expectTotalNoArriving(tester, 'the read is not at its pass\'s tip');
    });

    testWidgets('a stale snapshot (the re-read failed) shows the total', (
      tester,
    ) async {
      await mount(
        tester,
        status: const SyncStatus.upToDate(tip: 2000),
        staleAfter: true,
      );
      expectTotalNoArriving(tester, 'the figures may be from mid-pass');
    });

    testWidgets('a QUALIFIED reached tip (limited) shows the total', (
      tester,
    ) async {
      await mount(
        tester,
        status: const SyncStatus.upToDateLimited(tip: 2000),
        tip: 2000,
      );
      expectTotalNoArriving(tester, 'a caution state is not "synced"');
    });

    testWidgets('watch-only while catching up: pending is inside the total, '
        'so the pool line accounts for it (R4 in N2)', (tester) async {
      await mount(
        tester,
        status: const SyncStatus.scanning(
          from: 900,
          to: 2000,
          percent: 0.4,
          spendableReady: true,
          rewound: false,
        ),
        watchOnly: true,
      );
      expectTotalNoArriving(tester, 'not synced');
      expect(_poolLine, findsOneWidget);
      expect(_poolFigures(), [held]);
    });

    testWidgets('a routine one-block pass does not re-read the snapshot '
        'mid-pass (it would flip the headline to the total for seconds); '
        'its completion does', (tester) async {
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 2000),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 2000),
        ),
      );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final before = fake.snapshotCount;
      fake.push(
        const SyncStatus.scanning(
          from: 1999,
          to: 2001,
          percent: 1,
          spendableReady: true,
          rewound: false,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(
        fake.snapshotCount,
        before,
        reason: 'no mid-pass re-read while following the tip',
      );
      fake.push(const SyncStatus.upToDate(tip: 2001));
      await tester.pump();
      await tester.pump();
      expect(fake.snapshotCount, greaterThan(before));
      await tester.pumpAndSettle();
    });
  });

  testWidgets('S12 caption (founder S300) — "Balance · <time>" fits ONE line '
      'on a 347 dp phone (the OnePlus capture wrapped the old block-and-time '
      'caption)', (tester) async {
    tester.view.physicalSize = const Size(347, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    // The capture's own moment, and the caption's clock pinned to it: "today"
    // cannot turn into "yesterday" if the run crosses midnight.
    final now = DateTime(2026, 9, 24, 23, 38);
    balanceCaptionNow = () => now;
    addTearDown(() => balanceCaptionNow = DateTime.now);
    final fake = FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 3495000),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 3495000),
        lastSynced: SyncStamp(
          height: 3495000,
          at: now.millisecondsSinceEpoch ~/ 1000,
        ),
      ),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    final caption = find.text(
      l10n.walletBalanceHeaderAt(
        balanceCaptionTime(
          DateTime.fromMillisecondsSinceEpoch(
            (now.millisecondsSinceEpoch ~/ 1000) * 1000,
          ),
          l10n.localeName,
        ),
      ),
    );
    expect(caption, findsOneWidget, reason: 'today: the time alone');
    final style = tester.widget<Text>(caption).style!;
    expect(
      tester.getSize(caption).height,
      lessThan(style.fontSize! * 2),
      reason: 'one line',
    );
  });
}

/// A switchable session, for the watermark's session test.
class _SessionHolder extends Notifier<WalletSession?> {
  @override
  WalletSession? build() => null;

  void set(WalletSession s) => state = s;
}

/// A fixed-state rescan controller (the wallet_screen_test `_StubRescan`
/// shape) — renders a rescan phase without running one.
class _FixedRescan extends WalletRescanController {
  _FixedRescan(this._state);
  final WalletRescanState _state;

  @override
  WalletRescanState build() => _state;
}
