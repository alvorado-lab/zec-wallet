import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_example/app.dart';
import 'package:zec_wallet_example/core/router/router.dart';
import 'package:zec_wallet_example/core/theme/appearance_prefs.dart';
import 'package:zec_wallet_example/features/settings/appearance_screen.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:zec_wallet_ui/testing.dart';

void main() {
  testWidgets('app_renders_without_overflow_at_max_text_scale', (tester) async {
    // Spec §7 promises "no OS × user combination breaks layouts"; the
    // composition test proves the CLAMP arithmetic, this proves the
    // widget tree actually survives the ceiling (mobile review fold):
    // OS accessibility 3.0× × user 1.4× → clamped to 2.0× effective.
    // A RenderFlex overflow throws in tests, failing this loudly.
    tester.platformDispatcher.textScaleFactorTestValue = 3.0;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
    // Small-phone viewport (360×640 logical) — the worst realistic case.
    await tester.binding.setSurfaceSize(const Size(360, 640));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    SharedPreferences.setMockInitialValues({});
    // The app launches into the wallet (Welcome, via the host-VM fake) — so
    // this proves the wallet surface ALSO survives the 2.0× ceiling, not only
    // the appearance screen.
    final container = ProviderContainer(
      overrides: [
        initialTextScaleProvider.overrideWithValue(maxTextScale),
        walletProvisionerProvider.overrideWithValue(
          FakeWalletProvisioner(exists: false),
        ),
        onboardingStoreProvider.overrideWithValue(FakeOnboardingStore()),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const WalletExampleApp(),
      ),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);

    container.read(routerProvider).go(AppRoutes.appearance);
    await tester.pumpAndSettle();
    expect(find.byType(AppearanceScreen), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
