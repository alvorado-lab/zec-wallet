import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/settings/security_screen.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// Stage S2 `switch` (docs/plan/stage-2-the-hosts-lifecycle.md §3.4,
/// assertion 8's copy half): a delete refused because a server switch is
/// still running tells the user what to wait for — "finish the server switch
/// first" — instead of the silent no-op a raced double-tap gets. The copy's
/// key is the implementer's; this row asserts what the user reads.
void main() {
  testWidgets(
    'a delete refused under an unfinished server switch says to finish the '
    'switch first',
    (tester) async {
      final hung = Completer<void>();
      final provisioner = FakeWalletProvisioner(exists: true)
        ..holdSwitch = hung;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletProvisionerProvider.overrideWithValue(provisioner),
            onboardingStoreProvider.overrideWithValue(
              FakeOnboardingStore(confirmed: true),
            ),
            walletCustodyDisclosureProvider.overrideWith(
              (ref) => Future<CustodyDisclosure>.value(
                const CustodyDisclosure(
                  tier: 'apple_secure_enclave',
                  eraseAssurance: EraseAssurance.hardwareKeyDeleted,
                  degraded: false,
                ),
              ),
            ),
          ],
          child: MaterialApp(
            theme: lightTheme,
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            home: const SecurityScreen(),
          ),
        ),
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SecurityScreen)),
      );
      container.listen(onboardingControllerProvider, (_, _) {});
      await tester.pumpAndSettle();
      expect(
        container.read(onboardingControllerProvider),
        isA<OnboardingActive>(),
        reason: 'precondition: a deletable wallet',
      );
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SecurityScreen)),
      );
      final mentionsSwitch = find.textContaining(
        RegExp('switch', caseSensitive: false),
      );
      final switchMentionsBefore = mentionsSwitch.evaluate().length;

      // A server switch is running (started from the picker elsewhere).
      unawaited(
        container
            .read(onboardingControllerProvider.notifier)
            .switchSyncServer(const SyncServerChoice.predefined(id: 'x')),
      );
      await tester.pump();
      expect(provisioner.switchCount, 1);

      await tester.ensureVisible(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.securityDeleteWalletButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.securityDeleteDialogConfirm));
      await tester.pump();
      await tester.pump();

      expect(provisioner.deleteCount, 0, reason: 'nothing wiped mid-switch');
      expect(find.byType(SecurityScreen), findsOneWidget);
      expect(
        mentionsSwitch.evaluate().length,
        greaterThan(switchMentionsBefore),
        reason: 'the refusal names the switch the user must wait for',
      );

      hung.complete();
      await tester.pumpAndSettle();
    },
  );
}
