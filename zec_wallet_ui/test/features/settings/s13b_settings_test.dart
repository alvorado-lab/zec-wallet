import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/lifecycle/app_lifecycle_provider.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/settings/backup_screen.dart';
import 'package:zec_wallet_ui/features/settings/security_screen.dart';
import 'package:zec_wallet_ui/features/wallet/in_flight_swaps_section.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/bip39_wordlist.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_views.dart';
import 'package:zec_wallet_ui/features/wallet/reveal_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/wallet_cta.dart';
import 'package:zec_wallet_ui/shared/wallet_dialog.dart';
import 'package:zec_wallet_ui/shared/wallet_group.dart';
import 'package:zec_wallet_ui/shared/wallet_info_button.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// S13 Build B, the settings slice (plan §1.7): Security's groups and the
/// destructive row last, Backup's body once and its chip grid, the onboarding
/// pages' (i), gutter and CTAs, the dialog theme, and the two wallet sections
/// S12 deferred. Behaviour, never pixels.

WalletLocalizations _l10nOf(WidgetTester tester, Type screen) =>
    WalletLocalizations.of(tester.element(find.byType(screen)));

Finder _info(String body) =>
    find.byWidgetPredicate((w) => w is WalletInfoButton && w.body == body);

/// The rendered style of the paragraph showing exactly [text].
TextStyle? _styleOf(WidgetTester tester, String text) =>
    tester.renderObject<RenderParagraph>(find.text(text)).text.style;

const _hardwareKey = CustodyDisclosure(
  tier: 'apple_secure_enclave',
  eraseAssurance: EraseAssurance.hardwareKeyDeleted,
  degraded: false,
);

Widget _security({bool watchOnly = false}) => ProviderScope(
  overrides: [
    walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
    walletCustodyDisclosureProvider.overrideWith(
      (ref) => Future<CustodyDisclosure>.value(_hardwareKey),
    ),
    isWatchOnlyProvider.overrideWithValue(watchOnly),
  ],
  child: MaterialApp(
    theme: lightTheme,
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    home: const SecurityScreen(),
  ),
);

