import 'dart:ui' show Tristate;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
// `Override` (the list element type) is surfaced from misc.dart in Riverpod 3.x.
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_example/app.dart';
import 'package:zec_wallet_example/core/router/router.dart';
import 'package:zec_wallet_example/core/theme/theme_mode_provider.dart';
import 'package:zec_wallet_example/features/settings/appearance_screen.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_example/l10n/app_localizations.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Resolve the generated l10n bundle from a live element so finders are
/// coupled to l10n KEYS, not to English literals (code review fold: a
/// copy edit in app_en.arb must not silently decouple these tests).
AppLocalizations _l10nAt(WidgetTester tester, Finder anchor) =>
    AppLocalizations.of(tester.element(anchor));

/// The example launches straight into the wallet (the SDK is a wallet, so
/// there is no app-frame landing screen). The frame tests therefore inject a
/// host-VM fake provisioner — no native library — that boots to the Welcome
/// onboarding surface, the same seam the wallet feature tests use.
List<Override> _frameOverrides(ThemeMode mode) => [
  initialThemeModeProvider.overrideWithValue(mode),
  walletProvisionerProvider.overrideWithValue(
    FakeWalletProvisioner(exists: false),
  ),
  onboardingStoreProvider.overrideWithValue(FakeOnboardingStore()),
];

void main() {
  testWidgets('app_boots_offline_to_wallet_first_frame_themed', (tester) async {
    // The frame performs no I/O besides the (mocked) prefs read — pumping
    // with zero connectivity IS the production offline path (spec §6).
    SharedPreferences.setMockInitialValues({});
    await tester.pumpWidget(
      ProviderScope(
        overrides: _frameOverrides(ThemeMode.dark),
        child: const WalletExampleApp(),
      ),
    );

    // FIRST frame (no settle): the wallet is the home surface and already
    // wears the persisted theme — the flash-free-start contract (spec §3).
    expect(find.byType(WalletScreen), findsOneWidget);
    final ctx = tester.element(find.byType(WalletScreen));
    expect(Theme.of(ctx).brightness, Brightness.dark);

    // Settle the async boot probe (the fake resolves to Welcome) so no pending
    // future leaks into the next test; the wallet surface stays up.
    await tester.pumpAndSettle();
    expect(find.byType(WalletScreen), findsOneWidget);
  });

  testWidgets('unknown_route_shows_recoverable_error', (tester) async {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer(
      overrides: _frameOverrides(ThemeMode.dark),
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const WalletExampleApp(),
      ),
    );
    await tester.pumpAndSettle();

    container.read(routerProvider).go('/definitely/not/a/route');
    await tester.pumpAndSettle();

    // Plain language + a working next step, no codes (spec §6).
    final l10n = _l10nAt(tester, find.byType(Scaffold));
    expect(find.text(l10n.routeErrorTitle), findsOneWidget);
    await tester.tap(find.text(l10n.routeErrorGoHome));
    await tester.pumpAndSettle();
    expect(find.byType(WalletScreen), findsOneWidget);
  });

  testWidgets('appearance_screen_switches_theme_with_44px_semantic_targets', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer(
      overrides: _frameOverrides(ThemeMode.light),
    );
    addTearDown(container.dispose);
    // Disposed explicitly at the end of the body — the framework verifies
    // handles BEFORE addTearDown callbacks run.
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const WalletExampleApp(),
      ),
    );
    await tester.pumpAndSettle();

    container.read(routerProvider).go(AppRoutes.appearance);
    await tester.pumpAndSettle();
    expect(find.byType(AppearanceScreen), findsOneWidget);
    final l10n = _l10nAt(tester, find.byType(AppearanceScreen));

    // AMOLED only affects dark — while light is active the switch is
    // honestly DISABLED, not silently inert (spec §6 honest degradation):
    // at the widget level AND in what a screen reader is told. (Found by its
    // title: the screen also carries the Tor plugin's switch.)
    final amoledTile = find.widgetWithText(SwitchListTile, l10n.amoledTitle);
    expect(tester.widget<SwitchListTile>(amoledTile).onChanged, isNull);
    expect(
      tester.getSemantics(amoledTile).flagsCollection.isEnabled,
      Tristate.isFalse,
      reason: 'disabled AMOLED switch must announce disabled',
    );

    // Every interactive element ≥44×44 logical px (flutter-patterns
    // § Widget Conventions) AND exposes real control semantics at the
    // tile level (selected/on-off + enabled state — what TalkBack/
    // VoiceOver announce) — a regression to a bare GestureDetector fails
    // the flag asserts even though Material defaults would keep the
    // sizes green (review fold).
    final tileLabels = [
      l10n.themeModeSystemTitle,
      l10n.themeModeLightTitle,
      l10n.themeModeDarkTitle,
      l10n.amoledTitle,
    ];
    for (final label in tileLabels) {
      final tile = find.ancestor(
        of: find.text(label),
        matching: find.byWidgetPredicate(
          (w) => w is RadioListTile<ThemeMode> || w is SwitchListTile,
        ),
      );
      final size = tester.getSize(tile);
      expect(
        size.height,
        greaterThanOrEqualTo(44),
        reason: '$label tile is ${size.height} high',
      );
      expect(size.width, greaterThanOrEqualTo(44));

      final flags = tester.getSemantics(tile).flagsCollection;
      expect(
        flags.isSelected != Tristate.none,
        isTrue,
        reason: '$label lost its selected-state semantics',
      );
      expect(
        flags.isEnabled != Tristate.none,
        isTrue,
        reason: '$label lost its enabled-state semantics',
      );
    }
    final sliderSize = tester.getSize(find.byType(Slider));
    expect(sliderSize.height, greaterThanOrEqualTo(44));
    // One well-formed slider node announcing type + value (l10n-keyed).
    final sliderNode = tester.getSemantics(find.byType(Slider));
    expect(sliderNode.value, l10n.textSizeSemanticValue(100));

    // Switching to Dark takes effect (state + rendered brightness)…
    await tester.tap(find.text(l10n.themeModeDarkTitle));
    await tester.pumpAndSettle();
    expect(container.read(themeModeProvider), ThemeMode.dark);
    final ctx = tester.element(find.byType(AppearanceScreen));
    expect(Theme.of(ctx).brightness, Brightness.dark);

    // …and the AMOLED switch honestly re-enables under dark.
    expect(tester.widget<SwitchListTile>(amoledTile).onChanged, isNotNull);
    expect(
      tester.getSemantics(amoledTile).flagsCollection.isEnabled,
      Tristate.isTrue,
    );

    semantics.dispose();
  });
}
