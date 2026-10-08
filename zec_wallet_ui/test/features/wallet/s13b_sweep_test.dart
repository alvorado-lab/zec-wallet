import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart'
    show RescanTarget;
import 'package:zec_wallet_ui/features/wallet/rescan_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_token_picker.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_tokens_provider.dart';
import 'package:zec_wallet_ui/features/wallet/swap_deep_scan.dart';
import 'package:zec_wallet_ui/features/wallet/sync_server_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/transparent_funds_providers.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/transparent_funds_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/tx_detail_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/wallet_info_button.dart';
import 'package:zec_wallet_ui/shared/wallet_loading.dart';
import 'package:zec_wallet_ui/shared/wallet_sheet.dart';
import 'package:zec_wallet_ui/testing.dart';

/// S13 Build B, the sweep (plan §1.7): the sheets' one title row and close,
/// the spinners, the haptics, the glyph split, and the Public / Arriving terms.
void main() {
  // ── Harness ──

  Widget app(Widget home, {List<Override> overrides = const []}) =>
      ProviderScope(
        overrides: overrides,
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: home,
        ),
      );

  /// A button that opens a sheet through its real `show…` entry.
  Widget launcher(
    Future<void> Function(BuildContext) open, {
    List<Override> overrides = const [],
  }) => app(
    Scaffold(
      body: Builder(
        builder: (context) => Center(
          child: TextButton(
            onPressed: () => unawaited(open(context)),
            child: const Text('open'),
          ),
        ),
      ),
    ),
    overrides: overrides,
  );

  /// The one title row: its title in `titleLarge`, and the keyed 44 close.
  void expectHeader(WidgetTester tester, Key closeKey, {String? title}) {
    final header = find.byType(WalletSheetHeader);
    expect(header, findsOneWidget);
    final titles = find.descendant(of: header, matching: find.byType(Text));
    final text = tester.widget<Text>(
      title == null
          ? titles.first
          : find.descendant(of: header, matching: find.text(title)),
    );
    final large = Theme.of(tester.element(header)).textTheme.titleLarge!;
    expect(text.style?.fontSize, large.fontSize, reason: 'titleLarge size');
    expect(text.style?.fontWeight, large.fontWeight, reason: 'titleLarge');
    final close = find.byKey(closeKey);
    expect(close, findsOneWidget, reason: 'the keyed close');
    expect(tester.getSize(close), const Size.square(44));
  }

  group('the sheets: a titleLarge title and a keyed 44 close', () {
    testWidgets('the close is read aloud as the platform\'s "Close" and '
        'closes the sheet', (tester) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(
        launcher(
          (context) => showWalletSheet<void>(
            context,
            builder: (_) => const WalletSheetHeader(
              title: 'A sheet',
              closeKey: Key('probe-close'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expectHeader(tester, const Key('probe-close'), title: 'A sheet');
      final label = MaterialLocalizations.of(
        tester.element(find.byType(WalletSheetHeader)),
      ).closeButtonLabel;
      expect(find.bySemanticsLabel(label), findsOneWidget);
      await tester.tap(find.byKey(const Key('probe-close')));
      await tester.pumpAndSettle();
      expect(find.byType(WalletSheetHeader), findsNothing);
      handle.dispose();
    });

    testWidgets('transaction detail', (tester) async {
      await tester.pumpWidget(
        app(Scaffold(body: TxDetailSheet(tx: txSummaryFixture()))),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(TxDetailSheet)),
      );
      expectHeader(
        tester,
        const Key('tx-detail-close'),
        title: l10n.walletActivityReceived,
      );
    });

    testWidgets('sync status', (tester) async {
      await tester.pumpWidget(
        app(
          const Scaffold(body: SyncStatusSheet()),
          overrides: [
            walletSessionProvider.overrideWithValue(
              FakeWalletSession(current: const SyncStatus.upToDate(tip: 9)),
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      expectHeader(tester, const Key('sync-status-close'));
    });

    testWidgets('sync server', (tester) async {
      await tester.pumpWidget(
        app(
          const Scaffold(body: SyncServerSheet()),
          overrides: [
            walletSessionProvider.overrideWithValue(FakeWalletSession()),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SyncServerSheet)),
      );
      expectHeader(
        tester,
        const Key('sync-server-close'),
        title: l10n.walletSyncServerSheetTitle,
      );
    });

    testWidgets('swap token picker', (tester) async {
      await tester.pumpWidget(
        launcher(
          (context) => showSwapTokenPicker(context, title: 'Pick a token'),
          overrides: [
            swapTokensProvider.overrideWith(
              (ref) => Future.value(swapTokenListFixture()),
            ),
          ],
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expectHeader(
        tester,
        const Key('swap-token-picker-close'),
        title: 'Pick a token',
      );
    });

    testWidgets('deep scan', (tester) async {
      await tester.pumpWidget(
        launcher(
          showSwapDeepScanSheet,
          overrides: [
            walletSessionProvider.overrideWithValue(FakeWalletSession()),
          ],
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletSheetHeader)),
      );
      expectHeader(
        tester,
        const Key('deep-scan-close'),
        title: l10n.walletDeepScanTitle,
      );
    });

    testWidgets('shield (its review)', (tester) async {
      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture();
      await tester.pumpWidget(
        app(
          const Scaffold(body: ShieldSheet()),
          overrides: [walletSessionProvider.overrideWithValue(session)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ShieldSheet)),
      );
      expectHeader(
        tester,
        const Key('shield-sheet-close'),
        title: l10n.walletShieldSheetTitle,
      );
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

    testWidgets('move to public (its form)', (tester) async {
      await tester.pumpWidget(
        app(
          const Scaffold(body: MoveToTransparentSheet()),
          overrides: [walletSessionProvider.overrideWithValue(moveSession())],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(MoveToTransparentSheet)),
      );
      expectHeader(
        tester,
        const Key('move-sheet-close'),
        title: l10n.walletMoveSheetTitle,
      );
    });

    testWidgets('public funds', (tester) async {
      await tester.pumpWidget(
        app(
          const Scaffold(body: TransparentFundsSheet()),
          overrides: [
            walletSessionProvider.overrideWithValue(FakeWalletSession()),
            walletSettingsStoreProvider.overrideWithValue(
              FakeWalletSettingsStore(),
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(TransparentFundsSheet)),
      );
      expectHeader(
        tester,
        const Key('transparent-funds-close'),
        title: l10n.walletTransparentFundsTitle,
      );
    });

    testWidgets('the (i) explanation', (tester) async {
      await tester.pumpWidget(
        app(
          const Scaffold(
            body: Center(
              child: WalletInfoButton(label: 'Queue', body: 'Why.'),
            ),
          ),
        ),
      );
      await tester.tap(find.byType(WalletInfoButton));
      await tester.pumpAndSettle();
      expectHeader(tester, const ValueKey('wallet-info-close'), title: 'Queue');
    });

    testWidgets('rescan: a close while idle, NONE while it runs', (
      tester,
    ) async {
      final gate = Completer<void>();
      final session = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 8100000),
        snapshotValue: walletStateFixture(
          syncStatus: const SyncStatus.upToDate(tip: 8100000),
          tip: 8100000,
        ),
      )..birthdayHeightResult = 8000000;
      await tester.pumpWidget(
        launcher(
          showWalletRescanSheet,
          overrides: [
            walletRescanControllerProvider.overrideWith(
              () => _GatedRescan(gate),
            ),
            walletSessionProvider.overrideWithValue(session),
            walletProvisionerProvider.overrideWithValue(
              FakeWalletProvisioner(exists: true),
            ),
          ],
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletSheetHeader)),
      );
      expectHeader(
        tester,
        const Key('rescan-sheet-close'),
        title: l10n.walletRescanTitle,
      );

      final start = find.text(l10n.walletRescanConfirm);
      await tester.ensureVisible(start);
      await tester.pumpAndSettle();
      await tester.tap(start);
      await tester.pump();
      expect(find.text(l10n.walletRescanRunning), findsOneWidget);
      expect(
        find.byKey(const Key('rescan-sheet-close')),
        findsNothing,
        reason: 'the locked rescan offers no way out',
      );
      expect(find.text(l10n.walletRescanTitle), findsOneWidget);

      gate.complete();
      await tester.pumpAndSettle();
    });
  });

  // ── Spinners ──

  group('spinners', () {
    for (final platform in [TargetPlatform.android, TargetPlatform.iOS]) {
      testWidgets('a lone spinner is read aloud as "Loading" on '
          '${platform.name}', (tester) async {
        final handle = tester.ensureSemantics();
        await tester.pumpWidget(
          MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme.copyWith(platform: platform),
            home: const Scaffold(body: Center(child: WalletLoadingIndicator())),
          ),
        );
        expect(find.bySemanticsLabel('Loading'), findsOneWidget);
        handle.dispose();
      });
    }

    testWidgets('the token picker\'s loading spinner is named', (tester) async {
      final handle = tester.ensureSemantics();
      final never = Completer<SwapTokenList>();
      await tester.pumpWidget(
        launcher(
          (context) => showSwapTokenPicker(context, title: 'Pick'),
          overrides: [swapTokensProvider.overrideWith((ref) => never.future)],
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      expect(
        find.byKey(const Key('swap-token-picker-loading')),
        findsOneWidget,
      );
      expect(find.bySemanticsLabel('Loading'), findsOneWidget);
      handle.dispose();
    });

    test('every spinner in lib/ is adaptive, and the wallet tab\'s two lone '
        'ones carry the Loading name', () {
      final offenders = <String>[];
      for (final f in _dartFiles('lib')) {
        final src = f.readAsStringSync();
        if (src.contains('CircularProgressIndicator(')) offenders.add(f.path);
      }
      expect(
        offenders,
        isEmpty,
        reason: 'use CircularProgressIndicator.adaptive',
      );
      final wallet = File(
        'lib/features/wallet/wallet_screen.dart',
      ).readAsStringSync();
      expect(
        "WalletLoadingIndicator(key: Key('wallet-loading'))".allMatchesIn(
          wallet,
        ),
        2,
      );
      expect(
        wallet,
        contains("WalletLoadingIndicator(key: Key('wallet-activity-loading'))"),
      );
    });
  });

  // ── Haptics ──

  group('haptics', () {
    late List<MethodCall> calls;
    setUp(() => calls = <MethodCall>[]);

    void mockPlatform(WidgetTester tester) {
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
    }

    int lightImpacts() => calls
        .where(
          (c) =>
              c.method == 'HapticFeedback.vibrate' &&
              c.arguments == 'HapticFeedbackType.lightImpact',
        )
        .length;

    testWidgets('copying a transaction id is felt', (tester) async {
      mockPlatform(tester);
      await tester.pumpWidget(
        app(Scaffold(body: TxDetailSheet(tx: txSummaryFixture()))),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(TxDetailSheet)),
      );
      expect(lightImpacts(), 0);
      await tester.ensureVisible(find.text(l10n.walletTxDetailCopyTxid));
      await tester.tap(find.text(l10n.walletTxDetailCopyTxid));
      await tester.pumpAndSettle();
      expect(lightImpacts(), 1);
    });

    test('every copy to the clipboard in lib/ is followed by the light '
        'impact', () {
      var sites = 0;
      for (final f in _dartFiles('lib')) {
        final lines = f.readAsLinesSync();
        for (var i = 0; i < lines.length; i++) {
          if (!lines[i].contains('Clipboard.setData(')) continue;
          sites++;
          expect(
            lines[i + 1].trim(),
            'unawaited(HapticFeedback.lightImpact());',
            reason: '${f.path}:${i + 1}',
          );
        }
      }
      expect(sites, greaterThanOrEqualTo(6));
    });

    testWidgets('Send now is felt', (tester) async {
      mockPlatform(tester);
      final session =
          FakeWalletSession(
              current: const SyncStatus.upToDate(tip: 1),
              snapshotValue: walletStateFixture(
                syncStatus: const SyncStatus.upToDate(tip: 1),
                balance: balanceFixture(
                  spendableZat: 500000000,
                  totalZat: 500000000,
                ),
              ),
            )
            ..proposeResult = sendProposalFixture()
            ..sendResults = const [TxSubmitResult.success(txidHex: 'aabb')];
      await tester.pumpWidget(
        app(
          const SendScreen(),
          overrides: [walletSessionProvider.overrideWithValue(session)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SendScreen)),
      );
      await tester.enterText(find.byType(TextField).at(0), 'u1recipient');
      await tester.enterText(find.byType(TextField).at(1), '1');
      await tester.pump();
      await tester.tap(find.text(l10n.walletSendReviewButton));
      await tester.pumpAndSettle();
      expect(lightImpacts(), 0, reason: 'Review is not the confirm');
      await tester.tap(find.text(l10n.walletSendConfirmButton));
      await tester.pumpAndSettle();
      expect(lightImpacts(), 1);
    });

    testWidgets('Shield\'s confirm is felt', (tester) async {
      mockPlatform(tester);
      final session = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture()
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
      await tester.pumpWidget(
        app(
          const Scaffold(body: ShieldSheet()),
          overrides: [walletSessionProvider.overrideWithValue(session)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(ShieldSheet)),
      );
      await tester.tap(find.text(l10n.walletShieldConfirmButton));
      await tester.pumpAndSettle();
      expect(lightImpacts(), 1);
    });

    testWidgets('Move\'s confirm is felt', (tester) async {
      mockPlatform(tester);
      final session =
          FakeWalletSession(current: const SyncStatus.upToDate(tip: 1))
            ..currentTransparentAddressResult = 't1myownTaddr'
            ..setSnapshot(
              walletStateFixture(
                syncStatus: const SyncStatus.upToDate(tip: 1),
                balance: balanceFixture(spendableZat: 100000000),
              ),
            )
            ..proposeResult = sendProposalFixture(hasTransparentRecipient: true)
            ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
      await tester.pumpWidget(
        app(
          const Scaffold(body: MoveToTransparentSheet()),
          overrides: [walletSessionProvider.overrideWithValue(session)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(MoveToTransparentSheet)),
      );
      await tester.enterText(find.byType(TextField), '0.002');
      await tester.tap(find.text(l10n.walletMoveReviewButton));
      await tester.pumpAndSettle();
      expect(lightImpacts(), 0);
      await tester.ensureVisible(find.text(l10n.walletMoveConfirmButton));
      await tester.tap(find.text(l10n.walletMoveConfirmButton));
      await tester.pumpAndSettle();
      expect(lightImpacts(), 1);
    });

    testWidgets('an arrival is felt once — a re-delivery is not', (
      tester,
    ) async {
      mockPlatform(tester);
      final fake = FakeWalletSession(
        current: const SyncStatus.upToDate(tip: 100),
        snapshotValue: walletStateFixture(),
      )..transactionsResult = [];
      await tester.pumpWidget(
        app(
          const WalletScreen(),
          overrides: [walletSessionProvider.overrideWithValue(fake)],
        ),
      );
      await tester.pumpAndSettle();
      Future<void> arrive() async {
        fake.pushIncoming(
          const IncomingFundsEvent(
            kind: IncomingFundsEventKind.live,
            newTxCount: 1,
            totalTxDetected: 1,
            spanFromHeight: 150,
            spanToHeight: 150,
            cursor: 'w1:160',
          ),
        );
        await tester.pump();
        await tester.pump();
      }

      expect(lightImpacts(), 0);
      await arrive();
      expect(lightImpacts(), 1);
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();
      await arrive();
      expect(lightImpacts(), 1, reason: 'the same money is not felt twice');
    });
  });

  // ── Glyphs ──

  test('WalletGlyph.reveal only where a secret is revealed', () {
    const allowed = {
      'lib/features/settings/backup_screen.dart': 1,
      'lib/features/settings/security_screen.dart': 1,
      'lib/features/settings/export_viewing_key_screen.dart': 2,
      'lib/features/wallet/onboarding/onboarding_views.dart': 1,
    };
    final found = <String, int>{};
    for (final f in _dartFiles('lib')) {
      final n = 'WalletGlyph.reveal'.allMatchesIn(f.readAsStringSync());
      if (n > 0) found[f.path] = n;
    }
    expect(found, allowed);
  });

  // ── Terms ──

  group('terms', () {
    test('an incoming unmined payment reads "Arriving"; the user\'s own '
        'send in flight stays "Pending"', () async {
      final l10n = await WalletLocalizations.delegate.load(const Locale('en'));
      final incoming = txSummaryFixture(
        status: const TxStatus.pending(),
        minedHeight: null,
        netAmountZat: 250000,
      );
      final outgoing = txSummaryFixture(
        status: const TxStatus.pending(),
        minedHeight: null,
        netAmountZat: -250000,
      );
      expect(txRowStatusLabel(l10n, incoming), 'Arriving');
      expect(txRowStatusLabel(l10n, outgoing), 'Pending');
    });

    test('the forward-compat Unknown status follows the same rule: incoming '
        'reads "Arriving", the user\'s own send stays "Pending"', () async {
      final l10n = await WalletLocalizations.delegate.load(const Locale('en'));
      final incoming = txSummaryFixture(
        status: const TxStatus.unknown(),
        minedHeight: null,
        netAmountZat: 250000,
      );
      final outgoing = txSummaryFixture(
        status: const TxStatus.unknown(),
        minedHeight: null,
        netAmountZat: -250000,
      );
      expect(txRowStatusLabel(l10n, incoming), l10n.walletArrivingLabel);
      expect(txRowStatusLabel(l10n, outgoing), l10n.walletActivityPending);
    });

    test('held money (the not-synced breakdown row) is "Not spendable yet", '
        'a different word from "Arriving" in every locale', () async {
      final en = await WalletLocalizations.delegate.load(const Locale('en'));
      expect(en.walletNotSpendableYetLabel, 'Not spendable yet');
      for (final locale in WalletLocalizations.supportedLocales) {
        final l10n = await WalletLocalizations.delegate.load(locale);
        expect(
          l10n.walletNotSpendableYetLabel,
          isNot(l10n.walletArrivingLabel),
          reason: '$locale: money the user has never reads as arriving',
        );
      }
    });

    test('no English string says "transparent"; the removed keys are gone '
        'from every locale', () {
      final en =
          jsonDecode(File('lib/l10n/wallet_en.arb').readAsStringSync())
              as Map<String, dynamic>;
      final saysTransparent = [
        for (final e in en.entries)
          if (!e.key.startsWith('@') &&
              (e.value as String).toLowerCase().contains('transparent'))
            e.key,
      ];
      expect(saysTransparent, isEmpty);
      for (final f in Directory('lib/l10n').listSync().whereType<File>()) {
        if (!f.path.endsWith('.arb')) continue;
        for (final removed in [
          'walletSwapRefundInfoClose',
          'securityDeleteSectionTitle',
          // One source for the maintainer's term: walletArrivingLabel.
          'walletPendingLabel',
        ]) {
          expect(
            f.readAsStringSync(),
            isNot(contains(removed)),
            reason: f.path,
          );
        }
      }
    });
  });
}

/// A rescan that waits on [gate], so the running window can be observed.
class _GatedRescan extends WalletRescanController {
  _GatedRescan(this.gate);
  final Completer<void> gate;

  @override
  WalletRescanState build() => const WalletRescanIdle();

  @override
  Future<void> rescan(RescanTarget target) => gate.future;
}

Iterable<File> _dartFiles(String root) => Directory(root)
    .listSync(recursive: true)
    .whereType<File>()
    .where((f) => f.path.endsWith('.dart'));

extension on String {
  /// How many times this literal occurs in [haystack].
  int allMatchesIn(String haystack) =>
      RegExp(RegExp.escape(this)).allMatches(haystack).length;
}