Future<void> _pumpBackup(WidgetTester tester, {double textScale = 1.0}) async {
  final container = ProviderContainer(
    overrides: [
      walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
      screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
      walletRevealAuthorizerProvider.overrideWithValue(FakeRevealAuthorizer()),
      appLifecycleProvider.overrideWith(TestLifecycleNotifier.new),
    ],
  );
  addTearDown(container.dispose);
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        theme: lightTheme,
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        home: MediaQuery(
          data: MediaQueryData(textScaler: TextScaler.linear(textScale)),
          child: const BackupScreen(),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _reveal(WidgetTester tester) async {
  final reveal = find.text(_l10nOf(tester, BackupScreen).walletBackupReveal);
  await tester.ensureVisible(reveal);
  await tester.pumpAndSettle();
  await tester.tap(reveal);
  await tester.pumpAndSettle();
}

Widget _onboarding() => ProviderScope(
  overrides: [
    walletProvisionerProvider.overrideWithValue(FakeWalletProvisioner()),
    onboardingStoreProvider.overrideWithValue(FakeOnboardingStore()),
    bip39WordlistProvider.overrideWith((ref) => testBip39Wordlist),
    screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
  ],
  child: MaterialApp(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: lightTheme,
    home: const WalletScreen(),
  ),
);

/// [button] sits inside a [WalletCta] and is at least a page CTA tall.
void _expectPageCta(WidgetTester tester, Finder button) {
  expect(button, findsOneWidget);
  expect(
    find.ancestor(of: button, matching: find.byType(WalletCta)),
    findsOneWidget,
  );
  expect(
    tester.getSize(button).height,
    greaterThanOrEqualTo(WalletCta.floorOf(WalletCtaSize.page)),
  );
}

void _setView(WidgetTester tester, Size size) {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
}

void main() {
  group('Security', () {
    testWidgets('the groups run custody, keys, delete — the destructive row '
        'LAST, alone, in the error role; no Card', (tester) async {
      await tester.pumpWidget(_security());
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, SecurityScreen);

      final custody = tester.getRect(find.byKey(securityCustodyGroupKey));
      final keys = tester.getRect(find.byKey(securityKeysGroupKey));
      final delete = tester.getRect(find.byKey(securityDeleteGroupKey));
      expect(keys.top, greaterThan(custody.bottom));
      expect(delete.top, greaterThan(keys.bottom));

      // Last: the list's final child is the destructive group.
      final list = tester.widget<ListView>(find.byType(ListView));
      final children =
          (list.childrenDelegate as SliverChildListDelegate).children;
      expect(children.last.key, securityDeleteGroupKey);
      // Alone: its group holds exactly one row, the delete row.
      expect(
        find.descendant(
          of: find.byKey(securityDeleteGroupKey),
          matching: find.byType(ListTile),
        ),
        findsOneWidget,
      );

      expect(
        _styleOf(tester, l10n.securityDeleteWalletButton)?.color,
        lightTheme.colorScheme.error,
      );
      // The custody card became a group.
      expect(find.byType(Card), findsNothing);
      expect(find.byType(WalletGroup), findsNWidgets(3));
    });

    testWidgets('the delete explanation is behind the row\'s (i), and the '
        'row still opens the DESTRUCTIVE confirm', (tester) async {
      await tester.pumpWidget(_security());
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, SecurityScreen);

      expect(find.text(l10n.securityDeleteWalletSubtitle), findsNothing);
      expect(
        find.descendant(
          of: find.byKey(securityDeleteGroupKey),
          matching: _info(l10n.securityDeleteWalletSubtitle),
        ),
        findsOneWidget,
      );

      await tester.ensureVisible(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();
      final confirm = find.descendant(
        of: find.byType(AlertDialog),
        matching: find.text(l10n.securityDeleteDialogConfirm),
      );
      // Destructive kind: a red TEXT button, never a filled one (S11 C3).
      expect(
        find.ancestor(of: confirm, matching: find.byType(TextButton)),
        findsOneWidget,
      );
      expect(
        find.ancestor(of: confirm, matching: find.byType(FilledButton)),
        findsNothing,
      );
    });

    testWidgets('the custody erase line and the watch-only paragraph open '
        'from their (i), never on screen', (tester) async {
      await tester.pumpWidget(_security(watchOnly: true));
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, SecurityScreen);

      expect(find.text(l10n.walletWatchOnlyAboutBody), findsNothing);
      expect(find.text(l10n.securityCustodyHardwareKey), findsNothing);

      await tester.tap(_info(l10n.walletWatchOnlyAboutBody));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletWatchOnlyAboutBody), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('wallet-info-close')));
      await tester.pumpAndSettle();

      await tester.tap(_info(l10n.securityCustodyHardwareKey));
      await tester.pumpAndSettle();
      expect(find.text(l10n.securityCustodyHardwareKey), findsOneWidget);
    });
  });

  group('Backup', () {
    testWidgets('the body shows ONCE: on the sealed screen, and the revealed '
        'words take its place', (tester) async {
      await _pumpBackup(tester);
      final l10n = _l10nOf(tester, BackupScreen);
      expect(find.text(l10n.walletBackupBody), findsOneWidget);

      await _reveal(tester);
      expect(find.text('art'), findsOneWidget);
      expect(find.text(l10n.walletBackupBody), findsNothing);
      _expectPageCta(
        tester,
        find.widgetWithText(FilledButton, l10n.walletBackupDone),
      );
    });

    testWidgets('the reveal is a page CTA and the page has the 16 gutter', (
      tester,
    ) async {
      _setView(tester, const Size(360, 800));
      await _pumpBackup(tester);
      final l10n = _l10nOf(tester, BackupScreen);
      final reveal = find.ancestor(
        of: find.text(l10n.walletBackupReveal),
        matching: find.byWidgetPredicate((w) => w is FilledButton),
      );
      _expectPageCta(tester, reveal);
      expect(tester.getRect(reveal).left, 16);
      expect(tester.getRect(reveal).right, 360 - 16);
    });

    testWidgets('the words are a chip GRID read row by row: three columns on '
        'a phone, one at 3x on 320 dp, no overflow', (tester) async {
      _setView(tester, const Size(360, 1000));
      await _pumpBackup(tester);
      await _reveal(tester);
      Rect chip(int n) =>
          tester.getRect(find.byKey(ValueKey('recovery-word-chip-$n')));
      // Row 1 is words 1-3, equal width; word 4 starts row 2 under word 1.
      expect(chip(2).top, chip(1).top);
      expect(chip(3).top, chip(1).top);
      expect(chip(2).left, greaterThan(chip(1).right));
      expect(chip(1).width, closeTo(chip(3).width, 0.01));
      expect(chip(4).top, greaterThan(chip(1).bottom));
      expect(chip(4).left, chip(1).left);
    });

    testWidgets('at 3x on 320 dp the grid drops to one column, no overflow', (
      tester,
    ) async {
      _setView(tester, const Size(320, 1400));
      await _pumpBackup(tester, textScale: 3.0);
      await _reveal(tester);
      expect(tester.takeException(), isNull);
      Rect chip(int n) =>
          tester.getRect(find.byKey(ValueKey('recovery-word-chip-$n')));
      expect(chip(2).top, greaterThan(chip(1).bottom));
      expect(chip(2).left, chip(1).left);
    });
  });

  group('Dialogs', () {
    testWidgets('the dialog title resolves 600 at 20 and the body 16, from the '
        'theme', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () => showWalletConfirm(
                  context,
                  title: 'dialog-title',
                  body: 'dialog-body',
                  cancelLabel: 'no',
                  confirmLabel: 'yes',
                  kind: WalletConfirmKind.forward,
                ),
                child: const Text('open'),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      final title = _styleOf(tester, 'dialog-title');
      expect(title?.fontWeight, FontWeight.w600);
      expect(title?.fontSize, 20);
      expect(_styleOf(tester, 'dialog-body')?.fontSize, 16);
    });
  });

  group('Onboarding', () {
    testWidgets('Welcome: the three page CTAs, the body behind the title\'s '
        '(i)', (tester) async {
      await tester.pumpWidget(_onboarding());
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, WalletScreen);

      for (final label in [
        l10n.walletCreateButton,
        l10n.walletRestoreButton,
        l10n.walletWatchOnlyButton,
      ]) {
        _expectPageCta(
          tester,
          find.ancestor(
            of: find.text(label),
            matching: find.byWidgetPredicate(
              (w) => w is FilledButton || w is OutlinedButton,
            ),
          ),
        );
      }
      expect(find.text(l10n.walletOnboardingWelcomeTitle), findsOneWidget);
      expect(find.text(l10n.walletOnboardingWelcomeBody), findsNothing);
      expect(_info(l10n.walletOnboardingWelcomeBody), findsOneWidget);
    });

    testWidgets('Restore keeps its money-safety body ON screen, Watch puts '
        'its body behind the (i); the submit a page CTA, the 16 gutter', (
      tester,
    ) async {
      _setView(tester, const Size(400, 1000));
      await tester.pumpWidget(_onboarding());
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, WalletScreen);

      await tester.tap(find.text(l10n.walletRestoreButton));
      await tester.pumpAndSettle();
      // The 25th-word warning (a passphrase wallet restores EMPTY, not an
      // error) is read before typing a phrase, never hidden behind an (i).
      expect(find.text(l10n.walletRestoreBody), findsOneWidget);
      expect(
        tester.getRect(find.text(l10n.walletRestoreBody)).height,
        greaterThan(0),
      );
      expect(_info(l10n.walletRestoreBody), findsNothing);
      expect(tester.getRect(find.text(l10n.walletRestoreTitle)).left, 16);
      _expectPageCta(
        tester,
        find.widgetWithText(FilledButton, l10n.walletRestoreSubmit),
      );

      // Scroll to Back as a user would: since an earlier revision the scan-range buttons
      // wrap instead of ellipsizing, which puts Back just below this fold.
      await tester.ensureVisible(find.text(l10n.walletRestoreBack));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRestoreBack));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletWatchOnlyButton));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletWatchOnlyBody), findsNothing);
      expect(_info(l10n.walletWatchOnlyBody), findsOneWidget);
      _expectPageCta(tester, find.byKey(const ValueKey('watch-only-submit')));
    });

    testWidgets('Failed: the forward action is a page CTA', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            theme: lightTheme,
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            home: const Scaffold(
              body: WalletOnboardingFailedView(
                kind: OnboardingFailureKind.network,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, WalletOnboardingFailedView);
      _expectPageCta(
        tester,
        find.ancestor(
          of: find.text(l10n.walletOnboardingRetry),
          matching: find.byWidgetPredicate((w) => w is FilledButton),
        ),
      );
    });
  });

  group('The sections S12 deferred', () {
    testWidgets('in-flight swaps: the Activity header role and every row in '
        'ONE group', (tester) async {
      final fake = FakeWalletSession()
        ..inFlightSwapsResult = [
          swapRecordFixture(id: 'a'),
          swapRecordFixture(id: 'b', direction: SwapRecordDirection.intoZec),
        ];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
          child: MaterialApp(
            theme: lightTheme,
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            home: Scaffold(
              body: ListView(children: const [InFlightSwapsSection()]),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, InFlightSwapsSection);
      final group = find.byKey(const ValueKey('wallet-swaps-group'));
      for (final line in [
        l10n.walletSwapInFlightRowOutOfZec,
        l10n.walletSwapInFlightRowIntoZec,
      ]) {
        expect(
          find.descendant(of: group, matching: find.text(line)),
          findsOneWidget,
        );
      }
      // One hairline between the two rows.
      expect(
        find.descendant(of: group, matching: find.byType(Divider)),
        findsOneWidget,
      );
      expect(
        _styleOf(tester, l10n.walletSwapsInFlightTitle(2))?.fontSize,
        lightTheme.textTheme.titleLarge?.fontSize,
      );
    });

    testWidgets('parked sends: the Activity header role and every row in ONE '
        'group', (tester) async {
      _setView(tester, const Size(800, 1600));
      final fake =
          FakeWalletSession(
              current: const SyncStatus.upToDate(tip: 100),
              snapshotValue: walletStateFixture(
                syncStatus: const SyncStatus.upToDate(tip: 100),
                balance: balanceFixture(
                  spendableZat: 1000000,
                  totalZat: 1000000,
                ),
              ),
            )
            ..parkedSendsResult = [
              parkedSendFixture(id: 1, amountZat: 70000),
              parkedSendFixture(id: 2, amountZat: 90000),
            ];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
          child: MaterialApp(
            theme: lightTheme,
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nOf(tester, WalletScreen);
      final group = find.byKey(const ValueKey('wallet-parked-group'));
      await tester.ensureVisible(group);
      await tester.pumpAndSettle();
      expect(
        find.descendant(
          of: group,
          matching: find.text(l10n.walletParkedCancel),
        ),
        findsNWidgets(2),
      );
      expect(
        find.descendant(of: group, matching: find.byType(Divider)),
        findsOneWidget,
      );
      expect(
        _styleOf(tester, l10n.walletParkedTitle)?.fontSize,
        lightTheme.textTheme.titleLarge?.fontSize,
      );
    });
  });
}
