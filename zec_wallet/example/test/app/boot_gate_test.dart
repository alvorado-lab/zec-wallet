import 'dart:async' show Completer;

import 'package:flutter/material.dart';
// `Override` (the list element type) is surfaced from misc.dart in Riverpod 3.x.
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:zec_wallet_example/core/boot/wallet_boot_gate.dart';
import 'package:zec_wallet_example/core/theme/theme_mode_provider.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_startup_failed_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The boot gate (#356-F1) at the consumer seat: a FAILED startup renders the
/// SDK's honest couldn't-start screen (never the not-set-up copy, never a
/// blank frame), and its retry re-runs the injected boot — swapping to the
/// real app the moment an attempt succeeds.
void main() {
  const prefs = AppearanceBootPrefs(
    themeMode: ThemeMode.light,
    textScale: 1.0,
    amoled: false,
  );

  // A SUCCESSFUL attempt's override list — the app_frame_test fake seat (a
  // host-VM provisioner, no native lib), which boots the wallet to Welcome.
  List<Override> readyOverrides() => [
    initialThemeModeProvider.overrideWithValue(ThemeMode.light),
    walletProvisionerProvider.overrideWithValue(
      FakeWalletProvisioner(exists: false),
    ),
    onboardingStoreProvider.overrideWithValue(FakeOnboardingStore()),
  ];

  testWidgets(
    'failed boot → honest couldn\'t-start screen; retry succeeds → the real '
    'app replaces it',
    (tester) async {
      SharedPreferences.setMockInitialValues({});
      var attempts = 0;
      await tester.pumpWidget(
        WalletBootGate(
          prefs: prefs,
          firstAttempt: null, // the pre-runApp attempt failed
          boot: () async {
            attempts++;
            return readyOverrides(); // the retry succeeds
          },
        ),
      );
      await tester.pumpAndSettle();

      // The failure surface — not a blank frame, not the wallet shell.
      expect(find.byType(WalletStartupFailedScreen), findsOneWidget);
      expect(find.byType(WalletScreen), findsNothing);

      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletStartupFailedScreen)),
      );
      await tester.tap(find.text(l10n.walletOnboardingRetry));
      await tester.pumpAndSettle();

      expect(attempts, 1);
      // The real app took over (the wallet boots to Welcome on the fake seat).
      expect(find.byType(WalletStartupFailedScreen), findsNothing);
      expect(find.byType(WalletScreen), findsOneWidget);
    },
  );

  testWidgets(
    'a retry that fails again stays on the honest screen — and the button '
    'RE-ENABLES for the next attempt (a stuck retry would dead-end recovery)',
    (tester) async {
      SharedPreferences.setMockInitialValues({});
      var attempts = 0;
      await tester.pumpWidget(
        WalletBootGate(
          prefs: prefs,
          firstAttempt: null,
          // Fails once, then succeeds — the fail→recover sequence.
          boot: () async => ++attempts >= 2 ? readyOverrides() : null,
        ),
      );
      await tester.pumpAndSettle();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(WalletStartupFailedScreen)),
      );

      await tester.tap(find.text(l10n.walletOnboardingRetry));
      await tester.pumpAndSettle();
      expect(attempts, 1);
      expect(find.byType(WalletStartupFailedScreen), findsOneWidget);

      // The SECOND tap must act — pinning that `_retrying` reset after the
      // failed attempt (a permanently-disabled button passes a weaker test).
      await tester.tap(find.text(l10n.walletOnboardingRetry));
      await tester.pumpAndSettle();
      expect(attempts, 2);
      expect(find.byType(WalletStartupFailedScreen), findsNothing);
      expect(find.byType(WalletScreen), findsOneWidget);
    },
  );

  testWidgets('a successful first attempt renders the app directly', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    await tester.pumpWidget(
      WalletBootGate(
        prefs: prefs,
        firstAttempt: readyOverrides(),
        boot: () async => fail('boot must not re-run on a successful start'),
      ),
    );
    // FIRST frame: the wallet is already the home surface (the flash-free
    // contract holds through the gate).
    expect(find.byType(WalletScreen), findsOneWidget);
    await tester.pumpAndSettle();
  });

  testWidgets(
    'DESKTOP shape (attemptedPreRunApp: false): a themed pending spinner '
    'while the in-gate first attempt runs — never a blank window — then the '
    'app (S175)',
    (tester) async {
      SharedPreferences.setMockInitialValues({});
      final gate = Completer<List<Override>?>();
      await tester.pumpWidget(
        WalletBootGate(
          prefs: prefs,
          firstAttempt: null,
          attemptedPreRunApp: false,
          boot: () => gate.future,
        ),
      );
      await tester.pump();

      // Pending: a spinner inside the themed shell — NOT the failure screen,
      // NOT the app.
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.byType(WalletStartupFailedScreen), findsNothing);
      expect(find.byType(WalletScreen), findsNothing);

      gate.complete(readyOverrides());
      await tester.pumpAndSettle();
      expect(find.byType(WalletScreen), findsOneWidget);
    },
  );

  testWidgets('the gate survives a boot() that THROWS: failure screen, retry '
      're-enabled (the throw-belt finally is load-bearing)', (tester) async {
    SharedPreferences.setMockInitialValues({});
    var attempts = 0;
    await tester.pumpWidget(
      WalletBootGate(
        prefs: prefs,
        firstAttempt: null,
        boot: () async {
          attempts++;
          throw StateError('boot contract violated');
        },
      ),
    );
    await tester.pumpAndSettle();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(WalletStartupFailedScreen)),
    );

    await tester.tap(find.text(l10n.walletOnboardingRetry));
    await tester.pumpAndSettle();
    expect(attempts, 1);
    expect(find.byType(WalletStartupFailedScreen), findsOneWidget);

    // The button must still be live after the throw.
    await tester.tap(find.text(l10n.walletOnboardingRetry));
    await tester.pumpAndSettle();
    expect(attempts, 2);
  });
}
