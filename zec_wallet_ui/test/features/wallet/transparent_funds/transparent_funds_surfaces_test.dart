import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/auto_shield_controller.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/transparent_funds_providers.dart';
import 'package:zec_wallet_ui/features/wallet/transparent_funds/transparent_funds_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// A scripted [AutoShieldController] stand-in: pins the SURFACES (badge/cue)
/// without running the real loop against the fake session.
class _StubAutoShield extends AutoShieldController {
  _StubAutoShield(this._status);
  final AutoShieldStatus _status;
  @override
  AutoShieldStatus build() => _status;
}

void main() {
  group('transparent-funds sheet (§3.2i-3 (b))', () {
    WalletLocalizations l10nAt(WidgetTester tester) => WalletLocalizations.of(
      tester.element(find.byType(TransparentFundsSheet)),
    );

    Widget harness(FakeWalletSettingsStore store) {
      return ProviderScope(
        // Riverpod 3 auto-RETRIES failing providers with backoff, so an
        // errored flag oscillates error↔loading forever under a scripted
        // read failure — retry off makes the error state TERMINAL and the
        // error-direction assertions deterministic. (In production the
        // auto-retry is a bonus recovery path on top of retry-on-open.)
        retry: (retryCount, error) => null,
        overrides: [
          walletSessionProvider.overrideWithValue(FakeWalletSession()),
          walletSettingsStoreProvider.overrideWithValue(store),
        ],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const Scaffold(body: TransparentFundsSheet()),
        ),
      );
    }

    testWidgets('default posture: gate OFF — intro + expert toggle only; no '
        'auto-shield switch, no promoted unshield', (tester) async {
      await tester.pumpWidget(harness(FakeWalletSettingsStore()));
      await tester.pumpAndSettle();
      final l10n = l10nAt(tester);

      expect(find.text(l10n.walletTransparentFundsIntro), findsOneWidget);
      expect(
        find.byKey(const ValueKey('wallet-expert-toggle')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('wallet-auto-shield-toggle')),
        findsNothing,
      );
      expect(find.byKey(const ValueKey('wallet-move-entry')), findsNothing);
    });

    testWidgets(
      'turning the gate ON persists and reveals the auto-shield switch + '
      'the unshield entry',
      (tester) async {
        final store = FakeWalletSettingsStore();
        await tester.pumpWidget(harness(store));
        await tester.pumpAndSettle();

        await tester.tap(find.byKey(const ValueKey('wallet-expert-toggle')));
        await tester.pumpAndSettle();

        expect(store.expertValue, isTrue, reason: 'persist-then-publish');
        expect(store.expertWrites, 1);
        expect(
          find.byKey(const ValueKey('wallet-auto-shield-toggle')),
          findsOneWidget,
        );
        expect(find.byKey(const ValueKey('wallet-move-entry')), findsOneWidget);
      },
    );

    testWidgets('#383 R2: an UNSUPPORTED host renders NO auto-shield switch '
        'and NO automation claim — even with the expert gate ON', (
      tester,
    ) async {
      await tester.pumpWidget(
        ProviderScope(
          retry: (retryCount, error) => null,
          overrides: [
            walletSessionProvider.overrideWithValue(FakeWalletSession()),
            walletSettingsStoreProvider.overrideWithValue(
              // The gate is ON — the state that WOULD show the switch.
              FakeWalletSettingsStore(expertValue: true),
            ),
            walletAutoShieldSupportedProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: TransparentFundsSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('wallet-auto-shield-toggle')),
        findsNothing,
        reason:
            'an unsupported capability is an honest absence, not a '
            'switch that reads ON but can never run',
      );
      expect(
        find.byKey(const ValueKey('wallet-transparent-funds-auto-claim')),
        findsNothing,
        reason: 'no automation claim either way',
      );
      expect(
        find.byKey(const ValueKey('wallet-expert-toggle')),
        findsOneWidget,
        reason: 'the expert gate itself is unrelated and stays',
      );
    });

    testWidgets('S205-b: the expert-toggle subtitle drops the "turning '
        'automatic shielding off" clause when the host declared auto-shield '
        'unsupported — the promised switch never appears there', (
      tester,
    ) async {
      // Unsupported host: the no-auto-shield description.
      await tester.pumpWidget(
        ProviderScope(
          retry: (retryCount, error) => null,
          overrides: [
            walletSessionProvider.overrideWithValue(FakeWalletSession()),
            walletSettingsStoreProvider.overrideWithValue(
              FakeWalletSettingsStore(),
            ),
            walletAutoShieldSupportedProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: TransparentFundsSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      var l10n = l10nAt(tester);
      expect(
        find.text(l10n.walletExpertToggleDescriptionNoAutoShield),
        findsOneWidget,
      );
      expect(
        find.text(l10n.walletExpertToggleDescription),
        findsNothing,
        reason:
            'advertising "turning automatic shielding off" would promise a '
            'control that never renders on this host',
      );

      // The supported default keeps the full description (the sibling pin —
      // the two keys must not drift).
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(harness(FakeWalletSettingsStore()));
      await tester.pumpAndSettle();
      l10n = l10nAt(tester);
      expect(find.text(l10n.walletExpertToggleDescription), findsOneWidget);
      expect(
        find.text(l10n.walletExpertToggleDescriptionNoAutoShield),
        findsNothing,
      );
    });

    testWidgets(
      'flipping auto-shield OFF persists (the expert hold-transparent '
      'choice)',
      (tester) async {
        final store = FakeWalletSettingsStore(expertValue: true);
        await tester.pumpWidget(harness(store));
        await tester.pumpAndSettle();

        await tester.tap(
          find.byKey(const ValueKey('wallet-auto-shield-toggle')),
        );
        await tester.pumpAndSettle();

        expect(store.autoShieldValue, isFalse);
        expect(store.autoShieldWrites, 1);
      },
    );

    testWidgets('a FAILED settings write keeps the switch at its persisted '
        'value and says the save failed — never a switch that lies', (
      tester,
    ) async {
      final store = FakeWalletSettingsStore()..failWrites = true;
      await tester.pumpWidget(harness(store));
      await tester.pumpAndSettle();
      final l10n = l10nAt(tester);

      await tester.tap(find.byKey(const ValueKey('wallet-expert-toggle')));
      await tester.pumpAndSettle();

      expect(store.expertValue, isNull, reason: 'nothing persisted');
      // INLINE (UX review M4, measured): a SnackBar renders in the
      // Scaffold UNDER the modal sheet and is fully occluded.
      expect(
        find.byKey(const ValueKey('wallet-settings-save-failed')),
        findsOneWidget,
      );
      expect(find.text(l10n.walletSettingsSaveFailed), findsOneWidget);
      expect(
        find.byKey(const ValueKey('wallet-auto-shield-toggle')),
        findsNothing,
        reason: 'the gate did not silently open',
      );
    });

    testWidgets('the save-failed line renders ABOVE the switches — visible '
        'without scrolling at large text scales (S155 wrap UX MAJOR-1)', (
      tester,
    ) async {
      final store = FakeWalletSettingsStore()..failWrites = true;
      await tester.pumpWidget(harness(store));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('wallet-expert-toggle')));
      await tester.pumpAndSettle();

      final lineY = tester
          .getTopLeft(find.byKey(const ValueKey('wallet-settings-save-failed')))
          .dy;
      final toggleY = tester
          .getTopLeft(find.byKey(const ValueKey('wallet-expert-toggle')))
          .dy;
      expect(
        lineY,
        lessThan(toggleY),
        reason:
            'appended at the end of the scroll column it sat '
            'off-viewport at 2.0× — feedback must precede the switches',
      );
    });

    testWidgets('a failed settings READ: no automation claim (silence over a '
        'possibly-false one), switches stay ENABLED (a write is the '
        'recovery), and REOPENING the sheet retries the read (S155 wrap: '
        'retry-on-open + read-error direction, previously unpinned)', (
      tester,
    ) async {
      final store = FakeWalletSettingsStore()..failReads = true;
      await tester.pumpWidget(harness(store));
      await tester.pumpAndSettle();
      final l10n = l10nAt(tester);

      expect(
        find.byKey(const ValueKey('wallet-transparent-funds-auto-claim')),
        findsNothing,
        reason: 'the persisted value is unknown — no claim either way',
      );
      // The switch is enabled on ERROR (not loading): a successful write
      // publishes truthfully and heals the state.
      final toggle = tester.widget<SwitchListTile>(
        find.byKey(const ValueKey('wallet-expert-toggle')),
      );
      expect(
        toggle.onChanged,
        isNotNull,
        reason: 'write-as-recovery must stay reachable',
      );

      // The disk heals; REOPENING the sheet must retry the errored reads.
      store
        ..failReads = false
        ..autoShieldValue = false;
      final readsBefore = store.reads;
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(harness(store));
      await tester.pumpAndSettle();
      expect(
        store.reads,
        greaterThan(readsBefore),
        reason: 'retry-on-open re-read the errored flags',
      );
      expect(
        find.text(l10n.walletTransparentFundsAutoOff),
        findsOneWidget,
        reason: 'the healed read surfaces the REAL persisted OFF',
      );
    });

    testWidgets('a DENIED session\'s sheet says automation is paused — never '
        '"shielded automatically" over a latched-off loop (S155 wrap UX '
        'MINOR-1)', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(FakeWalletSession()),
            walletSettingsStoreProvider.overrideWithValue(
              FakeWalletSettingsStore(),
            ),
            walletAutoShieldControllerProvider.overrideWith(
              () => _StubAutoShield(AutoShieldStatus.denied),
            ),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const Scaffold(body: TransparentFundsSheet()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = l10nAt(tester);

      expect(find.text(l10n.walletTransparentFundsAutoDenied), findsOneWidget);
    });

    testWidgets('the automation sentence is CONDITIONALLY factual (S154 M1): '
        'ON claims automatic shielding, OFF says funds stay public — even '
        'with the gate closed over a persisted OFF', (tester) async {
      // Default (ON): the automatic claim, with the minimum.
      await tester.pumpWidget(harness(FakeWalletSettingsStore()));
      await tester.pumpAndSettle();
      var l10n = l10nAt(tester);
      expect(
        find.text(l10n.walletTransparentFundsAutoOn(formatZec(100000))),
        findsOneWidget,
      );

      // Persisted OFF with the expert gate CLOSED: the switch is hidden, but
      // the sheet must still SAY automation is off (never an invisible OFF).
      await tester.pumpWidget(
        harness(FakeWalletSettingsStore(autoShieldValue: false)),
      );
      await tester.pumpAndSettle();
      l10n = l10nAt(tester);
      expect(find.text(l10n.walletTransparentFundsAutoOff), findsOneWidget);
      expect(
        find.byKey(const ValueKey('wallet-auto-shield-toggle')),
        findsNothing,
        reason: 'gate closed — the claim is the only signal',
      );
    });
  });

  group('pool-clarity surfaces (§3.2i-3 (c) + (a) cue)', () {
    Widget walletHarness(
      FakeWalletSession session, {
      AutoShieldStatus autoShield = AutoShieldStatus.idle,
    }) {
      return ProviderScope(
        overrides: [
          walletSessionProvider.overrideWithValue(session),
          walletAutoShieldControllerProvider.overrideWith(
            () => _StubAutoShield(autoShield),
          ),
        ],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const WalletScreen(),
        ),
      );
    }

    FakeWalletSession activeFake({int transparentZat = 0}) => FakeWalletSession(
      current: const SyncStatus.upToDate(tip: 100),
      snapshotValue: walletStateFixture(
        syncStatus: const SyncStatus.upToDate(tip: 100),
        balance: balanceFixture(
          spendableZat: 1000000,
          totalZat: 1000000 + transparentZat,
          transparentZat: transparentZat,
        ),
      ),
    );

    testWidgets(
      'activity rows badge a transparent-output tx — one globe glyph, and '
      'the a11y label says it in words',
      (tester) async {
        final fake = activeFake()
          ..transactionsResult = [
            txSummaryFixture(hasTransparentOutput: true),
            txSummaryFixture(
              txidHex:
                  'bb00000000000000000000000000000000000000000000000000000000000000',
              hasTransparentOutput: false,
            ),
          ];
        await tester.pumpWidget(walletHarness(fake));
        await tester.pumpAndSettle();
        final l10n = WalletLocalizations.of(
          tester.element(find.byType(WalletScreen)),
        );

        expect(
          find.byIcon(Icons.public),
          findsOneWidget,
          reason: 'exactly the flagged row carries the badge',
        );
        expect(
          find.bySemanticsLabel(RegExp(l10n.walletActivityPublicBadge)),
          findsOneWidget,
          reason: 'the badge is words to a screen reader, not just a glyph',
        );

        // The fact SURVIVES the tap (UX review M3): the detail sheet — the
        // surface a user opens to check the facts — states the visibility.
        await tester.tap(find.byIcon(Icons.public));
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletTxDetailVisibility), findsOneWidget);
        expect(
          find.text(l10n.walletActivityPublicBadge),
          findsOneWidget,
          reason: 'the visibility row carries the same words as the badge',
        );
      },
    );

    testWidgets('an UNFLAGGED tx\'s detail sheet makes NO visibility claim '
        '(absence of a public output is not a "fully private" proof)', (
      tester,
    ) async {
      final fake = activeFake()
        ..transactionsResult = [txSummaryFixture(hasTransparentOutput: false)];
      await tester.pumpWidget(walletHarness(fake));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletScreen)),
      );

      await tester.tap(find.text(l10n.walletActivityReceived).first);
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletTxDetailVisibility), findsNothing);
    });

    testWidgets(
      'the balance card shows the auto-shield honesty cue when the loop '
      'FAILED and transparent funds sit visible',
      (tester) async {
        await tester.pumpWidget(
          walletHarness(
            activeFake(transparentZat: 250000),
            autoShield: AutoShieldStatus.failed,
          ),
        );
        await tester.pumpAndSettle();
        final l10n = WalletLocalizations.of(
          tester.element(find.byType(WalletScreen)),
        );

        expect(
          find.byKey(const ValueKey('wallet-auto-shield-cue')),
          findsOneWidget,
        );
        expect(find.text(l10n.walletAutoShieldIncomplete), findsOneWidget);
      },
    );

    testWidgets('no cue when the loop is idle (or funds are gone)', (
      tester,
    ) async {
      await tester.pumpWidget(
        walletHarness(activeFake(transparentZat: 250000)),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('wallet-auto-shield-cue')),
        findsNothing,
      );
    });

    testWidgets('parked rows STACK the Cancel under the text at large scales '
        'so the amount AND save time survive (S155 wrap UX MAJOR-2, measured '
        'real-font); ordinary scales keep the compact single row', (
      tester,
    ) async {
      final fake = activeFake()
        ..parkedSendsResult = [parkedSendFixture(amountZat: 70000)];
      Widget scaled(double scale) => MediaQuery(
        data: MediaQueryData(textScaler: TextScaler.linear(scale)),
        child: walletHarness(fake),
      );

      await tester.pumpWidget(scaled(2.0));
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletScreen)),
      );
      final cancelBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedCancel,
      );
      final rowText = find.textContaining('saved & pending');
      // The home body is a lazy ListView; DRIVE it until the parked row is
      // built AND on-screen — content above it (e.g. the #389 pool line) can
      // push the row past the initial build range at large text scales, where a
      // plain find/ensureVisible would see nothing.
      await tester.scrollUntilVisible(
        cancelBtn,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(
        tester.getTopLeft(cancelBtn).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(rowText).dy),
        reason: 'at 2.0× the button yields the row width to the figures',
      );

      // The 1.0× leg is UNCHANGED by #361: the stack/inline decision is made on
      // MEASURED width + scale (`LayoutBuilder`, arch review M3), not on a
      // phone assumption — so at the 800dp test surface and 1.0× the compact
      // single-row shape survives even though the row now carries TWO actions
      // (Send now + Cancel). A phone-width surface stacks at every scale; that
      // property is pinned by the parked-section a11y tests.
      await tester.pumpWidget(scaled(1.0));
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        cancelBtn,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(
        tester.getTopLeft(cancelBtn).dy,
        lessThan(tester.getBottomLeft(rowText).dy),
        reason: 'at 1.0× on a wide surface the compact single-row shape stays',
      );
      expect(
        find.widgetWithText(TextButton, l10n.walletParkedSendNow),
        findsOneWidget,
        reason: 'the second action is the FR-23-b user-paced send',
      );
    });
  });
}
